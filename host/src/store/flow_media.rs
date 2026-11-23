//! The picture and clip steps of routine flows, ported from `OpenBot` 0.70.0's media workflow steps:
//! besides bots, a flow hands work to a node that makes an image or a video with the agent's own
//! tool (`crate::media`). A node's id is `media:<id>`, which is also what the flow links name.
//!
//! Every mutation stamps a rev for the routine's audience, the same way a link does.

use super::flows::{audience, touch};
use super::{Store, now_ms};
use anyhow::{Result, bail, ensure};
use rusqlite::{Connection, Transaction, params};
use serde::Serialize;

/// The most of a node's description that is kept.
pub const MAX_PROMPT_BYTES: usize = 8_000;

/// One picture or clip step of a routine's flow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowMedia {
    pub id: String,
    /// A name the person gave this step, shown on the card and to agents reading the flow. Empty
    /// means "no name": a client falls back to the step's kind. Old steps have no column value.
    #[serde(default)]
    pub name: String,
    /// What the clients show as the node's title: the picture step's own words.
    pub kind: String,
    /// The media model (`media::catalog`), which also names the agent backend that makes the file.
    pub model: String,
    pub prompt: String,
    pub aspect: String,
    /// How many results the step makes, 1 to 4. Older steps read as 1.
    #[serde(default = "crate::media::default_batch")]
    pub batch: u32,
    /// Seconds; only a video uses it.
    pub duration: u32,
    /// Only a video uses it.
    pub resolution: String,
    pub transparent: bool,
    pub voice: Option<String>,
    pub created_at: i64,
    pub routine_id: String,
    #[serde(skip)]
    pub owner_bot_id: String,
}

/// The prefix of a media node's key, which is also what the flow links name.
pub const PREFIX: &str = "media:";

/// The node key a media step answers to on a canvas.
pub fn key(id: &str) -> String {
    format!("{PREFIX}{id}")
}

fn row_media(r: &rusqlite::Row) -> rusqlite::Result<FlowMedia> {
    Ok(FlowMedia {
        id: r.get(0)?,
        kind: r.get(1)?,
        model: r.get(2)?,
        prompt: r.get(3)?,
        aspect: r.get(4)?,
        duration: r.get(5)?,
        resolution: r.get(6)?,
        transparent: r.get(7)?,
        voice: r.get(8)?,
        created_at: r.get(9)?,
        routine_id: r.get(10)?,
        owner_bot_id: r.get(11)?,
        name: r.get(12)?,
        batch: r.get(13)?,
    })
}

const COLUMNS: &str = "id, kind, model, prompt, aspect, duration, resolution, transparent, voice, created_at, routine_id, owner_bot_id, name, batch";

fn media_of(c: &Connection, routine_id: &str) -> Result<Vec<FlowMedia>> {
    let mut st =
        c.prepare(&format!("SELECT {COLUMNS} FROM routine_flow_media WHERE routine_id = ?1 ORDER BY created_at, id"))?;
    Ok(st.query_map([routine_id], row_media)?.collect::<rusqlite::Result<_>>()?)
}

/// Takes every media node of a routine off its canvas, with the links that name them.
pub(super) fn forget_routine(tx: &Transaction, routine_id: &str) -> Result<usize> {
    let mut gone = 0;
    for media in media_of(tx, routine_id)? {
        gone += forget_node(tx, &media)?;
    }
    Ok(gone)
}

/// Takes every media node a bot owns off its routine.
pub(super) fn forget_owner(tx: &Transaction, owner: &str) -> Result<usize> {
    let media: Vec<FlowMedia> = {
        let mut st = tx.prepare(&format!("SELECT {COLUMNS} FROM routine_flow_media WHERE owner_bot_id = ?1"))?;
        st.query_map([owner], row_media)?.collect::<rusqlite::Result<_>>()?
    };
    let mut gone = 0;
    for media in &media {
        gone += forget_node(tx, media)?;
    }
    Ok(gone)
}

fn forget_node(tx: &Transaction, media: &FlowMedia) -> Result<usize> {
    let key = key(&media.id);
    let mut gone = tx.execute(
        "DELETE FROM routine_flow_links WHERE routine_id = ?1 AND (from_bot_id = ?2 OR to_bot_id = ?2)",
        params![media.routine_id, key],
    )?;
    gone += tx.execute("DELETE FROM routine_flow_positions WHERE node_key = ?1", [&key])?;
    gone += tx.execute("DELETE FROM routine_flow_media WHERE id = ?1", [&media.id])?;
    Ok(gone)
}

fn check_prompt(prompt: &str) -> Result<()> {
    if prompt.len() > MAX_PROMPT_BYTES {
        bail!("the description must be at most {MAX_PROMPT_BYTES} bytes");
    }
    Ok(())
}

/// A write through the store refuses an out-of-range batch; a request from a client is bounded
/// before it gets here.
fn check_batch(batch: u32) -> Result<()> {
    ensure!((1..=crate::media::MAX_BATCH).contains(&batch), "the batch must be 1 to 4");
    Ok(())
}

impl Store {
    /// The media nodes of a routine.
    pub fn flow_media(&self, routine_id: &str) -> Result<Vec<FlowMedia>> {
        self.with_connection(|c| media_of(c, routine_id))
    }

    pub fn flow_media_node(&self, id: &str) -> Result<Option<FlowMedia>> {
        self.with_connection(|c| {
            let mut st = c.prepare(&format!("SELECT {COLUMNS} FROM routine_flow_media WHERE id = ?1"))?;
            Ok(st.query_map([id], row_media)?.next().transpose()?)
        })
    }

    pub fn flow_media_for(&self, routine_ids: &[String]) -> Result<Vec<FlowMedia>> {
        let mut all = vec![];
        for id in routine_ids {
            all.extend(self.flow_media(id)?);
        }
        Ok(all)
    }

    pub fn flow_add_media(&self, media: &FlowMedia) -> Result<super::flows::FlowChange> {
        check_prompt(&media.prompt)?;
        check_batch(media.batch)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            tx.execute(
                &format!("INSERT INTO routine_flow_media({COLUMNS}) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"),
                params![
                    media.id,
                    media.kind,
                    media.model,
                    media.prompt,
                    media.aspect,
                    media.duration,
                    media.resolution,
                    media.transparent,
                    media.voice,
                    media.created_at,
                    media.routine_id,
                    media.owner_bot_id,
                    media.name,
                    media.batch
                ],
            )?;
            let bots = audience(&tx, &media.routine_id, &media.owner_bot_id)?;
            let change = touch(&tx, bots)?.expect("the owner is always in the audience");
            tx.commit()?;
            Ok(change)
        })
    }

    /// Rewrites a node's settings; the kind never changes.
    pub fn flow_update_media(&self, media: &FlowMedia) -> Result<Option<super::flows::FlowChange>> {
        check_prompt(&media.prompt)?;
        check_batch(media.batch)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let gone = tx.execute(
                "UPDATE routine_flow_media SET model = ?2, prompt = ?3, aspect = ?4, duration = ?5,
                 resolution = ?6, transparent = ?7, voice = ?8, batch = ?9 WHERE id = ?1",
                params![
                    media.id,
                    media.model,
                    media.prompt,
                    media.aspect,
                    media.duration,
                    media.resolution,
                    media.transparent,
                    media.voice,
                    media.batch
                ],
            )?;
            if gone == 0 {
                return Ok(None);
            }
            let bots = audience(&tx, &media.routine_id, &media.owner_bot_id)?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok(change)
        })
    }

    /// Gives a media step a name; an empty one clears it. Its settings are left alone.
    pub fn flow_rename_media(&self, id: &str, name: &str) -> Result<Option<super::flows::FlowChange>> {
        super::flows::check_name(name)?;
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let gone = tx.execute("UPDATE routine_flow_media SET name = ?2 WHERE id = ?1", params![id, name])?;
            if gone == 0 {
                return Ok(None);
            }
            let Some(media) = media_of_id(&tx, id)? else { return Ok(None) };
            let bots = audience(&tx, &media.routine_id, &media.owner_bot_id)?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            Ok(change)
        })
    }

    /// Takes a node off the flow with its links, its place on every canvas and its stored runs.
    pub fn flow_remove_media(&self, id: &str) -> Result<Option<super::flows::FlowChange>> {
        self.with_connection(|c| {
            let tx = c.unchecked_transaction()?;
            let Some(media) = media_of_id(&tx, id)? else { return Ok(None) };
            let bots = audience(&tx, &media.routine_id, &media.owner_bot_id)?;
            let gone = forget_node(&tx, &media)?;
            tx.execute("DELETE FROM routine_flow_steps WHERE bot_id = ?1", [key(id)])?;
            let change = touch(&tx, bots)?;
            tx.commit()?;
            debug_assert!(gone > 0);
            Ok(change)
        })
    }

    /// A media node's own row for its routine; `None` when it belongs to another one.
    pub fn flow_media_in(&self, id: &str, routine_id: &str) -> Result<Option<FlowMedia>> {
        Ok(self.flow_media_node(id)?.filter(|media| media.routine_id == routine_id))
    }
}

fn media_of_id(c: &Connection, id: &str) -> Result<Option<FlowMedia>> {
    let mut st = c.prepare(&format!("SELECT {COLUMNS} FROM routine_flow_media WHERE id = ?1"))?;
    Ok(st.query_map([id], row_media)?.next().transpose()?)
}

/// A node as it is made: the id and the times are the store's.
pub fn new_media(id: String, routine_id: String, owner_bot_id: String) -> FlowMedia {
    FlowMedia {
        id,
        name: String::new(),
        kind: "image".into(),
        model: String::new(),
        prompt: String::new(),
        aspect: "1:1".into(),
        batch: crate::media::DEFAULT_BATCH,
        duration: 6,
        resolution: "480p".into(),
        transparent: false,
        voice: None,
        created_at: now_ms(),
        routine_id,
        owner_bot_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_media_step_name_is_saved_and_can_be_cleared() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let mut media = new_media("m1".into(), "r".into(), "a".into());
        media.model = "gpt-image".into();
        store.flow_add_media(&media).expect("the step is added");
        assert_eq!(store.flow_media_node("m1").unwrap().unwrap().name, "", "a fresh step is unnamed");
        store.flow_rename_media("m1", "Hero shot").unwrap();
        assert_eq!(store.flow_media_node("m1").unwrap().unwrap().name, "Hero shot");
        assert_eq!(store.flow_media_node("m1").unwrap().unwrap().model, "gpt-image", "settings are untouched");
        store.flow_rename_media("m1", "").unwrap();
        assert_eq!(store.flow_media_node("m1").unwrap().unwrap().name, "");
    }

    #[test]
    fn a_step_batch_is_saved_and_an_out_of_range_one_is_refused() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("a store");
        let mut media = new_media("m2".into(), "r".into(), "a".into());
        media.model = "gpt-image".into();
        assert_eq!(media.batch, 1, "a new step makes one result");
        store.flow_add_media(&media).expect("the step is added");
        media.batch = 4;
        media.prompt = "a fox".into();
        store.flow_update_media(&media).expect("the batch is saved");
        assert_eq!(store.flow_media_node("m2").unwrap().unwrap().batch, 4);
        media.batch = 5;
        assert!(store.flow_update_media(&media).is_err());
        let mut other = new_media("m3".into(), "r".into(), "a".into());
        other.model = "gpt-image".into();
        other.batch = 0;
        assert!(store.flow_add_media(&other).is_err());
    }
}
