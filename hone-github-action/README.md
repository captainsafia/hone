# Hone GitHub Action

A GitHub Action to install the [Hone CLI](https://hone.safia.dev) and run integration tests.

## Usage

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

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `version` | Version of Hone to install (e.g., `v1.0.0`). | No | `preview` |
| `tests` | Test files or patterns to run (e.g., `tests/*.hone`). | Yes | - |

## Examples

### Run all tests with latest preview

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    tests: 'tests/**/*.hone'
```

### Run specific tests with a pinned version

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    version: 'v1.2.0'
    tests: 'tests/integration/api.hone tests/integration/cli.hone'
```

### Run tests from the examples directory

```yaml
- uses: captainsafia/hone/hone-github-action@main
  with:
    tests: 'examples/*.hone'
```

## License

MIT
