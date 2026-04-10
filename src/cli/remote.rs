// Remote management for registry git repos

use anyhow::{Context, Result};
use console::style;
use git2::Repository;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::cli::RemoteAction;

/// Add a remote to the registry working repo.
pub fn remote_add(name: &str, url: &str, quiet: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let registry = &ss_paths.registry;

    if !registry.exists() {
        anyhow::bail!("Registry not initialized. Run `skillsync init` first.");
    }

    let repo = Repository::open(registry)
        .with_context(|| "Failed to open registry repository")?;

    // Check if remote already exists.
    if repo.find_remote(name).is_ok() {
        anyhow::bail!("Remote '{}' already exists. Use `skillsync remote set-url {} <new-url>` to update.", name, name);
    }

    repo.remote(name, url)
        .with_context(|| format!("Failed to add remote '{}' at {}", name, url))?;

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::RemoteAdded { name: name.to_string(), url: url.to_string() })
        );
    }

    Ok(())
}

/// Remove a remote from the registry working repo.
pub fn remote_remove(name: &str, quiet: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let registry = &ss_paths.registry;

    if !registry.exists() {
        anyhow::bail!("Registry not initialized. Run `skillsync init` first.");
    }

    let repo = Repository::open(registry)
        .with_context(|| "Failed to open registry repository")?;

    // Check if remote exists.
    if repo.find_remote(name).is_err() {
        anyhow::bail!("Remote '{}' does not exist.", name);
    }

    repo.remote_delete(name)
        .with_context(|| format!("Failed to remove remote '{}'", name))?;

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::RemoteRemoved { name: name.to_string() })
        );
    }

    Ok(())
}

/// List all remotes in the registry working repo.
pub fn remote_list(quiet: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let registry = &ss_paths.registry;

    if !registry.exists() {
        anyhow::bail!("Registry not initialized. Run `skillsync init` first.");
    }

    let repo = Repository::open(registry)
        .with_context(|| "Failed to open registry repository")?;

    let remotes = repo.remotes()?;
    let names: Vec<String> = remotes.iter().filter_map(|n| n.map(|s| s.to_string())).collect();

    if names.is_empty() {
        if !quiet {
            println!("No remotes configured.");
        }
        return Ok(());
    }

    if !quiet {
        for name in &names {
            let remote = repo.find_remote(name)?;
            let url = remote.url().unwrap_or("(no URL)");
            println!("  {}  {}", style(name).cyan().bold(), url);
        }
    }

    Ok(())
}

pub fn run(action: RemoteAction, quiet: bool) -> Result<()> {
    match action {
        RemoteAction::Add { name, url } => remote_add(&name, &url, quiet),
        RemoteAction::Remove { name } => remote_remove(&name, quiet),
        RemoteAction::List {} => remote_list(quiet),
    }
}
