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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{QuoteType, RegexLiteral, StringLiteral};
    use crate::runner::shell::RunResult;

    #[test]
    fn test_normalize_whitespace_empty() {
        assert_eq!(normalize_whitespace(""), "");
    }

    #[test]
    fn test_normalize_whitespace_only_whitespace() {
        assert_eq!(normalize_whitespace("   \n  \n  "), "");
    }

    #[test]
    fn test_normalize_whitespace_simple() {
        assert_eq!(normalize_whitespace("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_whitespace_trailing_spaces() {
        assert_eq!(normalize_whitespace("line1  \nline2  "), "line1\nline2");
    }

    #[test]
    fn test_normalize_whitespace_leading_trailing() {
        assert_eq!(
            normalize_whitespace("  hello  \n  world  "),
            "hello\n  world"
        );
    }

    #[test]
    fn test_normalize_whitespace_crlf() {
        assert_eq!(normalize_whitespace("line1\r\nline2\r\n"), "line1\nline2");
    }

    #[test]
    fn test_normalize_whitespace_mixed_line_endings() {
        assert_eq!(
            normalize_whitespace("line1\r\nline2\nline3"),
            "line1\nline2\nline3"
        );
    }

    #[test]
    fn test_evaluate_contains_match() {
        let value = StringLiteral {
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_contains("hello world", &value);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_contains_no_match() {
        let value = StringLiteral {
            value: "goodbye".to_string(),
            raw: "\"goodbye\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_contains("hello world", &value);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_contains_empty_output() {
        let value = StringLiteral {
            value: "test".to_string(),
            raw: "\"test\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_contains("", &value);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_equals_match() {
        let value = StringLiteral {
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_equals("hello", &StringComparisonOperator::Equal, &value);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_equals_no_match() {
        let value = StringLiteral {
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_equals("goodbye", &StringComparisonOperator::Equal, &value);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_equals_not_equal_match() {
        let value = StringLiteral {
            value: "hello".to_string(),
            raw: "\"hello\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_equals("goodbye", &StringComparisonOperator::NotEqual, &value);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_equals_whitespace_normalization() {
        let value = StringLiteral {
            value: "line1\nline2".to_string(),
            raw: "\"line1\\nline2\"".to_string(),
            quote_type: QuoteType::Double,
        };
        let result = evaluate_equals(
            "line1  \r\nline2  ",
            &StringComparisonOperator::Equal,
            &value,
        );
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_matches_simple() {
        let value = RegexLiteral {
            pattern: "hello.*".to_string(),
            flags: "".to_string(),
            raw: "/hello.*/".to_string(),
        };
        let result = evaluate_matches("hello world", &value);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_matches_no_match() {
        let value = RegexLiteral {
            pattern: "^goodbye".to_string(),
            flags: "".to_string(),
            raw: "/^goodbye/".to_string(),
        };
        let result = evaluate_matches("hello world", &value);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_matches_case_insensitive() {
        let value = RegexLiteral {
            pattern: "HELLO".to_string(),
            flags: "i".to_string(),
            raw: "/HELLO/i".to_string(),
        };
        let result = evaluate_matches("hello world", &value);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_matches_invalid_regex() {
        let value = RegexLiteral {
            pattern: "[unclosed".to_string(),
            flags: "".to_string(),
            raw: "/[unclosed/".to_string(),
        };
        let result = evaluate_matches("test", &value);
        assert!(!result.passed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Invalid regex"));
    }

    #[test]
    fn test_evaluate_matches_multiline_flag() {
        let value = RegexLiteral {
            pattern: "^line2".to_string(),
            flags: "m".to_string(),
            raw: "/^line2/m".to_string(),
        };
        let result = evaluate_matches("line1\nline2", &value);
        assert!(result.passed);
    }

    #[test]
    fn test_get_output_value_stdout() {
        let run_result = RunResult {
            run_id: "test".to_string(),
            stdout: "stdout_data".to_string(),
            stdout_raw: "raw_data".to_string(),
            stderr: "stderr_data".to_string(),
            exit_code: 0,
            duration_ms: 100,
            stderr_path: "/tmp/stderr".to_string(),
        };
        assert_eq!(
            get_output_value(&run_result, &OutputSelector::Stdout),
            "stdout_data"
        );
    }

    #[test]
    fn test_get_output_value_stdout_raw() {
        let run_result = RunResult {
            run_id: "test".to_string(),
            stdout: "stdout_data".to_string(),
            stdout_raw: "raw_data".to_string(),
            stderr: "stderr_data".to_string(),
            exit_code: 0,
            duration_ms: 100,
            stderr_path: "/tmp/stderr".to_string(),
        };
        assert_eq!(
            get_output_value(&run_result, &OutputSelector::StdoutRaw),
            "raw_data"
        );
    }

    #[test]
    fn test_get_output_value_stderr() {
        let run_result = RunResult {
            run_id: "test".to_string(),
            stdout: "stdout_data".to_string(),
            stdout_raw: "raw_data".to_string(),
            stderr: "stderr_data".to_string(),
            exit_code: 0,
            duration_ms: 100,
            stderr_path: "/tmp/stderr".to_string(),
        };
        assert_eq!(
            get_output_value(&run_result, &OutputSelector::Stderr),
            "stderr_data"
        );
    }
}
