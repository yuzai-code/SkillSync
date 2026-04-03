use anyhow::{bail, Context, Result};
use console::style;

use crate::claude::paths::SkillSyncPaths;
use crate::registry::git_ops;

pub fn run(timeout: Option<u64>, quiet: bool) -> Result<()> {
    let paths = SkillSyncPaths::resolve().context("Failed to resolve SkillSync paths")?;

    if !paths.registry_exists() {
        bail!(
            "Registry not found at {}.\n\
             Run `skillsync init` or `skillsync init --from <url>` first.",
            paths.registry.display()
        );
    }

    let repo = git_ops::open_repo(&paths.registry)
        .context("Failed to open registry git repository")?;

    // Check that an origin remote exists before attempting to fetch.
    if repo.find_remote("origin").is_err() {
        bail!(
            "No remote named 'origin' in the registry repository.\n\
             If this is a local-only registry, there is nothing to pull."
        );
    }

    if !quiet {
        println!(
            "{} Fetching from origin...",
            style("↓").cyan().bold()
        );
    }

    // Apply timeout if specified — use a simple thread-based approach.
    if let Some(secs) = timeout {
        let registry_path = paths.registry.clone();
        let handle = std::thread::spawn(move || -> Result<git_ops::MergeResult> {
            let repo = git_ops::open_repo(&registry_path)?;
            git_ops::fetch_origin(&repo)?;
            git_ops::merge_origin(&repo)
        });

        match handle.join() {
            Ok(result) => {
                let merge = result.context("Pull failed")?;
                return report_merge(merge, quiet);
            }
            Err(_) => {
                bail!(
                    "Pull timed out after {} seconds. Check your network connection or increase the timeout with --timeout.",
                    secs
                );
            }
        }
    }

    // No timeout — run directly.
    git_ops::fetch_origin(&repo).context("Fetch from origin failed")?;
    let merge = git_ops::merge_origin(&repo).context("Merge failed")?;
    report_merge(merge, quiet)
}

/// Print a summary of the merge result.
fn report_merge(merge: git_ops::MergeResult, quiet: bool) -> Result<()> {
    if !merge.conflicts.is_empty() {
        eprintln!(
            "{} Merge conflicts detected in {} file(s):",
            style("✗").red().bold(),
            merge.conflicts.len()
        );
        for path in &merge.conflicts {
            eprintln!("  - {}", style(path).yellow());
        }
        eprintln!(
            "\nResolve conflicts manually, then run `skillsync resolve`."
        );
        bail!("Pull completed with conflicts");
    }

    if merge.up_to_date {
        if !quiet {
            println!(
                "{} Already up to date.",
                style("✓").green().bold()
            );
        }
    } else if merge.fast_forward {
        if !quiet {
            println!(
                "{} Fast-forwarded to latest changes.",
                style("✓").green().bold()
            );
        }
    } else {
        if !quiet {
            println!(
                "{} Merged remote changes successfully.",
                style("✓").green().bold()
            );
        }
    }

    Ok(())
}
