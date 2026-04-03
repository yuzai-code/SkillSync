use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use console::style;
use git2::Repository;

use crate::claude::paths::ClaudePaths;
use crate::registry::manifest::Manifest;

/// Return the default registry path: `~/.skillsync/registry/`.
fn registry_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".skillsync").join("registry"))
}

/// `skillsync init` — create a new local registry from scratch.
fn init_new(quiet: bool) -> Result<()> {
    let reg = registry_path()?;

    // Guard: registry must not already exist.
    if reg.exists() {
        bail!(
            "Registry already exists at {}\n\
             Use `skillsync sync` to update, or remove the directory to start fresh.",
            reg.display()
        );
    }

    // Create directory structure.
    let dirs = [
        reg.join("resources").join("skills"),
        reg.join("resources").join("plugins"),
        reg.join("resources").join("mcp"),
        reg.join("profiles"),
    ];
    for dir in &dirs {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    // Write empty manifest.
    let manifest_path = reg.join("manifest.yaml");
    Manifest::default_empty()
        .save(&manifest_path)
        .context("Failed to write empty manifest")?;

    // Initialize git repository.
    Repository::init(&reg)
        .with_context(|| format!("Failed to initialize git repository at {}", reg.display()))?;

    if !quiet {
        println!(
            "{} Initialized new SkillSync registry at {}",
            style("✓").green().bold(),
            style(reg.display().to_string()).cyan()
        );
    }

    Ok(())
}

/// `skillsync init --from <url>` — clone a remote registry and validate it.
fn init_from(url: &str, quiet: bool) -> Result<()> {
    let reg = registry_path()?;

    // Guard: registry must not already exist.
    if reg.exists() {
        bail!(
            "Registry already exists at {}\n\
             Use `skillsync sync` to update, or remove the directory to start fresh.",
            reg.display()
        );
    }

    // Ensure parent directory exists.
    if let Some(parent) = reg.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Clone remote repository.
    Repository::clone(url, &reg)
        .with_context(|| format!("Failed to clone registry from {}", url))?;

    // Validate that manifest.yaml exists and is parseable.
    let manifest_path = reg.join("manifest.yaml");
    if !manifest_path.exists() {
        bail!(
            "Cloned repository does not contain a manifest.yaml — \
             this may not be a valid SkillSync registry."
        );
    }

    let manifest = Manifest::load(&manifest_path)
        .context("Failed to parse manifest.yaml from cloned registry")?;

    if let Err(errors) = manifest.validate() {
        eprintln!(
            "{} manifest.yaml has validation warnings:",
            style("⚠").yellow().bold()
        );
        for err in &errors {
            eprintln!("  - {}", err);
        }
    }

    if !quiet {
        println!(
            "{} Cloned SkillSync registry from {}",
            style("✓").green().bold(),
            style(url).cyan()
        );
        println!(
            "  {} skill(s), {} plugin(s), {} MCP server(s), {} profile(s)",
            manifest.skills.len(),
            manifest.plugins.len(),
            manifest.mcp_servers.len(),
            manifest.profiles.len(),
        );
    }

    Ok(())
}

pub fn run(from: Option<String>, quiet: bool) -> Result<()> {
    match from {
        Some(url) => init_from(&url, quiet),
        None => init_new(quiet),
    }
}

// ---------------------------------------------------------------------------
// 3.3 — Auto-import discovery
// ---------------------------------------------------------------------------

/// A discovered resource from the user's existing Claude Code installation.
#[derive(Debug, Clone)]
pub struct DiscoveredResource {
    /// Resource name (directory name or MCP server name).
    pub name: String,
    /// The kind of resource.
    pub kind: DiscoveredKind,
    /// Path on disk (for skills) or description (for MCP servers).
    pub detail: String,
}

/// What kind of resource was discovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveredKind {
    Skill,
    McpServer,
}

impl std::fmt::Display for DiscoveredKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveredKind::Skill => write!(f, "skill"),
            DiscoveredKind::McpServer => write!(f, "mcp_server"),
        }
    }
}

/// Scan the user's `~/.claude/skills/` directory for existing skill directories.
///
/// Returns a list of discovered skills (each subdirectory of `~/.claude/skills/`
/// is treated as one skill).
pub fn scan_existing_skills() -> Result<Vec<DiscoveredResource>> {
    let claude = ClaudePaths::global()?;
    let mut results = Vec::new();

    if !claude.skills_dir.is_dir() {
        return Ok(results);
    }

    let entries = fs::read_dir(&claude.skills_dir)
        .with_context(|| format!("Failed to read {}", claude.skills_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        // Only consider directories as skills.
        if path.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                results.push(DiscoveredResource {
                    name: name.to_string(),
                    kind: DiscoveredKind::Skill,
                    detail: path.display().to_string(),
                });
            }
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

/// Scan the user's `~/.claude/.mcp.json` for existing MCP server definitions.
///
/// Returns a list of discovered MCP servers (each key in `mcpServers`).
pub fn scan_existing_mcp_servers() -> Result<Vec<DiscoveredResource>> {
    let claude = ClaudePaths::global()?;
    let mut results = Vec::new();

    if !claude.mcp_json.exists() {
        return Ok(results);
    }

    let contents = fs::read_to_string(&claude.mcp_json)
        .with_context(|| format!("Failed to read {}", claude.mcp_json.display()))?;

    let root: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse {}", claude.mcp_json.display()))?;

    if let Some(servers) = root.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, value) in servers {
            let command = value
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("(unknown)");
            results.push(DiscoveredResource {
                name: name.clone(),
                kind: DiscoveredKind::McpServer,
                detail: format!("command: {}", command),
            });
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(results)
}

/// Scan all existing Claude Code resources and return a combined list.
///
/// This is the main entry point for task 3.3. Call this from `init` (or
/// anywhere else) to discover what the user already has installed.
pub fn discover_existing_resources() -> Result<Vec<DiscoveredResource>> {
    let mut all = Vec::new();

    match scan_existing_skills() {
        Ok(skills) => all.extend(skills),
        Err(e) => eprintln!(
            "  {} Could not scan skills: {}",
            style("⚠").yellow(),
            e
        ),
    }

    match scan_existing_mcp_servers() {
        Ok(mcp) => all.extend(mcp),
        Err(e) => eprintln!(
            "  {} Could not scan MCP servers: {}",
            style("⚠").yellow(),
            e
        ),
    }

    Ok(all)
}

/// Print a summary of discovered resources (non-interactive reporting).
///
/// Returns the list so callers can decide what to do with it.
pub fn report_discovered_resources() -> Result<Vec<DiscoveredResource>> {
    let resources = discover_existing_resources()?;

    if resources.is_empty() {
        println!(
            "  {} No existing Claude Code resources found to import.",
            style("·").dim()
        );
        return Ok(resources);
    }

    println!(
        "\n{} Found {} existing resource(s) that could be imported:",
        style("ℹ").blue().bold(),
        resources.len()
    );

    for res in &resources {
        println!(
            "  {} [{}] {} — {}",
            style("·").dim(),
            style(res.kind.to_string()).dim(),
            style(&res.name).cyan(),
            style(&res.detail).dim()
        );
    }

    println!(
        "\n  Use '{}' to add these to the registry.",
        style("skillsync add <path>").cyan()
    );

    Ok(resources)
}
