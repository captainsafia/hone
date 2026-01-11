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
