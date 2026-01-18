use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::setup::detect::detect_zed;
use crate::setup::get_config_path;

pub fn setup() -> Result<()> {
    if !detect_zed() {
        anyhow::bail!("Zed not installed");
    }

    let config_path = get_zed_settings_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create Zed config directory")?;
    }

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

fn get_zed_settings_path() -> Result<PathBuf> {
    let base_path = if cfg!(target_os = "macos") {
        get_config_path("~/Library/Application Support/Zed")?
    } else if cfg!(target_os = "windows") {
        get_config_path("~/AppData/Roaming/Zed")?
    } else {
        get_config_path("~/.config/zed")?
    };

    Ok(base_path.join("settings.json"))
}

fn add_hone_configuration(settings: &mut Value) {
    let settings_obj = match settings.as_object_mut() {
        Some(obj) => obj,
        None => {
            *settings = json!({});
            settings.as_object_mut().expect("just created object")
        }
    };

    if !settings_obj.contains_key("lsp") || !settings_obj.get("lsp").is_some_and(|v| v.is_object())
    {
        settings_obj.insert("lsp".to_string(), json!({}));
    }

    let lsp = settings_obj
        .get_mut("lsp")
        .and_then(|v| v.as_object_mut())
        .expect("just ensured lsp is object");

    if !lsp.contains_key("hone") {
        lsp.insert(
            "hone".to_string(),
            json!({
                "command": "hone",
                "args": ["lsp"]
            }),
        );
    }

    if !settings_obj.contains_key("languages")
        || !settings_obj.get("languages").is_some_and(|v| v.is_object())
    {
        settings_obj.insert("languages".to_string(), json!({}));
    }

    let languages = settings_obj
        .get_mut("languages")
        .and_then(|v| v.as_object_mut())
        .expect("just ensured languages is object");

    if !languages.contains_key("Hone") {
        languages.insert(
            "Hone".to_string(),
            json!({
                "language_servers": ["hone"],
                "file_types": ["hone"]
            }),
        );
    }

    if !settings_obj.contains_key("file_types")
        || !settings_obj
            .get("file_types")
            .is_some_and(|v| v.is_object())
    {
        settings_obj.insert("file_types".to_string(), json!({}));
    }

    let file_types = settings_obj
        .get_mut("file_types")
        .and_then(|v| v.as_object_mut())
        .expect("just ensured file_types is object");

    if !file_types.contains_key("Hone") {
        file_types.insert(
            "Hone".to_string(),
            json!({
                "extensions": ["hone"]
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
        let lsp = obj.get("lsp").unwrap().as_object().unwrap();
        let hone_lsp = lsp.get("hone").unwrap().as_object().unwrap();
        assert_eq!(hone_lsp.get("command"), Some(&json!("hone")));
        assert_eq!(hone_lsp.get("args"), Some(&json!(["lsp"])));

        let languages = obj.get("languages").unwrap().as_object().unwrap();
        let hone_lang = languages.get("Hone").unwrap().as_object().unwrap();
        assert_eq!(hone_lang.get("language_servers"), Some(&json!(["hone"])));
        assert_eq!(hone_lang.get("file_types"), Some(&json!(["hone"])));

        let file_types = obj.get("file_types").unwrap().as_object().unwrap();
        let hone_ft = file_types.get("Hone").unwrap().as_object().unwrap();
        assert_eq!(hone_ft.get("extensions"), Some(&json!(["hone"])));
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
            "theme": "One Dark",
            "lsp": {
                "rust-analyzer": {
                    "command": "rust-analyzer"
                }
            }
        });
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        assert_eq!(obj.get("theme"), Some(&json!("One Dark")));

        let lsp = obj.get("lsp").unwrap().as_object().unwrap();
        assert!(lsp.contains_key("rust-analyzer"));
        assert!(lsp.contains_key("hone"));
    }

    #[test]
    fn test_add_hone_configuration_handles_non_object_root() {
        let mut settings = json!("invalid");
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        assert!(obj.contains_key("lsp"));
    }

    #[test]
    fn test_add_hone_configuration_handles_non_object_lsp() {
        let mut settings = json!({
            "lsp": "invalid"
        });
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let lsp = obj.get("lsp").unwrap().as_object().unwrap();
        assert!(lsp.contains_key("hone"));
    }

    #[test]
    fn test_add_hone_configuration_handles_non_object_languages() {
        let mut settings = json!({
            "languages": []
        });
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let languages = obj.get("languages").unwrap().as_object().unwrap();
        assert!(languages.contains_key("Hone"));
    }

    #[test]
    fn test_add_hone_configuration_handles_non_object_file_types() {
        let mut settings = json!({
            "file_types": null
        });
        add_hone_configuration(&mut settings);

        let obj = settings.as_object().unwrap();
        let file_types = obj.get("file_types").unwrap().as_object().unwrap();
        assert!(file_types.contains_key("Hone"));
    }
}
