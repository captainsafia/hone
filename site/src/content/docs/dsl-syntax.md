---
title: DSL Syntax
description: Complete reference for the Hone DSL
order: 3
---

Hone uses a simple, line-oriented domain-specific language (DSL) for writing tests. This page covers all available syntax elements.

## Pragmas

Pragmas are file-level configuration directives that appear at the top of the file. They start with `#!`.

```hone
#! shell: /bin/zsh
#! env: MY_VAR=value
#! timeout: 60s
```

| Pragma | Description |
|--------|-------------|
| `shell:` | Path to the shell executable (default: `/bin/sh`) |
| `env:` | Set environment variable for all tests in the file |
| `timeout:` | Maximum execution time per command (e.g., `30s`, `5m`) |

## TEST Blocks

The `TEST` keyword starts a new test block. Tests share the same shell session but have isolated ENV variables.

```hone
TEST "descriptive test name"

RUN some-command
ASSERT exit_code == 0
```

## RUN Commands

The `RUN` keyword executes a shell command. Commands can optionally be named for later reference in assertions.

```hone
# Basic command
RUN mycli init

# Named run for later reference
RUN build: mycli build --release

# Chain commands
RUN cd src && make build
```

## ENV Statements

The `ENV` keyword sets environment variables for subsequent commands within the current test.

```hone
# Set for subsequent commands in this test
ENV API_KEY=test123
ENV DEBUG=true

RUN mycli connect
```

## ASSERT Statements

The `ASSERT` keyword verifies conditions about command output, exit codes, timing, or files.

```hone
# Output assertions
ASSERT stdout contains "success"
ASSERT stdout not contains "error"
ASSERT stderr == ""
ASSERT stdout matches /version \d+/

# Exit code
ASSERT exit_code == 0
ASSERT exit_code != 1

# Timing
ASSERT duration < 5s

# Files
ASSERT file "output.txt" exists
ASSERT file "config.json" contains "enabled"
ASSERT file "config.json" not contains "disabled"
```

## Named Runs

Named runs allow you to reference specific commands in assertions using dot notation:

```hone
RUN compile: gcc main.c -o main
RUN execute: ./main

ASSERT compile.exit_code == 0
ASSERT compile.duration < 10s
ASSERT execute.stdout contains "Hello"
```

## Comments

Lines starting with `#` (but not `#!`) are comments:

```hone
# This is a comment
TEST "example"

# Comments can appear anywhere
RUN echo "test"  # But not inline
```

## String Literals

Hone supports both single and double quoted strings:

- **Double quotes** — Support escape sequences: `"hello\nworld"`
- **Single quotes** — Literal strings: `'hello\n'` contains backslash-n

---

**Next Steps**: Learn about all available assertion types in the [Assertions](/docs/assertions) reference.
