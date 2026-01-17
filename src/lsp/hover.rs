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
            "@test" => Some(self.test_keyword_doc()),
            "@setup" => Some(self.setup_keyword_doc()),
            "expect" => Some(self.expect_keyword_doc()),
            "run" => Some(self.run_keyword_doc()),
            "env" => Some(self.env_keyword_doc()),
            "stdout" => Some(self.stdout_assertion_doc()),
            "stdout_raw" => Some(self.stdout_raw_assertion_doc()),
            "stderr" => Some(self.stderr_assertion_doc()),
            "exitcode" => Some(self.exitcode_assertion_doc()),
            "duration" => Some(self.duration_assertion_doc()),
            "file" => Some(self.file_assertion_doc()),
            _ => None,
        }
    }

    fn test_keyword_doc(&self) -> String {
        r#"# @test

Define a test block that contains commands to run and assertions to verify.

## Syntax

```hone
@test "test name" {
  run command
  expect assertion
}
```

## Example

```hone
@test "should list files" {
  run ls -la
  expect stdout {
    contains "README.md"
  }
  expect exitcode 0
}
```
"#
        .to_string()
    }

    fn setup_keyword_doc(&self) -> String {
        r#"# @setup

Define a setup block that runs before all tests. Used to prepare the test environment.

## Syntax

```hone
@setup {
  run command
}
```

## Example

```hone
@setup {
  run mkdir -p test-dir
  env TEST_VAR=value
}
```
"#
        .to_string()
    }

    fn expect_keyword_doc(&self) -> String {
        r#"# expect

Define an assertion to verify test behavior. Must be inside a `@test` block.

## Syntax

```hone
expect <assertion-type> <predicate>
```

## Assertion Types

- `stdout` - Assert on standard output
- `stderr` - Assert on standard error
- `exitcode` - Assert on exit code
- `duration` - Assert on execution duration
- `file` - Assert on file content

## Example

```hone
expect stdout {
  contains "success"
}
expect exitcode 0
```
"#
        .to_string()
    }

    fn run_keyword_doc(&self) -> String {
        r#"# run

Execute a shell command. Can be used in `@setup` or `@test` blocks.

## Syntax

```hone
run <command>
run "<name>" <command>
```

## Example

```hone
run echo "Hello, World!"
run "build" cargo build --release
```

Named runs allow referencing specific command results in assertions.
"#
        .to_string()
    }

    fn env_keyword_doc(&self) -> String {
        r#"# env

Set an environment variable for test execution. Only valid in `@setup` blocks.

## Syntax

```hone
env KEY=value
```

## Example

```hone
@setup {
  env PATH=/custom/bin:$PATH
  env DEBUG=true
}
```
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

        // Test extracting @test
        let (word, start, end) = provider
            .extract_word_at_position("@test \"name\" {", 2)
            .unwrap();
        assert_eq!(word, "@test");
        assert_eq!(start, 0);
        assert_eq!(end, 5);

        // Test extracting expect
        let (word, _, _) = provider
            .extract_word_at_position("  expect stdout {", 5)
            .unwrap();
        assert_eq!(word, "expect");

        // Test extracting stdout
        let (word, _, _) = provider
            .extract_word_at_position("  expect stdout {", 10)
            .unwrap();
        assert_eq!(word, "stdout");
    }

    #[test]
    fn test_get_documentation_keywords() {
        let provider = HoverProvider::new();

        assert!(provider.get_documentation("@test").is_some());
        assert!(provider.get_documentation("@setup").is_some());
        assert!(provider.get_documentation("expect").is_some());
        assert!(provider.get_documentation("run").is_some());
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
