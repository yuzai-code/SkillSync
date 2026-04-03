// Resource abstraction for skills, plugins, MCP servers

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// ResourceType
// ---------------------------------------------------------------------------

/// The three kinds of syncable resources that SkillSync manages.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Skill,
    Plugin,
    McpServer,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Skill => write!(f, "skill"),
            ResourceType::Plugin => write!(f, "plugin"),
            ResourceType::McpServer => write!(f, "mcp_server"),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute a deterministic SHA-256 hash over every file inside `path`.
///
/// Files are visited in sorted order so the hash is reproducible across
/// platforms and runs.  The hash covers both the relative file path and the
/// file contents so that renames are detected.
///
/// Returns a string like `sha256:abcdef0123...`.
pub fn compute_hash(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();

    // Collect all file paths first so we can sort them for determinism.
    let mut entries: Vec<walkdir::DirEntry> = WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();

    entries.sort_by(|a, b| a.path().cmp(b.path()));

    for entry in entries {
        // Include the *relative* path in the hash so that moves are detected.
        let rel = entry
            .path()
            .strip_prefix(path)
            .unwrap_or(entry.path());
        hasher.update(rel.to_string_lossy().as_bytes());

        let contents = std::fs::read(entry.path())
            .with_context(|| format!("Failed to read file for hashing: {}", entry.path().display()))?;
        hasher.update(&contents);
    }

    let digest = hasher.finalize();
    Ok(format!("sha256:{:x}", digest))
}

/// Deep-copy a resource directory from `from` to `to`.
///
/// If `to` already exists it will be removed first so the copy is clean.
/// Parent directories of `to` are created automatically.
pub fn copy_resource(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        anyhow::bail!(
            "Source path does not exist: {}. Check the path and ensure the resource has not been deleted.",
            from.display()
        );
    }

    // Ensure the destination's parent directory exists.
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent dirs for {}", to.display()))?;
    }

    // Remove stale destination if present.
    if to.exists() {
        if to.is_dir() {
            std::fs::remove_dir_all(to)
                .with_context(|| format!("Failed to remove existing destination: {}", to.display()))?;
        } else {
            std::fs::remove_file(to)
                .with_context(|| format!("Failed to remove existing destination file: {}", to.display()))?;
        }
    }

    if from.is_dir() {
        let mut options = fs_extra::dir::CopyOptions::new();
        options.copy_inside = true;
        options.content_only = true;

        std::fs::create_dir_all(to)
            .with_context(|| format!("Failed to create destination dir: {}", to.display()))?;

        fs_extra::dir::copy(from, to, &options)
            .with_context(|| {
                format!(
                    "Failed to copy directory from {} to {}",
                    from.display(),
                    to.display()
                )
            })?;
    } else {
        // Single file — just copy it.
        std::fs::copy(from, to)
            .with_context(|| {
                format!(
                    "Failed to copy file from {} to {}",
                    from.display(),
                    to.display()
                )
            })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_compute_hash_deterministic() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        // Create a small tree.
        fs::create_dir_all(base.join("sub")).unwrap();
        fs::write(base.join("a.txt"), "hello").unwrap();
        fs::write(base.join("sub/b.txt"), "world").unwrap();

        let h1 = compute_hash(base).unwrap();
        let h2 = compute_hash(base).unwrap();
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn test_compute_hash_changes_on_content_change() {
        let dir = tempdir().unwrap();
        let base = dir.path();
        fs::write(base.join("f.txt"), "aaa").unwrap();

        let h1 = compute_hash(base).unwrap();

        fs::write(base.join("f.txt"), "bbb").unwrap();
        let h2 = compute_hash(base).unwrap();

        assert_ne!(h1, h2);
    }

    #[test]
    fn test_copy_resource_dir() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let src = src_dir.path();
        let dst = dst_dir.path().join("copy_target");

        fs::create_dir_all(src.join("inner")).unwrap();
        fs::write(src.join("a.txt"), "hello").unwrap();
        fs::write(src.join("inner/b.txt"), "world").unwrap();

        copy_resource(src, &dst).unwrap();

        assert!(dst.join("a.txt").exists());
        assert!(dst.join("inner/b.txt").exists());
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(dst.join("inner/b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn test_copy_resource_single_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");

        fs::write(&src, "content").unwrap();

        copy_resource(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(&dst).unwrap(), "content");
    }

    #[test]
    fn test_copy_resource_replaces_existing() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("f.txt"), "new").unwrap();

        // Pre-create destination with stale data.
        fs::create_dir_all(&dst).unwrap();
        fs::write(dst.join("old.txt"), "stale").unwrap();

        copy_resource(&src, &dst).unwrap();

        assert!(dst.join("f.txt").exists());
        // Old file should be gone because we removed the dir first.
        assert!(!dst.join("old.txt").exists());
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Skill.to_string(), "skill");
        assert_eq!(ResourceType::Plugin.to_string(), "plugin");
        assert_eq!(ResourceType::McpServer.to_string(), "mcp_server");
    }
}
