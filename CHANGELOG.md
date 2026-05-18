# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-05-18

### Added

- DSL parser with line- and column-aware diagnostics.
- Test runner backed by a sentinel-protocol shell session for accurate
  command boundary and exit-code tracking.
- Assertion suite covering `stdout`, `stderr`, `exit_code`, `duration`,
  and `file` checks.
- Language Server (LSP) implementation providing completion, hover,
  diagnostics, semantic tokens, and formatting.
- `hone setup` command for editor integration (VS Code, Neovim, Vim).
- `hone update` self-update flow for upgrading the installed binary.
- `--watch` mode that re-runs affected tests on file changes.
- JSON output format following the CTRF schema for CI integrations.

[Unreleased]: https://github.com/captainsafia/hone/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/captainsafia/hone/releases/tag/v1.0.0
