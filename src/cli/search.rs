use anyhow::{Context, Result};
use console::style;

use crate::claude::paths::SkillSyncPaths;
use crate::registry::manifest::Manifest;

pub fn run(query: &str) -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;
    if !ss_paths.registry_exists() {
        anyhow::bail!("Registry not found. Run 'skillsync init' first.");
    }

    let manifest = Manifest::load(&ss_paths.manifest)
        .context("Failed to load manifest")?;

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
            "No results for '{}'.",
            style(query).yellow()
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
        "Found {} result(s) for '{}':",
        style(matches.len()).bold(),
        style(query).yellow()
    );
    println!();

    for m in &matches {
        println!(
            "  {} {} {}",
            style(format!("[{}]", m.resource_type)).dim(),
            style(&m.name).cyan().bold(),
            if m.description.is_empty() {
                String::new()
            } else {
                format!("— {}", m.description)
            }
        );
        println!(
            "    matched on: {}",
            style(&m.match_context).dim()
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
