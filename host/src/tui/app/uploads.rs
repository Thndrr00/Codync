//! A draft file's way to the computer: its ring in the composer, its cancel, and its retry.
//!
//! Every file uploads as it is attached, under its own id, so one failure never takes the others
//! down and a retry sends only that file. Enter waits for the uploads before it sends the message.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::App;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UploadState {
    Uploading,
    Done,
    Failed(String),
    Cancelled,
}

/// One attached file on its way to the computer.
pub struct FileUpload {
    pub path: PathBuf,
    pub id: String,
    pub sent: u64,
    pub total: u64,
    pub state: UploadState,
    /// Set by a cancel: the task stops between chunks instead of sending another.
    pub cancel: Arc<AtomicBool>,
}

impl FileUpload {
    pub fn percent(&self) -> u64 {
        if self.total == 0 { 100 } else { (self.sent * 100 / self.total).min(100) }
    }

    pub fn failed(&self) -> bool {
        matches!(self.state, UploadState::Failed(_) | UploadState::Cancelled)
    }
}

impl App {
    /// A file just dropped on the composer: it uploads now, so the box shows what the computer has.
    pub fn start_upload(&mut self, key: &str, path: PathBuf) {
        let Some(bot) = self.bots.get(key).map(|bot| bot.id.clone()) else { return };
        let total = std::fs::metadata(&path).map_or(0, |m| m.len());
        let id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.uploads.entry(key.to_owned()).or_default().push(FileUpload {
            path: path.clone(),
            id: id.clone(),
            sent: 0,
            total,
            state: UploadState::Uploading,
            cancel: cancel.clone(),
        });
        self.client.spawn_upload(key.to_owned(), bot, path, id, cancel, self.tx.clone());
    }

    /// The uploads of a draft, in the order of its files.
    pub fn uploads_for(&self, key: &str) -> &[FileUpload] {
        self.uploads.get(key).map_or(&[], Vec::as_slice)
    }

    /// Stops a file's upload; the computer drops its partial chunks, and the file waits in the box.
    pub fn cancel_upload(&mut self, key: &str, path: &Path) {
        let Some(upload) = self.uploads.get_mut(key).and_then(|list| list.iter_mut().find(|u| u.path == path)) else {
            return;
        };
        if upload.state != UploadState::Uploading {
            return;
        }
        upload.cancel.store(true, Ordering::Relaxed);
        upload.state = UploadState::Cancelled;
        upload.sent = 0;
        let Some(bot) = self.bots.get(key).map(|bot| bot.id.clone()) else { return };
        let id = upload.id.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.call("cancelUpload", &serde_json::json!({"botId": bot, "uploadId": id})).await;
        });
    }

    /// Starts the first failed or cancelled file again, dropping the partial it left on the computer.
    pub fn retry_upload(&mut self, key: &str) {
        let Some((file_path, old_id, cancel)) = self.uploads.get(key).and_then(|list| {
            list.iter()
                .find(|upload| upload.failed())
                .map(|upload| (upload.path.clone(), upload.id.clone(), upload.cancel.clone()))
        }) else {
            return;
        };
        cancel.store(true, Ordering::Relaxed);
        let Some(bot) = self.bots.get(key).map(|bot| bot.id.clone()) else { return };
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.call("cancelUpload", &serde_json::json!({"botId": bot, "uploadId": old_id})).await;
        });
        if let Some(list) = self.uploads.get_mut(key) {
            list.retain(|upload| upload.path != file_path);
        }
        self.start_upload(key, file_path);
    }

    /// The file left the box: its upload stops, and what the computer has of it goes too.
    pub fn forget_upload(&mut self, key: &str, path: &Path) {
        let Some(upload) = self.uploads.get_mut(key).and_then(|list| {
            let at = list.iter().position(|u| u.path == path)?;
            Some(list.remove(at))
        }) else {
            return;
        };
        upload.cancel.store(true, Ordering::Relaxed);
        if upload.state != UploadState::Uploading {
            return;
        }
        let Some(bot) = self.bots.get(key).map(|bot| bot.id.clone()) else { return };
        let client = self.client.clone();
        tokio::spawn(async move {
            let _ = client.call("cancelUpload", &serde_json::json!({"botId": bot, "uploadId": upload.id})).await;
        });
    }

    /// The ids of a draft's files once every one reached the computer; `None` while one is on its
    /// way or stopped, which holds the send as the box says.
    pub fn uploaded_ids(&self, key: &str) -> Option<Vec<String>> {
        let uploads = self.uploads.get(key)?;
        let files = self.files.get(key)?;
        files
            .iter()
            .map(|path| {
                uploads
                    .iter()
                    .find(|upload| upload.path == *path && upload.state == UploadState::Done)
                    .map(|upload| upload.id.clone())
            })
            .collect()
    }

    /// Applies an upload message; false when it names a file that is no longer attached.
    pub fn on_upload(&mut self, key: &str, path: &Path, change: impl FnOnce(&mut FileUpload)) -> bool {
        let Some(upload) = self.uploads.get_mut(key).and_then(|list| list.iter_mut().find(|u| u.path == path)) else {
            return false;
        };
        change(upload);
        true
    }

    /// Keeps the draft's file list and its uploads in step after a file leaves.
    pub fn drop_last_file(&mut self, key: &str) -> Option<PathBuf> {
        let path = self.files.get_mut(key)?.pop()?;
        self.forget_upload(key, &path);
        Some(path)
    }
}
