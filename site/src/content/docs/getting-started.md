---
title: Getting Started
description: Write and run your first Hone test
order: 2
---

Learn how to write and run your first Hone test in just a few minutes.

## Your First Test

Create a new file called `my-test.hone` with the following content:

```hone
#! shell: /bin/bash

TEST "my first test"

RUN echo "Hello, World!"
ASSERT exit_code == 0
ASSERT stdout contains "Hello"
```

Let's break down what each line does:

- `#! shell: /bin/bash` — Sets the shell to use for running commands
- `TEST "my first test"` — Declares a test with a descriptive name
- `RUN echo "Hello, World!"` — Executes a shell command
- `ASSERT exit_code == 0` — Verifies the command succeeded
- `ASSERT stdout contains "Hello"` — Checks the output contains expected text

## Running Tests

Run your test using the Hone CLI:

```bash
hone run my-test.hone
```

You should see output like:

```
Running my-test.hone...
✓ my first test

1 test passed
```

## Multiple Tests

A test file can contain multiple tests. Each test runs in the same shell session, so state like working directory and variables persist:

```hone
#! shell: /bin/bash

TEST "check version"

RUN mycli --version
ASSERT exit_code == 0
ASSERT stdout matches /v\d+\.\d+\.\d+/

TEST "help command"

RUN mycli --help
ASSERT exit_code == 0
ASSERT stdout contains "Usage:"

TEST "create file"

RUN mycli create output.txt
ASSERT exit_code == 0
ASSERT file "output.txt" exists
```

## Running Multiple Files

Hone can run multiple test files at once:

```bash
# Run a single file
hone run tests/basic.hone

# Run multiple files
hone run tests/*.hone

# Run all .hone files in a directory
hone run tests/
```

## Environment Variables

Use the `ENV` statement to set environment variables for your tests:

```hone
TEST "with environment variables"

ENV API_KEY=test123
ENV DEBUG=true
RUN mycli connect
ASSERT exit_code == 0
```

---

**Next Steps**: Learn about all available commands and syntax in the [DSL Syntax](/docs/dsl-syntax) reference.
