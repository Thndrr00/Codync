//! The built-in `routines` MCP server, scoped to one bot: its tools and instructions.
use super::{Arc, Context, Hub, Result, Value, bail, json, required};

/// One case's text is prompt data for the first steps, not a new routine: keep it bounded.
const MAX_CASE_TEXT_CHARS: usize = 4_000;

/// The event names the agent-facing tool may use, from the same catalog the editor offers. OpenBot 0.75.0
/// (#12833): an agent could save a name such as `pr.opened` for a trigger the host couldn't match, the routine
/// saved without error, and no delivery ever started it. The refusal names the real events so the agent corrects
/// the trigger in the same chat.
fn event_names(source: &str) -> Option<Vec<&'static str>> {
    crate::routines::events::source(source).map(|s| s.events.iter().map(|e| e.id).collect())
}

/// Whether a request asks for work at a set time or on a repeat, so it belongs in a Codync routine
/// the user can see in Routines rather than an OpenCode automation nobody can find (OpenBot 0.78.0).
/// Managing an old automation is not a new schedule: those requests stay with the automation tools.
pub fn automation_request(text: &str) -> bool {
    let lower = format!(" {} ", text.trim().to_lowercase());
    let managing = ["list", "show", "pause", "resume", "delete", "remove", "stop", "cancel", "edit"]
        .iter()
        .any(|verb| lower.contains(verb))
        && lower.contains("automation");
    if managing {
        return false;
    }
    let repeating = [
        "every ",
        "each day",
        "each week",
        "each month",
        "daily",
        "weekly",
        "monthly",
        "hourly",
        "nightly",
        "annually",
    ]
    .iter()
    .any(|cue| lower.contains(cue));
    let dated = [
        "schedule",
        "cron",
        "remind me",
        "tomorrow",
        "tonight",
        "at 8",
        "at 9",
        "at 10",
        "at 11",
        "at 12",
        "at noon",
        "at midnight",
        "next monday",
        "on friday",
    ]
    .iter()
    .any(|cue| lower.contains(cue));
    repeating || dated
}

/// The tool answer for `automation_intent`, kept beside the decision so the tool and its test agree.
pub fn automation_intent(args: &Value) -> Value {
    json!({"routine": automation_request(args["text"].as_str().unwrap_or_default())})
}

/// Checks every event trigger of a save against the catalog. Sources outside the catalog keep their event as
/// they are (bot-set and older triggers); a known source must name one of the events it can actually send.
pub fn check_event_triggers(args: &Value) -> Result<()> {
    let triggers = args["triggers"].as_array().into_iter().flatten();
    let from_draft = args["schedule"]["kind"] == "event";
    let drafted = if from_draft {
        vec![json!({"type":"event", "source": args["schedule"]["eventSource"], "event": args["schedule"]["eventName"]})]
    } else {
        vec![]
    };
    for (index, trigger) in triggers.chain(drafted.iter()).enumerate() {
        if trigger["type"] != "event" {
            continue;
        }
        let Some(source) = trigger["source"].as_str().filter(|s| !s.is_empty()) else { continue };
        let Some(events) = event_names(source) else { continue };
        let event = trigger["event"].as_str().unwrap_or_default();
        if event == "*" || events.iter().any(|known| *known == event) {
            continue;
        }
        let given = if event.is_empty() { "has no event".to_owned() } else { format!("has event \"{event}\"") };
        bail!("Event trigger {}: the {source} trigger {given}. Set event to one of: {}.", index + 1, events.join(", "));
    }
    Ok(())
}

pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    match name {
        "automation_intent" => Ok(automation_intent(args)),
        "list_routine_templates" => Ok(super::blueprints::list()),
        "routine_template_draft" => super::blueprints::draft(args),
        "list_routines" => Ok(hub.routines.list(bot, |_| false)),
        "save_routine" => {
            check_event_triggers(args)?;
            hub.routines.save(hub, bot, args)
        }
        "set_routine_enabled" => hub.routines.set_enabled(
            &hub.store,
            bot,
            required(args, "id")?,
            args["enabled"].as_bool().context("enabled is required")?,
        ),
        "delete_routine" => hub.routines.remove(hub, bot, required(args, "id")?),
        "run_routine" => {
            let id = required(args, "id")?;
            if !hub.routines.owns(bot, id) {
                bail!(
                    "routine not found among yours: for another bot's routine, ask that bot with message_bot to run \
                     it with its own run_routine"
                );
            }
            // OpenBot 0.74.0: for a run over one case, the agent can hand that case's text to the
            // run. The flow gives it to its first steps only; the saved workflow is untouched.
            let mut event = json!({"source":"test"});
            if let Some(case) = args["caseText"].as_str().map(str::trim).filter(|text| !text.is_empty()) {
                event["caseText"] = json!(case.chars().take(MAX_CASE_TEXT_CHARS).collect::<String>());
            }
            hub.routines.enqueue(&hub.store, bot, id, event, None, true)
        }
        "routine_webhook" => {
            hub.routines.credentials(hub, bot, required(args, "id")?, args["rotate"].as_bool().unwrap_or(false))
        }
        // "Give the signing secret to <agent>" (OpenBot 0.74.0): rotate the routine's key and save it
        // in this agent's vault. The value never reaches the tool result or the model.
        "give_webhook_secret" => hub.routines.give_webhook_secret(hub, bot, required(args, "id")?),
        // OpenBot messaging.md: a routine can post to a connected place from the agent's own
        // chat. The same pure target and allowlist guard the chat tool uses applies here.
        "messaging_post" | "messaging_list_places" => {
            crate::chat::memory::messaging::routine_call(hub, bot, name, args)
        }
        n if super::flow::TOOLS.contains(&n) => super::flow::tool_call(hub, bot, n, args),
        _ => bail!("unknown routine tool"),
    }
}

pub const INSTRUCTIONS: &str = "Create and manage persistent Codync routines when the user asks for scheduled or event-driven work. Use the built-in routines tools (save_routine, list_routines, set_routine_enabled, delete_routine, run_routine, routine_webhook, give_webhook_secret), not operating-system cron or an improvised background process. For recurring clock schedules, always use a five-field cron trigger with an explicit IANA timezone, including minute/hour frequencies; do not create interval triggers for those requests. Preserve existing intervals when editing unrelated fields. Cron steps reset within their field: */7 minutes is not an exact seven-minute duration across hours. Use once for a single dated execution; never approximate a one-time task with an annually repeating cron expression. For requests like when this file, folder or page changes, use a watch trigger rather than a frequent schedule that checks itself: it costs nothing until something changes. Before saving, resolve the task, trigger and intended timezone for calendar/one-time schedules from the conversation; ask only for missing or ambiguous details. If the request is already clear, create it without another confirmation. Check that tools, connector sign-ins and files needed by the task are available; describe any missing setup rather than promising the task will work. Use list_routines before creating or editing to avoid duplicates; preserve unrelated triggers, enabled state and instructions. Save returns the actual routine ID, trigger descriptions and nextRunAt: use these to report what was really saved, including timezone, next occurrence and enabled/paused state. Do not claim success if the tool failed. run_routine executes the real task immediately, including its side effects; use it when the user, or another bot, asks for a test or immediate run of one of your routines, not merely to validate a schedule. You run only your own routines: for another bot's, ask that bot with message_bot to run it with its own run_routine. A routine runs locally in a session of its own; the host must be running and awake, its result appears in your chat with the user, and (pass) stays silent. A run that fails before using a tool that changes anything is retried automatically 1, 5 and 15 minutes later; one that already acted is left for the user to retry from its failure notice. Webhook triggers: routine_webhook explains how senders deliver; never claim a third-party integration is connected until a real delivery has been verified. Keep secrets out of routine instructions and ordinary status summaries. To have a routine pass its result on to other bots (a routine flow), use link_routine_bots with the routine, the bot to receive the reply (an id from list_bots) and what that bot should do with it; chain a third bot with fromBotId. A flow cannot loop or reach more than a few bots deep, and only a bot with an agent of its own can take part, not a group or a terminal session. A link applies from the routine's next run. unlink_routine_bots removes a link and list_routine_flow shows links and the latest steps; tell the user what you linked, not that the downstream bots finished before a run has shown it. OpenCode has its own automation tool, but an automation it creates is invisible in Codync Routines: for work at a set time or on a repeat, call automation_intent with the user's request and, when it says routine is true, save a routine here instead. You can still use the automation tools to list, pause or delete old automations; that is not a new schedule. A routine can post to a connected WhatsApp, Telegram, Slack or Discord place with messaging_post: list places first with messaging_list_places and post only to a destination it lists; secrets are redacted before the post leaves the host.";

pub fn tools() -> Value {
    let id = json!({"type":"string"});
    let mut tools = vec![
        json!({"name":"automation_intent","description":"Check whether a request for work at a set time or on a repeat belongs in Codync Routines instead of an automation the user can't see. Returns {routine: true} for a timed or repeated task; listing, pausing or deleting an old automation returns false.","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]},"annotations":{"readOnlyHint":true}}),
        json!({"name":"list_routines","description":"List this bot's routines and run history.","inputSchema":{"type":"object","properties":{}}}),
    ];
    tools.push(json!({"name":"save_routine","description":"Create or replace a routine. Omit id to create; include id to update. triggers is an OR list. Use cron for recurring clock schedules; interval is for explicitly requested elapsed-duration semantics or existing schedules: interval {type:interval,seconds}; once {type:once,at:Unix milliseconds}; cron {type:cron,expression:five cron fields,timeZone:IANA zone}; webhook {type:webhook}; event {type:event,source:slack/telegram/whatsapp/discord/github/origin/microsoftTeams/linear/sentry/pagerduty/email,event:event name or *,filters:field-to-value map}; watch {type:watch,kind:files or url,target:a file or folder path (relative to your working folder) or an https URL,everyMinutes:1-10080}: checked without you, it runs only when the target changed since the last check (the first check only records it; folders up to 2000 entries, .git and node_modules skipped). Events require a configured webhook sender. Minimum interval is one second.","inputSchema":{"type":"object","properties":{"id":id,"name":{"type":"string"},"instruction":{"type":"string"},"enabled":{"type":"boolean"},"timeoutSeconds":{"type":"integer","minimum":1,"maximum":86400},"triggers":{"type":"array","items":{"type":"object"}}},"required":["name","instruction","triggers"]}}));
    if let Some(save) = tools.iter_mut().find(|tool| tool["name"] == "save_routine") {
        // OpenBot 0.75.0 (#12833): name the events of each trigger source, so the agent saves a trigger that
        // can actually fire instead of an invented name the host silently ignores.
        let events = crate::routines::events::SOURCES
            .iter()
            .map(|source| {
                format!("{}: {}", source.id, source.events.iter().map(|event| event.id).collect::<Vec<_>>().join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ");
        save["description"] = json!(format!(
            "{} An event trigger must use one of the events its source can send: {}.",
            save["description"].as_str().unwrap_or_default(),
            events
        ));
        save["inputSchema"]["properties"]["outputs"] = json!({"type":"array","maxItems":10,
            "description":"Optional report destinations after the last workflow step. WhatsApp reuses this computer's existing linked number: target is the recipient JID, account is the linked sender JID (not the bot id). Link it before creating the run. Pending reports are revoked when that link is replaced or logged out. Do not retry an unknown send outcome.",
            "items":{"type":"object","additionalProperties":false,"required":["kind","target"],"properties":{
                "kind":{"type":"string","enum":["slack","telegram","whatsapp","discord","discordReact","githubComment","githubIssue","githubLabel","githubAssign","githubReview"]},
                "target":{"type":"string"},"account":{"type":"string"},"value":{"type":"string"}}}});
    }
    for (name, description) in [
        ("set_routine_enabled", "Pause or resume a routine; enabled boolean is required."),
        ("delete_routine", "Delete a routine and cancel pending runs."),
        (
            "run_routine",
            "Start a test run of one of your routines, including a paused one. caseText is the text of the single case \
             to run it over: the first steps of the routine's workflow get it, the workflow itself is not changed.",
        ),
        (
            "routine_webhook",
            "Get the routine's webhook: its public url (through the Codync cloud; null when the cloud is off, then only localUrl on this computer works), localUrl and per-routine key. Senders POST with Authorization: Bearer <key>; for GitHub, set the webhook's content type to application/json and its secret to the key (deliveries are verified by X-Hub-Signature-256) and they become events {source:github, event:<event>.<action>, repo, sender, text, url, payload} that event triggers can filter. Deliveries made while the computer is offline wait up to 72 hours in the cloud, which sees their contents (not end-to-end encrypted). rotate:true replaces the key; the old one stops working at once. This doesn't register a provider subscription: set that up with the user, and report the webhook as awaiting connection until then.",
        ),
        (
            "give_webhook_secret",
            "Give a webhook routine's signing secret to this agent as one of its own vault keys (list_secrets shows its name and destination; use it with run_with_secrets). Rotates the secret first, so the old one stops working, and later rotations keep the key in sync. Call this only when the user asks you to take over the sender's secret; never print or paste the value.",
        ),
    ] {
        let required = if name == "set_routine_enabled" { vec!["id", "enabled"] } else { vec!["id"] };
        let mut properties = json!({"id": id});
        match name {
            "set_routine_enabled" => properties["enabled"] = json!({"type":"boolean"}),
            "run_routine" => {
                properties["caseText"] = json!({"type":"string", "description":"The one case to run over, given to the first workflow steps only."})
            }
            "routine_webhook" => properties["rotate"] = json!({"type":"boolean"}),
            _ => {}
        }
        tools.push(json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required}}));
    }
    tools.extend(super::flow::tool_list());
    tools.extend(crate::chat::memory::messaging::routine_tools());
    tools.push(json!({"name":"list_routine_templates", "description":"List the bundled Hermes routine templates and their fields. Draft one with routine_template_draft, then save its result with save_routine.", "inputSchema":{"type":"object","properties":{}}, "annotations":{"readOnlyHint":true}}));
    tools.push(json!({"name":"routine_template_draft", "description":"Validate and fill a Hermes routine template. Returns a disabled Codync routine draft; save it with save_routine after resolving its destination and timezone with the user.", "inputSchema":{"type":"object","properties":{"template":{"type":"string"},"values":{"type":"object","additionalProperties":{"type":"string"}},"timeZone":{"type":"string"},"destination":{"type":"string"},"account":{"type":"string"}},"required":["template","timeZone"]}, "annotations":{"readOnlyHint":true}}));
    json!(tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timed_or_repeated_request_becomes_a_routine_but_managing_automations_does_not() {
        for request in [
            "remind me to take a pill every day at 9am",
            "make a weekly digest of the team's issues",
            "run a cron job that checks prices hourly",
            "schedule a scan for tomorrow at noon",
        ] {
            assert!(automation_request(request), "{request}");
        }
        for request in [
            "list my automations",
            "pause the daily automation",
            "delete that automation",
            "summarize this file for me",
            "run the task now",
        ] {
            assert!(!automation_request(request), "{request}");
        }
        assert_eq!(
            automation_intent(&json!({"text":"save a daily report"}))["routine"],
            serde_json::Value::Bool(true),
            "the tool reports the decision"
        );
    }

    #[test]
    fn an_event_trigger_must_name_an_event_its_source_can_send() {
        let valid = json!({"triggers": [
            {"type":"event", "source":"github", "event":"pull_request.opened"},
            {"type":"event", "source":"slack", "event":"*"}
        ]});
        assert!(check_event_triggers(&valid).is_ok());

        let wrong = json!({"triggers": [{"type":"event", "source":"github", "event":"pr.opened"}]});
        let error = check_event_triggers(&wrong).expect_err("an event the host can't match is refused");
        assert!(error.to_string().contains("pr.opened"));
        assert!(error.to_string().contains("pull_request.opened"), "the refusal lists the real events");

        let missing = json!({"triggers": [{"type":"event", "source":"discord"}]});
        assert!(check_event_triggers(&missing).is_err(), "a missing event is refused too");

        // Bot-set sources and older triggers keep their event as-is (the editor has the same rule).
        let external = json!({"triggers": [{"type":"event", "source":"origin", "event":"pull_request.opened"}]});
        assert!(check_event_triggers(&external).is_ok());
        let non_event = json!({"triggers": [{"type":"cron", "expression":"* * * * *"}]});
        assert!(check_event_triggers(&non_event).is_ok());
    }

    #[test]
    fn the_routines_instructions_point_a_timed_task_at_a_visible_routine() {
        assert!(INSTRUCTIONS.contains("automation_intent"), "the agent is told to check first");
        assert!(INSTRUCTIONS.contains("invisible in Codync Routines"));
        assert!(INSTRUCTIONS.contains("list, pause or delete old automations"));
        assert!(
            tools().as_array().expect("tools").iter().any(|tool| tool["name"] == "automation_intent"),
            "the decision has a tool the agent can call"
        );
    }

    #[test]
    fn a_routine_gets_the_same_post_tool_as_the_agents_chat() {
        let routine = tools().as_array().expect("tools").iter().find(|t| t["name"] == "messaging_post").expect("post");
        let chat =
            crate::chat::memory::messaging::tools().into_iter().find(|t| t["name"] == "messaging_post").expect("post");
        assert_eq!(routine["inputSchema"], chat["inputSchema"], "one tool, one guard");
        assert!(INSTRUCTIONS.contains("messaging_post"));
    }

    #[test]
    fn the_save_tool_names_the_events_of_each_source() {
        let save =
            tools().as_array().expect("tools").iter().find(|t| t["name"] == "save_routine").expect("save").clone();
        let description = save["description"].as_str().expect("description");
        assert!(description.contains("github: *,"), "the catalog names the real GitHub events");
        assert!(description.contains("pull_request.opened"));
        assert!(description.contains("slack: *,"));
    }
}
