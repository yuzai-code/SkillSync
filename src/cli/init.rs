use std::fs;

use anyhow::{bail, Context, Result};
use console::style;
use git2::Repository;
use inquire::Select;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::{ClaudePaths, SkillSyncPaths};
use crate::i18n::{Lang, Msg};
use crate::registry::manifest::Manifest;

/// `skillsync init` — create a new local registry from scratch.
fn init_new(quiet: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let registry_git = &ss_paths.registry_git;
    let registry = &ss_paths.registry;

    // Guard: neither registry.git nor registry must already exist.
    if registry_git.exists() || registry.exists() {
        bail!(
            "Registry already exists at {}\n\
             Use `skillsync sync` to update, or remove the directory to start fresh.",
            registry.display()
        );
    }

    // Prompt language selection if SKILLSYNC_LANG is not set.
    let lang = if std::env::var_os("SKILLSYNC_LANG").is_none() {
        let lang = prompt_language_selection()?;
        lang.save_preference()?;
        lang
    } else {
        crate::i18n::lang()
    };

    // Create bare git repo at registry.git/.
    fs::create_dir_all(registry_git)
        .with_context(|| t!(Msg::ContextCreateDir { path: registry_git.display().to_string() }))?;
    let _bare = Repository::init_bare(registry_git)
        .with_context(|| t!(Msg::ContextFailedToOpenRepo))?;

    // Create working tree repo at registry/ with origin pointing to registry.git/.
    fs::create_dir_all(registry)
        .with_context(|| t!(Msg::ContextCreateDir { path: registry.display().to_string() }))?;
    let working_repo = Repository::init(registry)
        .with_context(|| t!(Msg::ContextFailedToOpenRepo))?;
    working_repo
        .remote("origin", registry_git.to_string_lossy().as_ref())
        .with_context(|| "Failed to add origin remote")?;

    // Create directory structure inside the working tree.
    let dirs = [
        registry.join("resources").join("skills"),
        registry.join("resources").join("plugins"),
        registry.join("resources").join("mcp"),
        registry.join("profiles"),
    ];
    for dir in &dirs {
        fs::create_dir_all(dir)
            .with_context(|| t!(Msg::ContextCreateDir { path: dir.display().to_string() }))?;
    }

    // Write empty manifest.
    let manifest_path = registry.join("manifest.yaml");
    let mut manifest = Manifest::default_empty();

    // Scan ~/projects/*/.claude/skills/ for existing skills and add to manifest.
    match crate::registry::discover::scan_projects_skills() {
        Ok(discovered) if !discovered.is_empty() => {
            crate::registry::discover::register_discovered_skills(&mut manifest, &discovered);
            if !quiet {
                println!(
                    "  {} {}",
                    style("·").dim(),
                    t!(Msg::InitScannedProjects {
                        projects: discovered.len(),
                        skills: discovered.iter().filter(|s| s.project_path.to_string_lossy().contains(".claude/skills")).count()
                    })
                );
            }
        }
        Ok(_) => {}
        Err(e) => {
            if !quiet {
                eprintln!(
                    "{} {}",
                    style("⚠").yellow(),
                    t!(Msg::InitScanProjectsError { error: e.to_string() })
                );
            }
        }
    }

    manifest
        .save(&manifest_path)
        .context(t!(Msg::ContextFailedToSaveManifest))?;

    // Create initial commit in the working tree.
    {
        use git2::IndexAddOption;
        let mut index = working_repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_oid = index.write_tree()?;
        let tree = working_repo.find_tree(tree_oid)?;
        let sig = working_repo.signature()?;
        working_repo.commit(Some("HEAD"), &sig, &sig, "chore: initial registry", &tree, &[])?;
    }

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::InitSuccess { path: registry.display().to_string() })
        );
        println!(
            "  {}",
            t!(Msg::InitLanguageSet { lang: lang.tag().to_string() })
        );
        println!(
            "  {} {}",
            style("·").dim(),
            t!(Msg::InitRegistryGit { path: registry_git.display().to_string() })
        );
    }

    Ok(())
}

/// Prompt the user to select their preferred language.
fn prompt_language_selection() -> Result<Lang> {
    let prompt = t!(Msg::InitLanguageSelect);
    let options = ["English", "中文"];
    let selection = Select::new(&prompt, options.to_vec()).prompt()?;
    match selection {
        "English" => Ok(Lang::En),
        "中文" => Ok(Lang::Zh),
        _ => Ok(Lang::En),
    }
}

/// `skillsync init --from <url>` — clone a remote registry and validate it.
fn init_from(url: &str, quiet: bool) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let registry_git = &ss_paths.registry_git;
    let registry = &ss_paths.registry;

    // Guard: neither registry.git nor registry must already exist.
    if registry_git.exists() || registry.exists() {
        bail!(
            "Registry already exists.\n\
             Use `skillsync sync` to update, or remove the directory to start fresh."
        );
    }

    // Ensure parent directory exists.
    if let Some(parent) = registry.parent() {
        fs::create_dir_all(parent)
            .with_context(|| t!(Msg::ContextCreateDir { path: parent.display().to_string() }))?;
    }

    // Clone remote repository to registry/ (regular working tree).
    let _working_repo = Repository::clone(url, registry)
        .with_context(|| t!(Msg::ContextFailedToOpenRepo))?;

    // Create registry.git/ as a bare repo for local sync.
    fs::create_dir_all(registry_git)
        .with_context(|| t!(Msg::ContextCreateDir { path: registry_git.display().to_string() }))?;
    let _bare = Repository::init_bare(registry_git)
        .with_context(|| "Failed to init bare registry.git")?;

    // Add registry.git/ as a "backup" remote in the working repo.
    {
        let working = Repository::open(registry)?;
        working
            .remote("backup", registry_git.to_string_lossy().as_ref())
            .with_context(|| "Failed to add backup remote")?;
    }

    // Validate that manifest.yaml exists and is parseable.
    let manifest_path = registry.join("manifest.yaml");
    if !manifest_path.exists() {
        bail!("{}", t!(Msg::ContextFailedToLoadManifest));
    }

    let manifest = Manifest::load(&manifest_path)
        .context(t!(Msg::ContextFailedToLoadManifest))?;

    if let Err(errors) = manifest.validate() {
        eprintln!(
            "{} {}",
            style("⚠").yellow().bold(),
            t!(Msg::DoctorValidationIssues { count: errors.len() })
        );
        for err in &errors {
            eprintln!("{}", t!(Msg::DoctorValidationError { error: err.to_string() }));
        }
    }

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::InitCloned { url: url.to_string() })
        );
        println!(
            "  {}",
            t!(Msg::InitScanResult {
                skills: manifest.skills.len(),
                plugins: manifest.plugins.len(),
                mcp: manifest.mcp_servers.len(),
                profiles: manifest.profiles.len()
            })
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn scan_existing_skills() -> Result<Vec<DiscoveredResource>> {
    let claude = ClaudePaths::global()?;
    let mut results = Vec::new();

    if !claude.skills_dir.is_dir() {
        return Ok(results);
    }

    let entries = fs::read_dir(&claude.skills_dir)
        .with_context(|| t!(Msg::ContextReadDir { path: claude.skills_dir.display().to_string() }))?;

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
#[allow(dead_code)]
pub fn scan_existing_mcp_servers() -> Result<Vec<DiscoveredResource>> {
    let claude = ClaudePaths::global()?;
    let mut results = Vec::new();

    if !claude.mcp_json.exists() {
        return Ok(results);
    }

    let contents = fs::read_to_string(&claude.mcp_json)
        .with_context(|| t!(Msg::ContextReadDir { path: claude.mcp_json.display().to_string() }))?;

    let root: serde_json::Value = serde_json::from_str(&contents)
        .with_context(|| t!(Msg::ContextReadDir { path: claude.mcp_json.display().to_string() }))?;

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
#[allow(dead_code)]
pub fn discover_existing_resources() -> Result<Vec<DiscoveredResource>> {
    let mut all = Vec::new();

    match scan_existing_skills() {
        Ok(skills) => all.extend(skills),
        Err(e) => eprintln!(
            "{} {}",
            style("⚠").yellow(),
            t!(Msg::InitScanSkillsError { error: e.to_string() })
        ),
    }

    match scan_existing_mcp_servers() {
        Ok(mcp) => all.extend(mcp),
        Err(e) => eprintln!(
            "{} {}",
            style("⚠").yellow(),
            t!(Msg::InitScanMcpError { error: e.to_string() })
        ),
    }

    Ok(all)
}

/// Print a summary of discovered resources (non-interactive reporting).
///
/// Returns the list so callers can decide what to do with it.
#[allow(dead_code)]
pub fn report_discovered_resources() -> Result<Vec<DiscoveredResource>> {
    let resources = discover_existing_resources()?;

    if resources.is_empty() {
        println!("  {} {}", style("·").dim(), t!(Msg::InitNoResourcesFound));
        return Ok(resources);
    }

    println!(
        "\n{} {}",
        style("ℹ").blue().bold(),
        t!(Msg::InitFoundResources { count: resources.len() })
    );

    for res in &resources {
        println!(
            "  {} {}",
            style("·").dim(),
            t!(Msg::InitResourceItem {
                kind: res.kind.to_string(),
                name: res.name.clone(),
                detail: res.detail.clone()
            })
        );
    }

    println!(
        "\n  {}",
        t!(Msg::InitAddHint { cmd: "skillsync add <path>".to_string() })
    );

    Ok(resources)
}
