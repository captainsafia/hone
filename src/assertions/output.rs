use crate::assertions::AssertionResult;
use crate::parser::ast::{
    OutputPredicate, OutputSelector, RegexLiteral, StringComparisonOperator, StringLiteral,
};
use crate::runner::shell::RunResult;

pub fn get_output_value<'a>(result: &'a RunResult, selector: &OutputSelector) -> &'a str {
    match selector {
        OutputSelector::Stdout => &result.stdout,
        OutputSelector::StdoutRaw => &result.stdout_raw,
        OutputSelector::Stderr => &result.stderr,
    }
}

pub fn evaluate_output_predicate(output: &str, predicate: &OutputPredicate) -> AssertionResult {
    match predicate {
        OutputPredicate::Contains { value } => evaluate_contains(output, value),
        OutputPredicate::Matches { value } => evaluate_matches(output, value),
        OutputPredicate::Equals { operator, value } => evaluate_equals(output, operator, value),
    }
}

fn evaluate_contains(output: &str, value: &StringLiteral) -> AssertionResult {
    let passed = output.contains(&value.value);
    AssertionResult::new(
        passed,
        format!("to contain {}", value.raw),
        output.to_string(),
    )
}

fn evaluate_matches(output: &str, value: &RegexLiteral) -> AssertionResult {
    let pattern = if value.flags.is_empty() {
        value.pattern.clone()
    } else {
        format!("(?{}){}", value.flags, value.pattern)
    };

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let passed = re.is_match(output);
            AssertionResult::new(
                passed,
                format!("to match {}", value.raw),
                output.to_string(),
            )
        }
        Err(e) => AssertionResult::with_error(
            false,
            format!("to match {}", value.raw),
            output.to_string(),
            format!("Invalid regex: {}", e),
        ),
    }
}

fn evaluate_equals(
    output: &str,
    operator: &StringComparisonOperator,
    value: &StringLiteral,
) -> AssertionResult {
    let normalized_output = normalize_whitespace(output);
    let normalized_value = normalize_whitespace(&value.value);

    let is_equal = normalized_output == normalized_value;
    let passed = match operator {
        StringComparisonOperator::Equal => is_equal,
        StringComparisonOperator::NotEqual => !is_equal,
    };

    let op_str = match operator {
        StringComparisonOperator::Equal => "==",
        StringComparisonOperator::NotEqual => "!=",
    };

    AssertionResult::new(
        passed,
        format!("{} {}", op_str, value.raw),
        output.to_string(),
    )
}

fn normalize_whitespace(s: &str) -> String {
    s.replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
