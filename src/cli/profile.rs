use std::env;

use anyhow::{bail, Context, Result};
use console::style;

use crate::cli::ProfileAction;
use crate::claude::paths::{ProjectPaths, SkillSyncPaths};
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
        bail!("Registry not found. Run 'skillsync init' first.");
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .context("Failed to load manifest")?;

    if manifest.profiles.is_empty() {
        println!("No profiles found in the registry.");
        println!(
            "Use '{}' to create one.",
            style("skillsync profile create <name>").cyan()
        );
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
                    description: format!("(error loading {})", profile_ref.path),
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
        style("Profile").bold().underlined(),
        style("Description").bold().underlined(),
        style("Skills").bold().underlined(),
        style("Plugins").bold().underlined(),
        style("MCP").bold().underlined(),
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
    println!("  {} profile(s) total", style(rows.len()).bold());

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.2 — profile create
// ---------------------------------------------------------------------------

fn create_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("Registry not found. Run 'skillsync init' first.");
    }

    let mut manifest = Manifest::load(&ss_paths.manifest)
        .context("Failed to load manifest")?;

    // Check for duplicate name.
    if manifest.profiles.contains_key(name) {
        bail!(
            "Profile '{}' already exists. Remove it first with 'skillsync remove' or choose a different name.",
            name
        );
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
        .with_context(|| format!("Failed to write profile to {}", profile_path.display()))?;

    // Add ProfileRef to manifest.
    manifest.profiles.insert(
        name.to_string(),
        ProfileRef {
            path: relative_path.clone(),
        },
    );
    manifest
        .save(&ss_paths.manifest)
        .context("Failed to save manifest")?;

    println!(
        "{} Created profile '{}' at {}",
        style("✓").green().bold(),
        style(name).cyan(),
        style(&relative_path).dim()
    );
    println!();
    println!(
        "  Edit {} to add skills, plugins, and MCP servers.",
        style(profile_path.display().to_string()).dim()
    );
    println!(
        "  Or use '{}' to populate from a project config.",
        style(format!("skillsync profile export {}", name)).cyan()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.3 — profile apply
// ---------------------------------------------------------------------------

fn apply_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("Registry not found. Run 'skillsync init' first.");
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .context("Failed to load manifest")?;

    // Look up the profile in the manifest.
    let profile_ref = manifest
        .profiles
        .get(name)
        .with_context(|| format!("Profile '{}' not found in manifest.", name))?;

    let profile_path = ss_paths.registry.join(&profile_ref.path);
    let profile = ProfileConfig::load(&profile_path)
        .with_context(|| format!("Failed to load profile from {}", profile_path.display()))?;

    let cwd = env::current_dir().context("Failed to determine current directory")?;
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
                    format!("Failed to copy skill '{}' to project", skill_name)
                })?;
                installed_skills += 1;
                println!(
                    "  {} Installed skill '{}'",
                    style("✓").green(),
                    skill_name
                );
            } else {
                eprintln!(
                    "  {} Skill '{}' source not found at {}",
                    style("✗").red(),
                    skill_name,
                    src.display()
                );
            }
        } else {
            eprintln!(
                "  {} Skill '{}' not found in manifest (skipped)",
                style("⚠").yellow(),
                skill_name
            );
        }
    }

    // Merge MCP configs: write/merge entries into project .mcp.json.
    for mcp_name in &profile.mcp {
        if let Some(mcp_entry) = manifest.mcp_servers.get(mcp_name) {
            merge_mcp_entry(&project.mcp_json, mcp_name, mcp_entry)?;
            installed_mcp += 1;
            println!(
                "  {} Installed MCP server '{}'",
                style("✓").green(),
                mcp_name
            );
        } else {
            eprintln!(
                "  {} MCP server '{}' not found in manifest (skipped)",
                style("⚠").yellow(),
                mcp_name
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
        .context("Failed to write skillsync.yaml")?;

    println!();
    println!(
        "{} Applied profile '{}': {} skill(s), {} plugin(s), {} MCP server(s)",
        style("✓").green().bold(),
        style(name).cyan(),
        installed_skills,
        profile.plugins.len(),
        installed_mcp,
    );
    println!(
        "  Config written to {}",
        style(project.skillsync_yaml.display().to_string()).dim()
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
            .with_context(|| format!("Failed to read {}", mcp_json_path.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse {}", mcp_json_path.display()))?
    } else {
        serde_json::json!({ "mcpServers": {} })
    };

    // Ensure mcpServers key exists.
    let servers = root
        .as_object_mut()
        .context("Expected .mcp.json to be a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let server_obj = servers
        .as_object_mut()
        .context("Expected mcpServers to be a JSON object")?;

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
        .context("Failed to serialize .mcp.json")?;
    std::fs::write(mcp_json_path, formatted)
        .with_context(|| format!("Failed to write {}", mcp_json_path.display()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// 8.4 — profile export
// ---------------------------------------------------------------------------

fn export_profile(name: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        bail!("Registry not found. Run 'skillsync init' first.");
    }

    let cwd = env::current_dir().context("Failed to determine current directory")?;
    let project = ProjectPaths::new(&cwd);

    if !project.has_config() {
        bail!(
            "No skillsync.yaml found in current project.\n\
             Use 'skillsync use' to configure the project first."
        );
    }

    let config = SkillSyncConfig::load(&project.skillsync_yaml)
        .context("Failed to load project skillsync.yaml")?;

    let mut manifest = Manifest::load(&ss_paths.manifest)
        .context("Failed to load manifest")?;

    // Check if profile already exists.
    if manifest.profiles.contains_key(name) {
        bail!(
            "Profile '{}' already exists. Remove it first or choose a different name.",
            name
        );
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
        .with_context(|| format!("Failed to write profile to {}", profile_path.display()))?;

    // Add ProfileRef to manifest.
    manifest.profiles.insert(
        name.to_string(),
        ProfileRef {
            path: relative_path.clone(),
        },
    );
    manifest
        .save(&ss_paths.manifest)
        .context("Failed to save manifest")?;

    println!(
        "{} Exported project config as profile '{}'",
        style("✓").green().bold(),
        style(name).cyan(),
    );
    println!(
        "  {} skill(s), {} plugin(s), {} MCP server(s)",
        profile.skills.len(),
        profile.plugins.len(),
        profile.mcp.len(),
    );
    println!(
        "  Saved to {}",
        style(profile_path.display().to_string()).dim()
    );

    Ok(())
}
