# Update Checks and Self-Update Feature Specification

## Overview

Add automatic update checking and a self-update command to the Hone CLI, using the `relnotify` crate for GitHub release monitoring and the `i.safia.sh` installer service for performing updates.

## Dependencies

Add to `Cargo.toml`:
```toml
relnotify = "1"
reqwest = { version = "0.12", features = ["blocking"] }
```

## Feature Components

### 1. Automatic Update Checks

#### Trigger Conditions
- Update checks run on **every command invocation**
- Checks are performed **asynchronously in the background** (fire-and-forget)
- Notifications appear on the **next run** if the check completed after the previous command finished

#### Suppression Conditions
- **Environment variable**: Set `HONE_NO_UPDATE_CHECK=1` to disable all automatic checks
- **Non-TTY output**: Suppress notifications when stdout is not a terminal (piped output, CI logs, etc.)

#### Cache Configuration
- **Location**: `~/.hone/update-cache.json`
- **Duration**: 24 hours between checks
- Create `~/.hone/` directory if it doesn't exist

#### Error Handling
- **Silent failure**: All update check errors are swallowed completely
- Never interrupt or delay the user's workflow due to network issues, API rate limits, or other failures

#### Notification Format
Single subtle line printed after normal command output:
```
Update available: v1.2.0 → v1.3.0. Run `hone update` to install.
```

### 2. Self-Update Command

#### CLI Structure

Add new subcommand to `Commands` enum:
```rust
/// Update Hone to a newer version
Update {
    /// Target version to install (default: latest)
    #[arg(value_name = "VERSION")]
    version: Option<String>,
}
```

#### Usage Examples
```bash
hone update          # Update to latest stable release
hone update 1.3.0    # Update to specific version
```

#### Update Mechanism

1. **Download installer script** from `https://i.safia.sh/captainsafia/hone` (or `https://i.safia.sh/captainsafia/hone/<version>` for specific versions)
2. **Save script to temp file** (allows inspection, safer than direct pipe)
3. **Execute script** via shell subprocess
4. **Show full output** from installer script as it runs (verbose by default)
5. **Exit silently** with success code on completion

#### Platform Support
- **Supported**: Linux and macOS (Unix shell script)

#### Confirmation
- **No confirmation required** - running `hone update` is sufficient consent

#### Package Manager Detection
- **None** - do not attempt to detect if hone was installed via Homebrew, cargo, etc.
- Always proceed with the installer-based update

### 3. Implementation Details

#### New Module Structure
```
src/
├── update/
│   ├── mod.rs          # Module exports
│   ├── check.rs        # Background update checking logic
│   └── self_update.rs  # Self-update command implementation
```

#### Update Check Flow (check.rs)
```rust
pub async fn check_for_update() -> Option<String> {
    // 1. Check if HONE_NO_UPDATE_CHECK is set
    // 2. Check if stdout is a TTY
    // 3. Initialize ReleaseNotifier with:
    //    - repo: "captainsafia/hone"
    //    - cache_path: ~/.hone/update-cache.json
    //    - check_interval: 24 hours
    // 4. Call check_version() with current version from Cargo.toml
    // 5. Return Some(new_version) if update available, None otherwise
    // 6. Swallow all errors silently
}

pub fn spawn_update_check() {
    // Spawn tokio task for fire-and-forget check
    // Store result in cache file for next run
}

pub fn show_update_notification_if_available() {
    // Read cached result
    // If update available and stdout is TTY, print notification
}
```

#### Self-Update Flow (self_update.rs)
```rust
pub async fn perform_update(version: Option<String>) -> anyhow::Result<()> {
    // 1. Construct installer URL:
    //    - Latest: https://i.safia.sh/captainsafia/hone
    //    - Specific: https://i.safia.sh/captainsafia/hone/{version}
    // 2. Download script to temp file using reqwest
    // 3. Execute: sh <temp_file>
    // 4. Stream output to stdout
    // 5. Exit with installer's exit code
}
```

#### Integration Points

**main.rs changes:**
```rust
// At start of main(), before command dispatch:
update::spawn_update_check();

// At end of main(), after command completes:
update::show_update_notification_if_available();

// Add to Commands enum:
Update {
    version: Option<String>,
}

// Add match arm:
Some(Commands::Update { version }) => {
    update::perform_update(version).await?;
    Ok(())
}
```

### 4. Configuration Summary

| Setting | Value |
|---------|-------|
| GitHub repo | `captainsafia/hone` |
| Installer URL | `https://i.safia.sh/captainsafia/hone` |
| Cache location | `~/.hone/update-cache.json` |
| Cache duration | 24 hours |
| Disable env var | `HONE_NO_UPDATE_CHECK=1` |
| Platforms | Linux, macOS (Unix only) |

### 5. Example Outputs

#### Update Available Notification
```
$ hone tests/*.hone
Running 5 tests...
✓ All tests passed

Update available: v1.0.0 → v1.1.0. Run `hone update` to install.
```

#### Self-Update Output
```
$ hone update
Downloading installer script...
Installing hone v1.1.0...
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
100  5.2M  100  5.2M    0     0  12.3M      0 --:--:-- --:--:-- --:--:-- 12.3M
Installed hone to ~/.hone/bin/hone
```

#### Update to Specific Version
```
$ hone update 1.0.0
Downloading installer script for v1.0.0...
Installing hone v1.0.0...
...
```
