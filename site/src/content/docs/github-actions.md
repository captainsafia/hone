---
title: GitHub Actions
description: Run Hone tests in CI/CD pipelines
order: 6
---

Run Hone tests in your GitHub Actions CI/CD pipelines with the official Hone GitHub Action.

## Quick Start

Add the Hone GitHub Action to your workflow to automatically run integration tests on every push or pull request:

```yaml
name: Hone Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run Hone tests
        uses: captainsafia/hone/hone-github-action@main
        with:
          tests: 'tests/*.hone'
```

## Inputs

The action accepts the following inputs:

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `version` | Version of Hone to install (e.g., `v1.0.0`) | No | `preview` |
| `tests` | Test files or patterns to run (e.g., `tests/*.hone`) | Yes | - |

## Examples

### Run all tests with latest preview

By default, the action installs the latest preview release of Hone:

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    tests: 'tests/**/*.hone'
```

### Run specific tests with a pinned version

Pin a specific version for reproducible CI builds:

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    version: 'v1.2.0'
    tests: 'tests/integration/api.hone tests/integration/cli.hone'
```

### Run tests from the examples directory

You can run any test files in your repository:

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    tests: 'examples/*.hone'
```

## Complete Workflow Example

Here's a complete example that builds an application and runs Hone integration tests:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      # Build your CLI or application first
      - name: Build application
        run: |
          cargo build --release
          echo "$PWD/target/release" >> $GITHUB_PATH
      
      # Run Hone integration tests
      - name: Run integration tests
        uses: captainsafia/hone/hone-github-action@main
        with:
          tests: 'tests/**/*.hone'
```

> **Note**: Make sure the commands you're testing in your `.hone` files are available in your CI environment. You may need to build your application or install dependencies before running the Hone tests.

---

**Next Steps**: Learn more about writing tests in the [DSL Syntax](/docs/dsl-syntax) reference, or explore [example tests](/examples).
