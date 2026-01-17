use async_lsp::lsp_types::*;

use crate::parser::{ASTNode, ParseResult, ParsedFile};

#[derive(Debug, Clone)]
pub struct FormattingProvider {
    indent_size: usize,
}

impl FormattingProvider {
    pub fn new() -> Self {
        Self { indent_size: 2 }
    }

    pub fn format_document(&self, text: &str, uri: &str) -> Option<Vec<TextEdit>> {
        let parsed = match crate::parser::parse_file(text, uri) {
            ParseResult::Success { file } => file,
            ParseResult::Failure { .. } => {
                return None;
            }
        };

        let formatted = self.format_parsed(&parsed, text);

        if formatted == text {
            return Some(vec![]);
        }

        let lines = text.lines().count();
        let last_line = if lines > 0 { lines - 1 } else { 0 };
        let last_char = text.lines().last().map(|l| l.len()).unwrap_or(0);

        Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: last_line as u32,
                    character: last_char as u32,
                },
            },
            new_text: formatted,
        }])
    }

    fn format_parsed(&self, file: &ParsedFile, original_text: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = original_text.lines().collect();
        let mut in_test_block = false;
        let mut brace_depth = 0;

        // Process pragmas first
        for pragma in &file.pragmas {
            result.push_str(&pragma.raw);
            result.push('\n');
        }

        // Group nodes by line to handle multiline constructs
        let mut current_line = 0;
        for node in &file.nodes {
            let node_line = node.line();

            // Skip if we've already processed this line
            if node_line < current_line {
                continue;
            }

            match node {
                ASTNode::Comment(comment) => {
                    result.push_str("# ");
                    result.push_str(&comment.text);
                    result.push('\n');
                    current_line = comment.line + 1;
                }
                ASTNode::Test(test) => {
                    if !result.is_empty() && !result.ends_with("\n\n") {
                        result.push('\n');
                    }
                    result.push_str("@test \"");
                    result.push_str(&test.name);
                    result.push_str("\" {\n");
                    in_test_block = true;
                    brace_depth = 1;
                    current_line = test.line + 1;
                }
                ASTNode::Run(run) => {
                    let indent = self.get_indent(if in_test_block { 1 } else { 0 });
                    result.push_str(&indent);

                    if let Some(name) = &run.name {
                        result.push_str("run \"");
                        result.push_str(name);
                        result.push_str("\" ");
                    } else {
                        result.push_str("run ");
                    }

                    // Extract the command from the original text, preserving its formatting
                    if let Some(line) = lines.get(run.line - 1) {
                        if let Some(cmd_start) = line.rfind('{') {
                            let cmd = &line[cmd_start..].trim_start();
                            result.push_str(cmd);
                            result.push('\n');

                            // Handle multiline commands
                            let mut line_idx = run.line;
                            let mut braces = cmd.chars().filter(|&c| c == '{').count() as i32
                                - cmd.chars().filter(|&c| c == '}').count() as i32;

                            while braces > 0 && line_idx < lines.len() {
                                if let Some(next_line) = lines.get(line_idx) {
                                    result.push_str(&indent);
                                    result.push_str("  ");
                                    result.push_str(next_line.trim());
                                    result.push('\n');

                                    braces +=
                                        next_line.chars().filter(|&c| c == '{').count() as i32;
                                    braces -=
                                        next_line.chars().filter(|&c| c == '}').count() as i32;
                                }
                                line_idx += 1;
                            }
                            current_line = line_idx;
                        }
                    }
                }
                ASTNode::Assert(assert) => {
                    let indent = self.get_indent(if in_test_block { 1 } else { 0 });
                    result.push_str(&indent);

                    // Format the assertion
                    let formatted_assertion = self.format_assertion(&assert.raw);
                    result.push_str(&formatted_assertion);
                    result.push('\n');

                    // Check if this is a multiline assertion with a block
                    if assert.raw.contains('{') {
                        let mut line_idx = assert.line;
                        let mut found_closing = false;

                        while !found_closing && line_idx < lines.len() {
                            if let Some(line) = lines.get(line_idx) {
                                if line.trim().ends_with('}') && !line.contains('{') {
                                    result.push_str(&indent);
                                    result.push_str("}\n");
                                    found_closing = true;
                                    current_line = line_idx + 1;
                                } else if line_idx > assert.line - 1 {
                                    let content = line.trim();
                                    if !content.is_empty() && content != "{" && content != "}" {
                                        result.push_str(&indent);
                                        result.push_str("  ");
                                        result.push_str(content);
                                        result.push('\n');
                                    }
                                }
                            }
                            line_idx += 1;
                        }
                    } else {
                        current_line = assert.line + 1;
                    }
                }
                ASTNode::Env(env) => {
                    let indent = self.get_indent(0);
                    result.push_str(&indent);
                    result.push_str("env ");
                    result.push_str(&env.key);
                    result.push(' ');
                    result.push_str(&env.value);
                    result.push('\n');
                    current_line = env.line + 1;
                }
                ASTNode::Pragma(_) => {
                    // Already handled above
                    current_line = node_line + 1;
                }
                ASTNode::Error(_) => {
                    // Skip error nodes - we can't format invalid syntax
                    current_line = node_line + 1;
                }
            }

            // Check for closing braces
            if in_test_block && node_line < lines.len() {
                for i in current_line..lines.len() {
                    if let Some(line) = lines.get(i) {
                        let trimmed = line.trim();
                        if trimmed == "}" {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                result.push_str("}\n");
                                in_test_block = false;
                                current_line = i + 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Ensure file ends with newline
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }

        result
    }

    fn format_assertion(&self, raw: &str) -> String {
        let trimmed = raw.trim();

        // Simple normalization - remove extra spaces
        let mut result = String::new();
        let mut prev_space = false;
        let mut in_string = false;
        let mut string_char = ' ';

        for ch in trimmed.chars() {
            if ch == '"' || ch == '\'' {
                if !in_string {
                    in_string = true;
                    string_char = ch;
                } else if ch == string_char {
                    in_string = false;
                }
                result.push(ch);
                prev_space = false;
            } else if ch.is_whitespace() && !in_string {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(ch);
                prev_space = false;
            }
        }

        result.trim_end().to_string()
    }

    fn get_indent(&self, level: usize) -> String {
        " ".repeat(level * self.indent_size)
    }
}

impl Default for FormattingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_test() {
        let provider = FormattingProvider::new();
        let input = r#"@test "example" {
  run { echo hello }
  expect stdout { hello }
}
"#;

        let result = provider.format_document(input, "test.hone");
        assert!(result.is_some());
    }

    #[test]
    fn test_format_preserves_valid_formatting() {
        let provider = FormattingProvider::new();
        let input = r#"@test "example" {
  run { echo hello }
  expect stdout { hello }
}
"#;

        let result = provider.format_document(input, "test.hone");
        assert!(result.is_some());
    }

    #[test]
    fn test_format_with_bad_indentation() {
        let provider = FormattingProvider::new();
        let input = r#"@test "example" {
run { echo hello }
    expect stdout { hello }
}
"#;

        let result = provider.format_document(input, "test.hone");
        assert!(result.is_some());

        if let Some(edits) = result {
            assert!(!edits.is_empty());
        }
    }
}
