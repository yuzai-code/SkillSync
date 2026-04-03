use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;

use crate::registry::{
    compute_hash, copy_resource, Manifest, McpServerEntry, PluginEntry, ResourceScope, SkillEntry,
    SkillType,
};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".skillsync").join("registry"))
}

/// Resolve the manifest path: `~/.skillsync/registry/manifest.yaml`
fn manifest_path() -> Result<std::path::PathBuf> {
    Ok(registry_root()?.join("manifest.yaml"))
}

/// Parse a scope string ("global" or "shared") into `ResourceScope`.
fn parse_scope(scope: &str) -> Result<ResourceScope> {
    match scope.to_lowercase().as_str() {
        "global" => Ok(ResourceScope::Global),
        "shared" => Ok(ResourceScope::Shared),
        other => bail!(
            "Invalid scope '{}'. Must be 'global' or 'shared'.",
            other
        ),
    }
}

/// Add a skill from a local path.
///
/// - Infers the name from the directory/file name
/// - Copies into `registry/resources/skills/<name>/`
/// - Computes a SHA-256 hash
/// - Creates a `SkillEntry` and saves the manifest
fn add_skill(path_str: &str, scope: ResourceScope) -> Result<()> {
    let source = Path::new(path_str);
    if !source.exists() {
        bail!(
            "Path '{}' does not exist. Check the path and try again.",
            source.display()
        );
    }

    let name = source
        .file_name()
        .context("Cannot infer skill name from path")?
        .to_string_lossy()
        .to_string();

    let registry = registry_root()?;
    let manifest_file = registry.join("manifest.yaml");
    let mut manifest = Manifest::load(&manifest_file)
        .context("Failed to load manifest. Have you run 'skillsync init'?")?;

    if manifest.skills.contains_key(&name) {
        bail!(
            "Skill '{}' already exists in the registry. Remove it first or choose a different name.",
            name
        );
    }

    // Copy the skill into the registry resources directory.
    let dest = registry.join("resources").join("skills").join(&name);
    copy_resource(source, &dest)
        .with_context(|| format!("Failed to copy skill to {}", dest.display()))?;

    // Compute content hash over the copied resource.
    let hash = compute_hash(&dest)
        .with_context(|| format!("Failed to compute hash for {}", dest.display()))?;

    let relative_path = format!("resources/skills/{}", name);

    let entry = SkillEntry {
        skill_type: SkillType::Custom,
        scope,
        version: "0.1.0".into(),
        path: relative_path,
        description: None,
        tags: vec![],
        source: None,
        backup_hash: Some(hash),
    };

    manifest.skills.insert(name.clone(), entry);
    manifest.save(&manifest_file).context("Failed to save manifest")?;

    println!(
        "{} Added skill '{}' to registry",
        style("✓").green().bold(),
        style(&name).cyan()
    );
    Ok(())
}

/// Add a plugin from a `name@marketplace` specifier.
fn add_plugin(spec: &str, _scope: ResourceScope) -> Result<()> {
    let parts: Vec<&str> = spec.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!(
            "Invalid plugin format '{}'. Expected 'name@marketplace' (e.g. superpowers@claude-plugins-official).",
            spec
        );
    }
    let name = parts[0].to_string();
    let marketplace = parts[1].to_string();

    let manifest_file = manifest_path()?;
    let mut manifest = Manifest::load(&manifest_file)
        .context("Failed to load manifest. Have you run 'skillsync init'?")?;

    if manifest.plugins.contains_key(&name) {
        bail!(
            "Plugin '{}' already exists in the registry. Remove it first or choose a different name.",
            name
        );
    }

    let entry = PluginEntry {
        marketplace,
        version: "latest".into(),
        git_sha: None,
        repo: None,
    };

    manifest.plugins.insert(name.clone(), entry);
    manifest.save(&manifest_file).context("Failed to save manifest")?;

    println!(
        "{} Added plugin '{}' to registry",
        style("✓").green().bold(),
        style(&name).cyan()
    );
    Ok(())
}

/// Add an MCP server configuration.
fn add_mcp(name: &str, command: Option<String>, args: Option<Vec<String>>, scope: ResourceScope) -> Result<()> {
    let command = command.context(
        "MCP server requires --command. Usage: skillsync add --mcp <name> --command <cmd> [--args <args>...]",
    )?;

    let manifest_file = manifest_path()?;
    let mut manifest = Manifest::load(&manifest_file)
        .context("Failed to load manifest. Have you run 'skillsync init'?")?;

    if manifest.mcp_servers.contains_key(name) {
        bail!(
            "MCP server '{}' already exists in the registry. Remove it first or choose a different name.",
            name
        );
    }

    let entry = McpServerEntry {
        command,
        args: args.unwrap_or_default(),
        scope,
    };

    manifest.mcp_servers.insert(name.to_string(), entry);
    manifest.save(&manifest_file).context("Failed to save manifest")?;

    println!(
        "{} Added MCP server '{}' to registry",
        style("✓").green().bold(),
        style(name).cyan()
    );
    Ok(())
}

pub fn run(
    path: Option<String>,
    plugin: Option<String>,
    mcp: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    scope: String,
) -> Result<()> {
    let scope_enum = parse_scope(&scope)?;

    if let Some(ref p) = path {
        add_skill(p, scope_enum)
    } else if let Some(ref spec) = plugin {
        add_plugin(spec, scope_enum)
    } else if let Some(ref name) = mcp {
        add_mcp(name, command, args, scope_enum)
    } else {
        bail!(
            "Nothing to add. Provide a path, --plugin, or --mcp.\n\
             Usage:\n  \
               skillsync add <path>                              # add a skill\n  \
               skillsync add --plugin name@marketplace           # add a plugin\n  \
               skillsync add --mcp <name> --command <cmd>        # add an MCP server"
        );
    }
}
