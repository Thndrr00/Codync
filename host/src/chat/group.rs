//! Group chats (Grok Bot's design): several bots and the user in one conversation.
//!
//! A group is a roster row (`BotKind::Group`) with a member list and its own
//! transcript, but no agent. When the user writes, the group runs a *room turn*:
//! up to [`MAX_ROUNDS`] rounds in which the members answer one at a time, each in
//! its **own** session (a member's history is unified across its chats), told
//! what was said since it last spoke. In the first round everyone answers unless
//! the user @-mentions someone; later rounds only ask the members another member
//! @-mentioned and who haven't spoken or been asked since. A member with nothing
//! to add says `(pass)`, and a round where nobody speaks ends the turn. A newer
//! message in the same lane (or Stop) moves the lane's epoch, which ends a running
//! room turn before its next speaker.

use crate::LockExt;
use crate::agent::bot::{Cmd, NoticeStyle};
use crate::hub::{BotStatus, Hub};
use crate::remote::push::{self, AlertKind};
use crate::store::{BotConfig, BotKind, Entry, EntryKind, Lane, Store, TeamSettings};
use anyhow::{Result, anyhow, bail};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

const MAX_ROUNDS: usize = 3;
/// A member's turn that runs longer counts as a pass.
const MEMBER_TIMEOUT: Duration = Duration::from_mins(10);
/// Replies per room turn, across all rounds.
const MAX_REPLIES: usize = 10;
/// Room lines a member is shown per turn.
const HISTORY_LINES: i64 = 24;
/// A bot's line in another member's prompt is clipped (the user's never is).
const LINE_CHARS: usize = 2000;
const PASS: &str = "(pass)";

/// One member's turn in a room, queued on its actor like any other turn.
pub struct GroupTurn {
    pub lane: Lane,
    pub prompt: String,
    /// The newest message `prompt` shows (its `seq`): a reaction goes on what the member saw.
    pub shown: i64,
    /// The room turn that asked; a member skips it once a newer one started (see [`Rooms::current`]), unless `late`.
    pub epoch: u64,
    /// Queued while the member was busy elsewhere: it answers once free whatever the room did since, on a
    /// prompt built when it starts ([`refresh`]). Only the group's Stop, the room asking the member again, or the
    /// member leaving the group drops it (Stop in the member's own chat doesn't).
    pub late: bool,
    /// The reply the room sees, or `None` for a pass.
    pub reply: oneshot::Sender<Result<Option<String>>>,
    /// The user's message the room turn answers: one a voice call carried makes the reply the call's (`chat::calls`).
    pub answers: Vec<String>,
}

/// Room turn epochs per group.
#[derive(Default)]
pub struct Rooms(Mutex<HashMap<String, u64>>);

impl Rooms {
    fn next(&self, lane: &Lane) -> u64 {
        let mut map = self.0.locked();
        let epoch = map.entry(lane.chat.clone()).or_default();
        *epoch += 1;
        *epoch
    }

    pub fn current(&self, lane: &Lane, epoch: u64) -> bool {
        self.0.locked().get(&lane.chat).copied() == Some(epoch)
    }

    /// Stop: ends the room turn in `group` and the member turns in it.
    pub fn stop(&self, hub: &Hub, group: &str) {
        match self.0.locked().get_mut(group) {
            Some(epoch) => *epoch += 1,
            None => return,
        }
        let Ok(Some(row)) = hub.store.bot(group) else { return };
        for member in &row.config.members {
            let _ = hub.send_cmd(member, Cmd::CancelGroup { chat: group.to_owned() });
        }
    }
}

/// Existing, distinct agent bots (not groups, not the group itself).
pub fn valid_members(store: &Store, group_id: &str, ids: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = vec![];
    for id in ids.iter().map(|i| i.trim()) {
        if id.is_empty() || id == group_id || out.iter().any(|o| o == id) {
            continue;
        }
        match store.bot(id)? {
            Some(row) if !row.deleted && row.config.kind == BotKind::Agent => out.push(id.to_owned()),
            Some(row) if row.config.is_group() => bail!("a group can't be a member of another group"),
            _ => {}
        }
    }
    if out.is_empty() {
        bail!("a group needs at least one bot");
    }
    Ok(out)
}

/// A group whose members are exactly `members`.
pub fn with_members(store: &Store, members: &[String]) -> Result<Option<String>> {
    let mut wanted = members.to_vec();
    wanted.sort();
    Ok(store.bots()?.into_iter().find_map(|row| {
        let mut have = row.config.members.clone();
        have.sort();
        (!row.deleted && row.config.is_group() && have == wanted).then_some(row.config.id)
    }))
}

/// The user wrote in a group lane: start a room turn there (ending one still running).
pub fn start(hub: &Arc<Hub>, group: &BotConfig, lane: Lane) {
    let epoch = hub.groups.next(&lane);
    let mode = team_mode(group);
    let (hub, group_id) = (hub.clone(), group.id.clone());
    tokio::spawn(async move {
        let result = match mode {
            TeamMode::OneVoice => run_team(&hub, &group_id, &lane, epoch).await,
            TeamMode::Smart => run_smart(&hub, &group_id, &lane, epoch).await,
            TeamMode::Roundtable => run(&hub, &group_id, &lane, epoch).await,
        };
        if let Err(error) = result {
            tracing::warn!(group = group_id, error = format!("{error:#}"), "room turn failed");
        }
    });
}

/// How a group answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamMode {
    /// Classic: members answer in rounds, @mentions choose who.
    Roundtable,
    /// The lead answers once for the whole team.
    OneVoice,
    /// Who answers depends on the request: `/boss`, a role, or everyone.
    Smart,
}

/// The group's answering mode. A group without a team setup keeps the round table.
pub fn team_mode(group: &BotConfig) -> TeamMode {
    match group.team.as_ref().map(|t| t.mode.as_str()) {
        Some("team") => TeamMode::OneVoice,
        Some("smart") => TeamMode::Smart,
        _ => TeamMode::Roundtable,
    }
}

/// Who may speak besides the lead: `lead` (nobody), `lead_and_hands` (members the lead hands the
/// floor to) or `all`. Defaults to `lead`.
fn team_voice(group: &BotConfig) -> &str {
    match group.team.as_ref().map(|t| t.voice.as_str()) {
        Some("lead_and_hands") => "lead_and_hands",
        Some("all") => "all",
        _ => "lead",
    }
}

/// Checks a group's team setup against its members: ids belong to the group, and the reporting
/// lines never loop. The lead is the group's chief, or its first member.
pub fn validate_team(group: &BotConfig, team: &TeamSettings) -> Result<()> {
    if !matches!(team.mode.as_str(), "" | "team" | "roundtable" | "smart") {
        bail!("team mode must be team, roundtable or smart");
    }
    if !matches!(team.voice.as_str(), "" | "lead" | "lead_and_hands" | "all") {
        bail!("team voice must be lead, lead_and_hands or all");
    }
    for member in &team.members {
        if !group.members.contains(&member.bot_id) {
            bail!("the team structure names a bot that isn't in the group");
        }
        if let Some(reports_to) = &member.reports_to {
            if reports_to == &member.bot_id {
                bail!("a member can't report to itself");
            }
            if !group.members.contains(reports_to) {
                bail!("a member reports to a bot that isn't in the group");
            }
        }
        if member.role.len() > 60 || member.responsibility.len() > 240 {
            bail!("a team role or responsibility is too long");
        }
    }
    // Follow every reporting line: a loop would make the lead unreachable.
    for start in &team.members {
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(start.bot_id.clone());
        while let Some(id) = current {
            if !seen.insert(id.clone()) {
                bail!("the reporting lines form a loop");
            }
            current = team
                .members
                .iter()
                .find(|m| m.bot_id == id)
                .and_then(|m| m.reports_to.clone());
        }
    }
    Ok(())
}

/// Who should answer a smart-mode request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// `/boss`: only the team's lead.
    Boss,
    /// A role or a name: that member alone.
    Member(String),
    /// Nobody in particular: everyone answers in the room.
    All,
    /// `/standup`: everyone reports finished, next and blocked.
    Standup,
}

/// `/boss`, `/jefe`, `/lead` or `/supervisor` in the text: the lead answers.
fn calls_boss(text: &str) -> bool {
    let text = text.to_lowercase();
    ["/boss", "/jefe", "/lead", "/supervisor", "/lider", "/líder"].iter().any(|command| text.contains(command))
}

/// `/standup` or `/daily`: everyone answers with their own report.
fn calls_standup(text: &str) -> bool {
    let text = text.to_lowercase();
    ["/standup", "/daily", "/parte"].iter().any(|command| text.contains(command))
}

/// Folds a phrase for matching: lower case, no accents, punctuation as spaces.
fn fold(text: &str) -> String {
    let plain: String = text
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The member a message asks for by role or by name, when exactly one matches.
fn named_target(norm: &str, group: &BotConfig, members: &[BotConfig]) -> Option<String> {
    let mut best: Option<(usize, Vec<String>)> = None;
    let mut consider = |phrase: &str, id: &str| {
        let phrase = fold(phrase);
        if phrase.len() < 3 || !norm.contains(&phrase) {
            return;
        }
        match &mut best {
            Some((len, _)) if *len > phrase.len() => {}
            Some((len, ids)) if *len == phrase.len() => {
                if !ids.iter().any(|other| other == id) {
                    ids.push(id.to_owned());
                }
            }
            slot => *slot = Some((phrase.len(), vec![id.to_owned()])),
        }
    };
    for member in members {
        let role = group
            .team
            .as_ref()
            .and_then(|t| t.members.iter().find(|m| m.bot_id == member.id))
            .map(|m| m.role.clone())
            .unwrap_or_default();
        if !role.trim().is_empty() {
            consider(&role, &member.id);
        }
        if let Some(first) = member.name.split_whitespace().next() {
            consider(first, &member.id);
        }
    }
    match best {
        Some((_, ids)) if ids.len() == 1 => ids.into_iter().next(),
        _ => None,
    }
}

/// Decides who answers a smart-mode request from its text alone.
pub fn pick_target(text: &str, group: &BotConfig, members: &[BotConfig]) -> Target {
    let norm = fold(text);
    if calls_boss(text) {
        return Target::Boss;
    }
    if calls_standup(text) {
        return Target::Standup;
    }
    match named_target(&norm, group, members) {
        Some(id) => Target::Member(id),
        None => Target::All,
    }
}

/// The group's lead: its chief when that is a member, else its first member.
fn lead_of(group: &BotConfig, members: &[BotConfig]) -> Option<BotConfig> {
    group
        .chief_bot_id
        .clone()
        .filter(|id| members.iter().any(|m| &m.id == id))
        .or_else(|| members.first().map(|m| m.id.clone()))
        .and_then(|id| members.iter().find(|m| m.id == id).cloned())
}

/// The request answers itself: the lead for `/boss`, one member for a role or name, everyone else.
async fn run_smart(hub: &Arc<Hub>, group_id: &str, lane: &Lane, epoch: u64) -> Result<()> {
    let group = group(hub, group_id)?;
    let members = members(hub, &group);
    if members.is_empty() {
        bail!("the group has no members");
    }
    let since = since_last_user_message(hub, lane)?;
    let text = since.first().and_then(|e| e.data["text"].as_str()).unwrap_or_default().to_owned();
    // An @mention is already a direct address.
    let mentioned = responders(&members, &since);
    let target = if members.len() > 1 && mentioned.len() == 1 {
        Target::Member(mentioned[0].id.clone())
    } else {
        pick_target(&text, &group, &members)
    };
    match target {
        Target::All => run(hub, group_id, lane, epoch).await,
        Target::Standup => {
            for member in &members {
                if !hub.groups.current(lane, epoch) {
                    break;
                }
                let (prompt, shown) = ask(hub, &group, &members, member, lane)?;
                let prompt = format!(
                    "{prompt}\n\n[Standup] Report in at most three short lines: what you finished, what is next, and what is blocked. No greeting, no repeat of the agenda."
                );
                let _ = ask_once(hub, lane, epoch, member, prompt, shown).await;
            }
            Ok(())
        }
        Target::Boss => {
            let Some(lead) = lead_of(&group, &members) else { return Ok(()) };
            let (prompt, shown) = team_prompt(hub, &group, &members, &lead, lane)?;
            if let Some(text) = ask_once(hub, lane, epoch, &lead, prompt, shown).await? {
                alert(hub, group_id, &lead.id, &lead.name, &text)?;
            }
            Ok(())
        }
        Target::Member(id) => {
            let Some(member) = members.iter().find(|m| m.id == id).cloned() else {
                return Ok(());
            };
            let is_lead = lead_of(&group, &members).is_some_and(|lead| lead.id == member.id);
            let (prompt, shown) = if is_lead {
                team_prompt(hub, &group, &members, &member, lane)?
            } else {
                ask(hub, &group, &members, &member, lane)?
            };
            if let Some(text) = ask_once(hub, lane, epoch, &member, prompt, shown).await? {
                alert(hub, group_id, &member.id, &member.name, &text)?;
            }
            Ok(())
        }
    }
}

/// The lead answers for the whole team, in one message, after consulting whoever it needs.
async fn run_team(hub: &Arc<Hub>, group_id: &str, lane: &Lane, epoch: u64) -> Result<()> {
    let group = group(hub, group_id)?;
    let members = members(hub, &group);
    if members.is_empty() {
        bail!("the group has no members");
    }
    let lead = lead_of(&group, &members).ok_or_else(|| anyhow!("the group has no lead"))?;
    let since_user = since_last_user_message(hub, lane)?;
    // A person who names one member talks to that member, not to the team's voice.
    let responders = responders(&members, &since_user);
    if responders.len() == 1 && responders[0].id != lead.id {
        let target = (*responders[0]).clone();
        let (prompt, shown) = ask(hub, &group, &members, &target, lane)?;
        let _ = ask_once(hub, lane, epoch, &target, prompt, shown).await;
        return Ok(());
    }
    let (prompt, shown) = team_prompt(hub, &group, &members, &lead, lane)?;
    let said = ask_once(hub, lane, epoch, &lead, prompt, shown).await?;
    // The lead may hand the floor to a member by naming it: it then answers for itself.
    if team_voice(&group) != "lead" {
        for member in hand_raisers(hub, &members, &lead, lane)? {
            if !hub.groups.current(lane, epoch) || member.id == lead.id {
                continue;
            }
            let (prompt, shown) = ask(hub, &group, &members, &member, lane)?;
            let _ = ask_once(hub, lane, epoch, &member, prompt, shown).await;
        }
    }
    if let Some(text) = said {
        alert(hub, group_id, &lead.id, &lead.name, &text)?;
    }
    Ok(())
}

/// One member's turn in the room, waited on with the room's timeout; `None` when it passed, was
/// busy elsewhere or did not finish in time.
async fn ask_once(
    hub: &Arc<Hub>,
    lane: &Lane,
    epoch: u64,
    member: &BotConfig,
    prompt: String,
    shown: i64,
) -> Result<Option<String>> {
    if busy_elsewhere(hub, &member.id, lane) {
        return Ok(None);
    }
    let since_user = since_last_user_message(hub, lane)?;
    let (reply, answer) = oneshot::channel();
    let turn =
        GroupTurn { lane: lane.clone(), prompt, shown, epoch, late: false, reply, answers: answers(&since_user) };
    if hub.send_cmd(&member.id, Cmd::Group(turn)).is_err() {
        return Ok(None);
    }
    Ok(match tokio::time::timeout(MEMBER_TIMEOUT, answer).await {
        Ok(Ok(Ok(Some(text)))) if !is_pass(&text) => Some(text),
        _ => None,
    })
}

/// Members the lead named in its answer, in mention order, without the lead.
fn hand_raisers(hub: &Hub, members: &[BotConfig], lead: &BotConfig, lane: &Lane) -> Result<Vec<BotConfig>> {
    let since = since_last_user_message(hub, lane)?;
    let mut out: Vec<BotConfig> = vec![];
    for entry in since.iter().rev() {
        if entry.kind != EntryKind::Agent.as_str() || entry.data["author"].as_str() != Some(lead.id.as_str()) {
            continue;
        }
        let text = entry.data["text"].as_str().unwrap_or_default();
        for member in members {
            if member.id != lead.id && names(text, member) && !out.iter().any(|m| m.id == member.id) {
                out.push(member.clone());
            }
        }
        break;
    }
    out.truncate(2);
    Ok(out)
}

/// The lead's prompt: the hierarchy, the team's rules, and the room since the user last wrote.
fn team_prompt(
    hub: &Hub,
    group: &BotConfig,
    members: &[BotConfig],
    lead: &BotConfig,
    lane: &Lane,
) -> Result<(String, i64)> {
    let shown = hub.store.messages_after(lane, 0, 1)?.last().map_or(0, |e| e.seq);
    let about = group.description.trim();
    let room = if about.is_empty() { format!("\"{}\"", group.name) } else { format!("\"{}\" — {about}", group.name) };
    let mut out = vec![format!(
        "You are {}, the lead of the bot team in the group chat ({room}). You answer for the whole team: the \
         user sees one message from the team, written by you.",
        lead.name
    )];
    out.push("The team:".into());
    for member in members {
        let role = group
            .team
            .as_ref()
            .and_then(|t| t.members.iter().find(|m| m.bot_id == member.id))
            .cloned()
            .unwrap_or_default();
        let title = if role.role.trim().is_empty() { String::new() } else { format!(" — {}", role.role.trim()) };
        let responsibility =
            if role.responsibility.trim().is_empty() { String::new() } else { format!(" ({})", role.responsibility.trim()) };
        let reports = role
            .reports_to
            .as_ref()
            .and_then(|id| members.iter().find(|m| &m.id == id))
            .map(|m| format!(", reports to {}", m.name))
            .unwrap_or_default();
        let me = if member.id == lead.id { " (you)" } else { "" };
        out.push(format!("- {}{me}{title}{responsibility}{reports}", member.name));
    }
    out.push(
        "Answer as one team in a single short message: decide, then state the result in the team's voice. Use \
         ask_bot to consult a member when its specialty matters; its reply comes back in this turn, and you own \
         the final answer. Never claim a member did work it did not do, and never reveal private one-on-one context."
            .into(),
    );
    out.push(
        "If a member should speak for itself, name it with @Name at the end of your answer, on its own line; \
         only then will it add its own message."
            .into(),
    );
    out.push(String::new());
    let seen = hub.store.last_spoke(lane, &lead.id);
    let header = format!("[Group chat: \"{}\"]", group.name);
    let mut lines: Vec<String> = vec![];
    for e in hub.store.messages_after(lane, seen, HISTORY_LINES)? {
        lines.push(line(hub, group, &e, lead));
    }
    out.push(header);
    if lines.is_empty() {
        out.push("No new messages in the room since your last turn.".into());
    } else {
        out.push("New messages in the room (oldest first):".into());
        out.extend(lines);
    }
    out.push(String::new());
    out.push("It's your turn as the team's lead. Write the team's one answer.".into());
    Ok((out.join("\n"), shown))
}

async fn run(hub: &Arc<Hub>, group_id: &str, lane: &Lane, epoch: u64) -> Result<()> {
    let mut replies = 0;
    let mut last_reply: Option<(String, String, String)> = None;
    // Members busy elsewhere: their turn waits in their queue, and they answer on their own once free.
    let mut later: Vec<String> = vec![];
    // The newest message each member was shown when last asked: what it passed on, it has seen.
    let mut asked: HashMap<String, i64> = HashMap::new();
    'rounds: for round in 0..MAX_ROUNDS {
        let group = group(hub, group_id)?;
        let members = members(hub, &group);
        let since_user = since_last_user_message(hub, lane)?;
        let mut speakers =
            if round == 0 { responders(&members, &since_user) } else { cited(&members, &since_user, &asked) };
        if round == 0 && speakers.len() == members.len() && members.len() > 1 {
            let candidates: Vec<crate::chat::decider::BotCandidate> = members
                .iter()
                .map(|m| crate::chat::decider::BotCandidate {
                    id: m.id.clone(),
                    name: m.name.clone(),
                    description: m.description.clone(),
                })
                .collect();
            let user_msg = since_user.first().and_then(|e| e.data["text"].as_str()).unwrap_or_default().to_owned();
            let req = crate::chat::decider::DeciderRequest {
                group_name: group.name.clone(),
                recent_transcript: vec![],
                user_message: user_msg,
                candidates,
            };
            match crate::chat::decider::resolve_speaker(&req).await {
                crate::chat::decider::RoutingDecision::SingleBot(id) => {
                    if let Some(target) = members.iter().find(|m| m.id == id) {
                        speakers = vec![target];
                    }
                }
                crate::chat::decider::RoutingDecision::FallbackToLead => {
                    let lead_id = group.chief_bot_id.as_deref().or_else(|| members.first().map(|m| m.id.as_str()));
                    if let Some(id) = lead_id
                        && let Some(target) = members.iter().find(|m| m.id == id)
                    {
                        speakers = vec![target];
                    }
                }
                crate::chat::decider::RoutingDecision::AllMembers => {}
            }
        }
        // Each round starts one member later, so the same bot doesn't always speak first.
        if !speakers.is_empty() {
            let offset = round % speakers.len();
            speakers.rotate_left(offset);
        }
        let mut spoke = 0;
        for member in &speakers {
            if !hub.groups.current(lane, epoch) {
                return Ok(());
            }
            // A full room turn still reports its last reply.
            if replies >= MAX_REPLIES {
                break 'rounds;
            }
            if later.contains(&member.id) {
                continue;
            }
            let (prompt, shown) = ask(hub, &group, &members, member, lane)?;
            asked.insert(member.id.clone(), shown);
            let (reply, answer) = oneshot::channel();
            let busy = busy_elsewhere(hub, &member.id, lane);
            let turn = GroupTurn {
                lane: lane.clone(),
                prompt,
                shown,
                epoch,
                late: busy,
                reply,
                answers: answers(&since_user),
            };
            if hub.send_cmd(&member.id, Cmd::Group(turn)).is_err() {
                continue;
            }
            // The room doesn't wait on a turn elsewhere (it can run for hours): it says so, and the member
            // answers here when it's done.
            if busy {
                let text = format!("{} is busy in another chat and will answer here when it's done.", member.name);
                let data = serde_json::json!({"text": text, "style": NoticeStyle::Info});
                hub.add_entry(lane, EntryKind::Notice, hub.store.max_turn(group_id), &data);
                later.push(member.id.clone());
                tokio::spawn(answer_later(hub.clone(), group_id.to_owned(), (*member).clone(), answer));
                continue;
            }
            // A failed, cancelled or overdue member turn counts as a pass; its error shows in the room.
            if let Ok(Ok(Ok(Some(text)))) = tokio::time::timeout(MEMBER_TIMEOUT, answer).await {
                spoke += 1;
                replies += 1;
                last_reply = Some((member.id.clone(), member.name.clone(), text));
            }
        }
        if spoke == 0 {
            break;
        }
    }
    if let Some((from, name, text)) = last_reply {
        alert(hub, group_id, &from, &name, &text)?;
    }
    Ok(())
}

/// A busy member's reply, after the room turn moved on: it alerts like the room's last reply. Dropping the member's
/// queued turn (see [`GroupTurn::late`]) ends this wait.
async fn answer_later(
    hub: Arc<Hub>,
    group_id: String,
    member: BotConfig,
    answer: oneshot::Receiver<Result<Option<String>>>,
) {
    if let Ok(Ok(Some(text))) = answer.await
        && let Err(error) = alert(&hub, &group_id, &member.id, &member.name, &text)
    {
        tracing::warn!(group = group_id, error = format!("{error:#}"), "late room reply not pushed");
    }
}

/// The "done" push for `name`'s reply in the group.
fn alert(hub: &Hub, group_id: &str, from: &str, name: &str, text: &str) -> Result<()> {
    if let Some(group) = hub.store.bot(group_id)?.map(|r| r.config) {
        push::notify(hub, &group, Some(from), &format!("{name} in {}", group.name), text, AlertKind::Done);
    }
    Ok(())
}

/// `member` runs a turn outside this room, so a room turn would queue behind it.
fn busy_elsewhere(hub: &Hub, member: &str, lane: &Lane) -> bool {
    let r = hub.runtime(member);
    matches!(r.status, BotStatus::Working | BotStatus::NeedsInput) && r.lane.as_ref() != Some(lane)
}

/// A late turn starts: it shows the room as it is now, not as it was when the turn was queued.
pub fn refresh(hub: &Hub, turn: &mut GroupTurn, me: &BotConfig) -> Result<()> {
    let group = group(hub, &turn.lane.chat)?;
    if !group.members.contains(&me.id) {
        bail!("no longer in the group");
    }
    (turn.prompt, turn.shown) = ask(hub, &group, &members(hub, &group), me, &turn.lane)?;
    turn.answers = answers(&since_last_user_message(hub, &turn.lane)?);
    Ok(())
}

fn group(hub: &Hub, id: &str) -> Result<BotConfig> {
    Ok(hub.store.bot(id)?.filter(|r| !r.deleted).ok_or_else(|| anyhow!("group deleted"))?.config)
}

/// `me`'s prompt for a turn in the room, and the newest message it shows (its `seq`).
fn ask(hub: &Hub, group: &BotConfig, members: &[BotConfig], me: &BotConfig, lane: &Lane) -> Result<(String, i64)> {
    let shown = hub.store.messages_after(lane, 0, 1)?.last().map_or(0, |e| e.seq);
    Ok((member_prompt(hub, group, members, me, lane)?, shown))
}

fn members(hub: &Hub, group: &BotConfig) -> Vec<BotConfig> {
    group
        .members
        .iter()
        .filter_map(|id| hub.store.bot(id).ok().flatten())
        .filter(|r| !r.deleted)
        .map(|r| r.config)
        .collect()
}

/// The lane's messages since the user last wrote there, that message included.
fn since_last_user_message(hub: &Hub, lane: &Lane) -> Result<Vec<Entry>> {
    let recent = hub.store.messages_after(lane, 0, HISTORY_LINES)?;
    let start = recent.iter().rposition(|e| e.kind == EntryKind::User.as_str()).unwrap_or(0);
    Ok(recent[start..].to_vec())
}

/// The user's message a room turn answers: the first of `since` (see [`since_last_user_message`]).
fn answers(since: &[Entry]) -> Vec<String> {
    since.first().filter(|e| e.kind == EntryKind::User.as_str()).map(|e| e.id.clone()).into_iter().collect()
}

/// Who answers the user: the members @-mentioned or replied to since the user's last message, or
/// everyone when nobody (or `@all` / `@everyone`) is.
fn responders<'a>(members: &'a [BotConfig], messages: &[Entry]) -> Vec<&'a BotConfig> {
    let texts: Vec<&str> = messages.iter().filter_map(|e| e.data["text"].as_str()).collect();
    let everyone = texts.iter().any(|t| names_everyone(t));
    let replied = |m: &BotConfig| messages.iter().any(|e| e.data["replyTo"]["author"] == m.id.as_str());
    let named: Vec<&BotConfig> = members.iter().filter(|m| replied(m) || texts.iter().any(|t| names(t, m))).collect();
    if everyone || named.is_empty() { members.iter().collect() } else { named }
}

/// Who answers in a later round: the members another member @-mentioned since the user's last
/// message (`@all` / `@everyone`: all the others) and who haven't spoken or been `asked` since.
/// Nobody, and the room turn ends: a member pulls a teammate in by naming it.
fn cited<'a>(members: &'a [BotConfig], messages: &[Entry], asked: &HashMap<String, i64>) -> Vec<&'a BotConfig> {
    let mut cited_at: HashMap<&str, i64> = HashMap::new();
    // The newest message each member answered or passed on.
    let mut seen: HashMap<&str, i64> = asked.iter().map(|(id, seq)| (id.as_str(), *seq)).collect();
    for e in messages.iter().filter(|e| e.kind == EntryKind::Agent.as_str()) {
        let Some(author) = e.data["author"].as_str() else { continue };
        let at = seen.entry(author).or_default();
        *at = (*at).max(e.seq);
        let text = e.data["text"].as_str().unwrap_or_default();
        let everyone = names_everyone(text);
        for member in members.iter().filter(|m| m.id != author) {
            if everyone || names(text, member) {
                cited_at.insert(&member.id, e.seq);
            }
        }
    }
    let pending = |m: &&BotConfig| {
        cited_at.get(m.id.as_str()).is_some_and(|cited| seen.get(m.id.as_str()).is_none_or(|seen| seen < cited))
    };
    members.iter().filter(pending).collect()
}

/// `@all` or `@everyone` in `text`.
fn names_everyone(text: &str) -> bool {
    mentions(text, "all") || mentions(text, "everyone")
}

/// `member` @-mentioned in `text`.
fn names(text: &str, member: &BotConfig) -> bool {
    handles(&member.name).iter().any(|h| mentions(text, h))
}

/// Ways to @-mention a bot: its full name, without spaces, or its first word (lowercased).
fn handles(name: &str) -> Vec<String> {
    let lower = name.trim().to_lowercase();
    let mut out = vec![lower.clone(), lower.split_whitespace().collect::<String>()];
    if let Some(first) = lower.split_whitespace().next() {
        out.push(first.to_owned());
    }
    out.retain(|h| !h.is_empty());
    out.dedup();
    out
}

/// `@handle` in `text`, not followed by more of a Latin word (`@al` isn't `@alice`).
/// CJK names are often followed straight by the message, so only ASCII letters end a match early.
fn mentions(text: &str, handle: &str) -> bool {
    let lower = text.to_lowercase();
    let needle = format!("@{handle}");
    lower
        .match_indices(&needle)
        .any(|(i, _)| lower[i + needle.len()..].chars().next().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_')))
}

fn member_prompt(hub: &Hub, group: &BotConfig, members: &[BotConfig], me: &BotConfig, lane: &Lane) -> Result<String> {
    let peers: Vec<&BotConfig> = members.iter().filter(|m| m.id != me.id).collect();
    let peer_names = if peers.is_empty() {
        "just the user".to_owned()
    } else {
        peers.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
    };
    let about = group.description.trim();
    let room = if about.is_empty() { format!("\"{}\"", group.name) } else { format!("\"{}\" — {about}", group.name) };
    let mut out =
        vec![format!("You are {}, one participant in a group chat ({room}) with the user and {peer_names}.", me.name)];
    if !peers.is_empty() {
        out.push("Other participants in the room:".into());
        for p in &peers {
            let about =
                if p.description.trim().is_empty() { String::new() } else { format!(" ({})", p.description.trim()) };
            out.push(format!("- {}{about}", p.name));
        }
    }
    out.push(format!(
        "You have your full toolkit here. Do the work first, then answer: your final reply of this turn is \
         the one message the room sees. Keep it short and conversational. After this round, only the \
         participants someone @-mentions answer again: @-mention another participant when you need their \
         answer or action, and only then. If you have nothing new worth adding, reply exactly \"{PASS}\". Never \
         reveal private one-on-one context. Reactions are rare: react(emoji, to) only where a person would, \
         never to a question or a request, `to` naming whose newest message (\"user\" or a participant). With \
         nothing to add, a reaction and \"{PASS}\" agrees without a message."
    ));
    out.push(String::new());

    let seen = hub.store.last_spoke(lane, &me.id);
    let mut lines: Vec<String> = vec![];
    let header = format!("[Group chat: \"{}\" - with {peer_names}]", group.name);
    for e in hub.store.messages_after(lane, seen, HISTORY_LINES)? {
        lines.push(line(hub, group, &e, me));
    }
    out.push(header);
    if lines.is_empty() {
        out.push("No new messages in the room since your last turn.".into());
    } else {
        out.push("New messages in the room (oldest first):".into());
        out.extend(lines);
    }
    out.push(String::new());
    out.push(format!(
        "It's your turn, {}. Reply to the room if you have something worth adding, or reply exactly \"{PASS}\" if you don't.",
        me.name
    ));
    Ok(out.join("\n"))
}

fn line(hub: &Hub, group: &BotConfig, e: &Entry, me: &BotConfig) -> String {
    let text = e.data["text"].as_str().unwrap_or_default();
    let reacted = crate::chat::reactions::describe(hub, e);
    if e.kind == EntryKind::User.as_str() {
        // As in the member's own chat (its skills), with the files the group keeps.
        let root = crate::chat::uploads::root(group);
        let attachments = e.data["attachments"].as_array().map_or(&[][..], Vec::as_slice);
        let reply = crate::chat::replies::note(hub, &me.id, &e.data["replyTo"]).map(|n| format!("{n} "));
        return format!(
            "User: {}{}{reacted}",
            reply.unwrap_or_default(),
            crate::chat::prompt::of_message(&hub.store, &me.skills, &root, text, attachments)
        );
    }
    let author = e.data["author"].as_str().unwrap_or_default();
    let text = crate::agent::acp::truncate(text, LINE_CHARS);
    if author == me.id {
        format!("{} (you): {text}{reacted}", me.name)
    } else {
        format!("{}: {text}{reacted}", hub.bot_name(author))
    }
}

/// A member's final text is a pass (the room doesn't see it).
pub fn is_pass(text: &str) -> bool {
    let t = text.trim().trim_matches(|c: char| c == '"' || c == '.' || c == '*').to_lowercase();
    t.is_empty() || t == PASS || t == "pass"
}

#[cfg(test)]
mod team_tests;
#[cfg(test)]
mod tests;
