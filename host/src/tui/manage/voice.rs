//! The voice providers' keys, as a terminal sheet: which providers this computer holds a key for,
//! and what each one does. Keys are added and removed here; models and voices are picked per device
//! in the apps, so this sheet never guesses them.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::{Value, json};

use super::super::app::{App, Confirm, ConfirmAct, Overlay};
use super::{Reply, voice_key_fields};

/// One provider, as `voiceStatus` reports it.
pub struct VoiceProviderRow {
    pub id: String,
    pub name: String,
    pub configured: bool,
    /// The English source line of what it does (`Calls and dictation`, `Reads replies aloud`, `Dictation only`).
    pub role: &'static str,
}

/// The providers and their keys. Opened from Go to → Voice providers.
pub struct VoiceSheet {
    /// `None` until `voiceStatus` answers.
    pub providers: Option<Vec<VoiceProviderRow>>,
    pub cursor: usize,
    pub error: Option<String>,
}

impl VoiceSheet {
    pub fn new() -> Self {
        Self { providers: None, cursor: 0, error: None }
    }

    pub fn rows(&self) -> &[VoiceProviderRow] {
        self.providers.as_deref().unwrap_or_default()
    }

    pub fn current(&self) -> Option<&VoiceProviderRow> {
        self.rows().get(self.cursor)
    }

    /// Takes `voiceStatus`'s answer, keeping the cursor on a row that still exists.
    pub fn fill(&mut self, value: &Value) {
        let providers = value["providers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let id = entry["provider"].as_str()?.to_owned();
                Some(VoiceProviderRow {
                    name: provider_name(&id).to_owned(),
                    role: provider_role(&id),
                    configured: entry["configured"].as_bool().unwrap_or(false),
                    id,
                })
            })
            .collect::<Vec<_>>();
        self.cursor = self.cursor.min(providers.len().saturating_sub(1));
        self.providers = Some(providers);
        self.error = None;
    }
}

/// The product names, which are not translated.
fn provider_name(id: &str) -> &str {
    match id {
        "openai" => "OpenAI",
        "gemini" => "Gemini",
        "elevenlabs" => "ElevenLabs",
        "xai" => "xAI",
        "groq" => "Groq",
        other => other,
    }
}

/// What a provider does in a call: both halves, replies only (xAI), or hearing only (Groq).
fn provider_role(id: &str) -> &'static str {
    match id {
        "xai" => "Reads replies aloud",
        "groq" => "Dictation only",
        _ => "Calls and dictation",
    }
}

impl App {
    pub(in crate::tui) fn open_voice(&mut self) {
        self.overlays.push(Overlay::Voice(Box::new(VoiceSheet::new())));
        self.sheet("voiceStatus", json!({}), Reply::VoiceStatus);
    }

    pub(in crate::tui) fn voice_key(&mut self, mut sheet: Box<VoiceSheet>, k: KeyEvent) -> Option<Overlay> {
        let len = sheet.rows().len();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return None,
            KeyCode::Down | KeyCode::Char('j') => sheet.cursor = (sheet.cursor + 1).min(len.saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => sheet.cursor = sheet.cursor.saturating_sub(1),
            KeyCode::Enter => {
                if let Some(row) = sheet.current() {
                    let fields = voice_key_fields(&row.id, &row.name);
                    self.overlays.push(Overlay::Fields(Box::new(fields)));
                }
            }
            KeyCode::Char('d' | 'x') => {
                if let Some(row) = sheet.current().filter(|row| row.configured) {
                    let confirm = Confirm {
                        title: format!("Remove the {} key?", row.name),
                        detail: "Voice calls and dictation with it stop until a key is added again.".into(),
                        note: "Nothing else on this computer changes.".into(),
                        button: "Remove key",
                        act: ConfirmAct::RemoveVoiceKey { provider: row.id.clone() },
                    };
                    self.overlays.push(Overlay::Confirm(confirm));
                }
            }
            // Loads again, after a key was added on another device.
            KeyCode::Char('r') => {
                sheet.providers = None;
                self.sheet("voiceStatus", json!({}), Reply::VoiceStatus);
            }
            _ => {}
        }
        Some(Overlay::Voice(sheet))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheet_names_each_providers_half_of_a_call() {
        let mut sheet = VoiceSheet::new();
        sheet.fill(&json!({"providers": [
            {"provider": "openai", "configured": true},
            {"provider": "xai", "configured": false},
            {"provider": "groq", "configured": true},
            {"provider": "future", "configured": false}
        ]}));
        let rows = sheet.rows();
        assert_eq!(rows.len(), 4);
        assert_eq!((rows[0].name.as_str(), rows[0].role, rows[0].configured), ("OpenAI", "Calls and dictation", true));
        assert_eq!(rows[1].role, "Reads replies aloud");
        assert_eq!(rows[2].role, "Dictation only");
        // A provider a newer computer adds still shows, by its id.
        assert_eq!((rows[3].name.as_str(), rows[3].role), ("future", "Calls and dictation"));
    }

    #[test]
    fn a_shorter_answer_keeps_the_cursor_on_a_row() {
        let mut sheet = VoiceSheet::new();
        sheet.cursor = 4;
        sheet.fill(&json!({"providers": [{"provider": "groq"}]}));
        assert_eq!(sheet.cursor, 0);
        assert!(sheet.current().is_some());
    }
}
