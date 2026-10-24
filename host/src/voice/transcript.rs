use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Transcript {
    entries: Vec<Segment>,
    truncated: bool,
    interruptions: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Segment {
    role: Role,
    source: Source,
    text: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    Assistant,
}
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Source {
    Recognized,
    Generated,
    Queued,
}

impl Transcript {
    pub(super) fn parse(value: &Value) -> Result<Self> {
        let transcript: Self = serde_json::from_value(value.clone())?;
        if transcript.entries.len() > 96 || transcript.interruptions > 10000 {
            bail!("Call transcript exceeds its limits.");
        }
        let mut total = 0;
        for entry in &transcript.entries {
            let count = entry.text.chars().count();
            if count > 4000 || entry.text.trim().is_empty() || entry.text.contains('\0') {
                bail!("Invalid transcript text.");
            }
            if (entry.role == Role::User) != (entry.source == Source::Recognized) {
                bail!("Invalid transcript role/source combination.");
            }
            total += count;
        }
        if total > 32000 {
            bail!("Call transcript exceeds its text limit.");
        }
        Ok(transcript)
    }

    pub(super) fn display(&self) -> String {
        let mut text = format!(
            "Client-reported call text. Capture may be incomplete or out of order. Playback and decisions are unverified. Interruptions reported: {}.\n",
            self.interruptions
        );
        if self.truncated {
            text.push_str("Capture limit reached; some text was omitted.\n");
        }
        if self.entries.is_empty() {
            text.push_str("No transcript text was captured.\n");
        }
        for entry in &self.entries {
            let label = match entry.source {
                Source::Recognized => "User (recognized)",
                Source::Generated => "Assistant (provider text)",
                Source::Queued => "Assistant (queued for speech)",
            };
            text.push_str(&format!("\n{label}: {}\n", entry.text));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_authority_and_oversized_transcripts() {
        let mut value =
            json!({"entries":[{"role":"user","source":"recognized","text":"yes"}],"truncated":false,"interruptions":0});
        assert!(Transcript::parse(&value).is_ok());
        value["entries"][0]["source"] = json!("confirmed");
        assert!(Transcript::parse(&value).is_err());
        value["entries"][0]["source"] = json!("generated");
        assert!(Transcript::parse(&value).is_err());
        value["entries"][0]["source"] = json!("recognized");
        value["entries"][0]["text"] = json!("x".repeat(4001));
        assert!(Transcript::parse(&value).is_err());
    }
}
