// Profile selection UI
// Implements: task 4.2

use std::fmt;
use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;
use inquire::Select;

use crate::registry::{Manifest, ProfileConfig};

// ---------------------------------------------------------------------------
// ProfileOption — display wrapper for Select prompt
// ---------------------------------------------------------------------------

/// A profile entry rendered for the selection list.
#[derive(Debug, Clone)]
struct ProfileOption {
    /// Profile key in the manifest (used as return value).
    name: String,
    /// Human-readable description from the profile YAML.
    description: String,
    /// Number of skills declared in the profile.
    skill_count: usize,
    /// Number of plugins declared in the profile.
    plugin_count: usize,
    /// Number of MCP servers declared in the profile.
    mcp_count: usize,
}

impl fmt::Display for ProfileOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let counts = format!(
            "{} skills, {} plugins, {} mcp",
            self.skill_count, self.plugin_count, self.mcp_count
        );
        if self.description.is_empty() {
            write!(f, "{}  ({})", self.name, counts)
        } else {
            write!(f, "{}  — {}  ({})", self.name, self.description, counts)
        }
    }
}

// ---------------------------------------------------------------------------
// pick_profile (4.2)
// ---------------------------------------------------------------------------

/// Present a selection prompt listing all profiles from the manifest.
///
/// Each profile's YAML file is loaded from `<registry_root>/<profile.path>`
/// so we can show the description and resource counts.
///
/// Returns the profile name (key in the manifest).
pub fn pick_profile(manifest: &Manifest, registry_root: &Path) -> Result<String> {
    if manifest.profiles.is_empty() {
        bail!(
            "No profiles found in the registry. \
             Create one with `skillsync profile create <name>`."
        );
    }

    let mut options: Vec<ProfileOption> = Vec::new();

    // Sort profile names for deterministic ordering.
    let mut profile_names: Vec<&String> = manifest.profiles.keys().collect();
    profile_names.sort();

    for name in &profile_names {
        let profile_ref = &manifest.profiles[*name];
        let profile_path = registry_root.join(&profile_ref.path);

        // Try to load the profile YAML for display info.
        // If the file is missing or unparseable, show a fallback.
        let (description, skill_count, plugin_count, mcp_count) =
            match ProfileConfig::load(&profile_path) {
                Ok(cfg) => (
                    cfg.description.unwrap_or_default(),
                    cfg.skills.len(),
                    cfg.plugins.len(),
                    cfg.mcp.len(),
                ),
                Err(_) => {
                    eprintln!(
                        "  {} could not load profile YAML: {}",
                        style("warning:").yellow(),
                        profile_path.display()
                    );
                    (String::new(), 0, 0, 0)
                }
            };

        options.push(ProfileOption {
            name: (*name).clone(),
            description,
            skill_count,
            plugin_count,
            mcp_count,
        });
    }

    let selected = Select::new("Select a profile:", options)
        .with_help_message("Profiles bundle skills, plugins, and MCP servers together")
        .prompt()
        .context("Profile selection was cancelled")?;

    Ok(selected.name)
}
