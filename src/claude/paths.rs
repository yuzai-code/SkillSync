// Claude Code path discovery
// Implements: task 1.4

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Paths related to Claude Code's global configuration.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClaudePaths {
    /// `~/.claude/`
    pub home: PathBuf,
    /// `~/.claude/skills/`
    pub skills_dir: PathBuf,
    /// `~/.claude/plugins/`
    pub plugins_dir: PathBuf,
    /// `~/.claude/.mcp.json`
    pub mcp_json: PathBuf,
    /// `~/.claude/settings.json`
    pub settings_json: PathBuf,
}

impl ClaudePaths {
    /// Discover global Claude Code paths based on the user's home directory.
    pub fn global() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let claude_home = home_dir.join(".claude");
        Ok(Self {
            skills_dir: claude_home.join("skills"),
            plugins_dir: claude_home.join("plugins"),
            mcp_json: claude_home.join(".mcp.json"),
            settings_json: claude_home.join("settings.json"),
            home: claude_home,
        })
    }

    /// Create necessary directories if they don't exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.home)
            .with_context(|| format!("Failed to create {}", self.home.display()))?;
        std::fs::create_dir_all(&self.skills_dir)
            .with_context(|| format!("Failed to create {}", self.skills_dir.display()))?;
        Ok(())
    }

    /// Returns `true` if the `~/.claude/` directory exists on disk.
    pub fn exists(&self) -> bool {
        self.home.exists()
    }
}

/// Paths related to Claude Code within a specific project.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    /// Project root directory.
    pub root: PathBuf,
    /// `<project>/.claude/`
    pub claude_dir: PathBuf,
    /// `<project>/.claude/skills/`
    pub skills_dir: PathBuf,
    /// `<project>/.claude/skillsync.yaml`
    pub skillsync_yaml: PathBuf,
    /// `<project>/.claude/skillsync.lock`
    pub skillsync_lock: PathBuf,
    /// `<project>/.mcp.json`
    pub mcp_json: PathBuf,
}

impl ProjectPaths {
    /// Derive project-specific Claude Code paths from a project root.
    pub fn new(root: &Path) -> Self {
        let claude_dir = root.join(".claude");
        Self {
            skills_dir: claude_dir.join("skills"),
            skillsync_yaml: claude_dir.join("skillsync.yaml"),
            skillsync_lock: claude_dir.join("skillsync.lock"),
            mcp_json: root.join(".mcp.json"),
            root: root.to_path_buf(),
            claude_dir,
        }
    }

    /// Create necessary directories if they don't exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(&self.claude_dir)
            .with_context(|| format!("Failed to create {}", self.claude_dir.display()))?;
        std::fs::create_dir_all(&self.skills_dir)
            .with_context(|| format!("Failed to create {}", self.skills_dir.display()))?;
        Ok(())
    }

    /// Check if this project has a skillsync.yaml configuration.
    pub fn has_config(&self) -> bool {
        self.skillsync_yaml.exists()
    }
}

/// Paths for SkillSync's own data (`~/.skillsync/`).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SkillSyncPaths {
    /// `~/.skillsync/`
    pub home: PathBuf,
    /// `~/.skillsync/registry/`
    pub registry: PathBuf,
    /// `~/.skillsync/registry/manifest.yaml`
    pub manifest: PathBuf,
    /// `~/.skillsync/registry/resources/`
    pub resources: PathBuf,
    /// `~/.skillsync/registry/resources/skills/`
    pub skills_dir: PathBuf,
    /// `~/.skillsync/registry/resources/plugins/`
    pub plugins_dir: PathBuf,
    /// `~/.skillsync/registry/resources/mcp/`
    pub mcp_dir: PathBuf,
    /// `~/.skillsync/registry/profiles/`
    pub profiles_dir: PathBuf,
    /// `~/.skillsync/state.db`
    pub state_db: PathBuf,
}

#[allow(dead_code)]
impl SkillSyncPaths {
    /// Discover SkillSync paths based on the user's home directory.
    pub fn resolve() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        Ok(Self::with_root(home_dir.join(".skillsync")))
    }

    /// Create a `SkillSyncPaths` instance rooted at a custom directory.
    /// Useful for testing without touching the real `~/.skillsync/`.
    pub fn with_root(root: PathBuf) -> Self {
        let registry = root.join("registry");
        let resources = registry.join("resources");
        Self {
            manifest: registry.join("manifest.yaml"),
            skills_dir: resources.join("skills"),
            plugins_dir: resources.join("plugins"),
            mcp_dir: resources.join("mcp"),
            profiles_dir: registry.join("profiles"),
            state_db: root.join("state.db"),
            resources,
            registry,
            home: root,
        }
    }

    /// Returns `true` if the `~/.skillsync/` directory exists on disk.
    pub fn exists(&self) -> bool {
        self.home.exists()
    }

    /// Check if the registry has been initialized.
    pub fn registry_exists(&self) -> bool {
        self.registry.is_dir() && self.manifest.exists()
    }

    /// Create the full registry directory structure.
    pub fn ensure_registry_dirs(&self) -> Result<()> {
        for dir in [
            &self.skills_dir,
            &self.plugins_dir,
            &self.mcp_dir,
            &self.profiles_dir,
        ] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create {}", dir.display()))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -- ClaudePaths --------------------------------------------------------

    #[test]
    fn test_global_paths_structure() {
        let paths = ClaudePaths::global().unwrap();

        assert!(paths.home.ends_with(".claude"));
        assert!(paths.skills_dir.ends_with("skills"));
        assert!(paths.plugins_dir.ends_with("plugins"));
        assert!(paths.settings_json.ends_with("settings.json"));
        assert!(paths.mcp_json.ends_with(".mcp.json"));

        // skills_dir, plugins_dir, mcp_json, settings_json are children of home
        assert_eq!(paths.skills_dir.parent().unwrap(), paths.home);
        assert_eq!(paths.plugins_dir.parent().unwrap(), paths.home);
        assert_eq!(paths.mcp_json.parent().unwrap(), paths.home);
        assert_eq!(paths.settings_json.parent().unwrap(), paths.home);
    }

    #[test]
    fn test_global_ensure_dirs() {
        let dir = tempdir().unwrap();
        let claude_home = dir.path().join(".claude");
        let paths = ClaudePaths {
            skills_dir: claude_home.join("skills"),
            plugins_dir: claude_home.join("plugins"),
            mcp_json: claude_home.join(".mcp.json"),
            settings_json: claude_home.join("settings.json"),
            home: claude_home.clone(),
        };

        assert!(!paths.exists());
        paths.ensure_dirs().unwrap();
        assert!(paths.exists());
        assert!(paths.skills_dir.exists());
    }

    // -- ProjectPaths -------------------------------------------------------

    #[test]
    fn test_project_paths_structure() {
        let dir = tempdir().unwrap();
        let project = dir.path();
        let paths = ProjectPaths::new(project);

        assert_eq!(paths.root, project);
        assert_eq!(paths.claude_dir, project.join(".claude"));
        assert_eq!(paths.skills_dir, project.join(".claude/skills"));
        assert_eq!(paths.skillsync_yaml, project.join(".claude/skillsync.yaml"));
        assert_eq!(paths.skillsync_lock, project.join(".claude/skillsync.lock"));
        // Project MCP config lives at the project root, not inside .claude/
        assert_eq!(paths.mcp_json, project.join(".mcp.json"));
    }

    #[test]
    fn test_project_ensure_dirs() {
        let dir = tempdir().unwrap();
        let paths = ProjectPaths::new(dir.path());

        assert!(!paths.claude_dir.exists());
        paths.ensure_dirs().unwrap();
        assert!(paths.claude_dir.exists());
        assert!(paths.skills_dir.exists());
    }

    #[test]
    fn test_project_has_config() {
        let dir = tempdir().unwrap();
        let paths = ProjectPaths::new(dir.path());

        assert!(!paths.has_config());

        // Create the config file
        paths.ensure_dirs().unwrap();
        std::fs::write(&paths.skillsync_yaml, "profile: default\n").unwrap();
        assert!(paths.has_config());
    }

    // -- SkillSyncPaths -----------------------------------------------------

    #[test]
    fn test_skillsync_paths_resolve() {
        let paths = SkillSyncPaths::resolve().unwrap();
        assert!(paths.home.ends_with(".skillsync"));
        assert!(paths.registry.ends_with("registry"));
        assert!(paths.state_db.ends_with("state.db"));
    }

    #[test]
    fn test_skillsync_paths_with_root() {
        let dir = tempdir().unwrap();
        let paths = SkillSyncPaths::with_root(dir.path().to_path_buf());

        assert_eq!(paths.home, dir.path());
        assert_eq!(paths.registry, dir.path().join("registry"));
        assert_eq!(paths.manifest, dir.path().join("registry/manifest.yaml"));
        assert_eq!(paths.resources, dir.path().join("registry/resources"));
        assert_eq!(paths.skills_dir, dir.path().join("registry/resources/skills"));
        assert_eq!(paths.plugins_dir, dir.path().join("registry/resources/plugins"));
        assert_eq!(paths.mcp_dir, dir.path().join("registry/resources/mcp"));
        assert_eq!(paths.profiles_dir, dir.path().join("registry/profiles"));
        assert_eq!(paths.state_db, dir.path().join("state.db"));
    }

    #[test]
    fn test_skillsync_ensure_registry_dirs() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("newsync");
        let paths = SkillSyncPaths::with_root(root);

        assert!(!paths.exists());
        paths.ensure_registry_dirs().unwrap();
        assert!(paths.exists());
        assert!(paths.skills_dir.exists());
        assert!(paths.plugins_dir.exists());
        assert!(paths.mcp_dir.exists());
        assert!(paths.profiles_dir.exists());
    }

    #[test]
    fn test_skillsync_registry_exists() {
        let dir = tempdir().unwrap();
        let paths = SkillSyncPaths::with_root(dir.path().to_path_buf());

        assert!(!paths.registry_exists());

        // Create dirs but not manifest
        paths.ensure_registry_dirs().unwrap();
        assert!(!paths.registry_exists());

        // Create the manifest file
        std::fs::write(&paths.manifest, "version: 1\n").unwrap();
        assert!(paths.registry_exists());
    }
}
