//! The rules of one routine's flow without the database: which bots it reaches, the step each
//! runs in and which new link it accepts. A flow starts at the routine's own bot and follows the
//! links of that routine only.

use crate::store::FlowLink;
use crate::store::flows::{Branch, PortType};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

/// How many hops from the routine's own bot a flow may reach, unless configured otherwise.
pub const DEFAULT_MAX_DEPTH: usize = 5;
/// The most a configured depth may be.
pub const MAX_DEPTH_LIMIT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    SameBot,
    IntoOwner,
    NotOnPath,
    Duplicate,
    Cycle,
    TooDeep(usize),
    /// OpenBot `WORKFLOW_PORTS` and edge rules refuse this typed connection.
    Port(PortProblem),
}

/// Why a typed edge cannot join two nodes, from OpenBot `workflowEdgeProblem`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortProblem {
    Annotation,
    Port,
    ConditionBranch,
    Branch,
    Duplicate,
}

pub const ALL_PORTS: &[PortType] = &[PortType::Text, PortType::Image, PortType::File];
const TEXT_PORT: &[PortType] = &[PortType::Text];
const MEDIA_PORTS: &[PortType] = &[PortType::Image, PortType::File];
const NO_PORTS: &[PortType] = &[];

/// The ports a node accepts and gives, from OpenBot `WORKFLOW_PORTS`. The prefix of a node key names
/// its kind (`bot:`, `media:`, `condition:`…); an unknown or drawing-only node has no ports.
pub fn ports_of(node: &str) -> Option<(&'static [PortType], &'static [PortType])> {
    let kind = node.split_once(':').map_or("bot", |(prefix, _)| prefix);
    match kind {
        "routine" | "bot" | "agent" | "llm" => Some((ALL_PORTS, TEXT_PORT)),
        "media" | "image" => Some((ALL_PORTS, MEDIA_PORTS)),
        "asset" | "trigger" | "prompt" => Some((NO_PORTS, TEXT_PORT)),
        "condition" | "approval" => Some((TEXT_PORT, TEXT_PORT)),
        "output" => Some((ALL_PORTS, NO_PORTS)),
        "text" | "note" | "shape" => None,
        _ => None,
    }
}

/// Checks one edge against its ends and every other edge of the flow. A legacy text edge keeps
/// working even where a newer port table would not list text: old flows must not break on upgrade.
pub fn edge_problem(from: &str, to: &str, edges: &[FlowLink]) -> Option<PortProblem> {
    let edge = edges.iter().find(|link| link.from_bot_id == from && link.to_bot_id == to)?;
    let (Some((out, _)), Some((input, to_out))) = (ports_of(from), ports_of(to)) else {
        return Some(PortProblem::Annotation);
    };
    if out.is_empty() && to_out.is_empty() {
        return Some(PortProblem::Annotation);
    }
    if edge.port != PortType::Text && (!out.contains(&edge.port) || !input.contains(&edge.port)) {
        return Some(PortProblem::Port);
    }
    let condition = from.starts_with("condition:");
    match (condition, edge.branch.as_deref()) {
        (true, None) => return Some(PortProblem::ConditionBranch),
        (true, Some(branch)) if Branch::parse(branch).is_none() => return Some(PortProblem::ConditionBranch),
        (false, Some(_)) => return Some(PortProblem::Branch),
        _ => {}
    }
    let same = edges
        .iter()
        .filter(|link| {
            link.from_bot_id == from
                && link.to_bot_id == to
                && link.port == edge.port
                && link.branch.as_deref() == edge.branch.as_deref()
        })
        .count();
    (same > 1).then_some(PortProblem::Duplicate)
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameBot => f.write_str("a bot can't hand work to itself"),
            Self::IntoOwner => {
                f.write_str("the routine's own bot starts the flow, so nothing can hand work back to it")
            }
            Self::NotOnPath => {
                f.write_str("the flow doesn't reach that bot yet: connect it from the routine's bot first")
            }
            Self::Duplicate => f.write_str("these two bots are already connected in this routine"),
            Self::Cycle => f.write_str("that link would make the flow loop"),
            Self::TooDeep(max) => write!(f, "a flow reaches at most {max} bots deep"),
            Self::Port(problem) => match problem {
                PortProblem::Annotation => f.write_str("a drawing note cannot hand work on"),
                PortProblem::Port => f.write_str("these two steps do not share that port type"),
                PortProblem::ConditionBranch => f.write_str("a condition needs a yes or no branch on its edge"),
                PortProblem::Branch => f.write_str("only a condition can route a yes or no branch"),
                PortProblem::Duplicate => f.write_str("these two steps already have that edge"),
            },
        }
    }
}

impl std::error::Error for Problem {}

/// The links form a cycle, so no order of steps exists.
#[derive(Debug, PartialEq, Eq)]
pub struct Cyclic;

/// The step each bot of the flow runs in: 0 for the routine's own bot and, for the rest, one more
/// than the deepest bot handing work to it. A bot the flow doesn't reach is absent.
pub fn depths(owner: &str, links: &[FlowLink]) -> Result<BTreeMap<String, usize>, Cyclic> {
    let reached = reachable(owner, links);
    let edges: Vec<&FlowLink> = links.iter().filter(|l| reached.contains(l.from_bot_id.as_str())).collect();
    let mut waiting: BTreeMap<&str, usize> = BTreeMap::new();
    for link in &edges {
        *waiting.entry(link.to_bot_id.as_str()).or_default() += 1;
    }
    if waiting.contains_key(owner) {
        return Err(Cyclic);
    }
    let mut depth = BTreeMap::from([(owner.to_owned(), 0)]);
    let mut ready = VecDeque::from([owner]);
    let mut settled = 1;
    while let Some(bot) = ready.pop_front() {
        let here = depth[bot];
        for link in edges.iter().filter(|l| l.from_bot_id == bot) {
            let to = link.to_bot_id.as_str();
            let best = depth.entry(to.to_owned()).or_insert(0);
            *best = (*best).max(here + 1);
            let left = waiting.get_mut(to).expect("every target was counted");
            *left -= 1;
            if *left == 0 {
                settled += 1;
                ready.push_back(to);
            }
        }
    }
    if settled == reached.len() { Ok(depth) } else { Err(Cyclic) }
}

fn reachable<'a>(start: &'a str, links: &'a [FlowLink]) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut queue = vec![start];
    while let Some(bot) = queue.pop() {
        if seen.insert(bot) {
            queue.extend(links.iter().filter(|l| l.from_bot_id == bot).map(|l| l.to_bot_id.as_str()));
        }
    }
    seen
}

/// Every bot the flow reaches from the routine's own, the owner left out, even through a cycle.
pub fn reached(owner: &str, links: &[FlowLink]) -> Vec<String> {
    reachable(owner, links).into_iter().filter(|b| *b != owner).map(str::to_owned).collect()
}

/// The step a run starts at and every step it hands work on to, `start` included. The other steps of
/// the run keep their stored answers instead of being sent again. Stored links are acyclic, so the
/// walk always ends; a stray cycle only repeats nodes already seen.
pub fn downstream<'a>(start: &'a str, links: &'a [FlowLink]) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut queue = vec![start];
    while let Some(node) = queue.pop() {
        if !seen.insert(node) {
            continue;
        }
        queue.extend(links.iter().filter(|l| l.from_bot_id == node).map(|l| l.to_bot_id.as_str()));
    }
    seen
}

/// Why `from` can't hand work to `to` in this routine, or `None` when it can. A legacy text edge
/// with no branch is assumed; callers adding a typed edge use `connect_problem_typed`.
pub fn connect_problem(owner: &str, links: &[FlowLink], from: &str, to: &str, max_depth: usize) -> Option<Problem> {
    connect_problem_typed(owner, links, from, to, max_depth, PortType::Text, None)
}

/// The same path rules as `connect_problem`, with the port and branch of the edge being added.
pub fn connect_problem_typed(
    owner: &str,
    links: &[FlowLink],
    from: &str,
    to: &str,
    max_depth: usize,
    port: PortType,
    branch: Option<&str>,
) -> Option<Problem> {
    if from == to {
        return Some(Problem::SameBot);
    }
    if to == owner {
        return Some(Problem::IntoOwner);
    }
    let Ok(current) = depths(owner, links) else { return Some(Problem::Cycle) };
    if !current.contains_key(from) {
        return Some(Problem::NotOnPath);
    }
    if links.iter().any(|l| l.from_bot_id == from && l.to_bot_id == to) {
        return Some(Problem::Duplicate);
    }
    if reachable(to, links).contains(from) {
        return Some(Problem::Cycle);
    }
    // The new hop puts `to` one step after `from`, and everything `to` hands work on moves with it.
    let deepest_after = longest_from(to, links);
    if current[from] + 1 + deepest_after > max_depth {
        return Some(Problem::TooDeep(max_depth));
    }
    // `links` holds the edges already stored, so add the candidate to check its type and branch
    // against both ends and against an existing edge between the same two steps.
    let candidate = FlowLink {
        id: String::new(),
        routine_id: String::new(),
        owner_bot_id: owner.to_owned(),
        from_bot_id: from.to_owned(),
        to_bot_id: to.to_owned(),
        name: String::new(),
        instruction: String::new(),
        model: None,
        model_options: BTreeMap::new(),
        branch: branch.map(str::to_owned),
        port,
        created_at: 0,
    };
    let mut edges = links.to_vec();
    edges.push(candidate);
    edge_problem(from, to, &edges).map(Problem::Port)
}

/// The longest chain of hops that starts at `bot`.
fn longest_from(bot: &str, links: &[FlowLink]) -> usize {
    walk(bot, links, &mut BTreeMap::new())
}

fn walk<'a>(bot: &'a str, links: &'a [FlowLink], best: &mut BTreeMap<&'a str, usize>) -> usize {
    if let Some(known) = best.get(bot) {
        return *known;
    }
    // The links are acyclic here (the caller checked), so the walk ends.
    let longest =
        links.iter().filter(|l| l.from_bot_id == bot).map(|l| 1 + walk(&l.to_bot_id, links, best)).max().unwrap_or(0);
    best.insert(bot, longest);
    longest
}

/// The bots in the order their steps run: shallowest first, the routine's own bot left out.
pub fn order(depths: &BTreeMap<String, usize>) -> Vec<(&str, usize)> {
    let mut all: Vec<_> = depths.iter().filter(|(_, d)| **d > 0).map(|(b, d)| (b.as_str(), *d)).collect();
    all.sort_by_key(|(bot, depth)| (*depth, *bot));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(from: &str, to: &str) -> FlowLink {
        FlowLink {
            id: format!("{from}-{to}"),
            routine_id: "r".into(),
            owner_bot_id: "a".into(),
            from_bot_id: from.into(),
            to_bot_id: to.into(),
            name: String::new(),
            instruction: String::new(),
            model: None,
            model_options: BTreeMap::new(),
            branch: None,
            port: PortType::Text,
            created_at: 0,
        }
    }

    #[test]
    fn depth_is_the_longest_path() {
        let links = [link("a", "b"), link("b", "c"), link("a", "c"), link("a", "d")];
        let d = depths("a", &links).unwrap();
        assert_eq!((d["a"], d["b"], d["c"], d["d"]), (0, 1, 2, 1));
        assert_eq!(order(&d), [("b", 1), ("d", 1), ("c", 2)]);
    }

    #[test]
    fn a_bot_the_flow_does_not_reach_is_absent() {
        let d = depths("a", &[link("a", "b"), link("x", "y")]).unwrap();
        assert!(!d.contains_key("x") && !d.contains_key("y"));
    }

    #[test]
    fn connecting_checks_every_rule() {
        let links = [link("a", "b"), link("b", "c")];
        let check = |from, to| connect_problem("a", &links, from, to, 5);
        assert_eq!(check("b", "b"), Some(Problem::SameBot));
        assert_eq!(check("c", "a"), Some(Problem::IntoOwner));
        assert_eq!(check("x", "d"), Some(Problem::NotOnPath));
        assert_eq!(check("a", "b"), Some(Problem::Duplicate));
        assert_eq!(check("c", "b"), Some(Problem::Cycle));
        assert_eq!(check("b", "d"), None);
        assert_eq!(check("a", "c"), None, "a second path to the same bot is a join");
    }

    #[test]
    fn a_cycle_in_stored_links_is_detected() {
        let links = [link("a", "b"), link("b", "c"), link("c", "b")];
        assert_eq!(depths("a", &links), Err(Cyclic));
        assert_eq!(connect_problem("a", &links, "b", "d", 5), Some(Problem::Cycle));
        assert_eq!(depths("a", &[link("a", "b"), link("b", "a")]), Err(Cyclic));
    }

    #[test]
    fn a_media_step_is_hung_like_a_bot_and_can_close_a_loop() {
        let links = [link("a", "media:1"), link("media:1", "b")];
        let d = depths("a", &links).unwrap();
        assert_eq!((d["media:1"], d["b"]), (1, 2));
        assert_eq!(order(&d), [("media:1", 1), ("b", 2)]);
        assert_eq!(connect_problem("a", &links, "b", "media:1", 5), Some(Problem::Cycle));
        assert_eq!(connect_problem("a", &links, "media:1", "c", 2), None);
    }

    #[test]
    fn typed_edges_refuse_mismatched_duplicate_and_branchless_links() {
        let mut image = link("media:1", "b");
        image.port = PortType::Image;
        assert_eq!(edge_problem("media:1", "b", &[image]), None, "a media step gives an image");
        let mut accept = link("a", "b");
        accept.port = PortType::Image;
        assert_eq!(edge_problem("a", "b", &[accept]), Some(PortProblem::Port), "a bot does not give images");

        let mut file = link("media:1", "b");
        file.port = PortType::File;
        assert_eq!(edge_problem("media:1", "b", &[file.clone(), file.clone()]), Some(PortProblem::Duplicate));

        let mut condition = link("condition:1", "b");
        assert_eq!(edge_problem("condition:1", "b", &[condition.clone()]), Some(PortProblem::ConditionBranch));
        condition.branch = Some("yes".into());
        assert_eq!(edge_problem("condition:1", "b", &[condition.clone()]), None);
        let mut plain = link("a", "b");
        plain.branch = Some("no".into());
        assert_eq!(edge_problem("a", "b", &[plain]), Some(PortProblem::Branch));
        assert_eq!(
            connect_problem_typed("a", &[], "a", "b", 5, PortType::Text, Some("no")),
            Some(Problem::Port(PortProblem::Branch)),
            "the connect path uses the same typed-edge decision"
        );
        assert_eq!(edge_problem("note:1", "b", &[link("note:1", "b")]), Some(PortProblem::Annotation));
    }

    #[test]
    fn depth_is_limited_including_what_hangs_below_the_new_link() {
        let chain = [link("a", "b"), link("b", "c")];
        assert_eq!(connect_problem("a", &chain, "c", "d", 3), None);
        assert_eq!(connect_problem("a", &chain, "c", "d", 2), Some(Problem::TooDeep(2)));
        // Hanging a long branch under a shallow bot pushes the branch down.
        let links = [link("a", "b"), link("a", "x"), link("x", "y"), link("y", "z")];
        assert_eq!(connect_problem("a", &links, "b", "x", 3), Some(Problem::TooDeep(3)));
        assert_eq!(connect_problem("a", &links, "b", "x", 4), None);
    }

    #[test]
    fn a_run_from_a_step_reaches_only_that_step_and_what_hangs_below_it() {
        // a → b → d, a → c → d. Starting at b re-runs b and d; c keeps its stored answer.
        let links = [link("a", "b"), link("b", "d"), link("a", "c"), link("c", "d")];
        assert_eq!(downstream("b", &links), BTreeSet::from(["b", "d"]));
        assert_eq!(downstream("c", &links), BTreeSet::from(["c", "d"]));
        assert_eq!(downstream("d", &links), BTreeSet::from(["d"]));
        // A step that hangs below a media step is reached the same way.
        let media = [link("a", "media:1"), link("media:1", "b")];
        assert_eq!(downstream("media:1", &media), BTreeSet::from(["media:1", "b"]));
    }
}
