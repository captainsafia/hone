# Security Policy

## Supported Versions

The `1.0.x` series receives security fixes. Older pre-release builds are
not supported; please upgrade to the latest `1.0.x` release before
reporting an issue.

## Reporting a Vulnerability

Please report suspected security vulnerabilities by email to
<INSERT SECURITY CONTACT>. Do **not** open a public GitHub issue for
security problems — disclosing details publicly before a fix is
available puts users at risk.

We aim to acknowledge new reports within a few business days and target
a coordinated disclosure window of **90 days** from the initial report.
We will work with reporters to confirm the issue, prepare a fix, and
agree on a public disclosure date.

## Scope

In scope:

- The `hone` binary (CLI entry point and subcommands).
- The DSL parser and runner.
- The bundled Language Server (LSP).

Out of scope:

- Third-party shells invoked by Hone (bash, zsh, etc.).
- User-authored test code executed by Hone.
- The GitHub Action wrapper, which lives in its own repository and
  follows its own security and release conventions.

## Known Considerations

- `hone update` currently downloads and runs an installer script over
  HTTPS without verifying a release signature. Hardening this flow
  (signature verification and pinned checksums) is tracked as a known
  limitation; treat the self-update path accordingly in sensitive
  environments.
