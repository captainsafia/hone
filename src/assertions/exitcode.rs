use crate::assertions::AssertionResult;
use crate::parser::ast::{ExitCodePredicate, StringComparisonOperator};

pub fn evaluate_exit_code_predicate(
    exit_code: i32,
    predicate: &ExitCodePredicate,
) -> AssertionResult {
    let is_equal = exit_code == predicate.value;
    let passed = match predicate.operator {
        StringComparisonOperator::Equal => is_equal,
        StringComparisonOperator::NotEqual => !is_equal,
    };

    let op_str = match predicate.operator {
        StringComparisonOperator::Equal => "==",
        StringComparisonOperator::NotEqual => "!=",
    };

    AssertionResult::new(
        passed,
        format!("exit_code {} {}", op_str, predicate.value),
        exit_code.to_string(),
    )
}
