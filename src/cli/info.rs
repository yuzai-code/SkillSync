use anyhow::{Context, Result};
use console::style;

use crate::registry::{Manifest, ProfileConfig};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".skillsync").join("registry"))
}

/// Find all profiles that reference a given resource name.
fn find_profile_references(manifest: &Manifest, name: &str) -> Vec<String> {
    let registry = match registry_root() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut referencing = Vec::new();

    for (profile_name, profile_ref) in &manifest.profiles {
        let profile_path = registry.join(&profile_ref.path);
        if let Ok(profile) = ProfileConfig::load(&profile_path) {
            let referenced = profile.skills.iter().any(|s| s == name)
                || profile.plugins.iter().any(|p| {
                    p == name || p.starts_with(&format!("{}@", name))
                })
                || profile.mcp.iter().any(|m| m == name);
            if referenced {
                referencing.push(profile_name.clone());
            }
        }
    }

    referencing
}

pub fn run(name: &str) -> Result<()> {
    let registry = registry_root()?;
    let manifest_file = registry.join("manifest.yaml");
    let manifest = Manifest::load(&manifest_file)
        .context("Failed to load manifest. Have you run 'skillsync init'?")?;

    // Look up profiles that reference this resource.
    let profile_refs = find_profile_references(&manifest, name);

    // Check skills.
    if let Some(entry) = manifest.skills.get(name) {
        println!("{}", style(format!("Skill: {}", name)).bold().cyan());
        println!("  Type:        {:?}", entry.skill_type);
        println!("  Scope:       {:?}", entry.scope);
        println!("  Version:     {}", entry.version);
        println!("  Path:        {}", entry.path);
        if let Some(ref desc) = entry.description {
            println!("  Description: {}", desc);
        }
        if !entry.tags.is_empty() {
            println!("  Tags:        {}", entry.tags.join(", "));
        }
        if let Some(ref source) = entry.source {
            println!("  Source:");
            println!("    Marketplace: {}", source.marketplace);
            println!("    Plugin:      {}", source.plugin);
            println!("    Skill:       {}", source.skill);
        }
        if let Some(ref hash) = entry.backup_hash {
            println!("  Hash:        {}", hash);
        }
        if !profile_refs.is_empty() {
            println!("  Profiles:    {}", profile_refs.join(", "));
        }
        return Ok(());
    }

    // Check plugins.
    if let Some(entry) = manifest.plugins.get(name) {
        println!("{}", style(format!("Plugin: {}", name)).bold().cyan());
        println!("  Marketplace: {}", entry.marketplace);
        println!("  Version:     {}", entry.version);
        if let Some(ref sha) = entry.git_sha {
            println!("  Git SHA:     {}", sha);
        }
        if let Some(ref repo) = entry.repo {
            println!("  Repo:        {}", repo);
        }
        if !profile_refs.is_empty() {
            println!("  Profiles:    {}", profile_refs.join(", "));
        }
        return Ok(());
    }

    // Check MCP servers.
    if let Some(entry) = manifest.mcp_servers.get(name) {
        println!("{}", style(format!("MCP Server: {}", name)).bold().cyan());
        println!("  Command:     {}", entry.command);
        if !entry.args.is_empty() {
            println!("  Args:        {}", entry.args.join(" "));
        }
        println!("  Scope:       {:?}", entry.scope);
        if !profile_refs.is_empty() {
            println!("  Profiles:    {}", profile_refs.join(", "));
        }
        return Ok(());
    }

    // Not found — suggest similar names.
    eprintln!(
        "{} Resource '{}' not found in the registry.",
        style("error:").red().bold(),
        name
    );

    // Collect all known names for suggestions.
    let all_names: Vec<&str> = manifest
        .skills
        .keys()
        .chain(manifest.plugins.keys())
        .chain(manifest.mcp_servers.keys())
        .map(|s| s.as_str())
        .collect();

    if !all_names.is_empty() {
        // Simple substring-match suggestion.
        let suggestions: Vec<&&str> = all_names
            .iter()
            .filter(|n| {
                n.contains(name) || name.contains(**n)
            })
            .collect();

        if !suggestions.is_empty() {
            eprintln!("  Did you mean one of these?");
            for s in suggestions {
                eprintln!("    - {}", style(s).cyan());
            }
        } else {
            eprintln!(
                "  Use '{}' to see all registered resources.",
                style("skillsync list").cyan()
            );
        }
    }

    std::process::exit(1);
}
