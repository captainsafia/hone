pub mod exitcode;
pub mod filesystem;
pub mod output;
pub mod timing;

#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    pub error: Option<String>,
}

impl AssertionResult {
    pub fn new(passed: bool, expected: String, actual: String) -> Self {
        Self {
            passed,
            expected,
            actual,
            error: None,
        }
    }

    pub fn with_error(passed: bool, expected: String, actual: String, error: String) -> Self {
        Self {
            passed,
            expected,
            actual,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_result_new_passed() {
        let result = AssertionResult::new(true, "expected".to_string(), "actual".to_string());
        assert!(result.passed);
        assert_eq!(result.expected, "expected");
        assert_eq!(result.actual, "actual");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_assertion_result_new_failed() {
        let result = AssertionResult::new(false, "expected".to_string(), "actual".to_string());
        assert!(!result.passed);
        assert_eq!(result.expected, "expected");
        assert_eq!(result.actual, "actual");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_assertion_result_with_error() {
        let result = AssertionResult::with_error(
            false,
            "expected".to_string(),
            "actual".to_string(),
            "error message".to_string(),
        );
        assert!(!result.passed);
        assert_eq!(result.expected, "expected");
        assert_eq!(result.actual, "actual");
        assert_eq!(result.error, Some("error message".to_string()));
    }

    #[test]
    fn test_assertion_result_with_error_passed() {
        // Edge case: error can technically be set even when passed is true
        // (though this shouldn't happen in practice)
        let result = AssertionResult::with_error(
            true,
            "expected".to_string(),
            "actual".to_string(),
            "warning".to_string(),
        );
        assert!(result.passed);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_assertion_result_empty_strings() {
        let result = AssertionResult::new(true, String::new(), String::new());
        assert!(result.passed);
        assert_eq!(result.expected, "");
        assert_eq!(result.actual, "");
    }

    #[test]
    fn test_assertion_result_clone() {
        let original = AssertionResult::with_error(
            false,
            "exp".to_string(),
            "act".to_string(),
            "err".to_string(),
        );
        let cloned = original.clone();
        assert_eq!(original.passed, cloned.passed);
        assert_eq!(original.expected, cloned.expected);
        assert_eq!(original.actual, cloned.actual);
        assert_eq!(original.error, cloned.error);
    }
}
