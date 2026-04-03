// Manifest parsing, validation, and serialization
// Implements: tasks 2.1, 2.2

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Whether a skill is user-authored or pulled from a community marketplace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillType {
    Custom,
    Community,
}

/// Installation scope: global (all projects) or shared (synced via registry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceScope {
    Global,
    Shared,
}

// ---------------------------------------------------------------------------
// Skill
// ---------------------------------------------------------------------------

/// Provenance info for a community-sourced skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySource {
    pub marketplace: String,
    pub plugin: String,
    pub skill: String,
}

/// A single skill entry inside `manifest.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    /// custom | community
    #[serde(rename = "type")]
    pub skill_type: SkillType,

    /// global | shared
    pub scope: ResourceScope,

    /// Semver-ish version string.
    pub version: String,

    /// Relative path inside the registry (e.g. `resources/skills/yuque`).
    pub path: String,

    /// Human-readable one-liner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Searchable tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Only present for community skills.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CommunitySource>,

    /// SHA256 content hash used for integrity checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_hash: Option<String>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// A Claude-plugins marketplace plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub marketplace: String,
    pub version: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

/// An MCP server definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub command: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    pub scope: ResourceScope,
}

// ---------------------------------------------------------------------------
// Profile Reference (inside manifest.yaml)
// ---------------------------------------------------------------------------

/// Pointer to a profile YAML file stored in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileRef {
    pub path: String,
}

// ---------------------------------------------------------------------------
// Manifest (manifest.yaml)
// ---------------------------------------------------------------------------

/// Top-level representation of `manifest.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub skills: HashMap<String, SkillEntry>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub plugins: HashMap<String, PluginEntry>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, McpServerEntry>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, ProfileRef>,
}

impl Manifest {
    // -- Construction -------------------------------------------------------

    /// Create a minimal, empty manifest with `version: 1`.
    pub fn default_empty() -> Self {
        Self {
            version: 1,
            skills: HashMap::new(),
            plugins: HashMap::new(),
            mcp_servers: HashMap::new(),
            profiles: HashMap::new(),
        }
    }

    // -- I/O ----------------------------------------------------------------

    /// Read and deserialize a `Manifest` from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest file: {}", path.display()))?;
        let manifest: Manifest = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse manifest YAML: {}", path.display()))?;
        Ok(manifest)
    }

    /// Serialize and write the manifest to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dirs for {}", path.display()))?;
        }
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize manifest to YAML")?;
        std::fs::write(path, yaml)
            .with_context(|| format!("Failed to write manifest file: {}", path.display()))?;
        Ok(())
    }

    // -- Validation ---------------------------------------------------------

    /// Validate the manifest and return a list of human-readable error strings.
    /// Returns `Ok(())` when the manifest is valid.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors: Vec<String> = Vec::new();

        // Version must be 1 (only supported version so far).
        if self.version != 1 {
            errors.push(format!(
                "Unsupported manifest version: {} (expected 1)",
                self.version
            ));
        }

        // Validate skill entries.
        for (name, skill) in &self.skills {
            if skill.version.is_empty() {
                errors.push(format!("Skill '{}': version is required", name));
            }
            if skill.path.is_empty() {
                errors.push(format!("Skill '{}': path is required", name));
            }
            // Community skills must carry a source block.
            if skill.skill_type == SkillType::Community && skill.source.is_none() {
                errors.push(format!(
                    "Skill '{}': community skills must include a 'source' block",
                    name
                ));
            }
        }

        // Validate plugin entries.
        for (name, plugin) in &self.plugins {
            if plugin.marketplace.is_empty() {
                errors.push(format!("Plugin '{}': marketplace is required", name));
            }
            if plugin.version.is_empty() {
                errors.push(format!("Plugin '{}': version is required", name));
            }
        }

        // Validate MCP server entries.
        for (name, mcp) in &self.mcp_servers {
            if mcp.command.is_empty() {
                errors.push(format!("MCP server '{}': command is required", name));
            }
        }

        // Validate profile refs.
        for (name, profile_ref) in &self.profiles {
            if profile_ref.path.is_empty() {
                errors.push(format!("Profile '{}': path is required", name));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ---------------------------------------------------------------------------
// ProfileConfig (profiles/<name>.yaml)
// ---------------------------------------------------------------------------

/// A profile bundles a named set of skills, plugins, and MCP servers that can
/// be applied to a project in one shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<String>,
}

impl ProfileConfig {
    /// Read and deserialize a profile from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read profile file: {}", path.display()))?;
        let profile: ProfileConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse profile YAML: {}", path.display()))?;
        Ok(profile)
    }

    /// Serialize and write the profile to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dirs for {}", path.display()))?;
        }
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize profile to YAML")?;
        std::fs::write(path, yaml)
            .with_context(|| format!("Failed to write profile file: {}", path.display()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SkillSyncConfig (.claude/skillsync.yaml — per-project)
// ---------------------------------------------------------------------------

/// Per-project configuration that declares which profile / resources the
/// project uses.  Lives at `.claude/skillsync.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSyncConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<String>,
}

impl SkillSyncConfig {
    /// Read and deserialize a per-project config from a YAML file.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read skillsync config: {}", path.display()))?;
        let config: SkillSyncConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse skillsync config YAML: {}", path.display()))?;
        Ok(config)
    }

    /// Serialize and write the config to a YAML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dirs for {}", path.display()))?;
        }
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize skillsync config to YAML")?;
        std::fs::write(path, yaml)
            .with_context(|| format!("Failed to write skillsync config: {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_empty_manifest() {
        let m = Manifest::default_empty();
        assert_eq!(m.version, 1);
        assert!(m.skills.is_empty());
        assert!(m.plugins.is_empty());
        assert!(m.mcp_servers.is_empty());
        assert!(m.profiles.is_empty());
    }

    #[test]
    fn test_manifest_roundtrip() {
        let mut m = Manifest::default_empty();
        m.skills.insert(
            "yuque".into(),
            SkillEntry {
                skill_type: SkillType::Custom,
                scope: ResourceScope::Global,
                version: "1.0.4".into(),
                path: "resources/skills/yuque".into(),
                description: Some("Yuque docs".into()),
                tags: vec!["docs".into(), "yuque".into()],
                source: None,
                backup_hash: None,
            },
        );
        m.plugins.insert(
            "superpowers".into(),
            PluginEntry {
                marketplace: "claude-plugins-official".into(),
                version: "72b975468071".into(),
                git_sha: Some("bd041495".into()),
                repo: None,
            },
        );
        m.mcp_servers.insert(
            "openspec".into(),
            McpServerEntry {
                command: "npx".into(),
                args: vec!["-y".into(), "@fission-ai/openspec-mcp".into()],
                scope: ResourceScope::Global,
            },
        );
        m.profiles.insert(
            "default".into(),
            ProfileRef {
                path: "profiles/default.yaml".into(),
            },
        );

        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        m.save(&path).unwrap();

        let loaded = Manifest::load(&path).unwrap();
        assert_eq!(loaded.version, 1);
        assert!(loaded.skills.contains_key("yuque"));
        assert!(loaded.plugins.contains_key("superpowers"));
        assert!(loaded.mcp_servers.contains_key("openspec"));
        assert!(loaded.profiles.contains_key("default"));
    }

    #[test]
    fn test_manifest_validate_ok() {
        let m = Manifest::default_empty();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_manifest_validate_errors() {
        let mut m = Manifest::default_empty();
        m.version = 99;
        m.skills.insert(
            "bad".into(),
            SkillEntry {
                skill_type: SkillType::Community,
                scope: ResourceScope::Shared,
                version: "".into(),
                path: "".into(),
                description: None,
                tags: vec![],
                source: None, // missing for community
                backup_hash: None,
            },
        );
        m.plugins.insert(
            "bad-plugin".into(),
            PluginEntry {
                marketplace: "".into(),
                version: "".into(),
                git_sha: None,
                repo: None,
            },
        );

        let errs = m.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("version")));
        assert!(errs.iter().any(|e| e.contains("bad") && e.contains("version is required")));
        assert!(errs.iter().any(|e| e.contains("bad") && e.contains("path is required")));
        assert!(errs.iter().any(|e| e.contains("community")));
        assert!(errs.iter().any(|e| e.contains("bad-plugin")));
    }

    #[test]
    fn test_profile_roundtrip() {
        let p = ProfileConfig {
            name: "agent-dev".into(),
            description: Some("Agent workflow development".into()),
            skills: vec!["openspec-expert".into()],
            plugins: vec!["superpowers@claude-plugins-official".into()],
            mcp: vec!["openspec".into()],
        };

        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        p.save(&path).unwrap();

        let loaded = ProfileConfig::load(&path).unwrap();
        assert_eq!(loaded.name, "agent-dev");
        assert_eq!(loaded.skills, vec!["openspec-expert"]);
    }

    #[test]
    fn test_skillsync_config_roundtrip() {
        let c = SkillSyncConfig {
            profile: Some("agent-dev".into()),
            skills: vec!["openspec-expert".into()],
            plugins: vec!["superpowers@claude-plugins-official".into()],
            mcp: vec!["openspec".into()],
        };

        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        c.save(&path).unwrap();

        let loaded = SkillSyncConfig::load(&path).unwrap();
        assert_eq!(loaded.profile, Some("agent-dev".into()));
        assert_eq!(loaded.mcp, vec!["openspec"]);
    }

    // -----------------------------------------------------------------
    // 10.1 — Additional manifest edge-case tests
    // -----------------------------------------------------------------

    #[test]
    fn test_manifest_load_empty_file_fails() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::fs::write(&path, "").unwrap();

        let result = Manifest::load(&path);
        assert!(result.is_err(), "Loading an empty YAML file should fail");
    }

    #[test]
    fn test_manifest_load_invalid_yaml_fails() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::fs::write(&path, "{{{{not valid yaml").unwrap();

        let result = Manifest::load(&path);
        assert!(result.is_err(), "Loading invalid YAML should fail");
    }

    #[test]
    fn test_manifest_load_missing_version_field() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // YAML with skills but no version key — serde should error because
        // `version` is not optional.
        std::fs::write(&path, "skills: {}\n").unwrap();

        let result = Manifest::load(&path);
        assert!(result.is_err(), "Manifest without version field should fail");
    }

    #[test]
    fn test_manifest_validate_wrong_version() {
        let mut m = Manifest::default_empty();
        m.version = 0;
        let errs = m.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("Unsupported manifest version")));
    }

    #[test]
    fn test_manifest_validate_empty_mcp_command() {
        let mut m = Manifest::default_empty();
        m.mcp_servers.insert(
            "bad-mcp".into(),
            McpServerEntry {
                command: "".into(),
                args: vec![],
                scope: ResourceScope::Global,
            },
        );

        let errs = m.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("bad-mcp") && e.contains("command is required")));
    }

    #[test]
    fn test_manifest_validate_empty_profile_path() {
        let mut m = Manifest::default_empty();
        m.profiles.insert(
            "empty-path".into(),
            ProfileRef { path: "".into() },
        );

        let errs = m.validate().unwrap_err();
        assert!(errs.iter().any(|e| e.contains("empty-path") && e.contains("path is required")));
    }

    #[test]
    fn test_manifest_validate_custom_skill_without_source_ok() {
        let mut m = Manifest::default_empty();
        m.skills.insert(
            "custom-skill".into(),
            SkillEntry {
                skill_type: SkillType::Custom,
                scope: ResourceScope::Global,
                version: "1.0.0".into(),
                path: "resources/skills/custom-skill".into(),
                description: None,
                tags: vec![],
                source: None,
                backup_hash: None,
            },
        );

        assert!(m.validate().is_ok(), "Custom skills don't need a source block");
    }

    #[test]
    fn test_manifest_validate_community_skill_with_source_ok() {
        let mut m = Manifest::default_empty();
        m.skills.insert(
            "comm-skill".into(),
            SkillEntry {
                skill_type: SkillType::Community,
                scope: ResourceScope::Shared,
                version: "2.0.0".into(),
                path: "resources/skills/comm-skill".into(),
                description: Some("A community skill".into()),
                tags: vec!["community".into()],
                source: Some(CommunitySource {
                    marketplace: "claude-plugins-official".into(),
                    plugin: "some-plugin".into(),
                    skill: "comm-skill".into(),
                }),
                backup_hash: None,
            },
        );

        assert!(m.validate().is_ok(), "Community skill with source should validate");
    }

    #[test]
    fn test_manifest_load_nonexistent_file() {
        let result = Manifest::load(std::path::Path::new("/nonexistent/manifest.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_skillsync_config_empty_fields() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // Minimal YAML: empty document
        std::fs::write(&path, "{}\n").unwrap();

        let loaded = SkillSyncConfig::load(&path).unwrap();
        assert!(loaded.profile.is_none());
        assert!(loaded.skills.is_empty());
        assert!(loaded.plugins.is_empty());
        assert!(loaded.mcp.is_empty());
    }

    #[test]
    fn test_skillsync_config_partial_fields() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        std::fs::write(&path, "skills:\n  - my-skill\n").unwrap();

        let loaded = SkillSyncConfig::load(&path).unwrap();
        assert!(loaded.profile.is_none());
        assert_eq!(loaded.skills, vec!["my-skill"]);
        assert!(loaded.plugins.is_empty());
        assert!(loaded.mcp.is_empty());
    }

    #[test]
    fn test_profile_config_minimal() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let p = ProfileConfig {
            name: "minimal".into(),
            description: None,
            skills: vec![],
            plugins: vec![],
            mcp: vec![],
        };

        p.save(&path).unwrap();
        let loaded = ProfileConfig::load(&path).unwrap();
        assert_eq!(loaded.name, "minimal");
        assert!(loaded.description.is_none());
        assert!(loaded.skills.is_empty());
        assert!(loaded.plugins.is_empty());
        assert!(loaded.mcp.is_empty());
    }

    #[test]
    fn test_profile_config_load_empty_yaml_fails() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        std::fs::write(&path, "").unwrap();

        // ProfileConfig requires `name` field, so empty YAML should fail.
        let result = ProfileConfig::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_manifest_save_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let deep_path = dir.path().join("a").join("b").join("c").join("manifest.yaml");

        let m = Manifest::default_empty();
        m.save(&deep_path).unwrap();
        assert!(deep_path.exists());
    }

    #[test]
    fn test_manifest_multiple_validation_errors() {
        let mut m = Manifest::default_empty();
        m.version = 99;

        // Add a bad skill
        m.skills.insert(
            "s1".into(),
            SkillEntry {
                skill_type: SkillType::Custom,
                scope: ResourceScope::Global,
                version: "".into(),
                path: "".into(),
                description: None,
                tags: vec![],
                source: None,
                backup_hash: None,
            },
        );

        // Add a bad plugin
        m.plugins.insert(
            "p1".into(),
            PluginEntry {
                marketplace: "".into(),
                version: "".into(),
                git_sha: None,
                repo: None,
            },
        );

        // Add a bad MCP server
        m.mcp_servers.insert(
            "m1".into(),
            McpServerEntry {
                command: "".into(),
                args: vec![],
                scope: ResourceScope::Global,
            },
        );

        // Add a bad profile
        m.profiles.insert(
            "pr1".into(),
            ProfileRef { path: "".into() },
        );

        let errs = m.validate().unwrap_err();
        // Should have at least 7 errors: version + 2 for skill + 2 for plugin + 1 for mcp + 1 for profile
        assert!(errs.len() >= 7, "Expected at least 7 errors, got {}: {:?}", errs.len(), errs);
    }
}
