//! Release command - automate version bumping and publishing
//!
//! This module provides the `skillsync release` command which:
//! 1. Calculates the new version number
//! 2. Updates Cargo.toml
//! 3. Generates CHANGELOG.md
//! 4. Creates and pushes a git tag

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;
use git2::{Repository, StatusOptions};
use semver::Version;

use crate::t;
use crate::i18n::Msg;

/// Version bump level
#[derive(Debug, Clone, Copy, Default)]
pub enum BumpLevel {
    #[default]
    Patch,
    Minor,
    Major,
}

/// Release command options
pub struct ReleaseOptions {
    pub level: BumpLevel,
    pub dry_run: bool,
}

/// Run the release command
pub fn run(major: bool, minor: bool, dry_run: bool) -> Result<()> {
    let level = match (major, minor) {
        (true, _) => BumpLevel::Major,
        (_, true) => BumpLevel::Minor,
        _ => BumpLevel::Patch,
    };

    let options = ReleaseOptions { level, dry_run };
    run_inner(options)
}

fn run_inner(options: ReleaseOptions) -> Result<()> {
    let repo = Repository::discover(".")
        .with_context(|| t!(Msg::ReleaseNotGitRepo))?;

    // 1. Check working tree is clean
    check_working_tree_clean(&repo)?;

    // 2. Read current version from Cargo.toml
    let cargo_toml_path = Path::new("Cargo.toml");
    let current_version = read_cargo_version(cargo_toml_path)?;

    // 3. Calculate new version
    let new_version = bump_version(&current_version, options.level)?;
    let tag_name = format!("v{}", new_version);

    if options.dry_run {
        println!(
            "{} {}",
            style("Dry run:").yellow().bold(),
            t!(Msg::ReleaseDryRun {
                old_ver: current_version.clone(),
                new_ver: new_version.clone()
            })
        );
        println!();

        // Preview CHANGELOG
        let changelog = generate_changelog(&repo, &new_version)?;
        println!("{}", style("CHANGELOG preview:").dim());
        println!("{}", changelog);
        return Ok(());
    }

    // 4. Update Cargo.toml
    update_cargo_version(cargo_toml_path, &new_version)?;
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseVersionUpdated {
            old_ver: current_version.clone(),
            new_ver: new_version.clone()
        })
    );

    // 5. Generate CHANGELOG.md
    let changelog = generate_changelog(&repo, &new_version)?;
    update_changelog_file(&changelog)?;
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseChangelogUpdated)
    );

    // 6. git add + commit
    let mut index = repo.index()?;
    index.add_path(Path::new("Cargo.toml"))?;
    index.add_path(Path::new("CHANGELOG.md"))?;
    index.write()?;

    let signature = repo.signature()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let head = repo.head()?.target().context("No HEAD")?;
    let parent = repo.find_commit(head)?;

    let commit_msg = format!("chore: release {}", tag_name);
    repo.commit(Some("HEAD"), &signature, &signature, &commit_msg, &tree, &[&parent])?;
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseCommitCreated)
    );

    // 7. git tag v{version}
    let target = repo.head()?.target().context("No HEAD")?;
    repo.tag(&tag_name, &repo.find_object(target, None)?, &signature, "", false)?;
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseTagCreated { tag: tag_name.clone() })
    );

    // 8. git push origin main --tags
    push_release(&repo)?;

    println!();
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseSuccess { tag: tag_name })
    );

    Ok(())
}

/// Check if working tree is clean
fn check_working_tree_clean(repo: &Repository) -> Result<()> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    if !statuses.is_empty() {
        bail!(
            "{}",
            t!(Msg::ReleaseWorkingTreeDirty)
        );
    }

    Ok(())
}

/// Read version from Cargo.toml
fn read_cargo_version(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("version") {
            // Parse: version = "0.1.0"
            if let Some(eq_pos) = line.find('=') {
                let version_part = line[eq_pos + 1..].trim();
                // Remove quotes
                let version = version_part.trim_matches('"').trim();
                return Ok(version.to_string());
            }
        }
    }

    bail!("No version field found in Cargo.toml")
}

/// Update version in Cargo.toml
fn update_cargo_version(path: &Path, new_version: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let mut new_content = String::new();
    let mut found = false;

    for line in content.lines() {
        if line.trim().starts_with("version") && !found {
            if let Some(eq_pos) = line.find('=') {
                let prefix = &line[..eq_pos + 1];
                let new_line = format!("{} \"{}\"", prefix, new_version);
                new_content.push_str(&new_line);
                found = true;
            } else {
                new_content.push_str(line);
            }
        } else {
            new_content.push_str(line);
        }
        new_content.push('\n');
    }

    if !found {
        bail!("No version field found in Cargo.toml");
    }

    std::fs::write(path, new_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Generate CHANGELOG content
fn generate_changelog(repo: &Repository, new_version: &str) -> Result<String> {
    // Get commits since last tag
    let commits = get_commits_since_last_tag(repo)?;

    // Group by type
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
    let group_order = ["feat", "fix", "docs", "refactor", "test", "chore"];

    for commit in &commits {
        let msg = commit.trim();
        if let Some(slash_pos) = msg.find(':') {
            let commit_type = &msg[..slash_pos];
            let commit_msg = &msg[slash_pos + 1..].trim();
            if group_order.contains(&commit_type) {
                groups.entry(commit_type).or_default().push(commit_msg);
            }
        }
    }

    // Build changelog
    let mut changelog = format!("## [{}]\n\n", new_version);

    for commit_type in group_order {
        if let Some(items) = groups.get(commit_type) {
            let type_name = match commit_type {
                "feat" => "Features",
                "fix" => "Bug Fixes",
                "docs" => "Documentation",
                "refactor" => "Refactoring",
                "test" => "Tests",
                "chore" => "Chores",
                _ => commit_type,
            };
            changelog.push_str(&format!("### {}\n\n", type_name));
            for item in items {
                changelog.push_str(&format!("- {}\n", item));
            }
            changelog.push('\n');
        }
    }

    if commits.is_empty() {
        changelog.push_str("- No notable changes\n\n");
    }

    Ok(changelog)
}

/// Get commits since last tag
fn get_commits_since_last_tag(repo: &Repository) -> Result<Vec<String>> {
    // Find the latest tag
    let mut latest_tag: Option<(git2::Oid, String)> = None;

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        if let Some(tag_name) = name.strip_prefix("refs/tags/") {
            if tag_name.starts_with('v') {
                if latest_tag.is_none() {
                    latest_tag = Some((oid, tag_name.to_string()));
                }
            }
        }
        true
    })?;

    let mut commits = Vec::new();
    let head = repo.head()?.target().context("No HEAD")?;
    let _head_commit = repo.find_commit(head)?;

    // Walk from HEAD
    let mut revwalk = repo.revwalk()?;
    revwalk.push(head)?;

    let tag_oid = latest_tag.as_ref().map(|(oid, _)| *oid);

    for oid_result in revwalk {
        let oid = oid_result?;
        if let Some(tag_oid) = tag_oid {
            if oid == tag_oid {
                break;
            }
        }

        let commit = repo.find_commit(oid)?;
        if let Some(msg) = commit.message() {
            let msg = msg.trim().to_string();
            if !msg.is_empty() && !msg.starts_with("Merge") {
                commits.push(msg);
            }
        }
    }

    Ok(commits)
}

/// Update CHANGELOG.md file
fn update_changelog_file(new_content: &str) -> Result<()> {
    let changelog_path = Path::new("CHANGELOG.md");

    let existing = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path)?
    } else {
        String::new()
    };

    // Prepend new content
    let updated = format!(
        "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n{}\n{}",
        new_content,
        existing
            .lines()
            .skip_while(|l| l.starts_with('#') || l.starts_with("All notable") || l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    );

    std::fs::write(changelog_path, updated)?;
    Ok(())
}

/// Push release to remote
fn push_release(repo: &Repository) -> Result<()> {
    // Get remote
    let mut remote = if let Ok(r) = repo.find_remote("origin") {
        r
    } else {
        let remotes = repo.remotes()?;
        let name = remotes.get(0).context("No remote found")?;
        repo.find_remote(name)?
    };

    // Push current branch
    let head = repo.head()?;
    let branch_name = head.shorthand().context("No branch name")?;
    let refspec = format!("refs/heads/{}", branch_name);

    {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_, _, _| {
            git2::Cred::ssh_key_from_agent("git")
        });

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
    }
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleasePushed)
    );

    // Push tags
    {
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_, _, _| {
            git2::Cred::ssh_key_from_agent("git")
        });

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&["refs/tags/*:refs/tags/*"], Some(&mut push_options))?;
    }
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ReleaseTagsPushed)
    );

    Ok(())
}

/// Bump version according to level
fn bump_version(current: &str, level: BumpLevel) -> Result<String> {
    let mut version = Version::parse(current)
        .with_context(|| format!("Invalid version: {}", current))?;

    match level {
        BumpLevel::Patch => version.patch += 1,
        BumpLevel::Minor => {
            version.minor += 1;
            version.patch = 0;
        }
        BumpLevel::Major => {
            version.major += 1;
            version.minor = 0;
            version.patch = 0;
        }
    }

    Ok(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_patch() {
        assert_eq!(bump_version("0.1.0", BumpLevel::Patch).unwrap(), "0.1.1");
        assert_eq!(bump_version("1.2.3", BumpLevel::Patch).unwrap(), "1.2.4");
    }

    #[test]
    fn test_bump_minor() {
        assert_eq!(bump_version("0.1.0", BumpLevel::Minor).unwrap(), "0.2.0");
        assert_eq!(bump_version("1.2.3", BumpLevel::Minor).unwrap(), "1.3.0");
    }

    #[test]
    fn test_bump_major() {
        assert_eq!(bump_version("0.1.0", BumpLevel::Major).unwrap(), "1.0.0");
        assert_eq!(bump_version("1.2.3", BumpLevel::Major).unwrap(), "2.0.0");
    }

    #[test]
    fn test_bump_invalid_version() {
        assert!(bump_version("invalid", BumpLevel::Patch).is_err());
    }

    #[test]
    fn test_read_cargo_version() {
        use tempfile::tempdir;
        use std::fs;

        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, r#"[package]
name = "test"
version = "1.2.3"
edition = "2021"
"#).unwrap();

        let version = read_cargo_version(&cargo_toml).unwrap();
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn test_update_cargo_version() {
        use tempfile::tempdir;
        use std::fs;

        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, r#"[package]
name = "test"
version = "1.2.3"
edition = "2021"
"#).unwrap();

        update_cargo_version(&cargo_toml, "2.0.0").unwrap();

        let content = fs::read_to_string(&cargo_toml).unwrap();
        assert!(content.contains("version = \"2.0.0\""));
    }
}