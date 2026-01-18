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

pub fn parse_string_literal(input: &str, start_index: usize) -> Option<(StringLiteral, usize)> {
    let chars: Vec<char> = input.chars().collect();

    if start_index >= chars.len() {
        return None;
    }

    let start_char = chars[start_index];
    if start_char != '"' && start_char != '\'' {
        return None;
    }

    let quote_type = if start_char == '"' {
        QuoteType::Double
    } else {
        QuoteType::Single
    };

    let mut value = String::new();
    let mut i = start_index + 1;
    let mut escaped = false;

    while i < chars.len() {
        let ch = chars[i];

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
            // End of string
            let end_index = i + 1;
            let raw = input
                .chars()
                .skip(start_index)
                .take(end_index - start_index)
                .collect();
            return Some((
                StringLiteral {
                    value,
                    raw,
                    quote_type,
                },
                end_index,
            ));
        } else {
            value.push(ch);
        }
        i += 1;
    }

    None
}

pub fn parse_regex_literal(input: &str, start_index: usize) -> Option<(RegexLiteral, usize)> {
    let chars: Vec<char> = input.chars().collect();

    if start_index >= chars.len() || chars[start_index] != '/' {
        return None;
    }

    let mut pattern = String::new();
    let mut i = start_index + 1;
    let mut escaped = false;

    // Parse pattern
    while i < chars.len() {
        let ch = chars[i];

        if escaped {
            pattern.push(ch);
            escaped = false;
        } else if ch == '\\' {
            pattern.push(ch);
            escaped = true;
        } else if ch == '/' {
            // End of pattern, parse flags
            i += 1;
            let mut flags = String::new();
            while i < chars.len() && matches!(chars[i], 'g' | 'i' | 'm' | 's' | 'u' | 'y') {
                flags.push(chars[i]);
                i += 1;
            }

            let raw = input
                .chars()
                .skip(start_index)
                .take(i - start_index)
                .collect();
            return Some((
                RegexLiteral {
                    pattern,
                    flags,
                    raw,
                },
                i,
            ));
        } else {
            pattern.push(ch);
        }
        i += 1;
    }

    None
}

pub fn parse_duration(input: &str, start_index: usize) -> Option<(Duration, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = start_index;

    // Skip whitespace
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }

    let num_start = i;

    // Parse number (including decimal)
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }

    if i == num_start {
        return None;
    }

    let num_str: String = chars[num_start..i].iter().collect();
    let value = num_str.parse::<f64>().ok()?;

    // Parse unit
    let unit_start = i;
    while i < chars.len() && chars[i].is_ascii_lowercase() {
        i += 1;
    }

    let unit_str: String = chars[unit_start..i].iter().collect();
    let unit = match unit_str.as_str() {
        "ms" => DurationUnit::Milliseconds,
        "s" => DurationUnit::Seconds,
        _ => return None,
    };

    let raw: String = chars[start_index..i]
        .iter()
        .collect::<String>()
        .trim()
        .to_string();

    Some((Duration { value, unit, raw }, i))
}

pub fn parse_number(input: &str, start_index: usize) -> Option<(i32, usize)> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = start_index;

    // Skip whitespace
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }

    let num_start = i;

    // Handle negative numbers
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }

    // Parse digits
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    if i == num_start || (i == num_start + 1 && chars.get(num_start) == Some(&'-')) {
        return None;
    }

    let num_str: String = chars[num_start..i].iter().collect();
    let value = num_str.parse::<i32>().ok()?;

    Some((value, i))
}

pub fn skip_whitespace(input: &str, start_byte_index: usize) -> usize {
    let mut byte_index = start_byte_index;
    for ch in input[start_byte_index..].chars() {
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
    let remaining = &input[i..];

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
        let result = parse_regex_literal("/pattern/gi", 0);
        assert!(result.is_some());
        let (literal, _) = result.unwrap();
        assert_eq!(literal.pattern, "pattern");
        assert_eq!(literal.flags, "gi");
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
}
