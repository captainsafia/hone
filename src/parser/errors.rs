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
