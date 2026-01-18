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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_file_content_empty() {
        assert_eq!(normalize_file_content(""), "");
    }

    #[test]
    fn test_normalize_file_content_simple() {
        assert_eq!(normalize_file_content("hello"), "hello");
    }

    #[test]
    fn test_normalize_file_content_crlf() {
        assert_eq!(normalize_file_content("line1\r\nline2"), "line1\nline2");
    }

    #[test]
    fn test_normalize_file_content_trailing_whitespace() {
        assert_eq!(normalize_file_content("line1   \nline2  "), "line1\nline2");
    }

    #[test]
    fn test_normalize_file_content_leading_trailing_newlines() {
        assert_eq!(normalize_file_content("\n\nhello\n\n"), "hello");
    }

    #[test]
    fn test_normalize_file_content_mixed_line_endings() {
        assert_eq!(
            normalize_file_content("line1\r\nline2\nline3\r\n"),
            "line1\nline2\nline3"
        );
    }

    #[test]
    fn test_normalize_file_content_only_whitespace() {
        assert_eq!(normalize_file_content("   \n  \n   "), "");
    }

    #[test]
    fn test_normalize_file_content_preserves_internal_spacing() {
        assert_eq!(normalize_file_content("hello   world"), "hello   world");
    }

    // Async tests for file assertion functions
    use crate::parser::ast::QuoteType;

    fn make_string_literal(value: &str) -> StringLiteral {
        StringLiteral {
            value: value.to_string(),
            raw: format!("\"{}\"", value),
            quote_type: QuoteType::Double,
        }
    }

    fn make_regex_literal(pattern: &str) -> RegexLiteral {
        RegexLiteral {
            pattern: pattern.to_string(),
            flags: String::new(),
            raw: format!("/{}/", pattern),
        }
    }

    #[tokio::test]
    async fn test_evaluate_file_exists_nonexistent() {
        let path = make_string_literal("/nonexistent/file/path/12345.txt");
        let result = evaluate_file_predicate(&path, &FilePredicate::Exists, "/tmp").await;
        assert!(!result.passed);
        assert!(result.actual.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_evaluate_file_exists_existing() {
        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_exists.txt");
        tokio::fs::write(&temp_file, "test content").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let result =
            evaluate_file_predicate(&path, &FilePredicate::Exists, temp_dir.to_str().unwrap())
                .await;

        // Cleanup
        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_contains_match() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_contains.txt");
        tokio::fs::write(&temp_file, "hello world").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let search = make_string_literal("world");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Contains { value: search },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_contains_no_match() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_contains_no.txt");
        tokio::fs::write(&temp_file, "hello world").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let search = make_string_literal("goodbye");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Contains { value: search },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(!result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_contains_nonexistent() {
        let path = make_string_literal("nonexistent_file.txt");
        let search = make_string_literal("test");
        let result =
            evaluate_file_predicate(&path, &FilePredicate::Contains { value: search }, "/tmp")
                .await;

        assert!(!result.passed);
        assert!(result.actual.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_evaluate_file_matches_valid_regex() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_matches.txt");
        tokio::fs::write(&temp_file, "line1\nline2\nline3")
            .await
            .unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let regex = make_regex_literal(r"line\d+");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Matches { value: regex },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_matches_invalid_regex() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_invalid_regex.txt");
        tokio::fs::write(&temp_file, "test content").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let regex = RegexLiteral {
            pattern: "[unclosed".to_string(),
            flags: String::new(),
            raw: "/[unclosed/".to_string(),
        };
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Matches { value: regex },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(!result.passed);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Invalid regex"));
    }

    #[tokio::test]
    async fn test_evaluate_file_equals_match() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_equals.txt");
        tokio::fs::write(&temp_file, "exact content").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let expected = make_string_literal("exact content");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Equals {
                operator: StringComparisonOperator::Equal,
                value: expected,
            },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_equals_normalized() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_equals_normalized.txt");
        // File has trailing whitespace and CRLF
        tokio::fs::write(&temp_file, "line1  \r\nline2  ")
            .await
            .unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        // Expected value is normalized
        let expected = make_string_literal("line1\nline2");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Equals {
                operator: StringComparisonOperator::Equal,
                value: expected,
            },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_evaluate_file_not_equals() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("hone_test_not_equals.txt");
        tokio::fs::write(&temp_file, "content A").await.unwrap();

        let path = make_string_literal(temp_file.file_name().unwrap().to_str().unwrap());
        let expected = make_string_literal("content B");
        let result = evaluate_file_predicate(
            &path,
            &FilePredicate::Equals {
                operator: StringComparisonOperator::NotEqual,
                value: expected,
            },
            temp_dir.to_str().unwrap(),
        )
        .await;

        let _ = tokio::fs::remove_file(&temp_file).await;

        assert!(result.passed);
    }
}
