use anyhow::{bail, Context, Result};
use console::style;

use crate::claude::paths::SkillSyncPaths;
use crate::registry::git_ops;

pub fn run(quiet: bool) -> Result<()> {
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

    // Check that an origin remote exists.
    if repo.find_remote("origin").is_err() {
        bail!(
            "No remote named 'origin' in the registry repository.\n\
             If this is a local-only registry, there is nothing to sync."
        );
    }

    if !quiet {
        println!(
            "{} Syncing registry with remote...\n",
            style("⟳").cyan().bold()
        );
    }

    // ---- Phase 1: Pull (fetch + merge) ------------------------------------
    if !quiet {
        println!(
            "{} Fetching from origin...",
            style("↓").cyan().bold()
        );
    }

    git_ops::fetch_origin(&repo).context("Fetch from origin failed")?;
    let merge = git_ops::merge_origin(&repo).context("Merge failed")?;

    // Report pull result.
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
        bail!(
            "Sync aborted due to merge conflicts. Run 'skillsync resolve' to fix them, then retry."
        );
    }

    if merge.up_to_date {
        if !quiet {
            println!(
                "{} Already up to date with remote.",
                style("✓").green().bold()
            );
        }
    } else if merge.fast_forward {
        if !quiet {
            println!(
                "{} Fast-forwarded to latest remote changes.",
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

    // ---- Phase 2: Push (stage, commit, push) ------------------------------
    let (has_changes, changed_files) =
        git_ops::repo_status(&repo).context("Failed to read repository status")?;

    if !has_changes {
        if !quiet {
            println!(
                "{} No local changes to push.",
                style("✓").green().bold()
            );
            println!(
                "\n{} Sync complete.",
                style("✓").green().bold()
            );
        }
        return Ok(());
    }

    if !quiet {
        println!(
            "\n{} Pushing {} local change(s)...",
            style("↑").cyan().bold(),
            changed_files.len()
        );
    }

    git_ops::stage_all(&repo).context("Failed to stage changes")?;
    git_ops::commit(&repo, "sync: update registry")
        .context("Failed to create commit")?;
    git_ops::push_origin(&repo).context("Failed to push to origin")?;

    if !quiet {
        println!(
            "{} Pushed {} change(s) to remote.",
            style("✓").green().bold(),
            changed_files.len()
        );
        for f in &changed_files {
            println!("  - {}", style(f).dim());
        }
        println!(
            "\n{} Sync complete.",
            style("✓").green().bold()
        );
    }

    Ok(())
}
