// Claude Code settings.json operations
// Implements: task 5.4

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

/// Read settings.json, returning empty object if not found.
pub fn load_settings(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(Value::Object(serde_json::Map::new()));
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let settings: Value = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {} as JSON", path.display()))?;

    Ok(settings)
}

/// Write settings.json with pretty formatting.
pub fn save_settings(path: &Path, settings: &Value) -> Result<()> {
    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let contents = serde_json::to_string_pretty(settings)
        .context("Failed to serialize settings to JSON")?;

    std::fs::write(path, contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Add a plugin to the `enabledPlugins` list in settings.
/// Creates the key if it doesn't exist. Skips duplicates.
pub fn enable_plugin(settings: &mut Value, plugin_name: &str) {
    let obj = settings
        .as_object_mut()
        .expect("settings must be a JSON object");

    let arr = obj
        .entry("enabledPlugins")
        .or_insert_with(|| Value::Array(Vec::new()));

    let arr = arr.as_array_mut().expect("enabledPlugins must be an array");

    // Don't add duplicates.
    let already_present = arr
        .iter()
        .any(|v| v.as_str() == Some(plugin_name));

    if !already_present {
        arr.push(Value::String(plugin_name.to_string()));
    }
}

/// Add a marketplace URL to the `extraKnownMarketplaces` list in settings.
/// Creates the key if it doesn't exist. Skips duplicates.
pub fn add_marketplace(settings: &mut Value, marketplace_url: &str) {
    let obj = settings
        .as_object_mut()
        .expect("settings must be a JSON object");

    let arr = obj
        .entry("extraKnownMarketplaces")
        .or_insert_with(|| Value::Array(Vec::new()));

    let arr = arr
        .as_array_mut()
        .expect("extraKnownMarketplaces must be an array");

    // Don't add duplicates.
    let already_present = arr
        .iter()
        .any(|v| v.as_str() == Some(marketplace_url));

    if !already_present {
        arr.push(Value::String(marketplace_url.to_string()));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_settings_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let settings = load_settings(&path).unwrap();
        assert_eq!(settings, Value::Object(serde_json::Map::new()));
    }

    #[test]
    fn test_load_save_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut settings = Value::Object(serde_json::Map::new());
        enable_plugin(&mut settings, "test-plugin");

        save_settings(&path, &settings).unwrap();

        let loaded = load_settings(&path).unwrap();
        assert_eq!(settings, loaded);
    }

    #[test]
    fn test_enable_plugin_creates_array() {
        let mut settings = Value::Object(serde_json::Map::new());

        enable_plugin(&mut settings, "my-plugin@marketplace");

        let plugins = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].as_str().unwrap(), "my-plugin@marketplace");
    }

    #[test]
    fn test_enable_plugin_no_duplicates() {
        let mut settings = Value::Object(serde_json::Map::new());

        enable_plugin(&mut settings, "my-plugin");
        enable_plugin(&mut settings, "my-plugin");

        let plugins = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 1);
    }

    #[test]
    fn test_enable_plugin_preserves_existing() {
        let mut settings: Value = serde_json::from_str(
            r#"{"enabledPlugins": ["existing-plugin"]}"#,
        )
        .unwrap();

        enable_plugin(&mut settings, "new-plugin");

        let plugins = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].as_str().unwrap(), "existing-plugin");
        assert_eq!(plugins[1].as_str().unwrap(), "new-plugin");
    }

    #[test]
    fn test_add_marketplace_creates_array() {
        let mut settings = Value::Object(serde_json::Map::new());

        add_marketplace(&mut settings, "https://github.com/some/marketplace");

        let markets = settings["extraKnownMarketplaces"].as_array().unwrap();
        assert_eq!(markets.len(), 1);
        assert_eq!(
            markets[0].as_str().unwrap(),
            "https://github.com/some/marketplace"
        );
    }

    #[test]
    fn test_add_marketplace_no_duplicates() {
        let mut settings = Value::Object(serde_json::Map::new());

        add_marketplace(&mut settings, "https://example.com");
        add_marketplace(&mut settings, "https://example.com");

        let markets = settings["extraKnownMarketplaces"].as_array().unwrap();
        assert_eq!(markets.len(), 1);
    }

    #[test]
    fn test_save_settings_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("settings.json");

        let settings = Value::Object(serde_json::Map::new());
        save_settings(&path, &settings).unwrap();

        assert!(path.exists());
    }

    // -----------------------------------------------------------------
    // 10.2 — Additional settings edge-case tests
    // -----------------------------------------------------------------

    #[test]
    fn test_load_settings_empty_file_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        // Write an empty file — valid file but invalid JSON.
        std::fs::write(&path, "").unwrap();

        let result = load_settings(&path);
        assert!(result.is_err(), "Empty JSON file should fail to parse");
    }

    #[test]
    fn test_load_settings_preserves_unknown_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let data = r#"{"theme": "dark", "customStuff": [1, 2, 3]}"#;
        std::fs::write(&path, data).unwrap();

        let settings = load_settings(&path).unwrap();
        assert_eq!(settings["theme"], "dark");
        assert_eq!(settings["customStuff"][0], 1);
    }

    #[test]
    fn test_enable_plugin_dedup_multiple_calls() {
        let mut settings = Value::Object(serde_json::Map::new());

        enable_plugin(&mut settings, "plugin-a");
        enable_plugin(&mut settings, "plugin-b");
        enable_plugin(&mut settings, "plugin-a"); // duplicate
        enable_plugin(&mut settings, "plugin-c");
        enable_plugin(&mut settings, "plugin-b"); // duplicate

        let plugins = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 3);
        assert_eq!(plugins[0].as_str().unwrap(), "plugin-a");
        assert_eq!(plugins[1].as_str().unwrap(), "plugin-b");
        assert_eq!(plugins[2].as_str().unwrap(), "plugin-c");
    }

    #[test]
    fn test_enable_plugin_preserves_other_settings() {
        let mut settings: Value = serde_json::from_str(
            r#"{"theme": "dark", "enabledPlugins": ["existing"]}"#,
        )
        .unwrap();

        enable_plugin(&mut settings, "new-plugin");

        // theme should still be present
        assert_eq!(settings["theme"], "dark");
        let plugins = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 2);
    }

    #[test]
    fn test_add_marketplace_dedup_multiple_calls() {
        let mut settings = Value::Object(serde_json::Map::new());

        add_marketplace(&mut settings, "https://example.com/a");
        add_marketplace(&mut settings, "https://example.com/b");
        add_marketplace(&mut settings, "https://example.com/a"); // dup

        let markets = settings["extraKnownMarketplaces"].as_array().unwrap();
        assert_eq!(markets.len(), 2);
    }

    #[test]
    fn test_save_load_roundtrip_complex_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut settings = Value::Object(serde_json::Map::new());
        enable_plugin(&mut settings, "plugin-1");
        enable_plugin(&mut settings, "plugin-2");
        add_marketplace(&mut settings, "https://marketplace.example.com");

        save_settings(&path, &settings).unwrap();
        let loaded = load_settings(&path).unwrap();

        assert_eq!(
            loaded["enabledPlugins"].as_array().unwrap().len(),
            2
        );
        assert_eq!(
            loaded["extraKnownMarketplaces"].as_array().unwrap().len(),
            1
        );
    }
}
