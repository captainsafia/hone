use crate::runner::sentinel::{extract_sentinel, generate_run_id, generate_shell_wrapper, SentinelData};
use crate::parser::{PragmaNode, PragmaType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::{sleep, timeout, Duration};

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub shell: String,
    pub env: HashMap<String, String>,
    pub timeout_ms: u64,
    pub cwd: String,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct RunResult {
    pub run_id: String,
    pub stdout: String,
    pub stdout_raw: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub stderr_path: String,
}

fn get_shell_flags(shell_path: &str) -> Vec<&str> {
    let shell_name = Path::new(shell_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    match shell_name {
        "bash" => vec!["--norc", "--noprofile"],
        "zsh" => vec!["--no-rcs"],
        "sh" => vec![],
        _ => vec![],
    }
}

pub fn is_shell_supported(shell_path: &str) -> bool {
    let shell_name = Path::new(shell_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    matches!(shell_name, "bash" | "zsh" | "sh")
}

pub struct ShellSession {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    output_buffer: String,
    config: ShellConfig,
    run_index: usize,
    current_test_name: Option<String>,
    artifact_dir: PathBuf,
}

impl ShellSession {
    pub fn new(config: ShellConfig) -> Self {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let basename = Path::new(&config.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");

        let artifact_dir = PathBuf::from(&config.cwd)
            .join(".hone")
            .join("runs")
            .join(format!("{}-{}", timestamp, basename));

        Self {
            process: None,
            stdin: None,
            stdout_reader: None,
            output_buffer: String::new(),
            config,
            run_index: 0,
            current_test_name: None,
            artifact_dir,
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        let shell_flags = get_shell_flags(&self.config.shell);

        tokio::fs::create_dir_all(&self.artifact_dir)
            .await
            .map_err(|e| format!("Failed to create artifact directory: {}", e))?;

        let mut env = self.config.env.clone();
        env.insert("PS1".to_string(), "".to_string());
        env.insert("TERM".to_string(), "dumb".to_string());

        let mut child = Command::new(&self.config.shell)
            .args(&shell_flags)
            .current_dir(&self.config.cwd)
            .env_clear()
            .envs(&env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn shell: {}", e))?;

        self.stdin = Some(child.stdin.take().unwrap());
        let stdout = child.stdout.take().unwrap();
        self.stdout_reader = Some(BufReader::new(stdout));
        self.process = Some(child);

        self.wait_for_ready().await?;

        Ok(())
    }

    async fn wait_for_ready(&mut self) -> Result<(), String> {
        let ready_marker = format!("__HONE_READY_{}__", chrono::Utc::now().timestamp_millis());
        self.write_to_shell(&format!("echo \"{}\"\n", ready_marker))
            .await?;

        let found = self.wait_for_string(&ready_marker, 5000).await;

        if !found {
            return Err(format!(
                "Shell failed to start within 5000ms. Shell: {}",
                self.config.shell
            ));
        }

        self.output_buffer.clear();
        Ok(())
    }

    async fn wait_for_string(&mut self, marker: &str, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();

        while start.elapsed().as_millis() < timeout_ms as u128 {
            self.read_available().await;

            if self.output_buffer.contains(marker) {
                return true;
            }

            sleep(Duration::from_millis(10)).await;
        }

        false
    }

    async fn read_available(&mut self) {
        if let Some(reader) = &mut self.stdout_reader {
            loop {
                let mut line = String::new();
                match timeout(Duration::from_millis(1), reader.read_line(&mut line)).await {
                    Ok(Ok(n)) if n > 0 => {
                        self.output_buffer.push_str(&line);
                    }
                    _ => break, // No more data available or timeout/error
                }
            }
        }
    }

    pub fn set_current_test(&mut self, test_name: Option<String>) {
        self.current_test_name = test_name;
    }

    pub async fn set_env_vars(&mut self, vars: &[(String, String)]) -> Result<(), String> {
        for (key, value) in vars {
            let escaped_value = value.replace('\'', "'\\''");
            self.write_to_shell(&format!("export {}='{}'\n", key, escaped_value))
                .await?;
        }

        self.flush().await?;
        Ok(())
    }

    pub async fn get_cwd(&mut self) -> Result<String, String> {
        let marker = format!("__HONE_CWD_{}__", chrono::Utc::now().timestamp_millis());
        self.write_to_shell(&format!("echo \"{}$PWD{}\"\n", marker, marker))
            .await?;

        let found = self.wait_for_string(&marker, 2000).await;
        if !found {
            return Ok(self.config.cwd.clone()); // Fallback
        }

        let pattern = format!("{}(.+?){}", regex::escape(&marker), regex::escape(&marker));
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(captures) = re.captures(&self.output_buffer) {
                if let Some(cwd_match) = captures.get(1) {
                    let cwd = cwd_match.as_str().to_string();
                    self.output_buffer.clear();
                    return Ok(cwd);
                }
            }
        }

        Ok(self.config.cwd.clone()) // Fallback
    }

    async fn flush(&mut self) -> Result<(), String> {
        let flush_marker = format!("__HONE_FLUSH_{}__", chrono::Utc::now().timestamp_millis());
        self.write_to_shell(&format!("echo \"{}\"\n", flush_marker))
            .await?;

        self.wait_for_string(&flush_marker, 2000).await;
        self.output_buffer.clear();
        Ok(())
    }

    pub async fn run(&mut self, command: &str, name: Option<&str>) -> Result<RunResult, String> {
        if self.process.is_none() {
            return Err("Shell session not started".to_string());
        }

        self.run_index += 1;
        let run_id = generate_run_id(
            &self.config.filename,
            self.current_test_name.as_deref(),
            name,
            self.run_index,
        );

        let stderr_path = self.artifact_dir.join(format!("{}-stderr.txt", run_id));
        let stderr_path_str = stderr_path.to_str().unwrap();

        let wrapper = generate_shell_wrapper(command, &run_id, stderr_path_str);
        let start_time = std::time::Instant::now();

        self.write_to_shell(&format!("{}\n", wrapper)).await?;

        let result = self.wait_for_sentinel(&run_id).await?;
        let duration_ms = start_time.elapsed().as_millis() as u64;

        let stderr = tokio::fs::read_to_string(&stderr_path)
            .await
            .unwrap_or_default();

        Ok(RunResult {
            run_id: run_id.clone(),
            stdout: strip_ansi_escapes::strip_str(&result.output),
            stdout_raw: result.output,
            stderr,
            exit_code: result.sentinel.as_ref().map(|s| s.exit_code).unwrap_or(-1),
            duration_ms,
            stderr_path: stderr_path_str.to_string(),
        })
    }

    async fn wait_for_sentinel(&mut self, run_id: &str) -> Result<SentinelResult, String> {
        let start_time = std::time::Instant::now();

        while start_time.elapsed().as_millis() < self.config.timeout_ms as u128 {
            self.read_available().await;

            let result = extract_sentinel(&self.output_buffer, run_id);

            if result.found {
                self.output_buffer = result.remaining;
                return Ok(SentinelResult {
                    output: result.output,
                    sentinel: result.sentinel,
                });
            }

            sleep(Duration::from_millis(10)).await;
        }

        Err(format!(
            "Timeout waiting for command completion ({}ms). Run ID: {}",
            self.config.timeout_ms, run_id
        ))
    }

    async fn write_to_shell(&mut self, data: &str) -> Result<(), String> {
        if let Some(stdin) = &mut self.stdin {
            stdin
                .write_all(data.as_bytes())
                .await
                .map_err(|e| format!("Failed to write to shell: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush shell stdin: {}", e))?;
            Ok(())
        } else {
            Err("Shell stdin not available".to_string())
        }
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut process) = self.process.take() {
            let _ = self.write_to_shell("exit\n").await;

            match timeout(Duration::from_millis(100), process.wait()).await {
                Ok(_) => {}
                Err(_) => {
                    let _ = process.kill().await;
                }
            }
        }

        self.stdin = None;
        self.stdout_reader = None;
        Ok(())
    }
}

impl Drop for ShellSession {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }
    }
}

struct SentinelResult {
    output: String,
    sentinel: Option<SentinelData>,
}

pub fn create_shell_config(
    pragmas: &[PragmaNode],
    filename: &str,
    cwd: &str,
    override_shell: Option<&str>,
) -> ShellConfig {
    let mut shell = override_shell
        .map(String::from)
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| "/bin/bash".to_string());

    let mut env = HashMap::new();
    env.insert(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_string()),
    );
    env.insert(
        "HOME".to_string(),
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string()),
    );

    let mut timeout_ms = 30000; // 30 seconds default

    for pragma in pragmas {
        match pragma.pragma_type {
            PragmaType::Shell => {
                if override_shell.is_none() {
                    shell = pragma.value.clone();
                }
            }
            PragmaType::Env => {
                if let Some(key) = &pragma.key {
                    env.insert(key.clone(), pragma.value.clone());
                }
            }
            PragmaType::Timeout => {
                // Parse timeout value
                let re = regex::Regex::new(r"^(\d+(?:\.\d+)?)(ms|s)$").unwrap();
                if let Some(captures) = re.captures(&pragma.value) {
                    if let Ok(value) = captures.get(1).unwrap().as_str().parse::<f64>() {
                        let unit = captures.get(2).unwrap().as_str();
                        timeout_ms = if unit == "s" {
                            (value * 1000.0) as u64
                        } else {
                            value as u64
                        };
                    }
                }
            }
            _ => {}
        }
    }

    ShellConfig {
        shell,
        env,
        timeout_ms,
        cwd: cwd.to_string(),
        filename: filename.to_string(),
    }
}
