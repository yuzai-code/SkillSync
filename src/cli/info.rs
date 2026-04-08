use anyhow::{Context, Result};
use console::style;

use crate::i18n::Msg;
#[allow(unused_imports)]
use crate::t;
use crate::registry::{Manifest, ProfileConfig};

/// Resolve the registry root: `~/.skillsync/registry/`
fn registry_root() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;
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
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    // Look up profiles that reference this resource.
    let profile_refs = find_profile_references(&manifest, name);

    // Check skills.
    if let Some(entry) = manifest.skills.get(name) {
        println!("{}", style(t!(Msg::InfoSkillLabel { name: name.to_string() })).bold().cyan());
        println!("  {}  {:?}", t!(Msg::InfoType), entry.skill_type);
        println!("  {}  {:?}", t!(Msg::InfoScope), entry.scope);
        println!("  {}  {}", t!(Msg::InfoVersion), entry.version);
        println!("  {}  {}", t!(Msg::InfoPath), entry.path);
        if let Some(ref desc) = entry.description {
            println!("  {}  {}", t!(Msg::InfoDescription), desc);
        }
        if !entry.tags.is_empty() {
            println!("  {}  {}", t!(Msg::InfoTags), entry.tags.join(", "));
        }
        if let Some(ref source) = entry.source {
            println!("  {}", t!(Msg::InfoSource));
            println!("    {}  {}", t!(Msg::InfoMarketplace), source.marketplace);
            println!("    {}  {}", t!(Msg::InfoPlugin), source.plugin);
            println!("    {}  {}", t!(Msg::InfoSkill), source.skill);
        }
        if let Some(ref hash) = entry.backup_hash {
            println!("  {}  {}", t!(Msg::InfoHash), hash);
        }
        if !profile_refs.is_empty() {
            println!("  {}  {}", t!(Msg::InfoProfiles), profile_refs.join(", "));
        }
        return Ok(());
    }

    // Check plugins.
    if let Some(entry) = manifest.plugins.get(name) {
        println!("{}", style(t!(Msg::InfoPluginLabel { name: name.to_string() })).bold().cyan());
        println!("  {}  {}", t!(Msg::InfoPluginMarketplace), entry.marketplace);
        println!("  {}  {}", t!(Msg::InfoVersion), entry.version);
        if let Some(ref sha) = entry.git_sha {
            println!("  {}  {}", t!(Msg::InfoGitSha), sha);
        }
        if let Some(ref repo) = entry.repo {
            println!("  {}  {}", t!(Msg::InfoRepo), repo);
        }
        if !profile_refs.is_empty() {
            println!("  {}  {}", t!(Msg::InfoProfiles), profile_refs.join(", "));
        }
        return Ok(());
    }

    // Check MCP servers.
    if let Some(entry) = manifest.mcp_servers.get(name) {
        println!("{}", style(t!(Msg::InfoMcpLabel { name: name.to_string() })).bold().cyan());
        println!("  {}  {}", t!(Msg::InfoCommand), entry.command);
        if !entry.args.is_empty() {
            println!("  {}  {}", t!(Msg::InfoArgs), entry.args.join(" "));
        }
        println!("  {}  {:?}", t!(Msg::InfoScope), entry.scope);
        if !profile_refs.is_empty() {
            println!("  {}  {}", t!(Msg::InfoProfiles), profile_refs.join(", "));
        }
        return Ok(());
    }

    // Not found — suggest similar names.
    eprintln!(
        "{} {}",
        style("error:").red().bold(),
        t!(Msg::InfoNotFound { name: name.to_string() })
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
            eprintln!("{}", t!(Msg::InfoDidYouMean));
            for s in suggestions {
                eprintln!("{}", t!(Msg::InfoSuggestion { name: s.to_string() }));
            }
        } else {
            eprintln!(
                "{}",
                t!(Msg::InfoUseListHint { cmd: "skillsync list".to_string() })
            );
        }
    }

    std::process::exit(1);
}
