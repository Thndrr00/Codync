//! A module's manifest: what it is, what it may do, and the signature that ties it to its files.
//!
//! A module is a folder with `module.json` (this manifest), `module.sig` (an Ed25519 signature over the manifest's
//! canonical bytes) and the files the manifest lists, each with its SHA-256. The host checks both before it runs
//! anything: a file the signature doesn't cover is a file nobody signed.

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The manifest's format word and the bundle's, so a wrong file says so.
pub const FORMAT: &str = "codync-module";
pub const BUNDLE: &str = "codync-module-bundle";
/// The contract this host speaks. A module built for a newer one is refused, never guessed at.
pub const CONTRACT: u32 = 1;
pub const MANIFEST: &str = "module.json";
pub const SIGNATURE: &str = "module.sig";
/// A module's own writable folder, kept beside its files and never signed.
pub const DATA: &str = "data";
pub const MAX_FILES: usize = 200;
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// What the host tells a module about, by name. A module asks for these; nothing else reaches it.
pub const HOST_EVENTS: [&str; 2] = ["messageAdded", "botStatus"];

/// One module, as its author writes it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub contract: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    /// An SF Symbol the apps draw beside the name.
    #[serde(default)]
    pub symbol: Option<String>,
    pub entry: Entry,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub limits: Limits,
    /// The tools the module offers a bot, with their schemas: listed without waking it.
    #[serde(default)]
    pub tools: Vec<Tool>,
    /// The screens the module draws, by id.
    #[serde(default)]
    pub views: Vec<View>,
    /// Every file in the folder, with its hash. `module.json` and `module.sig` aren't listed.
    pub files: Vec<FileHash>,
    pub publisher: Publisher,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    /// The program to run, resolved on `PATH` when it isn't a path.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Permissions {
    /// Tools the bot may call, by name.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Host events it may hear, from [`HOST_EVENTS`].
    #[serde(default)]
    pub events: Vec<String>,
    /// Hosts it may reach. Declared and shown; not firewalled.
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub files: Files,
}

/// How much of the bot's folder a module may read or write. Shown and passed as a path; the module runs as you.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Files {
    #[default]
    None,
    Read,
    Write,
}

impl Files {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

/// What one call may cost. The host holds every module to these, whatever the manifest asks.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_calls")]
    pub calls_per_minute: u32,
    #[serde(default = "default_time")]
    pub time_ms: u64,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
}

fn default_calls() -> u32 {
    60
}
fn default_time() -> u64 {
    20_000
}
fn default_memory() -> u32 {
    256
}

impl Default for Limits {
    fn default() -> Self {
        Self { calls_per_minute: default_calls(), time_ms: default_time(), memory_mb: default_memory() }
    }
}

impl Limits {
    /// The manifest's numbers within what the host allows: a module can ask for less, never for more.
    pub fn clamped(&self) -> Self {
        Self {
            calls_per_minute: self.calls_per_minute.clamp(1, 600),
            time_ms: self.time_ms.clamp(1_000, 120_000),
            memory_mb: self.memory_mb.clamp(16, 2_048),
        }
    }

    pub fn time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.time_ms)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The JSON schema of its arguments, as MCP wants it.
    #[serde(default = "object_schema")]
    pub input_schema: serde_json::Value,
}

fn object_schema() -> serde_json::Value {
    serde_json::json!({"type": "object", "properties": {}})
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileHash {
    /// Relative to the module's folder, with `/` separators.
    pub path: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Publisher {
    pub name: String,
    /// The Ed25519 public key the signature was made with, base64.
    pub key: String,
}

impl Manifest {
    /// Reads, checks and verifies the module in `dir`. A file the signature doesn't cover, a missing file or a
    /// newer contract stops it here.
    pub fn read(dir: &Path) -> Result<Self> {
        let bytes = std::fs::read(dir.join(MANIFEST)).context("reading module.json")?;
        let signature = std::fs::read_to_string(dir.join(SIGNATURE)).context("reading module.sig")?;
        let manifest: Self = serde_json::from_slice(&bytes).context("reading module.json")?;
        manifest.check_contract()?;
        manifest.validate()?;
        manifest.verify(dir, &signature)?;
        Ok(manifest)
    }

    /// The bytes a signature covers: the manifest written the one way this host writes it.
    pub fn canonical(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// The signature for a key, base64, as `module.sig` holds it.
    pub fn sign(&self, key: &SigningKey) -> Result<String> {
        Ok(B64.encode(key.sign(&self.canonical()?).to_bytes()))
    }

    /// A newer module says so plainly instead of failing somewhere deeper.
    pub fn check_contract(&self) -> Result<()> {
        ensure!(self.format == FORMAT, "this is not a Codync module (`format` is `{}`)", self.format);
        if self.contract > CONTRACT {
            bail!("{} needs a newer Codync (contract {}; this one speaks {CONTRACT})", self.name, self.contract);
        }
        ensure!(self.contract == CONTRACT, "{} speaks an unknown module contract {}", self.name, self.contract);
        Ok(())
    }

    /// What can be checked without touching the disk.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.id.is_empty()
                && self.id.len() <= 96
                && self.id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')),
            "a module id is letters, digits, `.`, `-` and `_`"
        );
        ensure!(!self.name.trim().is_empty() && self.name.len() <= 60, "a module needs a name of up to 60 characters");
        semver::Version::parse(&self.version).context("a module's version is semver (1.2.0)")?;
        ensure!(!self.entry.command.trim().is_empty(), "a module needs a command to run");
        ensure!(self.files.len() <= MAX_FILES, "a module holds at most {MAX_FILES} files");
        let mut seen = BTreeSet::new();
        for file in &self.files {
            ensure!(!file.path.is_empty() && !file.path.starts_with('/'), "a file path is relative");
            ensure!(
                Path::new(&file.path).components().all(|c| matches!(c, std::path::Component::Normal(_))),
                "`{}` leaves the module's folder",
                file.path
            );
            ensure!(seen.insert(file.path.as_str()), "`{}` is listed twice", file.path);
            ensure!(file.sha256.len() == 64, "`{}` needs a SHA-256", file.path);
        }
        let mut tools = BTreeSet::new();
        for tool in &self.tools {
            ensure!(
                !tool.name.is_empty()
                    && tool.name.len() <= 64
                    && tool.name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')),
                "`{}` is not a tool name",
                tool.name
            );
            ensure!(tools.insert(tool.name.as_str()), "`{}` is declared twice", tool.name);
            ensure!(tool.input_schema.is_object(), "`{}` needs an object schema", tool.name);
        }
        let mut views = BTreeSet::new();
        for view in &self.views {
            ensure!(!view.id.is_empty() && views.insert(view.id.as_str()), "`{}` is declared twice", view.id);
            ensure!(!view.title.trim().is_empty(), "`{}` needs a title", view.id);
        }
        for name in &self.permissions.tools {
            ensure!(tools.contains(name.as_str()), "`{name}` is permitted but not declared as a tool");
        }
        for name in &self.permissions.events {
            ensure!(HOST_EVENTS.contains(&name.as_str()), "`{name}` is not an event Codync sends modules");
        }
        ensure!(
            self.publisher.key.len() == 44 || B64.decode(self.publisher.key.trim()).is_ok_and(|k| k.len() == 32),
            "a publisher key is a base64 Ed25519 public key"
        );
        Ok(())
    }

    /// The signature over the canonical manifest, and every file's hash. Nothing runs unless both hold.
    pub fn verify(&self, dir: &Path, signature: &str) -> Result<()> {
        let contents: std::collections::BTreeMap<String, Vec<u8>> = folder_files(dir)?
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(dir.join(&path)).with_context(|| format!("reading {path}"))?;
                Ok((path, bytes))
            })
            .collect::<Result<_>>()?;
        self.verify_contents(signature, &contents)
    }

    /// The same checks for a bundle, whose files come in memory.
    pub fn verify_contents(
        &self,
        signature: &str,
        contents: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<()> {
        let key: [u8; 32] = B64
            .decode(self.publisher.key.trim())
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(|| anyhow::anyhow!("the publisher's key is not an Ed25519 public key"))?;
        let signature = Signature::from_slice(&B64.decode(signature.trim()).context("module.sig is not base64")?)
            .context("module.sig is not a signature")?;
        VerifyingKey::from_bytes(&key)
            .context("the publisher's key is not an Ed25519 public key")?
            .verify_strict(&self.canonical()?, &signature)
            .context("module.json changed after it was signed")?;
        for file in &self.files {
            let bytes = contents.get(&file.path).ok_or_else(|| anyhow::anyhow!("{} is missing", file.path))?;
            ensure!(bytes.len() as u64 <= MAX_FILE_BYTES, "{} is larger than 4 MB", file.path);
            ensure!(sha256(bytes).eq_ignore_ascii_case(&file.sha256), "{} changed after it was signed", file.path);
        }
        let listed: BTreeSet<&str> = self.files.iter().map(|f| f.path.as_str()).collect();
        for path in contents.keys() {
            ensure!(listed.contains(path.as_str()), "`{path}` is in the folder but the manifest doesn't list it");
        }
        Ok(())
    }

    /// The same manifest with fresh hashes, for an author packaging a folder.
    pub fn hash_files(&mut self, dir: &Path) -> Result<()> {
        self.files = folder_files(dir)?
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(dir.join(&path))?;
                Ok(FileHash { path, sha256: sha256(&bytes) })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }

    /// Whether the module may hear `name`.
    pub fn hears(&self, name: &str) -> bool {
        self.permissions.events.iter().any(|e| e == name)
    }

    pub fn tool(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }
}

/// Every file in a module's folder, relative and with `/` separators: `module.json`, `module.sig` and the
/// module's own `data/` stay out.
pub fn folder_files(dir: &Path) -> Result<Vec<String>> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if name == DATA {
                    continue;
                }
                walk(root, &path, out)?;
            } else if name != MANIFEST && name != SIGNATURE {
                let relative: PathBuf = path.strip_prefix(root)?.to_owned();
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// A file's SHA-256, in lowercase hex.
pub fn sha256(bytes: &[u8]) -> String {
    let mut hash = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(hash, "{byte:02x}");
    }
    hash
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn manifest() -> Manifest {
        Manifest {
            format: FORMAT.into(),
            contract: CONTRACT,
            id: "com.example.weather".into(),
            name: "Weather".into(),
            version: "1.0.0".into(),
            description: "Forecast for the bot's city.".into(),
            symbol: Some("cloud.sun".into()),
            entry: Entry { command: "module-demo".into(), args: vec![] },
            permissions: Permissions {
                tools: vec!["forecast".into()],
                events: vec!["botStatus".into()],
                network: vec!["api.open-meteo.com".into()],
                files: Files::Read,
            },
            limits: Limits::default(),
            tools: vec![Tool {
                name: "forecast".into(),
                description: "Tomorrow's weather.".into(),
                input_schema: object_schema(),
            }],
            views: vec![View { id: "today".into(), title: "Today".into() }],
            files: vec![],
            publisher: Publisher { name: "Example".into(), key: String::new() },
        }
    }

    #[test]
    fn a_manifest_names_what_it_may_do() {
        let mut m = manifest();
        m.permissions.tools = vec!["nope".into()];
        assert!(m.validate().unwrap_err().to_string().contains("not declared as a tool"));
        let mut m = manifest();
        m.permissions.events = vec!["turnStarted".into()];
        assert!(m.validate().unwrap_err().to_string().contains("not an event"));
        let mut m = manifest();
        m.files = vec![FileHash { path: "../escape".into(), sha256: "0".repeat(64) }];
        assert!(m.validate().unwrap_err().to_string().contains("leaves the module's folder"));
        let mut m = manifest();
        m.version = "one".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn a_newer_contract_is_refused_by_name() {
        let mut m = manifest();
        m.contract = CONTRACT + 1;
        let error = m.check_contract().unwrap_err().to_string();
        assert!(error.contains("newer Codync"), "{error}");
    }

    #[test]
    fn limits_only_come_down() {
        let asked = Limits { calls_per_minute: 10_000, time_ms: 600_000, memory_mb: 9_999 };
        let held = asked.clamped();
        assert_eq!(held, Limits { calls_per_minute: 600, time_ms: 120_000, memory_mb: 2_048 });
        let small = Limits { calls_per_minute: 5, time_ms: 2_000, memory_mb: 64 };
        assert_eq!(small.clamped(), small);
    }

    #[test]
    fn a_signed_folder_verifies_and_a_changed_file_does_not() {
        let dir = crate::modules::tests::temp_dir("verify");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.js"), b"console.log(1)").unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let mut m = manifest();
        m.publisher.key = B64.encode(key.verifying_key().to_bytes());
        m.hash_files(&dir).unwrap();
        let signature = m.sign(&key).unwrap();
        std::fs::write(dir.join(MANIFEST), serde_json::to_vec(&m).unwrap()).unwrap();
        std::fs::write(dir.join(SIGNATURE), &signature).unwrap();
        let read = Manifest::read(&dir).unwrap();
        assert_eq!(read.files, m.files);

        std::fs::write(dir.join("main.js"), b"console.log(2)").unwrap();
        let error = Manifest::read(&dir).unwrap_err().to_string();
        assert!(error.contains("changed after it was signed"), "{error}");
    }

    #[test]
    fn a_file_nobody_signed_stops_the_module() {
        let dir = crate::modules::tests::temp_dir("extra");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.js"), b"1").unwrap();
        let key = SigningKey::from_bytes(&[9; 32]);
        let mut m = manifest();
        m.publisher.key = B64.encode(key.verifying_key().to_bytes());
        m.hash_files(&dir).unwrap();
        let signature = m.sign(&key).unwrap();
        std::fs::write(dir.join(MANIFEST), serde_json::to_vec(&m).unwrap()).unwrap();
        std::fs::write(dir.join(SIGNATURE), &signature).unwrap();
        std::fs::write(dir.join("sneaky.sh"), b"rm -rf /").unwrap();
        let error = Manifest::read(&dir).unwrap_err().to_string();
        assert!(error.contains("doesn't list it"), "{error}");
    }

    #[test]
    fn the_modules_own_data_folder_is_not_part_of_it() {
        let dir = crate::modules::tests::temp_dir("data");
        std::fs::create_dir_all(dir.join(DATA)).unwrap();
        std::fs::write(dir.join("main.js"), b"1").unwrap();
        std::fs::write(dir.join(DATA).join("state.json"), b"{}").unwrap();
        assert_eq!(folder_files(&dir).unwrap(), vec!["main.js".to_owned()]);
    }
}
