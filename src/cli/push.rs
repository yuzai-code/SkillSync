use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::registry::git_ops;

pub fn run(auto: bool, quiet: bool) -> Result<()> {
    let paths = SkillSyncPaths::resolve().context("Failed to resolve SkillSync paths")?;

    if !paths.registry_exists() {
        bail!("{}", t!(Msg::PushRegistryNotFound { path: paths.registry.display().to_string() }));
    }

    let repo = git_ops::open_repo(&paths.registry)
        .context("Failed to open registry git repository")?;

    // Check that an origin remote exists before attempting to push.
    if repo.find_remote("origin").is_err() {
        bail!("{}", t!(Msg::PushNoOrigin));
    }

    // Check for local changes.
    let (has_changes, changed_files) =
        git_ops::repo_status(&repo).context("Failed to read repository status")?;

    if !has_changes {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::PushNothingToPush)
            );
        }
        return Ok(());
    }

    // Stage everything.
    git_ops::stage_all(&repo).context("Failed to stage changes")?;

    // Build the commit message.
    let message = if auto {
        "auto: sync changes".to_string()
    } else {
        build_commit_message(&changed_files)
    };

    if !quiet {
        println!(
            "{} {}",
            style("→").cyan().bold(),
            t!(Msg::PushCommitting { count: changed_files.len() })
        );
    }

    git_ops::commit(&repo, &message).context("Failed to create commit")?;

    if !quiet {
        println!(
            "{} {}",
            style("↑").cyan().bold(),
            t!(Msg::PushPushing)
        );
    }

    git_ops::push_origin(&repo).context("Failed to push to origin")?;

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::PushSuccess { count: changed_files.len() })
        );
        for f in &changed_files {
            println!("{}", t!(Msg::PushCommitFile { file: f.clone() }));
        }
    }

    Ok(())
}

/// Build a descriptive commit message from the list of changed files.
fn build_commit_message(changed_files: &[String]) -> String {
    if changed_files.len() == 1 {
        return format!("update: {}", changed_files[0]);
    }

    // Categorize changes by directory/type.
    let mut skills = 0u32;
    let mut plugins = 0u32;
    let mut mcp = 0u32;
    let mut profiles = 0u32;
    let mut other = 0u32;

    for f in changed_files {
        if f.contains("skills") {
            skills += 1;
        } else if f.contains("plugins") {
            plugins += 1;
        } else if f.contains("mcp") {
            mcp += 1;
        } else if f.contains("profiles") || f.contains("profile") {
            profiles += 1;
        } else {
            other += 1;
        }
    }

    let mut parts: Vec<String> = Vec::new();
    if skills > 0 {
        parts.push(format!("{} skill(s)", skills));
    }
    if plugins > 0 {
        parts.push(format!("{} plugin(s)", plugins));
    }
    if mcp > 0 {
        parts.push(format!("{} MCP server(s)", mcp));
    }
    if profiles > 0 {
        parts.push(format!("{} profile(s)", profiles));
    }
    if other > 0 {
        parts.push(format!("{} other", other));
    }

    if parts.is_empty() {
        "update registry".to_string()
    } else {
        format!("update: {}", parts.join(", "))
    }
}
