//! A draft file's way to the computer: its percentage in the composer, its cancel, and its retry.
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
        if self.total == 0 { 100 } else { (self.sent.saturating_mul(100) / self.total).min(100) }
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
        let Some((path, old_id, cancel)) = self.uploads.get(key).and_then(|list| {
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
            list.retain(|upload| upload.path != path);
        }
        self.start_upload(key, path);
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
    pub fn drop_last_file(&mut self, key: &str) {
        let Some(path) = self.files.get_mut(key).and_then(Vec::pop) else { return };
        self.forget_upload(key, &path);
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_app;
    use super::*;

    fn upload(path: &str, state: UploadState) -> FileUpload {
        FileUpload {
            path: path.into(),
            id: format!("upload-{path}"),
            sent: 0,
            total: 100,
            state,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn a_file_reports_how_much_the_computer_has() {
        let mut file = upload("a.txt", UploadState::Uploading);
        assert_eq!(file.percent(), 0);
        file.sent = 42;
        assert_eq!(file.percent(), 42);
        file.sent = 100;
        assert_eq!(file.percent(), 100);
        file.total = 0;
        assert_eq!(file.percent(), 100, "an unknown size isn't a wedge");
        assert!(!file.failed());
        file.state = UploadState::Failed("the link dropped".into());
        assert!(file.failed());
        file.state = UploadState::Cancelled;
        assert!(file.failed());
    }

    #[test]
    fn a_send_waits_for_every_file() {
        let mut app = test_app();
        let (a, b) = (PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt"));
        app.files.insert("bot".into(), vec![a, b]);
        app.uploads.insert("bot".into(), vec![upload("/tmp/a.txt", UploadState::Done)]);
        assert_eq!(app.uploaded_ids("bot"), None, "b is still on its way");
        app.uploads.get_mut("bot").unwrap().push(upload("/tmp/b.txt", UploadState::Failed("no".into())));
        assert_eq!(app.uploaded_ids("bot"), None, "a stopped file holds the send too");
        app.uploads.get_mut("bot").unwrap()[1].state = UploadState::Done;
        assert_eq!(app.uploaded_ids("bot"), Some(vec!["upload-/tmp/a.txt".into(), "upload-/tmp/b.txt".into()]));
        assert_eq!(app.uploaded_ids("elsewhere"), None, "a draft with no uploads sends no attachments");
    }

    #[test]
    fn removing_a_file_forgets_its_upload() {
        let mut app = test_app();
        let (a, b) = (PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt"));
        app.files.insert("bot".into(), vec![a, b]);
        app.uploads.insert(
            "bot".into(),
            vec![upload("/tmp/a.txt", UploadState::Done), upload("/tmp/b.txt", UploadState::Done)],
        );
        app.drop_last_file("bot");
        assert_eq!(app.files["bot"].len(), 1);
        assert_eq!(app.uploads["bot"].len(), 1);
        assert_eq!(app.uploads["bot"][0].path, PathBuf::from("/tmp/a.txt"));
    }
}
