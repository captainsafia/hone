use crate::assertions::AssertionResult;
use crate::parser::ast::{FilePredicate, RegexLiteral, StringComparisonOperator, StringLiteral};
use std::path::{Path, PathBuf};

struct FileExistsResult {
    exists: bool,
    casing_match: bool,
    actual_name: Option<String>,
}

pub async fn evaluate_file_predicate(
    file_path: &StringLiteral,
    predicate: &FilePredicate,
    cwd: &str,
) -> AssertionResult {
    let resolved_path = PathBuf::from(cwd).join(&file_path.value);

    match predicate {
        FilePredicate::Exists => evaluate_file_exists(&resolved_path, &file_path.raw, cwd).await,
        FilePredicate::Contains { value } => {
            evaluate_file_contains(&resolved_path, value, &file_path.raw, cwd).await
        }
        FilePredicate::Matches { value } => {
            evaluate_file_matches(&resolved_path, value, &file_path.raw, cwd).await
        }
        FilePredicate::Equals { operator, value } => {
            evaluate_file_equals(&resolved_path, operator, value, &file_path.raw, cwd).await
        }
    }
}

async fn check_file_exists(file_path: &Path) -> FileExistsResult {
    match tokio::fs::metadata(file_path).await {
        Ok(_) => match tokio::fs::canonicalize(file_path).await {
            Ok(real_path) => {
                let expected_name = file_path.file_name().and_then(|s| s.to_str());
                let actual_name = real_path.file_name().and_then(|s| s.to_str());

                if expected_name != actual_name {
                    FileExistsResult {
                        exists: true,
                        casing_match: false,
                        actual_name: actual_name.map(String::from),
                    }
                } else {
                    FileExistsResult {
                        exists: true,
                        casing_match: true,
                        actual_name: None,
                    }
                }
            }
            Err(_) => FileExistsResult {
                exists: true,
                casing_match: true,
                actual_name: None,
            },
        },
        Err(_) => FileExistsResult {
            exists: false,
            casing_match: true,
            actual_name: None,
        },
    }
}

async fn evaluate_file_exists(file_path: &Path, path_raw: &str, _cwd: &str) -> AssertionResult {
    let result = check_file_exists(file_path).await;

    if result.exists && !result.casing_match {
        let actual = result.actual_name.as_deref().unwrap_or("");
        return AssertionResult::with_error(
            false,
            format!("file {} to exist", path_raw),
            format!("file exists but with different casing: \"{}\"", actual),
            format!(
                "Case mismatch: expected \"{}\" but found \"{}\"",
                file_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                actual
            ),
        );
    }

    AssertionResult::new(
        result.exists,
        format!("file {} to exist", path_raw),
        if result.exists {
            "file exists".to_string()
        } else {
            "file does not exist".to_string()
        },
    )
}

async fn read_file_content(
    file_path: &Path,
    path_raw: &str,
    _cwd: &str,
) -> (String, Option<AssertionResult>) {
    let result = check_file_exists(file_path).await;

    if !result.exists {
        return (
            String::new(),
            Some(AssertionResult::new(
                false,
                format!("file {} to exist", path_raw),
                "file does not exist".to_string(),
            )),
        );
    }

    if !result.casing_match {
        let actual = result.actual_name.as_deref().unwrap_or("");
        return (
            String::new(),
            Some(AssertionResult::with_error(
                false,
                format!("file {} to exist with exact casing", path_raw),
                format!("file exists but with different casing: \"{}\"", actual),
                format!(
                    "Case mismatch: expected \"{}\" but found \"{}\"",
                    file_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                    actual
                ),
            )),
        );
    }

    match tokio::fs::read_to_string(file_path).await {
        Ok(content) => (content, None),
        Err(e) => (
            String::new(),
            Some(AssertionResult::new(
                false,
                format!("to read file {}", path_raw),
                format!("failed to read file: {}", e),
            )),
        ),
    }
}

async fn evaluate_file_contains(
    file_path: &Path,
    value: &StringLiteral,
    path_raw: &str,
    cwd: &str,
) -> AssertionResult {
    let (content, error) = read_file_content(file_path, path_raw, cwd).await;
    if let Some(err) = error {
        return err;
    }

    let passed = content.contains(&value.value);
    AssertionResult::new(
        passed,
        format!("file {} to contain {}", path_raw, value.raw),
        content,
    )
}

async fn evaluate_file_matches(
    file_path: &Path,
    value: &RegexLiteral,
    path_raw: &str,
    cwd: &str,
) -> AssertionResult {
    let (content, error) = read_file_content(file_path, path_raw, cwd).await;
    if let Some(err) = error {
        return err;
    }

    let pattern = if value.flags.is_empty() {
        value.pattern.clone()
    } else {
        format!("(?{}){}", value.flags, value.pattern)
    };

    match regex::Regex::new(&pattern) {
        Ok(re) => {
            let passed = re.is_match(&content);
            AssertionResult::new(
                passed,
                format!("file {} to match {}", path_raw, value.raw),
                content,
            )
        }
        Err(e) => AssertionResult::with_error(
            false,
            format!("file {} to match {}", path_raw, value.raw),
            content,
            format!("Invalid regex: {}", e),
        ),
    }
}

fn normalize_file_content(content: &str) -> String {
    content
        .replace("\r\n", "\n")
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

async fn evaluate_file_equals(
    file_path: &Path,
    operator: &StringComparisonOperator,
    value: &StringLiteral,
    path_raw: &str,
    cwd: &str,
) -> AssertionResult {
    let (content, error) = read_file_content(file_path, path_raw, cwd).await;
    if let Some(err) = error {
        return err;
    }

    let normalized_content = normalize_file_content(&content);
    let normalized_value = normalize_file_content(&value.value);

    let is_equal = normalized_content == normalized_value;
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
        format!("file {} {} {}", path_raw, op_str, value.raw),
        content,
    )
}
