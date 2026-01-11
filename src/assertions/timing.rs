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
