use super::*;
use serde_json::json;

fn fields(spec: &str) -> Vec<AnswerField> {
    parse_spec(spec).0
}

#[test]
fn integer_fields_take_only_whole_numbers_and_number_fields_take_decimal_text() {
    let spec = fields("count: int; size: integer; price: number");
    let (answer, problems) = check_answer(&spec, &json!({"count": 3, "size": "4", "price": "1.5"}), ExtraKeys::Refuse);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(answer["count"], AnswerValue::Integer(3));
    assert_eq!(answer["size"], AnswerValue::Integer(4));
    assert_eq!(answer["price"], AnswerValue::Number(1.5));

    let (_, problems) = check_answer(&spec, &json!({"count": 3.5, "size": 4, "price": 2}), ExtraKeys::Refuse);
    assert_eq!(problems.len(), 1, "3.5 is not a whole number: {problems:?}");
    assert_eq!(problems[0].name, "count");
    assert_eq!(problems[0].problem, Problem::Type);

    let (_, problems) = check_answer(&spec, &json!({"count": "0x10", "size": 4, "price": "0x10"}), ExtraKeys::Refuse);
    assert_eq!(problems.len(), 2, "hex is not decimal text: {problems:?}");
    assert!(problems.iter().all(|problem| problem.problem == Problem::Type));

    let (answer, problems) = check_answer(&spec, &json!({"count": 3, "size": 4, "price": 1e3}), ExtraKeys::Refuse);
    assert!(problems.is_empty(), "a JSON number is a number: {problems:?}");
    assert_eq!(answer["price"], AnswerValue::Number(1000.0));
}

#[test]
fn a_list_field_needs_a_list_not_one_text() {
    let spec = fields("tags: list");
    let (answer, problems) = check_answer(&spec, &json!({"tags": ["one", "two"]}), ExtraKeys::Refuse);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(answer["tags"], AnswerValue::List(vec!["one".to_owned(), "two".to_owned()]));

    let (_, problems) = check_answer(&spec, &json!({"tags": "one, two"}), ExtraKeys::Refuse);
    assert_eq!(problems, vec![ValueProblem { name: "tags".to_owned(), problem: Problem::Type }]);
}

#[test]
fn wrong_case_unknown_and_missing_fields_are_problems() {
    let spec = fields("name: text; urgent: yes/no");
    let (answer, problems) = check_answer(&spec, &json!({"name": " Ada "}), ExtraKeys::Refuse);
    assert_eq!(answer["name"], AnswerValue::Text("Ada".to_owned()));
    assert_eq!(problems, vec![ValueProblem { name: "urgent".to_owned(), problem: Problem::Missing }]);

    let (_, problems) =
        check_answer(&spec, &json!({"Name": "Ada", "name": "Ada", "extra": 1, "urgent": true}), ExtraKeys::Refuse);
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|problem| problem.problem == Problem::Unknown));
    assert!(problems.iter().any(|problem| problem.name == "Name"), "wrong case is a problem");

    // A sender that adds its own keys keeps working; only a wrong case is refused.
    let (_, problems) =
        check_answer(&spec, &json!({"name": "Ada", "urgent": true, "source": "web"}), ExtraKeys::RefuseMiscased);
    assert!(problems.is_empty(), "{problems:?}");
    let (_, problems) = check_answer(&spec, &json!({"NAME": "Ada", "urgent": true}), ExtraKeys::RefuseMiscased);
    assert_eq!(problems, vec![ValueProblem { name: "NAME".to_owned(), problem: Problem::Unknown }]);
}

#[test]
fn a_reply_without_json_is_a_json_problem_and_the_last_object_is_used() {
    let spec = fields("name: text");
    assert_eq!(
        read_reply(&spec, "I think the answer is Ada"),
        vec![ValueProblem { name: "name".to_owned(), problem: Problem::Json }]
    );
    let (answer, problems) = read_reply_ok(&spec, "Here you go: {\"name\": \"Ada\"} and that's all.");
    assert!(problems.is_empty());
    assert_eq!(answer["name"], AnswerValue::Text("Ada".to_owned()));
    // Braces inside a string do not end the object.
    let (answer, _) = read_reply_ok(&spec, "{\"name\": \"Ada } Smith\"}");
    assert_eq!(answer["name"], AnswerValue::Text("Ada } Smith".to_owned()));
}

fn read_reply(fields: &[AnswerField], text: &str) -> Vec<ValueProblem> {
    match last_json_object(text) {
        Some(value) => check_answer(fields, &value, ExtraKeys::Refuse).1,
        None => vec![ValueProblem {
            name: fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>().join(", "),
            problem: Problem::Json,
        }],
    }
}

fn read_reply_ok(fields: &[AnswerField], text: &str) -> (Answer, Vec<ValueProblem>) {
    match last_json_object(text) {
        Some(value) => check_answer(fields, &value, ExtraKeys::Refuse),
        None => (Answer::new(), read_reply(fields, text)),
    }
}

#[test]
fn choice_labels_and_flags_are_checked() {
    let spec = fields("priority: one of high, normal, low; done: bool");
    let (answer, problems) = check_answer(&spec, &json!({"priority": "HIGH", "done": "yes"}), ExtraKeys::Refuse);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(answer["priority"], AnswerValue::Text("high".to_owned()));
    assert_eq!(answer["done"], AnswerValue::YesNo(true));
    let (_, problems) = check_answer(&spec, &json!({"priority": "urgent", "done": true}), ExtraKeys::Refuse);
    assert_eq!(problems, vec![ValueProblem { name: "priority".to_owned(), problem: Problem::Label }]);
}

#[test]
fn an_apostrophe_in_a_word_does_not_hide_and_or() {
    let check = parse_check("title = it's and count > 2 or not done").expect("a check");
    assert_eq!(check.len(), 2, "or splits into two groups: {check:?}");
    assert_eq!(check[0].len(), 2, "and splits the first group: {check:?}");
    assert_eq!(check[0][0].value, "it's", "the apostrophe is part of the value");
    let check = parse_check("urgent and count > 2").expect("a check");
    let answer =
        Answer::from([("urgent".to_owned(), AnswerValue::YesNo(true)), ("count".to_owned(), AnswerValue::Integer(3))]);
    assert!(check_passes(&check, &answer));
    let answer =
        Answer::from([("urgent".to_owned(), AnswerValue::YesNo(false)), ("count".to_owned(), AnswerValue::Integer(3))]);
    assert!(!check_passes(&check, &answer));
}

#[test]
fn a_check_on_a_missing_value_is_false_even_for_not_equal() {
    let check = parse_check("missing != 3").expect("a check");
    let answer = Answer::from([("other".to_owned(), AnswerValue::Integer(3))]);
    assert!(!check_passes(&check, &answer), "!= on a missing field is false");
    let check = parse_check("not missing").expect("a check");
    assert!(check_passes(&check, &answer), "an unset check is true for a missing field");
    let check = parse_check("missing").expect("a check");
    assert!(!check_passes(&check, &answer));

    let answer = Answer::from([("missing".to_owned(), AnswerValue::Integer(4))]);
    assert!(check_passes(&parse_check("missing != 3").expect("a check"), &answer));
}

#[test]
fn template_names_and_fill_replace_only_supplied_fields() {
    assert_eq!(template_names("Use {{count}} and {{name}} and {{count}}"), vec!["count".to_owned(), "name".to_owned()]);
    let values = BTreeMap::from([("count".to_owned(), "3".to_owned())]);
    assert_eq!(fill_template("Use {{count}} and {{name}}", &values), "Use 3 and {{name}}");
    assert_eq!(fill_template("no fields", &values), "no fields");
}

#[test]
fn a_tool_answer_is_checked_when_it_is_given() {
    let spec = fields("name: text; count: int");
    let (answer, problems) = tool_answer(&json!({"answer": {"name": "Ada", "count": 2}}), &spec);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(answer["name"], AnswerValue::Text("Ada".to_owned()));

    let (_, problems) = tool_answer(&json!({"answer": {"Name": "Ada", "name": "Ada", "count": 2}}), &spec);
    assert_eq!(problems, vec![ValueProblem { name: "Name".to_owned(), problem: Problem::Unknown }]);
    let (_, problems) = tool_answer(&json!({"answer": {"name": "Ada"}}), &spec);
    assert_eq!(problems, vec![ValueProblem { name: "count".to_owned(), problem: Problem::Missing }]);
    assert!(tool_fix_ask(&spec, &problems).contains(ANSWER_TOOL), "the agent is told to call the tool again");
    assert!(agent_ask(&spec).contains("- count: integer"), "the agent sees the field types");
}

#[test]
fn claude_is_given_the_exact_answer_shape() {
    let spec = fields("name: text; count: int; urgent: yes/no; tags: list; level: one of high, low; note?: text");
    let schema = answer_schema(&spec);
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["count"]["type"], "integer");
    assert_eq!(schema["properties"]["urgent"]["type"], "boolean");
    assert_eq!(schema["properties"]["tags"]["type"], "array");
    assert_eq!(schema["properties"]["level"]["enum"], json!(["high", "low"]));
    assert_eq!(schema["required"], json!(["name", "count", "urgent", "tags", "level"]));
    assert_eq!(schema["additionalProperties"], false);
    assert!(claude_ask(&spec).contains("\"type\": \"object\""));
    let tool = answer_tool_schema(&spec);
    assert_eq!(tool["required"], json!(["answer"]));
}

#[test]
fn spec_lines_are_bounded_and_reported() {
    let (fields, problems) = parse_spec("name: text; name: text; bad: wat; 9bad: text; one: a|b");
    assert_eq!(fields.len(), 2);
    assert!(problems.iter().any(|p| matches!(p, SpecProblem::Duplicate { .. })));
    assert!(problems.iter().any(|p| matches!(p, SpecProblem::Type { .. })));
    assert!(problems.iter().any(|p| matches!(p, SpecProblem::Name { .. })));
    assert!(problems.iter().any(|p| matches!(p, SpecProblem::Labels { .. })));
    let many = (0..MAX_FIELDS + 2).map(|n| format!("f{n}: text")).collect::<Vec<_>>().join("\n");
    let (fields, problems) = parse_spec(&many);
    assert_eq!(fields.len(), MAX_FIELDS);
    assert!(problems.contains(&SpecProblem::Many));
}

#[test]
fn an_empty_answer_keeps_blank_values_out_and_optional_fields_absent() {
    let spec = fields("note?: text; count: int");
    let (answer, problems) = check_answer(&spec, &json!({"note": "", "count": 0}), ExtraKeys::Refuse);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(!answer.contains_key("note"), "a blank optional field is left out");
    assert_eq!(answer["count"], AnswerValue::Integer(0), "zero is a value");
    assert_eq!(display_value(&AnswerValue::YesNo(false)), "no");
    assert_eq!(answer_lines(&answer), "count: 0");
}
