// Merge MCP server declarations into .mcp.json
// Implements: task 5.3

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use crate::registry::McpServerEntry;

/// Merge MCP server configs into a `.mcp.json` file.
///
/// - If the file exists, its contents are read and preserved.
/// - Declared servers are upserted into the `mcpServers` object.
/// - Existing entries NOT in `servers` are left untouched.
/// - The file is created (with parent dirs) if it does not exist.
pub fn merge_mcp_config(
    servers: &HashMap<String, McpServerEntry>,
    mcp_json_path: &Path,
) -> Result<()> {
    // Read existing file, or start from an empty object.
    let mut root: Value = if mcp_json_path.exists() {
        let contents = std::fs::read_to_string(mcp_json_path)
            .with_context(|| format!("Failed to read {}", mcp_json_path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", mcp_json_path.display()))?
    } else {
        json!({})
    };

    // Ensure the top-level object has a `mcpServers` key.
    let root_obj = root
        .as_object_mut()
        .context("Expected .mcp.json to be a JSON object")?;

    if !root_obj.contains_key("mcpServers") {
        root_obj.insert("mcpServers".to_string(), json!({}));
    }

    let mcp_servers = root_obj
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .context("Expected 'mcpServers' to be a JSON object")?;

    // Upsert each declared server.
    for (name, entry) in servers {
        let mut server_obj = Map::new();
        server_obj.insert("command".to_string(), Value::String(entry.command.clone()));
        server_obj.insert(
            "args".to_string(),
            Value::Array(entry.args.iter().map(|a| Value::String(a.clone())).collect()),
        );
        mcp_servers.insert(name.clone(), Value::Object(server_obj));
    }

    // Write back with pretty formatting.
    if let Some(parent) = mcp_json_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dirs for {}", mcp_json_path.display()))?;
    }

    let json_str = serde_json::to_string_pretty(&root)
        .context("Failed to serialize .mcp.json")?;

    std::fs::write(mcp_json_path, format!("{}\n", json_str))
        .with_context(|| format!("Failed to write {}", mcp_json_path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ResourceScope;
    use tempfile::tempdir;

    fn make_entry(cmd: &str, args: &[&str]) -> McpServerEntry {
        McpServerEntry {
            command: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            scope: ResourceScope::Shared,
        }
    }

    #[test]
    fn test_merge_creates_new_file() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        let mut servers = HashMap::new();
        servers.insert("openspec".to_string(), make_entry("npx", &["-y", "@fission-ai/openspec-mcp"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        let mcp = content["mcpServers"].as_object().unwrap();
        assert!(mcp.contains_key("openspec"));
        assert_eq!(mcp["openspec"]["command"], "npx");
    }

    #[test]
    fn test_merge_preserves_existing_entries() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        // Pre-populate with an existing server.
        let existing = json!({
            "mcpServers": {
                "existing-server": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        // Merge a new server.
        let mut servers = HashMap::new();
        servers.insert("new-server".to_string(), make_entry("python", &["serve.py"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        let mcp = content["mcpServers"].as_object().unwrap();
        assert!(mcp.contains_key("existing-server"), "existing server preserved");
        assert!(mcp.contains_key("new-server"), "new server added");
    }

    #[test]
    fn test_merge_updates_existing_server() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        // Pre-populate.
        let existing = json!({
            "mcpServers": {
                "my-server": {
                    "command": "old-cmd",
                    "args": []
                }
            }
        });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        // Merge with updated command.
        let mut servers = HashMap::new();
        servers.insert("my-server".to_string(), make_entry("new-cmd", &["--flag"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["my-server"]["command"], "new-cmd");
    }

    #[test]
    fn test_merge_empty_servers_preserves_file() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        let existing = json!({
            "mcpServers": {
                "keep-me": {
                    "command": "echo",
                    "args": []
                }
            }
        });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let servers: HashMap<String, McpServerEntry> = HashMap::new();
        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        assert!(content["mcpServers"]["keep-me"].is_object());
    }

    // -----------------------------------------------------------------
    // 10.2 — Additional MCP merge edge-case tests
    // -----------------------------------------------------------------

    #[test]
    fn test_merge_multiple_servers_at_once() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        let mut servers = HashMap::new();
        servers.insert("server-a".to_string(), make_entry("node", &["a.js"]));
        servers.insert("server-b".to_string(), make_entry("python", &["b.py"]));
        servers.insert("server-c".to_string(), make_entry("ruby", &["c.rb"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        let mcp = content["mcpServers"].as_object().unwrap();
        assert_eq!(mcp.len(), 3);
        assert!(mcp.contains_key("server-a"));
        assert!(mcp.contains_key("server-b"));
        assert!(mcp.contains_key("server-c"));
    }

    #[test]
    fn test_merge_preserves_extra_top_level_keys() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        // Pre-populate with extra top-level keys alongside mcpServers.
        let existing = json!({
            "version": 2,
            "mcpServers": {
                "old-server": {
                    "command": "echo",
                    "args": ["hello"]
                }
            },
            "customKey": "should-be-preserved"
        });
        std::fs::write(&mcp_path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();

        let mut servers = HashMap::new();
        servers.insert("new-server".to_string(), make_entry("npx", &["serve"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        // Extra keys should survive.
        assert_eq!(content["version"], 2);
        assert_eq!(content["customKey"], "should-be-preserved");
        // Both servers should be present.
        assert!(content["mcpServers"]["old-server"].is_object());
        assert!(content["mcpServers"]["new-server"].is_object());
    }

    #[test]
    fn test_merge_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join("nested").join("deep").join(".mcp.json");

        let mut servers = HashMap::new();
        servers.insert("srv".to_string(), make_entry("node", &["index.js"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();
        assert!(mcp_path.exists());
    }

    #[test]
    fn test_merge_to_file_without_mcp_servers_key() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        // Pre-populate with a JSON object that has no mcpServers key.
        std::fs::write(&mcp_path, r#"{"otherKey": true}"#).unwrap();

        let mut servers = HashMap::new();
        servers.insert("new-srv".to_string(), make_entry("python", &["-m", "serve"]));

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        assert_eq!(content["otherKey"], true);
        assert!(content["mcpServers"]["new-srv"].is_object());
    }

    #[test]
    fn test_merge_server_with_no_args() {
        let dir = tempdir().unwrap();
        let mcp_path = dir.path().join(".mcp.json");

        let mut servers = HashMap::new();
        servers.insert(
            "simple".to_string(),
            McpServerEntry {
                command: "my-server".to_string(),
                args: vec![],
                scope: ResourceScope::Global,
            },
        );

        merge_mcp_config(&servers, &mcp_path).unwrap();

        let content: Value =
            serde_json::from_str(&std::fs::read_to_string(&mcp_path).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["simple"]["command"], "my-server");
        let args = content["mcpServers"]["simple"]["args"].as_array().unwrap();
        assert!(args.is_empty());
    }
}
