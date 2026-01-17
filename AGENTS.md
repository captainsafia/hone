# Hone Development Guide for AI Agents

This project is a Rust-based CLI tool for integration testing of command-line applications.

## Build & Test Commands

- Use `cargo build` to build the project (not `npm` or `bun`)
- Use `cargo test` for unit tests
- Use `cargo run -- <args>` to run the CLI during development
- Use `cargo run --release -- <args>` for optimized builds
- Use `cargo clippy` for linting
- Use `cargo fmt` for formatting
- **Always run `cargo clippy` and `cargo fmt --check` before considering code changes complete** to catch warnings and ensure code quality

## Running Integration Tests

The project tests itself using `.hone` test files:

```sh
# Run all integration tests
cargo run --release -- tests/integration/*.hone

# Run example tests
cargo run --release -- examples/*.hone

# Run a specific test file
cargo run --release -- tests/integration/exit-codes.hone
```

## Project Structure

```
hone/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── parser/              # DSL parser
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   ├── lexer.rs
│   │   ├── ast.rs
│   │   └── errors.rs
│   ├── runner/              # Test execution
│   │   ├── mod.rs
│   │   ├── executor.rs
│   │   ├── shell.rs
│   │   ├── sentinel.rs
│   │   └── reporter.rs
│   ├── assertions/          # Assertion implementations
│   │   ├── mod.rs
│   │   ├── output.rs
│   │   ├── exitcode.rs
│   │   ├── timing.rs
│   │   └── filesystem.rs
│   └── utils/
│       ├── mod.rs
│       └── ansi.rs
├── tests/integration/       # .hone integration tests
├── examples/                # .hone example files
├── Cargo.toml
└── README.md
```

## Key Implementation Details

- **Async Runtime**: Uses Tokio for async/await
- **Shell Execution**: Spawns shell processes and tracks output via PTY
- **Sentinel Protocol**: Wraps commands with markers to detect completion
- **Multi-file Tests**: Each test file gets its own shell session
- **Error Handling**: Uses anyhow for error propagation

## Style Guidelines

### General Principles

- **Avoid excessive comments**: Code should be self-documenting through clear naming
- Only add comments for non-obvious logic, complex algorithms, or important context
- Avoid inline comments that restate what the code does (e.g., `// Loop through items`)
- Prefer doc comments (`///`) for public APIs over inline comments

### Idiomatic Rust

- Use `snake_case` for functions, methods, variables, and modules
- Use `PascalCase` for types, traits, and enum variants
- Use `SCREAMING_SNAKE_CASE` for constants and statics
- Prefer `impl Trait` over explicit generics when possible
- Use `?` operator for error propagation instead of explicit `match`
- Prefer iterators and combinators over explicit loops when clearer
- Use `#[must_use]` for functions where ignoring the return value is likely a bug
- Derive common traits (`Debug`, `Clone`, `PartialEq`) when appropriate
- Use `Self` to refer to the implementing type in `impl` blocks
- Prefer `&str` over `String` for function parameters when ownership isn't needed
- Use `Option` and `Result` idiomatically; avoid sentinel values

### Source Control Conventions

- All commits must follow the Convential Commit format.
- Commits MUST NOT be co-authored by the agent.

### Code Organization

- Keep modules focused and cohesive
- Use `mod.rs` to re-export public items from submodules
- Place tests in the same file using `#[cfg(test)]` modules
- Group related imports; separate std, external crates, and local imports
- Use `crate::` for all internal imports; avoid `super::` (exception: `use super::*` in test modules is acceptable)

## Testing Approach

1. Unit tests in Rust using `#[test]` annotations
2. Integration tests using `.hone` files that test the compiled binary
3. Examples serve as both documentation and smoke tests

## Release Process

- Binaries are cross-compiled for multiple platforms using `cross`
- GitHub Actions builds for Linux (x64, ARM64) and macOS (x64, ARM64)
- Version is managed in `Cargo.toml`