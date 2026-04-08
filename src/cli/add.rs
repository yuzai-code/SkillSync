use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::i18n::Msg;
use crate::registry::{
    compute_hash, copy_resource, Manifest, McpServerEntry, PluginEntry, ResourceScope, SkillEntry,
    SkillType,
};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context(t!(Msg::ContextHomeDir))?;
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
        other => bail!("{}", t!(Msg::AddInvalidScope { scope: other.to_string() })),
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
        bail!("{}", t!(Msg::AddPathNotExist { path: source.display().to_string() }));
    }

    let name = source
        .file_name()
        .context(t!(Msg::ContextCurrentDir))?
        .to_string_lossy()
        .to_string();

    let registry = registry_root()?;
    let manifest_file = registry.join("manifest.yaml");
    let mut manifest = Manifest::load(&manifest_file)
        .context(t!(Msg::ContextFailedToLoadManifest))?;

    if manifest.skills.contains_key(&name) {
        bail!("{}", t!(Msg::AddSkillAlreadyExists { name: name.clone() }));
    }

    // Copy the skill into the registry resources directory.
    let dest = registry.join("resources").join("skills").join(&name);
    copy_resource(source, &dest)
        .with_context(|| t!(Msg::ContextCreateDir { path: dest.display().to_string() }))?;

    // Compute content hash over the copied resource.
    let hash = compute_hash(&dest)
        .with_context(|| t!(Msg::ContextCreateDir { path: dest.display().to_string() }))?;

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
    manifest.save(&manifest_file).context(t!(Msg::ContextFailedToSaveManifest))?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::AddSkillSuccess { name: name.clone() })
    );
    Ok(())
}

/// Add a plugin from a `name@marketplace` specifier.
fn add_plugin(spec: &str, _scope: ResourceScope) -> Result<()> {
    let parts: Vec<&str> = spec.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!("{}", t!(Msg::AddInvalidPluginFormat { input: spec.to_string() }));
    }
    let name = parts[0].to_string();
    let marketplace = parts[1].to_string();

    let manifest_file = manifest_path()?;
    let mut manifest = Manifest::load(&manifest_file)
        .context(t!(Msg::ContextFailedToLoadManifest))?;

    if manifest.plugins.contains_key(&name) {
        bail!("{}", t!(Msg::AddPluginAlreadyExists { name: name.clone() }));
    }

    let entry = PluginEntry {
        marketplace,
        version: "latest".into(),
        git_sha: None,
        repo: None,
    };

    manifest.plugins.insert(name.clone(), entry);
    manifest.save(&manifest_file).context(t!(Msg::ContextFailedToSaveManifest))?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::AddPluginSuccess { name: name.clone() })
    );
    Ok(())
}

/// Add an MCP server configuration.
fn add_mcp(name: &str, command: Option<String>, args: Option<Vec<String>>, scope: ResourceScope) -> Result<()> {
    let command = command.context(t!(Msg::AddMcpRequiresCommand))?;

    let manifest_file = manifest_path()?;
    let mut manifest = Manifest::load(&manifest_file)
        .context(t!(Msg::ContextFailedToLoadManifest))?;

    if manifest.mcp_servers.contains_key(name) {
        bail!("{}", t!(Msg::AddMcpAlreadyExists { name: name.to_string() }));
    }

    let entry = McpServerEntry {
        command,
        args: args.unwrap_or_default(),
        scope,
    };

    manifest.mcp_servers.insert(name.to_string(), entry);
    manifest.save(&manifest_file).context(t!(Msg::ContextFailedToSaveManifest))?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::AddMcpSuccess { name: name.to_string() })
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
        bail!("{}", t!(Msg::AddNothingToAdd));
    }
}
