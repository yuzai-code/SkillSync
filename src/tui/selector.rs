// Multi-select / single-select resource selector
// Implements: tasks 4.1, 4.3, 4.4

use std::fmt;

use anyhow::{Context, Result};
use console::style;
use inquire::{Confirm, MultiSelect, Select};

use crate::registry::Manifest;

// ---------------------------------------------------------------------------
// ConfigMethod (4.1)
// ---------------------------------------------------------------------------

/// How the user wants to configure their project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigMethod {
    /// Start from an existing profile and optionally adjust.
    FromProfile,
    /// Manually pick individual resources.
    Manual,
    /// Copy configuration from another project that already has skillsync.yaml.
    CopyFromProject,
}

impl fmt::Display for ConfigMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigMethod::FromProfile => write!(f, "From profile  — start with a predefined bundle"),
            ConfigMethod::Manual => write!(f, "Manual        — pick resources one by one"),
            ConfigMethod::CopyFromProject => {
                write!(f, "Copy project  — reuse another project's config")
            }
        }
    }
}

/// Present a single-select prompt for the configuration method.
pub fn choose_config_method() -> Result<ConfigMethod> {
    let options = vec![
        ConfigMethod::FromProfile,
        ConfigMethod::Manual,
        ConfigMethod::CopyFromProject,
    ];

    let choice = Select::new("How would you like to configure this project?", options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context("Configuration method selection was cancelled")?;

    Ok(choice)
}

// ---------------------------------------------------------------------------
// SelectedResources
// ---------------------------------------------------------------------------

/// The user's final selection of resources to install.
#[derive(Debug, Clone, Default)]
pub struct SelectedResources {
    pub skills: Vec<String>,
    pub plugins: Vec<String>,
    pub mcp: Vec<String>,
}

impl SelectedResources {
    /// Total number of selected resources.
    pub fn total(&self) -> usize {
        self.skills.len() + self.plugins.len() + self.mcp.len()
    }

    /// Returns true if nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

// ---------------------------------------------------------------------------
// ResourceOption — wrapper for multi-select display
// ---------------------------------------------------------------------------

/// A selectable resource item shown in the multi-select list.
#[derive(Debug, Clone)]
struct ResourceOption {
    /// Display label (e.g. "[skill] yuque — Yuque docs helper")
    label: String,
    /// The resource name (key in the manifest).
    name: String,
    /// Category tag for grouping.
    category: ResourceCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceCategory {
    Skill,
    Plugin,
    Mcp,
}

impl fmt::Display for ResourceOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

/// Build the list of selectable options from the manifest, grouped by category.
fn build_resource_options(manifest: &Manifest) -> Vec<ResourceOption> {
    let mut options: Vec<ResourceOption> = Vec::new();

    // Skills
    let mut skill_names: Vec<&String> = manifest.skills.keys().collect();
    skill_names.sort();
    for name in skill_names {
        let entry = &manifest.skills[name];
        let scope_tag = match entry.scope {
            crate::registry::ResourceScope::Global => "global",
            crate::registry::ResourceScope::Shared => "shared",
        };
        let desc = entry
            .description
            .as_deref()
            .unwrap_or("no description");
        let label = format!("[skill]  {} ({}) — {}", name, scope_tag, desc);
        options.push(ResourceOption {
            label,
            name: name.clone(),
            category: ResourceCategory::Skill,
        });
    }

    // Plugins
    let mut plugin_names: Vec<&String> = manifest.plugins.keys().collect();
    plugin_names.sort();
    for name in plugin_names {
        let entry = &manifest.plugins[name];
        let label = format!("[plugin] {} v{} @ {}", name, entry.version, entry.marketplace);
        options.push(ResourceOption {
            label,
            name: name.clone(),
            category: ResourceCategory::Plugin,
        });
    }

    // MCP servers
    let mut mcp_names: Vec<&String> = manifest.mcp_servers.keys().collect();
    mcp_names.sort();
    for name in mcp_names {
        let entry = &manifest.mcp_servers[name];
        let scope_tag = match entry.scope {
            crate::registry::ResourceScope::Global => "global",
            crate::registry::ResourceScope::Shared => "shared",
        };
        let label = format!(
            "[mcp]    {} ({}) — {} {}",
            name,
            scope_tag,
            entry.command,
            entry.args.join(" ")
        );
        options.push(ResourceOption {
            label,
            name: name.clone(),
            category: ResourceCategory::Mcp,
        });
    }

    options
}

// ---------------------------------------------------------------------------
// select_resources (4.3, 4.4)
// ---------------------------------------------------------------------------

/// Present a multi-select prompt for resources from the manifest.
///
/// `pre_selected` contains resource names that should be checked by default
/// (e.g. from a profile or an existing project config).
///
/// inquire's built-in type-to-filter provides the search-as-you-type
/// functionality (task 4.4) with no extra code needed.
pub fn select_resources(
    manifest: &Manifest,
    pre_selected: &[String],
) -> Result<SelectedResources> {
    let options = build_resource_options(manifest);

    if options.is_empty() {
        println!(
            "{}",
            style("Registry is empty — nothing to select.").yellow()
        );
        return Ok(SelectedResources::default());
    }

    // Determine which indices should be pre-selected.
    let defaults: Vec<usize> = options
        .iter()
        .enumerate()
        .filter(|(_, opt)| pre_selected.contains(&opt.name))
        .map(|(i, _)| i)
        .collect();

    let selected = MultiSelect::new("Select resources to install:", options)
        .with_default(&defaults)
        .with_help_message("Type to filter, Space to toggle, Enter to confirm")
        .with_page_size(15)
        .prompt()
        .context("Resource selection was cancelled")?;

    // Split selections back into categories.
    let mut result = SelectedResources::default();
    for item in &selected {
        match item.category {
            ResourceCategory::Skill => result.skills.push(item.name.clone()),
            ResourceCategory::Plugin => result.plugins.push(item.name.clone()),
            ResourceCategory::Mcp => result.mcp.push(item.name.clone()),
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// confirm_preview (4.6)
// ---------------------------------------------------------------------------

/// Display a summary of selected resources and ask for confirmation.
pub fn confirm_preview(selected: &SelectedResources) -> Result<bool> {
    println!();
    println!("{}", style("=== Installation Preview ===").bold().cyan());
    println!();

    if !selected.skills.is_empty() {
        println!("  {} Skills:", style(selected.skills.len()).green().bold());
        for name in &selected.skills {
            println!("    {} {}", style("+").green(), name);
        }
    }

    if !selected.plugins.is_empty() {
        println!(
            "  {} Plugins:",
            style(selected.plugins.len()).green().bold()
        );
        for name in &selected.plugins {
            println!("    {} {}", style("+").green(), name);
        }
    }

    if !selected.mcp.is_empty() {
        println!(
            "  {} MCP servers:",
            style(selected.mcp.len()).green().bold()
        );
        for name in &selected.mcp {
            println!("    {} {}", style("+").green(), name);
        }
    }

    if selected.is_empty() {
        println!(
            "  {}",
            style("No resources selected.").yellow()
        );
        return Ok(false);
    }

    println!();
    println!(
        "  Total: {} resource(s)",
        style(selected.total()).bold()
    );
    println!();

    let confirmed = Confirm::new("Apply these changes?")
        .with_default(true)
        .prompt()
        .context("Confirmation prompt was cancelled")?;

    Ok(confirmed)
}
