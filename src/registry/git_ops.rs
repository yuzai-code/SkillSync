// Git operations: clone, pull, push, status, stage, commit, conflict detection
// Implements: task 6.1

use std::path::Path;

use anyhow::{bail, Context, Result};
use git2::{IndexAddOption, MergeAnalysis, Repository, StatusOptions};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of merging the fetched remote branch into HEAD.
pub struct MergeResult {
    /// Remote and local are already at the same commit.
    pub up_to_date: bool,
    /// Merge was a fast-forward (no divergent history).
    pub fast_forward: bool,
    /// Paths with merge conflicts (empty if merge was clean).
    pub conflicts: Vec<String>,
}

// ---------------------------------------------------------------------------
// Repository helpers
// ---------------------------------------------------------------------------

/// Open an existing git repository at `path`.
pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::open(path)
        .with_context(|| format!("Failed to open git repository at {}", path.display()))
}

/// Open an existing bare git repository at `path`.
pub fn open_bare_repo(path: &Path) -> Result<Repository> {
    Repository::open_bare(path)
        .with_context(|| format!("Failed to open bare git repository at {}", path.display()))
}

/// Get repository status — returns `(has_changes, changed_files)`.
///
/// `has_changes` is `true` when there are any staged, unstaged, or untracked
/// modifications in the working directory.
pub fn repo_status(repo: &Repository) -> Result<(bool, Vec<String>)> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true);

    let statuses = repo
        .statuses(Some(&mut opts))
        .context("Failed to read repository status")?;

    let mut changed_files: Vec<String> = Vec::new();

    for entry in statuses.iter() {
        if let Some(path) = entry.path() {
            changed_files.push(path.to_string());
        }
    }

    let has_changes = !changed_files.is_empty();
    Ok((has_changes, changed_files))
}

// ---------------------------------------------------------------------------
// Staging & committing
// ---------------------------------------------------------------------------

/// Stage all changes (new, modified, deleted) in the working directory.
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index().context("Failed to get repository index")?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .context("Failed to stage files")?;

    // Also pick up deletions: update index entries for tracked files that
    // have been removed from the working directory.
    index
        .update_all(["*"].iter(), None)
        .context("Failed to update index for deleted files")?;

    index.write().context("Failed to write index")?;
    Ok(())
}

/// Stage only skill-related changes: resources/skills/ and manifest.yaml.
/// This is used for incremental sync that only tracks skills.
pub fn stage_skills_only(repo: &Repository) -> Result<()> {
    let mut index = repo.index().context("Failed to get repository index")?;

    // Stage all files under resources/skills/
    index
        .add_all(["resources/skills/*"].iter(), IndexAddOption::DEFAULT, None)
        .context("Failed to stage skills directory")?;

    // Stage manifest.yaml changes.
    index
        .add_all(["manifest.yaml"].iter(), IndexAddOption::DEFAULT, None)
        .context("Failed to stage manifest.yaml")?;

    // Also pick up deletions in the skills directory.
    index
        .update_all(["resources/skills/*"].iter(), None)
        .context("Failed to update index for deleted skill files")?;

    index.write().context("Failed to write index")?;
    Ok(())
}

/// Create a commit on HEAD with the given message.
///
/// All currently staged changes (from the index) are included.
/// Returns the new commit's OID.
pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid> {
    let mut index = repo.index().context("Failed to get repository index")?;
    let tree_oid = index
        .write_tree()
        .context("Failed to write index tree")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("Failed to find tree for commit")?;

    let sig = repo
        .signature()
        .context(
            "Failed to determine commit author. Run 'git config --global user.name \"Your Name\"' \
             and 'git config --global user.email \"you@example.com\"' to configure.",
        )?;

    // Determine parent commits — there may be none on an initial commit.
    let parent_commit = match repo.head() {
        Ok(head_ref) => {
            let oid = head_ref
                .target()
                .context("HEAD reference has no target")?;
            Some(repo.find_commit(oid).context("Failed to find HEAD commit")?)
        }
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .context("Failed to create commit")?;

    Ok(oid)
}

// ---------------------------------------------------------------------------
// Remote operations
// ---------------------------------------------------------------------------

/// Build `RemoteCallbacks` that try SSH-agent authentication.
pub fn make_callbacks<'a>() -> git2::RemoteCallbacks<'a> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed| {
        let user = username_from_url.unwrap_or("git");
        git2::Cred::ssh_key_from_agent(user)
    });
    callbacks
}

/// Detect the remote branch name — prefers `main`, falls back to `master`.
fn detect_remote_branch(repo: &Repository) -> Result<String> {
    // Check which remote branches exist.
    let refs = repo.references().context("Failed to list references")?;
    let mut has_main = false;
    let mut has_master = false;

    for r in refs.flatten() {
        if let Some(name) = r.name() {
            if name == "refs/remotes/origin/main" {
                has_main = true;
            } else if name == "refs/remotes/origin/master" {
                has_master = true;
            }
        }
    }

    if has_main {
        Ok("main".to_string())
    } else if has_master {
        Ok("master".to_string())
    } else {
        // Default to main if we can't detect yet (e.g., before first fetch).
        Ok("main".to_string())
    }
}

/// Detect the local branch name from HEAD.
fn detect_local_branch(repo: &Repository) -> Result<String> {
    match repo.head() {
        Ok(head) => {
            if let Some(name) = head.shorthand() {
                Ok(name.to_string())
            } else {
                Ok("main".to_string())
            }
        }
        Err(_) => Ok("main".to_string()),
    }
}

/// Fetch from the `origin` remote.
pub fn fetch_origin(repo: &Repository) -> Result<()> {
    let mut remote = repo
        .find_remote("origin")
        .context("No remote named 'origin'. Is this a cloned registry?")?;

    let callbacks = make_callbacks();
    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // Fetch both main and master — only the one that exists on the remote
    // will actually transfer data; the other will be silently ignored.
    remote
        .fetch(&["main", "master"], Some(&mut fetch_opts), None)
        .context("Failed to fetch from origin")?;

    Ok(())
}

/// Merge the fetched remote branch into HEAD.
///
/// Tries `origin/main` first, then `origin/master`.
pub fn merge_origin(repo: &Repository) -> Result<MergeResult> {
    let branch = detect_remote_branch(repo)?;
    let remote_ref_name = format!("refs/remotes/origin/{}", branch);

    let remote_ref = repo
        .find_reference(&remote_ref_name)
        .with_context(|| {
            format!(
                "Could not find remote branch '{}'. Try fetching first.",
                remote_ref_name
            )
        })?;

    let fetch_commit_oid = remote_ref
        .target()
        .context("Remote reference has no target")?;

    let fetch_commit = repo
        .find_annotated_commit(fetch_commit_oid)
        .context("Failed to find fetched commit")?;

    let (analysis, _pref) = repo
        .merge_analysis(&[&fetch_commit])
        .context("Merge analysis failed")?;

    // --- Up-to-date ----------------------------------------------------------
    if analysis.contains(MergeAnalysis::ANALYSIS_UP_TO_DATE) {
        return Ok(MergeResult {
            up_to_date: true,
            fast_forward: false,
            conflicts: vec![],
        });
    }

    // --- Fast-forward --------------------------------------------------------
    if analysis.contains(MergeAnalysis::ANALYSIS_FASTFORWARD) {
        let target_commit = repo
            .find_commit(fetch_commit_oid)
            .context("Failed to find commit for fast-forward")?;
        let tree = target_commit
            .tree()
            .context("Failed to get tree for fast-forward")?;

        repo.checkout_tree(tree.as_object(), None)
            .context("Failed to checkout tree for fast-forward")?;

        // Update HEAD to point to the new commit.
        let local_branch = detect_local_branch(repo)?;
        let refname = format!("refs/heads/{}", local_branch);
        repo.reference(&refname, fetch_commit_oid, true, "fast-forward merge")
            .with_context(|| format!("Failed to update {} to fetched commit", refname))?;

        // Update HEAD if it was pointing at the branch.
        repo.set_head(&refname)
            .context("Failed to set HEAD after fast-forward")?;

        return Ok(MergeResult {
            up_to_date: false,
            fast_forward: true,
            conflicts: vec![],
        });
    }

    // --- Normal merge --------------------------------------------------------
    if analysis.contains(MergeAnalysis::ANALYSIS_NORMAL) {
        let their_commit = repo
            .find_commit(fetch_commit_oid)
            .context("Failed to find remote commit for merge")?;

        repo.merge(&[&fetch_commit], None, None)
            .context("Merge operation failed")?;

        // Check for conflicts.
        let index = repo.index().context("Failed to get index after merge")?;
        if index.has_conflicts() {
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

            return Ok(MergeResult {
                up_to_date: false,
                fast_forward: false,
                conflicts,
            });
        }

        // No conflicts — create a merge commit.
        let mut index = repo.index().context("Failed to get index")?;
        let tree_oid = index
            .write_tree()
            .context("Failed to write tree from merged index")?;
        let tree = repo
            .find_tree(tree_oid)
            .context("Failed to find merged tree")?;

        let sig = repo.signature().context("Failed to determine commit author")?;
        let head_commit = repo
            .head()
            .context("Failed to get HEAD")?
            .peel_to_commit()
            .context("Failed to peel HEAD to commit")?;

        let msg = format!("Merge origin/{}", branch);
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &tree,
            &[&head_commit, &their_commit],
        )
        .context("Failed to create merge commit")?;

        // Clean up merge state.
        repo.cleanup_state()
            .context("Failed to clean up merge state")?;

        return Ok(MergeResult {
            up_to_date: false,
            fast_forward: false,
            conflicts: vec![],
        });
    }

    bail!(
        "Unexpected merge analysis result. The repository may be in an unusual state. \
         Try running 'skillsync doctor' to diagnose, or check the git repository manually."
    );
}

/// Push the current branch to `origin`.
pub fn push_origin(repo: &Repository) -> Result<()> {
    let mut remote = repo
        .find_remote("origin")
        .context("No remote named 'origin'. Is this a cloned registry?")?;

    let local_branch = detect_local_branch(repo)?;
    let refspec = format!("refs/heads/{}:refs/heads/{}", local_branch, local_branch);

    let callbacks = make_callbacks();
    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    remote
        .push(&[&refspec], Some(&mut push_opts))
        .with_context(|| format!("Failed to push to origin (branch: {})", local_branch))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// LWW Conflict Resolution (7.2)
// ---------------------------------------------------------------------------

/// Resolve merge conflicts using Last-Write-Wins strategy.
///
/// For each conflicting file, compares commit timestamps and keeps the newer version.
/// Returns the list of resolved file paths.
pub fn resolve_conflicts_lww(repo: &Repository) -> Result<Vec<String>> {
    let mut index = repo.index().context("Failed to get repository index")?;

    if !index.has_conflicts() {
        return Ok(Vec::new());
    }

    let mut resolved: Vec<String> = Vec::new();

    // Get HEAD commit time for "ours" side
    let head_time = repo
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .map(|commit| commit.time().seconds());

    // Get FETCH_HEAD commit time for "theirs" side
    let fetch_time = repo
        .find_reference("FETCH_HEAD")
        .ok()
        .and_then(|ref_| ref_.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .map(|commit| commit.time().seconds());

    // Collect conflicts first (can't iterate while modifying)
    let conflicts: Vec<_> = index
        .conflicts()
        .context("Failed to read merge conflicts")?
        .filter_map(|c| c.ok())
        .collect();

    for conflict in conflicts {
        let path = conflict
            .our
            .as_ref()
            .or(conflict.their.as_ref())
            .or(conflict.ancestor.as_ref())
            .and_then(|e| String::from_utf8(e.path.clone()).ok());

        if let Some(path_str) = path.clone() {
            // LWW: compare timestamps, keep newer
            let keep_ours = match (head_time, fetch_time) {
                (Some(local), Some(remote)) => local >= remote, // Keep local if equal or newer
                (Some(_), None) => true,  // No remote time, keep local
                (None, Some(_)) => false, // No local time, keep remote
                (None, None) => true,     // No times available, keep local (safe default)
            };

            if keep_ours {
                // Keep our version - checkout from HEAD
                if let Some(our) = &conflict.our {
                    let blob = repo.find_blob(our.id)
                        .context("Failed to find blob for our version")?;
                    let content = blob.content();
                    let full_path = repo.workdir()
                        .context("Repository has no working directory")?
                        .join(&path_str);
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("Failed to create parent dir for {}", path_str))?;
                    }
                    std::fs::write(&full_path, content)
                        .with_context(|| format!("Failed to write resolved file: {}", path_str))?;
                }
            } else {
                // Keep their version - checkout from FETCH_HEAD
                if let Some(their) = &conflict.their {
                    let blob = repo.find_blob(their.id)
                        .context("Failed to find blob for their version")?;
                    let content = blob.content();
                    let full_path = repo.workdir()
                        .context("Repository has no working directory")?
                        .join(&path_str);
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("Failed to create parent dir for {}", path_str))?;
                    }
                    std::fs::write(&full_path, content)
                        .with_context(|| format!("Failed to write resolved file: {}", path_str))?;
                }
            }

            resolved.push(path_str);
        }
    }

    // Re-add all resolved files to the index
    for path in &resolved {
        index.add_path(Path::new(path))
            .with_context(|| format!("Failed to stage resolved file: {}", path))?;
    }

    index.write().context("Failed to write resolved index")?;

    Ok(resolved)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Helper: create a minimal git repo with an initial commit.
    fn init_repo_with_commit(path: &Path) -> Repository {
        let repo = Repository::init(path).unwrap();

        // Configure author for tests.
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        // Create a file and commit it.
        fs::write(path.join("README.md"), "# Test\n").unwrap();

        {
            let mut index = repo.index().unwrap();
            index
                .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
                .unwrap();
            index.write().unwrap();

            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();

            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        repo
    }

    #[test]
    fn test_open_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        let repo = open_repo(dir.path());
        assert!(repo.is_ok());
    }

    #[test]
    fn test_open_repo_nonexistent() {
        let dir = tempdir().unwrap();
        let bad_path = dir.path().join("nope");
        assert!(open_repo(&bad_path).is_err());
    }

    #[test]
    fn test_repo_status_clean() {
        let dir = tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());
        let (has_changes, files) = repo_status(&repo).unwrap();
        assert!(!has_changes);
        assert!(files.is_empty());
    }

    #[test]
    fn test_repo_status_with_changes() {
        let dir = tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        // Create an untracked file.
        fs::write(dir.path().join("new.txt"), "hello").unwrap();

        let (has_changes, files) = repo_status(&repo).unwrap();
        assert!(has_changes);
        assert!(files.contains(&"new.txt".to_string()));
    }

    #[test]
    fn test_stage_all_and_commit() {
        let dir = tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        // Add a new file.
        fs::write(dir.path().join("added.txt"), "new content").unwrap();

        stage_all(&repo).unwrap();
        let oid = commit(&repo, "add file").unwrap();

        // The returned OID should be valid.
        assert!(repo.find_commit(oid).is_ok());

        // Working tree should be clean now.
        let (has_changes, _) = repo_status(&repo).unwrap();
        assert!(!has_changes);
    }

    #[test]
    fn test_stage_all_picks_up_deletions() {
        let dir = tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());

        // Delete the tracked file.
        fs::remove_file(dir.path().join("README.md")).unwrap();

        stage_all(&repo).unwrap();
        let oid = commit(&repo, "remove readme").unwrap();
        assert!(repo.find_commit(oid).is_ok());

        let (has_changes, _) = repo_status(&repo).unwrap();
        assert!(!has_changes);
    }

    #[test]
    fn test_fetch_origin_no_remote() {
        let dir = tempdir().unwrap();
        let repo = init_repo_with_commit(dir.path());
        // No origin remote configured — should error.
        assert!(fetch_origin(&repo).is_err());
    }
}
