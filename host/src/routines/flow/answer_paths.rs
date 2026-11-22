//! Which answer fields a step can see, and the problems of a flow that uses them wrongly.
//!
//! Ported from OpenBot nightly's workflow path check and fail-closed steps. A field is only usable
//! if every path into a step gives it: the other branch of a condition, another trigger or the
//! first pass of a loop must not leave it empty. Two steps that can both give one field in the same
//! run are ambiguous. At run time a step whose only suppliers failed or stopped fails clearly, a
//! `{{name}}` no predecessor supplies fails, and a check on a missing value is false (also for `!=`).
//!
//! Everything here is pure. `engine` builds this graph from the links of a run and calls
//! [`step_answer_gap`] before sending a step; `api` calls [`graph_problems`] to show editor problems
//! and refuses an ambiguous new link.

use super::answers::{self, Answer, AnswerField, AnswerValue, ExtraKeys, NodeSpec, Problem, ValueProblem};
use crate::store::StepStatus;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// How many start/condition combinations the path walk may try before it gives up.
pub const PATH_WALK_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Trigger,
    Agent,
    Model,
    Media,
    Condition,
    Approval,
    Other,
}

impl NodeKind {
    fn passes_through(self) -> bool {
        matches!(self, Self::Condition | Self::Approval)
    }

    fn gives_answer(self) -> bool {
        matches!(self, Self::Agent | Self::Model | Self::Trigger)
    }
}

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub spec: NodeSpec,
    /// The instruction a step runs with; `{{name}}` in it refers to an earlier answer.
    pub instruction: String,
}

impl GraphNode {
    fn fields(&self) -> Vec<AnswerField> {
        if self.kind.gives_answer() { answers::fields_of(&self.spec) } else { Vec::new() }
    }

    /// The condition of a step that tests earlier answers, when it has one.
    pub fn check(&self) -> Option<answers::Check> {
        if self.spec.check.trim().is_empty() { None } else { answers::parse_check(self.spec.check.trim()) }
    }
}

#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    /// Which way out of a condition this edge is (`yes` or `no`).
    pub branch: Option<String>,
    /// A loop-back edge is not a path into a fresh step.
    pub is_loop: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub nodes: BTreeMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug)]
pub struct AnswerSource {
    pub node_id: String,
    pub fields: Vec<AnswerField>,
}

/// The edges that can carry a value into a fresh step. Loop-backs are excluded: the first pass of
/// a loop has no value from the step that will run again later.
pub fn forward_edges(graph: &Graph) -> Vec<GraphEdge> {
    graph.edges.iter().filter(|edge| !edge.is_loop).cloned().collect()
}

/// Every earlier step whose answer this step can read, walking through conditions and approvals.
pub fn answer_sources(graph: &Graph, edges: &[GraphEdge], node_id: &str) -> Vec<AnswerSource> {
    let mut sources = Vec::new();
    let mut seen = BTreeSet::from([node_id.to_owned()]);
    let mut pending = vec![node_id.to_owned()];
    while let Some(id) = pending.pop() {
        for edge in edges.iter().filter(|edge| edge.to == id) {
            if seen.contains(&edge.from) {
                continue;
            }
            seen.insert(edge.from.clone());
            let Some(node) = graph.nodes.get(&edge.from) else { continue };
            let fields = node.fields();
            if !fields.is_empty() {
                sources.push(AnswerSource { node_id: edge.from.clone(), fields });
            } else if node.kind.passes_through() {
                pending.push(edge.from.clone());
            }
        }
    }
    sources
}

/// All the fields a step can name, with every declaration that provides one (to spot conflicts).
pub fn input_fields(graph: &Graph, edges: &[GraphEdge], node_id: &str) -> BTreeMap<String, Vec<AnswerField>> {
    let mut found = BTreeMap::new();
    for source in answer_sources(graph, edges, node_id) {
        for field in source.fields {
            found.entry(field.name.clone()).or_insert_with(Vec::new).push(field);
        }
    }
    found
}

/// The condition a step tests before it runs, if any.
pub fn node_check(node: &GraphNode) -> Option<answers::Check> {
    node.check()
}

/// The fields a node's instruction and check use.
pub fn used_names(node: &GraphNode) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = answers::template_names(&node.instruction).into_iter().collect();
    if let Some(check) = node.check() {
        for test in check.iter().flatten() {
            names.insert(test.name.clone());
        }
    }
    names
}

/// A problem with a flow's answer use, shown in the canvas and refused for agents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphProblem {
    Spec { node: String, name: String },
    Check { node: String },
    Field { node: String, name: String },
    Test { node: String, name: String },
    Template { node: String, field: String, name: String },
    Conflict { node: String, name: String },
    Path { node: String, name: String },
    Repeated { node: String, name: String },
}

impl GraphProblem {
    /// The text an editor shows for this problem.
    pub fn text(&self) -> String {
        match self {
            Self::Spec { name, .. } => format!("answer field {name} has an invalid spec"),
            Self::Check { .. } => "check is not valid. Write tests like: priority = high, count > 2, urgent, not done, title contains \"refund\"; join them with and / or.".to_owned(),
            Self::Field { name, .. } => format!("check uses field {name}, but no connected step before it gives that field in its answer."),
            Self::Test { name, .. } => format!("check cannot test field {name} like that: compare a number with = != > >= < <=, a yes/no with = yes or = no, a choice with = and one of its labels, and text or a list with = != or contains."),
            Self::Template { field, name, .. } => format!("{field} uses {{{{{name}}}}}, but no connected step before it gives that field in its answer, so the step fails when it runs."),
            Self::Conflict { name, .. } => format!("two steps before it give the field {name} with different types. Rename one of them."),
            Self::Path { name, .. } => format!("uses field {name}, but on some paths into this step no step gives it: another condition branch, another trigger, or the first pass of a loop. Give it on every path, or mark it optional ({name}?) in the answer of the steps that give it."),
            Self::Repeated { name, .. } => format!("two steps before it can both give the field {name} in the same run, so it is not clear which value it uses. Rename one of them."),
        }
    }
}

/// Checks one node against the fields reachable from it. `forward` is `None` when only the static
/// problems are wanted (a step about to run); path problems are meaningless then.
fn problems_of(graph: &Graph, edges: Option<&[GraphEdge]>, node: &GraphNode) -> Vec<GraphProblem> {
    let mut problems = Vec::new();
    if node.kind.gives_answer() && !node.spec.answer.trim().is_empty() {
        for problem in answers::parse_spec(&node.spec.answer).1 {
            let name = match problem {
                answers::SpecProblem::Name { line } => line,
                answers::SpecProblem::Type { name, .. } => name,
                answers::SpecProblem::Labels { name } => name,
                answers::SpecProblem::Duplicate { name } => name,
                answers::SpecProblem::Many => answers::ANSWER_FIELD.to_owned(),
            };
            problems.push(GraphProblem::Spec { node: node.id.clone(), name });
        }
    }
    let check = node.check();
    if !node.spec.check.trim().is_empty() && check.is_none() {
        problems.push(GraphProblem::Check { node: node.id.clone() });
    }
    let template_names: Vec<String> = answers::template_names(&node.instruction);
    let check_names: Vec<String> = check.iter().flatten().map(|test| test.name.clone()).collect();
    if template_names.is_empty() && check_names.is_empty() {
        return problems;
    }
    let live = edges.unwrap_or(&[]);
    let available = input_fields(graph, live, &node.id);
    let conflicts = BTreeSet::new();
    let mut conflicts = conflicts;
    let field_of = |name: &str| -> Option<Option<AnswerField>> {
        let options = available.get(name)?;
        let kinds: BTreeSet<String> = options.iter().map(|field| kind_key(field)).collect();
        if kinds.len() == 1 { options.first().cloned().map(Some) } else { Some(None) }
    };
    for name in &template_names {
        if !available.contains_key(name) {
            problems.push(GraphProblem::Template {
                node: node.id.clone(),
                field: "instruction".to_owned(),
                name: name.clone(),
            });
            continue;
        }
        if let Some(None) = field_of(name) {
            problems.push(problem_conflict(&node.id, name, &mut conflicts));
        }
    }
    if let Some(check) = check {
        for test in check.iter().flatten() {
            if !available.contains_key(&test.name) {
                problems.push(GraphProblem::Field { node: node.id.clone(), name: test.name.clone() });
                continue;
            }
            if let Some(Some(field)) = field_of(&test.name) {
                if !answers::check_fits(test, &field) {
                    problems.push(GraphProblem::Test { node: node.id.clone(), name: test.name.clone() });
                }
            } else {
                problems.push(problem_conflict(&node.id, &test.name, &mut conflicts));
            }
        }
    }
    let Some(edges) = edges else { return problems };
    let mut names: Vec<String> = template_names;
    names.extend(check_names);
    names.sort();
    names.dedup();
    let reached: Vec<String> =
        names.into_iter().filter(|name| available.contains_key(name) && !conflicts.contains(name)).collect();
    let required: BTreeSet<String> = reached
        .iter()
        .filter(|name| available.get(*name).is_some_and(|fields| fields.iter().any(|field| !field.optional)))
        .cloned()
        .collect();
    if reached.is_empty() {
        return problems;
    }
    let sources = answer_sources(graph, edges, &node.id);
    for problem in name_path_problems(graph, edges, &node.id, &sources, &reached, &required) {
        problems.push(match problem.code {
            PathCode::Path => GraphProblem::Path { node: node.id.clone(), name: problem.name },
            PathCode::Repeated => GraphProblem::Repeated { node: node.id.clone(), name: problem.name },
        });
    }
    problems
}

fn problem_conflict(node: &str, name: &str, conflicts: &mut BTreeSet<String>) -> GraphProblem {
    conflicts.insert(name.to_owned());
    GraphProblem::Conflict { node: node.to_owned(), name: name.to_owned() }
}

fn kind_key(field: &AnswerField) -> String {
    let kind = if field.ty == answers::AnswerType::Integer { "number" } else { field.ty.ty_name() };
    format!("{kind}:{}", field.labels.join("|"))
}

/// All the answer problems of a flow, for the canvas.
pub fn graph_problems(graph: &Graph) -> Vec<GraphProblem> {
    let edges = forward_edges(graph);
    graph.nodes.values().flat_map(|node| problems_of(graph, Some(&edges), node)).collect()
}

/// Why a step about to run cannot get the answer fields it uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnswerGap {
    /// The step's own spec, check or template names do not match the steps before it.
    Setup { names: String },
    /// Every step that gives a required field failed, was stopped or was skipped.
    Source { names: String },
}

impl AnswerGap {
    /// The error a failing step records.
    pub fn text(&self) -> String {
        match self {
            Self::Setup { names } => {
                format!("The step cannot run: its answer setup is wrong ({names}). Fix the fields it uses.")
            }
            Self::Source { names } => {
                format!("The step cannot run: no step that gives {names} finished. Run those steps again first.")
            }
        }
    }
}

/// Checks a step before it runs: with typed steps before it, a bad field name or a supplier that
/// failed/stopped fails the step instead of handing on an empty value. Without any typed step before
/// it, `{{...}}` stays literal so older flows with braces keep working.
pub fn step_answer_gap(graph: &Graph, node_id: &str, statuses: &BTreeMap<String, StepStatus>) -> Option<AnswerGap> {
    let node = graph.nodes.get(node_id)?;
    let forward = forward_edges(graph);
    let sources = answer_sources(graph, &forward, node_id);
    if sources.is_empty() {
        return None;
    }
    let setup: Vec<GraphProblem> = problems_of(graph, Some(&forward), node)
        .into_iter()
        .filter(|problem| {
            !matches!(problem, GraphProblem::Check { .. } | GraphProblem::Path { .. } | GraphProblem::Repeated { .. })
        })
        .collect();
    if !setup.is_empty() {
        let mut names = setup.iter().map(|problem| problem.name_text()).collect::<Vec<_>>();
        names.sort();
        names.dedup();
        return Some(AnswerGap::Setup { names: names.join(", ") });
    }
    let used = used_names(node);
    if used.is_empty() {
        return None;
    }
    let ended = |source: &AnswerSource| {
        statuses
            .get(&source.node_id)
            .is_some_and(|status| matches!(status, StepStatus::Failed | StepStatus::Cancelled | StepStatus::Skipped))
    };
    let lost: Vec<String> = used
        .into_iter()
        .filter(|name| {
            let giving: Vec<&AnswerSource> =
                sources.iter().filter(|source| source.fields.iter().any(|field| &field.name == name)).collect();
            !giving.is_empty()
                && giving.iter().any(|source| source.fields.iter().any(|field| &field.name == name && !field.optional))
                && giving.iter().all(|source| ended(source))
        })
        .collect();
    (!lost.is_empty()).then(|| AnswerGap::Source { names: lost.join(", ") })
}

impl GraphProblem {
    /// The node whose answer use is wrong, for an editor to mark on the canvas.
    pub fn node_key(&self) -> &str {
        match self {
            Self::Spec { node, .. }
            | Self::Check { node }
            | Self::Field { node, .. }
            | Self::Test { node, .. }
            | Self::Template { node, .. }
            | Self::Conflict { node, .. }
            | Self::Path { node, .. }
            | Self::Repeated { node, .. } => node,
        }
    }

    fn name_text(&self) -> String {
        match self {
            Self::Spec { name, .. }
            | Self::Field { name, .. }
            | Self::Test { name, .. }
            | Self::Conflict { name, .. }
            | Self::Path { name, .. }
            | Self::Repeated { name, .. } => name.clone(),
            Self::Template { name, .. } => name.clone(),
            Self::Check { .. } => answers::CHECK_FIELD.to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Paths: which suppliers can run at the same time as this step
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathCode {
    Path,
    Repeated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamePathProblem {
    pub name: String,
    pub code: PathCode,
}

/// For each possible combination of trigger and condition outcomes that reaches `node_id`, check
/// how many suppliers of each field run: none means a path without a value, two means an ambiguous
/// value. A repeated supplier wins over a missing one.
pub fn name_path_problems(
    graph: &Graph,
    edges: &[GraphEdge],
    node_id: &str,
    sources: &[AnswerSource],
    names: &[String],
    required: &BTreeSet<String>,
) -> Vec<NamePathProblem> {
    if names.is_empty() {
        return Vec::new();
    }
    let mut before: BTreeMap<String, Vec<GraphEdge>> = BTreeMap::new();
    for edge in edges {
        before.entry(edge.to.clone()).or_default().push(edge.clone());
    }
    let ancestors = ancestors_of(node_id, &before);
    let conditions: Vec<String> = ancestors
        .iter()
        .filter(|id| {
            id.as_str() != node_id && graph.nodes.get(id.as_str()).is_some_and(|node| node.kind == NodeKind::Condition)
        })
        .cloned()
        .collect();
    let triggers: Vec<String> = ancestors
        .iter()
        .filter(|id| graph.nodes.get(*id).is_some_and(|node| node.kind == NodeKind::Trigger))
        .cloned()
        .collect();
    let starts: Vec<Option<String>> =
        if triggers.is_empty() { vec![None] } else { triggers.into_iter().map(Some).collect() };
    if starts.len() * (1usize << conditions.len()) > PATH_WALK_LIMIT {
        return Vec::new();
    }
    let mut found: BTreeMap<String, PathCode> = BTreeMap::new();
    for start in &starts {
        for mask in 0..(1usize << conditions.len()) {
            let outcome: BTreeMap<String, String> = conditions
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    let branch = if mask & (1 << index) == 0 { "no" } else { "yes" };
                    (id.clone(), branch.to_owned())
                })
                .collect();
            let runs = Checker {
                graph,
                before: &before,
                start: start.as_deref(),
                outcome,
                known: std::cell::RefCell::new(BTreeMap::new()),
            };
            if !runs(node_id) {
                continue;
            }
            let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
            for name in names {
                let count = sources
                    .iter()
                    .filter(|source| source.fields.iter().any(|field| &field.name == name) && runs(&source.node_id))
                    .count();
                by_name.insert(name.as_str(), count);
            }
            for (name, count) in by_name {
                if count > 1 {
                    found.insert(name.to_owned(), PathCode::Repeated);
                } else if count == 0 && required.contains(name) && !found.contains_key(name) {
                    found.insert(name.to_owned(), PathCode::Path);
                }
            }
        }
    }
    found.into_iter().map(|(name, code)| NamePathProblem { name, code }).collect()
}

fn ancestors_of(node_id: &str, before: &BTreeMap<String, Vec<GraphEdge>>) -> Vec<String> {
    let mut seen = BTreeSet::from([node_id.to_owned()]);
    let mut pending = vec![node_id.to_owned()];
    while let Some(id) = pending.pop() {
        for edge in before.get(&id).into_iter().flatten() {
            if seen.insert(edge.from.clone()) {
                pending.push(edge.from.clone());
            }
        }
    }
    seen.into_iter().collect()
}

/// Whether each node runs for one trigger and one combination of condition outcomes. The walk is
/// memoised so a diamond of links does not turn into an exponential search.
struct Checker<'a> {
    graph: &'a Graph,
    before: &'a BTreeMap<String, Vec<GraphEdge>>,
    start: Option<&'a str>,
    outcome: BTreeMap<String, String>,
    known: std::cell::RefCell<BTreeMap<String, bool>>,
}

impl Checker<'_> {
    fn active(&self, edge: &GraphEdge) -> bool {
        self.graph.nodes.get(&edge.from).is_none_or(|node| node.kind != NodeKind::Condition)
            || self.outcome.get(&edge.from).is_some_and(|branch| Some(branch.as_str()) == edge.branch.as_deref())
    }

    fn runs(&self, id: &str) -> bool {
        if let Some(saved) = self.known.borrow().get(id) {
            return *saved;
        }
        // Mark it as not running while visiting its ancestors: a loop would otherwise recurse forever.
        self.known.borrow_mut().insert(id.to_owned(), false);
        let result = match self.graph.nodes.get(id) {
            None => false,
            Some(node) if node.kind == NodeKind::Trigger => Some(id) == self.start,
            Some(_) => self
                .before
                .get(id)
                .map_or(true, |edges| edges.iter().any(|edge| self.active(edge) && self.runs(&edge.from))),
        };
        self.known.borrow_mut().insert(id.to_owned(), result);
        result
    }
}

// ---------------------------------------------------------------------------
// Run input, webhook bodies, retries and "run one step"
// ---------------------------------------------------------------------------

/// Checks the input an agent gave when it started a workflow against the trigger's answer fields.
/// No fields means no check, so a plain start is unchanged. The input may wrap the answers in
/// `data`, as a webhook does.
pub fn run_input_problems(fields: &[AnswerField], input: Option<&Value>) -> Vec<ValueProblem> {
    if fields.is_empty() {
        return Vec::new();
    }
    let Some(input) = input else { return vec![json_problem(fields)] };
    let record = input.as_object().cloned().or_else(|| {
        let text = match input {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        answers::last_json_object(&text).and_then(|value| value.as_object().cloned())
    });
    let Some(record) = record else { return vec![json_problem(fields)] };
    let wrapped = Value::Object(record);
    let given = wrapped.get("data").filter(|value| value.is_object()).unwrap_or(&wrapped);
    answers::check_answer(fields, given, ExtraKeys::RefuseMiscased).1
}

/// The checked answer an input object gives, when it fits. A plain start gives none and keeps its
/// text reply; a start with typed input hands the checked JSON object to the first steps.
pub fn run_input_answer(fields: &[AnswerField], input: Option<&Value>) -> Option<Answer> {
    if fields.is_empty() || !run_input_problems(fields, input).is_empty() {
        return None;
    }
    let input = input?;
    let record = input.as_object().cloned().or_else(|| {
        let text = match input {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        answers::last_json_object(&text).and_then(|value| value.as_object().cloned())
    })?;
    let wrapped = Value::Object(record);
    let given = wrapped.get("data").filter(|value| value.is_object()).unwrap_or(&wrapped);
    let (answer, problems) = answers::check_answer(fields, given, ExtraKeys::RefuseMiscased);
    problems.is_empty().then_some(answer)
}

fn json_problem(fields: &[AnswerField]) -> ValueProblem {
    ValueProblem {
        name: fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", "),
        problem: Problem::Json,
    }
}

/// Checks a webhook body. Extra keys from the sender are fine; a field name that differs from a
/// declared field only in case is refused.
pub fn webhook_problems(fields: &[AnswerField], data: &Value) -> Vec<ValueProblem> {
    if fields.is_empty() {
        return Vec::new();
    }
    if !data.is_object() {
        return vec![json_problem(fields)];
    }
    answers::check_answer(fields, data, ExtraKeys::RefuseMiscased).1
}

/// The answer fields of a retry come from the run that is being retried, not the edited workflow:
/// steps after it were checked against that spec. `None` means the run predates typed answers.
pub fn with_run_spec<'a>(current: &'a [AnswerField], run: Option<&'a [AnswerField]>) -> &'a [AnswerField] {
    run.unwrap_or(current)
}

/// "Run one step" must not reuse the answer of a step before it when the newest attempt of that
/// step failed or was stopped. It asks the person to run that step again first.
pub fn stale_seed_problem(sources: &[String], statuses: &BTreeMap<String, StepStatus>) -> Option<String> {
    let ended: Vec<&str> = sources
        .iter()
        .filter(|id| {
            statuses.get(*id).is_some_and(|status| matches!(status, StepStatus::Failed | StepStatus::Cancelled))
        })
        .map(String::as_str)
        .collect();
    (!ended.is_empty()).then(|| {
        format!("A step before this one ({}) failed or was stopped. Run that step again first.", ended.join(", "))
    })
}

/// The names a step uses, for the stale-seed check above.
pub fn source_ids(graph: &Graph, edges: &[GraphEdge], node_id: &str) -> Vec<String> {
    let mut ids: Vec<String> = edges.iter().filter(|edge| edge.to == node_id).map(|edge| edge.from.clone()).collect();
    for source in answer_sources(graph, edges, node_id) {
        if !ids.contains(&source.node_id) {
            ids.push(source.node_id);
        }
    }
    ids
}

/// The values a step's `{{name}}` fields take from the answers of earlier steps. Text and lists are
/// JSON inside a model prompt, plain when a person reads them.
pub fn template_values(
    answers_by_name: &BTreeMap<String, (AnswerField, AnswerValue)>,
    for_person: bool,
) -> BTreeMap<String, String> {
    answers_by_name
        .iter()
        .map(|(name, (field, value))| {
            let text = if for_person { answers::display_value(value) } else { answers::template_value(field, value) };
            (name.clone(), text)
        })
        .collect()
}

/// Shows a supplier's typed answer as plain `name: value` lines instead of raw JSON when the next
/// step is given that answer. A list is comma separated and a flag is yes/no, not JSON quotes.
pub fn readable_input(fields: &[AnswerField], text: &str) -> Option<String> {
    let object = answers::last_json_object(text)?;
    let (answer, problems) = answers::check_answer(fields, &object, ExtraKeys::Ignore);
    (!answer.is_empty() && problems.is_empty()).then(|| answers::answer_lines(&answer))
}

/// Collects the checked answers of finished supplier steps, earlier suppliers first.
pub fn answers_from(
    sources: &[AnswerSource],
    outputs: &BTreeMap<String, String>,
) -> BTreeMap<String, (AnswerField, AnswerValue)> {
    let mut found = BTreeMap::new();
    for source in sources {
        let Some(text) = outputs.get(&source.node_id) else { continue };
        let Some(value) = answers::last_json_object(text) else { continue };
        let (answer, _) = answers::check_answer(&source.fields, &value, ExtraKeys::Ignore);
        for field in &source.fields {
            if let Some(value) = answer.get(&field.name) {
                found.insert(field.name.clone(), (field.clone(), value.clone()));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests;
