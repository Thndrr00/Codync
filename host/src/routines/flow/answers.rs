//! Typed answers of routine-flow steps, ported from OpenBot nightly's workflow answers.
//!
//! A step may declare the fields it gives (`answer: "name: text; count: int"`), and a step after it
//! may test those fields (`check: "count > 2 and it's urgent"`) or name them in its instruction
//! (`{{count}}`). This module is the pure core: it parses the spec, checks a reply against it, fills
//! templates, evaluates checks and builds the shape Claude and the `workflow_answer` tool are given.
//! `answer_paths` decides which fields a step can see; `engine` and `tools` call both.
//!
//! Nothing here touches the store except the small wrappers at the bottom, which keep tool answers
//! across a turn. A reply with no JSON object, a wrongly cased field or a missing required field is
//! a problem the caller gives back to the step so it can correct its answer in the same turn.

use crate::store::Store;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The step setting that declares the fields a step answers with.
pub const ANSWER_FIELD: &str = "answer";
/// The step setting that holds a condition over the fields of earlier steps.
pub const CHECK_FIELD: &str = "check";
/// How many fields one answer may declare.
pub const MAX_FIELDS: usize = 20;
/// The name of the tool an agent step uses to give a checked answer.
pub const ANSWER_TOOL: &str = "workflow_answer";

/// The types an answer field can have. `int` is an alias of `integer`, `bool` of `yes/no`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnswerType {
    Text,
    Number,
    Integer,
    YesNo,
    List,
    Choice,
}

/// One declared field of a step's answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerField {
    pub name: String,
    pub ty: AnswerType,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub optional: bool,
}

/// A checked answer value. A choice is kept as the label the step declared (not the casing sent).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnswerValue {
    Text(String),
    Number(f64),
    Integer(i64),
    YesNo(bool),
    List(Vec<String>),
}

/// A checked answer: only declared fields that had a value are present.
pub type Answer = BTreeMap<String, AnswerValue>;

/// Why a declared spec line is not usable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecProblem {
    Name { line: String },
    Type { name: String, ty: String },
    Labels { name: String },
    Duplicate { name: String },
    Many,
}

/// What is wrong with one answer value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// A required field had no value (blank text and an empty list count as no value).
    Missing,
    /// A number in hex, a list given as text, and the like.
    Type,
    /// A choice value that is not one of its labels.
    Label,
    /// A key the answer does not declare (a wrong-cased field name is one of these).
    Unknown,
    /// The reply had no JSON object at all.
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueProblem {
    pub name: String,
    pub problem: Problem,
}

/// How strictly extra keys are checked. A webhook and a run's input refuse only keys that differ
/// from a declared field in case, so a sender that adds its own keys keeps working. An agent's
/// answer refuses every extra key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraKeys {
    Ignore,
    Refuse,
    RefuseMiscased,
}

/// The saved spec of one node of a flow, frozen into a run when it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    #[serde(default)]
    pub answer: String,
    #[serde(default)]
    pub check: String,
}

/// Reads the answer spec of a node: an empty spec means the node gives no typed answer.
pub fn fields_of(spec: &NodeSpec) -> Vec<AnswerField> {
    if spec.answer.trim().is_empty() { Vec::new() } else { parse_spec(&spec.answer).0 }
}

/// Parses lines like `name: text`, `count: int`, `urgent: yes/no`, `tag: one of high, low`.
/// A trailing `?` makes a field optional; a name without a type is text.
pub fn parse_spec(text: &str) -> (Vec<AnswerField>, Vec<SpecProblem>) {
    let mut fields: Vec<AnswerField> = Vec::new();
    let mut problems = Vec::new();
    for line in text.split(|c| c == '\n' || c == ';').map(str::trim).filter(|line| !line.is_empty()) {
        let colon = line.find(':');
        let raw_name = colon.map_or(line, |at| &line[..at]).trim();
        let optional = raw_name.ends_with('?');
        let name = if optional { raw_name[..raw_name.len() - 1].trim() } else { raw_name };
        if !valid_name(name) {
            problems.push(SpecProblem::Name { line: line.to_owned() });
            continue;
        }
        if fields.iter().any(|field| field.name == name) {
            problems.push(SpecProblem::Duplicate { name: name.to_owned() });
            continue;
        }
        let type_text = colon.map_or("", |at| &line[at + 1..]).trim();
        match parse_type(type_text) {
            None => problems.push(SpecProblem::Type { name: name.to_owned(), ty: type_text.to_owned() }),
            Some((ty, labels)) if ty == AnswerType::Choice && labels.len() < 2 => {
                problems.push(SpecProblem::Labels { name: name.to_owned() });
            }
            Some((ty, labels)) => fields.push(AnswerField { name: name.to_owned(), ty, labels, optional }),
        }
    }
    if fields.len() > MAX_FIELDS {
        problems.push(SpecProblem::Many);
        fields.truncate(MAX_FIELDS);
    }
    (fields, problems)
}

fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 40 || name.starts_with("__") {
        return false;
    }
    (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes[1..].iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

fn parse_type(text: &str) -> Option<(AnswerType, Vec<String>)> {
    let word = text.to_ascii_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let known = match word.as_str() {
        "" | "text" | "string" => Some(AnswerType::Text),
        "number" | "float" => Some(AnswerType::Number),
        "integer" | "int" => Some(AnswerType::Integer),
        "yes/no" | "yesno" | "boolean" | "bool" => Some(AnswerType::YesNo),
        "list" | "list of text" | "text[]" | "string[]" | "array" => Some(AnswerType::List),
        _ => None,
    };
    if let Some(ty) = known {
        return Some((ty, Vec::new()));
    }
    let (labels_text, separator) = if let Some(rest) = text.trim().strip_prefix("one of ") {
        (rest.trim(), if rest.contains('|') { '|' } else { ',' })
    } else if text.contains('|') {
        (text.trim(), '|')
    } else {
        return None;
    };
    let mut labels = Vec::new();
    for label in labels_text.split(separator) {
        let label = label.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        if !label.is_empty() && !labels.iter().any(|known: &String| known.eq_ignore_ascii_case(label)) {
            labels.push(label.to_owned());
        }
    }
    (labels.len() >= 2).then_some((AnswerType::Choice, labels))
}

/// Checks a reply's value against the declared fields.
pub fn check_answer(fields: &[AnswerField], value: &Value, extra: ExtraKeys) -> (Answer, Vec<ValueProblem>) {
    let record = value.as_object();
    let mut problems = Vec::new();
    if let Some(record) = record {
        let names: BTreeSet<&str> = fields.iter().map(|field| field.name.as_str()).collect();
        for key in record.keys() {
            let extra_key = !names.contains(key.as_str())
                && match extra {
                    ExtraKeys::Ignore => false,
                    ExtraKeys::Refuse => true,
                    ExtraKeys::RefuseMiscased => names.iter().any(|name| name.eq_ignore_ascii_case(key)),
                };
            if extra_key {
                problems.push(ValueProblem { name: key.clone(), problem: Problem::Unknown });
            }
        }
    }
    let mut answer = Answer::new();
    for field in fields {
        let Some(raw) = record.and_then(|record| record.get(&field.name)) else {
            if !field.optional {
                problems.push(ValueProblem { name: field.name.clone(), problem: Problem::Missing });
            }
            continue;
        };
        let blank = match raw {
            Value::Null => true,
            Value::String(text) => text.is_empty(),
            Value::Array(items) => items.is_empty(),
            _ => false,
        };
        if blank {
            if !field.optional {
                problems.push(ValueProblem { name: field.name.clone(), problem: Problem::Missing });
            }
            continue;
        }
        match answer_value(field, raw) {
            Ok(value) => {
                answer.insert(field.name.clone(), value);
            }
            Err(problem) => problems.push(ValueProblem { name: field.name.clone(), problem }),
        }
    }
    (answer, problems)
}

fn answer_value(field: &AnswerField, raw: &Value) -> std::result::Result<AnswerValue, Problem> {
    match field.ty {
        AnswerType::Text => match raw {
            Value::String(text) => Ok(AnswerValue::Text(text.trim().to_owned())),
            Value::Number(number) => Ok(AnswerValue::Text(number.to_string())),
            Value::Bool(flag) => Ok(AnswerValue::Text(flag.to_string())),
            _ => Err(Problem::Type),
        },
        AnswerType::Number | AnswerType::Integer => {
            let number = match raw {
                Value::Number(number) => number.as_f64(),
                Value::String(text) => decimal(text.trim()),
                _ => None,
            };
            let Some(number) = number else { return Err(Problem::Type) };
            if field.ty == AnswerType::Integer {
                // A whole number only, and one a JSON reader can hold exactly.
                let whole = number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0;
                whole.then(|| AnswerValue::Integer(number as i64)).ok_or(Problem::Type)
            } else {
                number.is_finite().then_some(AnswerValue::Number(number)).ok_or(Problem::Type)
            }
        }
        AnswerType::YesNo => match raw {
            Value::Bool(flag) => Ok(AnswerValue::YesNo(*flag)),
            Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" => Ok(AnswerValue::YesNo(true)),
                "false" | "no" => Ok(AnswerValue::YesNo(false)),
                _ => Err(Problem::Type),
            },
            _ => Err(Problem::Type),
        },
        AnswerType::List => {
            let Some(items) = raw.as_array() else { return Err(Problem::Type) };
            let mut texts = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(text) => texts.push(text.trim().to_owned()),
                    Value::Number(number) => texts.push(number.to_string()),
                    _ => return Err(Problem::Type),
                }
            }
            texts.retain(|text| !text.is_empty());
            Ok(AnswerValue::List(texts))
        }
        AnswerType::Choice => {
            let Value::String(text) = raw else { return Err(Problem::Type) };
            field
                .labels
                .iter()
                .find(|label| label.eq_ignore_ascii_case(text.trim()))
                .cloned()
                .map(AnswerValue::Text)
                .ok_or(Problem::Label)
        }
    }
}

/// Parses decimal text only: `12`, `-1.5`, `.5`, `1e3`. Not `0x10`, `NaN` or `inf`.
fn decimal(text: &str) -> Option<f64> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut at = usize::from(bytes[0] == b'+' || bytes[0] == b'-');
    let mut digits = 0;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        at += 1;
        digits += 1;
    }
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    (at == bytes.len()).then(|| text.parse::<f64>().ok()).flatten().filter(|number| number.is_finite())
}

/// The last balanced JSON object in a reply, so a short explanation before or after it is fine.
pub fn last_json_object(text: &str) -> Option<Value> {
    let bytes = text.as_bytes();
    let mut found = None;
    let mut start = 0;
    while let Some(open) = text[start..].find('{').map(|at| start + at) {
        if let Some(end) = balanced_end(bytes, open) {
            if let Ok(value) = serde_json::from_str::<Value>(&text[open..=end]) {
                if value.is_object() {
                    found = Some(value);
                    start = end + 1;
                    continue;
                }
            }
        }
        start = open + 1;
    }
    found
}

fn balanced_end(bytes: &[u8], open: usize) -> Option<usize> {
    let (mut depth, mut in_string, mut escaped) = (0i32, false, false);
    for at in open..bytes.len() {
        let byte = bytes[at];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'{' {
            depth += 1;
        } else if byte == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(at);
            }
        }
    }
    None
}

/// The fix a step is given when its answer did not fit, in one line per problem.
pub fn fix_text(problems: &[ValueProblem]) -> String {
    problems
        .iter()
        .map(|problem| match problem.problem {
            Problem::Missing => format!("{} is missing", problem.name),
            Problem::Type => format!("{} has the wrong type", problem.name),
            Problem::Label => format!("{} is not one of the labels", problem.name),
            Problem::Unknown => format!("{} is not one of the fields (names are case-sensitive)", problem.name),
            Problem::Json => "the reply has no JSON object".to_owned(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Writes fields back as a spec line, so a run can freeze the fields it started with.
pub fn spec_text(fields: &[AnswerField]) -> String {
    fields
        .iter()
        .map(|field| {
            let kind = match field.ty {
                AnswerType::Choice => format!("one of {}", field.labels.join(", ")),
                other => other.ty_name().to_owned(),
            };
            format!("{}{}: {}", field.name, if field.optional { "?" } else { "" }, kind)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The field list every answer ask ends with.
pub fn field_instruction(fields: &[AnswerField]) -> String {
    fields
        .iter()
        .map(|field| {
            let hint = match field.ty {
                AnswerType::Choice => format!(
                    "one of {}",
                    field.labels.iter().map(|l| json!(l).to_string()).collect::<Vec<_>>().join(", ")
                ),
                AnswerType::YesNo => "true or false".to_owned(),
                AnswerType::List => "list of text".to_owned(),
                AnswerType::Text | AnswerType::Number | AnswerType::Integer => field.ty.ty_name().to_owned(),
            };
            format!("- {}: {}{}", field.name, hint, if field.optional { ", optional" } else { "" })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl AnswerType {
    pub fn ty_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::YesNo => "yes/no",
            Self::List => "list",
            Self::Choice => "choice",
        }
    }
}

/// The names a step's instruction or check refers to, e.g. `{{count}}`.
pub fn template_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(open) = text[at..].find("{{").map(|i| at + i) {
        let value_start = open + 2;
        let Some(space) = bytes[value_start..].iter().position(|b| *b == b'}') else {
            at = value_start;
            continue;
        };
        let end = value_start + space;
        if end + 1 < bytes.len() && bytes[end + 1] == b'}' {
            let name = text[value_start..end].trim();
            if valid_name(name) && !names.iter().any(|known: &String| known == name) {
                names.push(name.to_owned());
            }
            at = end + 2;
        } else {
            at = value_start;
        }
    }
    names
}

/// Replaces `{{name}}` with a value; a name with no value keeps its braces.
pub fn fill_template(text: &str, values: &BTreeMap<String, String>) -> String {
    if !text.contains("{{") {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(space) = after.find("}}") else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = after[..space].trim();
        match values.get(name) {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[open..open + 2 + space + 2]),
        }
        rest = &after[space + 2..];
    }
    out.push_str(rest);
    out
}

/// How a field reads inside `{{...}}`: text and lists are JSON, numbers and flags are plain.
pub fn template_value(field: &AnswerField, value: &AnswerValue) -> String {
    match (field.ty, value) {
        (AnswerType::Text | AnswerType::List, AnswerValue::Text(text)) => json!(text).to_string(),
        (AnswerType::List, AnswerValue::List(items)) => json!(items).to_string(),
        (_, AnswerValue::YesNo(flag)) => flag.to_string(),
        (_, AnswerValue::Integer(number)) => number.to_string(),
        (_, AnswerValue::Number(number)) => number.to_string(),
        (_, AnswerValue::Text(text)) => text.clone(),
    }
}

/// How a field reads when a person sees it: lists are comma separated and flags are yes/no.
pub fn display_value(value: &AnswerValue) -> String {
    match value {
        AnswerValue::Text(text) => text.clone(),
        AnswerValue::Number(number) => number.to_string(),
        AnswerValue::Integer(number) => number.to_string(),
        AnswerValue::YesNo(flag) => if *flag { "yes" } else { "no" }.to_owned(),
        AnswerValue::List(items) => items.join(", "),
    }
}

/// The plain `name: value` lines shown to a person, e.g. in an approval question.
pub fn answer_lines(answer: &Answer) -> String {
    answer.iter().map(|(name, value)| format!("{name}: {}", display_value(value))).collect::<Vec<_>>().join("\n")
}

// ---------------------------------------------------------------------------
// Conditions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckOperator {
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Contains,
    Set,
    Unset,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckTest {
    pub name: String,
    pub operator: CheckOperator,
    pub value: String,
}

/// A check is groups of tests joined by `or`, each group joined by `and`.
pub type Check = Vec<Vec<CheckTest>>;

/// Parses `count > 2 and it's urgent or not done`. An apostrophe inside a word is not a quote, so
/// the `and` after `it's` still splits the test.
pub fn parse_check(text: &str) -> Option<Check> {
    let mut check = Vec::new();
    for group in split_words(text.trim(), "or")? {
        let tests: Option<Vec<CheckTest>> = split_words(&group, "and")?.iter().map(|test| parse_test(test)).collect();
        let tests = tests?;
        if tests.is_empty() {
            return None;
        }
        check.push(tests);
    }
    (!check.is_empty()).then_some(check)
}

fn split_words(text: &str, word: &str) -> Option<Vec<String>> {
    let bytes = text.as_bytes();
    let needle = format!(" {word} ");
    let needle = needle.as_bytes();
    let mut parts = Vec::new();
    let (mut quote, mut start, mut at) = (None, 0usize, 0usize);
    while at < bytes.len() {
        let byte = bytes[at];
        if let Some(open) = quote {
            if byte == open {
                quote = None;
            }
            at += 1;
            continue;
        }
        // A quote starts a value, never an apostrophe in a word such as `it's`.
        if (byte == b'"' || byte == b'\'')
            && (at == 0 || bytes[at - 1].is_ascii_whitespace() || b"=<>!".contains(&bytes[at - 1]))
        {
            quote = Some(byte);
            at += 1;
            continue;
        }
        if at + needle.len() <= bytes.len() && &bytes[at..at + needle.len()] == needle {
            parts.push(text[start..at].trim().to_owned());
            at += needle.len();
            start = at;
            continue;
        }
        at += 1;
    }
    parts.push(text[start..].trim().to_owned());
    (!parts.iter().any(String::is_empty)).then_some(parts)
}

fn parse_test(text: &str) -> Option<CheckTest> {
    let text = text.trim();
    let (not, rest) = match text.strip_prefix("not ") {
        Some(rest) => (true, rest.trim()),
        None => (false, text),
    };
    if valid_name(rest) {
        return Some(CheckTest {
            name: rest.to_owned(),
            operator: if not { CheckOperator::Unset } else { CheckOperator::Set },
            value: String::new(),
        });
    }
    if not {
        return None;
    }
    if let Some(at) = text.find(|c: char| "=<>!".contains(c)) {
        let name = text[..at].trim();
        if !valid_name(name) {
            return None;
        }
        let rest = &text[at..];
        let operator = ["!=", ">=", "<=", "==", "=", ">", "<"].into_iter().find(|op| rest.starts_with(op))?;
        let value = rest[operator.len()..].trim();
        if value.is_empty() || value.starts_with(|c: char| "=<>!".contains(c)) {
            return None;
        }
        let operator = match operator {
            "!=" => CheckOperator::Ne,
            ">=" => CheckOperator::Ge,
            "<=" => CheckOperator::Le,
            "==" | "=" => CheckOperator::Eq,
            ">" => CheckOperator::Gt,
            _ => CheckOperator::Lt,
        };
        return Some(CheckTest { name: name.to_owned(), operator, value: unquote(value) });
    }
    for (word, operator) in
        [(" is not ", CheckOperator::Ne), (" contains ", CheckOperator::Contains), (" is ", CheckOperator::Eq)]
    {
        if let Some(at) = find_ascii_ci(text, word) {
            let name = &text[..at];
            let value = unquote(text[at + word.len()..].trim());
            if valid_name(name) && !value.is_empty() {
                return Some(CheckTest { name: name.to_owned(), operator, value });
            }
        }
    }
    None
}

fn find_ascii_ci(text: &str, needle: &str) -> Option<usize> {
    let lower = text.to_ascii_lowercase();
    lower.find(needle)
}

fn unquote(text: &str) -> String {
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && (bytes[0] == b'"' || bytes[0] == b'\'') && bytes[bytes.len() - 1] == bytes[0] {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

/// A check passes when any group has all of its tests true. A missing value is false for every
/// operator except `unset`, so `!=` on a value that was never given is false too.
pub fn check_passes(check: &Check, answer: &Answer) -> bool {
    check.iter().any(|group| group.iter().all(|test| test_passes(test, answer.get(&test.name))))
}

fn test_passes(test: &CheckTest, value: Option<&AnswerValue>) -> bool {
    if test.operator == CheckOperator::Set {
        return is_set(value);
    }
    if test.operator == CheckOperator::Unset {
        return !is_set(value);
    }
    let Some(value) = value else { return false };
    let wanted = test.value.trim();
    match value {
        AnswerValue::List(items) => {
            let lower: Vec<&str> = items.iter().map(String::as_str).collect();
            let word = wanted.to_ascii_lowercase();
            match test.operator {
                CheckOperator::Contains => lower.iter().any(|item| item.to_ascii_lowercase().contains(&word)),
                CheckOperator::Eq => lower.iter().any(|item| item.eq_ignore_ascii_case(&word)),
                CheckOperator::Ne => !lower.iter().any(|item| item.eq_ignore_ascii_case(&word)),
                _ => compare_numbers(test.operator, items.len() as f64, decimal(&wanted).unwrap_or(f64::NAN)),
            }
        }
        AnswerValue::Number(number) => match test.operator {
            CheckOperator::Contains => number.to_string().contains(wanted),
            _ => compare_numbers(test.operator, *number, decimal(&wanted).unwrap_or(f64::NAN)),
        },
        AnswerValue::Integer(number) => match test.operator {
            CheckOperator::Contains => number.to_string().contains(wanted),
            _ => compare_numbers(test.operator, *number as f64, decimal(&wanted).unwrap_or(f64::NAN)),
        },
        AnswerValue::YesNo(flag) => {
            let word = wanted.to_ascii_lowercase();
            let truth = word == "true" || word == "yes";
            match test.operator {
                CheckOperator::Eq => *flag == truth,
                CheckOperator::Ne => *flag != truth,
                _ => false,
            }
        }
        AnswerValue::Text(text) => {
            let lower = text.trim().to_ascii_lowercase();
            let word = wanted.to_ascii_lowercase();
            match test.operator {
                CheckOperator::Contains => lower.contains(&word),
                CheckOperator::Eq => lower == word,
                CheckOperator::Ne => lower != word,
                _ => compare_numbers(
                    test.operator,
                    decimal(&lower).unwrap_or(f64::NAN),
                    decimal(&wanted).unwrap_or(f64::NAN),
                ),
            }
        }
    }
}

fn is_set(value: Option<&AnswerValue>) -> bool {
    match value {
        None => false,
        Some(AnswerValue::YesNo(flag)) => *flag,
        Some(AnswerValue::Number(number)) => *number != 0.0,
        Some(AnswerValue::Integer(number)) => *number != 0,
        Some(AnswerValue::List(items)) => !items.is_empty(),
        Some(AnswerValue::Text(text)) => !text.trim().is_empty(),
    }
}

fn compare_numbers(operator: CheckOperator, left: f64, right: f64) -> bool {
    if !left.is_finite() || !right.is_finite() {
        return false;
    }
    match operator {
        CheckOperator::Eq => left == right,
        CheckOperator::Ne => left != right,
        CheckOperator::Gt => left > right,
        CheckOperator::Ge => left >= right,
        CheckOperator::Lt => left < right,
        CheckOperator::Le => left <= right,
        _ => false,
    }
}

/// Whether a check's comparison makes sense for the field's type.
pub fn check_fits(test: &CheckTest, field: &AnswerField) -> bool {
    if matches!(test.operator, CheckOperator::Set | CheckOperator::Unset) {
        return true;
    }
    let ordered =
        matches!(test.operator, CheckOperator::Gt | CheckOperator::Ge | CheckOperator::Lt | CheckOperator::Le);
    let value = test.value.trim().to_ascii_lowercase();
    match field.ty {
        AnswerType::Number | AnswerType::Integer => decimal(&value).is_some(),
        AnswerType::YesNo => {
            matches!(test.operator, CheckOperator::Eq | CheckOperator::Ne)
                && ["yes", "no", "true", "false"].contains(&value.as_str())
        }
        AnswerType::Choice => {
            matches!(test.operator, CheckOperator::Eq | CheckOperator::Ne)
                && field.labels.iter().any(|label| label.eq_ignore_ascii_case(&value))
        }
        AnswerType::List => {
            if ordered {
                decimal(&value).is_some()
            } else {
                !value.is_empty()
            }
        }
        AnswerType::Text => !ordered && !value.is_empty(),
    }
}

// ---------------------------------------------------------------------------
// Asking for an answer: a tool for agents, a JSON schema for Claude, JSON text for other models
// ---------------------------------------------------------------------------

/// The JSON schema of one step's answer, given to Claude as its exact output shape.
pub fn answer_schema(fields: &[AnswerField]) -> Value {
    let mut properties = serde_json::Map::new();
    for field in fields {
        let schema = match field.ty {
            AnswerType::Text => json!({"type": "string"}),
            AnswerType::Number => json!({"type": "number"}),
            AnswerType::Integer => json!({"type": "integer"}),
            AnswerType::YesNo => json!({"type": "boolean"}),
            AnswerType::List => json!({"type": "array", "items": {"type": "string"}}),
            AnswerType::Choice => json!({"type": "string", "enum": &field.labels}),
        };
        properties.insert(field.name.clone(), schema);
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": fields.iter().filter(|field| !field.optional).map(|field| &field.name).collect::<Vec<_>>(),
        "additionalProperties": false,
    })
}

/// The tool's input schema: the answer under `answer`, checked before the tool returns.
pub fn answer_tool_schema(fields: &[AnswerField]) -> Value {
    json!({
        "type": "object",
        "properties": {"answer": answer_schema(fields)},
        "required": ["answer"],
    })
}

/// What an agent step is told: use the tool, correct a problem in the same turn, or end with JSON.
pub fn agent_ask(fields: &[AnswerField]) -> String {
    format!(
        "Give your answer with the {ANSWER_TOOL} tool, as {{\"answer\":{{…}}}}, with these fields:\n{}\n\
         When it is not in your tool list, run it with tool_call. It checks the fields at once; when it \
         tells you a problem, correct the answer and call it again. When you cannot call it, end your \
         reply with the same JSON object.",
        field_instruction(fields)
    )
}

/// The problem sent back by the tool so the agent can correct its answer immediately.
pub fn tool_fix_ask(fields: &[AnswerField], problems: &[ValueProblem]) -> String {
    format!(
        "The answer did not fit: {}. Call {ANSWER_TOOL} again with the corrected answer, with these fields:\n{}",
        fix_text(problems),
        field_instruction(fields)
    )
}

/// The problem appended to the next turn when the agent ended its reply with a bad JSON object.
pub fn answer_fix_ask(fields: &[AnswerField], problems: &[ValueProblem]) -> String {
    format!(
        "Your answer did not fit: {}. Send only the corrected JSON object with these fields:\n{}",
        fix_text(problems),
        field_instruction(fields)
    )
}

/// What a model step is told when it cannot use a tool.
pub fn model_ask(fields: &[AnswerField]) -> String {
    format!("Answer with only one JSON object, with no other text, with these fields:\n{}", field_instruction(fields))
}

/// Claude is given the exact shape instead of a description, so it needs fewer retries.
pub fn claude_ask(fields: &[AnswerField]) -> String {
    format!("{}\nAnswer with only one JSON object fitting this schema:\n{}", model_ask(fields), answer_schema(fields))
}

/// Reads the `answer` argument of a `workflow_answer` tool call.
pub fn tool_answer(args: &Value, fields: &[AnswerField]) -> (Answer, Vec<ValueProblem>) {
    check_answer(fields, &args["answer"], ExtraKeys::Refuse)
}

// ---------------------------------------------------------------------------
// Keeping a tool answer across the turn, and the specs a run started with
// ---------------------------------------------------------------------------

const SPEC_PREFIX: &str = "routineflow.answers.";
const TOOL_PREFIX: &str = "routineflow.toolanswer.";

/// The spec map of a bot's flow, saved beside its canvas documents.
pub fn load_specs(store: &Store, bot: &str) -> BTreeMap<String, NodeSpec> {
    store
        .kv_get(&format!("{SPEC_PREFIX}{bot}"))
        .and_then(|raw| serde_json::from_str::<BTreeMap<String, NodeSpec>>(&raw).ok())
        .unwrap_or_default()
}

/// Saves one node's answer/check spec, or removes it when both are empty.
pub fn save_spec(store: &Store, bot: &str, node_key: &str, answer: Option<&str>, check: Option<&str>) -> Result<()> {
    let mut specs = load_specs(store, bot);
    let spec = NodeSpec {
        answer: answer.unwrap_or_default().trim().to_owned(),
        check: check.unwrap_or_default().trim().to_owned(),
    };
    if spec.answer.is_empty() && spec.check.is_empty() {
        specs.remove(node_key);
    } else {
        specs.insert(node_key.to_owned(), spec);
    }
    store.kv_set(&format!("{SPEC_PREFIX}{bot}"), &serde_json::to_string(&specs)?)
}

/// Remembers the checked answer a tool call gave, for the step that is waiting on it.
pub fn save_tool_answer(store: &Store, run: &str, step: &str, answer: &Answer) -> Result<()> {
    store.kv_set(&format!("{TOOL_PREFIX}{run}:{step}"), &serde_json::to_string(answer)?)
}

/// Takes the answer a tool call gave for a step, if one is waiting.
pub fn take_tool_answer(store: &Store, run: &str, step: &str) -> Option<Answer> {
    let key = format!("{TOOL_PREFIX}{run}:{step}");
    let raw = store.kv_get(&key)?;
    let _ = store.kv_delete(&key);
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests;
