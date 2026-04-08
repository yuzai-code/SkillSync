use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::registry::git_ops;

pub fn run(timeout: Option<u64>, quiet: bool) -> Result<()> {
    let paths = SkillSyncPaths::resolve().context("Failed to resolve SkillSync paths")?;

    if !paths.registry_exists() {
        bail!("{}", t!(Msg::PullRegistryNotFound { path: paths.registry.display().to_string() }));
    }

    let repo = git_ops::open_repo(&paths.registry)
        .context("Failed to open registry git repository")?;

    // Check that an origin remote exists before attempting to fetch.
    if repo.find_remote("origin").is_err() {
        bail!("{}", t!(Msg::PullNoOrigin));
    }

    if !quiet {
        println!(
            "{} {}",
            style("↓").cyan().bold(),
            t!(Msg::PullFetching)
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
                bail!("{}", t!(Msg::PullTimedOut { secs }));
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
            "{} {}",
            style("✗").red().bold(),
            t!(Msg::PullMergeConflicts { count: merge.conflicts.len() })
        );
        for path in &merge.conflicts {
            eprintln!("{}", t!(Msg::PullConflictFile { file: path.clone() }));
        }
        eprintln!("{}", t!(Msg::PullResolveHint));
        bail!("{}", t!(Msg::PullConflicts));
    }

    if merge.up_to_date {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::PullUpToDate)
            );
        }
    } else if merge.fast_forward {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::PullFastForwarded)
            );
        }
    } else {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::PullMerged)
            );
        }
    }

    Ok(())
}
