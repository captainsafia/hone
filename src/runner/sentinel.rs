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
    let sentinel_index = buffer.find(SENTINEL_PREFIX);

    if sentinel_index.is_none() {
        return SentinelExtractResult {
            found: false,
            output: buffer.to_string(),
            sentinel: None,
            remaining: String::new(),
        };
    }

    let sentinel_index = sentinel_index.unwrap();

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

    if parsed.is_none() || parsed.as_ref().unwrap().run_id != expected_run_id {
        return SentinelExtractResult {
            found: false,
            output: buffer.to_string(),
            sentinel: None,
            remaining: String::new(),
        };
    }

    let clean_output = output.strip_suffix('\n').unwrap_or(output);

    SentinelExtractResult {
        found: true,
        output: clean_output.to_string(),
        sentinel: parsed,
        remaining: remaining.to_string(),
    }
}
