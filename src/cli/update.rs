use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;

use crate::registry::{compute_hash, copy_resource, Manifest};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context(
        "Could not determine home directory. Ensure the HOME environment variable is set.",
    )?;
    Ok(home.join(".skillsync").join("registry"))
}

/// Bump the patch component of a semver-ish version string.
///
/// Examples: "1.0.4" -> "1.0.5", "0.1.0" -> "0.1.1", "latest" -> "0.0.1"
fn bump_patch(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() == 3 {
        if let Ok(patch) = parts[2].parse::<u64>() {
            return format!("{}.{}.{}", parts[0], parts[1], patch + 1);
        }
    }
    // If version is not a valid semver, start fresh.
    "0.0.1".to_string()
}

/// Update a resource already registered in the manifest.
///
/// For skills: if `new_path` is provided, re-copy the skill files from that
/// path and recompute the hash. Otherwise just bump the version.
///
/// For plugins / MCP servers: bump the version field.
pub fn run(name: &str) -> Result<()> {
    run_inner(name, None)
}

/// Inner implementation that also accepts an optional new source path (used by
/// tests and future CLI extensions).
pub fn run_inner(name: &str, new_path: Option<&str>) -> Result<()> {
    let registry = registry_root()?;
    let manifest_file = registry.join("manifest.yaml");
    let mut manifest = Manifest::load(&manifest_file).context(
        "Failed to load manifest. Run 'skillsync init' to create a registry first.",
    )?;

    // --- Skill ---
    if let Some(entry) = manifest.skills.get_mut(name) {
        if let Some(src) = new_path {
            let source = Path::new(src);
            if !source.exists() {
                bail!(
                    "Source path '{}' does not exist. Provide a valid path to the updated skill.",
                    src
                );
            }

            let dest = registry.join(&entry.path);
            copy_resource(source, &dest)
                .with_context(|| format!("Failed to copy updated skill to {}", dest.display()))?;

            let hash = compute_hash(&dest)
                .with_context(|| format!("Failed to compute hash for {}", dest.display()))?;
            entry.backup_hash = Some(hash);
        }

        let old_version = entry.version.clone();
        entry.version = bump_patch(&old_version);
        let new_version = entry.version.clone();

        manifest
            .save(&manifest_file)
            .context("Failed to save manifest after update")?;

        println!(
            "{} Updated skill '{}': {} -> {}",
            style("✓").green().bold(),
            style(name).cyan(),
            style(&old_version).dim(),
            style(&new_version).green()
        );
        return Ok(());
    }

    // --- Plugin ---
    if let Some(entry) = manifest.plugins.get_mut(name) {
        let old_version = entry.version.clone();
        entry.version = bump_patch(&old_version);
        let new_version = entry.version.clone();

        manifest
            .save(&manifest_file)
            .context("Failed to save manifest after update")?;

        println!(
            "{} Updated plugin '{}': {} -> {}",
            style("✓").green().bold(),
            style(name).cyan(),
            style(&old_version).dim(),
            style(&new_version).green()
        );
        return Ok(());
    }

    // --- MCP Server ---
    if manifest.mcp_servers.contains_key(name) {
        // MCP servers don't have a version field to bump.
        // Just report that it is already registered.
        println!(
            "{} MCP server '{}' is already registered. Update its command/args via 'skillsync remove' + 'skillsync add'.",
            style("·").dim(),
            style(name).cyan()
        );
        return Ok(());
    }

    // --- Not found ---
    bail!(
        "Resource '{}' not found in the registry.\n\
         Use 'skillsync list' to see all registered resources.",
        name
    );
}

/// Update a resource using explicit registry/manifest paths (for testing).
pub fn update_in_manifest(
    name: &str,
    manifest: &mut Manifest,
    registry_root: &Path,
    new_path: Option<&str>,
) -> Result<()> {
    // --- Skill ---
    if let Some(entry) = manifest.skills.get_mut(name) {
        if let Some(src) = new_path {
            let source = Path::new(src);
            if !source.exists() {
                bail!(
                    "Source path '{}' does not exist. Provide a valid path to the updated skill.",
                    src
                );
            }

            let dest = registry_root.join(&entry.path);
            copy_resource(source, &dest)
                .with_context(|| format!("Failed to copy updated skill to {}", dest.display()))?;

            let hash = compute_hash(&dest)
                .with_context(|| format!("Failed to compute hash for {}", dest.display()))?;
            entry.backup_hash = Some(hash);
        }

        entry.version = bump_patch(&entry.version);
        return Ok(());
    }

    // --- Plugin ---
    if let Some(entry) = manifest.plugins.get_mut(name) {
        entry.version = bump_patch(&entry.version);
        return Ok(());
    }

    // --- MCP Server ---
    if manifest.mcp_servers.contains_key(name) {
        return Ok(());
    }

    bail!(
        "Resource '{}' not found in the registry.\n\
         Use 'skillsync list' to see all registered resources.",
        name
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_patch_semver() {
        assert_eq!(bump_patch("1.0.4"), "1.0.5");
        assert_eq!(bump_patch("0.1.0"), "0.1.1");
        assert_eq!(bump_patch("2.3.99"), "2.3.100");
    }

    #[test]
    fn test_bump_patch_non_semver() {
        assert_eq!(bump_patch("latest"), "0.0.1");
        assert_eq!(bump_patch(""), "0.0.1");
        assert_eq!(bump_patch("1.0"), "0.0.1");
    }

    #[test]
    fn test_update_skill_in_manifest() {
        use crate::registry::{Manifest, ResourceScope, SkillEntry, SkillType};
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let registry_root = dir.path().join("registry");
        fs::create_dir_all(registry_root.join("resources/skills/test-skill")).unwrap();
        fs::write(
            registry_root.join("resources/skills/test-skill/CLAUDE.md"),
            "# Test",
        )
        .unwrap();

        let mut manifest = Manifest::default_empty();
        manifest.skills.insert(
            "test-skill".into(),
            SkillEntry {
                skill_type: SkillType::Custom,
                scope: ResourceScope::Shared,
                version: "1.0.0".into(),
                path: "resources/skills/test-skill".into(),
                description: None,
                tags: vec![],
                source: None,
                backup_hash: None,
            },
        );

        update_in_manifest("test-skill", &mut manifest, &registry_root, None).unwrap();
        assert_eq!(manifest.skills["test-skill"].version, "1.0.1");
    }

    #[test]
    fn test_update_skill_with_new_path() {
        use crate::registry::{Manifest, ResourceScope, SkillEntry, SkillType};
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let registry_root = dir.path().join("registry");
        fs::create_dir_all(registry_root.join("resources/skills/my-skill")).unwrap();
        fs::write(
            registry_root.join("resources/skills/my-skill/CLAUDE.md"),
            "# Old content",
        )
        .unwrap();

        // Create a new source with different content.
        let new_src = dir.path().join("new-source");
        fs::create_dir_all(&new_src).unwrap();
        fs::write(new_src.join("CLAUDE.md"), "# Updated content").unwrap();

        let mut manifest = Manifest::default_empty();
        manifest.skills.insert(
            "my-skill".into(),
            SkillEntry {
                skill_type: SkillType::Custom,
                scope: ResourceScope::Shared,
                version: "0.1.0".into(),
                path: "resources/skills/my-skill".into(),
                description: None,
                tags: vec![],
                source: None,
                backup_hash: None,
            },
        );

        update_in_manifest(
            "my-skill",
            &mut manifest,
            &registry_root,
            Some(new_src.to_str().unwrap()),
        )
        .unwrap();

        assert_eq!(manifest.skills["my-skill"].version, "0.1.1");
        assert!(manifest.skills["my-skill"].backup_hash.is_some());

        // Verify the files were actually copied.
        let content = fs::read_to_string(
            registry_root.join("resources/skills/my-skill/CLAUDE.md"),
        )
        .unwrap();
        assert_eq!(content, "# Updated content");
    }

    #[test]
    fn test_update_plugin_in_manifest() {
        use crate::registry::{Manifest, PluginEntry};

        let dir = tempfile::tempdir().unwrap();

        let mut manifest = Manifest::default_empty();
        manifest.plugins.insert(
            "superpowers".into(),
            PluginEntry {
                marketplace: "claude-plugins-official".into(),
                version: "1.2.3".into(),
                git_sha: None,
                repo: None,
            },
        );

        update_in_manifest("superpowers", &mut manifest, dir.path(), None).unwrap();
        assert_eq!(manifest.plugins["superpowers"].version, "1.2.4");
    }

    #[test]
    fn test_update_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = Manifest::default_empty();
        let result = update_in_manifest("nonexistent", &mut manifest, dir.path(), None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"));
    }
}
