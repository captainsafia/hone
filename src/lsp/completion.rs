use async_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InsertTextFormat,
    Position,
};

use crate::lsp::shell::ShellCommands;
use crate::parser::ast::ParsedFile;

/// Converts a UTF-16 code unit offset to a byte offset in a UTF-8 string.
/// LSP protocol uses UTF-16 code units for character positions.
/// Returns the byte offset, or the string length if the UTF-16 offset is beyond the string.
fn utf16_offset_to_byte_offset(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    for (byte_idx, ch) in s.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_idx;
        }
        utf16_count += ch.len_utf16();
    }
    s.len()
}

/// Returns a string prefix up to `utf16_offset` UTF-16 code units.
/// If `utf16_offset` is beyond the string length, returns the entire string.
fn safe_prefix(s: &str, utf16_offset: usize) -> &str {
    let byte_idx = utf16_offset_to_byte_offset(s, utf16_offset);
    &s[..byte_idx]
}

#[derive(Debug, Clone)]
pub struct CompletionProvider {
    shell_commands: ShellCommands,
}

impl CompletionProvider {
    pub(crate) fn new() -> Self {
        Self {
            shell_commands: ShellCommands::new(),
        }
    }

    pub(crate) fn provide_completions(
        &self,
        parsed: &ParsedFile,
        params: &CompletionParams,
        document_text: &str,
    ) -> Option<CompletionResponse> {
        let position = params.text_document_position.position;
        let context = self.determine_context(parsed, position, document_text);

        let items = match context.context_type {
            CompletionContextType::TopLevel => self.top_level_completions(&context),
            CompletionContextType::InsideTest => self.inside_test_completions(&context),
            CompletionContextType::AfterExpect => self.assertion_completions(&context),
            CompletionContextType::AfterRun => self.shell_command_completions(),
            CompletionContextType::Unknown => Vec::new(),
        };

        if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        }
    }

    fn determine_context(
        &self,
        parsed: &ParsedFile,
        position: Position,
        document_text: &str,
    ) -> CompletionContextInfo {
        let line_idx = position.line as usize;
        let col_idx = position.character as usize;

        let lines: Vec<&str> = document_text.lines().collect();
        if line_idx >= lines.len() {
            return CompletionContextInfo {
                context_type: CompletionContextType::Unknown,
                current_line: String::new(),
                prefix: String::new(),
                indent: 0,
            };
        }

        let current_line = lines[line_idx];
        let prefix = safe_prefix(current_line, col_idx);

        // Calculate indentation of current line
        let indent = current_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .count();

        // Check if we're after "ASSERT " (with trailing space)
        if prefix.trim_start().starts_with("ASSERT ") && prefix.ends_with(' ') {
            return CompletionContextInfo {
                context_type: CompletionContextType::AfterExpect,
                current_line: current_line.to_string(),
                prefix: prefix.to_string(),
                indent,
            };
        }

        // Check if we're after "RUN " (with trailing space) or "run "
        if (prefix.trim_start().starts_with("RUN ") || prefix.trim_start().starts_with("run "))
            && prefix.ends_with(' ')
        {
            return CompletionContextInfo {
                context_type: CompletionContextType::AfterRun,
                current_line: current_line.to_string(),
                prefix: prefix.to_string(),
                indent,
            };
        }

        // Determine if we're inside a test block by checking AST nodes
        // In the line-oriented syntax, we're inside a test from the TEST line
        // until we hit another TEST or the end of file
        let current_line_num = line_idx + 1; // AST uses 1-based line numbers
        let mut inside_test = false;

        for node in &parsed.nodes {
            let node_line = node.line();

            // If this is a test node and it's before or at the current line, we might be inside it
            if let crate::parser::ast::ASTNode::Test(_) = node {
                if node_line < current_line_num {
                    // We're past a test node, so we're inside it
                    inside_test = true;
                } else if node_line == current_line_num {
                    // We're on the TEST line itself, treat as top-level
                    inside_test = false;
                    break;
                } else {
                    // We hit a test after the current line, stop
                    break;
                }
            }
        }

        if inside_test {
            return CompletionContextInfo {
                context_type: CompletionContextType::InsideTest,
                current_line: current_line.to_string(),
                prefix: prefix.to_string(),
                indent,
            };
        }

        // Default to top-level context
        CompletionContextInfo {
            context_type: CompletionContextType::TopLevel,
            current_line: current_line.to_string(),
            prefix: prefix.to_string(),
            indent,
        }
    }

    fn top_level_completions(&self, context: &CompletionContextInfo) -> Vec<CompletionItem> {
        let indent_str = " ".repeat(context.indent);

        vec![CompletionItem {
            label: "TEST".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Define a test".to_string()),
            insert_text: Some(format!(
                "TEST \"${{1:name}}\"\n{indent_str}RUN ${{2:command}}",
                indent_str = indent_str
            )),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }]
    }

    fn inside_test_completions(&self, _context: &CompletionContextInfo) -> Vec<CompletionItem> {
        let mut items = vec![
            CompletionItem {
                label: "ASSERT".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Add an assertion".to_string()),
                insert_text: Some("ASSERT ".to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            },
            CompletionItem {
                label: "RUN".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Execute a shell command".to_string()),
                insert_text: Some("RUN ${1:command}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ];

        // Also suggest shell commands at this level
        items.extend(self.shell_command_completions());

        items
    }

    fn assertion_completions(&self, context: &CompletionContextInfo) -> Vec<CompletionItem> {
        let indent = context.indent;
        let indent_str = " ".repeat(indent);
        let inner_indent = " ".repeat(indent + 2);

        vec![
            CompletionItem {
                label: "stdout".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on standard output".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the standard output of the command".to_string(),
                )),
                insert_text: Some(format!("stdout {{\n{inner_indent}${{1:contains \"text\"}}\n{indent_str}}}", inner_indent=inner_indent, indent_str=indent_str)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "stderr".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on standard error".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the standard error of the command".to_string(),
                )),
                insert_text: Some(format!("stderr {{\n{inner_indent}${{1:contains \"text\"}}\n{indent_str}}}", inner_indent=inner_indent, indent_str=indent_str)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "exitcode".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on exit code".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the exit code of the command (0-255)".to_string(),
                )),
                insert_text: Some("exitcode ${1:0}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "file".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on file contents".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check the contents of a file".to_string(),
                )),
                insert_text: Some(format!("file \"${{1:path}}\" {{\n{inner_indent}${{2:contains \"text\"}}\n{indent_str}}}", inner_indent=inner_indent, indent_str=indent_str)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "duration".to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Assert on execution duration".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    "Check how long the command took to execute".to_string(),
                )),
                insert_text: Some("duration < ${1:1s}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    fn shell_command_completions(&self) -> Vec<CompletionItem> {
        // Get common commands with descriptions
        let mut items: Vec<CompletionItem> = self
            .shell_commands
            .common_with_descriptions()
            .into_iter()
            .map(|(name, description)| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Shell command".to_string()),
                documentation: Some(async_lsp::lsp_types::Documentation::String(
                    description.to_string(),
                )),
                insert_text: Some(name.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                sort_text: Some(format!("0{}", name)), // Prioritize common commands
                ..Default::default()
            })
            .collect();

        // Add PATH commands (lower priority)
        let all_commands = self.shell_commands.all_commands();
        for cmd in all_commands {
            // Skip if already in common commands
            if self.shell_commands.get_description(&cmd).is_some() {
                continue;
            }

            items.push(CompletionItem {
                label: cmd.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Shell command".to_string()),
                insert_text: Some(cmd.clone()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                sort_text: Some(format!("1{}", cmd)), // Lower priority
                ..Default::default()
            });
        }

        items
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionContextType {
    TopLevel,
    InsideTest,
    AfterExpect,
    AfterRun,
    Unknown,
}

#[derive(Debug, Clone)]
struct CompletionContextInfo {
    context_type: CompletionContextType,
    #[allow(dead_code)]
    current_line: String,
    #[allow(dead_code)]
    prefix: String,
    indent: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;

    #[test]
    fn test_safe_prefix_ascii() {
        // For ASCII, UTF-16 offset == byte offset == char count
        assert_eq!(safe_prefix("hello", 3), "hel");
        assert_eq!(safe_prefix("hello", 5), "hello");
        assert_eq!(safe_prefix("hello", 10), "hello");
        assert_eq!(safe_prefix("hello", 0), "");
    }

    #[test]
    fn test_safe_prefix_utf8() {
        // "日本語" is 9 bytes (3 chars × 3 bytes each), but 3 UTF-16 code units
        let s = "日本語";
        assert_eq!(safe_prefix(s, 0), "");
        assert_eq!(safe_prefix(s, 1), "日"); // 1 UTF-16 code unit = first char
        assert_eq!(safe_prefix(s, 2), "日本"); // 2 UTF-16 code units
        assert_eq!(safe_prefix(s, 3), "日本語"); // all 3 UTF-16 code units
        assert_eq!(safe_prefix(s, 100), "日本語"); // beyond length
    }

    #[test]
    fn test_safe_prefix_mixed() {
        // "Test 日本語" - mixed ASCII and UTF-8
        // UTF-16: T(1) + e(1) + s(1) + t(1) + space(1) + 日(1) + 本(1) + 語(1) = 8 code units
        let s = "Test 日本語";
        assert_eq!(safe_prefix(s, 5), "Test "); // ASCII part (5 UTF-16 code units)
        assert_eq!(safe_prefix(s, 6), "Test 日"); // includes first kanji
        assert_eq!(safe_prefix(s, 7), "Test 日本"); // includes second kanji
        assert_eq!(safe_prefix(s, 8), "Test 日本語"); // full string
    }

    #[test]
    fn test_utf16_offset_to_byte_offset() {
        // ASCII: UTF-16 offset == byte offset
        assert_eq!(utf16_offset_to_byte_offset("hello", 0), 0);
        assert_eq!(utf16_offset_to_byte_offset("hello", 3), 3);
        assert_eq!(utf16_offset_to_byte_offset("hello", 5), 5);
        assert_eq!(utf16_offset_to_byte_offset("hello", 100), 5); // beyond length

        // CJK: each char is 1 UTF-16 code unit but 3 bytes
        let cjk = "日本語";
        assert_eq!(utf16_offset_to_byte_offset(cjk, 0), 0);
        assert_eq!(utf16_offset_to_byte_offset(cjk, 1), 3); // after 日
        assert_eq!(utf16_offset_to_byte_offset(cjk, 2), 6); // after 本
        assert_eq!(utf16_offset_to_byte_offset(cjk, 3), 9); // full string

        // Mixed
        let mixed = "A日B";
        assert_eq!(utf16_offset_to_byte_offset(mixed, 0), 0);
        assert_eq!(utf16_offset_to_byte_offset(mixed, 1), 1); // after A
        assert_eq!(utf16_offset_to_byte_offset(mixed, 2), 4); // after 日 (1 + 3 bytes)
        assert_eq!(utf16_offset_to_byte_offset(mixed, 3), 5); // full string (1 + 3 + 1 bytes)
    }

    #[test]
    fn test_safe_prefix_emoji_surrogate_pair() {
        // Emoji 🎉 (U+1F389) is 4 bytes in UTF-8, but 2 UTF-16 code units (surrogate pair)
        let s = "A🎉B";
        // UTF-16: A(1) + 🎉(2) + B(1) = 4 code units
        // Bytes: A(1) + 🎉(4) + B(1) = 6 bytes
        assert_eq!(safe_prefix(s, 0), "");
        assert_eq!(safe_prefix(s, 1), "A"); // after A
                                            // Position 2 is in the middle of the emoji's surrogate pair.
                                            // We return up to the end of the last complete character before the offset,
                                            // but since offset 2 starts the second half of the emoji, we've already
                                            // "seen" the first code unit, so we include the whole emoji.
        assert_eq!(safe_prefix(s, 2), "A🎉"); // mid-emoji; includes whole emoji
        assert_eq!(safe_prefix(s, 3), "A🎉"); // after emoji
        assert_eq!(safe_prefix(s, 4), "A🎉B"); // full string
    }

    #[test]
    fn test_context_detection_top_level() {
        let provider = CompletionProvider::new();
        let text = "\n\n";
        let parsed = match parse_file(text, "test.hone") {
            crate::parser::ParseResult::Success { file } => file,
            _ => panic!("Failed to parse"),
        };

        let position = Position {
            line: 0,
            character: 0,
        };
        let context = provider.determine_context(&parsed, position, text);

        assert_eq!(context.context_type, CompletionContextType::TopLevel);
    }

    #[test]
    fn test_context_detection_inside_test() {
        let provider = CompletionProvider::new();
        let text = "TEST \"example\"\nRUN ls\n";
        let parsed = match parse_file(text, "test.hone") {
            crate::parser::ParseResult::Success { file } => file,
            _ => panic!("Failed to parse"),
        };

        let position = Position {
            line: 1,
            character: 0,
        };
        let context = provider.determine_context(&parsed, position, text);

        assert_eq!(context.context_type, CompletionContextType::InsideTest);
    }

    #[test]
    fn test_context_detection_after_expect() {
        let provider = CompletionProvider::new();
        let text = "TEST \"example\"\nRUN ls\nASSERT ";
        let parsed = match parse_file(text, "test.hone") {
            crate::parser::ParseResult::Success { file } => file,
            _ => panic!("Failed to parse"),
        };

        let position = Position {
            line: 2,
            character: 7,
        };
        let context = provider.determine_context(&parsed, position, text);

        // After "ASSERT " we should detect AfterExpect context
        assert_eq!(context.context_type, CompletionContextType::AfterExpect);
    }

    #[test]
    fn test_context_detection_after_run() {
        let provider = CompletionProvider::new();
        let text = "TEST \"example\"\nRUN ";
        let parsed = match parse_file(text, "test.hone") {
            crate::parser::ParseResult::Success { file } => file,
            _ => panic!("Failed to parse"),
        };

        let position = Position {
            line: 1,
            character: 4,
        };
        let context = provider.determine_context(&parsed, position, text);

        assert_eq!(context.context_type, CompletionContextType::AfterRun);
    }

    #[test]
    fn test_snippet_indentation_at_top_level() {
        let provider = CompletionProvider::new();
        let context = CompletionContextInfo {
            context_type: CompletionContextType::TopLevel,
            current_line: String::new(),
            prefix: String::new(),
            indent: 0,
        };

        let items = provider.top_level_completions(&context);
        assert!(!items.is_empty());

        let test_item = items.iter().find(|i| i.label == "TEST").unwrap();
        assert!(test_item.insert_text.is_some());
    }

    #[test]
    fn test_snippet_indentation_with_indent() {
        let provider = CompletionProvider::new();
        let context = CompletionContextInfo {
            context_type: CompletionContextType::TopLevel,
            current_line: String::new(),
            prefix: String::new(),
            indent: 4,
        };

        let items = provider.top_level_completions(&context);
        let test_item = items.iter().find(|i| i.label == "TEST").unwrap();

        // Should have some insert text
        assert!(test_item.insert_text.is_some());
    }

    #[test]
    fn test_assertion_completions_with_indentation() {
        let provider = CompletionProvider::new();
        let context = CompletionContextInfo {
            context_type: CompletionContextType::AfterExpect,
            current_line: "  ASSERT ".to_string(),
            prefix: "  ASSERT ".to_string(),
            indent: 2,
        };

        let items = provider.assertion_completions(&context);
        assert!(!items.is_empty());

        // Should have completions for assertion types
        assert!(items.iter().any(|i| i.label == "stdout"));
    }

    #[test]
    fn test_context_detection_with_unicode_no_panic() {
        let provider = CompletionProvider::new();
        // Unicode test name: "日本語テスト"
        let text = "TEST \"日本語テスト\"\nRUN echo hi\n";
        let parsed = match parse_file(text, "test.hone") {
            crate::parser::ParseResult::Success { file } => file,
            _ => panic!("Failed to parse"),
        };

        // Position inside the multi-byte characters - this should not panic
        let position = Position {
            line: 0,
            character: 7, // Inside the unicode test name
        };
        // Should not panic, just return some context
        let _context = provider.determine_context(&parsed, position, text);
    }
}
