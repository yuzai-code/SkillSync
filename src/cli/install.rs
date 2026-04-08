use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::{ClaudePaths, ProjectPaths, SkillSyncPaths};
use crate::i18n::Msg;
use crate::installer::mcp_installer::merge_mcp_config;
use crate::installer::settings_merger::{write_lock_file, LockEntry};
use crate::installer::skill_installer::{
    install_global_skills, install_project_skills, InstallResult,
};
use crate::registry::{Manifest, McpServerEntry, ResourceScope, SkillSyncConfig};

/// Print a summary of install results to stdout.
fn print_summary(label: &str, results: &[InstallResult]) {
    if results.is_empty() {
        return;
    }

    let installed = results
        .iter()
        .filter(|r| matches!(r, InstallResult::Installed(_)))
        .count();
    let updated = results
        .iter()
        .filter(|r| matches!(r, InstallResult::Updated(_)))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, InstallResult::Skipped(_)))
        .count();

    println!("  {} {}:", style(label).bold(), style("skills").dim());
    for r in results {
        let (icon, color_name) = match r {
            InstallResult::Installed(n) => ("+", style(n).green()),
            InstallResult::Updated(n) => ("~", style(n).yellow()),
            InstallResult::Skipped(n) => ("=", style(n).dim()),
        };
        println!("    {} {}", icon, color_name);
    }

    let mut parts = Vec::new();
    if installed > 0 {
        parts.push(format!("{} installed", installed));
    }
    if updated > 0 {
        parts.push(format!("{} updated", updated));
    }
    if skipped > 0 {
        parts.push(format!("{} up-to-date", skipped));
    }
    println!("    ({})", parts.join(", "));
}

/// Build lock entries from skill install results by reading hashes from the
/// installed locations.
fn build_skill_lock_entries(
    results: &[InstallResult],
    manifest: &Manifest,
) -> Vec<LockEntry> {
    results
        .iter()
        .filter_map(|r| {
            let name = r.name();
            let entry = manifest.skills.get(name)?;
            Some(LockEntry {
                name: name.to_string(),
                resource_type: "skill".to_string(),
                version: entry.version.clone(),
                hash: entry.backup_hash.clone().unwrap_or_default(),
            })
        })
        .collect()
}

/// Build lock entries for MCP servers.
fn build_mcp_lock_entries(
    names: &[String],
    manifest: &Manifest,
) -> Vec<LockEntry> {
    names
        .iter()
        .filter_map(|name| {
            let entry = manifest.mcp_servers.get(name)?;
            // MCP servers don't have file hashes; record the command as a
            // fingerprint so the lock file still tracks what was installed.
            Some(LockEntry {
                name: name.clone(),
                resource_type: "mcp_server".to_string(),
                version: "0.0.0".to_string(),
                hash: format!("cmd:{} {}", entry.command, entry.args.join(" ")),
            })
        })
        .collect()
}

/// `skillsync install --global` — install all global-scope skills and MCP
/// servers from the registry manifest.
fn run_global(ss_paths: &SkillSyncPaths) -> Result<()> {
    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    let claude = ClaudePaths::global()
        .with_context(|| t!(Msg::ContextResolvePaths))?;
    claude.ensure_dirs()
        .with_context(|| t!(Msg::ContextCreateDir { path: claude.home.display().to_string() }))?;

    println!(
        "{} {}",
        style("→").cyan().bold(),
        t!(Msg::InstallGlobal)
    );

    // --- Skills ---
    let skill_results =
        install_global_skills(&manifest, &ss_paths.registry, &claude.skills_dir)?;
    print_summary("Global", &skill_results);

    // --- MCP servers (global scope) ---
    let global_mcp: HashMap<String, McpServerEntry> = manifest
        .mcp_servers
        .iter()
        .filter(|(_, e)| e.scope == ResourceScope::Global)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if !global_mcp.is_empty() {
        merge_mcp_config(&global_mcp, &claude.mcp_json)
            .context("Failed to merge global MCP config")?;
        println!(
            "  {}",
            t!(Msg::InstallMcpMerged { count: global_mcp.len(), path: claude.mcp_json.display().to_string() })
        );
    }

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::InstallGlobalComplete)
    );

    Ok(())
}

/// `skillsync install` (no flags) — read the project's `.claude/skillsync.yaml`
/// and install all declared resources.
fn run_project(ss_paths: &SkillSyncPaths) -> Result<()> {
    let cwd = std::env::current_dir()
        .with_context(|| t!(Msg::ContextCurrentDir))?;
    let project = ProjectPaths::new(&cwd);

    if !project.has_config() {
        bail!(
            "{}",
            t!(Msg::InstallNoConfig { path: project.skillsync_yaml.display().to_string() })
        );
    }

    let config = SkillSyncConfig::load(&project.skillsync_yaml)
        .with_context(|| t!(Msg::InstallNoConfig { path: project.skillsync_yaml.display().to_string() }))?;

    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    project.ensure_dirs()
        .with_context(|| t!(Msg::ContextCreateDir { path: project.root.display().to_string() }))?;

    println!(
        "{} {}",
        style("→").cyan().bold(),
        t!(Msg::InstallProject { path: cwd.display().to_string() })
    );

    let mut all_lock_entries: Vec<LockEntry> = Vec::new();

    // --- Skills ---
    let skill_results = if !config.skills.is_empty() {
        let results = install_project_skills(
            &config.skills,
            &manifest,
            &ss_paths.registry,
            &project.skills_dir,
        )?;
        all_lock_entries.extend(build_skill_lock_entries(&results, &manifest));
        results
    } else {
        Vec::new()
    };
    print_summary("Project", &skill_results);

    // --- MCP servers ---
    if !config.mcp.is_empty() {
        let mcp_servers: HashMap<String, McpServerEntry> = config
            .mcp
            .iter()
            .filter_map(|name| {
                manifest
                    .mcp_servers
                    .get(name)
                    .map(|e| (name.clone(), e.clone()))
            })
            .collect();

        // Warn about any MCP servers referenced in config but missing from manifest.
        for name in &config.mcp {
            if !manifest.mcp_servers.contains_key(name) {
                eprintln!(
                    "  {}",
                    t!(Msg::InstallMcpNotFound { name: name.clone() })
                );
            }
        }

        if !mcp_servers.is_empty() {
            merge_mcp_config(&mcp_servers, &project.mcp_json)
                .context("Failed to merge project MCP config")?;
            println!(
                "  {}",
                t!(Msg::InstallMcpMerged { count: mcp_servers.len(), path: project.mcp_json.display().to_string() })
            );

            let installed_names: Vec<String> = mcp_servers.keys().cloned().collect();
            all_lock_entries.extend(build_mcp_lock_entries(&installed_names, &manifest));
        }
    }

    // --- Write lock file ---
    if !all_lock_entries.is_empty() {
        write_lock_file(&all_lock_entries, &project.skillsync_lock)
            .context("Failed to write lock file")?;
        println!(
            "  {}",
            t!(Msg::InstallLockWritten { path: project.skillsync_lock.display().to_string() })
        );
    }

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::InstallProjectComplete)
    );

    Ok(())
}

pub fn run(global: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;

    if !ss_paths.registry_exists() {
        bail!(
            "{}",
            t!(Msg::SyncRegistryNotFound { path: ss_paths.registry.display().to_string() })
        );
    }

    if global {
        run_global(&ss_paths)
    } else {
        run_project(&ss_paths)
    }
}
