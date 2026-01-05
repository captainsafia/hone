# Hone

Hone is a CLI integration test runner for command-line applications, inspired by [Hurl](https://hurl.dev).

Write tests in a simple, line-oriented DSL that executes real shell commands and asserts on output, exit codes, file contents, and timing.

## Features

- File assertions (exists, contains, matches, equals) and duration assertions
- Persistent shell sessions (`cd`, variables, and state carry across commands)
- Separate stdout/stderr capture
- ANSI escape code stripping (or assert on raw output with `stdout.raw`)

## Installation

### Using the installer (recommended)

```sh
# Install latest release
curl https://i.captainsafia.sh/captainsafia/hone | sh

# Install to /usr/local/bin (may require sudo)
curl "https://i.captainsafia.sh/captainsafia/hone?move=1" | sh

# Install a specific version
curl https://i.captainsafia.sh/captainsafia/hone/v1.0.0 | sh

# Install latest prerelease
curl https://i.captainsafia.sh/captainsafia/hone/preview | sh
```

## Quick Start

Create a test file `example.hone`:

```
#! shell: /bin/bash

TEST "echo works"

RUN echo "Hello, World!"
ASSERT exit_code == 0
ASSERT stdout contains "Hello"

TEST "file creation"

RUN echo "test content" > output.txt
ASSERT file "output.txt" exists
ASSERT file "output.txt" contains "test content"

TEST "error handling"

RUN ls nonexistent
ASSERT exit_code != 0
ASSERT stderr contains "No such file"
```

Run the tests:

```sh
hone run example.hone
```

## DSL Reference

### Pragmas

File-level configuration at the top of the file:

```
#! shell: /bin/zsh
#! env: MY_VAR=value
#! timeout: 60s
```

### TEST Blocks

Group related commands and assertions:

```
TEST "my test name"
```

Tests share the same shell session but have isolated ENV variables.

### RUN Commands

Execute shell commands:

```
RUN mycli init
RUN build: mycli build --release
```

Named runs (like `build:`) can be referenced in assertions.

### ENV Statements

Set environment variables for a test:

```
ENV API_KEY=test123
ENV DEBUG=true
```

### Assertions

#### Output Assertions

```
ASSERT stdout contains "expected text"
ASSERT stdout == "exact match"
ASSERT stdout != "not this"
ASSERT stdout matches /pattern/i
ASSERT stderr contains "error"
ASSERT stdout.raw contains "\x1b[32m"
```

#### Exit Code Assertions

```
ASSERT exit_code == 0
ASSERT exit_code != 0
```

#### Timing Assertions

```
ASSERT duration < 500ms
ASSERT duration <= 2s
```

#### File Assertions

```
ASSERT file "path/to/file.txt" exists
ASSERT file "output.json" contains "success"
ASSERT file "config.yaml" matches /version: \d+/
ASSERT file "exact.txt" == "exact content"
```

#### Named Target Assertions

Reference a specific RUN command:

```
RUN build: make build
RUN test: make test
ASSERT build.exit_code == 0
ASSERT build.duration < 30s
ASSERT test.stdout contains "PASSED"
```

### Comments

```
# This is a comment
```

## CLI Usage

```sh
# Run a single test file
hone run tests/integration.hone

# Run all test files matching a pattern
hone run 'tests/*.hone'

# Show version
hone --version

# Show help
hone --help
```

## Example Test Files

See the [examples/](examples/) directory for more comprehensive examples:

- `basic.hone` — Simple command and file assertions
- `assertions.hone` — All assertion types
- `environment.hone` — Environment variable handling
- `filesystem.hone` — File system assertions
- `shell-state.hone` — Shell state persistence
- `strings.hone` — String literal and escape handling

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All tests passed |
| 1 | One or more tests failed |

## Tips

### Testing exit codes

To test a command that exits the shell (like `exit 42`), wrap it in a subshell:

```
RUN (exit 42)
ASSERT exit_code == 42
```

### Literal vs escaped strings

- Single quotes are literal: `'hello\n'` contains backslash-n
- Double quotes support escapes: `"hello\n"` contains a newline

### Shell state

Commands share shell state within a file:

```
RUN cd /tmp
RUN pwd
ASSERT stdout contains "/tmp"
```

## Development Setup

```sh
# Clone the repository
git clone https://github.com/captainsafia/hone.git
cd hone

# Install dependencies
bun install

# Run tests
bun test

# Run integration tests
bun run test:integration

# Type check
bun run typecheck

# Lint
bun run lint

# Build executable
bun run build:compile
```