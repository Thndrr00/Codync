//! `doctor`: finds the bots whose setup can't work before a turn fails on them: a folder that's gone, no agent
//! chosen, a fallback that names nothing, a group with a member that was deleted. It only reads; fixing is the user's.

use crate::hub::Hub;
use crate::store::{BotConfig, BotKind};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// The bot can't run as configured.
    Error,
    /// It runs, but probably not as meant.
    Warning,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    NoFolder,
    MissingFolder,
    NoAgent,
    UnknownAgent,
    EmptyFallback,
    MissingMember,
    SmallGroup,
    DuplicateName,
    /// The bot saved as many skills as it may keep.
    SkillLimit,
    /// Skills the bot saved that nobody used for 30 days.
    DormantSkills,
}

impl Problem {
    fn severity(self) -> Severity {
        match self {
            Self::NoFolder | Self::MissingFolder | Self::NoAgent | Self::EmptyFallback | Self::MissingMember => {
                Severity::Error
            }
            Self::UnknownAgent | Self::SmallGroup | Self::DuplicateName | Self::SkillLimit | Self::DormantSkills => {
                Severity::Warning
            }
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::NoFolder => "has no project folder",
            Self::MissingFolder => "its project folder doesn't exist",
            Self::NoAgent => "has no agent chosen",
            Self::UnknownAgent => "uses an agent this host doesn't know",
            Self::EmptyFallback => "has a fallback with no agent",
            Self::MissingMember => "has a member that no longer exists",
            Self::SmallGroup => "has fewer than two members",
            Self::DuplicateName => "shares its name with another bot",
            Self::SkillLimit => "has saved as many skills as a bot may keep",
            Self::DormantSkills => "has skills it saved that nobody used for 30 days",
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub bot_id: String,
    pub bot_name: String,
    pub severity: Severity,
    pub problem: Problem,
    pub message: String,
}

/// What the checks ask of the machine, so they can be tested without one.
pub struct Probe<'a> {
    pub folder_exists: &'a dyn Fn(&str) -> bool,
    pub agent_known: &'a dyn Fn(&str) -> bool,
    /// How many skills a bot saved, and how many of those are dormant.
    pub written_skills: &'a dyn Fn(&str) -> (usize, usize),
}

pub fn inspect(bots: &[BotConfig], probe: &Probe) -> Vec<Finding> {
    let by_id: HashMap<&str, &BotConfig> = bots.iter().map(|b| (b.id.as_str(), b)).collect();
    let mut names: HashMap<String, usize> = HashMap::new();
    for bot in bots {
        *names.entry(bot.name.trim().to_lowercase()).or_default() += 1;
    }
    let mut findings = Vec::new();
    for bot in bots {
        let mut add = |problem: Problem| {
            findings.push(Finding {
                bot_id: bot.id.clone(),
                bot_name: bot.name.clone(),
                severity: problem.severity(),
                problem,
                message: format!("{} {}", bot.name, problem.text()),
            });
        };
        if names[&bot.name.trim().to_lowercase()] > 1 {
            add(Problem::DuplicateName);
        }
        match bot.kind {
            BotKind::Group => {
                let live = bot.members.iter().filter(|m| by_id.get(m.as_str()).is_some_and(|b| !b.is_group())).count();
                if live < bot.members.len() {
                    add(Problem::MissingMember);
                } else if live < 2 {
                    add(Problem::SmallGroup);
                }
            }
            BotKind::Agent => {
                if bot.cwd.trim().is_empty() {
                    add(Problem::NoFolder);
                } else if !(probe.folder_exists)(&bot.cwd) {
                    add(Problem::MissingFolder);
                }
                if bot.backend.is_empty() && bot.command.is_none() {
                    add(Problem::NoAgent);
                } else if bot.command.is_none() && !(probe.agent_known)(&bot.backend) {
                    add(Problem::UnknownAgent);
                }
                if bot.fallbacks.iter().any(|f| f.backend.is_empty() && f.command.is_none()) {
                    add(Problem::EmptyFallback);
                }
                let (saved, dormant) = (probe.written_skills)(&bot.id);
                if saved >= crate::market::MAX_WRITTEN_PER_BOT {
                    add(Problem::SkillLimit);
                }
                if dormant > 0 {
                    add(Problem::DormantSkills);
                }
            }
        }
    }
    findings
}

/// The `doctor` API answer: every finding, and how many are errors.
pub fn report(hub: &Hub) -> anyhow::Result<Value> {
    let bots: Vec<BotConfig> = hub.store.bots()?.into_iter().filter(|r| !r.deleted).map(|r| r.config).collect();
    let probe = Probe {
        folder_exists: &|path| Path::new(path).is_dir(),
        agent_known: &crate::agent::backends::is_known,
        written_skills: &|bot| crate::market::written_skills(&hub.store, bot, crate::store::now_ms()),
    };
    let findings = inspect(&bots, &probe);
    let errors = findings.iter().filter(|f| f.severity == Severity::Error).count();
    Ok(json!({"checked": bots.len(), "errors": errors, "findings": findings}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bot(value: Value) -> BotConfig {
        let mut base = json!({"id": "x", "name": "X", "backend": "claude", "cwd": "/work", "permission": "ask"});
        let Value::Object(extra) = value else { panic!("bot overrides must be an object") };
        base.as_object_mut().unwrap().extend(extra);
        serde_json::from_value(base).unwrap()
    }

    fn problems(bots: &[BotConfig], folders: bool) -> Vec<Problem> {
        problems_with_skills(bots, folders, (0, 0))
    }

    fn problems_with_skills(bots: &[BotConfig], folders: bool, skills: (usize, usize)) -> Vec<Problem> {
        let probe = Probe {
            folder_exists: &move |_| folders,
            agent_known: &|id| id == "claude",
            written_skills: &move |_| skills,
        };
        inspect(bots, &probe).into_iter().map(|f| f.problem).collect()
    }

    #[test]
    fn skills_at_the_limit_or_unused_are_warnings() {
        let one = [bot(json!({}))];
        let limit = crate::market::MAX_WRITTEN_PER_BOT;
        assert_eq!(problems_with_skills(&one, true, (limit - 1, 0)), Vec::<Problem>::new());
        assert_eq!(problems_with_skills(&one, true, (limit, 0)), [Problem::SkillLimit]);
        assert_eq!(problems_with_skills(&one, true, (3, 2)), [Problem::DormantSkills]);
        assert_eq!(problems_with_skills(&one, true, (limit, 5)), [Problem::SkillLimit, Problem::DormantSkills]);
        assert_eq!(Problem::SkillLimit.severity(), Severity::Warning);
        assert_eq!(Problem::DormantSkills.severity(), Severity::Warning);
    }

    #[test]
    fn a_healthy_bot_has_no_findings() {
        assert_eq!(problems(&[bot(json!({}))], true), Vec::<Problem>::new());
    }

    #[test]
    fn a_missing_folder_or_agent_is_reported() {
        assert_eq!(problems(&[bot(json!({}))], false), [Problem::MissingFolder]);
        assert_eq!(problems(&[bot(json!({"cwd": ""}))], true), [Problem::NoFolder]);
        assert_eq!(problems(&[bot(json!({"backend": ""}))], true), [Problem::NoAgent]);
        assert_eq!(problems(&[bot(json!({"backend": "zzz"}))], true), [Problem::UnknownAgent]);
        assert_eq!(Problem::UnknownAgent.severity(), Severity::Warning);
    }

    #[test]
    fn a_command_stands_in_for_a_known_agent() {
        assert_eq!(problems(&[bot(json!({"backend": "mine", "command": "run-it"}))], true), Vec::<Problem>::new());
    }

    #[test]
    fn groups_need_live_members() {
        let a = bot(json!({"id": "a", "name": "A"}));
        let b = bot(json!({"id": "b", "name": "B"}));
        let group = |members: Value| {
            bot(json!({"id": "g", "name": "G", "kind": "group", "backend": "", "cwd": "", "members": members}))
        };
        assert_eq!(problems(&[a.clone(), b.clone(), group(json!(["a", "b"]))], true), Vec::<Problem>::new());
        assert_eq!(problems(&[a.clone(), b.clone(), group(json!(["a", "gone"]))], true), [Problem::MissingMember]);
        assert_eq!(problems(&[a, b, group(json!(["a"]))], true), [Problem::SmallGroup]);
    }

    #[test]
    fn duplicate_names_warn_for_each_bot() {
        let found =
            problems(&[bot(json!({"id": "a", "name": "Same"})), bot(json!({"id": "b", "name": " same "}))], true);
        assert_eq!(found, [Problem::DuplicateName, Problem::DuplicateName]);
    }

    #[test]
    fn an_empty_fallback_is_an_error() {
        assert_eq!(problems(&[bot(json!({"fallbacks": [{"backend": ""}]}))], true), [Problem::EmptyFallback]);
    }
}
