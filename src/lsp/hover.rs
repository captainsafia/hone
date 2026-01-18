use async_lsp::lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

#[derive(Debug, Clone, Default)]
pub struct HoverProvider;

impl HoverProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn provide_hover(&self, content: &str, params: &HoverParams) -> Option<Hover> {
        let position = params.text_document_position_params.position;
        let line_idx = position.line as usize;
        let char_idx = position.character as usize;

        // Get the line content
        let lines: Vec<&str> = content.lines().collect();
        if line_idx >= lines.len() {
            return None;
        }

        let line = lines[line_idx];
        if char_idx >= line.len() {
            return None;
        }

        // Find the word at the cursor position
        let (word, _start, _end) = self.extract_word_at_position(line, char_idx)?;

        // Look up documentation for the word
        let documentation = self.get_documentation(&word)?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: documentation,
            }),
            range: None,
        })
    }

    fn extract_word_at_position(
        &self,
        line: &str,
        char_idx: usize,
    ) -> Option<(String, usize, usize)> {
        // Find word boundaries
        let chars: Vec<char> = line.chars().collect();
        if char_idx >= chars.len() {
            return None;
        }

        // Find start of word
        let mut start = char_idx;
        while start > 0 {
            let ch = chars[start - 1];
            if !ch.is_alphanumeric() && ch != '@' && ch != '_' {
                break;
            }
            start -= 1;
        }

        // Find end of word
        let mut end = char_idx;
        while end < chars.len() {
            let ch = chars[end];
            if !ch.is_alphanumeric() && ch != '@' && ch != '_' {
                break;
            }
            end += 1;
        }

        if start >= end {
            return None;
        }

        let word: String = chars[start..end].iter().collect();
        Some((word, start, end))
    }

    fn get_documentation(&self, word: &str) -> Option<String> {
        match word {
            "TEST" => Some(self.test_keyword_doc()),
            "RUN" => Some(self.run_keyword_doc()),
            "ASSERT" => Some(self.assert_keyword_doc()),
            "stdout" => Some(self.stdout_assertion_doc()),
            "stdout_raw" => Some(self.stdout_raw_assertion_doc()),
            "stderr" => Some(self.stderr_assertion_doc()),
            "exitcode" | "exit_code" => Some(self.exitcode_assertion_doc()),
            "duration" => Some(self.duration_assertion_doc()),
            "file" => Some(self.file_assertion_doc()),
            _ => None,
        }
    }

    fn test_keyword_doc(&self) -> String {
        r#"# TEST

Define a test case with a name.

## Syntax

```hone
TEST "test name"
RUN command
ASSERT assertion
```

## Example

```hone
TEST "should list files"
RUN ls -la
ASSERT stdout contains "README.md"
ASSERT exit_code == 0
```
"#
        .to_string()
    }

    fn assert_keyword_doc(&self) -> String {
        r#"# ASSERT

Define an assertion to verify test behavior. Must be inside a TEST block.

## Syntax

```hone
ASSERT <assertion-type> <predicate>
```

## Assertion Types

- `stdout` - Assert on standard output
- `stderr` - Assert on standard error
- `exit_code` - Assert on exit code
- `duration` - Assert on execution duration
- `file` - Assert on file content

## Example

```hone
ASSERT stdout contains "success"
ASSERT exit_code == 0
```
"#
        .to_string()
    }

    fn run_keyword_doc(&self) -> String {
        r#"# RUN

Execute a shell command.

## Syntax

```hone
RUN <command>
RUN "<name>" <command>
```

## Example

```hone
RUN echo "Hello, World!"
RUN "build" cargo build --release
```

Named runs allow referencing specific command results in assertions.
"#
        .to_string()
    }

    fn stdout_assertion_doc(&self) -> String {
        r#"# stdout

Assert on standard output content.

## Predicates

- `contains "text"` - Output contains the text
- `matches /regex/` - Output matches the regex pattern
- `equals "text"` or `== "text"` - Output equals the text exactly
- `!= "text"` - Output does not equal the text

## Example

```hone
expect stdout {
  contains "success"
}
expect stdout {
  matches /^OK/
}
```
"#
        .to_string()
    }

    fn stdout_raw_assertion_doc(&self) -> String {
        r#"# stdout_raw

Assert on raw standard output content (without ANSI escape sequences stripped).

## Predicates

Same as `stdout`:
- `contains "text"`
- `matches /regex/`
- `equals "text"` or `== "text"`
- `!= "text"`

## Example

```hone
expect stdout_raw {
  contains "\x1b[32m"
}
```
"#
        .to_string()
    }

    fn stderr_assertion_doc(&self) -> String {
        r#"# stderr

Assert on standard error content.

## Predicates

- `contains "text"` - Error output contains the text
- `matches /regex/` - Error output matches the regex pattern
- `equals "text"` or `== "text"` - Error output equals the text exactly
- `!= "text"` - Error output does not equal the text

## Example

```hone
expect stderr {
  contains "error"
}
```
"#
        .to_string()
    }

    fn exitcode_assertion_doc(&self) -> String {
        r#"# exitcode

Assert on command exit code.

## Syntax

```hone
expect exitcode <number>
expect exitcode <operator> <number>
```

## Operators

- `==` or no operator - Equals
- `!=` - Not equals
- `<` - Less than
- `<=` - Less than or equal
- `>` - Greater than
- `>=` - Greater than or equal

## Example

```hone
expect exitcode 0
expect exitcode != 0
expect exitcode > 0
```
"#
        .to_string()
    }

    fn duration_assertion_doc(&self) -> String {
        r#"# duration

Assert on command execution duration.

## Syntax

```hone
expect duration <operator> <time>
```

## Time Units

- `ms` - Milliseconds
- `s` - Seconds

## Operators

- `<` - Less than
- `<=` - Less than or equal
- `>` - Greater than
- `>=` - Greater than or equal

## Example

```hone
expect duration < 100ms
expect duration <= 2s
```
"#
        .to_string()
    }

    fn file_assertion_doc(&self) -> String {
        r#"# file

Assert on file content or existence.

## Predicates

- `exists` - File exists
- `contains "text"` - File contains the text
- `matches /regex/` - File content matches the regex pattern
- `equals "text"` or `== "text"` - File content equals the text exactly
- `!= "text"` - File content does not equal the text

## Example

```hone
expect file "output.txt" {
  exists
}
expect file "config.json" {
  contains "port"
}
```
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word_at_position() {
        let provider = HoverProvider::new();

        // Test extracting TEST
        let (word, start, end) = provider
            .extract_word_at_position("TEST \"name\"", 2)
            .unwrap();
        assert_eq!(word, "TEST");
        assert_eq!(start, 0);
        assert_eq!(end, 4);

        // Test extracting ASSERT
        let (word, _, _) = provider
            .extract_word_at_position("  ASSERT stdout contains \"ok\"", 5)
            .unwrap();
        assert_eq!(word, "ASSERT");

        // Test extracting stdout
        let (word, _, _) = provider
            .extract_word_at_position("  ASSERT stdout contains \"ok\"", 10)
            .unwrap();
        assert_eq!(word, "stdout");
    }

    #[test]
    fn test_get_documentation_keywords() {
        let provider = HoverProvider::new();

        assert!(provider.get_documentation("TEST").is_some());
        assert!(provider.get_documentation("RUN").is_some());
        assert!(provider.get_documentation("ASSERT").is_some());
    }

    #[test]
    fn test_get_documentation_assertions() {
        let provider = HoverProvider::new();

        assert!(provider.get_documentation("stdout").is_some());
        assert!(provider.get_documentation("stderr").is_some());
        assert!(provider.get_documentation("exitcode").is_some());
        assert!(provider.get_documentation("duration").is_some());
        assert!(provider.get_documentation("file").is_some());
    }

    #[test]
    fn test_get_documentation_unknown() {
        let provider = HoverProvider::new();

        assert!(provider.get_documentation("unknown").is_none());
        assert!(provider.get_documentation("random").is_none());
    }
}
