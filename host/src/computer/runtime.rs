//! Where a bot's computer runs: this host (`Local`) or a Docker container (`Container`).
//!
//! The agent loop doesn't change: the workspace tools keep reading the bot's folder on the host, and the shell
//! tool runs the same command either here or through `docker exec` in the container. A container gets its own
//! home volume, its own browser profile folder and the bot's workspace mounted at `/workspace`; the Docker
//! socket is never mounted and no host credential is passed in. When Docker isn't there, a container bot falls
//! back to the local workspace and the panel says so.

use crate::hub::Hub;
use crate::store::BotConfig;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// The image a container computer runs when the person names none. It carries a shell, git and the usual
/// command-line tools; see `docs/features/computer-containers.md`.
pub const DEFAULT_IMAGE: &str = "codync/computer:1";
/// Docker answers these quickly; a hung engine must not hold a panel poll open.
const DOCKER_TIMEOUT: Duration = Duration::from_secs(20);
/// The first `docker run` may pull the image; it gets longer than a status check.
const DOCKER_RUN_TIMEOUT: Duration = Duration::from_secs(300);
/// Where the workspace is mounted inside every container.
pub const WORKSPACE_MOUNT: &str = "/workspace";
/// Where a bot's browser profile is mounted inside every container.
pub const PROFILE_MOUNT: &str = "/profile";
const HOME_MOUNT: &str = "/home/codync";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The real host, as Codync has always run it.
    #[default]
    Local,
    /// A Docker container per bot.
    Container,
}

impl Kind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Container => "container",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "local" => Some(Self::Local),
            "container" | "docker" => Some(Self::Container),
            _ => None,
        }
    }
}

/// A bot's chooser for where its computer runs. Stored in the bot's configuration, so every client sees it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub kind: Kind,
    /// The image used for a container computer; empty means [`DEFAULT_IMAGE`].
    pub image: String,
    /// Use gVisor (`--runtime=runsc`) when Docker lists that runtime.
    pub gvisor: bool,
    /// A container bot whose Docker is missing runs in its local workspace instead of failing.
    pub fallback_local: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { kind: Kind::Local, image: String::new(), gvisor: true, fallback_local: true }
    }
}

impl Settings {
    /// The image actually used.
    pub fn image(&self) -> &str {
        if self.image.trim().is_empty() { DEFAULT_IMAGE } else { self.image.trim() }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.image.chars().count() <= 200, "The container image name is too long.");
        if !self.image.trim().is_empty() {
            ensure!(
                self.image.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:@".contains(c)),
                "Use a Docker image name such as codync/computer:1."
            );
            ensure!(!self.image.starts_with('-'), "The container image name can't start with a dash.");
        }
        Ok(())
    }
}

/// Resolved for one bot: the folder it works in and, for a container, how that container is named and made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Runtime {
    pub kind: Kind,
    pub bot_id: String,
    pub root: PathBuf,
    pub container: String,
    pub image: String,
    pub gvisor: bool,
    pub fallback_local: bool,
}

impl Runtime {
    pub fn local(cfg: &BotConfig) -> Self {
        Self::resolve(cfg, &Settings::default())
    }

    pub fn resolve(cfg: &BotConfig, settings: &Settings) -> Self {
        Self {
            kind: settings.kind,
            bot_id: cfg.id.clone(),
            root: workspace_root(cfg),
            container: container_name(&cfg.id),
            image: settings.image().to_owned(),
            gvisor: settings.gvisor,
            fallback_local: settings.fallback_local,
        }
    }

    pub fn is_container(&self) -> bool {
        self.kind == Kind::Container
    }

    /// The folder the workspace tools read on the host. A container mounts this same folder at [`WORKSPACE_MOUNT`],
    /// so listing and previewing a file never needs `docker exec`, and the workspace survives the container.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The browser profile folder for this bot, kept out of a project workspace.
    pub fn browser_profile(&self) -> PathBuf {
        crate::service::data_dir().join("bots").join(&self.bot_id).join("browser-profile")
    }

    /// The command that runs `line`: here, or in the container's shell.
    pub fn shell(&self, line: &str) -> Command {
        match self.kind {
            Kind::Local => {
                let mut command = crate::shell::line(line).tokio();
                command.current_dir(&self.root);
                command
            }
            Kind::Container => docker_shell(&self.container, line),
        }
    }

    /// Makes the container, or starts it again, and returns the runtime that can run a command. A container bot
    /// with no Docker falls back to its local workspace when that is allowed (`fallback_local`).
    pub async fn ready(&self) -> Result<Ready> {
        if !self.is_container() {
            return Ok(Ready::Local);
        }
        match ensure_container(self).await {
            Ok(gvisor) => Ok(Ready::Container { gvisor }),
            Err(error) if self.fallback_local => {
                tracing::warn!(
                    bot = %self.bot_id,
                    error = format!("{error:#}"),
                    "the bot's container is unavailable; using its local workspace"
                );
                Ok(Ready::LocalFallback(error.to_string()))
            }
            Err(error) => Err(error.context("The bot's container computer is unavailable.")),
        }
    }
}

/// What [`Runtime::ready`] decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ready {
    /// The bot runs here.
    Local,
    /// The bot runs in its container; whether gVisor was used.
    Container { gvisor: bool },
    /// Docker (or the container) isn't available, so the bot runs in its local workspace instead.
    LocalFallback(String),
}

/// The state the panel shows. Nothing here is a secret: no image environment, no socket path.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// What the bot is set to ("local" or "container").
    pub kind: &'static str,
    /// What actually runs now: "local" after a fallback.
    pub active: &'static str,
    pub available: bool,
    /// The Docker engine version, when it answered.
    pub engine: Option<String>,
    /// The setting: gVisor requested.
    pub gvisor: bool,
    /// Whether this run uses `runsc`.
    pub gvisor_active: bool,
    pub container: Option<String>,
    pub running: bool,
    pub image: String,
    pub workspace: String,
    pub browser_profile: String,
    /// A sentence for the person, including why a container bot runs here instead.
    pub detail: String,
}

/// The status of `cfg`'s computer: Docker's engine and runtimes, and whether its container is running. It never starts
/// the container (its first shell command does).
pub async fn status(_hub: &Hub, cfg: &BotConfig) -> Status {
    let settings = &cfg.computer_runtime;
    let runtime = Runtime::resolve(cfg, settings);
    let mut status = Status {
        kind: settings.kind.as_str(),
        active: settings.kind.as_str(),
        available: true,
        engine: None,
        gvisor: settings.gvisor,
        gvisor_active: false,
        container: None,
        running: false,
        image: runtime.image.clone(),
        workspace: runtime.root.display().to_string(),
        browser_profile: runtime.browser_profile().display().to_string(),
        detail: "Runs on this computer.".into(),
    };
    if !cfg.computer {
        status.detail = "Computer access is off for this bot.".into();
        return status;
    }
    if !runtime.is_container() {
        return status;
    }
    status.container = Some(runtime.container.clone());
    let engine = match docker(&["version", "--format", "{{.Server.Version}}"]).await {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        Err(error) => {
            status.available = false;
            status.active = if runtime.fallback_local { "local" } else { "container" };
            status.detail = if runtime.fallback_local {
                format!("Docker isn't available ({error}); this bot uses its local workspace.")
            } else {
                format!("Docker isn't available ({error}); its shell tool won't run.")
            };
            return status;
        }
    };
    status.engine = Some(engine);
    let runtimes = docker(&["info", "--format", "{{json .Runtimes}}"]).await.ok();
    let supports_gvisor =
        runtimes.as_ref().is_some_and(|output| gvisor_listed(&String::from_utf8_lossy(&output.stdout)));
    // The panel only looks: a container starts on the bot's first shell command, so opening a panel never makes one.
    let running = inspect_running(&runtime.container).await.unwrap_or(false);
    status.running = running;
    status.gvisor_active = running && runtime.gvisor && supports_gvisor;
    status.detail = if running {
        if status.gvisor_active {
            "Runs in a Docker container with gVisor.".into()
        } else if settings.gvisor && !supports_gvisor {
            "Runs in a Docker container; gVisor wasn't found, so it uses Docker's default runtime.".into()
        } else {
            "Runs in a Docker container.".into()
        }
    } else {
        "Its container isn't running yet; the bot's first shell command starts it.".into()
    };
    status
}

/// The container's name: Docker accepts letters, digits, `_`, `.` and `-`; bot ids are UUIDs.
pub fn container_name(bot_id: &str) -> String {
    let safe: String =
        bot_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    format!("codync-bot-{safe}")
}

/// The host folder a bot works in, matching `agent::workspace`.
pub fn workspace_root(cfg: &BotConfig) -> PathBuf {
    if cfg.cwd.trim().is_empty() { crate::agent::workspace::path(&cfg.id) } else { PathBuf::from(&cfg.cwd) }
}

/// The browser profile folder under the host's data directory. It is made when a container is prepared.
pub fn browser_profile(bot_id: &str) -> PathBuf {
    crate::service::data_dir().join("bots").join(bot_id).join("browser-profile")
}

/// Whether Docker's runtimes list (`docker info --format '{{json .Runtimes}}'`) has gVisor.
pub fn gvisor_listed(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|value| value.as_object().map(|map| map.contains_key("runsc")))
        .unwrap_or_else(|| json.contains("\"runsc\""))
}

/// `docker exec` into `container` with `line` as its shell command.
fn docker_shell(container: &str, line: &str) -> Command {
    let mut command = Command::new(docker_program());
    command.args(["exec", "-i", "-w", WORKSPACE_MOUNT, container, "sh", "-lc", line]);
    command
}

fn docker_program() -> PathBuf {
    crate::shell::program("docker")
}

/// Runs one Docker CLI command. Docker's stderr is the useful part when it fails.
async fn docker(args: &[&str]) -> Result<std::process::Output> {
    docker_for(args, DOCKER_TIMEOUT).await
}

async fn docker_for(args: &[&str], timeout: Duration) -> Result<std::process::Output> {
    let output = tokio::time::timeout(
        timeout,
        Command::new(docker_program())
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("docker {} timed out", args.first().copied().unwrap_or_default()))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        bail!("docker {} failed: {}", args.first().copied().unwrap_or_default(), truncate(stderr, 300));
    }
    Ok(output)
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// The arguments that make a container computer. Kept separate so a test can check the locks (no Docker socket,
/// no privileged mode, no host environment) without Docker installed.
pub fn run_args(runtime: &Runtime, gvisor: bool) -> Vec<String> {
    let profile = runtime.browser_profile();
    let mut args = vec![
        "run".into(),
        "-d".into(),
        "--name".into(),
        runtime.container.clone(),
        "--label".into(),
        format!("codync.bot={}", runtime.bot_id),
        "--label".into(),
        format!("codync.image={}", runtime.image),
        "--hostname".into(),
        "codync".into(),
        "--restart".into(),
        "unless-stopped".into(),
        "--cap-drop".into(),
        "ALL".into(),
        "--security-opt".into(),
        "no-new-privileges".into(),
        "--pids-limit".into(),
        "512".into(),
        "--memory".into(),
        "4g".into(),
        "-v".into(),
        format!("{}:{WORKSPACE_MOUNT}", runtime.root.display()),
        "-v".into(),
        format!("{}:{PROFILE_MOUNT}", profile.display()),
        "-e".into(),
        format!("HOME={HOME_MOUNT}"),
        "-w".into(),
        WORKSPACE_MOUNT.into(),
    ];
    // The home folder is a named volume, so a bot's tools and shell history survive a recreated container.
    args.extend([
        "-v".into(),
        format!("{}_home:{HOME_MOUNT}", runtime.container),
        runtime.image.clone(),
        "sleep".into(),
        "infinity".into(),
    ]);
    if gvisor {
        args.insert(2, "--runtime".to_owned());
        args.insert(3, "runsc".to_owned());
    }
    args
}

/// Makes or starts the bot's container. Returns whether gVisor is used.
async fn ensure_container(runtime: &Runtime) -> Result<bool> {
    ensure!(runtime.is_container(), "not a container computer");
    ensure!(runtime.root.is_dir(), "the bot's workspace folder doesn't exist: {}", runtime.root.display());
    let profile = runtime.browser_profile();
    std::fs::create_dir_all(&profile).context("creating the bot's browser profile folder")?;
    let exists = inspect_running(&runtime.container).await.is_ok();
    let runtimes = docker(&["info", "--format", "{{json .Runtimes}}"]).await.ok();
    let gvisor =
        runtime.gvisor && runtimes.as_ref().is_some_and(|o| gvisor_listed(&String::from_utf8_lossy(&o.stdout)));
    if exists {
        let image = docker(&["inspect", "--format", "{{index .Config.Labels \"codync.image\"}}", &runtime.container])
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_default();
        if image != runtime.image {
            // The workspace and home are volumes: replacing the container keeps the bot's files.
            docker(&["rm", "-f", &runtime.container]).await?;
        } else {
            match inspect_running(&runtime.container).await {
                Ok(true) => return Ok(gvisor),
                Ok(false) => {
                    docker(&["start", &runtime.container]).await?;
                    return Ok(gvisor);
                }
                Err(_) => {}
            }
        }
    }
    let args = run_args(runtime, gvisor);
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    docker_for(&borrowed, DOCKER_RUN_TIMEOUT).await?;
    Ok(gvisor)
}

/// `Some(running)` for a container that exists; an error when there is no such container.
async fn inspect_running(container: &str) -> Result<bool> {
    let output = docker(&["inspect", "--format", "{{.State.Running}}", container]).await?;
    Ok(String::from_utf8_lossy(&output.stdout).trim() == "true")
}

/// The shell command a bot's shell tool runs, with the container fallback. Used by `terminal`.
pub async fn shell_command(runtime: &Runtime, line: &str) -> Result<Command> {
    if runtime.is_container() {
        let ready = runtime.ready().await?;
        if matches!(ready, Ready::Container { .. }) {
            return Ok(runtime.shell(line));
        }
    }
    Ok(Runtime::local_for(runtime).shell(line))
}

impl Runtime {
    /// A local runtime with this runtime's workspace: what a fallback runs in.
    fn local_for(runtime: &Runtime) -> Runtime {
        Runtime {
            kind: Kind::Local,
            bot_id: runtime.bot_id.clone(),
            root: runtime.root.clone(),
            container: runtime.container.clone(),
            image: runtime.image.clone(),
            gvisor: false,
            fallback_local: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg(value: serde_json::Value) -> BotConfig {
        serde_json::from_value(value).expect("a bot config")
    }

    #[test]
    fn a_bot_without_a_choice_runs_here() {
        let bot = cfg(json!({"id": "b1", "name": "B", "backend": "x", "command": "x", "cwd": "/tmp"}));
        assert_eq!(bot.computer_runtime, Settings::default());
        assert_eq!(Runtime::resolve(&bot, &bot.computer_runtime).kind, Kind::Local);
        assert!(!Runtime::resolve(&bot, &bot.computer_runtime).is_container());
    }

    #[test]
    fn a_container_gets_its_own_name_and_profile_but_the_same_workspace() {
        let bot = cfg(json!({
            "id": "0d6b8b3e-1b7f-4a1e-9d5a-2d0e6b7c8a91", "name": "B", "backend": "x", "command": "x",
            "cwd": "/work/project",
            "computerRuntime": {"kind": "container", "image": "codync/computer:1", "gvisor": true}
        }));
        let runtime = Runtime::resolve(&bot, &bot.computer_runtime);
        assert_eq!(runtime.kind, Kind::Container);
        assert_eq!(runtime.container, "codync-bot-0d6b8b3e-1b7f-4a1e-9d5a-2d0e6b7c8a91");
        assert_eq!(runtime.root, std::path::PathBuf::from("/work/project"));
        assert!(runtime.browser_profile().ends_with("browser-profile"));
        assert!(!runtime.browser_profile().starts_with("/work/project"));
    }

    #[test]
    fn a_container_with_no_image_uses_the_shipped_one_and_rejects_a_bad_name() {
        let mut bot = cfg(json!({"id": "b", "name": "B", "backend": "x", "command": "x", "cwd": "/tmp"}));
        bot.computer_runtime = Settings { kind: Kind::Container, ..Settings::default() };
        assert_eq!(Runtime::resolve(&bot, &bot.computer_runtime).image, DEFAULT_IMAGE);
        for bad in ["", "-x", "bad image", "a;rm"] {
            let settings = Settings { image: bad.into(), ..Settings::default() };
            assert_eq!(settings.validate().is_ok(), bad.is_empty(), "{bad:?}");
        }
        assert!(Settings { image: "ghcr.io/codync/computer:2".into(), ..Settings::default() }.validate().is_ok());
    }

    #[test]
    fn the_container_gets_a_workspace_and_a_profile_but_never_the_docker_socket() {
        let bot = cfg(json!({
            "id": "b", "name": "B", "backend": "x", "command": "x", "cwd": "/work",
            "computerRuntime": {"kind": "container"}
        }));
        let runtime = Runtime::resolve(&bot, &bot.computer_runtime);
        let args = run_args(&runtime, true);
        let joined = args.join(" ");
        assert!(joined.contains("--runtime runsc"), "{joined}");
        assert!(joined.contains(":/workspace"), "{joined}");
        assert!(joined.contains(":/profile"), "{joined}");
        assert!(!joined.contains("docker.sock"), "{joined}");
        assert!(!joined.contains("--privileged"), "{joined}");
        assert!(!joined.contains("--network=host"), "{joined}");
        assert!(joined.contains("--cap-drop ALL"), "{joined}");
        // Without gVisor the flag is absent, not silently changed.
        assert!(!run_args(&runtime, false).join(" ").contains("runsc"));
    }

    #[test]
    fn gvisor_is_found_in_dockers_runtime_list() {
        assert!(gvisor_listed(r#"{"runc":{"path":"runc"},"runsc":{"path":"runsc"}}"#));
        assert!(!gvisor_listed(r#"{"runc":{"path":"runc"}}"#));
        assert!(!gvisor_listed("not json"));
    }

    #[test]
    fn a_docker_shell_command_targets_the_workspace_only() {
        let bot = cfg(json!({
            "id": "b", "name": "B", "backend": "x", "command": "x", "cwd": "/work",
            "computerRuntime": {"kind": "container"}
        }));
        let runtime = Runtime::resolve(&bot, &bot.computer_runtime);
        let command = runtime.shell("echo hi");
        let args: Vec<String> = command.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, vec!["exec", "-i", "-w", "/workspace", "codync-bot-b", "sh", "-lc", "echo hi"]);
    }
}
