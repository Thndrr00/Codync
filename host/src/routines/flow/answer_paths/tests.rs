use super::*;
use crate::store::StepStatus;
use serde_json::json;

fn node(id: &str, kind: NodeKind, answer: &str, check: &str, instruction: &str) -> GraphNode {
    GraphNode {
        id: id.to_owned(),
        kind,
        spec: NodeSpec { answer: answer.to_owned(), check: check.to_owned() },
        instruction: instruction.to_owned(),
    }
}

fn edge(from: &str, to: &str) -> GraphEdge {
    GraphEdge { from: from.to_owned(), to: to.to_owned(), branch: None, is_loop: false }
}

fn branch(from: &str, to: &str, outcome: &str) -> GraphEdge {
    GraphEdge { from: from.to_owned(), to: to.to_owned(), branch: Some(outcome.to_owned()), is_loop: false }
}

fn status(pairs: &[(&str, StepStatus)]) -> BTreeMap<String, StepStatus> {
    pairs.iter().map(|(id, status)| ((*id).to_owned(), *status)).collect()
}

#[test]
fn a_step_whose_only_supplier_failed_fails_clearly() {
    let mut graph = Graph::default();
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "count: int", "", ""));
    graph.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "", "", "Use {{count}}"));
    graph.edges.push(edge("a", "b"));

    assert!(step_answer_gap(&graph, "b", &status(&[("a", StepStatus::Succeeded)])).is_none());
    let gap = step_answer_gap(&graph, "b", &status(&[("a", StepStatus::Failed)])).expect("a gap");
    assert_eq!(gap, AnswerGap::Source { names: "count".to_owned() });
    assert!(gap.text().contains("Run those steps again first"));
    let gap = step_answer_gap(&graph, "b", &status(&[("a", StepStatus::Cancelled)])).expect("a stopped supplier");
    assert!(matches!(gap, AnswerGap::Source { .. }));
}

#[test]
fn an_unknown_name_with_typed_steps_before_it_fails_the_step() {
    let mut graph = Graph::default();
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "count: int", "", ""));
    graph.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "", "", "Use {{missing}}"));
    graph.edges.push(edge("a", "b"));
    let gap = step_answer_gap(&graph, "b", &status(&[("a", StepStatus::Succeeded)])).expect("a setup gap");
    assert!(matches!(gap, AnswerGap::Setup { .. }), "{gap:?}");

    // With no typed step before it, `{{...}}` stays literal for older flows.
    let mut old = Graph::default();
    old.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "", "", "Use {{missing}}"));
    assert!(step_answer_gap(&old, "b", &BTreeMap::new()).is_none());
}

#[test]
fn a_check_field_no_step_gives_fails_the_step() {
    let mut graph = Graph::default();
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "count: int", "", ""));
    graph.nodes.insert("c".to_owned(), node("c", NodeKind::Condition, "", "missing > 2", ""));
    graph.edges.push(edge("a", "c"));
    let problems = graph_problems(&graph);
    assert!(
        problems.iter().any(|p| matches!(p, GraphProblem::Field { name, .. } if name == "missing")),
        "{problems:?}"
    );
}

#[test]
fn starting_a_workflow_checks_its_input_against_the_triggers_fields() {
    let fields = answers::fields_of(&NodeSpec { answer: "count: int; name: text".to_owned(), check: String::new() });
    assert!(run_input_problems(&fields, None).iter().any(|p| p.problem == Problem::Json));
    assert!(
        run_input_problems(&fields, Some(&json!({"Count": 3, "name": "Ada"})))
            .iter()
            .any(|p| p.name == "Count" && p.problem == Problem::Unknown)
    );
    assert!(
        run_input_problems(&fields, Some(&json!({"count": 3})))
            .iter()
            .any(|p| p.name == "name" && p.problem == Problem::Missing)
    );
    assert!(run_input_problems(&fields, Some(&json!({"data": {"count": 3, "name": "Ada"}}))).is_empty());
    assert!(run_input_problems(&fields, Some(&json!({"count": 3, "name": "Ada", "source": "manual"}))).is_empty());
    assert!(run_input_problems(&[], Some(&json!({}))).is_empty(), "a start with no answer fields is unchanged");

    // A webhook body with a wrong-cased field is refused, but extra keys are kept.
    assert!(webhook_problems(&fields, &json!({"count": 3, "name": "Ada", "source": "svc"})).is_empty());
    let wrong = webhook_problems(&fields, &json!({"Count": 3, "name": "Ada"}));
    assert!(
        wrong.iter().any(|problem| problem.name == "Count" && problem.problem == answers::Problem::Unknown),
        "{wrong:?}"
    );
}

#[test]
fn a_retry_uses_the_answer_fields_of_the_run_not_the_edited_workflow() {
    let current = answers::fields_of(&NodeSpec { answer: "new: text".to_owned(), check: String::new() });
    let ran = answers::fields_of(&NodeSpec { answer: "old: int".to_owned(), check: String::new() });
    assert_eq!(with_run_spec(&current, Some(&ran))[0].name, "old");
    assert_eq!(with_run_spec(&current, None)[0].name, "new");
}

#[test]
fn run_one_step_refuses_a_source_that_failed_last_time() {
    let sources = vec!["a".to_owned(), "b".to_owned()];
    let good = status(&[("a", StepStatus::Succeeded), ("b", StepStatus::Succeeded)]);
    assert!(stale_seed_problem(&sources, &good).is_none());
    let bad = status(&[("a", StepStatus::Succeeded), ("b", StepStatus::Failed)]);
    assert!(stale_seed_problem(&sources, &bad).is_some_and(|text| text.contains("b")));
    let stopped = status(&[("a", StepStatus::Cancelled), ("b", StepStatus::Succeeded)]);
    assert!(stale_seed_problem(&sources, &stopped).is_some_and(|text| text.contains("a")));
}

#[test]
fn a_field_missing_on_one_path_into_a_step_is_reported() {
    let mut graph = Graph::default();
    graph.nodes.insert("t1".to_owned(), node("t1", NodeKind::Trigger, "", "", ""));
    graph.nodes.insert("t2".to_owned(), node("t2", NodeKind::Trigger, "", "", ""));
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "name: text", "", ""));
    graph.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "", "", "Use {{name}}"));
    graph.edges.push(edge("t1", "a"));
    graph.edges.push(edge("a", "b"));
    graph.edges.push(edge("t2", "b"));

    let problems = graph_problems(&graph);
    assert!(problems.iter().any(|p| matches!(p, GraphProblem::Path { name, .. } if name == "name")), "{problems:?}");

    // Marking the field optional means a path without it is fine.
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "name?: text", "", ""));
    let problems = graph_problems(&graph);
    assert!(!problems.iter().any(|p| matches!(p, GraphProblem::Path { .. })), "{problems:?}");
}

#[test]
fn one_field_from_two_steps_in_the_same_run_is_reported() {
    let mut graph = Graph::default();
    graph.nodes.insert("t".to_owned(), node("t", NodeKind::Trigger, "", "", ""));
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "name: text", "", ""));
    graph.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "name: text", "", ""));
    graph.nodes.insert("c".to_owned(), node("c", NodeKind::Agent, "", "", "Use {{name}}"));
    graph.edges.push(edge("t", "a"));
    graph.edges.push(edge("t", "b"));
    graph.edges.push(edge("a", "c"));
    graph.edges.push(edge("b", "c"));

    let problems = graph_problems(&graph);
    assert!(
        problems.iter().any(|p| matches!(p, GraphProblem::Repeated { name, .. } if name == "name")),
        "{problems:?}"
    );

    // A condition that takes one of the two branches gives only one supplier per path.
    graph.nodes.insert("cond".to_owned(), node("cond", NodeKind::Condition, "", "urgent", ""));
    graph.edges.retain(|e| !(e.from == "t" && (e.to == "a" || e.to == "b")));
    graph.edges.push(edge("t", "cond"));
    graph.edges.push(branch("cond", "a", "no"));
    graph.edges.push(branch("cond", "b", "yes"));
    let problems = graph_problems(&graph);
    assert!(!problems.iter().any(|p| matches!(p, GraphProblem::Repeated { .. })), "{problems:?}");
}

#[test]
fn the_other_branch_of_a_condition_does_not_give_the_field() {
    let mut graph = Graph::default();
    graph.nodes.insert("t".to_owned(), node("t", NodeKind::Trigger, "", "", ""));
    graph.nodes.insert("cond".to_owned(), node("cond", NodeKind::Condition, "", "urgent", ""));
    graph.nodes.insert("a".to_owned(), node("a", NodeKind::Agent, "name: text", "", ""));
    graph.nodes.insert("b".to_owned(), node("b", NodeKind::Agent, "", "", "Use {{name}}"));
    graph.edges.push(edge("t", "cond"));
    graph.edges.push(branch("cond", "a", "yes"));
    graph.edges.push(branch("cond", "b", "no"));
    graph.edges.push(edge("a", "b"));

    let problems = graph_problems(&graph);
    assert!(problems.iter().any(|p| matches!(p, GraphProblem::Path { name, .. } if name == "name")), "{problems:?}");
}

#[test]
fn answers_from_finished_suppliers_are_merged_for_templates() {
    let source = AnswerSource {
        node_id: "a".to_owned(),
        fields: answers::fields_of(&NodeSpec { answer: "name: text; count: int".to_owned(), check: String::new() }),
    };
    let outputs = BTreeMap::from([("a".to_owned(), "{\"name\":\"Ada\",\"count\":3}".to_owned())]);
    let values = answers_from(&[source], &outputs);
    assert_eq!(values["name"].1, answers::AnswerValue::Text("Ada".to_owned()));
    assert_eq!(values["count"].1, answers::AnswerValue::Integer(3));
    let model = template_values(&values, false);
    assert_eq!(model["name"], "\"Ada\"", "text is JSON inside a model prompt");
    assert_eq!(model["count"], "3");
    let person = template_values(&values, true);
    assert_eq!(person["name"], "Ada", "a person sees plain text");
}
