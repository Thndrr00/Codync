//! Methods about this computer itself: host updates, hello/sync, agents and setup
//! terminals, usage, folders, voice and the remote screen.

use super::devices::Caller;
use super::str_arg;
use crate::LockExt;
use crate::agent::backends;
use crate::hub::Hub;
use crate::usage;
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::ops::ControlFlow;
use std::sync::Arc;

pub(super) async fn call(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<ControlFlow<Value, Value>> {
    // Who is on this computer: only the owner manages it.
    if crate::people::METHODS.contains(&method) {
        return Ok(ControlFlow::Break(crate::people::call(hub, method, &b)?));
    }
    Ok(ControlFlow::Break(match method {
        "openaiKeys" | "openaiCreateKey" | "openaiRevokeKey" => super::openai_keys::call(hub, method, &b).await?,
        "hostUpdateStatus" => crate::update::status()?,
        "hostHealth" => crate::health::value(hub),
        // How often each skill's text entered a turn, with its estimated tokens and cost.
        "skillUsage" => crate::market::skill_usage_report(&hub.store),
        "channelActivity" => {
            let limit = b["limit"].as_u64().and_then(|n| usize::try_from(n).ok()).unwrap_or(30);
            let activity = if b["mentionsOnly"].as_bool().unwrap_or(true) {
                hub.store.channel_mentions(limit)?
            } else {
                hub.store.channel_activity(limit)?
            };
            json!({"activity": activity})
        }
        "diagnostics" => crate::diagnostics::snapshot(hub)?,
        // The security decisions made on this computer.
        "auditLog" => crate::audit::list(hub),
        "enterpriseStatus" => crate::enterprise::status(&hub.store)?,
        "setEnterprise" => crate::enterprise::set(hub, &b).await?,
        "disconnectEnterprise" => crate::enterprise::disconnect(&hub.store)?,
        "rotateScim" => json!({"token": crate::enterprise::rotate_scim(&hub.store)?}),
        "members" => crate::enterprise::member_list(&hub.store)?,
        "addMember" => {
            crate::enterprise::add_member(&hub.store, str_arg(&b, "email")?, b["name"].as_str().unwrap_or_default())?
        }
        "removeMember" => json!({"removed": crate::enterprise::remove_member(&hub.store, str_arg(&b, "id")?)?}),
        "clearAuditLog" => {
            crate::audit::clear(hub)?;
            json!({})
        }
        // Palettes from the person's own theme files; a phone may read them, only the computer chooses one.
        "themeThemes" => json!({"themes": crate::theme::list(), "active": crate::theme::active(&hub.store)}),
        "setTheme" => {
            crate::theme::select(&hub.store, b["name"].as_str())?;
            crate::theme::active(&hub.store)
        }
        "serverSettings" => crate::server_settings::Settings::load(&hub.store).to_json(),
        "setServerSettings" => {
            let mut settings = crate::server_settings::Settings::load(&hub.store);
            settings.apply(&b)?;
            settings.save(&hub.store)?;
            let _ = hub.events.send(json!({"type": "serverSettings", "settings": settings.to_json()}));
            json!({"settings": settings.to_json()})
        }
        "setHealthThresholds" => {
            let thresholds = crate::health::Thresholds {
                warn: u8::try_from(b["warn"].as_u64().unwrap_or(0)).unwrap_or(0),
                critical: u8::try_from(b["critical"].as_u64().unwrap_or(0)).unwrap_or(0),
            };
            hub.health.set_thresholds(&hub.store, thresholds)?;
            crate::health::value(hub)
        }
        "scriptLimit" => json!({
            "perHour": crate::routines::limits::saved(&hub.store),
            "choices": crate::routines::limits::CHOICES,
        }),
        "setScriptLimit" => {
            let per_hour = u32::try_from(b["perHour"].as_u64().context("perHour is required")?).unwrap_or(u32::MAX);
            json!({"perHour": crate::routines::limits::set(&hub.store, per_hour)?})
        }
        // The computer's local app safeguard: how many mini-apps it holds and the ceiling.
        "localApps" => crate::local_apps::status(&hub.store)?,
        "setLocalAppsLimit" => {
            let limit = u32::try_from(b["limit"].as_u64().context("limit is required")?).unwrap_or(u32::MAX);
            json!({"limit": crate::local_apps::set(&hub.store, limit)?})
        }
        "planStart" => json!({
            "afterMs": crate::external::plan::saved(&hub.store),
            "presets": crate::external::plan::PRESETS,
            "min": crate::external::plan::MIN,
            "max": crate::external::plan::MAX,
        }),
        "setPlanStart" => {
            let after = match b.get("afterMs") {
                None | Some(Value::Null) => None,
                Some(value) => Some(value.as_i64().context("afterMs must be a number of milliseconds or null")?),
            };
            json!({"afterMs": crate::external::plan::set(&hub.store, after)?})
        }
        "weekSummary" => {
            let days = b["days"].as_i64().unwrap_or(7).clamp(1, 31);
            let since = crate::store::now_ms() - days * 24 * 60 * 60 * 1000;
            serde_json::to_value(hub.store.week(since)?)?
        }
        "whatsappState" => {
            let mut view = serde_json::to_value(hub.whatsapp.state().await)?;
            // The bot this number feeds is a setting of the computer, not of the link's last event.
            if let Some(bot) = hub.store.kv_get("whatsapp.bot") {
                view["bot"] = Value::String(bot);
            }
            view["allowed"] = serde_json::to_value(crate::whatsapp::allowed(hub))?;
            view["requests"] = serde_json::to_value(crate::whatsapp::pending_requests(hub))?;
            view["people"] = serde_json::to_value(crate::whatsapp::people(hub))?;
            view
        }
        "whatsappConnect" => {
            hub.whatsapp.start(hub);
            json!({"ok": true})
        }
        "whatsappPair" => {
            let phone = str_arg(&b, "phone")?;
            hub.whatsapp.pair(hub, phone)?;
            let mut view = serde_json::to_value(hub.whatsapp.state().await)?;
            if let Some(bot) = hub.store.kv_get("whatsapp.bot") {
                view["bot"] = Value::String(bot);
            }
            view["allowed"] = serde_json::to_value(crate::whatsapp::allowed(hub))?;
            view
        }
        "whatsappLogout" => {
            hub.whatsapp.logout(hub).await?;
            json!({"ok": true})
        }
        "whatsappSetBot" => {
            let bot = str_arg(&b, "botId")?;
            crate::whatsapp::set_bot(hub, bot)?;
            json!({"ok": true})
        }
        "whatsappAllowed" => json!({"allowed": crate::whatsapp::allowed(hub)}),
        "whatsappSetAllowed" => {
            let jid = str_arg(&b, "jid")?;
            let list = crate::whatsapp::set_allowed(hub, jid, b["allowed"].as_bool().unwrap_or(true))?;
            json!({"allowed": list})
        }
        "whatsappAnswerRequest" => {
            let code = str_arg(&b, "code")?;
            let approve = b["approve"].as_bool().unwrap_or(false);
            if let Some(jid) = crate::whatsapp::answer_request(hub, code, approve)? {
                // One line to the person who asked, so they know they may write now.
                let line = "Tu solicitud fue aprobada. Ya puedes escribir a este asistente.";
                if let Err(error) = hub.whatsapp.send_message(&jid, line, None, None, None).await {
                    tracing::warn!(%error, "WhatsApp could not tell an approved person");
                }
            }
            json!({"requests": crate::whatsapp::pending_requests(hub)})
        }
        "whatsappSend" => {
            let body: crate::whatsapp::SendBody = serde_json::from_value(b)?;
            hub.whatsapp
                .send_message(
                    &body.to,
                    &body.text,
                    body.path.as_deref(),
                    body.name.as_deref(),
                    body.mimetype.as_deref(),
                )
                .await?;
            json!({"ok": true})
        }
        "sentinelHealth" => crate::sentinel::health_json(&hub.store, b["lookbackMs"].as_i64())?,
        "checkHostUpdate" => crate::update::check().await?,
        "setHostAutomaticUpdates" => {
            crate::update::set_automatic(b["enabled"].as_bool().context("enabled must be a boolean")?)?
        }
        "installHostUpdate" => {
            let port = hub.port;
            let force = b["force"].as_bool().unwrap_or(false);
            if !force && hub.busy() {
                bail!("A bot is working on this computer. Update it once the bot is done.");
            }
            tokio::task::spawn_blocking(move || crate::update::request_install(port, force)).await??;
            json!({"scheduled": true})
        }
        "voiceStatus" => crate::voice::status(&hub.store)?,
        "voiceServer" => crate::voice::server_state(&hub.store)?,
        "setVoiceServer" => crate::voice::set_server(&hub.store, &b)?,
        "testVoiceServer" => crate::voice::test_server(&hub.store).await?,
        "setVoiceKey" => {
            let provider = crate::voice::Provider::parse(str_arg(&b, "provider")?)?;
            crate::voice::set_key(&hub.store, provider, b["key"].as_str().unwrap_or_default()).await?
        }
        "voiceSession" => {
            let provider = crate::voice::Provider::parse(str_arg(&b, "provider")?)?;
            crate::voice::session(&hub.store, provider, str_arg(&b, "model")?, str_arg(&b, "voice")?).await?
        }
        "voiceModels" => {
            crate::voice::models(&hub.store, crate::voice::Provider::parse(str_arg(&b, "provider")?)?).await?
        }
        "voiceVoices" => {
            crate::voice::voices(&hub.store, crate::voice::Provider::parse(str_arg(&b, "provider")?)?).await?
        }
        "voiceTranscribe" => {
            let provider = crate::voice::Provider::parse(str_arg(&b, "provider")?)?;
            crate::voice::transcribe(&hub.store, provider, str_arg(&b, "model")?, str_arg(&b, "audio")?).await?
        }
        "voiceSpeak" => {
            if b["provider"] == "neutts" {
                return Ok(ControlFlow::Break(
                    crate::voice::synthesis::speak(
                        &hub.store,
                        caller.device_key(),
                        str_arg(&b, "requestId")?,
                        b["botId"].as_str(),
                        str_arg(&b, "text")?,
                    )
                    .await?,
                ));
            }
            let provider = crate::voice::Provider::parse(str_arg(&b, "provider")?)?;
            crate::voice::speak(
                &hub.store,
                provider,
                str_arg(&b, "model")?,
                str_arg(&b, "voice")?,
                str_arg(&b, "text")?,
            )
            .await?
        }
        "voiceCancelSpeech" => crate::voice::synthesis::cancel(caller.device_key(), str_arg(&b, "requestId")?)?,
        "voiceLocalStatus" => crate::voice::synthesis::status(&hub.store, b["botId"].as_str()).await,
        "voiceStartSpeech" => crate::voice::synthesis::start(
            hub,
            caller.device_key(),
            str_arg(&b, "requestId")?,
            b["botId"].as_str(),
            str_arg(&b, "text")?,
        )?,
        "voiceSpeechChunk" => crate::voice::synthesis::next(caller.device_key(), str_arg(&b, "requestId")?).await?,
        "hello" => {
            let port = hub.port;
            let icons = crate::icons::snapshot(&hub.store)?;
            json!({
                "hostId": hub.host_id,
                "computerId": hub.identity.computer_id(),
                "signKey": hub.identity.sign_pub_b64(),
                "boxKey": hub.identity.box_pub_b64(),
                // Only where a phone may use it: in no account, phones go over Wi-Fi or Tailscale.
                "cloud": crate::remote::owner::url_for_phones(hub, caller.device_key()),
                "name": crate::remote::managed::computer_name(&hub.store),
                "version": env!("CARGO_PKG_VERSION"),
                "minApp": crate::compat::MIN_APP,
                // Whether `installHostUpdate` can update this computer, from here or a phone.
                "canUpdate": tokio::task::spawn_blocking(crate::update::installable).await?,
                // The latest release, so a phone can say whether this computer is current (null: unknown).
                "latestVersion": tokio::task::spawn_blocking(crate::update::latest_version).await?,
                "os": std::env::consts::OS,
                "device": crate::remote::managed::computer_device(&hub.store).await?,
                "home": dirs::home_dir().map(|p| p.to_string_lossy().into_owned()),
                "backends": backends::list(),
                "icons": icons.icons,
                "iconsRevision": icons.icons_revision,
                "rev": hub.store.current_rev(),
                "screen": hub.screen.state(),
                "n8n": hub.n8n.state(),
                "analytics": hub.analytics.state(),
                // Before 2.41.0, apps called `joinAccount` when this was true. It takes a certificate now, which they
                // don't send, so they aren't told to.
                "joinAccount": false,
                // A signed-in phone may offer to bring this computer into its account (`joinAccount`).
                "canJoinAccount": crate::remote::owner::can_join(hub),
                // Shells out to `tailscale`: keep it off the async workers.
                "urls": tokio::task::spawn_blocking(move || crate::service::addresses(port)).await?,
            })
        }
        "refreshBackends" => {
            tokio::task::spawn_blocking(backends::hydrate_path).await?;
            crate::market::agent_sign_in::refresh(hub).await;
            json!({"backends": backends::list()})
        }
        "usage" => {
            if b["refresh"].as_bool().unwrap_or(false) {
                usage::refresh(hub).await;
            }
            serde_json::to_value(hub.usage.locked().visible())?
        }
        // Whether the Usage screens list this agent's limits.
        "setUsageShown" => {
            let id = str_arg(&b, "id")?.to_owned();
            let shown = b["shown"].as_bool().context("shown is required")?;
            let hub = hub.clone();
            let usage = tokio::task::spawn_blocking(move || {
                usage::set_shown(&hub, &id, shown)?;
                anyhow::Ok(hub.usage.locked().visible())
            })
            .await??;
            serde_json::to_value(usage)?
        }
        // The order the Usage screens list the agents in.
        "setUsageOrder" => {
            let ids: Vec<String> = serde_json::from_value(b["ids"].clone()).context("ids is required")?;
            let hub = hub.clone();
            let usage = tokio::task::spawn_blocking(move || {
                usage::set_order(&hub, &ids)?;
                anyhow::Ok(hub.usage.locked().visible())
            })
            .await??;
            serde_json::to_value(usage)?
        }
        "listDirs" => {
            let path = b["path"].as_str().map(std::path::PathBuf::from);
            let hub = hub.clone();
            tokio::task::spawn_blocking(move || super::folders::listing(&hub.store, path)).await??
        }
        "selectWorkspace" => {
            let path = str_arg(&b, "path")?.to_owned();
            let hub = hub.clone();
            tokio::task::spawn_blocking(move || super::folders::select(&hub.store, &path)).await??
        }
        "agentSetup" => {
            let step: crate::agent::term::Step =
                serde_json::from_value(b["step"].clone()).context("`step` is install or login")?;
            let (cols, rows) = term_size(&b);
            // Node.js belongs to the computer, not to one agent.
            let backend = if step == crate::agent::term::Step::Node { "" } else { str_arg(&b, "backend")? };
            json!({"term": hub.terms.start(backend, step, b["method"].as_str(), cols, rows).await?})
        }
        "agentModels" => {
            let backend = str_arg(&b, "backend")?;
            let mut catalog = crate::agent::auth::models(&hub.store, backend, b["account"].as_str()).await?;
            catalog["hidden"] = json!(crate::agent::auth::hidden(&hub.store, backend));
            catalog
        }
        "setModelHidden" => {
            crate::agent::auth::set_hidden(
                &hub.store,
                str_arg(&b, "backend")?,
                str_arg(&b, "model")?,
                b["hidden"].as_bool().unwrap_or(true),
            )?;
            json!({})
        }
        "agentAuth" | "agentAuthenticate" | "setAgentEnv" | "agentEnv" => {
            let backend = str_arg(&b, "backend")?;
            let status = match method {
                "agentAuth" => crate::agent::auth::check(&hub.store, backend).await?,
                "agentEnv" => crate::agent::auth::env_names(&hub.store, backend, str_arg(&b, "botId")?)?,
                "agentAuthenticate" => {
                    crate::agent::auth::authenticate(&hub.store, backend, str_arg(&b, "method")?).await?
                }
                _ => {
                    let vars = b["vars"].as_object().ok_or_else(|| anyhow!("`vars` is required"))?;
                    let status = crate::agent::auth::set_env(&hub.store, backend, b["botId"].as_str(), vars).await?;
                    // The AG-UI adapter reads its endpoint from this environment, so its process starts again.
                    if backend == crate::agui::BACKEND {
                        let targets: Vec<String> = match b["botId"].as_str() {
                            Some(id) => vec![id.to_owned()],
                            None => hub
                                .store
                                .bots()?
                                .into_iter()
                                .filter(|row| !row.deleted && !row.config.is_group() && row.config.backend == backend)
                                .map(|row| row.config.id)
                                .collect(),
                        };
                        for id in targets {
                            let _ = hub.send_cmd(&id, crate::agent::bot::Cmd::RestartAgent);
                        }
                    }
                    status
                }
            };
            // Chats that asked the user to sign in get their messages answered now.
            if status["signedIn"] == true {
                crate::market::agent_sign_in::agent_signed_in(hub, backend)?;
            }
            status
        }
        "listAccounts" => {
            let backend = b["backend"].as_str();
            json!({"accounts": crate::agent::account::list(backend)?})
        }
        "accountAuthenticate" => {
            let backend = str_arg(&b, "backend")?;
            let name = b["name"].as_str().map(str::to_owned);
            json!({"account": crate::agent::account::authenticate(&hub.store, backend, name).await?})
        }
        "agentSignOut" => {
            let backend = str_arg(&b, "backend")?;
            crate::agent::account::sign_out(&hub.store, backend).await?;
            json!({"auth": crate::agent::auth::check(&hub.store, backend).await?})
        }
        "deleteAccount" => {
            let account_id = str_arg(&b, "accountId")?;
            crate::agent::account::delete(&hub.store, account_id).await?;
            json!({})
        }
        "deleteAgentDownload" => {
            let backend = str_arg(&b, "backend")?.to_owned();
            // A build can be gigabytes: removing it must not hold an async worker.
            let removed =
                tokio::task::spawn_blocking(move || crate::agent::backends::remove_download(&backend)).await??;
            let listed = tokio::task::spawn_blocking(backends::list).await?;
            json!({"removed": removed, "backends": listed})
        }
        "termInput" => {
            hub.terms.write(str_arg(&b, "term")?, str_arg(&b, "data")?)?;
            json!({})
        }
        "termResize" => {
            let (cols, rows) = term_size(&b);
            hub.terms.resize(str_arg(&b, "term")?, cols, rows)?;
            json!({})
        }
        "termClose" => {
            hub.terms.close(str_arg(&b, "term")?);
            json!({})
        }
        "screenStatus" => hub.screen.state(),
        "requestScreenPermission" => hub.screen.request_permission(&b).await?,
        "answerScreenRequest" => {
            if !matches!(caller, Caller::Local) {
                return Err(anyhow::anyhow!("Only this computer answers a remote control request."));
            }
            let id = str_arg(&b, "id")?;
            let allow = b["allow"].as_bool().unwrap_or(false);
            json!({"answered": hub.screen.answer_consent(id, allow)?, "screen": hub.screen.state()})
        }
        "screenStopAll" => {
            // A device needs the Screen scope (api/devices.rs), and the same policy that lets a
            // phone control this computer: "Only this computer" keeps stopping to the computer.
            if !matches!(caller, Caller::Local) {
                let policy = crate::server_settings::Settings::load(&hub.store).remote_control;
                if !crate::server_settings::allows_remote(policy.access, crate::server_settings::RemoteRole::Device) {
                    bail!("This computer only allows remote control from itself.");
                }
            }
            let stopped = hub.screen.stop_all().await?;
            json!({"closed": stopped.viewers, "takeover": stopped.takeover, "screen": hub.screen.state()})
        }
        "screenPrepare" => {
            if !hub.screen.enabled() {
                bail!("Remote screen is turned off on this computer.");
            }
            // Who may control this computer, and whether the person here allows each session.
            let policy = crate::server_settings::Settings::load(&hub.store).remote_control;
            let role = match caller {
                Caller::Local => crate::server_settings::RemoteRole::Owner,
                _ => crate::server_settings::RemoteRole::Device,
            };
            if !crate::server_settings::allows_remote(policy.access, role) {
                bail!("This computer only allows remote control from itself.");
            }
            if policy.ask_on_host
                && role == crate::server_settings::RemoteRole::Device
                && !hub.screen.take_approval(caller.device_key())
            {
                hub.screen.await_consent(caller.device_key()).await?;
            }
            let ice = if b["relay"].as_bool().unwrap_or(false) {
                crate::remote::cloud::screen_ice(hub, caller.device_key()).await?
            } else {
                crate::screen::IceConfig::default()
            };
            let result = hub.screen.prepare(caller.device_key(), ice, b["trickle"].as_bool().unwrap_or(false))?;
            let session = result["session"].as_str().context("missing screen session")?.to_owned();
            tokio::spawn(crate::screen::watch_viewer(hub.clone(), session, caller.device_key().to_owned()));
            result
        }
        "screenOffer" => {
            let display = b["display"].as_u64().and_then(|d| u32::try_from(d).ok());
            hub.screen.offer(caller.device_key(), str_arg(&b, "sdp")?, str_arg(&b, "session")?, display).await?
        }
        "screenCandidate" => {
            let candidate = serde_json::from_value(b["candidate"].clone())?;
            hub.screen.candidate(str_arg(&b, "session")?, caller.device_key(), candidate).await?;
            json!({})
        }
        "screenClose" => {
            let session = str_arg(&b, "session")?;
            if hub.screen.owns_session(session, caller.device_key()) {
                hub.screen.close(session).await?;
            }
            json!({})
        }
        "screenTakeover" => {
            hub.screen.takeover(b["on"].as_bool().unwrap_or(false));
            hub.screen.state()
        }
        "setAnalytics" => {
            hub.analytics.set_enabled(&hub.store, b["enabled"].as_bool().unwrap_or(false))?;
            crate::remote::managed::share_analytics(hub);
            json!({"enabled": hub.analytics.state()})
        }
        "track" => {
            crate::analytics::track(hub, &b)?;
            json!({})
        }
        "reportError" => {
            crate::analytics::errors::report(hub, &b)?;
            json!({})
        }
        "setScreenEnabled" => {
            hub.screen.set_enabled(&hub.store, b["enabled"].as_bool().unwrap_or(false)).await?;
            hub.screen.state()
        }
        "n8nStatus" => crate::n8n_remote::view(caller, hub.n8n.state()),
        "n8nStart" => {
            hub.n8n.enable(hub)?;
            crate::n8n_remote::view(caller, hub.n8n.state())
        }
        "n8nStop" => {
            hub.n8n.stop(&hub.store).await;
            crate::n8n_remote::view(caller, hub.n8n.state())
        }
        "setN8nEnabled" => {
            if b["enabled"].as_bool().context("`enabled` must be a boolean")? {
                hub.n8n.enable(hub)?;
            } else {
                hub.n8n.disable(&hub.store).await?;
            }
            crate::n8n_remote::view(caller, hub.n8n.state())
        }
        // A phone's view of the workflows n8n runs here: list them, publish or unpublish one, read its runs.
        "n8nWorkflows" => crate::n8n_remote::workflows(hub).await?,
        "n8nSetWorkflowActive" => {
            let active = b["active"].as_bool().context("`active` must be a boolean")?;
            crate::n8n_remote::set_workflow_active(hub, str_arg(&b, "id")?, active).await?
        }
        "n8nExecutions" => crate::n8n_remote::executions(hub, b["workflowId"].as_str(), b["limit"].as_u64()).await?,
        "setN8nPath" => {
            hub.n8n.set_path(&hub.store, str_arg(&b, "path")?).await?;
            hub.n8n.state()
        }
        "setN8nPort" => {
            let port = b["port"].as_u64().context("`port` is required")?;
            hub.n8n.set_port(&hub.store, u16::try_from(port).context("`port` must be 1024 or higher")?).await?;
            hub.n8n.state()
        }
        "toolStatus" => hub.tools.status(&hub.store, str_arg(&b, "id")?).await?,
        "toolStart" => {
            hub.tools.start(hub, str_arg(&b, "id")?)?;
            hub.tools.status(&hub.store, str_arg(&b, "id")?).await?
        }
        "toolStop" => {
            hub.tools.stop(&hub.store, str_arg(&b, "id")?).await?;
            hub.tools.status(&hub.store, str_arg(&b, "id")?).await?
        }
        "comfyCall" => {
            let args = b.get("arguments").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}));
            crate::tools::comfy_mcp::call(hub, str_arg(&b, "botId")?, str_arg(&b, "name")?, &args).await?
        }
        "comfyCancel" => hub.tools.comfy_jobs().cancel(hub).await?,
        "comfyJobImage" => crate::tools::comfy_mcp::job_image(hub).await?,
        "computerTools" => crate::screen::computer_tools(hub).await,
        "computerCall" => {
            let args = b.get("arguments").filter(|a| a.is_object()).cloned().unwrap_or_else(|| json!({}));
            crate::screen::computer(hub, str_arg(&b, "botId")?, str_arg(&b, "name")?, args).await?
        }
        _ => return Ok(ControlFlow::Continue(b)),
    }))
}

fn term_size(b: &Value) -> (u16, u16) {
    let dim = |k: &str, default: u16| b[k].as_u64().and_then(|v| u16::try_from(v).ok()).unwrap_or(default);
    (dim("cols", 80), dim("rows", 24))
}
