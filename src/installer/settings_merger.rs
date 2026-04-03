// skillsync.yaml and skillsync.lock generation utilities
// Implements: tasks 5.5, 5.6

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::registry::SkillSyncConfig;

// ---------------------------------------------------------------------------
// Lock file types
// ---------------------------------------------------------------------------

/// A single entry in `skillsync.lock` recording the installed version and
/// content hash of a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub resource_type: String,
    pub version: String,
    pub hash: String,
}

/// The top-level lock file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    /// Lock file format version.
    pub version: u32,
    /// Installed resources with their hashes.
    pub resources: Vec<LockEntry>,
}

// ---------------------------------------------------------------------------
// skillsync.yaml generation (task 5.5)
// ---------------------------------------------------------------------------

/// Generate (or overwrite) a `skillsync.yaml` file from a `SkillSyncConfig`.
///
/// Parent directories are created if they do not exist.
pub fn write_skillsync_config(config: &SkillSyncConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dirs for {}", path.display()))?;
    }

    let yaml = serde_yaml::to_string(config)
        .context("Failed to serialize skillsync config to YAML")?;

    std::fs::write(path, yaml)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// skillsync.lock generation (task 5.6)
// ---------------------------------------------------------------------------

/// Generate (or overwrite) a `skillsync.lock` file recording installed
/// versions and content hashes.
///
/// The lock file uses YAML for human readability and easy diffing.
pub fn write_lock_file(installed: &[LockEntry], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dirs for {}", path.display()))?;
    }

    let lock = LockFile {
        version: 1,
        resources: installed.to_vec(),
    };

    let yaml = serde_yaml::to_string(&lock)
        .context("Failed to serialize lock file to YAML")?;

    std::fs::write(path, yaml)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Read an existing lock file from disk.
///
/// Returns `None` if the file does not exist.
pub fn read_lock_file(path: &Path) -> Result<Option<LockFile>> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let lock: LockFile = serde_yaml::from_str(&contents)
        .with_context(|| format!("Failed to parse lock file: {}", path.display()))?;
    Ok(Some(lock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_and_read_skillsync_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude").join("skillsync.yaml");

        let config = SkillSyncConfig {
            profile: Some("agent-dev".into()),
            skills: vec!["openspec-expert".into(), "yuque".into()],
            plugins: vec!["superpowers@claude-plugins-official".into()],
            mcp: vec!["openspec".into()],
        };

        write_skillsync_config(&config, &path).unwrap();
        assert!(path.exists());

        let loaded = SkillSyncConfig::load(&path).unwrap();
        assert_eq!(loaded.profile, Some("agent-dev".into()));
        assert_eq!(loaded.skills, vec!["openspec-expert", "yuque"]);
        assert_eq!(loaded.mcp, vec!["openspec"]);
    }

    #[test]
    fn test_write_skillsync_config_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("skillsync.yaml");

        let config = SkillSyncConfig {
            profile: None,
            skills: vec![],
            plugins: vec![],
            mcp: vec![],
        };

        write_skillsync_config(&config, &path).unwrap();
        assert!(path.exists());

        let loaded = SkillSyncConfig::load(&path).unwrap();
        assert!(loaded.profile.is_none());
        assert!(loaded.skills.is_empty());
    }

    #[test]
    fn test_write_and_read_lock_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("skillsync.lock");

        let entries = vec![
            LockEntry {
                name: "yuque".into(),
                resource_type: "skill".into(),
                version: "1.0.4".into(),
                hash: "sha256:abc123".into(),
            },
            LockEntry {
                name: "openspec".into(),
                resource_type: "mcp_server".into(),
                version: "0.1.0".into(),
                hash: "sha256:def456".into(),
            },
        ];

        write_lock_file(&entries, &path).unwrap();
        assert!(path.exists());

        let lock = read_lock_file(&path).unwrap().unwrap();
        assert_eq!(lock.version, 1);
        assert_eq!(lock.resources.len(), 2);
        assert_eq!(lock.resources[0].name, "yuque");
        assert_eq!(lock.resources[1].hash, "sha256:def456");
    }

    #[test]
    fn test_read_lock_file_nonexistent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("no-such-file.lock");

        let result = read_lock_file(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_lock_file_roundtrip_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("skillsync.lock");

        write_lock_file(&[], &path).unwrap();

        let lock = read_lock_file(&path).unwrap().unwrap();
        assert_eq!(lock.version, 1);
        assert!(lock.resources.is_empty());
    }
}
