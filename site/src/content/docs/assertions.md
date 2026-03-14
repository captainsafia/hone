---
title: Assertions
description: Reference for all Hone assertion types
order: 4
---

Hone provides a rich set of assertions for verifying command output, exit codes, timing, and file contents.

## Output Assertions

### stdout

Assert on the standard output of the most recent (or named) command:

```hone
# Contains - check if output includes text
ASSERT stdout contains "success"
ASSERT stdout contains "Hello, World!"

# Not contains - check output does NOT include text
ASSERT stdout not contains "error"

# Equals - exact match
ASSERT stdout == "exact output"
ASSERT stdout != "not this"

# Regex - pattern matching
ASSERT stdout matches /version \d+\.\d+\.\d+/
ASSERT stdout matches /hello world/i  # case insensitive
```

### stderr

Assert on the standard error output:

```hone
# Same operators work for stderr
ASSERT stderr contains "warning"
ASSERT stderr not contains "fatal"
ASSERT stderr == ""
ASSERT stderr matches /error: .*/
```

### Raw Output

By default, ANSI escape codes are stripped from output. Use `.raw` to preserve them:

```hone
# Access raw output with ANSI codes preserved
ASSERT stdout.raw contains "\x1b[32m"
```

## Exit Code Assertions

Verify the exit status of commands:

```hone
# Check exit status
ASSERT exit_code == 0
ASSERT exit_code != 0
ASSERT exit_code == 42
```

## Duration Assertions

Check how long a command took to execute. Supports `ms`, `s`, `m`, and `h` units:

```hone
# Time constraints
ASSERT duration < 500ms
ASSERT duration <= 2s
ASSERT duration < 1m
```

## File Assertions

### Existence

Check if a file exists:

```hone
# Check file existence
ASSERT file "output.txt" exists
ASSERT file "/tmp/cache.json" exists
```

### Content

Assert on file contents using the same operators as stdout:

```hone
# Check file contents
ASSERT file "config.yaml" contains "enabled: true"
ASSERT file "config.yaml" not contains "debug: true"
ASSERT file "output.json" == '{"status": "ok"}'
ASSERT file "version.txt" matches /v\d+\.\d+/
```

## Named Run Targets

When you have multiple RUN commands, use named runs to assert on specific ones:

```hone
RUN compile: gcc main.c -o main
RUN test: ./main --test

# Reference specific commands
ASSERT compile.exit_code == 0
ASSERT compile.stdout contains "compiled"
ASSERT compile.duration < 30s

ASSERT test.exit_code == 0
ASSERT test.stdout contains "PASSED"
```

## Assertion Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `contains` | Substring match | `stdout contains "ok"` |
| `not contains` | Negative substring match | `stdout not contains "error"` |
| `==` | Exact equality | `exit_code == 0` |
| `!=` | Not equal | `stderr != ""` |
| `matches` | Regex pattern | `stdout matches /v\d+/` |
| `exists` | File exists | `file "x.txt" exists` |
| `<` | Less than | `duration < 5s` |
| `<=` | Less than or equal | `duration <= 10s` |

---

**Next Steps**: Learn about configuration options in the [Configuration](/docs/configuration) guide.
