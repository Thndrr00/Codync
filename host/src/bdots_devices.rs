#![allow(dead_code, clippy::pedantic, clippy::style, clippy::perf)]

//! Pure helpers ported from agentes-chat's `devices.ts`: device status, portraits, audiences, list
//! rows, connect links, Android steps and Wi-Fi pairing. Composes helpers already ported into this
//! host. Only `botInsights` calls this.

use serde_json::{Map, Value, json};

/// Status pill looks, the same on every screen (StatusLook).
pub const STATUS_LOOKS: [&str; 8] =
    ["ready", "inuse", "busy", "needs", "preparing", "paused", "offline", "unsupported"];

/// Filters of the devices list (DeviceFilter).
pub const DEVICE_FILTERS: [&str; 3] = ["all", "computers", "phones"];

/// Steps of the Android cable guide, in order (ANDROID_STEPS).
pub const ANDROID_STEPS: [&str; 4] = ["devOptions", "usbDebugging", "plug", "allow"];

/// States of a guide step (StepState).
pub const STEP_STATES: [&str; 4] = ["done", "current", "waiting", "needs"];

/// How often a connect dialog polls its link (LINK_POLL_MS).
pub const LINK_POLL_MS: u64 = 2_000;

/// How often a Wi-Fi pairing is polled (WIFI_PAIR_POLL_MS).
pub const WIFI_PAIR_POLL_MS: u64 = 1_500;

/// Pill look of a device status (statusLook).
pub fn status_look(kind: &str) -> &'static str {
    match kind {
        "in_use" => "inuse",
        "needs_you" => "needs",
        "busy" | "preparing" | "paused" | "offline" | "unsupported" => kind,
        _ => "ready",
    }
}

/// SF Symbol for a status look.
pub fn status_icon(look: &str) -> &'static str {
    match look {
        "inuse" => "person.crop.circle.fill.badge.checkmark",
        "busy" => "bolt.fill",
        "needs" => "exclamationmark.triangle.fill",
        "preparing" => "gearshape.arrow.triangle.2.circlepath",
        "paused" => "pause.circle.fill",
        "offline" => "wifi.slash",
        "unsupported" => "xmark.octagon.fill",
        _ => "checkmark.circle.fill",
    }
}

/// Semantic tone for a status look.
pub fn status_tone(look: &str) -> &'static str {
    match look {
        "inuse" | "busy" | "preparing" => "accent",
        "needs" | "unsupported" => "attention",
        "paused" | "offline" => "secondary",
        _ => "good",
    }
}

/// Known reason codes the apps word (reasonKey).
pub fn reason_key<'a>(reason: &'a str, os: Option<&str>) -> &'a str {
    const REASONS: [&str; 17] = [
        "locked",
        "unauthorized",
        "untrusted",
        "developer_mode_off",
        "ui_automation_off",
        "xcode_missing",
        "xcode_too_old",
        "device_support",
        "helper_missing",
        "helper_expired",
        "apple_account_missing",
        "apple_team_missing",
        "ui_not_authorized",
        "team_limit",
        "helper_untrusted",
        "booting",
        "no_driver",
    ];
    if !REASONS.contains(&reason) {
        "other"
    } else if reason == "no_driver" && os == Some("linux") {
        "no_driverLinux"
    } else {
        reason
    }
}

/// Pixel portrait kind and state of a device (portraitOf).
pub fn portrait_of(device: &Value) -> Value {
    let kind_status = device["status"]["kind"].as_str().unwrap_or("");
    let state = if kind_status == "offline" {
        "offline"
    } else if kind_status == "paused" || (device.get("platform").is_some() && kind_status == "needs_you") {
        "paused"
    } else {
        "online"
    };
    let kind = if let Some(platform) = device["platform"].as_str() {
        if platform == "ios" { "iphone" } else { "android" }
    } else if device["machineKind"].as_str() == Some("managed") {
        "linux"
    } else {
        match device["os"].as_str().unwrap_or("") {
            "darwin" => "mac",
            "windows" => "windows",
            _ => "linux",
        }
    };
    json!({"kind": kind, "state": state})
}

/// What a device is called on screen (deviceName).
pub fn device_name(device: &Value) -> String {
    let name = device["name"].as_str().unwrap_or("");
    if !name.is_empty() {
        return name.to_owned();
    }
    if device["machineKind"].as_str() == Some("host") {
        "This computer".to_owned()
    } else {
        device["hostname"].as_str().unwrap_or("").to_owned()
    }
}

/// Human name of an OS code (osName).
pub fn os_name(os: &str) -> String {
    match os {
        "darwin" => "macOS".to_owned(),
        "windows" => "Windows".to_owned(),
        "linux" => "Linux".to_owned(),
        "other" => "Other".to_owned(),
        _ => os.to_owned(),
    }
}

/// "macOS · 12 CPU · 36 GB" for a computer row (computerLine).
pub fn computer_line(device: &Value) -> String {
    let mut parts = Vec::new();
    parts.push(if device["machineKind"].as_str() == Some("managed") {
        "Cloud".to_owned()
    } else {
        os_name(device["os"].as_str().unwrap_or(""))
    });
    if let Some(cpus) = device["cpus"].as_u64().filter(|n| *n > 0) {
        parts.push(format!("{cpus} CPU"));
    }
    if let Some(memory) = device["memoryGb"].as_u64().filter(|n| *n > 0) {
        parts.push(format!("{memory} GB"));
    }
    parts.join(" · ")
}

/// How a phone is reached: USB, Wi-Fi, Emulator or Simulator (phoneLink).
pub fn phone_link<'a>(phone: &'a Value) -> Option<&'a str> {
    let kind = phone["phoneKind"].as_str().unwrap_or("device");
    if kind != "device" {
        return Some(match kind {
            "emulator" => "Emulator",
            "simulator" => "Simulator",
            other => other,
        });
    }
    match phone["transport"].as_str().unwrap_or("") {
        "usb" => Some("USB"),
        "wifi" => Some("Wi-Fi"),
        _ => None,
    }
}

/// Narrowness rank of an audience rule.
pub fn audience_rank(mode: &str) -> u8 {
    match mode {
        "owner" => 0,
        "people" => 1,
        _ => 2,
    }
}

/// Icon of an audience rule (AudienceIcon).
pub fn audience_icon(mode: &str) -> &'static str {
    match mode {
        "everyone" => "audienceEveryone",
        "owner" => "audienceOwner",
        _ => "audiencePeople",
    }
}

/// Whether a rule only narrows `from` to `to` (narrowsAudience).
pub fn narrows_audience(from: &Value, to: &Value) -> bool {
    let from_mode = from["mode"].as_str().unwrap_or("");
    let to_mode = to["mode"].as_str().unwrap_or("");
    if to_mode == "owner" || from_mode == "everyone" {
        return true;
    }
    if to_mode == "everyone" || from_mode == "owner" {
        return false;
    }
    let contains = |array: &Value, id: &str| {
        array.as_array().is_some_and(|values| values.iter().any(|value| value.as_str() == Some(id)))
    };
    to["userIds"]
        .as_array()
        .is_none_or(|ids| ids.iter().all(|id| id.as_str().is_none_or(|value| contains(&from["userIds"], value))))
        && to["roleIds"]
            .as_array()
            .is_none_or(|ids| ids.iter().all(|id| id.as_str().is_none_or(|value| contains(&from["roleIds"], value))))
}

/// Two audience rules are the same (sameAudience).
pub fn same_audience(a: &Value, b: &Value) -> bool {
    let mode = a["mode"].as_str().unwrap_or("");
    if mode != b["mode"].as_str().unwrap_or("") {
        return false;
    }
    if mode != "people" {
        return true;
    }
    let same_set = |left: &Value, right: &Value| {
        let left =
            left.as_array().map_or(Vec::new(), |values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>());
        let right =
            right.as_array().map_or(Vec::new(), |values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>());
        left.len() == right.len() && left.iter().all(|id| right.contains(id))
    };
    same_set(&a["userIds"], &b["userIds"]) && same_set(&a["roleIds"], &b["roleIds"])
}

/// Workspace computer first, then the server's order (sortDevices).
pub fn sort_devices(list: &[Value]) -> Vec<Value> {
    let mut out = list.to_vec();
    out.sort_by(|a, b| {
        let managed = |value: &Value| u8::from(value["machineKind"].as_str() == Some("managed"));
        managed(b).cmp(&managed(a))
    });
    out
}

/// Counts of all devices, computers and phones (deviceCounts).
pub fn device_counts(list: &[Value]) -> Value {
    let computers = list.len();
    let phones = list.iter().map(|device| device["phones"].as_array().map_or(0, Vec::len)).sum::<usize>();
    json!({"all": computers + phones, "computers": computers, "phones": phones})
}

/// Rows shown by the devices list, phones nested under their host (deviceRows).
pub fn device_rows(list: &[Value], filter: &str, query: &str) -> Vec<Value> {
    let q = query.trim().to_ascii_lowercase();
    let matches =
        |needle: Option<&str>| q.is_empty() || needle.is_some_and(|value| value.to_ascii_lowercase().contains(&q));
    let mut rows = Vec::new();
    for device in sort_devices(list) {
        let self_match =
            matches(device["name"].as_str()) || matches(device["hostname"].as_str()) || matches(device["os"].as_str());
        let phones: Vec<Value> = device["phones"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter(|phone| self_match || matches(phone["name"].as_str()) || matches(phone["model"].as_str()))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if filter == "phones" {
            for phone in phones {
                rows.push(json!({"kind": "phone", "device": phone, "host": device, "nested": false}));
            }
            continue;
        }
        if !self_match && phones.is_empty() {
            continue;
        }
        rows.push(json!({"kind": "computer", "device": device}));
        if filter == "all" {
            for phone in phones {
                rows.push(json!({"kind": "phone", "device": phone, "host": device, "nested": true}));
            }
        }
    }
    rows
}

/// A phone by id with its host, or null (findPhone).
pub fn find_phone(list: &[Value], phone_id: &str) -> Value {
    for host in list {
        if let Some(phones) = host["phones"].as_array() {
            if let Some(phone) = phones.iter().find(|phone| phone["id"].as_str() == Some(phone_id)) {
                return json!({"phone": phone, "host": host});
            }
        }
    }
    Value::Null
}

/// Where a connect link stands, including local expiry (linkPhase).
pub fn link_phase<'a>(status: Option<&'a str>, expires_at_ms: i64, now_ms: i64) -> &'a str {
    let status = status.unwrap_or("waiting");
    if status == "waiting" && now_ms >= expires_at_ms {
        "expired"
    } else {
        match status {
            "waiting" | "asking" | "connected" | "expired" | "failed" => status,
            _ => "waiting",
        }
    }
}

/// Whether a link phase still polls (linkPolls).
pub fn link_polls(phase: &str) -> bool {
    phase == "waiting" || phase == "asking"
}

/// Where plugging an Android phone stands (androidStage).
pub fn android_stage(host: &Value, known: &[String]) -> Value {
    let phones: Vec<&Value> = host["phones"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter(|phone| {
                    phone["platform"].as_str() == Some("android")
                        && phone["phoneKind"].as_str() == Some("device")
                        && phone["status"]["kind"].as_str() != Some("offline")
                })
                .collect()
        })
        .unwrap_or_default();
    let waits_on = |phone: &&Value, reason: &str| {
        phone["status"]["kind"].as_str() == Some("needs_you") && phone["status"]["reason"].as_str() == Some(reason)
    };
    if let Some(phone) = phones.iter().find(|phone| waits_on(phone, "unauthorized")) {
        return json!({"kind": "allow", "phone": phone});
    }
    if let Some(phone) = phones.iter().find(|phone| waits_on(phone, "no_driver")) {
        return json!({"kind": "driver", "phone": phone});
    }
    if let Some(phone) =
        phones.iter().find(|phone| phone["id"].as_str().is_none_or(|id| !known.iter().any(|known| known == id)))
    {
        let kind = if phone["status"]["kind"].as_str() == Some("needs_you") { "allow" } else { "ready" };
        return json!({"kind": kind, "phone": phone});
    }
    json!({"kind": "looking", "phone": Value::Null})
}

/// The cable guide's checklist (androidSteps).
pub fn android_steps(stage: &str, ticked: &[String], asked: Option<&str>) -> Value {
    let seen = stage != "looking";
    let done = |step: &str| match step {
        "allow" => stage == "ready",
        "plug" => stage == "allow" || stage == "ready",
        _ => seen || ticked.iter().any(|value| value == step),
    };
    let now = ANDROID_STEPS.iter().copied().find(|step| !done(step)).unwrap_or("allow");
    let mut states = Map::new();
    for step in ANDROID_STEPS {
        let state = if done(step) {
            "done"
        } else if step != now {
            "waiting"
        } else if seen {
            "needs"
        } else {
            "current"
        };
        states.insert(step.to_owned(), Value::String(state.to_owned()));
    }
    let by_hand: Vec<&str> = if seen {
        Vec::new()
    } else {
        ticked
            .iter()
            .filter(|step| matches!(step.as_str(), "devOptions" | "usbDebugging"))
            .map(String::as_str)
            .collect()
    };
    json!({
        "states": Value::Object(states),
        "now": now,
        "open": asked.unwrap_or(now),
        "byHand": by_hand,
    })
}

/// Whether a Wi-Fi pairing still runs (wifiPairLive).
pub fn wifi_pair_live(state: Option<&str>) -> bool {
    matches!(state, Some("waiting" | "pairing" | "connecting"))
}

/// Compact count label, reusing the shared formatter.
pub fn count_label(n: usize) -> String {
    crate::bdots_desk::compact_count(n as u64)
}

/// Summary rows for the card.
pub fn summary(devices: usize, phones: usize, online: usize, looking: bool) -> Vec<(&'static str, String)> {
    vec![
        ("devices", count_label(devices)),
        ("phones", count_label(phones)),
        ("online", count_label(online)),
        ("looking", if looking { "yes" } else { "no" }.to_owned()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_and_reasons() {
        assert_eq!(status_look("in_use"), "inuse");
        assert_eq!(status_look("needs_you"), "needs");
        assert_eq!(status_look("ready"), "ready");
        assert!(STATUS_LOOKS.contains(&status_look("busy")));
        assert_eq!(status_icon("needs"), "exclamationmark.triangle.fill");
        assert_eq!(status_tone("offline"), "secondary");
        assert_eq!(reason_key("no_driver", Some("linux")), "no_driverLinux");
        assert_eq!(reason_key("unknown", None), "other");
    }

    #[test]
    fn portraits_and_lines() {
        let device = json!({"name":"Mac","os":"darwin","cpus":12,"memoryGb":36,"status":{"kind":"ready"}});
        assert_eq!(portrait_of(&device)["kind"], "mac");
        assert_eq!(portrait_of(&device)["state"], "online");
        assert_eq!(device_name(&device), "Mac");
        assert_eq!(os_name("darwin"), "macOS");
        assert_eq!(computer_line(&device), "macOS · 12 CPU · 36 GB");
        let phone = json!({"platform":"ios","phoneKind":"device","transport":"usb","name":"iPhone"});
        assert_eq!(portrait_of(&phone)["kind"], "iphone");
        assert_eq!(phone_link(&phone), Some("USB"));
        assert_eq!(phone_link(&json!({"phoneKind":"simulator"})), Some("Simulator"));
    }

    #[test]
    fn audiences() {
        assert_eq!(audience_rank("owner"), 0);
        assert_eq!(audience_icon("people"), "audiencePeople");
        let everyone = json!({"mode":"everyone","userIds":[],"roleIds":[]});
        let owner = json!({"mode":"owner","userIds":[],"roleIds":[]});
        assert!(narrows_audience(&everyone, &owner));
        assert!(!narrows_audience(&owner, &everyone));
        assert!(same_audience(&owner, &json!({"mode":"owner"})));
    }

    #[test]
    fn lists_and_links() {
        let devices = vec![
            json!({"id":"1","name":"Mac","os":"darwin","phones":[{"id":"p1","name":"iPhone","model":"15"}]}),
            json!({"id":"2","name":"Cloud","machineKind":"managed","os":"linux","phones":[]}),
        ];
        assert_eq!(sort_devices(&devices)[0]["name"], "Cloud");
        assert_eq!(device_counts(&devices)["all"], 3);
        assert_eq!(device_rows(&devices, "all", "").len(), 3);
        assert_eq!(device_rows(&devices, "phones", "").len(), 1);
        assert_eq!(device_rows(&devices, "all", "iphone").len(), 2);
        assert_eq!(find_phone(&devices, "p1")["host"]["id"], "1");
        assert_eq!(link_phase(None, 100, 200), "expired");
        assert_eq!(link_phase(Some("asking"), 100, 200), "asking");
        assert!(link_polls("waiting"));
        assert!(!link_polls("expired"));
    }

    #[test]
    fn guides_and_summary() {
        let host = json!({"phones":[{"id":"p1","platform":"android","phoneKind":"device","status":{"kind":"needs_you","reason":"unauthorized"}}]});
        assert_eq!(android_stage(&host, &[])["kind"], "allow");
        assert_eq!(android_stage(&json!({"phones":[]}), &[])["kind"], "looking");
        let steps = android_steps("looking", &["devOptions".to_owned()], None);
        assert_eq!(steps["now"], "usbDebugging");
        assert_eq!(steps["states"]["devOptions"], "done");
        assert_eq!(steps["byHand"][0], "devOptions");
        assert!(wifi_pair_live(Some("pairing")));
        assert!(!wifi_pair_live(Some("connected")));
        assert_eq!(count_label(1500), "1.5K");
        assert_eq!(summary(2, 1, 1, false).len(), 4);
    }
}
