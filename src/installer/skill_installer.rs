// Copy skill files from registry to target directories
// Implements: tasks 5.1, 5.2

use std::fmt;
use std::path::Path;

use anyhow::{bail, Context, Result};

#[allow(unused_imports)]
use crate::t;
use crate::i18n::Msg;
use crate::registry::{compute_hash, copy_resource, Manifest, ResourceScope};

/// Outcome of a single skill installation attempt.
#[derive(Debug, Clone)]
pub enum InstallResult {
    /// Skill was freshly installed (did not exist at target).
    Installed(String),
    /// Skill was updated (hash mismatch, re-copied).
    Updated(String),
    /// Skill was skipped (hashes match, already up-to-date).
    Skipped(String),
}

impl InstallResult {
    /// Return the skill name regardless of variant.
    pub fn name(&self) -> &str {
        match self {
            InstallResult::Installed(n) => n,
            InstallResult::Updated(n) => n,
            InstallResult::Skipped(n) => n,
        }
    }
}

impl fmt::Display for InstallResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallResult::Installed(n) => write!(f, "{}", t!(Msg::InstallerInstalled { name: n.clone() })),
            InstallResult::Updated(n) => write!(f, "{}", t!(Msg::InstallerUpdated { name: n.clone() })),
            InstallResult::Skipped(n) => write!(f, "{}", t!(Msg::InstallerSkipped { name: n.clone() })),
        }
    }
}

/// Install a single skill from the registry to a target skills directory.
///
/// The skill is copied into `<target_skills_dir>/<skill_name>/`. Before copying,
/// the source and destination hashes are compared; if they match the copy is
/// skipped.
pub fn install_skill(
    registry_skill_path: &Path,
    target_skills_dir: &Path,
    skill_name: &str,
) -> Result<InstallResult> {
    if !registry_skill_path.exists() {
        bail!("{}", t!(Msg::InstallerSkillPathNotExist { path: registry_skill_path.display().to_string() }));
    }

    // Ensure the target skills directory exists.
    std::fs::create_dir_all(target_skills_dir)
        .with_context(|| format!("Failed to create skills dir: {}", target_skills_dir.display()))?;

    let dest = target_skills_dir.join(skill_name);

    // Compute source hash.
    let source_hash = compute_hash(registry_skill_path)
        .with_context(|| format!("Failed to hash source: {}", registry_skill_path.display()))?;

    // If destination already exists, compare hashes to decide whether to skip.
    if dest.exists() {
        let dest_hash = compute_hash(&dest)
            .with_context(|| format!("Failed to hash destination: {}", dest.display()))?;

        if source_hash == dest_hash {
            return Ok(InstallResult::Skipped(skill_name.to_string()));
        }

        // Hashes differ — update.
        copy_resource(registry_skill_path, &dest).with_context(|| {
            t!(Msg::InstallerUpdateFailed {
                name: skill_name.to_string(),
                path: dest.display().to_string()
            })
        })?;
        return Ok(InstallResult::Updated(skill_name.to_string()));
    }

    // Destination does not exist — fresh install.
    copy_resource(registry_skill_path, &dest).with_context(|| {
        t!(Msg::InstallerInstallFailed {
            name: skill_name.to_string(),
            path: dest.display().to_string()
        })
    })?;
    Ok(InstallResult::Installed(skill_name.to_string()))
}

/// Install all global-scope skills from the manifest to `~/.claude/skills/`.
pub fn install_global_skills(
    manifest: &Manifest,
    registry_root: &Path,
    global_skills_dir: &Path,
) -> Result<Vec<InstallResult>> {
    let mut results = Vec::new();

    for (name, entry) in &manifest.skills {
        if entry.scope != ResourceScope::Global {
            continue;
        }

        let source = registry_root.join(&entry.path);
        let result = install_skill(&source, global_skills_dir, name)
            .with_context(|| format!("Failed to install global skill '{}'", name))?;
        results.push(result);
    }

    Ok(results)
}

/// Install project-scope skills from a name list to `<project>/.claude/skills/`.
///
/// Only skills listed in `skill_names` are installed. Each name must exist in
/// the manifest.
pub fn install_project_skills(
    skill_names: &[String],
    manifest: &Manifest,
    registry_root: &Path,
    project_skills_dir: &Path,
) -> Result<Vec<InstallResult>> {
    let mut results = Vec::new();

    for name in skill_names {
        let entry = manifest
            .skills
            .get(name)
            .with_context(|| t!(Msg::InstallerNotInManifest { name: name.clone() }))?;

        let source = registry_root.join(&entry.path);
        let result = install_skill(&source, project_skills_dir, name)
            .with_context(|| format!("Failed to install project skill '{}'", name))?;
        results.push(result);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_skill_dir(base: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("CLAUDE.md"), content).unwrap();
        dir
    }

    #[test]
    fn test_install_skill_fresh() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let skill_path = make_skill_dir(src_dir.path(), "my-skill", "# My Skill");
        let result = install_skill(&skill_path, dst_dir.path(), "my-skill").unwrap();

        assert!(matches!(result, InstallResult::Installed(ref n) if n == "my-skill"));
        assert!(dst_dir.path().join("my-skill/CLAUDE.md").exists());
    }

    #[test]
    fn test_install_skill_skipped_when_identical() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let skill_path = make_skill_dir(src_dir.path(), "my-skill", "# My Skill");

        // First install
        install_skill(&skill_path, dst_dir.path(), "my-skill").unwrap();

        // Second install should be skipped
        let result = install_skill(&skill_path, dst_dir.path(), "my-skill").unwrap();
        assert!(matches!(result, InstallResult::Skipped(_)));
    }

    #[test]
    fn test_install_skill_updated_when_changed() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let skill_path = make_skill_dir(src_dir.path(), "my-skill", "# Version 1");

        // First install
        install_skill(&skill_path, dst_dir.path(), "my-skill").unwrap();

        // Modify source
        fs::write(skill_path.join("CLAUDE.md"), "# Version 2").unwrap();

        // Second install should update
        let result = install_skill(&skill_path, dst_dir.path(), "my-skill").unwrap();
        assert!(matches!(result, InstallResult::Updated(_)));

        let content = fs::read_to_string(dst_dir.path().join("my-skill/CLAUDE.md")).unwrap();
        assert_eq!(content, "# Version 2");
    }

    #[test]
    fn test_install_skill_nonexistent_source() {
        let dst_dir = tempdir().unwrap();
        let bad_path = std::path::PathBuf::from("/nonexistent/skill");
        let result = install_skill(&bad_path, dst_dir.path(), "ghost");
        assert!(result.is_err());
    }

    #[test]
    fn test_install_result_display() {
        let r = InstallResult::Installed("foo".into());
        // Display output is language-dependent; just verify it contains the name
        assert!(r.to_string().contains("foo"));
        assert_eq!(r.name(), "foo");
    }
}
