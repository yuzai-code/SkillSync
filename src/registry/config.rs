// Global SkillSync configuration at ~/.skillsync/config.yaml

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Global configuration for SkillSync (lives at `~/.skillsync/config.yaml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Auto-sync enabled/disabled for the watcher.
    #[serde(default = "default_auto_sync", rename = "auto_sync")]
    pub auto_sync: bool,

    /// Default remote URL for the registry git repo.
    #[serde(rename = "registry_remote")]
    pub registry_remote: Option<String>,
}

fn default_auto_sync() -> bool {
    true
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            registry_remote: None,
        }
    }
}

impl GlobalConfig {
    /// Load global config from `~/.skillsync/config.yaml`.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    /// Load global config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: GlobalConfig = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse config YAML: {}", path.display()))?;
        Ok(config)
    }

    /// Save global config to `~/.skillsync/config.yaml`.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    /// Save global config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create parent dirs for {}", path.display()))?;
        }
        let yaml = serde_yaml::to_string(self)
            .context("Failed to serialize config to YAML")?;
        std::fs::write(path, yaml)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    /// Get the path to the global config file.
    fn path() -> Result<std::path::PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".skillsync").join("config.yaml"))
    }

    /// Set the registry remote URL.
    pub fn set_registry_remote(&mut self, url: String) {
        self.registry_remote = Some(url);
    }

    /// Check if auto-sync is enabled.
    pub fn is_auto_sync_enabled(&self) -> bool {
        self.auto_sync
    }

    /// Enable or disable auto-sync.
    pub fn set_auto_sync(&mut self, enabled: bool) {
        self.auto_sync = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert!(config.auto_sync);
        assert!(config.registry_remote.is_none());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.yaml");

        let mut config = GlobalConfig::default();
        config.set_auto_sync(false);
        config.set_registry_remote("git@github.com:user/registry.git".to_string());

        config.save_to(&path).unwrap();

        let loaded = GlobalConfig::load_from(&path).unwrap();
        assert!(!loaded.auto_sync);
        assert_eq!(loaded.registry_remote, Some("git@github.com:user/registry.git".to_string()));
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.yaml");

        let loaded = GlobalConfig::load_from(&path).unwrap();
        assert!(loaded.auto_sync);
        assert!(loaded.registry_remote.is_none());
    }
}
