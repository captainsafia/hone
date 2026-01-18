use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::setup::detect::detect_vscode;
use crate::setup::get_config_path;

pub fn setup() -> Result<()> {
    if !detect_vscode() {
        anyhow::bail!("VS Code not installed");
    }

    let config_path = get_vscode_settings_path()?;

    fs::create_dir_all(config_path.parent().unwrap())
        .context("Failed to create VS Code config directory")?;

    let mut settings = if config_path.exists() {
        let content = fs::read_to_string(&config_path).context("Failed to read settings.json")?;
        serde_json::from_str::<Value>(&content).context("Failed to parse settings.json")?
    } else {
        json!({})
    };

    add_hone_configuration(&mut settings);

    let formatted =
        serde_json::to_string_pretty(&settings).context("Failed to serialize settings.json")?;
    fs::write(&config_path, formatted).context("Failed to write settings.json")?;

    Ok(())
}

fn get_vscode_settings_path() -> Result<PathBuf> {
    let base_path = if cfg!(target_os = "macos") {
        get_config_path("~/Library/Application Support/Code/User")?
    } else if cfg!(target_os = "windows") {
        get_config_path("~/AppData/Roaming/Code/User")?
    } else {
        get_config_path("~/.config/Code/User")?
    };

    Ok(base_path.join("settings.json"))
}

fn add_hone_configuration(settings: &mut Value) {
    let settings_obj = settings.as_object_mut().unwrap();

    if !settings_obj.contains_key("hone.lsp.enabled") {
        settings_obj.insert("hone.lsp.enabled".to_string(), json!(true));
    }

    if !settings_obj.contains_key("hone.lsp.path") {
        settings_obj.insert("hone.lsp.path".to_string(), json!("hone"));
    }

    if !settings_obj.contains_key("files.associations") {
        settings_obj.insert("files.associations".to_string(), json!({}));
    }

    let file_associations = settings_obj
        .get_mut("files.associations")
        .unwrap()
        .as_object_mut()
        .unwrap();

    if !file_associations.contains_key("*.hone") {
        file_associations.insert("*.hone".to_string(), json!("hone"));
    }

    add_textmate_grammar(settings_obj);
    add_lsp_configuration(settings_obj);
}

fn add_textmate_grammar(settings: &mut serde_json::Map<String, Value>) {
    if !settings.contains_key("editor.tokenColorCustomizations") {
        settings.insert(
            "editor.tokenColorCustomizations".to_string(),
            json!({"textMateRules": []}),
        );
    }

    let token_colors = settings
        .get_mut("editor.tokenColorCustomizations")
        .unwrap()
        .as_object_mut()
        .unwrap();

    if !token_colors.contains_key("textMateRules") {
        token_colors.insert("textMateRules".to_string(), json!([]));
    }

    let rules = token_colors
        .get_mut("textMateRules")
        .unwrap()
        .as_array_mut()
        .unwrap();

    let hone_rule = json!({
        "scope": "source.hone",
        "settings": {
            "foreground": "#D4D4D4"
        }
    });

    let mut found = false;
    for rule in rules.iter() {
        if rule.get("scope").and_then(|v| v.as_str()) == Some("source.hone") {
            found = true;
            break;
        }
    }

    if !found {
        rules.push(hone_rule);
    }
}

fn add_lsp_configuration(settings: &mut serde_json::Map<String, Value>) {
    if !settings.contains_key("hone.languageServer") {
        settings.insert(
            "hone.languageServer".to_string(),
            json!({
                "command": "hone",
                "args": ["lsp"],
                "filetypes": ["hone"]
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_hone_configuration_to_empty_settings() {
        let mut settings = json!({});
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        assert_eq!(obj.get("hone.lsp.enabled"), Some(&json!(true)));
        assert_eq!(obj.get("hone.lsp.path"), Some(&json!("hone")));

        let file_assocs = obj.get("files.associations").unwrap().as_object().unwrap();
        assert_eq!(file_assocs.get("*.hone"), Some(&json!("hone")));

        assert!(obj.contains_key("editor.tokenColorCustomizations"));
        assert!(obj.contains_key("hone.languageServer"));
    }

    #[test]
    fn test_add_hone_configuration_idempotent() {
        let mut settings = json!({});
        add_hone_configuration(&mut settings);
        let first = serde_json::to_string_pretty(&settings).unwrap();

        add_hone_configuration(&mut settings);
        let second = serde_json::to_string_pretty(&settings).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn test_add_hone_configuration_preserves_existing() {
        let mut settings = json!({
            "editor.fontSize": 14,
            "files.associations": {
                "*.custom": "plaintext"
            }
        });
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        assert_eq!(obj.get("editor.fontSize"), Some(&json!(14)));

        let file_assocs = obj.get("files.associations").unwrap().as_object().unwrap();
        assert_eq!(file_assocs.get("*.custom"), Some(&json!("plaintext")));
        assert_eq!(file_assocs.get("*.hone"), Some(&json!("hone")));
    }
}
