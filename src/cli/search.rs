use anyhow::{Context, Result};

use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
#[allow(unused_imports)]
use crate::t;
use crate::registry::manifest::Manifest;

pub fn run(query: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        anyhow::bail!("{}", t!(Msg::PullRegistryNotFound { path: ss_paths.registry.to_string_lossy().to_string() }));
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    let query_lower = query.to_lowercase();
    let mut matches: Vec<SearchMatch> = Vec::new();

    // Search skills.
    for (name, entry) in &manifest.skills {
        let mut matched = false;
        let mut match_context = String::new();

        // Match on name.
        if name.to_lowercase().contains(&query_lower) {
            matched = true;
            match_context = "name".into();
        }

        // Match on description.
        if let Some(ref desc) = entry.description {
            if desc.to_lowercase().contains(&query_lower) {
                matched = true;
                match_context = if match_context.is_empty() {
                    "description".into()
                } else {
                    format!("{}, description", match_context)
                };
            }
        }

        // Match on tags.
        for tag in &entry.tags {
            if tag.to_lowercase().contains(&query_lower) {
                matched = true;
                match_context = if match_context.is_empty() {
                    format!("tag:{}", tag)
                } else {
                    format!("{}, tag:{}", match_context, tag)
                };
                break; // one tag match is enough
            }
        }

        if matched {
            matches.push(SearchMatch {
                resource_type: "skill",
                name: name.clone(),
                description: entry.description.clone().unwrap_or_default(),
                match_context,
            });
        }
    }

    // Search plugins.
    for (name, entry) in &manifest.plugins {
        if name.to_lowercase().contains(&query_lower)
            || entry.marketplace.to_lowercase().contains(&query_lower)
        {
            matches.push(SearchMatch {
                resource_type: "plugin",
                name: name.clone(),
                description: format!("marketplace: {}", entry.marketplace),
                match_context: "name".into(),
            });
        }
    }

    // Search MCP servers.
    for (name, entry) in &manifest.mcp_servers {
        if name.to_lowercase().contains(&query_lower)
            || entry.command.to_lowercase().contains(&query_lower)
        {
            matches.push(SearchMatch {
                resource_type: "mcp",
                name: name.clone(),
                description: format!("command: {}", entry.command),
                match_context: "name".into(),
            });
        }
    }

    if matches.is_empty() {
        println!(
            "{}",
            t!(Msg::SearchNoResults { query: query.to_string() })
        );
        return Ok(());
    }

    // Sort by type then name.
    matches.sort_by(|a, b| {
        a.resource_type
            .cmp(&b.resource_type)
            .then_with(|| a.name.cmp(&b.name))
    });

    println!(
        "{}",
        t!(Msg::SearchResults { count: matches.len(), query: query.to_string() })
    );
    println!();

    for m in &matches {
        println!(
            "  {}",
            t!(Msg::SearchResultRow {
                kind: format!("[{}]", m.resource_type),
                name: m.name.clone(),
                desc: if m.description.is_empty() {
                    String::new()
                } else {
                    format!("— {}", m.description)
                }
            })
        );
        println!(
            "    {}",
            t!(Msg::SearchMatchedOn { field: m.match_context.clone() })
        );
    }

    Ok(())
}

struct SearchMatch {
    resource_type: &'static str,
    name: String,
    description: String,
    match_context: String,
}
