use std::path::Path;

const UNIT_SEPARATOR: char = '\x1f';
const SENTINEL_PREFIX: &str = "__HONE__";

#[derive(Debug, Clone, PartialEq)]
pub struct SentinelData {
    pub run_id: String,
    pub exit_code: i32,
    pub end_timestamp_ms: u64,
}

pub fn generate_run_id(
    filename: &str,
    test_name: Option<&str>,
    run_name: Option<&str>,
    run_index: usize,
) -> String {
    let mut parts = Vec::new();

    let base = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    parts.push(base.to_string());

    if let Some(test) = test_name {
        let sanitized = test.replace(char::is_whitespace, "-").to_lowercase();
        parts.push(sanitized);
    }

    if let Some(run) = run_name {
        parts.push(run.to_string());
    } else {
        parts.push(run_index.to_string());
    }

    parts.join("-")
}

pub fn generate_shell_wrapper(command: &str, run_id: &str, stderr_path: &str) -> String {
    let escaped_stderr_path = stderr_path.replace('\'', "'\"'\"'");

    // Shell wrapper uses command grouping {...} to preserve shell state
    // (working directory, variables, etc.) across commands.
    // Note: Commands that would exit the shell (like bare `exit`) should
    // be wrapped in a subshell by the test: (exit 42) instead of exit 42
    [
        format!(": > '{}'", escaped_stderr_path),
        format!("{{ {} ; }} 2> '{}'", command, escaped_stderr_path),
        "HONE_EC=$?".to_string(),
        format!(
            "printf \"{}{}{}{}%d{}%s\\n\" \"$HONE_EC\" \"$(date +%s%3N)\"",
            SENTINEL_PREFIX, UNIT_SEPARATOR, run_id, UNIT_SEPARATOR, UNIT_SEPARATOR
        ),
    ]
    .join("\n")
}

pub fn parse_sentinel(line: &str) -> Option<SentinelData> {
    // Expected format: __HONE__<US><RUN_ID><US><EXIT_CODE><US><END_TS_MS>
    if !line.starts_with(SENTINEL_PREFIX) {
        return None;
    }

    let parts: Vec<&str> = line.split(UNIT_SEPARATOR).collect();

    if parts.len() != 4 {
        return None;
    }

    let run_id = parts[1];
    let exit_code_str = parts[2];
    let timestamp_str = parts[3];

    if run_id.is_empty() || exit_code_str.is_empty() || timestamp_str.is_empty() {
        return None;
    }

    let exit_code = exit_code_str.parse::<i32>().ok()?;
    let end_timestamp_ms = timestamp_str.parse::<u64>().ok()?;

    Some(SentinelData {
        run_id: run_id.to_string(),
        exit_code,
        end_timestamp_ms,
    })
}

pub fn contains_sentinel(line: &str) -> bool {
    line.contains(SENTINEL_PREFIX)
}

#[derive(Debug)]
pub struct SentinelExtractResult {
    pub found: bool,
    pub output: String,
    pub sentinel: Option<SentinelData>,
    pub remaining: String,
}

pub fn extract_sentinel(buffer: &str, expected_run_id: &str) -> SentinelExtractResult {
    // Sentinel might be on the same line as output if command didn't output a trailing newline
    let Some(sentinel_index) = buffer.find(SENTINEL_PREFIX) else {
        return SentinelExtractResult {
            found: false,
            output: buffer.to_string(),
            sentinel: None,
            remaining: String::new(),
        };
    };

    let output = &buffer[..sentinel_index];

    let after_sentinel = &buffer[sentinel_index..];
    let newline_index = after_sentinel.find('\n');

    let (sentinel_line, remaining) = if let Some(newline_index) = newline_index {
        (
            &after_sentinel[..newline_index],
            &after_sentinel[newline_index + 1..],
        )
    } else {
        return SentinelExtractResult {
            found: false,
            output: buffer.to_string(),
            sentinel: None,
            remaining: String::new(),
        };
    };

    let parsed = parse_sentinel(sentinel_line.trim());

    let parsed = match parsed {
        Some(p) if p.run_id == expected_run_id => p,
        _ => {
            return SentinelExtractResult {
                found: false,
                output: buffer.to_string(),
                sentinel: None,
                remaining: String::new(),
            };
        }
    };

    let clean_output = output.strip_suffix('\n').unwrap_or(output);

    SentinelExtractResult {
        found: true,
        output: clean_output.to_string(),
        sentinel: Some(parsed),
        remaining: remaining.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sentinel_valid() {
        let line = format!(
            "__HONE__{}test-run{}0{}1234567890",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);

        assert!(result.is_some());
        let sentinel = result.unwrap();
        assert_eq!(sentinel.run_id, "test-run");
        assert_eq!(sentinel.exit_code, 0);
        assert_eq!(sentinel.end_timestamp_ms, 1234567890);
    }

    #[test]
    fn test_parse_sentinel_non_zero_exit() {
        let line = format!(
            "__HONE__{}my-test{}127{}9876543210",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);

        assert!(result.is_some());
        let sentinel = result.unwrap();
        assert_eq!(sentinel.exit_code, 127);
    }

    #[test]
    fn test_parse_sentinel_negative_exit_code() {
        let line = format!(
            "__HONE__{}test{}-1{}1000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);

        assert!(result.is_some());
        let sentinel = result.unwrap();
        assert_eq!(sentinel.exit_code, -1);
    }

    #[test]
    fn test_parse_sentinel_missing_prefix() {
        let line = format!(
            "NOTHONE{}test{}0{}1234",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_too_few_fields() {
        let line = format!("__HONE__{}test{}0", UNIT_SEPARATOR, UNIT_SEPARATOR);
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_too_many_fields() {
        let line = format!(
            "__HONE__{}test{}0{}1234{}extra",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_empty_run_id() {
        let line = format!(
            "__HONE__{}{}0{}1234",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_empty_exit_code() {
        let line = format!(
            "__HONE__{}test{}{}1234",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_empty_timestamp() {
        let line = format!(
            "__HONE__{}test{}0{}",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_invalid_exit_code() {
        let line = format!(
            "__HONE__{}test{}abc{}1234",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_invalid_timestamp() {
        let line = format!(
            "__HONE__{}test{}0{}not-a-number",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sentinel_exit_code_overflow() {
        let line = format!(
            "__HONE__{}test{}999999999999999999999{}1234",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let result = parse_sentinel(&line);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_sentinel_simple() {
        let sentinel_line = format!(
            "__HONE__{}test-1{}0{}1000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("command output\n{}\nremaining", sentinel_line);

        let result = extract_sentinel(&buffer, "test-1");

        assert!(result.found);
        assert_eq!(result.output, "command output");
        assert!(result.sentinel.is_some());
        assert_eq!(result.sentinel.unwrap().run_id, "test-1");
        assert_eq!(result.remaining, "remaining");
    }

    #[test]
    fn test_extract_sentinel_no_preceding_output() {
        let sentinel_line = format!(
            "__HONE__{}run-2{}0{}2000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("{}\nafter", sentinel_line);

        let result = extract_sentinel(&buffer, "run-2");

        assert!(result.found);
        assert_eq!(result.output, "");
        assert_eq!(result.remaining, "after");
    }

    #[test]
    fn test_extract_sentinel_output_no_trailing_newline() {
        let sentinel_line = format!(
            "__HONE__{}test{}0{}3000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("output{}\n", sentinel_line);

        let result = extract_sentinel(&buffer, "test");

        assert!(result.found);
        assert_eq!(result.output, "output");
        assert_eq!(result.remaining, "");
    }

    #[test]
    fn test_extract_sentinel_wrong_run_id() {
        let sentinel_line = format!(
            "__HONE__{}wrong-id{}0{}4000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("output\n{}\n", sentinel_line);

        let result = extract_sentinel(&buffer, "expected-id");

        assert!(!result.found);
        assert_eq!(result.output, buffer);
        assert!(result.sentinel.is_none());
    }

    #[test]
    fn test_extract_sentinel_not_found() {
        let buffer = "output without sentinel\nmore output\n";

        let result = extract_sentinel(buffer, "test");

        assert!(!result.found);
        assert_eq!(result.output, buffer);
        assert!(result.sentinel.is_none());
        assert_eq!(result.remaining, "");
    }

    #[test]
    fn test_extract_sentinel_incomplete_no_newline() {
        let sentinel_line = format!(
            "__HONE__{}test{}0{}5000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("output\n{}", sentinel_line);

        let result = extract_sentinel(&buffer, "test");

        assert!(!result.found);
    }

    #[test]
    fn test_extract_sentinel_malformed() {
        let buffer = "output\n__HONE__malformed\n";

        let result = extract_sentinel(buffer, "test");

        assert!(!result.found);
    }

    #[test]
    fn test_extract_sentinel_multiline_output() {
        let sentinel_line = format!(
            "__HONE__{}test{}0{}6000",
            UNIT_SEPARATOR, UNIT_SEPARATOR, UNIT_SEPARATOR
        );
        let buffer = format!("line1\nline2\nline3\n{}\nafter", sentinel_line);

        let result = extract_sentinel(&buffer, "test");

        assert!(result.found);
        assert_eq!(result.output, "line1\nline2\nline3");
        assert_eq!(result.remaining, "after");
    }

    #[test]
    fn test_contains_sentinel_present() {
        let line = "__HONE__some data";
        assert!(contains_sentinel(line));
    }

    #[test]
    fn test_contains_sentinel_absent() {
        let line = "normal output";
        assert!(!contains_sentinel(line));
    }

    #[test]
    fn test_generate_run_id_simple() {
        let id = generate_run_id("test.hone", None, None, 0);
        assert_eq!(id, "test-0");
    }

    #[test]
    fn test_generate_run_id_with_test_name() {
        let id = generate_run_id("file.hone", Some("My Test"), None, 1);
        assert_eq!(id, "file-my-test-1");
    }

    #[test]
    fn test_generate_run_id_with_named_run() {
        let id = generate_run_id("test.hone", Some("test"), Some("setup"), 0);
        assert_eq!(id, "test-test-setup");
    }

    #[test]
    fn test_generate_run_id_whitespace_sanitization() {
        let id = generate_run_id("f.hone", Some("Test  With   Spaces"), None, 0);
        assert_eq!(id, "f-test--with---spaces-0");
    }
}
