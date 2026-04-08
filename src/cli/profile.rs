use std::env;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::cli::ProfileAction;
use crate::claude::paths::{ProjectPaths, SkillSyncPaths};
use crate::i18n::Msg;
use crate::registry::manifest::{Manifest, ProfileConfig, ProfileRef, SkillSyncConfig};
use crate::registry::resource::copy_resource;

pub fn run(action: ProfileAction) -> Result<()> {
    match action {
        ProfileAction::List {} => list_profiles(),
        ProfileAction::Create { name } => create_profile(&name),
        ProfileAction::Apply { name } => apply_profile(&name),
        ProfileAction::Export { name } => export_profile(&name),
    }
}

// ---------------------------------------------------------------------------
// 8.1 — profile list
// ---------------------------------------------------------------------------

fn list_profiles() -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("{}", t!(Msg::ContextResolvePaths));
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if manifest.profiles.is_empty() {
        println!("{}", t!(Msg::ProfileListEmpty));
        println!("{}", t!(Msg::ProfileListHint { cmd: "skillsync profile create <name>".to_string() }));
        return Ok(());
    }

    // Collect profile  load each profile YAML to get counts.
    struct ProfileRow {
        name: String,
        description: String,
        skills: usize,
        plugins: usize,
        mcp: usize,
    }

    let mut rows: Vec<ProfileRow> = Vec::new();

    for (name, profile_ref) in &manifest.profiles {
        let profile_path = ss_paths.registry.join(&profile_ref.path);
        match ProfileConfig::load(&profile_path) {
            Ok(profile) => {
                rows.push(ProfileRow {
                    name: name.clone(),
                    description: profile.description.unwrap_or_default(),
                    skills: profile.skills.len(),
                    plugins: profile.plugins.len(),
                    mcp: profile.mcp.len(),
                });
            }
            Err(_) => {
                rows.push(ProfileRow {
                    name: name.clone(),
                    description: t!(Msg::ProfileErrorLoading { path: profile_ref.path.clone() }),
                    skills: 0,
                    plugins: 0,
                    mcp: 0,
                });
            }
        }
    }

    rows.sort_by(|a, b| a.name.cmp(&b.name));

    // Compute column widths.
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(4).max(7);
    let desc_w = rows
        .iter()
        .map(|r| r.description.len())
        .max()
        .unwrap_or(11)
        .max(11);

    // Print header.
    println!(
        "  {:<name_w$}  {:<desc_w$}  {:>6}  {:>7}  {:>3}",
        style(t!(Msg::ProfileColName)).bold().underlined(),
        style(t!(Msg::ProfileColDesc)).bold().underlined(),
        style(t!(Msg::ProfileColSkills)).bold().underlined(),
        style(t!(Msg::ProfileColPlugins)).bold().underlined(),
        style(t!(Msg::ProfileColMcp)).bold().underlined(),
        name_w = name_w,
        desc_w = desc_w,
    );

    for row in &rows {
        println!(
            "  {:<name_w$}  {:<desc_w$}  {:>6}  {:>7}  {:>3}",
            row.name,
            row.description,
            row.skills,
            row.plugins,
            row.mcp,
            name_w = name_w,
            desc_w = desc_w,
        );
    }

    println!();
    println!("  {}", t!(Msg::ProfileTotal { count: rows.len() }));

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.2 — profile create
// ---------------------------------------------------------------------------

fn create_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("{}", t!(Msg::ContextResolvePaths));
    }

    let mut manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    // Check for duplicate name.
    if manifest.profiles.contains_key(name) {
        bail!("{}", t!(Msg::ProfileAlreadyExists { name: name.to_string() }));
    }

    // Create a minimal profile YAML.
    let profile = ProfileConfig {
        name: name.to_string(),
        description: None,
        skills: vec![],
        plugins: vec![],
        mcp: vec![],
    };

    let relative_path = format!("profiles/{}.yaml", name);
    let profile_path = ss_paths.registry.join(&relative_path);
    profile
        .save(&profile_path)
        .with_context(|| t!(Msg::ContextCreateDir { path: profile_path.display().to_string() }))?;

    // Add ProfileRef to manifest.
    manifest.profiles.insert(
        name.to_string(),
        ProfileRef {
            path: relative_path.clone(),
        },
    );
    manifest
        .save(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToSaveManifest))?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ProfileCreateSuccess { name: name.to_string(), path: relative_path.clone() })
    );
    println!();
    println!(
        "  {}",
        t!(Msg::ProfileEditHint { path: profile_path.display().to_string() })
    );
    println!(
        "  {}",
        t!(Msg::ProfileExportHint { cmd: format!("skillsync profile export {}", name) })
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.3 — profile apply
// ---------------------------------------------------------------------------

fn apply_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("{}", t!(Msg::ContextResolvePaths));
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    // Look up the profile in the manifest.
    let profile_ref = manifest
        .profiles
        .get(name)
        .with_context(|| t!(Msg::ProfileNotFound { name: name.to_string() }))?;

    let profile_path = ss_paths.registry.join(&profile_ref.path);
    let profile = ProfileConfig::load(&profile_path)
        .with_context(|| t!(Msg::ContextCreateDir { path: profile_path.display().to_string() }))?;

    let cwd = env::current_dir().with_context(|| t!(Msg::ContextCurrentDir))?;
    let project = ProjectPaths::new(&cwd);
    project.ensure_dirs()?;

    let mut installed_skills = 0usize;
    let mut installed_mcp = 0usize;

    // Copy each skill from registry resources to the project .claude/skills/.
    for skill_name in &profile.skills {
        if let Some(skill_entry) = manifest.skills.get(skill_name) {
            let src = ss_paths.registry.join(&skill_entry.path);
            let dst = project.skills_dir.join(skill_name);
            if src.exists() {
                copy_resource(&src, &dst).with_context(|| {
                    t!(Msg::InstallerInstallFailed { name: skill_name.clone(), path: dst.display().to_string() })
                })?;
                installed_skills += 1;
                println!(
                    "  {} {}",
                    style("✓").green(),
                    t!(Msg::ProfileInstallSkill { name: skill_name.clone() })
                );
            } else {
                eprintln!(
                    "  {} {}",
                    style("✗").red(),
                    t!(Msg::ProfileSkillSourceNotFound { name: skill_name.clone(), path: src.display().to_string() })
                );
            }
        } else {
            eprintln!(
                "  {} {}",
                style("⚠").yellow(),
                t!(Msg::ProfileSkillNotFound { name: skill_name.clone() })
            );
        }
    }

    // Merge MCP configs: write/merge entries into project .mcp.json.
    for mcp_name in &profile.mcp {
        if let Some(mcp_entry) = manifest.mcp_servers.get(mcp_name) {
            merge_mcp_entry(&project.mcp_json, mcp_name, mcp_entry)?;
            installed_mcp += 1;
            println!(
                "  {} {}",
                style("✓").green(),
                t!(Msg::ProfileInstallMcp { name: mcp_name.clone() })
            );
        } else {
            eprintln!(
                "  {} {}",
                style("⚠").yellow(),
                t!(Msg::ProfileMcpNotFound { name: mcp_name.clone() })
            );
        }
    }

    // Write skillsync.yaml.
    let config = SkillSyncConfig {
        profile: Some(name.to_string()),
        skills: profile.skills.clone(),
        plugins: profile.plugins.clone(),
        mcp: profile.mcp.clone(),
    };
    config
        .save(&project.skillsync_yaml)
        .with_context(|| t!(Msg::ContextFailedToSaveManifest))?;

    println!();
    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ProfileApplySuccess {
            name: name.to_string(),
            skills: installed_skills,
            plugins: profile.plugins.len(),
            mcp: installed_mcp
        }),
    );
    println!(
        "  {}",
        t!(Msg::ProfileConfigWritten { path: project.skillsync_yaml.display().to_string() })
    );

    Ok(())
}

/// Merge a single MCP server entry into a .mcp.json file.
fn merge_mcp_entry(
    mcp_json_path: &std::path::Path,
    name: &str,
    entry: &crate::registry::manifest::McpServerEntry,
) -> Result<()> {
    // Read existing .mcp.json or start from empty object.
    let mut root: serde_json::Value = if mcp_json_path.exists() {
        let contents = std::fs::read_to_string(mcp_json_path)
            .with_context(|| t!(Msg::ContextReadDir { path: mcp_json_path.display().to_string() }))?;
        serde_json::from_str(&contents)
            .with_context(|| t!(Msg::ContextReadDir { path: mcp_json_path.display().to_string() }))?
    } else {
        serde_json::json!({ "mcpServers": {} })
    };

    // Ensure mcpServers key exists.
    let servers = root
        .as_object_mut()
        .context(t!(Msg::ContextFailedToLoadManifest))?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let server_obj = servers
        .as_object_mut()
        .context(t!(Msg::ContextFailedToLoadManifest))?;

    // Build the server entry.
    let mut server_value = serde_json::json!({
        "command": entry.command,
    });
    if !entry.args.is_empty() {
        server_value["args"] = serde_json::json!(entry.args);
    }

    server_obj.insert(name.to_string(), server_value);

    // Write back.
    let formatted = serde_json::to_string_pretty(&root)
        .context(t!(Msg::ContextFailedToSaveManifest))?;
    std::fs::write(mcp_json_path, formatted)
        .with_context(|| t!(Msg::ContextCreateDir { path: mcp_json_path.display().to_string() }))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.4 — profile export
// ---------------------------------------------------------------------------

fn export_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("{}", t!(Msg::ContextResolvePaths));
    }

    let cwd = env::current_dir().with_context(|| t!(Msg::ContextCurrentDir))?;
    let project = ProjectPaths::new(&cwd);

    if !project.has_config() {
        bail!("{}", t!(Msg::ProfileExportNoConfig));
    }

    let config = SkillSyncConfig::load(&project.skillsync_yaml)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    let mut manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    // Check if profile already exists.
    if manifest.profiles.contains_key(name) {
        bail!("{}", t!(Msg::ProfileAlreadyExists { name: name.to_string() }));
    }

    // Build the profile from the project config.
    let profile = ProfileConfig {
        name: name.to_string(),
        description: Some(format!("Exported from {}", cwd.display())),
        skills: config.skills,
        plugins: config.plugins,
        mcp: config.mcp,
    };

    let relative_path = format!("profiles/{}.yaml", name);
    let profile_path = ss_paths.registry.join(&relative_path);
    profile
        .save(&profile_path)
        .with_context(|| t!(Msg::ContextCreateDir { path: profile_path.display().to_string() }))?;

    // Add ProfileRef to manifest.
    manifest.profiles.insert(
        name.to_string(),
        ProfileRef {
            path: relative_path.clone(),
        },
    );
    manifest
        .save(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToSaveManifest))?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::ProfileExportSuccess { name: name.to_string() }),
    );
    println!(
        "  {}",
        t!(Msg::ProfileExportSummary {
            skills: profile.skills.len(),
            plugins: profile.plugins.len(),
            mcp: profile.mcp.len()
        }),
    );
    println!(
        "  {}",
        t!(Msg::ProfileSavedTo { path: profile_path.display().to_string() })
    );

    Ok(())
}
