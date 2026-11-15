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
    piper_model().is_some()
        || cfg!(target_os = "macos")
        || cfg!(target_os = "windows")
        || which("espeak-ng")
        || which("espeak")
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
    let result = if piper_model().is_some() && which("piper") {
        piper(text, &wav).await
    } else if cfg!(target_os = "macos") {
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

/// The Piper voice this computer was given: `CODYNC_PIPER_MODEL` or `<data>/voice/piper.onnx`.
fn piper_model() -> Option<PathBuf> {
    let configured = std::env::var_os("CODYNC_PIPER_MODEL").map(PathBuf::from);
    let default = crate::service::data_dir().join("voice").join("piper.onnx");
    configured.filter(|path| path.is_file()).or_else(|| default.is_file().then_some(default))
}

/// A neural voice when a Piper model and binary are installed; it reads the text on stdin.
async fn piper(text: &str, wav: &Path) -> Result<()> {
    let model = piper_model().ok_or_else(|| anyhow!("no Piper model"))?;
    let mut child = Command::new("piper")
        .args(["--model"])
        .arg(&model)
        .args(["--output_file"])
        .arg(wav)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt as _;
        stdin.write_all(text.as_bytes()).await?;
        stdin.shutdown().await?;
    }
    let output = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| anyhow!("the voice took too long"))??;
    if !output.status.success() {
        return Err(anyhow!("piper failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(())
}

/// Speech to text with whisper.cpp when a model is installed: `CODYNC_WHISPER_MODEL` or
/// `<data>/voice/ggml-base.bin`. Returns the transcript.
pub(super) async fn transcribe(wav: &[u8]) -> Result<String> {
    let model = std::env::var_os("CODYNC_WHISPER_MODEL")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            let default = crate::service::data_dir().join("voice").join("ggml-base.bin");
            default.is_file().then_some(default)
        })
        .ok_or_else(|| anyhow!("no whisper model: set CODYNC_WHISPER_MODEL or add voice/ggml-base.bin"))?;
    let binary = if which("whisper-cli") { "whisper-cli" } else { "whisper" };
    ensure_whisper(binary)?;
    let input = scratch("in.wav");
    let prefix = scratch("out");
    tokio::fs::write(&input, wav).await?;
    let output = tokio::time::timeout(
        Duration::from_mins(2),
        Command::new(binary)
            .args(["-m"])
            .arg(&model)
            .args(["-f"])
            .arg(&input)
            .args(["-otxt", "-of"])
            .arg(&prefix)
            .output(),
    )
    .await
    .map_err(|_| anyhow!("the transcription took too long"))??;
    let text = tokio::fs::read_to_string(prefix.with_extension("txt")).await;
    let _ = tokio::fs::remove_file(&input).await;
    let _ = tokio::fs::remove_file(prefix.with_extension("txt")).await;
    if !output.status.success() {
        return Err(anyhow!("{binary} failed: {}", String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(text?.trim().to_owned())
}

fn ensure_whisper(binary: &str) -> Result<()> {
    if which(binary) { Ok(()) } else { Err(anyhow!("no whisper.cpp binary on PATH (whisper-cli or whisper)")) }
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
