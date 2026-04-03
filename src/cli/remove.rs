use anyhow::{bail, Context, Result};
use console::style;

use crate::registry::{Manifest, ProfileConfig};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".skillsync").join("registry"))
}

/// Check all profiles for references to the given resource name and return
/// a list of profile names that reference it.
fn find_profile_references(manifest: &Manifest, name: &str) -> Vec<String> {
    let registry = match registry_root() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut referencing_profiles = Vec::new();

    for (profile_name, profile_ref) in &manifest.profiles {
        let profile_path = registry.join(&profile_ref.path);
        if let Ok(profile) = ProfileConfig::load(&profile_path) {
            let referenced = profile.skills.iter().any(|s| s == name)
                || profile.plugins.iter().any(|p| {
                    // Plugin references may be stored as "name" or "name@marketplace"
                    p == name || p.starts_with(&format!("{}@", name))
                })
                || profile.mcp.iter().any(|m| m == name);
            if referenced {
                referencing_profiles.push(profile_name.clone());
            }
        }
    }

    referencing_profiles
}

pub fn run(name: &str) -> Result<()> {
    let registry = registry_root()?;
    let manifest_file = registry.join("manifest.yaml");
    let mut manifest = Manifest::load(&manifest_file)
        .context("Failed to load manifest. Have you run 'skillsync init'?")?;

    // Determine which type of resource this is.
    let is_skill = manifest.skills.contains_key(name);
    let is_plugin = manifest.plugins.contains_key(name);
    let is_mcp = manifest.mcp_servers.contains_key(name);

    if !is_skill && !is_plugin && !is_mcp {
        bail!(
            "Resource '{}' not found in the registry.\n\
             Use 'skillsync list' to see all registered resources.",
            name
        );
    }

    // Check for profile references and warn.
    let refs = find_profile_references(&manifest, name);
    if !refs.is_empty() {
        eprintln!(
            "{} Resource '{}' is referenced by profile(s): {}",
            style("warning:").yellow().bold(),
            name,
            refs.join(", ")
        );
        eprintln!(
            "         You may need to update these profiles after removal."
        );
    }

    // Remove the resource from the manifest.
    let resource_type;
    if is_skill {
        let entry = manifest.skills.remove(name).unwrap();
        resource_type = "skill";

        // Also delete the skill files from registry/resources/skills/<name>/
        let skill_dir = registry.join(&entry.path);
        if skill_dir.exists() {
            std::fs::remove_dir_all(&skill_dir).with_context(|| {
                format!(
                    "Failed to delete skill directory: {}",
                    skill_dir.display()
                )
            })?;
        }
    } else if is_plugin {
        manifest.plugins.remove(name);
        resource_type = "plugin";
    } else {
        manifest.mcp_servers.remove(name);
        resource_type = "MCP server";
    }

    manifest.save(&manifest_file).context("Failed to save manifest")?;

    println!(
        "{} Removed {} '{}' from registry",
        style("✓").green().bold(),
        resource_type,
        style(name).cyan()
    );

    Ok(())
}
