// Interactive project configuration — `skillsync use`
// Implements: tasks 4.1–4.6

use std::collections::HashMap;
use std::env;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::{ProjectPaths, SkillSyncPaths};
use crate::i18n::Msg;
use crate::installer::mcp_installer::merge_mcp_config;
use crate::installer::settings_merger::write_skillsync_config;
use crate::installer::skill_installer::install_project_skills;
use crate::registry::{Manifest, McpServerEntry, ProfileConfig, SkillSyncConfig};
use crate::tui::profile_picker;
use crate::tui::selector::{self, ConfigMethod, SelectedResources};

pub fn run() -> Result<()> {
    // 1. Resolve paths and check that the registry exists.
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!(
            "Registry not initialized. Run `skillsync init` first."
        );
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    let project_root = env::current_dir()
        .with_context(|| t!(Msg::ContextCurrentDir))?;
    let project = ProjectPaths::new(&project_root);

    println!(
        "{} {}",
        style(">>").cyan().bold(),
        t!(Msg::UseConfiguring { path: project_root.display().to_string() })
    );

    // 2. Ask how to configure.
    let method = selector::choose_config_method()?;

    // 3. Branch by method.
    let selected = match method {
        ConfigMethod::FromProfile => configure_from_profile(&manifest, &ss_paths)?,
        ConfigMethod::Manual => selector::select_resources(&manifest, &[])?,
        ConfigMethod::CopyFromProject => configure_from_project(&manifest)?,
    };

    // 4. Preview and confirm.
    let confirmed = selector::confirm_preview(&selected)?;
    if !confirmed {
        println!("{}", style(t!(Msg::UseCancelled)).yellow());
        return Ok(());
    }

    // 5. Apply: install skills, merge MCP config, write skillsync.yaml.
    apply_selection(&selected, &manifest, &ss_paths, &project)?;

    println!();
    println!(
        "{} {}",
        style("OK").green().bold(),
        t!(Msg::UseSuccess)
    );

    Ok(())
}

/// FromProfile flow: pick a profile, load it, then let user adjust selections.
fn configure_from_profile(
    manifest: &Manifest,
    ss_paths: &SkillSyncPaths,
) -> Result<SelectedResources> {
    let profile_name = profile_picker::pick_profile(manifest, &ss_paths.registry)?;

    // Load the profile to get pre-selected resources.
    let profile_ref = manifest
        .profiles
        .get(&profile_name)
        .context("Selected profile not found in manifest")?;
    let profile_path = ss_paths.registry.join(&profile_ref.path);
    let profile = ProfileConfig::load(&profile_path)
        .with_context(|| format!("Failed to load profile '{}'", profile_name))?;

    // Combine all profile resources as pre-selected names.
    let mut pre_selected: Vec<String> = Vec::new();
    pre_selected.extend(profile.skills.iter().cloned());
    pre_selected.extend(profile.plugins.iter().cloned());
    pre_selected.extend(profile.mcp.iter().cloned());

    println!(
        "  {}",
        t!(Msg::UseProfilePreSelects { profile: profile_name.clone(), count: pre_selected.len() })
    );

    selector::select_resources(manifest, &pre_selected)
}

/// CopyFromProject flow (task 4.5): find another project's skillsync.yaml and
/// use its resources as the starting selection.
fn configure_from_project(manifest: &Manifest) -> Result<SelectedResources> {
    // Look in common locations for projects with skillsync.yaml.
    // We search the parent directory of the current project for sibling projects.
    let cwd = env::current_dir()
        .with_context(|| t!(Msg::ContextCurrentDir))?;

    let mut candidates: Vec<String> = Vec::new();

    if let Some(parent) = cwd.parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path != cwd {
                    let candidate = ProjectPaths::new(&path);
                    if candidate.has_config() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            candidates.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    if candidates.is_empty() {
        println!(
            "{}",
            style(t!(Msg::UseNoSiblingProjects)).yellow()
        );
        println!("{}", t!(Msg::UseFallbackManual));
        return selector::select_resources(manifest, &[]);
    }

    candidates.sort();

    let selected_project = inquire::Select::new(
        &t!(Msg::UseSelectProject),
        candidates,
    )
    .prompt()
    .context("Project selection was cancelled")?;

    // Load the source project's config.
    let parent = cwd
        .parent()
        .context("Cannot determine parent directory")?;
    let source_project = ProjectPaths::new(&parent.join(&selected_project));
    let config = SkillSyncConfig::load(&source_project.skillsync_yaml)
        .with_context(|| {
            format!(
                "Failed to load skillsync.yaml from project '{}'",
                selected_project
            )
        })?;

    // Build pre-selected list from the source config.
    let mut pre_selected: Vec<String> = Vec::new();
    pre_selected.extend(config.skills.iter().cloned());
    pre_selected.extend(config.plugins.iter().cloned());
    pre_selected.extend(config.mcp.iter().cloned());

    println!(
        "  {}",
        t!(Msg::UseLoadedResources { count: pre_selected.len(), project: selected_project.clone() })
    );

    selector::select_resources(manifest, &pre_selected)
}

/// Apply the selected resources: install skills, merge MCP, write config.
fn apply_selection(
    selected: &SelectedResources,
    manifest: &Manifest,
    ss_paths: &SkillSyncPaths,
    project: &ProjectPaths,
) -> Result<()> {
    // Ensure project directories exist.
    project.ensure_dirs()?;

    // Install skills.
    if !selected.skills.is_empty() {
        let results = install_project_skills(
            &selected.skills,
            manifest,
            &ss_paths.registry,
            &project.skills_dir,
        )?;
        for r in &results {
            println!("  {} {}", style("*").cyan(), r);
        }
    }

    // Merge MCP server configurations.
    if !selected.mcp.is_empty() {
        let mut mcp_servers: HashMap<String, McpServerEntry> = HashMap::new();
        for name in &selected.mcp {
            if let Some(entry) = manifest.mcp_servers.get(name) {
                mcp_servers.insert(name.clone(), entry.clone());
            } else {
                eprintln!(
                    "  {}",
                    t!(Msg::UseMcpNotFound { name: name.clone() })
                );
            }
        }
        if !mcp_servers.is_empty() {
            merge_mcp_config(&mcp_servers, &project.mcp_json)?;
            println!(
                "  {}",
                t!(Msg::UseMergedMcp { count: mcp_servers.len(), path: project.mcp_json.display().to_string() })
            );
        }
    }

    // Write skillsync.yaml.
    let config = SkillSyncConfig {
        profile: None,
        skills: selected.skills.clone(),
        plugins: selected.plugins.clone(),
        mcp: selected.mcp.clone(),
    };
    write_skillsync_config(&config, &project.skillsync_yaml)?;
    println!(
        "  {}",
        t!(Msg::UseWrote { path: project.skillsync_yaml.display().to_string() })
    );

    Ok(())
}
