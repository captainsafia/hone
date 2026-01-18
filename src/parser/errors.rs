use crate::parser::ast::{ParseErrorDetail, ParseWarning};

pub struct ParseErrorCollector {
    errors: Vec<ParseErrorDetail>,
    warnings: Vec<ParseWarning>,
    filename: String,
}

impl ParseErrorCollector {
    pub fn new(filename: String) -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            filename,
        }
    }

    pub fn add_error(&mut self, message: String, line: usize) {
        self.errors.push(ParseErrorDetail {
            message,
            line,
            filename: self.filename.clone(),
        });
    }

    pub fn add_warning(&mut self, message: String, line: usize) {
        self.warnings.push(ParseWarning {
            message,
            line,
            filename: self.filename.clone(),
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn get_errors(&self) -> Vec<ParseErrorDetail> {
        self.errors.clone()
    }

    pub fn get_warnings(&self) -> Vec<ParseWarning> {
        self.warnings.clone()
    }

    pub fn format_error(error: &ParseErrorDetail) -> String {
        format!("{}:{} :: {}", error.filename, error.line, error.message)
    }

    pub fn format_warning(warning: &ParseWarning) -> String {
        format!(
            "{}:{} :: Warning: {}",
            warning.filename, warning.line, warning.message
        )
    }
}

// Legacy error type for compatibility
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Parse error: {0}")]
    Generic(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_collector() {
        let collector = ParseErrorCollector::new("test.hone".to_string());
        assert!(!collector.has_errors());
        assert!(collector.get_errors().is_empty());
        assert!(collector.get_warnings().is_empty());
    }

    #[test]
    fn test_add_error() {
        let mut collector = ParseErrorCollector::new("test.hone".to_string());
        collector.add_error("Syntax error".to_string(), 5);

        assert!(collector.has_errors());
        let errors = collector.get_errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Syntax error");
        assert_eq!(errors[0].line, 5);
        assert_eq!(errors[0].filename, "test.hone");
    }

    #[test]
    fn test_add_multiple_errors() {
        let mut collector = ParseErrorCollector::new("file.hone".to_string());
        collector.add_error("Error 1".to_string(), 1);
        collector.add_error("Error 2".to_string(), 3);

        let errors = collector.get_errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 1);
        assert_eq!(errors[1].line, 3);
    }

    #[test]
    fn test_add_warning() {
        let mut collector = ParseErrorCollector::new("test.hone".to_string());
        collector.add_warning("Deprecated syntax".to_string(), 10);

        assert!(!collector.has_errors());
        let warnings = collector.get_warnings();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, "Deprecated syntax");
        assert_eq!(warnings[0].line, 10);
        assert_eq!(warnings[0].filename, "test.hone");
    }

    #[test]
    fn test_format_error() {
        let error = ParseErrorDetail {
            message: "Unknown token".to_string(),
            line: 42,
            filename: "example.hone".to_string(),
        };
        let formatted = ParseErrorCollector::format_error(&error);
        assert_eq!(formatted, "example.hone:42 :: Unknown token");
    }

    #[test]
    fn test_format_warning() {
        let warning = ParseWarning {
            message: "Unused variable".to_string(),
            line: 7,
            filename: "script.hone".to_string(),
        };
        let formatted = ParseErrorCollector::format_warning(&warning);
        assert_eq!(formatted, "script.hone:7 :: Warning: Unused variable");
    }

    #[test]
    fn test_errors_and_warnings_independent() {
        let mut collector = ParseErrorCollector::new("test.hone".to_string());
        collector.add_error("Error".to_string(), 1);
        collector.add_warning("Warning".to_string(), 2);

        assert!(collector.has_errors());
        assert_eq!(collector.get_errors().len(), 1);
        assert_eq!(collector.get_warnings().len(), 1);
    }
}
