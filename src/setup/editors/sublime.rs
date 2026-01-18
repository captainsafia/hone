use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::setup::detect::detect_sublime;
use crate::setup::expand_home;

pub fn setup() -> Result<()> {
    if !detect_sublime() {
        anyhow::bail!("Sublime Text not installed");
    }

    let config_dir = get_sublime_config_dir()?;
    fs::create_dir_all(&config_dir).context("Failed to create Sublime Text config directory")?;

    configure_lsp(&config_dir)?;
    configure_syntax(&config_dir)?;

    Ok(())
}

fn get_sublime_config_dir() -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        expand_home("~/Library/Application Support/Sublime Text/Packages/User")
    } else if cfg!(target_os = "windows") {
        expand_home("~/AppData/Roaming/Sublime Text/Packages/User")
    } else {
        expand_home("~/.config/sublime-text/Packages/User")
    }
}

fn configure_lsp(config_dir: &Path) -> Result<()> {
    let lsp_settings_path = config_dir.join("LSP.sublime-settings");

    let mut settings = if lsp_settings_path.exists() {
        let content = fs::read_to_string(&lsp_settings_path)
            .context("Failed to read LSP.sublime-settings")?;
        serde_json::from_str::<Value>(&content).context("Failed to parse LSP.sublime-settings")?
    } else {
        json!({})
    };

    add_lsp_configuration(&mut settings);

    let formatted = serde_json::to_string_pretty(&settings)
        .context("Failed to serialize LSP.sublime-settings")?;
    fs::write(&lsp_settings_path, formatted).context("Failed to write LSP.sublime-settings")?;

    Ok(())
}

fn add_lsp_configuration(settings: &mut Value) {
    let settings_obj = match settings.as_object_mut() {
        Some(obj) => obj,
        None => {
            *settings = json!({});
            settings.as_object_mut().expect("just created object")
        }
    };

    if !settings_obj.contains_key("clients")
        || !settings_obj.get("clients").is_some_and(|v| v.is_object())
    {
        settings_obj.insert("clients".to_string(), json!({}));
    }

    let clients = settings_obj
        .get_mut("clients")
        .and_then(|v| v.as_object_mut())
        .expect("just ensured clients is object");

    if !clients.contains_key("hone") {
        clients.insert(
            "hone".to_string(),
            json!({
                "enabled": true,
                "command": ["hone", "lsp"],
                "selector": "source.hone"
            }),
        );
    }
}

fn configure_syntax(config_dir: &Path) -> Result<()> {
    let syntax_path = config_dir.join("Hone.sublime-syntax");

    let syntax_content = get_sublime_syntax();
    fs::write(&syntax_path, syntax_content).context("Failed to write Hone.sublime-syntax")?;

    Ok(())
}

fn get_sublime_syntax() -> String {
    r#"%YAML 1.2
---
name: Hone
file_extensions:
  - hone
scope: source.hone

contexts:
  main:
    - include: comments
    - include: pragmas
    - include: test-block
    - include: run-statement
    - include: env-statement
    - include: assert-statement

  comments:
    - match: ^\s*#(?!!).*$
      scope: comment.line.number-sign.hone

  pragmas:
    - match: ^(#!)(\s*)(shell|env|timeout)(:)(\s*)(.*)$
      captures:
        1: punctuation.definition.pragma.hone
        3: keyword.other.pragma.hone
        4: punctuation.separator.hone
        6: string.unquoted.hone

  test-block:
    - match: ^\s*(test)\s+(.*)$
      captures:
        1: keyword.control.test.hone
        2: entity.name.function.hone
      push: test-body

  test-body:
    - match: ^\s*$
      pop: true
    - include: run-statement
    - include: assert-statement
    - include: env-statement
    - include: comments

  run-statement:
    - match: ^\s*(>>>)(.*)$
      captures:
        1: keyword.operator.run.hone
        2: source.shell

  env-statement:
    - match: ^\s*(env)\s+([A-Z_][A-Z0-9_]*)\s*(=)(.*)$
      captures:
        1: keyword.other.env.hone
        2: variable.other.env.hone
        3: keyword.operator.assignment.hone
        4: string.unquoted.hone

  assert-statement:
    - match: ^\s*(exitcode|output|file|timing)\s+(.*)$
      captures:
        1: keyword.control.assertion.hone
        2: string.unquoted.hone
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_lsp_configuration_to_empty_settings() {
        let mut settings = json!({});
        add_lsp_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let clients = obj.get("clients").unwrap().as_object().unwrap();
        let hone = clients.get("hone").unwrap().as_object().unwrap();

        assert_eq!(hone.get("enabled"), Some(&json!(true)));
        assert_eq!(hone.get("command"), Some(&json!(["hone", "lsp"])));
        assert_eq!(hone.get("selector"), Some(&json!("source.hone")));
    }

    #[test]
    fn test_add_lsp_configuration_idempotent() {
        let mut settings = json!({});
        add_lsp_configuration(&mut settings);
        let first = serde_json::to_string_pretty(&settings).unwrap();

        add_lsp_configuration(&mut settings);
        let second = serde_json::to_string_pretty(&settings).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn test_add_lsp_configuration_preserves_existing() {
        let mut settings = json!({
            "clients": {
                "rust-analyzer": {
                    "enabled": true
                }
            }
        });
        add_lsp_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let clients = obj.get("clients").unwrap().as_object().unwrap();

        assert!(clients.contains_key("rust-analyzer"));
        assert!(clients.contains_key("hone"));
    }

    #[test]
    fn test_sublime_syntax_not_empty() {
        let syntax = get_sublime_syntax();
        assert!(!syntax.is_empty());
        assert!(syntax.contains("name: Hone"));
        assert!(syntax.contains("file_extensions"));
    }

    #[test]
    fn test_add_lsp_configuration_handles_non_object_root() {
        let mut settings = json!([]);
        add_lsp_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        assert!(obj.contains_key("clients"));
    }

    #[test]
    fn test_add_lsp_configuration_handles_non_object_clients() {
        let mut settings = json!({
            "clients": "invalid"
        });
        add_lsp_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let clients = obj.get("clients").unwrap().as_object().unwrap();
        assert!(clients.contains_key("hone"));
    }
}
