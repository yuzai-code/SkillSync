// Conflict resolution — `skillsync resolve`
// Implements: task 6.6

use std::env;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::registry::git_ops;
use crate::tui::diff_viewer::{self, Resolution};

pub fn run() -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("{}", t!(Msg::ResolveNotInitialized));
    }

    let repo = git_ops::open_repo(&ss_paths.registry)
        .context("Failed to open registry repository")?;

    // Check if there are any conflicts in the index.
    let index = repo
        .index()
        .context("Failed to read repository index")?;

    if !index.has_conflicts() {
        println!(
            "{} {}",
            style("OK").green().bold(),
            t!(Msg::ResolveNoConflicts)
        );
        return Ok(());
    }

    // Collect conflicting file paths.
    let conflicts: Vec<String> = index
        .conflicts()
        .context("Failed to read merge conflicts")?
        .filter_map(|c| c.ok())
        .filter_map(|c| {
            c.our
                .as_ref()
                .or(c.their.as_ref())
                .or(c.ancestor.as_ref())
                .and_then(|e| String::from_utf8(e.path.clone()).ok())
        })
        .collect();

    if conflicts.is_empty() {
        println!(
            "{} {}",
            style("OK").green().bold(),
            t!(Msg::ResolveNoConflicts)
        );
        return Ok(());
    }

    println!(
        "{} {}",
        style("!!").red().bold(),
        t!(Msg::ResolveFound { count: conflicts.len() })
    );
    for path in &conflicts {
        println!(
            "{} {}",
            style("-").red(),
            path
        );
    }
    println!();

    // Resolve each conflict.
    let mut resolved_count = 0usize;
    for conflict_path in &conflicts {
        let full_path = ss_paths.registry.join(conflict_path);

        // Read the working-directory version (which contains conflict markers).
        let working_content = if full_path.exists() {
            std::fs::read_to_string(&full_path).unwrap_or_default()
        } else {
            String::new()
        };

        // Try to extract ours/theirs from conflict markers.
        let (local_content, remote_content) =
            extract_conflict_sides(&working_content);

        // Show diff.
        diff_viewer::show_diff(&local_content, &remote_content, conflict_path);

        // Ask user how to resolve.
        let resolution = diff_viewer::choose_resolution()?;

        match resolution {
            Resolution::KeepLocal => {
                std::fs::write(&full_path, &local_content)
                    .with_context(|| format!("Failed to write local version to {}", full_path.display()))?;
                println!(
                    "  {} {}",
                    style("*").cyan(),
                    t!(Msg::ResolveKeptLocal { file: conflict_path.clone() })
                );
            }
            Resolution::UseRemote => {
                std::fs::write(&full_path, &remote_content)
                    .with_context(|| format!("Failed to write remote version to {}", full_path.display()))?;
                println!(
                    "  {} {}",
                    style("*").cyan(),
                    t!(Msg::ResolveUsedRemote { file: conflict_path.clone() })
                );
            }
            Resolution::OpenEditor => {
                open_in_editor(&full_path)?;
                println!(
                    "  {} {}",
                    style("*").cyan(),
                    t!(Msg::ResolveManuallyEdited { file: conflict_path.clone() })
                );
            }
        }

        resolved_count += 1;
    }

    // Stage all resolved files and create a merge commit.
    if resolved_count > 0 {
        git_ops::stage_all(&repo).context("Failed to stage resolved files")?;

        let message = format!(
            "Resolve {} merge conflict(s)",
            resolved_count
        );
        git_ops::commit(&repo, &message).context("Failed to create merge commit")?;

        // Clean up merge state.
        repo.cleanup_state()
            .context("Failed to clean up merge state")?;

        println!();
        println!(
            "{} {}",
            style("OK").green().bold(),
            t!(Msg::ResolveSuccess { count: resolved_count })
        );
    }

    Ok(())
}

/// Extract "ours" and "theirs" content from git conflict markers.
///
/// If the file does not contain standard conflict markers, the whole content
/// is returned as both local and remote (user can still manually edit).
fn extract_conflict_sides(content: &str) -> (String, String) {
    let mut local = String::new();
    let mut remote = String::new();
    let mut common = String::new();

    enum Section {
        Common,
        Ours,
        Theirs,
    }

    let mut section = Section::Common;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            section = Section::Ours;
            continue;
        } else if line.starts_with("=======") {
            if matches!(section, Section::Ours) {
                section = Section::Theirs;
                continue;
            }
        } else if line.starts_with(">>>>>>>") {
            section = Section::Common;
            continue;
        }

        match section {
            Section::Common => {
                if !common.is_empty() || !local.is_empty() || !remote.is_empty() {
                    common.push('\n');
                }
                common.push_str(line);
                // Common lines go to both sides.
                if !local.is_empty() {
                    local.push('\n');
                }
                local.push_str(line);
                if !remote.is_empty() {
                    remote.push('\n');
                }
                remote.push_str(line);
            }
            Section::Ours => {
                if !local.is_empty() {
                    local.push('\n');
                }
                local.push_str(line);
            }
            Section::Theirs => {
                if !remote.is_empty() {
                    remote.push('\n');
                }
                remote.push_str(line);
            }
        }
    }

    // If no conflict markers were found, return the original as both sides.
    if local == remote && local == common {
        return (content.to_string(), content.to_string());
    }

    (local, remote)
}

/// Open a file in the user's preferred editor.
fn open_in_editor(path: &std::path::Path) -> Result<()> {
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let status = std::process::Command::new(&editor)
        .arg(path)
        .status()
        .with_context(|| t!(Msg::ResolveEditorLaunch { editor: editor.clone() }))?;

    if !status.success() {
        bail!("{}", t!(Msg::ResolveEditorFailed { editor }));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_conflict_sides_with_markers() {
        let content = "\
common line 1
<<<<<<< HEAD
local change
=======
remote change
>>>>>>> origin/main
common line 2";

        let (local, remote) = extract_conflict_sides(content);
        assert!(local.contains("local change"));
        assert!(local.contains("common line 1"));
        assert!(local.contains("common line 2"));
        assert!(!local.contains("remote change"));

        assert!(remote.contains("remote change"));
        assert!(remote.contains("common line 1"));
        assert!(remote.contains("common line 2"));
        assert!(!remote.contains("local change"));
    }

    #[test]
    fn test_extract_conflict_sides_no_markers() {
        let content = "just a normal file\nwith two lines";
        let (local, remote) = extract_conflict_sides(content);
        assert_eq!(local, content);
        assert_eq!(remote, content);
    }

    #[test]
    fn test_extract_conflict_sides_multiple_conflicts() {
        let content = "\
start
<<<<<<< HEAD
ours1
=======
theirs1
>>>>>>> origin/main
middle
<<<<<<< HEAD
ours2
=======
theirs2
>>>>>>> origin/main
end";

        let (local, remote) = extract_conflict_sides(content);
        assert!(local.contains("ours1"));
        assert!(local.contains("ours2"));
        assert!(!local.contains("theirs1"));
        assert!(!local.contains("theirs2"));

        assert!(remote.contains("theirs1"));
        assert!(remote.contains("theirs2"));
        assert!(!remote.contains("ours1"));
        assert!(!remote.contains("ours2"));

        // Common parts should be in both.
        assert!(local.contains("start"));
        assert!(local.contains("middle"));
        assert!(local.contains("end"));
        assert!(remote.contains("start"));
        assert!(remote.contains("middle"));
        assert!(remote.contains("end"));
    }
}
