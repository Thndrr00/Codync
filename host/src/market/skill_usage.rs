//! When the skills bots wrote were last used, and when a bot was last offered to save one. A skill a bot wrote and
//! nobody used for 30 days is dormant: `doctor` says so and the user can turn those skills off everywhere (the files
//! stay, so turning one back on brings it back). A skill counts as used when the user picks it with `/name`, or when
//! a bot's tool call touches a file inside its folder.
//!
//! Ideas from mercury-agent's `skill-synthesis.ts` and `self-made.ts` (MIT); no code copied.

use super::skills::MAX_WRITTEN_PER_BOT;
use super::{Skill, skills, skills_dir};
use crate::hub::Hub;
use crate::store::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

const KEY: &str = "skills.usage.v1";
const DAY_MS: i64 = 86_400_000;
/// Unused this long, a skill a bot wrote is dormant.
pub const DORMANT_AFTER_MS: i64 = 30 * DAY_MS;
/// A skill's use is noted at most this often: a bot reading it step by step is one use.
const TOUCH_EVERY_MS: i64 = 3_600_000;
/// A bot is offered the chance to save a skill at most this often.
pub const OFFER_EVERY_MS: i64 = DAY_MS;
/// Finished tool calls in one turn that make it worth offering a skill: most turns use a few.
pub const OFFER_AFTER_CALLS: u32 = 8;

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Usage {
    /// Skill id → when it was last used (or first seen, for one never used).
    #[serde(default)]
    used: HashMap<String, i64>,
    /// Bot id → when it was last offered to save a skill.
    #[serde(default)]
    offered: HashMap<String, i64>,
    /// Skill id → how many turns loaded its text, and what that text is worth.
    #[serde(default)]
    loads: HashMap<String, Load>,
}

/// How often a skill's text entered a turn. One turn loads a skill at most once, however many times
/// its agent reads the file, so the count is turns, not file reads.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Load {
    pub count: u64,
    /// Estimated tokens of the skill text at each load, added up (`SKILL.md` bytes / 4).
    pub tokens: u64,
    /// Estimated list cost, in millionths of a dollar, of the loads whose model has a known rate.
    #[serde(default)]
    pub cost_micros: u64,
    /// Loads whose model had a known rate. No known load leaves the skill unpriced, never free.
    #[serde(default)]
    pub priced: u64,
    /// The bots that loaded it.
    #[serde(default)]
    pub agents: BTreeSet<String>,
}

impl Usage {
    pub fn load(store: &Store) -> Self {
        store.kv_read(KEY).ok().flatten().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, store: &Store) -> Result<()> {
        store.kv_set(KEY, &serde_json::to_string(self)?)
    }

    /// Notes a use of `id`. False when one was noted within the hour already, so there's nothing to save.
    pub fn touch(&mut self, id: &str, now: i64) -> bool {
        if self.used.get(id).is_some_and(|&at| (0..TOUCH_EVERY_MS).contains(&(now - at))) {
            return false;
        }
        self.used.insert(id.to_owned(), now);
        true
    }

    /// Starts the idle count of the skills seen for the first time, from `now`. True when any started.
    pub fn seed(&mut self, all: &[Skill], now: i64) -> bool {
        let before = self.used.len();
        for skill in all {
            self.used.entry(skill.id.clone()).or_insert(now);
        }
        self.used.len() != before
    }

    /// A skill a bot wrote that nobody has used for `DORMANT_AFTER_MS`. Installed skills and the user's never are.
    pub fn is_dormant(&self, skill: &Skill, now: i64) -> bool {
        skill.written_by.is_some() && self.used.get(&skill.id).is_some_and(|&at| now - at >= DORMANT_AFTER_MS)
    }

    /// Whether `bot`, after a turn of `calls` finished tool calls, should be offered to save a skill: the turn was
    /// long, the bot has room for another, and it wasn't offered one today. Counts the offer when it says yes.
    pub fn offer(&mut self, bot: &str, calls: u32, has_room: bool, now: i64) -> bool {
        let due = self.offered.get(bot).is_none_or(|&at| now - at >= OFFER_EVERY_MS);
        if calls < OFFER_AFTER_CALLS || !has_room || !due {
            return false;
        }
        self.offered.insert(bot.to_owned(), now);
        true
    }
}

/// Notes that the skill `id` was used. Failing to save isn't worth interrupting a turn for.
pub fn note_use(store: &Store, id: &str, now: i64) {
    let mut usage = Usage::load(store);
    if usage.touch(id, now)
        && let Err(error) = usage.save(store)
    {
        tracing::warn!(error = format!("{error:#}"), "couldn't note a skill's use");
    }
}

/// Estimated tokens of a skill's `SKILL.md`: about four bytes of text make one token. A missing
/// file (a skill that was removed while a turn ran) has no estimate.
pub fn load_tokens(id: &str) -> Option<u64> {
    const MAX: u64 = 1_000_000;
    let size = std::fs::metadata(skills_dir().join(id).join("SKILL.md")).ok()?.len();
    Some(size.div_ceil(4).min(MAX))
}

/// The id of the skill a tool call's `input` read: the path names `<root>/<id>/SKILL.md`. Reading a
/// script or another file in the skill's folder is a use of the skill, but not a load of its text.
pub fn skill_read(input: &Value, root: &str) -> Option<String> {
    if input.is_null() || root.is_empty() {
        return None;
    }
    // JSON writes a Windows path's backslashes doubled; both forms become `/`, and case doesn't matter there.
    let text = input.to_string().replace("\\\\", "/").replace('\\', "/").to_lowercase();
    let root = root.trim_end_matches('/').to_lowercase();
    let after = &text[text.find(&root)? + root.len()..];
    let rest = after.strip_prefix('/')?;
    let (id, file) = rest.split_once('/')?;
    let id: String = id.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')).collect();
    let loads = file.starts_with("skill.md")
        && file["skill.md".len()..].chars().next().is_none_or(|c| matches!(c, '"' | '\\' | '?' | ' '));
    (loads && !id.is_empty() && id != "." && id != "..").then_some(id)
}

/// Notes one turn's load of `id` by `bot`: a count, the text's estimated tokens, and its list cost
/// when `cost` is known. The first load of the hour also counts as the skill's use.
pub fn note_load(store: &Store, bot: &str, id: &str, tokens: Option<u64>, cost: Option<f64>, now: i64) {
    let mut usage = Usage::load(store);
    usage.touch(id, now);
    let load = usage.loads.entry(id.to_owned()).or_default();
    load.count = load.count.saturating_add(1);
    load.tokens = load.tokens.saturating_add(tokens.unwrap_or(0));
    if let Some(cost) = cost.filter(|cost| cost.is_finite() && *cost >= 0.0) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let micros = (cost.min(1_000_000.0) * 1_000_000.0).round() as u64;
        load.cost_micros = load.cost_micros.saturating_add(micros);
        load.priced = load.priced.saturating_add(1);
    }
    load.agents.insert(bot.to_owned());
    if let Err(error) = usage.save(store) {
        tracing::warn!(error = format!("{error:#}"), "couldn't note a skill's load");
    }
}

/// `skillUsage`: every skill's loads, estimated tokens and cost, its share of all skill-load tokens,
/// and how many bots loaded it, the most tokens first.
pub fn report(store: &Store) -> Value {
    let usage = Usage::load(store);
    let names: HashMap<String, String> = skills(store).into_iter().map(|skill| (skill.id, skill.name)).collect();
    let total: u64 = usage.loads.values().map(|load| load.tokens).sum();
    let mut rows: Vec<Value> = usage
        .loads
        .iter()
        .map(|(id, load)| {
            #[allow(clippy::cast_precision_loss, reason = "token counts are far below f64's exact range")]
            let share = if total == 0 { 0.0 } else { load.tokens as f64 / total as f64 };
            json!({
                "id": id,
                "name": names.get(id).cloned().unwrap_or_else(|| id.clone()),
                "loads": load.count,
                "tokens": load.tokens,
                "costUsd": (load.priced > 0).then(|| load.cost_micros as f64 / 1_000_000.0),
                "share": share,
                "agents": load.agents.len(),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        let tokens = b["tokens"].as_u64().cmp(&a["tokens"].as_u64());
        let loads = b["loads"].as_u64().cmp(&a["loads"].as_u64());
        tokens.then(loads).then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    json!({"skills": rows, "totalTokens": total})
}

/// The skills folder as text, with `/` separators, for matching it in a tool call's input.
pub fn skills_root() -> String {
    skills_dir().to_string_lossy().replace('\\', "/")
}

/// How many skills `bot` saved and how many of them are dormant. Skills seen for the first time start counting now.
pub fn written_skills(store: &Store, bot: &str, now: i64) -> (usize, usize) {
    let all = skills(store);
    let mut usage = Usage::load(store);
    if usage.seed(&all, now) {
        let _ = usage.save(store);
    }
    let mine: Vec<&Skill> = all.iter().filter(|s| s.written_by.as_deref() == Some(bot)).collect();
    (mine.len(), mine.iter().filter(|s| usage.is_dormant(s, now)).count())
}

/// Offers `bot` to save a skill after a turn of `calls` tool calls, when that's due. True when it should be offered.
pub fn offer_skill(store: &Store, bot: &str, calls: u32, now: i64) -> bool {
    let (saved, _) = written_skills(store, bot, now);
    let mut usage = Usage::load(store);
    if !usage.offer(bot, calls, saved < MAX_WRITTEN_PER_BOT, now) {
        return false;
    }
    if let Err(error) = usage.save(store) {
        tracing::warn!(error = format!("{error:#}"), "couldn't note the skill offer");
    }
    true
}

/// Turns the dormant skills off for every bot, and says how many switches it flipped. The skills stay installed.
pub fn retire_dormant(hub: &Hub, now: i64) -> Result<usize> {
    let all = skills(&hub.store);
    let mut usage = Usage::load(&hub.store);
    if usage.seed(&all, now) {
        usage.save(&hub.store)?;
    }
    let dormant: HashSet<&str> = all.iter().filter(|s| usage.is_dormant(s, now)).map(|s| s.id.as_str()).collect();
    if dormant.is_empty() {
        return Ok(0);
    }
    let mut turned_off = 0;
    for row in hub.store.bots()?.into_iter().filter(|r| !r.deleted && !r.config.is_group()) {
        let keep: Vec<&String> = row.config.skills.iter().filter(|id| !dormant.contains(id.as_str())).collect();
        let removed = row.config.skills.len() - keep.len();
        if removed > 0 {
            hub.update_bot(&json!({"id": row.config.id, "skills": keep}))?;
            turned_off += removed;
        }
    }
    Ok(turned_off)
}

/// The id of the skill a tool call's `input` touched: it names a file inside `root` (the skills folder).
pub fn skill_touched(input: &Value, root: &str) -> Option<String> {
    if input.is_null() || root.is_empty() {
        return None;
    }
    // JSON writes a Windows path's backslashes doubled; both forms become `/`, and case doesn't matter there.
    let text = input.to_string().replace("\\\\", "/").replace('\\', "/").to_lowercase();
    let root = root.trim_end_matches('/').to_lowercase();
    let after = &text[text.find(&root)? + root.len()..];
    let id: String = after
        .strip_prefix('/')?
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .collect();
    (!id.is_empty() && id != "." && id != "..").then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: &str, by: Option<&str>) -> Skill {
        serde_json::from_value(json!({"id": id, "name": id, "source": "custom", "writtenBy": by})).expect("skill")
    }

    #[test]
    fn a_skill_a_bot_wrote_goes_dormant_after_thirty_days_unused() {
        let mut usage = Usage::default();
        let mine = skill("mine", Some("a"));
        let installed = skill("installed", None);
        assert!(usage.seed(&[mine.clone(), installed.clone()], 0));
        assert!(!usage.seed(std::slice::from_ref(&mine), 5), "a skill already seen keeps its time");
        assert!(!usage.is_dormant(&mine, DORMANT_AFTER_MS - 1));
        assert!(usage.is_dormant(&mine, DORMANT_AFTER_MS));
        assert!(!usage.is_dormant(&installed, DORMANT_AFTER_MS * 5), "only skills bots wrote lapse");
        assert!(usage.touch("mine", DORMANT_AFTER_MS));
        assert!(!usage.is_dormant(&mine, DORMANT_AFTER_MS * 2 - 1), "a use starts the count over");
    }

    #[test]
    fn an_unseen_skill_is_not_dormant() {
        assert!(!Usage::default().is_dormant(&skill("new", Some("a")), DORMANT_AFTER_MS * 10));
    }

    #[test]
    fn a_use_is_noted_once_an_hour() {
        let mut usage = Usage::default();
        assert!(usage.touch("s", 1_000));
        assert!(!usage.touch("s", 1_000 + TOUCH_EVERY_MS - 1));
        assert!(usage.touch("s", 1_000 + TOUCH_EVERY_MS));
    }

    #[test]
    fn a_bot_is_offered_a_skill_after_a_long_turn_once_a_day() {
        let mut usage = Usage::default();
        assert!(!usage.offer("a", OFFER_AFTER_CALLS - 1, true, 0), "a short turn");
        assert!(!usage.offer("a", OFFER_AFTER_CALLS, false, 0), "no room for another skill");
        assert!(usage.offer("a", OFFER_AFTER_CALLS, true, 0));
        assert!(!usage.offer("a", OFFER_AFTER_CALLS * 2, true, OFFER_EVERY_MS - 1), "already offered today");
        assert!(usage.offer("a", OFFER_AFTER_CALLS, true, OFFER_EVERY_MS));
        assert!(usage.offer("b", OFFER_AFTER_CALLS, true, 1), "each bot has its own day");
    }

    #[test]
    fn usage_survives_a_round_trip_through_the_store() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("memory store");
        let mut usage = Usage::default();
        usage.touch("s", 7);
        usage.offer("a", OFFER_AFTER_CALLS, true, 9);
        usage.save(&store).expect("save");
        assert_eq!(Usage::load(&store), usage);
        assert_eq!(Usage::load(&Store::open(std::path::Path::new(":memory:")).expect("store")), Usage::default());
    }

    #[test]
    fn a_tool_call_inside_a_skills_folder_names_the_skill() {
        let root = "C:/Users/me/.codync/skills";
        let unix = json!({"path": "C:/Users/me/.codync/skills/release-checklist/SKILL.md"});
        assert_eq!(skill_touched(&unix, root).as_deref(), Some("release-checklist"));
        let windows = json!({"file_path": "C:\\Users\\me\\.codync\\skills\\pdf\\scripts\\merge.py"});
        assert_eq!(skill_touched(&windows, root).as_deref(), Some("pdf"));
        let upper = json!({"command": "type c:\\users\\ME\\.codync\\SKILLS\\pdf\\SKILL.md"});
        assert_eq!(skill_touched(&upper, root).as_deref(), Some("pdf"));
    }

    #[test]
    fn only_the_skill_file_loads_its_text() {
        let root = "C:/Users/me/.codync/skills";
        let file = json!({"path": "C:/Users/me/.codync/skills/release-checklist/SKILL.md"});
        assert_eq!(skill_read(&file, root).as_deref(), Some("release-checklist"));
        let windows = json!({"file_path": "C:\\Users\\me\\.codync\\skills\\pdf\\SKILL.md"});
        assert_eq!(skill_read(&windows, root).as_deref(), Some("pdf"));
        let script = json!({"path": "C:/Users/me/.codync/skills/pdf/scripts/merge.py"});
        assert_eq!(skill_read(&script, root), None, "a script is a use, not a load of the text");
        let other = json!({"path": "C:/Users/me/.codync/skills/pdf/NOTES.md"});
        assert_eq!(skill_read(&other, root), None);
        assert_eq!(skill_read(&Value::Null, root), None);
        assert_eq!(skill_read(&file, ""), None);
    }

    #[test]
    fn loads_keep_their_count_tokens_cost_and_bots() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("memory store");
        note_load(&store, "ada", "pdf", Some(300), Some(0.0006), 1_000);
        note_load(&store, "ada", "pdf", Some(300), Some(0.0006), 2_000);
        note_load(&store, "grace", "pdf", Some(300), None, 3_000);
        note_load(&store, "ada", "deploy", Some(100), None, 4_000);
        let usage = Usage::load(&store);
        let pdf = &usage.loads["pdf"];
        assert_eq!((pdf.count, pdf.tokens, pdf.priced, pdf.cost_micros), (3, 900, 2, 1_200));
        assert_eq!(pdf.agents.len(), 2);
        assert_eq!(usage.loads["deploy"].count, 1);
    }

    #[test]
    fn the_report_shares_tokens_and_leaves_an_unpriced_skill_unpriced() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("memory store");
        note_load(&store, "ada", "pdf", Some(300), Some(0.0006), 1_000);
        note_load(&store, "grace", "pdf", Some(300), None, 2_000);
        note_load(&store, "ada", "deploy", Some(100), None, 3_000);
        let report = report(&store);
        let skills = report["skills"].as_array().expect("skills");
        assert_eq!(report["totalTokens"], 700);
        assert_eq!(skills[0]["id"], "pdf");
        assert_eq!(skills[0]["loads"], 2);
        assert_eq!(skills[0]["agents"], 2);
        assert!((skills[0]["share"].as_f64().expect("share") - 600.0 / 700.0).abs() < 1e-9);
        assert!((skills[0]["costUsd"].as_f64().expect("priced") - 0.0006).abs() < 1e-9);
        assert_eq!(skills[1]["costUsd"], Value::Null, "an unpriced skill is not free");
    }

    #[test]
    fn other_paths_and_empty_inputs_name_no_skill() {
        let root = "C:/Users/me/.codync/skills";
        assert_eq!(skill_touched(&json!({"path": "C:/Users/me/project/src/main.rs"}), root), None);
        assert_eq!(skill_touched(&json!({"path": "C:/Users/me/.codync/skills"}), root), None);
        assert_eq!(skill_touched(&json!({"path": "C:/Users/me/.codync/skills/../token"}), root), None);
        assert_eq!(skill_touched(&Value::Null, root), None);
        assert_eq!(skill_touched(&json!({"path": "x"}), ""), None);
    }
}
