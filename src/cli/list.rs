use anyhow::{bail, Context, Result};
use console::style;
use std::fs;

use crate::claude::paths::ClaudePaths;
use crate::i18n::Msg;
#[allow(unused_imports)]
use crate::t;
use crate::registry::Manifest;

#[allow(dead_code)]

/// Resolve the manifest path: `~/.skillsync/registry/manifest.yaml`
fn manifest_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;
    Ok(home.join(".skillsync").join("registry").join("manifest.yaml"))
}

/// A row in the output table.
struct TableRow {
    name: String,
    resource_type: String,
    scope: String,
    version: String,
}

pub fn run(type_filter: Option<&str>) -> Result<()> {
    // Validate the filter value if provided.
    if let Some(filter) = type_filter {
        match filter {
            "skill" | "plugin" | "mcp" => {}
            other => bail!("{}", t!(Msg::ListInvalidTypeFilter { filter: other.to_string() })),
        }
    }

    let path = manifest_path()?;

    // Try to load manifest; if registry doesn't exist, scan local skills.
    let manifest = match Manifest::load(&path) {
        Ok(m) => m,
        Err(_) => {
            // Registry not initialized — scan local skills as fallback.
            if type_filter.is_none() || type_filter == Some("skill") {
                list_local_skills()?;
            } else {
                println!("{}", t!(Msg::ListNoResources));
                println!("{}", t!(Msg::ListUseAddHint { cmd: "skillsync init".to_string() }));
            }
            return Ok(());
        }
    };

    let mut rows: Vec<TableRow> = Vec::new();

    // Collect skills.
    if type_filter.is_none() || type_filter == Some("skill") {
        for (name, entry) in &manifest.skills {
            rows.push(TableRow {
                name: name.clone(),
                resource_type: "skill".into(),
                scope: format!("{:?}", entry.scope).to_lowercase(),
                version: entry.version.clone(),
            });
        }
    }

    // Collect plugins.
    if type_filter.is_none() || type_filter == Some("plugin") {
        for (name, entry) in &manifest.plugins {
            rows.push(TableRow {
                name: name.clone(),
                resource_type: "plugin".into(),
                scope: entry.marketplace.clone(),
                version: entry.version.clone(),
            });
        }
    }

    // Collect MCP servers.
    if type_filter.is_none() || type_filter == Some("mcp") {
        for (name, entry) in &manifest.mcp_servers {
            rows.push(TableRow {
                name: name.clone(),
                resource_type: "mcp".into(),
                scope: format!("{:?}", entry.scope).to_lowercase(),
                version: entry.command.clone(),
            });
        }
    }

    if rows.is_empty() {
        if let Some(filter) = type_filter {
            println!("{}", t!(Msg::ListNoResourcesOfType { kind: filter.to_string() }));
        } else {
            println!("{}", t!(Msg::ListNoResources));
        }
        // Also scan local skills when registry is empty.
        if type_filter.is_none() || type_filter == Some("skill") {
            list_local_skills()?;
        } else {
            println!("{}", t!(Msg::ListUseAddHint { cmd: "skillsync add".to_string() }));
        }
        return Ok(());
    }

    // Sort rows by type then name for stable output.
    rows.sort_by(|a, b| {
        a.resource_type
            .cmp(&b.resource_type)
            .then_with(|| a.name.cmp(&b.name))
    });

    // Compute column widths for alignment.
    let name_width = rows.iter().map(|r| r.name.len()).max().unwrap_or(4).max(4);
    let type_width = rows.iter().map(|r| r.resource_type.len()).max().unwrap_or(4).max(4);
    let scope_width = rows.iter().map(|r| r.scope.len()).max().unwrap_or(5).max(5);
    let version_width = rows.iter().map(|r| r.version.len()).max().unwrap_or(7).max(7);

    // Print header.
    println!(
        "  {:<name_w$}  {:<type_w$}  {:<scope_w$}  {:<ver_w$}",
        style(t!(Msg::ListColName)).bold().underlined(),
        style(t!(Msg::ListColType)).bold().underlined(),
        style(t!(Msg::ListColScope)).bold().underlined(),
        style(t!(Msg::ListColVersion)).bold().underlined(),
        name_w = name_width,
        type_w = type_width,
        scope_w = scope_width,
        ver_w = version_width,
    );

    // Print rows.
    for row in &rows {
        println!(
            "  {:<name_w$}  {:<type_w$}  {:<scope_w$}  {:<ver_w$}",
            row.name,
            row.resource_type,
            row.scope,
            row.version,
            name_w = name_width,
            type_w = type_width,
            scope_w = scope_width,
            ver_w = version_width,
        );
    }

    println!();
    println!(
        "  {}",
        t!(Msg::ListTotal { count: rows.len() })
    );

    Ok(())
}

/// Scan `~/.claude/skills/` for skills not managed by registry.
fn list_local_skills() -> Result<()> {
    let claude = ClaudePaths::global()?;

    if !claude.skills_dir.is_dir() {
        println!("{}", t!(Msg::ListNoResources));
        println!("{}", t!(Msg::ListUseAddHint { cmd: "skillsync init".to_string() }));
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&claude.skills_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    if entries.is_empty() {
        println!("{}", t!(Msg::ListNoResources));
        println!("{}", t!(Msg::ListUseAddHint { cmd: "skillsync init".to_string() }));
        return Ok(());
    }

    println!("{}", t!(Msg::ListLocalSkillsFound { count: entries.len() }));
    println!();

    for entry in &entries {
        if let Some(name) = entry.file_name().to_str() {
            println!("  {}", name);
        }
    }

    println!();
    println!("{}", t!(Msg::ListUseAddHint { cmd: "skillsync add".to_string() }));

    Ok(())
}
