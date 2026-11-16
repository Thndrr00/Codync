use super::{Store, now_ms};
use crate::LockExt;
use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

pub(super) fn open(db: &rusqlite::Connection) -> Result<()> {
    // Older hosts keyed all bots' conversations by chat alone. Keep their root and
    // approval history, while letting a different bot have its own conversation.
    let (legacy, has_sender) = {
        let mut columns = db.prepare("PRAGMA table_info(whatsapp_threads)")?;
        let names = columns.query_map([], |row| row.get::<_, String>(1))?.collect::<rusqlite::Result<Vec<_>>>()?;
        (!names.is_empty() && !names.iter().any(|name| name == "bot_id"), names.iter().any(|name| name == "sender"))
    };
    if legacy {
        // The oldest tables kept only the chat and its root: there is no sender to carry over.
        let sender = if has_sender { "t.sender" } else { "''" };
        let tx = db.unchecked_transaction()?;
        tx.execute_batch("ALTER TABLE whatsapp_threads RENAME TO whatsapp_threads_legacy;")?;
        tx.execute_batch(include_str!("whatsapp.sql"))?;
        tx.execute_batch(&format!(
            "INSERT INTO whatsapp_threads(chat,bot_id,root_id,sender)
             SELECT t.chat,e.bot_id,t.root_id,{sender} FROM whatsapp_threads_legacy t
             JOIN entries e ON e.id=t.root_id;
             DROP TABLE whatsapp_threads_legacy;"
        ))?;
        tx.commit()?;
    }
    db.execute_batch(include_str!("whatsapp.sql"))?;
    Ok(())
}

pub struct WhatsAppOutgoing {
    pub entry: String,
    pub chat: String,
    pub text: String,
    /// The files the reply carries: path, name and type, as the entry stores them.
    pub files: Vec<serde_json::Value>,
}

/// The most facts one person's record holds; the prompt states the limit, so a refusal can too.
pub const WHATSAPP_FACTS: usize = 40;
/// The most one person fact holds, in characters.
pub const WHATSAPP_FACT_CHARS: usize = 500;

/// A person Codync knows in one linked `WhatsApp` account: their name and the facts they gave.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsAppPerson {
    pub sender: String,
    pub name: String,
    pub facts: Vec<String>,
}

#[cfg(test)]
mod tests;

impl Store {
    /// The root entry of a `WhatsApp` conversation, if it has one.
    pub fn whatsapp_root(&self, chat: &str, bot: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .locked()
            .query_row("SELECT root_id FROM whatsapp_threads WHERE chat=?1 AND bot_id=?2", params![chat, bot], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn map_whatsapp_thread(&self, chat: &str, bot: &str, root: &str, sender: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO whatsapp_threads(chat,bot_id,root_id,sender) VALUES(?1,?2,?3,?4)
            ON CONFLICT(chat,bot_id) DO UPDATE SET sender=excluded.sender",
            params![chat, bot, root, sender],
        )?;
        Ok(())
    }
    /// Agent replies that are final and not in the outbox yet, with their conversation.
    pub fn pending_whatsapp_replies(&self) -> Result<Vec<WhatsAppOutgoing>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT e.id,t.chat,COALESCE(json_extract(e.data,'$.text'),''),COALESCE(json_extract(e.data,'$.files'),'[]')
            FROM entries e JOIN whatsapp_threads t ON t.root_id=e.thread_id AND t.bot_id=e.bot_id
            WHERE e.kind='agent' AND json_extract(e.data,'$.final')=1
            AND (COALESCE(json_extract(e.data,'$.text'),'')<>'' OR json_extract(e.data,'$.files') IS NOT NULL)
            AND NOT EXISTS(SELECT 1 FROM whatsapp_outbox o WHERE o.entry_id=e.id) ORDER BY e.seq LIMIT 20",
        )?;
        Ok(q.query_map([], |r| {
            let files: String = r.get(3)?;
            Ok(WhatsAppOutgoing {
                entry: r.get(0)?,
                chat: r.get(1)?,
                text: r.get(2)?,
                files: serde_json::from_str(&files).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn claim_whatsapp_reply(&self, entry: &str) -> Result<bool> {
        Ok(self.db.locked().execute(
            "INSERT INTO whatsapp_outbox VALUES(?1,'sending',?2) ON CONFLICT DO NOTHING",
            params![entry, now_ms()],
        )? == 1)
    }
    pub fn finish_whatsapp_reply(&self, entry: &str, state: &str) -> Result<()> {
        self.db.locked().execute(
            "UPDATE whatsapp_outbox SET state=?2,updated_at=?3 WHERE entry_id=?1 AND state='sending'",
            params![entry, state, now_ms()],
        )?;
        Ok(())
    }

    /// Approval cards still pending in `WhatsApp` chats that were not shown yet.
    pub fn pending_whatsapp_cards(&self) -> Result<Vec<(String, String)>> {
        let db = self.db.locked();
        let mut q = db.prepare(
            "SELECT t.chat,e.id FROM entries e JOIN whatsapp_threads t ON t.root_id=e.thread_id AND t.bot_id=e.bot_id
            WHERE e.kind='permission' AND json_extract(e.data,'$.status')='pending'
            AND NOT EXISTS(SELECT 1 FROM whatsapp_cards c WHERE c.chat=t.chat AND c.entry_id=e.id)
            AND NOT EXISTS(SELECT 1 FROM whatsapp_card_exclusions x WHERE x.entry_id=e.id)
            ORDER BY e.seq LIMIT 10",
        )?;
        Ok(q.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)
    }
    pub fn mark_whatsapp_card(&self, chat: &str, entry: &str, sender: &str) -> Result<()> {
        self.db.locked().execute(
            "INSERT INTO whatsapp_cards VALUES(?1,?2,?3,?4) ON CONFLICT DO NOTHING",
            params![chat, entry, sender, now_ms()],
        )?;
        Ok(())
    }
    /// A legacy or mixed-author decision remains in the app, without blocking later cards.
    pub fn exclude_whatsapp_card(&self, entry: &str) -> Result<()> {
        self.db.locked().execute("INSERT INTO whatsapp_card_exclusions VALUES(?1) ON CONFLICT DO NOTHING", [entry])?;
        Ok(())
    }
    /// The newest approval card shown in a chat, for the person who may answer it.
    pub fn whatsapp_card(&self, chat: &str, sender: &str) -> Option<String> {
        self.db
            .locked()
            .query_row(
                "SELECT entry_id FROM whatsapp_cards WHERE chat=?1 AND sender=?2 ORDER BY sent_at DESC",
                params![chat, sender],
                |r| r.get(0),
            )
            .ok()
    }
    pub fn forget_whatsapp_card(&self, chat: &str, entry: &str) -> Result<()> {
        self.db.locked().execute("DELETE FROM whatsapp_cards WHERE chat=?1 AND entry_id=?2", params![chat, entry])?;
        Ok(())
    }

    /// The chat and its sender for the conversation a thread belongs to, when it is a `WhatsApp` one.
    pub fn whatsapp_thread_sender(&self, root: &str) -> Result<Option<(String, String)>> {
        Ok(self
            .db
            .locked()
            .query_row("SELECT chat,sender FROM whatsapp_threads WHERE root_id=?1", [root], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()?)
    }

    /// One person of a linked account: their name and the facts they gave, when they are known.
    pub fn whatsapp_person(&self, account: &str, sender: &str) -> Result<Option<WhatsAppPerson>> {
        Ok(self
            .db
            .locked()
            .query_row(
                "SELECT sender,name,facts FROM whatsapp_people WHERE account=?1 AND sender=?2",
                params![account, sender],
                person_row,
            )
            .optional()?)
    }

    /// Every person of a linked account, by their newest change: the facts the sheet shows.
    pub fn whatsapp_people(&self, account: &str) -> Result<Vec<WhatsAppPerson>> {
        let db = self.db.locked();
        let mut query = db.prepare(
            "SELECT sender,name,facts FROM whatsapp_people WHERE account=?1 ORDER BY updated_at DESC LIMIT 100",
        )?;
        Ok(query.query_map([account], person_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Keeps the name `WhatsApp` gives for a person, without a fact: a message from them refreshes it.
    pub fn note_whatsapp_person(&self, account: &str, sender: &str, name: &str) -> Result<()> {
        if account.is_empty() || sender.is_empty() {
            return Ok(());
        }
        let name = name.trim();
        self.db.locked().execute(
            "INSERT INTO whatsapp_people(account,sender,name,facts,updated_at) VALUES(?1,?2,?3,'[]',?4)
            ON CONFLICT(account,sender) DO UPDATE SET
                name=CASE WHEN excluded.name<>'' THEN excluded.name ELSE whatsapp_people.name END,
                updated_at=excluded.updated_at",
            params![account, sender, name.chars().take(120).collect::<String>(), now_ms()],
        )?;
        Ok(())
    }

    /// Saves one fact the person gave, or corrects the fact with the same text: the record never holds it twice.
    pub fn remember_whatsapp_person(
        &self,
        account: &str,
        sender: &str,
        name: &str,
        raw: &str,
    ) -> Result<WhatsAppPerson> {
        if account.is_empty() || sender.is_empty() {
            bail!("A person needs the WhatsApp account and their own id.");
        }
        let fact = one_line(raw);
        if fact.is_empty() {
            bail!("text is required: the fact to keep, in one short line");
        }
        let chars = fact.chars().count();
        if chars > WHATSAPP_FACT_CHARS {
            bail!(
                "A fact about a person holds at most {WHATSAPP_FACT_CHARS} characters, and this one has {chars}: \
                 save the point in fewer words."
            );
        }
        let mut person = self
            .whatsapp_person(account, sender)?
            .unwrap_or_else(|| WhatsAppPerson { sender: sender.to_owned(), ..WhatsAppPerson::default() });
        match person.facts.iter().position(|kept| kept.eq_ignore_ascii_case(&fact)) {
            Some(at) => person.facts[at] = fact,
            None if person.facts.len() >= WHATSAPP_FACTS => {
                bail!(
                    "This person has {WHATSAPP_FACTS} facts, as many as one record holds: forget one with \
                     person_forget_memory first, or keep this point shorter."
                );
            }
            None => person.facts.push(fact),
        }
        if !name.trim().is_empty() {
            person.name = name.trim().chars().take(120).collect();
        }
        self.db.locked().execute(
            "INSERT INTO whatsapp_people(account,sender,name,facts,updated_at) VALUES(?1,?2,?3,?4,?5)
            ON CONFLICT(account,sender) DO UPDATE SET name=excluded.name, facts=excluded.facts, updated_at=excluded.updated_at",
            params![account, sender, person.name, serde_json::to_string(&person.facts)?, now_ms()],
        )?;
        Ok(person)
    }

    /// Takes one fact out, matched by its text exactly or ignoring case when typed from memory.
    pub fn forget_whatsapp_person_fact(&self, account: &str, sender: &str, raw: &str) -> Result<WhatsAppPerson> {
        let wanted = one_line(raw);
        if wanted.is_empty() {
            bail!("text is required: the fact to forget, exactly as the person's record lists it");
        }
        let Some(mut person) = self.whatsapp_person(account, sender)? else {
            bail!("This person has no saved facts yet.");
        };
        let at = person
            .facts
            .iter()
            .position(|kept| *kept == wanted)
            .or_else(|| person.facts.iter().position(|kept| kept.eq_ignore_ascii_case(&wanted)))
            .ok_or_else(|| {
                anyhow::anyhow!("No saved fact matches that text: give one exactly as the record lists it.")
            })?;
        person.facts.remove(at);
        self.db.locked().execute(
            "UPDATE whatsapp_people SET facts=?3, updated_at=?4 WHERE account=?1 AND sender=?2",
            params![account, sender, serde_json::to_string(&person.facts)?, now_ms()],
        )?;
        Ok(person)
    }
}

fn person_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WhatsAppPerson> {
    let facts: String = row.get(2)?;
    Ok(WhatsAppPerson {
        sender: row.get(0)?,
        name: row.get(1)?,
        facts: serde_json::from_str(&facts).unwrap_or_default(),
    })
}

/// A fact on one line, so it cannot start a section of the prompt of its own.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
