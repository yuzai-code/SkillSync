// Multi-select / single-select resource selector
// Implements: tasks 4.1, 4.3, 4.4, 6.1-6.4

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use console::style;
use inquire::{Confirm, MultiSelect, Select};

use crate::i18n::Msg;
use crate::registry::Manifest;

#[allow(unused_imports)]
use crate::t;

/// Extract source location from source_path.
/// Returns "global" for ~/.claude/skills/ or "project: <name>" for project skills.
fn extract_source(source_path: Option<&str>) -> String {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return "unknown".to_string(),
    };

    let path = match source_path {
        Some(p) => PathBuf::from(p),
        None => return "unknown".to_string(),
    };

    // Check if it's a global skill (~/.claude/skills/<name>)
    let global_skills_prefix = home.join(".claude").join("skills");
    if path.starts_with(&global_skills_prefix) {
        return "global".to_string();
    }

    // Extract project name from path
    // Path format: ~/.../<project>/.claude/skills/<name>
    // Walk up to find .claude directory, then get parent (project name)
    let mut current = path.as_path();
    while let Some(parent) = current.parent() {
        if parent.file_name() == Some(std::ffi::OsStr::new(".claude")) {
            if let Some(project_path) = parent.parent() {
                if let Some(project_name) = project_path.file_name().and_then(|n| n.to_str()) {
                    return format!("project: {}", project_name);
                }
            }
            break;
        }
        current = parent;
    }

    "unknown".to_string()
}

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
            ConfigMethod::FromProfile => write!(f, "{}", t!(Msg::SelectorFromProfile)),
            ConfigMethod::Manual => write!(f, "{}", t!(Msg::SelectorManual)),
            ConfigMethod::CopyFromProject => write!(f, "{}", t!(Msg::SelectorCopyProject)),
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

    let prompt = t!(Msg::SelectorConfigurePrompt);
    let choice = Select::new(&prompt, options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context(t!(Msg::SelectorConfigureCancelled))?;

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
        let source = extract_source(entry.source_path.as_deref());
        let desc = entry
            .description
            .as_deref()
            .unwrap_or("n/a");
        let label = format!("[skill]  {} ({}) [{}] — {}", name, scope_tag, style(&source).dim(), desc);
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
        let label = format!("[plugin] {} v{} @ {} [marketplace]", name, entry.version, entry.marketplace);
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
            "[mcp]    {} ({}) [config] — {} {}",
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
        println!("{}", style(t!(Msg::SelectorEmpty)).yellow());
        return Ok(SelectedResources::default());
    }

    // Determine which indices should be pre-selected.
    let defaults: Vec<usize> = options
        .iter()
        .enumerate()
        .filter(|(_, opt)| pre_selected.contains(&opt.name))
        .map(|(i, _)| i)
        .collect();

    let resources_prompt = t!(Msg::SelectorResourcesPrompt);
    let selected = MultiSelect::new(&resources_prompt, options)
        .with_default(&defaults)
        .with_help_message("Type to filter, Space to toggle, Enter to confirm")
        .with_page_size(15)
        .prompt()
        .with_context(|| t!(Msg::SelectorResourcesCancelled))?;

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
pub fn confirm_preview(selected: &SelectedResources, manifest: &Manifest) -> Result<bool> {
    println!();
    println!("{}", style(t!(Msg::SelectorPreviewHeader)).bold().cyan());
    println!();

    if !selected.skills.is_empty() {
        println!("  {} {}", style(selected.skills.len()).green().bold(), t!(Msg::SelectorSkillsLabel));
        for name in &selected.skills {
            let source = manifest.skills.get(name)
                .map(|e| extract_source(e.source_path.as_deref()))
                .unwrap_or_else(|| "unknown".to_string());
            let source_display = if source == "global" {
                style(&source).green().to_string()
            } else {
                style(&source).cyan().to_string()
            };
            println!("    {} {} [{}]", style("+").green(), name, source_display);
        }
    }

    if !selected.plugins.is_empty() {
        println!(
            "  {} {}",
            style(selected.plugins.len()).green().bold(),
            t!(Msg::SelectorPluginsLabel)
        );
        for name in &selected.plugins {
            let marketplace = manifest.plugins.get(name)
                .map(|e| e.marketplace.clone())
                .unwrap_or_else(|| "unknown".to_string());
            println!("    {} {} [{}]", style("+").green(), name, style(&marketplace).dim());
        }
    }

    if !selected.mcp.is_empty() {
        println!(
            "  {} {}",
            style(selected.mcp.len()).green().bold(),
            t!(Msg::SelectorMcpLabel)
        );
        for name in &selected.mcp {
            let command = manifest.mcp_servers.get(name)
                .map(|e| e.command.clone())
                .unwrap_or_else(|| "unknown".to_string());
            println!("    {} {} [{}]", style("+").green(), name, style(&command).dim());
        }
    }

    if selected.is_empty() {
        println!("  {}", style(t!(Msg::SelectorNoResources)).yellow());
        return Ok(false);
    }

    println!();
    println!(
        "  {}",
        t!(Msg::SelectorTotal { count: selected.total() })
    );
    println!();

    let apply_prompt = t!(Msg::SelectorApplyPrompt);
    let confirmed = Confirm::new(&apply_prompt)
        .with_default(true)
        .prompt()
        .with_context(|| t!(Msg::SelectorConfirmCancelled))?;

    Ok(confirmed)
}

// ---------------------------------------------------------------------------
// Remote Skills Selection (6.1-6.4)
// ---------------------------------------------------------------------------

/// A skill entry from remote registry, ready for selection.
#[derive(Debug, Clone)]
pub struct RemoteSkillItem {
    /// Skill name.
    pub name: String,
    /// Source project path (where the skill was discovered from).
    pub source_project: String,
    /// Description if available.
    pub description: Option<String>,
}

impl fmt::Display for RemoteSkillItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            t!(Msg::SelectorRemoteSkillItem {
                name: self.name.clone(),
                source: self.source_project.clone()
            })
        )
    }
}

/// Present a multi-select prompt for remote skills.
///
/// Returns the selected skill names, or an empty vector if cancelled or none selected.
pub fn select_remote_skills(skills: &[RemoteSkillItem]) -> Result<Vec<String>> {
    if skills.is_empty() {
        println!("{}", style(t!(Msg::SelectorRemoteSkillsEmpty)).yellow());
        return Ok(Vec::new());
    }

    let prompt = t!(Msg::SelectorRemoteSkillsPrompt);
    let selected = MultiSelect::new(&prompt, skills.to_vec())
        .with_help_message("Type to filter, Space to toggle, Enter to confirm")
        .with_page_size(15)
        .prompt()
        .with_context(|| t!(Msg::SelectorRemoteSkillsCancelled))?;

    Ok(selected.iter().map(|s| s.name.clone()).collect())
}

// ---------------------------------------------------------------------------
// Install Scope Selection (6.2)
// ---------------------------------------------------------------------------

/// Where to install selected skills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallScope {
    /// Install globally to ~/.claude/skills/
    Global,
    /// Install to a specific project.
    Project { path: String },
}

impl fmt::Display for InstallScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallScope::Global => write!(f, "{}", t!(Msg::SelectorInstallScopeGlobal)),
            InstallScope::Project { .. } => write!(f, "{}", t!(Msg::SelectorInstallScopeProject)),
        }
    }
}

/// Present a single-select prompt for install scope.
pub fn select_install_scope() -> Result<InstallScope> {
    let options = vec![
        InstallScope::Global,
        InstallScope::Project { path: String::new() },
    ];

    let prompt = t!(Msg::SelectorInstallScopePrompt);
    let choice = Select::new(&prompt, options)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context(t!(Msg::SelectorInstallScopeCancelled))?;

    Ok(choice)
}

// ---------------------------------------------------------------------------
// Project Picker (6.3)
// ---------------------------------------------------------------------------

/// A project entry for the project picker.
#[derive(Debug, Clone)]
pub struct ProjectItem {
    /// Project name (directory name).
    pub name: String,
    /// Full path to the project.
    pub path: String,
}

impl fmt::Display for ProjectItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.path)
    }
}

/// Present a single-select prompt for target project.
pub fn select_target_project(projects: &[ProjectItem]) -> Result<String> {
    if projects.is_empty() {
        println!("{}", style(t!(Msg::SelectorProjectPickerEmpty)).yellow());
        return Ok(String::new());
    }

    let prompt = t!(Msg::SelectorProjectPickerPrompt);
    let choice = Select::new(&prompt, projects.to_vec())
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context(t!(Msg::SelectorProjectPickerCancelled))?;

    Ok(choice.path)
}

// ---------------------------------------------------------------------------
// Dry-Run Preview (6.4)
// ---------------------------------------------------------------------------

/// Display a dry-run preview of what would be installed.
pub fn show_dry_run_preview(skills: &[String], target: &str) -> Result<()> {
    println!();
    println!("{}", style(t!(Msg::SelectorDryRunHeader)).bold().cyan());
    println!();

    if skills.is_empty() {
        println!("  {}", style(t!(Msg::SelectorDryRunEmpty)).yellow());
        return Ok(());
    }

    for name in skills {
        println!(
            "{}",
            t!(Msg::SelectorDryRunSkill {
                name: name.clone(),
                target: target.to_string()
            })
        );
    }

    println!();
    Ok(())
}
