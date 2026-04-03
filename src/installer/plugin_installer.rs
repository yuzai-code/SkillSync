// Enable plugins in Claude Code settings
// Implements: task 5.4

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::claude::settings;

/// Result of enabling/installing a single plugin.
#[allow(dead_code)]
#[derive(Debug)]
pub enum PluginResult {
    /// Plugin was newly added to `enabledPlugins`.
    Enabled(String),
    /// Plugin was already in `enabledPlugins`.
    AlreadyEnabled(String),
    /// Attempted to run `claude plugins install`; `bool` indicates success.
    InstallAttempted(String, bool),
}

/// Enable plugins in Claude Code settings and attempt installation via the
/// Claude CLI.
///
/// For each plugin name:
/// 1. Parse `name@marketplace` format — if a marketplace is present, add it
///    to `extraKnownMarketplaces`.
/// 2. Add the full identifier to `enabledPlugins`.
/// 3. Save settings.
/// 4. Best-effort attempt `claude plugins install <name>`.
#[allow(dead_code)]
pub fn enable_plugins(
    plugin_names: &[String],
    settings_path: &Path,
) -> Result<Vec<PluginResult>> {
    let mut settings_val = settings::load_settings(settings_path)
        .context("Failed to load Claude Code settings")?;

    let mut results = Vec::with_capacity(plugin_names.len());

    for full_name in plugin_names {
        // Parse name@marketplace format.
        let (plugin_id, marketplace) = parse_plugin_name(full_name);

        // If a marketplace URL is embedded, register it.
        if let Some(marketplace_url) = marketplace {
            settings::add_marketplace(&mut settings_val, marketplace_url);
        }

        // Check if already enabled by inspecting the current array.
        let already_enabled = settings_val
            .get("enabledPlugins")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|v| v.as_str() == Some(full_name)))
            .unwrap_or(false);

        if already_enabled {
            results.push(PluginResult::AlreadyEnabled(full_name.clone()));
        } else {
            settings::enable_plugin(&mut settings_val, full_name);
            results.push(PluginResult::Enabled(full_name.clone()));
        }

        // Save after each plugin so partial progress is persisted.
        settings::save_settings(settings_path, &settings_val)
            .context("Failed to save Claude Code settings")?;

        // Best-effort: try to install via Claude CLI.
        let install_success = attempt_cli_install(plugin_id);
        if install_success {
            results.push(PluginResult::InstallAttempted(full_name.clone(), true));
        }
        // We intentionally don't push a failure result — the CLI may not be
        // available and that's fine.
    }

    Ok(results)
}

/// Parse `name@marketplace` format. Returns `(full_name_or_id, Option<marketplace>)`.
///
/// If there's no `@`, returns the whole string as the plugin id with `None`
/// marketplace. If there is an `@`, splits on the last `@` so that plugin
/// names with `@` in them (like `superpowers@claude-plugins-official`) work
/// correctly. The part before `@` is the plugin name and the full string is
/// the identifier used by Claude Code.
fn parse_plugin_name(full_name: &str) -> (&str, Option<&str>) {
    // Claude Code uses `name@marketplace` as the full identifier.
    // The marketplace portion might be a simple name or a URL.
    // We split on the last `@` to get the marketplace.
    match full_name.rsplit_once('@') {
        Some((name, marketplace)) => {
            // Only treat as marketplace URL if it looks like a URL.
            if marketplace.starts_with("http://") || marketplace.starts_with("https://") {
                (name, Some(marketplace))
            } else {
                // It's a marketplace name (e.g., "claude-plugins-official"),
                // not a URL — no need to add to extraKnownMarketplaces.
                (full_name, None)
            }
        }
        None => (full_name, None),
    }
}

/// Attempt to install a plugin via `claude plugins install <name>`.
/// Returns `true` on success, `false` if the CLI is not available or fails.
fn attempt_cli_install(plugin_name: &str) -> bool {
    Command::new("claude")
        .args(["plugins", "install", plugin_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_plugin_name_simple() {
        let (name, marketplace) = parse_plugin_name("superpowers");
        assert_eq!(name, "superpowers");
        assert!(marketplace.is_none());
    }

    #[test]
    fn test_parse_plugin_name_with_named_marketplace() {
        let (name, marketplace) =
            parse_plugin_name("superpowers@claude-plugins-official");
        // Named marketplace — not a URL, so no marketplace to add.
        assert_eq!(name, "superpowers@claude-plugins-official");
        assert!(marketplace.is_none());
    }

    #[test]
    fn test_parse_plugin_name_with_url_marketplace() {
        let (name, marketplace) =
            parse_plugin_name("my-plugin@https://github.com/some/marketplace");
        assert_eq!(name, "my-plugin");
        assert_eq!(
            marketplace.unwrap(),
            "https://github.com/some/marketplace"
        );
    }

    #[test]
    fn test_enable_plugins_basic() {
        let dir = tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");

        let plugins = vec!["test-plugin@marketplace-name".to_string()];
        let results = enable_plugins(&plugins, &settings_path).unwrap();

        // Should have at least an Enabled result.
        assert!(results.iter().any(|r| matches!(r, PluginResult::Enabled(_))));

        // Verify settings file was written.
        let settings = settings::load_settings(&settings_path).unwrap();
        let enabled = settings["enabledPlugins"].as_array().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(
            enabled[0].as_str().unwrap(),
            "test-plugin@marketplace-name"
        );
    }

    #[test]
    fn test_enable_plugins_with_url_marketplace() {
        let dir = tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");

        let plugins =
            vec!["my-plugin@https://github.com/some/marketplace".to_string()];
        let results = enable_plugins(&plugins, &settings_path).unwrap();

        assert!(results.iter().any(|r| matches!(r, PluginResult::Enabled(_))));

        let settings = settings::load_settings(&settings_path).unwrap();

        // Marketplace URL should be registered.
        let markets = settings["extraKnownMarketplaces"].as_array().unwrap();
        assert_eq!(markets.len(), 1);
        assert_eq!(
            markets[0].as_str().unwrap(),
            "https://github.com/some/marketplace"
        );
    }

    #[test]
    fn test_enable_plugins_already_enabled() {
        let dir = tempdir().unwrap();
        let settings_path = dir.path().join("settings.json");

        // Pre-populate settings.
        let settings = serde_json::json!({
            "enabledPlugins": ["existing-plugin"]
        });
        settings::save_settings(&settings_path, &settings).unwrap();

        let plugins = vec!["existing-plugin".to_string()];
        let results = enable_plugins(&plugins, &settings_path).unwrap();

        assert!(results
            .iter()
            .any(|r| matches!(r, PluginResult::AlreadyEnabled(_))));
    }
}
