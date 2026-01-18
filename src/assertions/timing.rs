use crate::assertions::AssertionResult;
use crate::parser::ast::{ComparisonOperator, Duration, DurationPredicate, DurationUnit};

pub fn duration_to_ms(duration: &Duration) -> f64 {
    match duration.unit {
        DurationUnit::Seconds => duration.value * 1000.0,
        DurationUnit::Milliseconds => duration.value,
    }
}

pub fn format_duration(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else {
        format!("{}ms", ms as u64)
    }
}

pub fn evaluate_duration_predicate(
    duration_ms: u64,
    predicate: &DurationPredicate,
) -> AssertionResult {
    let expected_ms = duration_to_ms(&predicate.value);
    let passed = evaluate_comparison(duration_ms as f64, &predicate.operator, expected_ms);

    let op_str = match predicate.operator {
        ComparisonOperator::Equal => "==",
        ComparisonOperator::NotEqual => "!=",
        ComparisonOperator::LessThan => "<",
        ComparisonOperator::LessThanOrEqual => "<=",
        ComparisonOperator::GreaterThan => ">",
        ComparisonOperator::GreaterThanOrEqual => ">=",
    };

    AssertionResult::new(
        passed,
        format!("duration {} {}", op_str, predicate.value.raw),
        format_duration(duration_ms as f64),
    )
}

fn evaluate_comparison(actual: f64, operator: &ComparisonOperator, expected: f64) -> bool {
    match operator {
        ComparisonOperator::Equal => actual == expected,
        ComparisonOperator::NotEqual => actual != expected,
        ComparisonOperator::LessThan => actual < expected,
        ComparisonOperator::LessThanOrEqual => actual <= expected,
        ComparisonOperator::GreaterThan => actual > expected,
        ComparisonOperator::GreaterThanOrEqual => actual >= expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_duration(value: f64, unit: DurationUnit) -> Duration {
        let raw = match unit {
            DurationUnit::Seconds => format!("{}s", value),
            DurationUnit::Milliseconds => format!("{}ms", value),
        };
        Duration { value, unit, raw }
    }

    fn make_predicate(op: ComparisonOperator, value: f64, unit: DurationUnit) -> DurationPredicate {
        DurationPredicate {
            operator: op,
            value: make_duration(value, unit),
        }
    }

    #[test]
    fn test_duration_to_ms_seconds() {
        let dur = make_duration(2.5, DurationUnit::Seconds);
        assert_eq!(duration_to_ms(&dur), 2500.0);
    }

    #[test]
    fn test_duration_to_ms_milliseconds() {
        let dur = make_duration(150.0, DurationUnit::Milliseconds);
        assert_eq!(duration_to_ms(&dur), 150.0);
    }

    #[test]
    fn test_duration_to_ms_zero() {
        let dur = make_duration(0.0, DurationUnit::Seconds);
        assert_eq!(duration_to_ms(&dur), 0.0);
    }

    #[test]
    fn test_format_duration_milliseconds() {
        assert_eq!(format_duration(500.0), "500ms");
        assert_eq!(format_duration(0.0), "0ms");
        assert_eq!(format_duration(999.0), "999ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1000.0), "1.00s");
        assert_eq!(format_duration(1500.0), "1.50s");
        assert_eq!(format_duration(2000.0), "2.00s");
    }

    #[test]
    fn test_less_than_pass() {
        let predicate = make_predicate(
            ComparisonOperator::LessThan,
            200.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(100, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_less_than_fail() {
        let predicate = make_predicate(
            ComparisonOperator::LessThan,
            100.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(100, &predicate);
        assert!(!result.passed);
    }

    #[test]
    fn test_less_than_or_equal_boundary() {
        let predicate = make_predicate(
            ComparisonOperator::LessThanOrEqual,
            100.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(100, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_greater_than_pass() {
        let predicate = make_predicate(
            ComparisonOperator::GreaterThan,
            100.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(200, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_greater_than_or_equal_boundary() {
        let predicate = make_predicate(
            ComparisonOperator::GreaterThanOrEqual,
            100.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(100, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_equal_pass() {
        let predicate = make_predicate(ComparisonOperator::Equal, 1.0, DurationUnit::Seconds);
        let result = evaluate_duration_predicate(1000, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_not_equal_pass() {
        let predicate = make_predicate(ComparisonOperator::NotEqual, 1.0, DurationUnit::Seconds);
        let result = evaluate_duration_predicate(500, &predicate);
        assert!(result.passed);
    }

    #[test]
    fn test_expected_string_format() {
        let predicate = make_predicate(
            ComparisonOperator::LessThan,
            500.0,
            DurationUnit::Milliseconds,
        );
        let result = evaluate_duration_predicate(100, &predicate);
        assert_eq!(result.expected, "duration < 500ms");
        assert_eq!(result.actual, "100ms");
    }

    #[test]
    fn test_actual_formatted_as_seconds() {
        let predicate = make_predicate(ComparisonOperator::LessThan, 5.0, DurationUnit::Seconds);
        let result = evaluate_duration_predicate(2500, &predicate);
        assert_eq!(result.actual, "2.50s");
    }
}
