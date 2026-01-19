# Hone

Hone is a CLI integration test runner for command-line applications, inspired by [Hurl](https://hurl.dev).

Write tests in a simple, line-oriented DSL that executes real shell commands and asserts on output, exit codes, file contents, and timing.

📚 **[Read the documentation](https://hone.safia.dev)** for installation instructions, guides, and the full DSL reference.

## Features

- Simple, readable test syntax
- Real shell execution with persistent state
- Rich assertions for stdout, stderr, exit codes, files, and timing
- Built-in Language Server (LSP) for editor integration
- Single binary with no runtime dependencies

## Quick Example

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
```

Run the tests:

```sh
hone run example.hone
```

## Installation

```sh
curl https://i.safia.sh/captainsafia/hone | sh
```

See the [installation docs](https://hone.safia.dev/docs/installation) for more options.

## Documentation

- [Getting Started](https://hone.safia.dev/docs/getting-started) — Write your first test
- [DSL Syntax](https://hone.safia.dev/docs/dsl-syntax) — Full language reference
- [Assertions](https://hone.safia.dev/docs/assertions) — All assertion types
- [Examples](https://hone.safia.dev/examples) — Real-world test examples

## Editor Setup

Hone includes a built-in Language Server. Configure your editor automatically:

```sh
hone setup vscode
hone setup neovim
```

Run `hone setup` to see all supported editors.

## Development

```sh
# Build
cargo build

# Run tests
cargo test

# Run integration tests
cargo run -- tests/integration/*.hone
```

## License

MIT