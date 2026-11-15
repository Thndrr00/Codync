//! Voice without a provider key: the computer's own speech engine. macOS speaks with `say` and
//! converts to WAV with `afconvert`; Windows uses System.Speech through PowerShell; Linux uses
//! `espeak-ng` (or `espeak`) when it is installed. The phones play the WAV like any cloud voice.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

const TIMEOUT: Duration = Duration::from_secs(60);

pub(super) struct Audio {
    pub bytes: Vec<u8>,
    pub format: &'static str,
}

/// Whether this computer can speak on its own.
pub fn available() -> bool {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return true;
    }
    which("espeak-ng") || which("espeak")
}

fn which(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            let candidate = dir.join(program);
            candidate.is_file() || candidate.with_extension("exe").is_file()
        })
    })
}

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("codync-speak-{}-{name}", uuid::Uuid::new_v4().simple()))
}

pub(super) async fn speak(text: &str) -> Result<Audio> {
    let text = text.trim();
    if text.is_empty() {
        return Err(anyhow!("nothing to say"));
    }
    let wav = scratch("out.wav");
    let result = if cfg!(target_os = "macos") {
        macos(text, &wav).await
    } else if cfg!(target_os = "windows") {
        windows(text, &wav).await
    } else {
        linux(text, &wav).await
    };
    let bytes = tokio::fs::read(&wav).await;
    let _ = tokio::fs::remove_file(&wav).await;
    result?;
    Ok(Audio { bytes: bytes.map_err(|e| anyhow!("the voice engine left no audio: {e}"))?, format: "wav" })
}

async fn macos(text: &str, wav: &Path) -> Result<()> {
    let aiff = scratch("say.aiff");
    let said = Command::new("say").args(["-o"]).arg(&aiff).args(["--data-format=LEF32@22050", "--", text]).output();
    let output = tokio::time::timeout(TIMEOUT, said).await.map_err(|_| anyhow!("the voice took too long"))??;
    if !output.status.success() {
        let _ = tokio::fs::remove_file(&aiff).await;
        return Err(anyhow!("say failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    let converted = Command::new("afconvert").args(["-f", "WAVE", "-d", "LEI16"]).arg(&aiff).arg(wav).output().await;
    let _ = tokio::fs::remove_file(&aiff).await;
    let output = converted?;
    if !output.status.success() {
        return Err(anyhow!("afconvert failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(())
}

async fn windows(text: &str, wav: &Path) -> Result<()> {
    // The text goes through a file: it can hold quotes, newlines and non-Latin scripts.
    let source = scratch("text.txt");
    tokio::fs::write(&source, text).await?;
    let quote = |path: &Path| path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.SetOutputToWaveFile('{}'); $s.Speak([IO.File]::ReadAllText('{}')); $s.Dispose()",
        quote(wav),
        quote(&source)
    );
    let output = tokio::time::timeout(
        TIMEOUT,
        Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]).output(),
    )
    .await
    .map_err(|_| anyhow!("the voice took too long"))??;
    let _ = tokio::fs::remove_file(&source).await;
    if !output.status.success() {
        return Err(anyhow!("the system voice failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(())
}

async fn linux(text: &str, wav: &Path) -> Result<()> {
    let engine = if which("espeak-ng") { "espeak-ng" } else { "espeak" };
    let output = tokio::time::timeout(TIMEOUT, Command::new(engine).args(["-w"]).arg(wav).args(["--", text]).output())
        .await
        .map_err(|_| anyhow!("the voice took too long"))??;
    if !output.status.success() {
        return Err(anyhow!("{engine} failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(())
}
