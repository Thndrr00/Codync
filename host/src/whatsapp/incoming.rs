//! Incoming `WhatsApp` messages, routed through the host bot actor.

use super::{Arc, Hub, Value, WhatsApp, entry_id, inbox, json, save_media, settings, voice_notes};

/// OpenBot 0.74.0: the status reactions a number puts on a person's message — 👀 when it arrives,
/// then ✅ / ❌ / 🫡 when the request ends. Replies and error messages are not reactions and are
/// never held back by the number's switch.
const STATUS_REACTIONS: [&str; 4] = ["👀", "✅", "❌", "🫡"];

/// Whether `emoji` goes out with this number's status-reactions switch in `on`. Pure so the
/// decision is testable without a link: only the four status emoji depend on the switch.
fn status_reaction_allowed(on: bool, emoji: &str) -> bool {
    on || !STATUS_REACTIONS.contains(&emoji)
}

/// Whether an arriving message deserves the "Working on it…" line. A result meant for a teammate
/// is internal to this computer (OpenBot nightly triage-13131): the contact never asked for it, so
/// no working line goes out. The number's owner and ordinary contact messages still get one.
/// Pure so the decision can be tested without a link or a bot actor.
fn working_line_allowed(internal_result: bool, owner: bool) -> bool {
    owner || !internal_result
}

/// The agent-facing prompt for one message. `place` already carries the group-name fallback from
/// [`super::place_name`], so a group whose name is missing still reads as "WhatsApp group" here.
/// Pure so the fallback reaching the prompt is testable without a link or a bot actor.
fn prompt(place: &str, note: &str) -> String {
    format!("WhatsApp message from {place}:\n\n{note}")
}

/// OpenBot 0.72.0: the message you reply to brings its text and file into the agent's prompt. Pure
/// and deliberately brief: the path is already saved by the media store, and no sender id is logged.
fn quoted_context(text: &str, file: Option<&str>, name: Option<&str>) -> Option<String> {
    let text = text.trim();
    let mut lines = Vec::new();
    if !text.is_empty() {
        lines.push(format!("Replying to: {text}"));
    } else if let Some(name) = name.filter(|name| !name.is_empty()) {
        lines.push(format!("Replying to a file: {name}"));
    }
    if let Some(file) = file {
        lines.push(format!("[Attached: {file}]"));
    }
    (!lines.is_empty()).then(|| lines.join("\n\n"))
}

/// Unescapes the printable parts of a vCard value. Folded continuation lines are handled by the
/// caller, and a literal newline becomes a space so the text stays one line in a prompt.
fn unescape_vcard(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 'N') => out.push(' '),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// OpenBot 0.77.0: a shared contact card used to reach the agent as an empty message. Turns the
/// vCard the bridge sends into one readable line with the contact's name and phone numbers, for
/// both the live prompt and the kept history. Pure so synthetic cards can be tested without a
/// link. The numbers go only into the prompt and the history text, never into a log.
pub(crate) fn contact_card_text(vcard: &str, display_name: &str) -> Option<String> {
    let mut name = String::new();
    let mut numbers: Vec<String> = Vec::new();
    for line in vcard.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            continue; // a folded continuation of the previous line, already read
        }
        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.split(';').next().unwrap_or_default().trim().to_ascii_uppercase();
        let value = unescape_vcard(value.trim());
        match key.as_str() {
            "FN" if name.is_empty() && !value.is_empty() => name = value,
            "TEL" => {
                let plain = value.strip_prefix("tel:").unwrap_or(&value);
                let number: String = plain.chars().filter(|c| c.is_ascii_digit() || *c == '+').take(32).collect();
                if number.chars().count() > 1 && !numbers.contains(&number) && numbers.len() < 8 {
                    numbers.push(number);
                }
            }
            _ => {}
        }
    }
    if name.is_empty() {
        name = unescape_vcard(display_name.trim());
    }
    if name.is_empty() && numbers.is_empty() {
        return None;
    }
    let mut text = format!("Contact card: {}", if name.is_empty() { "Unnamed contact" } else { &name });
    if !numbers.is_empty() {
        text = format!("{text} — {}", numbers.join(", "));
    }
    Some(text)
}

/// OpenBot 0.77.0: in an allowed group, a reply to a post an agent or a routine made goes to the
/// configured WhatsApp bot (the orchestrator), even without a mention and even if the reply text
/// names someone: the orchestrator sees which agent wrote the post and routes it. Every other
/// message keeps the usual `@name`-then-default choice. Pure so the routing is testable without a
/// store or a link.
pub(crate) fn route_reply<'a>(
    group: bool,
    quoted: bool,
    named: Option<&'a str>,
    configured: Option<&'a str>,
) -> Option<&'a str> {
    if group && quoted {
        return configured.or(named);
    }
    named.or(configured)
}

impl WhatsApp {
    pub(super) async fn on_message(&self, hub: &Arc<Hub>, event: &Value, expected: &str) {
        // OpenBot 0.77.0: a contact card arrives with no text. Read it before the empty-message
        // check so the card reaches both the history row and the agent's prompt.
        let card = event["contact"].as_object().and_then(|contact| {
            contact_card_text(
                contact["vcard"].as_str().unwrap_or_default(),
                contact["name"].as_str().unwrap_or_default(),
            )
        });
        let text = card.unwrap_or_else(|| event["text"].as_str().unwrap_or_default().trim().to_owned());
        let Some(chat) = event["chat"].as_str().filter(|c| !c.is_empty()) else { return };
        if text.is_empty() && event["media"].is_null() && event["quotedMedia"].is_null() {
            // OpenBot 0.70.0: a newly linked number may first get a group message it can't decrypt.
            // WhatsApp sends it again once it can; remember this empty first copy so the readable
            // copy reaches the group history and the orchestrator instead of being dropped.
            if let Some(id) = event["id"].as_str() {
                super::history::note_unreadable(chat, id, crate::store::now_ms());
            }
            return;
        }
        let sender = event["sender"].as_str().unwrap_or_default();
        let group = event["group"].as_bool().unwrap_or(false);
        let media = event["media"].as_object();
        // The file is saved as the message arrives, before the mention and allow checks, because the
        // number keeps its files along with its text (OpenBot 0.72.0).
        let saved = match media {
            Some(media) => save_media(media).await.ok().flatten(),
            None => None,
        };
        let quoted_media = event["quotedMedia"].as_object();
        let quoted_saved = match quoted_media {
            Some(media) => save_media(media).await.ok().flatten(),
            None => None,
        };
        // Ported from `OpenBot` 0.72.0: the number keeps what it received for 90 days, also the group
        // lines that do not mention it, so an agent can read them back later. OpenBot 0.70.0: a
        // readable resend of an unreadable first copy is kept; an ordinary duplicate stops here.
        if !super::history::remember(
            &hub.store,
            &super::history::Message {
                id: event["id"].as_str().unwrap_or_default().to_owned(),
                chat,
                sender,
                name: event["name"].as_str().unwrap_or_default(),
                text: text.as_str(),
                file: saved.as_deref(),
                at: crate::store::now_ms(),
            },
        )
        .unwrap_or(false)
        {
            return;
        }
        if group && !event["mentioned"].as_bool().unwrap_or(false) && !event["quoted"].as_bool().unwrap_or(false) {
            return;
        }
        // With a list, only the owner, allowed people and allowed groups get through. A read-only
        // entry still had its message kept above, but opens no bot turn (OpenBot 0.77.0).
        let allowed = settings::read_allowed(&hub.store).unwrap_or_default();
        let me = self.state.lock().await.me.clone().unwrap_or_default();
        let own = settings::is_own_chat(&me, chat, sender);
        let principal = if group { chat } else { sender };
        if !allowed.is_empty() && !own {
            let Some(entry) = allowed.iter().find(|entry| entry.jid == principal) else { return };
            if !settings::can_reply(Some(entry)) {
                return;
            }
        }
        // The orchestrator: an `@name` in the message picks that bot, anything else the chosen one.
        let roster: Vec<crate::store::BotRow> = hub
            .store
            .bots()
            .unwrap_or_default()
            .into_iter()
            .filter(|bot| {
                !bot.deleted
                    && !bot.config.hidden
                    && !bot.config.is_group()
                    && !crate::external::is_external(&bot.config)
            })
            .collect();
        let uses_default = event["_codyncUsesDefault"] == true;
        let bot =
            event["_codyncBotId"].as_str().map(str::to_owned).filter(|id| roster.iter().any(|b| &b.config.id == id));
        let Some(bot) = bot else {
            let _ = self
                .send_message(
                    chat,
                    "No hay ningún bot asignado a este WhatsApp. Elige uno en Ajustes.",
                    None,
                    None,
                    None,
                )
                .await;
            return;
        };
        // A message routed to a bot that is waiting on a teammate is an internal result, not a
        // request from the contact: it must not post a working line (triage-13131).
        let internal_result = hub.team.pending_for(&bot).iter().any(|request| request["from"] == bot);
        let post_working_line = working_line_allowed(internal_result, own);
        let mut attached = Vec::new();
        let note = if let Some(m) = media {
            if let Some(path) = &saved {
                attached.push(path.clone());
                format!("{text}\n\n[Attached: {path}]").trim().to_owned()
            } else {
                let kind = m.get("kind").and_then(Value::as_str).unwrap_or("file");
                format!("[{kind}] {}", m.get("name").and_then(Value::as_str).unwrap_or(&text))
            }
        } else {
            text.clone()
        };
        let note = if let Some(media) = media { voice_notes::append(&hub.store, media, note).await } else { note };
        let quoted_name = quoted_media.and_then(|media| media.get("name")).and_then(Value::as_str);
        let note = if let Some(context) =
            quoted_context(event["quotedText"].as_str().unwrap_or_default(), quoted_saved.as_deref(), quoted_name)
        {
            if let Some(path) = &quoted_saved {
                attached.push(path.clone());
            }
            format!("{note}\n\n{context}")
        } else {
            note
        };
        // Audio work can outlive a configuration edit or unlink. Do not hand the old message
        // to a bot after its connection, allowlist, or default binding has been revoked.
        let current_allowed = settings::read_allowed(&hub.store).unwrap_or_default();
        let current_link = self.state().await;
        if current_link.state != "connected"
            || current_link.me.as_deref().unwrap_or_default() != me
            || (!current_allowed.is_empty()
                && !own
                && !current_allowed.iter().any(|entry| entry.jid == principal && settings::can_reply(Some(entry))))
            || (uses_default && hub.store.kv_get("whatsapp.bot").as_deref() != Some(bot.as_str()))
        {
            return;
        }
        // From this fence through enqueue there are no await points. Setting/link mutations use
        // the same lock, and the actor checks this fence again before starting a model turn.
        let configuration = crate::LockExt::locked(&hub.store.connector_lock);
        match inbox::snapshot(hub, event, current_link.me.as_deref().unwrap_or_default()) {
            Ok(Some(current)) if current == expected => {}
            Ok(_) => return,
            Err(error) => {
                tracing::warn!(%error, "WhatsApp authorization could not be checked");
                return;
            }
        }
        let routine_event = json!({
            "source": "whatsapp",
            "event": if group { "groupMention" } else { "message" },
            "chat": event["chat"],
            "from": event["sender"],
            "name": super::place_name(event["name"].as_str(), group, sender),
            "text": text,
            "group": event["group"],
            "media": media.cloned(),
        });
        hub.routines.receive_event(&hub.store, &routine_event, event["id"].as_str().map(str::to_owned));
        let Some(bot_row) = hub.store.bot(&bot).ok().flatten().filter(|row| !row.deleted && !row.config.hidden) else {
            return;
        };
        let root = match hub.store.whatsapp_root(chat, &bot) {
            Ok(root) => root,
            Err(error) => {
                tracing::warn!(%error, "Could not read the WhatsApp conversation");
                return;
            }
        };
        let root = if let Some(root) = root {
            root
        } else {
            let root = entry_id(&["root", chat, &bot, expected]);
            if let Err(error) = hub.add_entry_once(
                &root,
                &crate::store::Lane::main(&bot),
                crate::store::EntryKind::User,
                hub.store.max_turn(&bot),
                &json!({"text":format!("WhatsApp conversation · {}", super::place_name(event["name"].as_str(), group, sender)), "source":"whatsapp", "status":"sent", "whatsappChat": chat}),
            ) {
                tracing::warn!(%error, "Could not create the WhatsApp conversation");
                return;
            }
            root
        };
        if let Err(error) = hub.store.map_whatsapp_thread(chat, &bot, &root, sender) {
            tracing::warn!(%error, "Could not route the WhatsApp conversation");
            return;
        }
        let id = entry_id(&["message", chat, event["id"].as_str().unwrap_or_default()]);
        let prompt = prompt(super::place_name(event["name"].as_str(), group, sender), &note);
        let lane = crate::store::Lane::in_thread(&bot, &root);
        let Ok((entry, _)) = hub.add_entry_once(
            &id,
            &lane,
            crate::store::EntryKind::User,
            hub.store.max_turn(&bot) + 1,
            &json!({"text":prompt, "status":"queued", "source":"whatsapp", "whatsappChat": chat,
                "whatsappFiles": attached, "whatsappGeneration":expected,
                "whatsappFence":{"_codyncBotId":bot,"_codyncUsesDefault":uses_default,"_codyncMe":me,
                    "chat":chat,"sender":sender,"group":group}}),
        ) else {
            return;
        };
        if entry.data["status"] == "queued" {
            let text = crate::chat::prompt::of_entry(hub, &bot_row.config, &entry.data);
            if let Err(error) = hub.send_cmd(&bot, crate::agent::bot::Cmd::Send { lane, entry_id: entry.id, text }) {
                tracing::warn!(%error, "Could not deliver a WhatsApp message to a bot");
            }
            // The fence did its job at the enqueue; the acknowledgement is network work.
            drop(configuration);
            self.acknowledge(
                hub,
                chat,
                sender,
                group,
                event["fromMe"].as_bool().unwrap_or(false),
                event["id"].as_str().unwrap_or_default(),
                post_working_line,
            )
            .await;
        }
    }

    /// Tells the sender that the request arrived: a 👀 reaction on their message, then, unless the
    /// number's owner turned it off, a "Working on it…" line. The answer itself follows. The 👀 is
    /// a status reaction (OpenBot 0.74.0), so the number's switch can drop it without dropping the
    /// working line or the answer.
    async fn acknowledge(
        &self,
        hub: &Hub,
        chat: &str,
        sender: &str,
        group: bool,
        from_me: bool,
        message_id: &str,
        post_working_line: bool,
    ) {
        if message_id.is_empty() {
            return;
        }
        let Ok(snapshot) = hub.store.snapshot_whatsapp_output(&json!({})) else { return };
        let binding = snapshot["whatsappBinding"].clone();
        let account = binding[0].as_str().unwrap_or_default();
        // A group reaction names its author; in a direct chat the chat itself is the author.
        let participant = if group { sender } else { "" };
        if status_reaction_allowed(super::settings::status_reactions(&hub.store), "👀") {
            if let Err(error) =
                self.send_bound_react(&hub.store, account, &binding, chat, message_id, participant, from_me, "👀").await
            {
                tracing::debug!(%error, "WhatsApp could not acknowledge a message");
                return;
            }
        }
        if !post_working_line || !crate::whatsapp::working_message(&hub.store) {
            return;
        }
        if let Err(error) = self.send_bound_text(&hub.store, account, &binding, chat, "Trabajando en ello…").await {
            tracing::debug!(%error, "WhatsApp could not post the working line");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_status_reactions_off_drops_only_the_status_emoji() {
        // The four OpenBot status reactions go out when the switch is on and stop when it is off.
        for emoji in STATUS_REACTIONS {
            assert!(status_reaction_allowed(true, emoji), "{emoji} goes out with the switch on");
            assert!(!status_reaction_allowed(false, emoji), "{emoji} stops with the switch off");
        }
        // Answers, error messages and a person's own reactions never depend on the switch.
        for emoji in ["👍", "❤️", "🎉", "💔"] {
            assert!(status_reaction_allowed(true, emoji));
            assert!(status_reaction_allowed(false, emoji), "{emoji} is not a status reaction");
        }
    }

    #[test]
    fn only_an_internal_teammate_result_loses_the_working_line() {
        assert!(working_line_allowed(false, false), "a contact's message gets an acknowledgement");
        assert!(!working_line_allowed(true, false), "a teammate's result is internal");
        assert!(working_line_allowed(true, true), "the owner still gets one");
        assert!(working_line_allowed(false, true));
    }

    #[test]
    fn a_reply_shows_the_quoted_text_and_its_file_to_the_agent() {
        let file = "/data/codync/whatsapp-media/123-quoted.pdf";
        assert_eq!(
            quoted_context("  see the attached plan  ", Some(file), Some("plan.pdf")),
            Some(format!("Replying to: see the attached plan\n\n[Attached: {file}]"))
        );
        assert_eq!(
            quoted_context("looks good", None, None),
            Some("Replying to: looks good".to_owned()),
            "a fileless quote still reaches the agent"
        );
        assert_eq!(
            quoted_context("", Some(file), Some("plan.pdf")),
            Some(format!("Replying to a file: plan.pdf\n\n[Attached: {file}]"))
        );
        assert_eq!(
            quoted_context("   ", Some(file), None),
            Some(format!("[Attached: {file}]")),
            "a captionless quoted file is still named"
        );
        assert_eq!(quoted_context("", None, None), None, "an empty quote adds nothing");
        assert_eq!(quoted_context("  ", None, Some("")), None, "blank names are not shown");
    }

    #[test]
    fn a_group_with_no_name_is_named_in_the_agent_prompt() {
        // The prompt takes the same fallback as the chat list and history, so an agent never sees
        // an empty group name (OpenBot 0.74.0).
        let note = "Please review the PR";
        assert_eq!(
            prompt(super::super::place_name(None, true, "sender"), note),
            "WhatsApp message from WhatsApp group:\n\nPlease review the PR"
        );
        assert_eq!(
            prompt(super::super::place_name(Some("   "), true, "sender"), note),
            "WhatsApp message from WhatsApp group:\n\nPlease review the PR"
        );
        assert_eq!(
            prompt(super::super::place_name(Some("Family"), true, "sender"), note),
            "WhatsApp message from Family:\n\nPlease review the PR"
        );
        assert_eq!(
            prompt(super::super::place_name(None, false, "sender"), note),
            "WhatsApp message from sender:\n\nPlease review the PR"
        );
    }

    #[test]
    fn a_shared_contact_card_reaches_the_agent_with_its_name_and_numbers() {
        let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Ana García\r\nTEL;TYPE=CELL:+52 555 555 5555\r\n\
                     TEL;TYPE=WORK;VALUE=uri:tel:+525556666666\r\nEND:VCARD";
        let text = contact_card_text(vcard, "").unwrap();
        assert!(text.starts_with("Contact card: Ana García"), "{text}");
        assert!(text.contains("+525555555555"), "{text}");
        assert!(text.contains("+525556666666"), "{text}");
        assert_eq!(text.matches('+').count(), 2, "a number is not printed twice");
        assert!(!text.contains("BEGIN:VCARD"), "only the name and numbers are shown");

        // An unnamed card still says who sent it, and a repeated number is printed once.
        let unnamed = "BEGIN:VCARD\nTEL:+525551111111\nTEL:+52 555 111 1111\nEND:VCARD";
        let text = contact_card_text(unnamed, "  Pepe  ").unwrap();
        assert_eq!(text, "Contact card: Pepe — +525551111111");
        assert_eq!(contact_card_text("", ""), None, "an empty card is still empty");
        assert_eq!(contact_card_text("FN:Ana", "").unwrap(), "Contact card: Ana");
        assert_eq!(
            contact_card_text("TEL:+525551111111", "").unwrap(),
            "Contact card: Unnamed contact — +525551111111"
        );
    }

    #[test]
    fn a_group_reply_to_an_own_post_reaches_the_configured_bot() {
        let configured = Some("orchestrator");
        assert_eq!(route_reply(true, true, None, configured), configured, "a reply needs no mention");
        assert_eq!(
            route_reply(true, true, Some("named"), configured),
            configured,
            "the orchestrator sees the post's author"
        );
        assert_eq!(route_reply(true, false, Some("named"), configured), Some("named"), "an ordinary group @name wins");
        assert_eq!(route_reply(true, false, None, configured), configured, "an ordinary group line uses the default");
        assert_eq!(route_reply(false, false, None, configured), configured, "a direct chat uses the default");
        assert_eq!(route_reply(true, true, None, None), None, "a reply with no bot configured has no route");
        assert_eq!(route_reply(true, true, Some("named"), None), Some("named"), "a typed name still routes");
    }
}
