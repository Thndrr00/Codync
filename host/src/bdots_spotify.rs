#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from coucou's `windows/src/core/spotify.ts`: player position, track kinds,
//! volume, playback rules and Mochi dance conditions. Only `botInsights` calls this.

use serde_json::{Value, json};

/// Spotify pill id (SPOTIFY_ID).
pub const SPOTIFY_ID: &str = "integration_spotify";

/// Spotify brand green (SPOTIFY_GREEN).
pub const SPOTIFY_GREEN: &str = "#1DB954";

/// States where Mochi dances (DANCE_STATES).
pub const DANCE_STATES: [&str; 5] = ["idle", "working", "thinking", "searching", "finished"];

/// Kinds of Spotify URI (track / episode / ad).
pub const TRACK_KINDS: [&str; 3] = ["track", "episode", "ad"];

/// Icons of the four volume levels.
pub const VOLUME_ICONS: [&str; 4] =
    ["speaker.slash.fill", "speaker.wave.1.fill", "speaker.wave.2.fill", "speaker.wave.3.fill"];

/// Default volume when nothing is playing (IDLE_SPOTIFY.volume).
pub const DEFAULT_VOLUME: u64 = 50;

/// Extrapolated position while playing, capped at the track end (spotifyPosition).
pub fn spotify_position(state: &Value, now_ms: i64) -> f64 {
    let elapsed = if state["playing"] == true {
        ((now_ms - state["positionAt"].as_i64().unwrap_or(0)) as f64 / 1_000.0).max(0.0)
    } else {
        0.0
    };
    let position = state["position"].as_f64().unwrap_or(0.0) + elapsed;
    let duration = state["track"]["duration"].as_f64().unwrap_or(0.0);
    if duration > 0.0 { position.min(duration) } else { position }
}

/// Freeze or restart the clock at its current position (withPlaying).
pub fn with_playing(state: &Value, playing: bool, now_ms: i64) -> Value {
    if state["playing"] == playing {
        return state.clone();
    }
    let mut next = state.clone();
    next["position"] = json!(spotify_position(state, now_ms));
    next["positionAt"] = json!(now_ms);
    next["playing"] = json!(playing);
    next
}

/// Whether a track is an ad (isAd).
pub fn is_ad(track: Option<&Value>) -> bool {
    track.and_then(|value| value["id"].as_str()).is_some_and(|id| id.starts_with("spotify:ad:"))
}

/// m:ss, or h:mm:ss past an hour (formatTime).
pub fn format_time(seconds: f64) -> String {
    let total = if seconds.is_finite() { seconds.max(0.0).floor() as u64 } else { 0 };
    let pad = |n: u64| format!("{n:02}");
    if total >= 3_600 {
        format!("{}:{}:{}", total / 3_600, pad((total % 3_600) / 60), pad(total % 60))
    } else {
        format!("{}:{}", total / 60, pad(total % 60))
    }
}

/// 0 = muted, then one, two or three waves (volumeLevel).
pub fn volume_level(volume: f64) -> u8 {
    if volume <= 0.0 {
        0
    } else if volume < 34.0 {
        1
    } else if volume < 67.0 {
        2
    } else {
        3
    }
}

/// Music playing on a declared Spotify pill (musicPlaying).
pub fn music_playing(state: &Value, active: &[String]) -> bool {
    state["playing"] == true && state["track"].is_object() && active.iter().any(|id| id == SPOTIFY_ID)
}

/// Cover that arrived for the current track (currentArtwork).
pub fn current_artwork(state: &Value, artwork: Option<&Value>) -> Option<String> {
    let art = artwork?;
    let url = state["track"]["artUrl"].as_str()?;
    (art["artUrl"].as_str() == Some(url)).then(|| art["dataUrl"].as_str().unwrap_or("").to_owned())
}

/// Whether the island Mochi dances (islandDances).
pub fn island_dances(music: bool, state: &str, mode: &str, view: &str, focus: Option<&str>) -> bool {
    if !music || !DANCE_STATES.contains(&state) {
        return false;
    }
    if mode == "compact" {
        return true;
    }
    mode == "expanded" && view == "overview" && focus == Some(SPOTIFY_ID)
}

/// Whether the desktop Mochi dances (desktopDances).
pub fn desktop_dances(music: bool, state: &str) -> bool {
    music && DANCE_STATES.contains(&state)
}

/// Kind of URI: ad, episode or track (trackKind).
pub fn track_kind(track: Option<&Value>) -> &'static str {
    let Some(id) = track.and_then(|value| value["id"].as_str()) else {
        return "none";
    };
    if id.starts_with("spotify:ad:") {
        "ad"
    } else if id.starts_with("spotify:episode:") {
        "episode"
    } else {
        "track"
    }
}

/// Whether the current item is a podcast episode (isEpisode).
pub fn is_episode(track: Option<&Value>) -> bool {
    track_kind(track) == "episode"
}

/// Playback progress from zero to one (progressFraction).
pub fn progress_fraction(state: &Value, now_ms: i64) -> f64 {
    let duration = state["track"]["duration"].as_f64().unwrap_or(0.0);
    if duration <= 0.0 {
        return 0.0;
    }
    (spotify_position(state, now_ms) / duration).clamp(0.0, 1.0)
}

/// Seconds left in the current track (remainingSeconds).
pub fn remaining_seconds(state: &Value, now_ms: i64) -> f64 {
    let duration = state["track"]["duration"].as_f64().unwrap_or(0.0);
    (duration - spotify_position(state, now_ms)).max(0.0)
}

/// SF Symbol for a volume level (volumeIcon).
pub fn volume_icon(volume: f64) -> &'static str {
    VOLUME_ICONS[volume_level(volume) as usize]
}

/// Change the volume in a state copy (withVolume).
pub fn with_volume(state: &Value, volume: f64) -> Value {
    let mut next = state.clone();
    next["volume"] = json!(volume.clamp(0.0, 100.0));
    next
}

/// Whether a track can be skipped: no ad, a track exists (canSkip).
pub fn can_skip(state: &Value) -> bool {
    state["track"].is_object() && !is_ad(Some(&state["track"]))
}

/// "Artist — Title" of a track, or an empty string (trackLine).
pub fn track_line(track: Option<&Value>) -> String {
    let Some(track) = track.filter(|value| value.is_object()) else {
        return String::new();
    };
    let artist = track["artist"].as_str().unwrap_or("");
    let title = track["title"].as_str().unwrap_or("");
    if artist.is_empty() { title.to_owned() } else { format!("{artist} — {title}") }
}

/// One line describing the player (stateLine).
pub fn state_line(state: &Value) -> String {
    let verb = if state["playing"] == true { "playing" } else { "paused" };
    format!(
        "{} · {} · {}",
        verb,
        format_time(spotify_position(state, state["positionAt"].as_i64().unwrap_or(0))),
        track_line(Some(&state["track"]))
    )
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(tracks: usize, dances: usize, volume: u8, playing: bool) -> Vec<(&'static str, String)> {
    vec![
        ("tracks", count_label(tracks)),
        ("dances", count_label(dances)),
        ("volume", format!("{volume}")),
        ("playing", if playing { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> Value {
        json!({
            "playing": true,
            "position": 10.0,
            "positionAt": 1_000,
            "volume": 50,
            "shuffle": false,
            "repeat": false,
            "track": {"id":"spotify:track:1","title":"Song","artist":"Band","duration":200.0,"artUrl":"cover"},
        })
    }

    #[test]
    fn constants_and_clock() {
        assert_eq!(SPOTIFY_ID, "integration_spotify");
        assert_eq!(SPOTIFY_GREEN, "#1DB954");
        assert_eq!(DANCE_STATES.len(), 5);
        assert_eq!(TRACK_KINDS.len(), 3);
        assert_eq!(VOLUME_ICONS.len(), 4);
        assert_eq!(DEFAULT_VOLUME, 50);
        assert_eq!(spotify_position(&state(), 11_000), 20.0);
        assert_eq!(spotify_position(&state(), 999_000), 200.0);
        let paused = with_playing(&state(), false, 11_000);
        assert_eq!(paused["position"], 20.0);
        assert_eq!(paused["positionAt"], 11_000);
        assert_eq!(paused["playing"], false);
        assert_eq!(with_playing(&paused, false, 20_000)["position"], 20.0);
    }

    #[test]
    fn tracks_and_time() {
        let track = state()["track"].clone();
        assert!(!is_ad(Some(&track)));
        assert!(!is_episode(Some(&track)));
        assert_eq!(track_kind(Some(&track)), "track");
        assert_eq!(track_kind(Some(&json!({"id":"spotify:ad:2"}))), "ad");
        assert_eq!(track_kind(Some(&json!({"id":"spotify:episode:3"}))), "episode");
        assert_eq!(track_kind(None), "none");
        assert!(can_skip(&state()));
        assert!(!can_skip(&json!({"track":{"id":"spotify:ad:1"}})));
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(95.9), "1:35");
        assert_eq!(format_time(3_725.0), "1:02:05");
        assert_eq!(format_time(f64::NAN), "0:00");
        assert_eq!(track_line(Some(&track)), "Band — Song");
        assert_eq!(track_line(None), "");
    }

    #[test]
    fn volume_and_dance() {
        assert_eq!(volume_level(0.0), 0);
        assert_eq!(volume_level(33.0), 1);
        assert_eq!(volume_level(50.0), 2);
        assert_eq!(volume_level(80.0), 3);
        assert_eq!(volume_icon(0.0), VOLUME_ICONS[0]);
        assert_eq!(with_volume(&state(), 120.0)["volume"], 100.0);
        assert_eq!(with_volume(&state(), -2.0)["volume"], 0.0);
        let active = vec![SPOTIFY_ID.to_owned()];
        assert!(music_playing(&state(), &active));
        assert!(!music_playing(&state(), &[]));
        assert!(island_dances(true, "working", "compact", "chat", None));
        assert!(island_dances(true, "idle", "expanded", "overview", Some(SPOTIFY_ID)));
        assert!(!island_dances(true, "error", "compact", "chat", None));
        assert!(desktop_dances(true, "thinking"));
        assert!(!desktop_dances(false, "thinking"));
    }

    #[test]
    fn artwork_and_presentation() {
        let art = json!({"artUrl":"cover","dataUrl":"data:image/png;base64,abc"});
        assert!(current_artwork(&state(), Some(&art)).is_some());
        assert!(current_artwork(&state(), Some(&json!({"artUrl":"other","dataUrl":"x"}))).is_none());
        assert!(current_artwork(&state(), None).is_none());
        assert_eq!(progress_fraction(&state(), 11_000), 0.1);
        assert_eq!(remaining_seconds(&state(), 11_000), 180.0);
        assert!(state_line(&state()).contains("Band — Song"));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(1, 5, 2, true).len(), 4);
    }
}
