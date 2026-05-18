# Contributing to Hone

Thanks for your interest in improving Hone! This guide covers the basics
of building, testing, and submitting changes.

## Quick Start

Hone is a Rust project built with Cargo. You will need a recent stable
Rust toolchain (see `Cargo.toml` for the required edition).

```sh
# Build
cargo build

# Unit tests
cargo test

# Run the CLI locally
cargo run -- tests/integration/*.hone

# Optimized build
cargo run --release -- tests/integration/*.hone
```

## Project Layout

The Rust sources live under `src/`:

- `parser/` — Lexer, parser, and AST for the `.hone` DSL.
- `runner/` — Test executor, shell session, sentinel protocol, reporter.
- `assertions/` — Implementations of stdout/stderr/exit code/timing/file assertions.
- `lsp/` — Language Server implementation (completion, hover, diagnostics, etc.).
- `setup/` — `hone setup` editor integrations (VS Code, Neovim, Vim).
- `update/` — Self-update and update-check logic.
- `watcher/` — `--watch` mode file watcher and scheduler.

Integration tests written in the DSL live under `tests/integration/`,
and end-to-end examples live under `examples/`.

## Running the Test Suites

Run the Rust unit tests with Cargo, and exercise the compiled binary
against the `.hone` integration tests:

```sh
cargo test
cargo run --release -- tests/integration/*.hone
cargo run --release -- examples/*.hone
```

## Coding Style

Follow the conventions documented in `AGENTS.md`, including the
idiomatic Rust guidelines (naming, error propagation with `?`, preferring
`&str` parameters, etc.). Before opening a pull request:

```sh
cargo fmt --check
cargo clippy -- -D warnings
```

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) for all
commit messages (e.g. `feat: add file size assertion`, `fix(parser): ...`,
`docs: ...`). Do not add a `Co-Authored-By` trailer for AI assistants.

## Submitting a Pull Request

1. Fork the repository and create a topic branch from `main`.
2. Open the PR as a **draft** while you iterate. Link the related issue
   in the description when one exists.
3. Keep the change focused and include tests where practical.
4. Ensure CI is green before marking the PR ready for review.

## Reporting Bugs

Please file bugs and feature requests through the project's
[GitHub issue tracker](https://github.com/captainsafia/hone/issues),
including a minimal `.hone` reproduction and the output of `hone --version`.
