//! The server's Library: every bot's artifacts and files, newest first, with the bot's name. Also the text preview
//! of a file (`readFile` or `readUpload`), since a terminal shows no pictures.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::super::app::{App, Overlay};
use super::super::net::artifacts::ArtifactTo;
use super::{LibraryItem, Reply};
use crate::tui::l10n::tr;

/// What the server's Library shows: everything, only images, or only other files.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ServerFilter {
    #[default]
    All,
    Images,
    Files,
}

impl ServerFilter {
    /// The filter `Tab` goes to next: All → Images → Files → All.
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Images,
            Self::Images => Self::Files,
            Self::Files => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Images => "Images",
            Self::Files => "Files",
        }
    }

    /// Whether an item shows under this filter; the host applies the same rule to `image`.
    pub fn matches(self, item: &ServerLibraryItem) -> bool {
        match self {
            Self::All => true,
            Self::Images => item.image,
            Self::Files => !item.image,
        }
    }
}

/// One thing in the server's Library (`serverLibrary`): a chat's item, with the bot that published or shared it.
#[derive(Clone, Debug)]
pub struct ServerLibraryItem {
    pub bot: String,
    pub bot_name: String,
    /// A picture the apps preview, as the host marks it: a terminal marks it and offers to save it.
    pub image: bool,
    pub item: LibraryItem,
}

impl ServerLibraryItem {
    fn parse(v: &Value) -> Option<Self> {
        Some(Self {
            bot: v["botId"].as_str()?.to_owned(),
            bot_name: v["botName"].as_str().unwrap_or_default().to_owned(),
            image: v["image"].as_bool() == Some(true),
            item: LibraryItem::parse(v)?,
        })
    }

    pub fn at(&self) -> i64 {
        self.item.at()
    }

    /// Its name and what it is, as a row reads it: the bot first, then the artifact's title or the file's name and size.
    pub fn label(&self) -> String {
        let kind = match &self.item {
            LibraryItem::Artifact(a) => format!("▤ {} · {}", a.title, tr("Artifact")),
            LibraryItem::File { file, .. } => {
                format!("▤ {} · {}", file.name, crate::tui::view::file_size(file.size))
            }
            LibraryItem::Attachment { name, size, .. } => {
                format!("▤ {name} · {}", crate::tui::view::file_size(*size))
            }
        };
        let mark =
            if self.image { format!("▣ {} · {}", tr("Image"), kind.trim_start_matches('▤').trim()) } else { kind };
        format!("{} · {mark}", self.bot_name)
    }
}

/// A file's text, shown in place of the list: what `readFile` or `readUpload` sent.
pub struct Preview {
    pub name: String,
    pub text: String,
}

/// The sheet: the host's list (once read), the filter, and an open preview.
#[derive(Default)]
pub struct ServerLibrary {
    pub filter: ServerFilter,
    pub items: Option<Vec<ServerLibraryItem>>,
    pub cursor: usize,
    pub preview: Option<Preview>,
    pub scroll: u16,
    pub error: Option<String>,
}

impl ServerLibrary {
    /// The rows the filter shows, newest first (the host's order).
    pub fn visible(&self) -> Vec<&ServerLibraryItem> {
        self.items.as_deref().unwrap_or_default().iter().filter(|item| self.filter.matches(item)).collect()
    }
}

impl App {
    pub fn open_server_library(&mut self) {
        self.overlays.push(Overlay::ServerLibrary(ServerLibrary::default()));
        self.sheet("serverLibrary", json!({}), Reply::ServerLibrary);
    }

    pub(in crate::tui) fn server_library_key(&mut self, mut s: ServerLibrary, k: KeyEvent) -> Option<Overlay> {
        if s.preview.is_some() {
            match k.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                    s.preview = None;
                    s.scroll = 0;
                }
                KeyCode::Down | KeyCode::Char('j') => s.scroll = s.scroll.saturating_add(1),
                KeyCode::Up | KeyCode::Char('k') => s.scroll = s.scroll.saturating_sub(1),
                _ => {}
            }
            return Some(Overlay::ServerLibrary(s));
        }
        let n = s.visible().len();
        let current = s.visible().get(s.cursor).map(|item| (**item).clone());
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Tab | KeyCode::BackTab => {
                s.filter = s.filter.next();
                s.cursor = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => s.cursor = (s.cursor + 1).min(n.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => s.cursor = s.cursor.saturating_sub(1),
            KeyCode::Enter | KeyCode::Char('o') => {
                if let Some(item) = current {
                    self.open_server_item(&mut s, item);
                }
            }
            KeyCode::Char('f') => {
                if let Some(item) = current {
                    self.save_server_item(item);
                }
            }
            KeyCode::Char('r') => {
                if let Some(ServerLibraryItem { bot, item: LibraryItem::Artifact(a), .. }) = current {
                    self.overlays.push(Overlay::ServerLibrary(s));
                    self.overlays.push(Overlay::Fields(Box::new(super::library::rename_fields(bot, a.id, &a.title))));
                    return None;
                }
                self.flash("Only artifacts can be renamed");
            }
            _ => {}
        }
        Some(Overlay::ServerLibrary(s))
    }

    /// Enter: an artifact opens in the browser, a file's text opens in the sheet, an image says how to save it.
    fn open_server_item(&mut self, sheet: &mut ServerLibrary, item: ServerLibraryItem) {
        match &item.item {
            LibraryItem::Artifact(a) => self.fetch_artifact(&item.bot, &a.id, ArtifactTo::Browser),
            LibraryItem::File { entry, file, .. } => {
                if item.image {
                    self.flash(&tr("An image. f saves it to Downloads."));
                    return;
                }
                sheet.preview = Some(Preview { name: file.name.clone(), text: tr("Loading…").into_owned() });
                let body = json!({"entryId": entry, "fileId": file.id, "offset": 0});
                self.call("readFile", body, super::super::app::After::Sheet(Reply::ServerLibraryPreview));
            }
            LibraryItem::Attachment { id, name, .. } => {
                if item.image {
                    self.flash(&tr("An image. f saves it to Downloads."));
                    return;
                }
                sheet.preview = Some(Preview { name: name.clone(), text: tr("Loading…").into_owned() });
                let body = json!({"botId": item.bot, "uploadId": id, "offset": 0});
                self.call("readUpload", body, super::super::app::After::Sheet(Reply::ServerLibraryPreview));
            }
        }
    }

    fn save_server_item(&mut self, item: ServerLibraryItem) {
        self.save_item(&item.bot, item.item);
    }

    /// A fetched server Library: shown, and its artifacts' titles kept for their cards.
    pub(in crate::tui) fn server_library_loaded(&mut self, items: &[Value]) {
        let items: Vec<ServerLibraryItem> = items.iter().filter_map(ServerLibraryItem::parse).collect();
        for item in &items {
            if let LibraryItem::Artifact(a) = &item.item {
                self.artifact_titles.titles.insert(a.id.clone(), a.title.clone());
            }
        }
        for o in &mut self.overlays {
            if let Overlay::ServerLibrary(s) = o {
                s.cursor = s.cursor.min(items.len().saturating_sub(1));
                s.items = Some(items.clone());
                s.error = None;
            }
        }
    }

    /// A file's first chunk as text: a terminal shows no pictures, and a binary file has no text to show.
    pub(in crate::tui) fn server_library_preview(&mut self, value: &Value) {
        let text = preview_text(value);
        for o in &mut self.overlays {
            if let Overlay::ServerLibrary(s) = o
                && let Some(preview) = &mut s.preview
            {
                preview.text = text.clone();
                s.scroll = 0;
            }
        }
    }
}

/// The first chunk of a `readFile` or `readUpload` answer as text, cut for the sheet; a binary file says so.
fn preview_text(value: &Value) -> String {
    use base64::Engine as _;
    let encoded = value["data"].as_str().unwrap_or_default();
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes);
    let text: String = text.chars().filter(|c| *c != '\0').collect();
    if text.chars().any(|c| c == '\u{FFFD}') {
        return tr("No text preview for this file. f saves it to Downloads.").into_owned();
    }
    text.chars().take(MAX_PREVIEW_CHARS).collect()
}

/// How much of a file a terminal preview keeps.
const MAX_PREVIEW_CHARS: usize = 128 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    fn item(image: bool) -> ServerLibraryItem {
        ServerLibraryItem {
            bot: "bot".into(),
            bot_name: "Writer".into(),
            image,
            item: LibraryItem::Artifact(
                serde_json::from_value(json!({"id": "a", "botId": "bot", "title": "Report", "size": 1,
                    "sha256": "", "createdAt": 0, "updatedAt": 1}))
                .expect("an artifact"),
            ),
        }
    }

    #[test]
    fn the_filter_keeps_images_and_files_apart() {
        assert!(ServerFilter::All.matches(&item(false)));
        assert!(ServerFilter::Images.matches(&item(true)));
        assert!(!ServerFilter::Images.matches(&item(false)));
        assert!(ServerFilter::Files.matches(&item(false)));
        assert_eq!(ServerFilter::All.next(), ServerFilter::Images);
        assert_eq!(ServerFilter::Files.next(), ServerFilter::All);
    }

    #[test]
    fn a_row_names_its_bot_and_marks_an_image() {
        assert!(item(false).label().starts_with("Writer · ▤ Report"));
        assert!(item(true).label().contains("▣ Image"));
    }

    #[test]
    fn a_binary_file_has_no_text_preview() {
        let value = json!({"data": "//7/", "size": 3});
        assert_eq!(preview_text(&value), tr("No text preview for this file. f saves it to Downloads.").into_owned());
    }
}
