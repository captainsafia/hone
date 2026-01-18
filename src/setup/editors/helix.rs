use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use toml_edit::{DocumentMut, Item, Table};

use crate::setup::detect::detect_helix;
use crate::setup::expand_home;

pub fn setup() -> Result<()> {
    if !detect_helix() {
        anyhow::bail!("Helix not installed");
    }

    let config_path = get_helix_config_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).context("Failed to create Helix config directory")?;
    }

    let mut doc = if config_path.exists() {
        let content = fs::read_to_string(&config_path).context("Failed to read languages.toml")?;
        content
            .parse::<DocumentMut>()
            .context("Failed to parse languages.toml")?
    } else {
        DocumentMut::new()
    };

    add_hone_language(&mut doc);

    fs::write(&config_path, doc.to_string()).context("Failed to write languages.toml")?;

    Ok(())
}

fn get_helix_config_path() -> Result<PathBuf> {
    let base_path = if cfg!(target_os = "windows") {
        expand_home("~/AppData/Roaming/helix")?
    } else {
        expand_home("~/.config/helix")?
    };

    Ok(base_path.join("languages.toml"))
}

fn add_hone_language(doc: &mut DocumentMut) {
    if !doc.contains_key("language") || !doc["language"].is_array_of_tables() {
        doc["language"] = Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
    }

    let languages = doc["language"]
        .as_array_of_tables_mut()
        .expect("just ensured language is array of tables");

    let mut found = false;
    for lang in languages.iter_mut() {
        if lang.get("name").and_then(|v| v.as_str()) == Some("hone") {
            found = true;
            break;
        }
    }

    if !found {
        let mut hone_table = Table::new();
        hone_table.insert("name", toml_edit::value("hone"));
        hone_table.insert("scope", toml_edit::value("source.hone"));

        let mut file_types = toml_edit::Array::new();
        file_types.push("hone");
        hone_table.insert("file-types", toml_edit::value(file_types));

        hone_table.insert("comment-token", toml_edit::value("#"));
        hone_table.insert("indent", {
            let mut indent = toml_edit::InlineTable::new();
            indent.insert("tab-width", 2.into());
            indent.insert("unit", "  ".into());
            toml_edit::value(indent)
        });

        let mut lang_servers = toml_edit::Array::new();
        lang_servers.push("hone");
        hone_table.insert("language-servers", toml_edit::value(lang_servers));

        languages.push(hone_table);
    }

    if !doc.contains_key("language-server") || !doc["language-server"].is_table() {
        doc["language-server"] = Item::Table(Table::new());
    }

    let language_servers = doc["language-server"]
        .as_table_mut()
        .expect("just ensured language-server is table");

    if !language_servers.contains_key("hone") {
        let mut hone_server = Table::new();
        hone_server.insert("command", toml_edit::value("hone"));

        let mut args = toml_edit::Array::new();
        args.push("lsp");
        hone_server.insert("args", toml_edit::value(args));

        language_servers.insert("hone", Item::Table(hone_server));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_hone_language_to_empty_doc() {
        let mut doc = DocumentMut::new();
        add_hone_language(&mut doc);

        let result = doc.to_string();
        assert!(result.contains("name = \"hone\""));
        assert!(result.contains("language-servers = [\"hone\"]"));
        assert!(result.contains("[language-server.hone]"));
        assert!(result.contains("command = \"hone\""));
        assert!(result.contains("args = [\"lsp\"]"));
    }

    #[test]
    fn test_add_hone_language_idempotent() {
        let mut doc = DocumentMut::new();
        add_hone_language(&mut doc);
        let first = doc.to_string();

        add_hone_language(&mut doc);
        let second = doc.to_string();

        assert_eq!(first, second);
    }

    #[test]
    fn test_add_hone_language_handles_malformed_language_key() {
        let mut doc = "language = \"invalid\"".parse::<DocumentMut>().unwrap();
        add_hone_language(&mut doc);

        let result = doc.to_string();
        assert!(result.contains("name = \"hone\""));
    }

    #[test]
    fn test_add_hone_language_handles_malformed_language_server_key() {
        let mut doc = "language-server = []".parse::<DocumentMut>().unwrap();
        add_hone_language(&mut doc);

        let result = doc.to_string();
        assert!(result.contains("[language-server.hone]"));
    }
}
