use std::collections::HashSet;
use std::env;
use std::path::Path;

/// Common shell commands that are frequently used
const COMMON_COMMANDS: &[(&str, &str)] = &[
    ("ls", "List directory contents"),
    ("cd", "Change directory"),
    ("cat", "Concatenate and print files"),
    ("echo", "Display a line of text"),
    ("grep", "Search for patterns in files"),
    ("find", "Search for files in a directory hierarchy"),
    ("sed", "Stream editor for filtering and transforming text"),
    ("awk", "Pattern scanning and text processing"),
    ("cut", "Remove sections from each line of files"),
    ("sort", "Sort lines of text files"),
    ("uniq", "Report or omit repeated lines"),
    ("head", "Output the first part of files"),
    ("tail", "Output the last part of files"),
    ("wc", "Print newline, word, and byte counts"),
    ("diff", "Compare files line by line"),
    ("patch", "Apply a diff file to an original"),
    ("tar", "Archive files"),
    ("gzip", "Compress or expand files"),
    ("gunzip", "Expand compressed files"),
    ("zip", "Package and compress files"),
    ("unzip", "Extract compressed files"),
    ("curl", "Transfer data from or to a server"),
    ("wget", "Retrieve files from the web"),
    ("ssh", "Secure shell remote login"),
    ("scp", "Secure copy files between hosts"),
    ("rsync", "Remote file synchronization"),
    ("git", "Distributed version control system"),
    ("make", "Build automation tool"),
    ("cmake", "Cross-platform build system"),
    ("npm", "Node package manager"),
    ("yarn", "JavaScript package manager"),
    ("pnpm", "Fast, disk space efficient package manager"),
    ("cargo", "Rust package manager and build tool"),
    ("rustc", "Rust compiler"),
    ("python", "Python interpreter"),
    ("python3", "Python 3 interpreter"),
    ("pip", "Python package installer"),
    ("pip3", "Python 3 package installer"),
    ("node", "JavaScript runtime"),
    ("deno", "Secure JavaScript/TypeScript runtime"),
    ("bun", "Fast JavaScript runtime"),
    ("ruby", "Ruby interpreter"),
    ("gem", "Ruby package manager"),
    ("go", "Go compiler and tool"),
    ("java", "Java runtime"),
    ("javac", "Java compiler"),
    ("mvn", "Maven build tool"),
    ("gradle", "Gradle build tool"),
    ("docker", "Container platform"),
    ("kubectl", "Kubernetes command-line tool"),
    ("terraform", "Infrastructure as code tool"),
    ("ansible", "Automation tool"),
    ("chmod", "Change file permissions"),
    ("chown", "Change file owner and group"),
    ("mkdir", "Create directories"),
    ("rm", "Remove files or directories"),
    ("mv", "Move or rename files"),
    ("cp", "Copy files and directories"),
    ("touch", "Create empty files or update timestamps"),
    ("ln", "Create links between files"),
    ("ps", "Report process status"),
    ("top", "Display system resource usage"),
    ("htop", "Interactive process viewer"),
    ("kill", "Send signal to a process"),
    ("killall", "Kill processes by name"),
    ("which", "Locate a command"),
    ("whereis", "Locate binary, source, and manual pages"),
    ("man", "Display manual pages"),
    ("env", "Display or set environment variables"),
    ("export", "Set environment variables"),
    ("source", "Execute commands from a file"),
    ("pwd", "Print working directory"),
    ("basename", "Strip directory and suffix from filenames"),
    ("dirname", "Strip last component from file name"),
    ("realpath", "Print the resolved path"),
    ("date", "Display or set the system date and time"),
    ("cal", "Display a calendar"),
    ("sleep", "Delay for a specified amount of time"),
    ("time", "Time command execution"),
    ("watch", "Execute a program periodically"),
    (
        "xargs",
        "Build and execute command lines from standard input",
    ),
    ("tee", "Read from standard input and write to files"),
    ("tr", "Translate or delete characters"),
    ("expr", "Evaluate expressions"),
    ("test", "Evaluate conditional expressions"),
    ("true", "Return success exit status"),
    ("false", "Return failure exit status"),
    ("yes", "Output a string repeatedly until killed"),
];

/// Shell command knowledge base
#[derive(Debug, Clone)]
pub struct ShellCommands {
    common: HashSet<String>,
    path_commands: HashSet<String>,
}

impl ShellCommands {
    /// Create a new shell commands knowledge base
    pub fn new() -> Self {
        let common = COMMON_COMMANDS
            .iter()
            .map(|(name, _)| name.to_string())
            .collect();

        let path_commands = Self::scan_path();

        Self {
            common,
            path_commands,
        }
    }

    /// Scan PATH for available executables
    fn scan_path() -> HashSet<String> {
        let mut commands = HashSet::new();

        if let Ok(path_var) = env::var("PATH") {
            for path_dir in env::split_paths(&path_var) {
                if let Ok(entries) = std::fs::read_dir(&path_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() || file_type.is_symlink() {
                                if let Some(name) = entry.file_name().to_str() {
                                    // Check if executable
                                    if Self::is_executable(&entry.path()) {
                                        commands.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        commands
    }

    /// Check if a file is executable
    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let permissions = metadata.permissions();
            permissions.mode() & 0o111 != 0
        } else {
            false
        }
    }

    #[cfg(windows)]
    fn is_executable(path: &Path) -> bool {
        // On Windows, check for common executable extensions
        if let Some(ext) = path.extension() {
            matches!(
                ext.to_str(),
                Some("exe") | Some("bat") | Some("cmd") | Some("com")
            )
        } else {
            false
        }
    }

    #[cfg(not(any(unix, windows)))]
    fn is_executable(_path: &Path) -> bool {
        true
    }

    /// Get all available commands
    pub fn all_commands(&self) -> Vec<String> {
        let mut commands: Vec<String> = self.common.union(&self.path_commands).cloned().collect();
        commands.sort();
        commands
    }

    /// Get common commands with descriptions
    pub fn common_with_descriptions(&self) -> Vec<(&str, &str)> {
        COMMON_COMMANDS.to_vec()
    }

    /// Check if a command is known
    pub fn is_known(&self, command: &str) -> bool {
        self.common.contains(command) || self.path_commands.contains(command)
    }

    /// Get description for a common command
    pub fn get_description(&self, command: &str) -> Option<&str> {
        COMMON_COMMANDS
            .iter()
            .find(|(name, _)| *name == command)
            .map(|(_, desc)| *desc)
    }
}

impl Default for ShellCommands {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_commands() {
        let shell = ShellCommands::new();
        assert!(shell.is_known("ls"));
        assert!(shell.is_known("grep"));
        assert!(shell.is_known("git"));
    }

    #[test]
    fn test_get_description() {
        let shell = ShellCommands::new();
        assert_eq!(shell.get_description("ls"), Some("List directory contents"));
        assert_eq!(
            shell.get_description("grep"),
            Some("Search for patterns in files")
        );
        assert_eq!(shell.get_description("nonexistent"), None);
    }

    #[test]
    fn test_all_commands() {
        let shell = ShellCommands::new();
        let commands = shell.all_commands();
        assert!(!commands.is_empty());
        assert!(commands.contains(&"ls".to_string()));
    }
}
