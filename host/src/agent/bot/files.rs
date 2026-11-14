//! File copies run outside the actor; publication still belongs to its original turn. Artifact cards post the same way.
//!
//! A picture or a clip that a media tool made is shared as a file too, with the kind and what was
//! asked for on its entry, so the chat shows the picture instead of a download
//! (see `crate::media`).
use super::Actor;
use crate::chat::files::{self, Snapshot};
use crate::media::{Kind, Output};
use crate::store::EntryKind;
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, oneshot};

type Reply = oneshot::Sender<Result<()>>;
/// The most a generated file may be, the same limit a sent file has.
#[allow(clippy::cast_possible_truncation)] // 32-bit hosts only lose the limit past 4 GiB.
const MAX_MEDIA: usize = crate::chat::uploads::MAX_FILE as usize;

pub(super) struct Completed {
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    reply: Reply,
    result: Result<Snapshot>,
    /// The kind of a generated file, `None` for a file the bot sent.
    media: Option<Kind>,
    /// What the person asked the media tool for, shown as the message.
    prompt: Option<String>,
}

pub(super) struct FileShares {
    epoch: u64,
    pub summary: Option<String>,
    pending: Vec<Arc<AtomicBool>>,
    /// The receivers of shares nobody waits for; they keep a reply from reading as cancelled.
    quiet: Vec<oneshot::Receiver<Result<()>>>,
    pub tx: mpsc::UnboundedSender<Completed>,
    pub rx: mpsc::UnboundedReceiver<Completed>,
}
impl Default for FileShares {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { epoch: 0, summary: None, pending: Vec::new(), quiet: Vec::new(), tx, rx }
    }
}
impl FileShares {
    pub fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        for flag in self.pending.drain(..) {
            flag.store(true, Ordering::Release);
        }
    }
}
impl Drop for FileShares {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl Actor {
    pub(super) fn begin_file(
        &mut self,
        source: String,
        name: Option<String>,
        cancelled: Arc<AtomicBool>,
        reply: Reply,
    ) {
        let epoch = self.files.epoch;
        self.begin_share(source, name, None, None, epoch, cancelled, reply);
    }

    /// A picture or a clip a media tool made: shared like `send_file`, with its kind and what was
    /// asked for, so the chat shows the picture instead of a download. A tool that leaves the
    /// bytes inline (Codex) has them written to the host's media folder first.
    pub(super) fn begin_media(&mut self, kind: Kind, output: Output, prompt: Option<String>) {
        // A finished one is dropped; the rest keep their send from reading as cancelled.
        self.files.quiet.retain_mut(|done| matches!(done.try_recv(), Err(oneshot::error::TryRecvError::Empty)));
        let name = crate::media::display_name(kind, output.path.as_deref(), 0);
        let source = if let Some(path) = output.path {
            path
        } else {
            let Some(data) = output.data else { return };
            // Small enough to decode here; only the copy leaves the actor.
            let bytes = match crate::media::decode_base64(&data, MAX_MEDIA) {
                Ok(bytes) => bytes,
                Err(error) => return self.media_failed(kind, &error.to_string()),
            };
            let folder = crate::service::data_dir().join("media");
            let extension = name.rsplit('.').next().unwrap_or("png").to_owned();
            let path = folder.join(format!("{}.{extension}", uuid::Uuid::new_v4()));
            let written = std::fs::create_dir_all(&folder).and_then(|()| std::fs::write(&path, bytes));
            if let Err(error) = written {
                let why = format!("it couldn't be saved: {error}");
                return self.media_failed(kind, &why);
            }
            path.to_string_lossy().into_owned()
        };
        let (reply, done) = oneshot::channel();
        self.files.quiet.push(done);
        let epoch = self.files.epoch;
        self.begin_share(source, Some(name), Some(kind), prompt, epoch, Arc::new(AtomicBool::new(false)), reply);
    }

    #[allow(clippy::too_many_arguments, reason = "the two extra facts only a media file has")]
    fn begin_share(
        &mut self,
        source: String,
        name: Option<String>,
        media: Option<Kind>,
        prompt: Option<String>,
        epoch: u64,
        cancelled: Arc<AtomicBool>,
        reply: Reply,
    ) {
        if let Err(error) = self.can_send_file() {
            let _ = reply.send(Err(error));
            return;
        }
        self.files.pending.retain(|flag| !flag.load(Ordering::Acquire));
        if self.files.pending.len() >= 4 {
            let _ = reply.send(Err(anyhow!("too many pending file shares")));
            return;
        }
        self.files.pending.push(cancelled.clone());
        let root = files::root(&self.cfg.id);
        let cwd = PathBuf::from(&self.cfg.cwd);
        let tx = self.files.tx.clone();
        tokio::spawn(async move {
            let flag = cancelled.clone();
            let result =
                tokio::task::spawn_blocking(move || files::snapshot(&root, &cwd, &source, name.as_deref(), &flag))
                    .await;
            let result = result.map_err(anyhow::Error::from).and_then(std::convert::identity);
            let _ = tx.send(Completed { epoch, cancelled, reply, result, media, prompt });
        });
    }

    fn can_send_file(&self) -> Result<i64> {
        ensure!(!self.stop_requested, "turn is stopping");
        ensure!(
            self.active_group.is_none() && self.active_request.is_none() && self.active_routine.is_none(),
            "send_file and artifact cards are only available in the bot's own chat"
        );
        self.turn.ok_or_else(|| anyhow!("no turn is running"))
    }

    /// A published artifact's card (`chat::artifacts`, its entry's `data`): a message of the turn, like a shared file.
    pub(super) fn post_artifact(&mut self, data: Value) -> Result<String> {
        let turn = self.can_send_file()?;
        let text = data["text"].as_str().unwrap_or_default().to_owned();
        crate::chat::artifacts::prepare_card(&self.hub, &self.cfg.id, &data, self.proactive_turn)?;
        self.close_seg();
        let entry = self.add(EntryKind::Agent, turn, data).ok_or_else(|| anyhow!("couldn't save the artifact card"))?;
        self.files.summary = Some(text);
        self.set_activity("Working…");
        Ok(entry.id)
    }

    pub(super) fn finish_file(&mut self, completed: Completed) {
        self.files.pending.retain(|flag| !Arc::ptr_eq(flag, &completed.cancelled));
        let result = (|| {
            ensure!(
                completed.epoch == self.files.epoch
                    && !completed.cancelled.load(Ordering::Acquire)
                    && !completed.reply.is_closed(),
                "file sharing cancelled"
            );
            let turn = self.can_send_file()?;
            ensure!(self.hub.store.bot(&self.cfg.id)?.is_some_and(|bot| !bot.deleted), "bot was deleted");
            let mut snapshot = completed.result?;
            let text = completed
                .prompt
                .filter(|prompt| !prompt.trim().is_empty())
                .unwrap_or_else(|| format!("{} ({} bytes)", snapshot.meta.name, snapshot.meta.size));
            let mut data = json!({"text": text, "final": true, "files": [snapshot.meta]});
            if let Some(kind) = completed.media {
                data["media"] = json!({"kind": kind.as_str()});
            }
            self.close_seg();
            self.add(EntryKind::Agent, turn, data).ok_or_else(|| anyhow!("couldn't save the file message"))?;
            snapshot.commit();
            self.files.summary = Some(text);
            self.set_activity("Working…");
            Ok(())
        })();
        // Nobody waits for a media share: the reason is told in the chat instead.
        let why = result.as_ref().err().map(|error| format!("{error:#}"));
        if let Some(why) = why
            && let Some(kind) = completed.media
            && !why.contains("cancelled")
        {
            self.media_failed(kind, &why);
        }
        let _ = completed.reply.send(result);
    }

    /// Tells the chat why a made picture or clip never got there; the turn carries on.
    fn media_failed(&self, kind: Kind, why: &str) {
        let text = format!("The {} couldn't be shown: {why}", kind.as_str());
        self.hub.add_entry(
            &self.lane,
            EntryKind::Notice,
            self.turn.unwrap_or(0),
            &json!({"text": text, "style": "error"}),
        );
    }
}
