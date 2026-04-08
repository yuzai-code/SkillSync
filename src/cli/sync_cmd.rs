use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::registry::git_ops;

pub fn run(quiet: bool) -> Result<()> {
    let paths = SkillSyncPaths::resolve().context("Failed to resolve SkillSync paths")?;

    if !paths.registry_exists() {
        bail!("{}", t!(Msg::SyncRegistryNotFound { path: paths.registry.display().to_string() }));
    }

    let repo = git_ops::open_repo(&paths.registry)
        .context("Failed to open registry git repository")?;

    // Check that an origin remote exists.
    if repo.find_remote("origin").is_err() {
        bail!("{}", t!(Msg::SyncNoOrigin));
    }

    if !quiet {
        println!(
            "{} {}",
            style("⟳").cyan().bold(),
            t!(Msg::SyncSyncing)
        );
    }

    // ---- Phase 1: Pull (fetch + merge) ------------------------------------
    if !quiet {
        println!(
            "{} {}",
            style("↓").cyan().bold(),
            t!(Msg::SyncFetching)
        );
    }

    git_ops::fetch_origin(&repo).context("Fetch from origin failed")?;
    let merge = git_ops::merge_origin(&repo).context("Merge failed")?;

    // Report pull result.
    if !merge.conflicts.is_empty() {
        eprintln!(
            "{} {}",
            style("✗").red().bold(),
            t!(Msg::SyncMergeConflicts { count: merge.conflicts.len() })
        );
        for path in &merge.conflicts {
            eprintln!("{}", t!(Msg::SyncConflictFile { file: path.clone() }));
        }
        eprintln!("{}", t!(Msg::SyncResolveHint));
        bail!("{}", t!(Msg::SyncAborted));
    }

    if merge.up_to_date {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncUpToDate)
            );
        }
    } else if merge.fast_forward {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncFastForwarded)
            );
        }
    } else {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncMerged)
            );
        }
    }

    // ---- Phase 2: Push (stage, commit, push) ------------------------------
    let (has_changes, changed_files) =
        git_ops::repo_status(&repo).context("Failed to read repository status")?;

    if !has_changes {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncNoLocalChanges)
            );
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncComplete)
            );
        }
        return Ok(());
    }

    if !quiet {
        println!(
            "{} {}",
            style("↑").cyan().bold(),
            t!(Msg::SyncPushing { count: changed_files.len() })
        );
    }

    git_ops::stage_all(&repo).context("Failed to stage changes")?;
    git_ops::commit(&repo, "sync: update registry")
        .context("Failed to create commit")?;
    git_ops::push_origin(&repo).context("Failed to push to origin")?;

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::SyncPushed { count: changed_files.len() })
        );
        for f in &changed_files {
            println!("{}", t!(Msg::SyncCommitFile { file: f.clone() }));
        }
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::SyncComplete)
        );
    }

    Ok(())
}
