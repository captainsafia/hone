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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_predicate(op: StringComparisonOperator, value: i32) -> ExitCodePredicate {
        ExitCodePredicate { operator: op, value }
    }

    #[test]
    fn test_equal_match() {
        let predicate = make_predicate(StringComparisonOperator::Equal, 0);
        let result = evaluate_exit_code_predicate(0, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_equal_no_match() {
        let predicate = make_predicate(StringComparisonOperator::Equal, 0);
        let result = evaluate_exit_code_predicate(1, &predicate);
        assert!(!result.passed);
    }

    #[test]
    fn test_not_equal_match() {
        let predicate = make_predicate(StringComparisonOperator::NotEqual, 0);
        let result = evaluate_exit_code_predicate(1, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_not_equal_no_match() {
        let predicate = make_predicate(StringComparisonOperator::NotEqual, 0);
        let result = evaluate_exit_code_predicate(0, &predicate);
        assert!(!result.passed);
    }

    #[test]
    fn test_negative_exit_code() {
        let predicate = make_predicate(StringComparisonOperator::Equal, -1);
        let result = evaluate_exit_code_predicate(-1, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_large_exit_code() {
        let predicate = make_predicate(StringComparisonOperator::Equal, 255);
        let result = evaluate_exit_code_predicate(255, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_expected_string_format() {
        let predicate = make_predicate(StringComparisonOperator::Equal, 42);
        let result = evaluate_exit_code_predicate(0, &predicate);
        assert_eq!(result.expected, "exit_code == 42");
        assert_eq!(result.actual, "0");
    }

    #[test]
    fn test_not_equal_expected_string_format() {
        let predicate = make_predicate(StringComparisonOperator::NotEqual, 1);
        let result = evaluate_exit_code_predicate(1, &predicate);
        assert_eq!(result.expected, "exit_code != 1");
    }
}
