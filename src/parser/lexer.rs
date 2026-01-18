use crate::parser::ast::{
    ComparisonOperator, Duration, DurationUnit, QuoteType, RegexLiteral, StringLiteral,
};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Pragma,
    Comment,
    Test,
    Run,
    Assert,
    Env,
    Empty,
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub content: String,
    pub line: usize,
}

pub fn classify_line(line: &str, line_number: usize) -> Token {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        return Token {
            token_type: TokenType::Empty,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with("#!") {
        return Token {
            token_type: TokenType::Pragma,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with('#') {
        return Token {
            token_type: TokenType::Comment,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with("TEST ") {
        return Token {
            token_type: TokenType::Test,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with("RUN ") {
        return Token {
            token_type: TokenType::Run,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with("ASSERT ") {
        return Token {
            token_type: TokenType::Assert,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    if trimmed.starts_with("ENV ") {
        return Token {
            token_type: TokenType::Env,
            content: trimmed.to_string(),
            line: line_number,
        };
    }

    Token {
        token_type: TokenType::Unknown,
        content: trimmed.to_string(),
        line: line_number,
    }
}

pub fn parse_string_literal(
    input: &str,
    start_byte_index: usize,
) -> Option<(StringLiteral, usize)> {
    let remaining = input.get(start_byte_index..)?;
    let mut chars = remaining.char_indices();

    let (_, start_char) = chars.next()?;
    if start_char != '"' && start_char != '\'' {
        return None;
    }

    let quote_type = if start_char == '"' {
        QuoteType::Double
    } else {
        QuoteType::Single
    };

    let mut value = String::new();
    let mut escaped = false;

    for (byte_offset, ch) in chars {
        if escaped {
            if quote_type == QuoteType::Double {
                // Handle escape sequences in double-quoted strings
                match ch {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    '"' => value.push('"'),
                    '\\' => value.push('\\'),
                    _ => {
                        // Unknown escape, keep as-is
                        value.push('\\');
                        value.push(ch);
                    }
                }
            } else {
                // Single quotes: no escape sequences, literal backslash
                value.push('\\');
                value.push(ch);
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == start_char {
            // End of string - byte_offset is relative to remaining, add start_byte_index
            let end_byte_index = start_byte_index + byte_offset + ch.len_utf8();
            let raw = input[start_byte_index..end_byte_index].to_string();
            return Some((
                StringLiteral {
                    value,
                    raw,
                    quote_type,
                },
                end_byte_index,
            ));
        } else {
            value.push(ch);
        }
    }

    None
}

pub fn parse_regex_literal(input: &str, start_byte_index: usize) -> Option<(RegexLiteral, usize)> {
    let remaining = input.get(start_byte_index..)?;
    let mut chars = remaining.char_indices();

    let (_, first_char) = chars.next()?;
    if first_char != '/' {
        return None;
    }

    let mut pattern = String::new();
    let mut escaped = false;

    for (byte_offset, ch) in chars {
        if escaped {
            pattern.push(ch);
            escaped = false;
        } else if ch == '\\' {
            pattern.push(ch);
            escaped = true;
        } else if ch == '/' {
            // End of pattern, parse flags
            let mut flags = String::new();
            let flags_start = start_byte_index + byte_offset + ch.len_utf8();
            let flags_remaining = input.get(flags_start..)?;

            for flag_ch in flags_remaining.chars() {
                // Only accept flags supported by Rust's regex crate:
                // i: case-insensitive, m: multi-line, s: dotall, u: unicode, x: verbose
                // Note: 'g' (global) and 'y' (sticky) are JavaScript-specific and not supported
                if matches!(flag_ch, 'i' | 'm' | 's' | 'u' | 'x') {
                    flags.push(flag_ch);
                } else {
                    break;
                }
            }

            let end_byte_index = flags_start + flags.len();
            let raw = input[start_byte_index..end_byte_index].to_string();
            return Some((
                RegexLiteral {
                    pattern,
                    flags,
                    raw,
                },
                end_byte_index,
            ));
        } else {
            pattern.push(ch);
        }
    }

    None
}

pub fn parse_duration(input: &str, start_byte_index: usize) -> Option<(Duration, usize)> {
    let remaining = input.get(start_byte_index..)?;
    let mut byte_offset = 0;

    // Skip whitespace
    for ch in remaining.chars() {
        if ch != ' ' {
            break;
        }
        byte_offset += ch.len_utf8();
    }

    let num_start = byte_offset;

    // Parse number (including decimal)
    for ch in remaining[byte_offset..].chars() {
        if !ch.is_ascii_digit() && ch != '.' {
            break;
        }
        byte_offset += ch.len_utf8();
    }

    if byte_offset == num_start {
        return None;
    }

    let num_str = &remaining[num_start..byte_offset];
    let value = num_str.parse::<f64>().ok()?;

    // Reject non-finite values (infinity, NaN)
    if !value.is_finite() {
        return None;
    }

    // Parse unit
    let unit_start = byte_offset;
    for ch in remaining[byte_offset..].chars() {
        if !ch.is_ascii_lowercase() {
            break;
        }
        byte_offset += ch.len_utf8();
    }

    let unit_str = &remaining[unit_start..byte_offset];
    let unit = match unit_str {
        "ms" => DurationUnit::Milliseconds,
        "s" => DurationUnit::Seconds,
        _ => return None,
    };

    // Validate duration is reasonable (max ~1 year) to prevent overflow/confusion
    // Check the value in milliseconds for consistent validation
    const MAX_DURATION_MS: f64 = 365.0 * 24.0 * 60.0 * 60.0 * 1000.0; // ~31.5 billion ms
    let value_ms = match unit {
        DurationUnit::Seconds => value * 1000.0,
        DurationUnit::Milliseconds => value,
    };
    if value_ms > MAX_DURATION_MS {
        return None;
    }

    let raw = remaining[..byte_offset].trim().to_string();

    Some((
        Duration { value, unit, raw },
        start_byte_index + byte_offset,
    ))
}

#[derive(Debug, PartialEq)]
pub enum ParseNumberResult {
    Success(i32, usize),
    Overflow,
    NotANumber,
}

pub fn parse_number_checked(input: &str, start_byte_index: usize) -> ParseNumberResult {
    let Some(remaining) = input.get(start_byte_index..) else {
        return ParseNumberResult::NotANumber;
    };
    let mut byte_offset = 0;

    // Skip whitespace
    for ch in remaining.chars() {
        if ch != ' ' {
            break;
        }
        byte_offset += ch.len_utf8();
    }

    let num_start = byte_offset;
    let after_ws = &remaining[byte_offset..];

    // Handle negative numbers
    let mut chars_iter = after_ws.chars().peekable();
    if chars_iter.peek() == Some(&'-') {
        byte_offset += 1;
        chars_iter.next();
    }

    // Parse digits
    for ch in chars_iter {
        if !ch.is_ascii_digit() {
            break;
        }
        byte_offset += ch.len_utf8();
    }

    if byte_offset == num_start
        || (byte_offset == num_start + 1 && remaining.as_bytes().get(num_start) == Some(&b'-'))
    {
        return ParseNumberResult::NotANumber;
    }

    let num_str = &remaining[num_start..byte_offset];
    match num_str.parse::<i32>() {
        Ok(value) => ParseNumberResult::Success(value, start_byte_index + byte_offset),
        Err(_) => ParseNumberResult::Overflow,
    }
}

pub fn parse_number(input: &str, start_byte_index: usize) -> Option<(i32, usize)> {
    match parse_number_checked(input, start_byte_index) {
        ParseNumberResult::Success(value, end_index) => Some((value, end_index)),
        _ => None,
    }
}

pub fn skip_whitespace(input: &str, start_byte_index: usize) -> usize {
    let remaining = match input.get(start_byte_index..) {
        Some(s) => s,
        None => return start_byte_index,
    };

    let mut byte_index = start_byte_index;
    for ch in remaining.chars() {
        if ch != ' ' {
            break;
        }
        byte_index += ch.len_utf8();
    }
    byte_index
}

pub fn match_word(input: &str, start_byte_index: usize, word: &str) -> bool {
    let remaining = match input.get(start_byte_index..) {
        Some(s) => s,
        None => return false,
    };

    if !remaining.starts_with(word) {
        return false;
    }

    // Ensure word boundary - next char must be space, dot, or end of input
    let after_word = &remaining[word.len()..];
    if after_word.is_empty() {
        return true;
    }

    matches!(after_word.chars().next(), Some(' ') | Some('.'))
}

pub fn parse_comparison_operator(
    input: &str,
    start_index: usize,
) -> Option<(ComparisonOperator, usize)> {
    let i = skip_whitespace(input, start_index);
    let remaining = input.get(i..)?;

    if remaining.starts_with("==") {
        Some((ComparisonOperator::Equal, i + 2))
    } else if remaining.starts_with("!=") {
        Some((ComparisonOperator::NotEqual, i + 2))
    } else if remaining.starts_with("<=") {
        Some((ComparisonOperator::LessThanOrEqual, i + 2))
    } else if remaining.starts_with(">=") {
        Some((ComparisonOperator::GreaterThanOrEqual, i + 2))
    } else if remaining.starts_with('<') {
        Some((ComparisonOperator::LessThan, i + 1))
    } else if remaining.starts_with('>') {
        Some((ComparisonOperator::GreaterThan, i + 1))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_line_empty() {
        assert_eq!(classify_line("", 1).token_type, TokenType::Empty);
        assert_eq!(classify_line("   ", 1).token_type, TokenType::Empty);
    }

    #[test]
    fn test_classify_line_comment() {
        assert_eq!(classify_line("# comment", 1).token_type, TokenType::Comment);
    }

    #[test]
    fn test_classify_line_pragma() {
        assert_eq!(
            classify_line("#! shell: /bin/bash", 1).token_type,
            TokenType::Pragma
        );
    }

    #[test]
    fn test_classify_line_test() {
        assert_eq!(
            classify_line("TEST \"name\"", 1).token_type,
            TokenType::Test
        );
    }

    #[test]
    fn test_classify_line_run() {
        assert_eq!(
            classify_line("RUN echo hello", 1).token_type,
            TokenType::Run
        );
    }

    #[test]
    fn test_classify_line_assert() {
        assert_eq!(
            classify_line("ASSERT exit_code == 0", 1).token_type,
            TokenType::Assert
        );
    }

    #[test]
    fn test_classify_line_env() {
        assert_eq!(classify_line("ENV FOO=bar", 1).token_type, TokenType::Env);
    }

    #[test]
    fn test_classify_line_unknown() {
        assert_eq!(
            classify_line("UNKNOWN statement", 1).token_type,
            TokenType::Unknown
        );
    }

    #[test]
    fn test_span_creation() {
        let span = crate::parser::ast::Span::new(0, 10, 1, 0, 1, 10);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
        assert_eq!(span.start_line, 1);
        assert_eq!(span.end_line, 1);
    }

    #[test]
    fn test_span_single_line() {
        let span = crate::parser::ast::Span::single_line(5, 10, 20);
        assert_eq!(span.start_line, 5);
        assert_eq!(span.end_line, 5);
        assert_eq!(span.start_col, 10);
        assert_eq!(span.end_col, 20);
    }

    #[test]
    fn test_parse_string_literal_double_quoted() {
        let result = parse_string_literal("\"hello world\"", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.value, "hello world");
        assert_eq!(literal.quote_type, QuoteType::Double);
    }

    #[test]
    fn test_parse_string_literal_single_quoted() {
        let result = parse_string_literal("'hello world'", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.value, "hello world");
        assert_eq!(literal.quote_type, QuoteType::Single);
    }

    #[test]
    fn test_parse_string_literal_escape_sequences_double_quotes() {
        let result = parse_string_literal("\"line1\\nline2\"", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.value, "line1\nline2");
    }

    #[test]
    fn test_parse_string_literal_no_escape_single_quotes() {
        let result = parse_string_literal("'line1\\nline2'", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.value, "line1\\nline2");
    }

    #[test]
    fn test_parse_string_literal_non_string() {
        assert!(parse_string_literal("hello", 0).is_none());
    }

    #[test]
    fn test_parse_string_literal_unterminated() {
        assert!(parse_string_literal("\"hello", 0).is_none());
    }

    #[test]
    fn test_parse_string_literal_at_offset() {
        let result = parse_string_literal("contains \"text\"", 9);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.value, "text");
    }

    #[test]
    fn test_parse_regex_literal_simple() {
        let result = parse_regex_literal("/pattern/", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.pattern, "pattern");
        assert_eq!(literal.flags, "");
    }

    #[test]
    fn test_parse_regex_literal_with_flags() {
        // Test with valid Rust regex flags: i, m, s, u, x
        let result = parse_regex_literal("/pattern/ims", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.pattern, "pattern");
        assert_eq!(literal.flags, "ims");
    }

    #[test]
    fn test_parse_regex_literal_invalid_flags_not_parsed() {
        // JavaScript-specific flags like 'g' and 'y' should stop parsing
        // (they are not valid Rust regex flags)
        let result = parse_regex_literal("/pattern/ig", 0);
        assert!(result.is_some());
        let (literal, end_index) = result.unwrap();
        assert_eq!(literal.pattern, "pattern");
        // 'i' is valid, but 'g' stops parsing
        assert_eq!(literal.flags, "i");
        // end_index should stop before 'g'
        assert_eq!(end_index, 10); // "/pattern/i" = 10 chars
    }

    #[test]
    fn test_parse_regex_literal_escaped_slashes() {
        let result = parse_regex_literal("/path\\/to\\/file/", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.pattern, "path\\/to\\/file");
    }

    #[test]
    fn test_parse_regex_literal_non_regex() {
        assert!(parse_regex_literal("pattern", 0).is_none());
    }

    #[test]
    fn test_parse_regex_literal_unterminated() {
        assert!(parse_regex_literal("/pattern", 0).is_none());
    }

    #[test]
    fn test_parse_duration_milliseconds() {
        let result = parse_duration("200ms", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 200.0);
        assert_eq!(duration.unit, DurationUnit::Milliseconds);
    }

    #[test]
    fn test_parse_duration_seconds() {
        let result = parse_duration("5s", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 5.0);
        assert_eq!(duration.unit, DurationUnit::Seconds);
    }

    #[test]
    fn test_parse_duration_decimal_seconds() {
        let result = parse_duration("1.5s", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 1.5);
        assert_eq!(duration.unit, DurationUnit::Seconds);
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        assert!(parse_duration("100min", 0).is_none());
    }

    #[test]
    fn test_parse_duration_missing_unit() {
        assert!(parse_duration("100", 0).is_none());
    }

    #[test]
    fn test_parse_duration_zero() {
        let result = parse_duration("0ms", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 0.0);
        assert_eq!(duration.unit, DurationUnit::Milliseconds);

        let result = parse_duration("0s", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 0.0);
        assert_eq!(duration.unit, DurationUnit::Seconds);
    }

    #[test]
    fn test_parse_duration_leading_decimal_point() {
        let result = parse_duration(".5s", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 0.5);
    }

    #[test]
    fn test_parse_duration_trailing_decimal_point() {
        let result = parse_duration("5.s", 0);
        assert!(result.is_some());
        let (duration, _) = result.unwrap();
        assert_eq!(duration.value, 5.0);
    }

    #[test]
    fn test_parse_duration_invalid_multiple_decimals() {
        assert!(parse_duration("1.2.3s", 0).is_none());
    }

    #[test]
    fn test_parse_duration_rejects_negative() {
        assert!(parse_duration("-5s", 0).is_none());
    }

    #[test]
    fn test_parse_duration_rejects_infinity() {
        // Very large values that parse to f64::INFINITY should be rejected
        let huge = format!("{}s", "9".repeat(400));
        assert!(
            parse_duration(&huge, 0).is_none(),
            "parse_duration should reject values that become infinity"
        );
    }

    #[test]
    fn test_parse_duration_rejects_very_large_seconds() {
        // Values larger than ~1 year should be rejected
        let large_seconds = "999999999999s"; // ~31k years
        assert!(
            parse_duration(large_seconds, 0).is_none(),
            "parse_duration should reject unreasonably large durations in seconds"
        );
    }

    #[test]
    fn test_parse_duration_rejects_very_large_milliseconds() {
        // Same duration limit applies to milliseconds
        // ~1 year = 31,536,000,000 ms, so 999,999,999,999,999 ms should be rejected
        let large_ms = "999999999999999ms";
        assert!(
            parse_duration(large_ms, 0).is_none(),
            "parse_duration should reject unreasonably large durations in milliseconds"
        );
    }

    #[test]
    fn test_parse_duration_accepts_reasonable_large_values() {
        // ~11.5 days in seconds should be accepted (used in integration tests)
        let result = parse_duration("999999s", 0);
        assert!(result.is_some(), "~11.5 days should be acceptable");

        // ~1 week in milliseconds should be accepted
        let result = parse_duration("604800000ms", 0);
        assert!(result.is_some(), "~1 week in ms should be acceptable");
    }

    #[test]
    fn test_parse_number_positive() {
        let result = parse_number("42", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_parse_number_negative() {
        let result = parse_number("-1", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, -1);
    }

    #[test]
    fn test_parse_number_zero() {
        let result = parse_number("0", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, 0);
    }

    #[test]
    fn test_parse_number_non_number() {
        assert!(parse_number("abc", 0).is_none());
    }

    #[test]
    fn test_parse_number_with_whitespace() {
        let result = parse_number("  42", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_parse_comparison_operator_with_leading_spaces() {
        // Test that whitespace is properly skipped
        let result = parse_comparison_operator("  ==", 0);
        assert!(result.is_some());
        let (op, end_idx) = result.unwrap();
        assert_eq!(op, ComparisonOperator::Equal);
        assert_eq!(end_idx, 4); // 2 spaces + 2 chars for "=="
    }

    #[test]
    fn test_parse_comparison_operator_no_panic_on_utf8() {
        // Ensure we don't panic on UTF-8 input (even if we don't find an operator)
        let result = parse_comparison_operator("日本語", 0);
        assert!(result.is_none()); // No operator, but no panic
    }

    #[test]
    fn test_skip_whitespace_returns_byte_index() {
        // Verify skip_whitespace works correctly with UTF-8
        let input = "日  x";
        let byte_index = skip_whitespace(input, 3); // Start after 日 (3 bytes)
                                                    // Should skip 2 spaces and return byte position of 'x' (byte 5)
        assert_eq!(byte_index, 5);
        // Should be valid for slicing
        assert_eq!(&input[byte_index..], "x");
    }

    #[test]
    fn test_match_word_with_byte_index() {
        // Test that match_word works with byte indices
        let input = "日 contains";
        // 日 is 3 bytes, space is 1 byte, so "contains" starts at byte 4
        assert!(match_word(input, 4, "contains"));
        assert!(!match_word(input, 0, "contains"));
    }

    #[test]
    fn test_parse_string_literal_with_unicode() {
        // Test parsing string with unicode content
        let result = parse_string_literal("\"日本語\"", 0);
        assert!(result.is_some());
        let (literal, end_index) = result.unwrap();
        assert_eq!(literal.value, "日本語");
        // Verify end_index is a valid byte index: 1 + 3*3 + 1 = 11 bytes
        assert_eq!(end_index, 11);
    }

    #[test]
    fn test_parse_string_literal_returns_byte_index() {
        // Test that returned index is byte index, not char index
        let input = "\"日本語\" exists";
        let result = parse_string_literal(input, 0);
        assert!(result.is_some());
        let (_, end_index) = result.unwrap();
        // end_index should be usable for slicing
        assert_eq!(&input[end_index..], " exists");
    }

    #[test]
    fn test_parse_string_literal_at_byte_offset() {
        // Test parsing string at a byte offset after unicode
        let input = "日 \"test\"";
        // 日 is 3 bytes, space is 1 byte, quote starts at byte 4
        let result = parse_string_literal(input, 4);
        assert!(result.is_some());
        let (literal, end_index) = result.unwrap();
        assert_eq!(literal.value, "test");
        assert_eq!(end_index, 10); // 4 + 1 + 4 + 1
        assert_eq!(&input[end_index..], "");
    }

    #[test]
    fn test_parse_regex_literal_with_unicode() {
        // Test parsing regex with unicode content
        let result = parse_regex_literal("/日本語/", 0);
        assert!(result.is_some());
        let (literal, end_index) = result.unwrap();
        assert_eq!(literal.pattern, "日本語");
        // Verify end_index is a valid byte index
        assert_eq!(end_index, 11);
    }

    #[test]
    fn test_parse_regex_literal_returns_byte_index() {
        // Test that returned index is byte index
        let input = "/日本語/i more";
        let result = parse_regex_literal(input, 0);
        assert!(result.is_some());
        let (literal, end_index) = result.unwrap();
        assert_eq!(literal.flags, "i");
        // end_index should be usable for slicing
        assert_eq!(&input[end_index..], " more");
    }

    #[test]
    fn test_parse_number_at_byte_offset() {
        // Test parsing number at a byte offset after unicode chars
        let input = "日 42";
        // 日 is 3 bytes, space is 1 byte, so "42" starts at byte 4
        let result = parse_number(input, 4);
        assert!(
            result.is_some(),
            "parse_number should succeed at byte offset 4"
        );
        let (value, end_index) = result.unwrap();
        assert_eq!(value, 42);
        // end_index should be usable for string slicing
        assert_eq!(&input[end_index..], "");
    }

    #[test]
    fn test_parse_number_returns_byte_index() {
        // Test that the returned index is a byte index, not char index
        let input = "日 42 more";
        let result = parse_number(input, 4);
        assert!(result.is_some());
        let (value, end_index) = result.unwrap();
        assert_eq!(value, 42);
        // end_index should be usable for slicing to get remaining content
        assert_eq!(&input[end_index..], " more");
    }

    #[test]
    fn test_parse_duration_at_byte_offset() {
        // Test parsing duration at a byte offset after unicode chars
        let input = "日 200ms";
        // 日 is 3 bytes, space is 1 byte, so "200ms" starts at byte 4
        let result = parse_duration(input, 4);
        assert!(
            result.is_some(),
            "parse_duration should succeed at byte offset 4"
        );
        let (duration, end_index) = result.unwrap();
        assert_eq!(duration.value, 200.0);
        assert_eq!(duration.unit, DurationUnit::Milliseconds);
        // end_index should be usable for string slicing
        assert_eq!(&input[end_index..], "");
    }

    #[test]
    fn test_parse_duration_returns_byte_index() {
        // Test that the returned index is a byte index, not char index
        let input = "日 1.5s more";
        let result = parse_duration(input, 4);
        assert!(result.is_some());
        let (duration, end_index) = result.unwrap();
        assert_eq!(duration.value, 1.5);
        // end_index should be usable for slicing to get remaining content
        assert_eq!(&input[end_index..], " more");
    }

    #[test]
    fn test_skip_whitespace_out_of_bounds() {
        let input = "hello";
        // Should not panic and should return the input index unchanged
        let result = skip_whitespace(input, 100);
        assert_eq!(result, 100);
    }

    #[test]
    fn test_skip_whitespace_at_end() {
        let input = "hello";
        // At exactly the end of the string
        let result = skip_whitespace(input, 5);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_parse_comparison_operator_out_of_bounds() {
        let input = "hello";
        // Should not panic and should return None
        let result = parse_comparison_operator(input, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_number_overflow_positive() {
        // Value exceeding i32::MAX should return None
        let result = parse_number("99999999999999999999", 0);
        assert!(result.is_none(), "Overflow should return None");
    }

    #[test]
    fn test_parse_number_overflow_negative() {
        // Value below i32::MIN should return None
        let result = parse_number("-99999999999999999999", 0);
        assert!(result.is_none(), "Negative overflow should return None");
    }

    #[test]
    fn test_parse_number_i32_max() {
        // i32::MAX should parse successfully
        let result = parse_number("2147483647", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, i32::MAX);
    }

    #[test]
    fn test_parse_number_i32_min() {
        // i32::MIN should parse successfully
        let result = parse_number("-2147483648", 0);
        assert!(result.is_some());
        let (value, _) = result.unwrap();
        assert_eq!(value, i32::MIN);
    }

    #[test]
    fn test_parse_number_just_over_i32_max() {
        // One more than i32::MAX should fail
        let result = parse_number("2147483648", 0);
        assert!(
            result.is_none(),
            "Value just over i32::MAX should return None"
        );
    }

    #[test]
    fn test_parse_number_just_under_i32_min() {
        // One less than i32::MIN should fail
        let result = parse_number("-2147483649", 0);
        assert!(
            result.is_none(),
            "Value just under i32::MIN should return None"
        );
    }

    #[test]
    fn test_parse_number_checked_overflow() {
        let result = parse_number_checked("99999999999999999999", 0);
        assert_eq!(result, ParseNumberResult::Overflow);
    }

    #[test]
    fn test_parse_number_checked_not_a_number() {
        let result = parse_number_checked("abc", 0);
        assert_eq!(result, ParseNumberResult::NotANumber);
    }

    #[test]
    fn test_parse_number_checked_success() {
        let result = parse_number_checked("42", 0);
        assert_eq!(result, ParseNumberResult::Success(42, 2));
    }
}
