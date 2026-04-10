// Project skills discovery - scan ~/projects/*/.claude/skills/ for local skills

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::Repository;

#[allow(unused_imports)]
use crate::i18n::Msg;
use crate::registry::manifest::{Manifest, SkillEntry, SkillType, ResourceScope};
use crate::registry::resource::compute_hash;

/// A skill discovered from a project directory.
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// Skill name (directory name).
    pub name: String,
    /// Full path to the skill directory.
    pub path: PathBuf,
    /// Project path where this skill was found.
    pub project_path: PathBuf,
    /// Content hash for deduplication.
    pub content_hash: String,
}

/// Scan `~/.claude/skills/` for global skills.
///
/// Returns skills with their content hashes for deduplication.
pub fn scan_global_skills() -> Result<Vec<DiscoveredSkill>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let global_skills_dir = home.join(".claude").join("skills");

    if !global_skills_dir.exists() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();

    let entries = fs::read_dir(&global_skills_dir)
        .with_context(|| format!("Failed to read global skills directory: {}", global_skills_dir.display()))?;

    for entry in entries.flatten() {
        let skill_path = entry.path();
        if skill_path.is_dir() {
            if let Some(name) = skill_path.file_name().and_then(|n| n.to_str()) {
                // Skip hidden directories
                if name.starts_with('.') {
                    continue;
                }

                let hash = compute_skill_hash(&skill_path)?;

                discovered.push(DiscoveredSkill {
                    name: name.to_string(),
                    path: skill_path.clone(),
                    project_path: home.clone(), // Global skills have home as "project"
                    content_hash: hash,
                });
            }
        }
    }

    discovered.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(discovered)
}

/// Get project directories to scan for skills.
///
/// Checks multiple common locations in order:
/// 1. ~/projects/
/// 2. ~/Desktop/project/
/// 3. Custom paths from ~/.skillsync/config.yaml (if exists)
fn get_project_dirs() -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let mut dirs = Vec::new();

    // Standard location: ~/projects/
    let projects_dir = home.join("projects");
    if projects_dir.exists() {
        dirs.push(projects_dir);
    }

    // Alternative location: ~/Desktop/project/
    let desktop_project_dir = home.join("Desktop").join("project");
    if desktop_project_dir.exists() {
        dirs.push(desktop_project_dir);
    }

    // TODO: Load custom paths from ~/.skillsync/config.yaml

    Ok(dirs)
}

/// Scan project directories for skills.
///
/// Scans multiple project directories and their subdirectories for .claude/skills/.
/// Only scans directories that are git repositories (to avoid scanning junk).
/// Returns skills with their content hashes for deduplication.
pub fn scan_projects_skills() -> Result<Vec<DiscoveredSkill>> {
    let project_dirs = get_project_dirs()?;

    if project_dirs.is_empty() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();

    for projects_dir in project_dirs {
        // Read all entries in the projects directory
        let entries = match fs::read_dir(&projects_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let project_path = entry.path();

            // Skip if not a directory
            if !project_path.is_dir() {
                continue;
            }

            // Skip if not a git repository (user may have non-git projects)
            if !is_git_repo(&project_path) {
                continue;
            }

            // Check for .claude/skills/ directory
            let skills_dir = project_path.join(".claude").join("skills");
            if !skills_dir.is_dir() {
                continue;
            }

            // Scan skill directories
            let skill_entries = match fs::read_dir(&skills_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for skill_entry in skill_entries.flatten() {
                let skill_path = skill_entry.path();
                if skill_path.is_dir() {
                    if let Some(name) = skill_path.file_name().and_then(|n| n.to_str()) {
                        // Skip hidden directories
                        if name.starts_with('.') {
                            continue;
                        }

                        // Compute content hash
                        let hash = compute_skill_hash(&skill_path)?;

                        discovered.push(DiscoveredSkill {
                            name: name.to_string(),
                            path: skill_path,
                            project_path: project_path.clone(),
                            content_hash: hash,
                        });
                    }
                }
            }
        }
    }

    discovered.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(discovered)
}

/// Scan all local skills (global + project-level).
///
/// Deduplicates by name, preferring global version if same name exists.
pub fn scan_all_local_skills() -> Result<Vec<DiscoveredSkill>> {
    let mut skills: Vec<DiscoveredSkill> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    // First scan global skills (higher priority)
    let global = scan_global_skills()?;
    for skill in global {
        seen_names.insert(skill.name.clone());
        skills.push(skill);
    }

    // Then scan project skills (skip if already seen)
    let projects = scan_projects_skills()?;
    for skill in projects {
        if !seen_names.contains(&skill.name) {
            seen_names.insert(skill.name.clone());
            skills.push(skill);
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

/// Check if a directory is a git repository.
fn is_git_repo(path: &Path) -> bool {
    Repository::open(path).is_ok()
}

/// Compute a deterministic hash for a skill directory.
/// Uses the same hashing approach as resource.rs.
fn compute_skill_hash(skill_path: &Path) -> Result<String> {
    let hash = compute_hash(skill_path)
        .with_context(|| format!("Failed to compute hash for skill: {}", skill_path.display()))?;
    Ok(format!("sha256:{}", hash))
}

/// Add discovered skills to the manifest with deduplication by content hash.
///
/// For skills that already exist in the manifest (by name):
/// - If content_hash matches: skip (deduplication - same skill from different source)
/// - If content_hash differs: update (skill was modified)
///
/// For new skills, adds new entries.
pub fn register_discovered_skills(
    manifest: &mut Manifest,
    discovered: &[DiscoveredSkill],
) {
    for skill in discovered {
        // Check if skill with this name already exists in manifest
        if let Some(existing) = manifest.skills.get(&skill.name) {
            // Same content hash = deduplication (skip, already registered)
            if existing.backup_hash.as_ref() == Some(&skill.content_hash) {
                continue;
            }
            // Different content hash = update (skill was modified)
        }

        let entry = SkillEntry {
            skill_type: SkillType::Custom,
            scope: ResourceScope::Shared,
            version: "1.0.0".to_string(),
            description: None,
            tags: Vec::new(),
            path: format!("resources/skills/{}", skill.name),
            backup_hash: Some(skill.content_hash.clone()),
            source: None,
            source_path: Some(skill.project_path.join(".claude").join("skills").join(&skill.name).display().to_string()),
        };

        manifest.skills.insert(skill.name.clone(), entry);
    }
}

/// Get list of project paths that have .claude/skills/ directories.
pub fn get_project_skills_dirs() -> Result<Vec<PathBuf>> {
    let project_dirs = get_project_dirs()?;

    if project_dirs.is_empty() {
        return Ok(Vec::new());
    }

    let mut dirs = Vec::new();

    for projects_dir in project_dirs {
        let entries = match fs::read_dir(&projects_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let project_path = entry.path();
            if !project_path.is_dir() {
                continue;
            }
            if !is_git_repo(&project_path) {
                continue;
            }

            let skills_dir = project_path.join(".claude").join("skills");
            if skills_dir.is_dir() {
                dirs.push(skills_dir);
            }
        }
    }

    Ok(dirs)
}

/// Remove manifest entries for skills that no longer exist on disk.
///
/// This cleans up orphaned entries when skill directories are deleted.
/// Returns the list of removed skill names.
pub fn cleanup_deleted_skills(
    manifest: &mut Manifest,
    registry_path: &Path,
) -> Vec<String> {
    let mut removed = Vec::new();

    // Collect skills to remove (can't mutate while iterating)
    let to_remove: Vec<String> = manifest
        .skills
        .iter()
        .filter(|(_, entry)| {
            let skill_path = registry_path.join(&entry.path);
            !skill_path.exists()
        })
        .map(|(name, _)| name.clone())
        .collect();

    for name in &to_remove {
        manifest.skills.remove(name);
        removed.push(name.clone());
    }

    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_no_projects_dir() {
        // When ~/projects doesn't exist, should return empty
        let result = scan_projects_skills();
        // This test depends on the environment - may not fail even if no projects dir
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_git_repo() {
        let dir = TempDir::new().unwrap();
        // Empty directory is not a git repo
        assert!(!is_git_repo(dir.path()));

        // Initialize as git repo
        Repository::init(dir.path()).unwrap();
        assert!(is_git_repo(dir.path()));
    }
}
