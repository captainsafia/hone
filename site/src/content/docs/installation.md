---
title: Installation
description: Install Hone on your system
order: 1
---

Hone is distributed as a single binary with no runtime dependencies. Choose your preferred installation method below.

> Hone is currently in preview. APIs and features may change before the stable release.

## Using the Installer (Recommended)

The easiest way to install Hone is using the install script. It automatically detects your platform and downloads the appropriate binary.

```bash
# Install latest preview
curl https://i.safia.sh/captainsafia/hone/preview | sh
```

### Specific Version

To install a specific version, append the version tag to the URL:

```bash
# Install a specific version
curl https://i.safia.sh/captainsafia/hone/v1.0.0-preview.e615276 | sh
```

### Stable Releases

Once Hone reaches a stable release, you can install the latest stable version:

```bash
# Install latest stable release (when available)
curl https://i.safia.sh/captainsafia/hone | sh
```

## Building from Source

If you have Rust installed, you can build Hone from source using Cargo:

```bash
# Build and install from source
cargo install --git https://github.com/captainsafia/hone
```

## Verify Installation

After installation, verify that Hone is available in your PATH:

```bash
# Verify installation
hone --version
```

## Supported Platforms

Hone provides pre-built binaries for the following platforms:

- Linux (x64, ARM64)
- macOS (x64, ARM64 / Apple Silicon)

---

**Next Steps**: Ready to write your first test? Head over to the [Getting Started](/docs/getting-started) guide.
