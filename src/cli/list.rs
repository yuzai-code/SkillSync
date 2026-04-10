use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use console::style;
use unicode_width::UnicodeWidthStr;

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

/// A row in the output table.
struct TableRow {
    name: String,
    resource_type: String,
    scope: String,
    version: String,
    source: String,
}

/// Format a string with padding for display width (handles CJK characters).
fn pad_to_width(s: &str, width: usize) -> String {
    let display_width = UnicodeWidthStr::width(s);
    if display_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - display_width))
    }
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
                println!("Run 'skillsync init' to initialize the registry.");
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
                version: format!("v{}", entry.version),
                source: extract_source(entry.source_path.as_deref()),
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
                source: "marketplace".to_string(),
            });
        }
    }

    // Collect MCP servers.
    if type_filter.is_none() || type_filter == Some("mcp") {
        for (name, entry) in &manifest.mcp_servers {
            // Truncate command display if too long
            let cmd_display = if entry.command.len() > 15 {
                format!("{}...", &entry.command[..12])
            } else {
                entry.command.clone()
            };
            rows.push(TableRow {
                name: name.clone(),
                resource_type: "mcp".into(),
                scope: format!("{:?}", entry.scope).to_lowercase(),
                version: cmd_display,
                source: "config".to_string(),
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
            println!("Use 'skillsync add' to register resources.");
        }
        return Ok(());
    }

    // Sort rows by type priority (skill > plugin > mcp) then name for stable output.
    rows.sort_by(|a, b| {
        let type_order = |t: &str| -> u8 {
            match t {
                "skill" => 0,
                "plugin" => 1,
                "mcp" => 2,
                _ => 3,
            }
        };
        type_order(&a.resource_type)
            .cmp(&type_order(&b.resource_type))
            .then_with(|| a.name.cmp(&b.name))
    });

    // Compute column widths for alignment (using unicode display width).
    let name_width = rows.iter().map(|r| UnicodeWidthStr::width(r.name.as_str())).max().unwrap_or(4).max(4);
    let scope_width = rows.iter().map(|r| UnicodeWidthStr::width(r.scope.as_str())).max().unwrap_or(5).max(5);
    let version_width = rows.iter().map(|r| UnicodeWidthStr::width(r.version.as_str())).max().unwrap_or(7).max(7);
    let source_width = rows.iter().map(|r| UnicodeWidthStr::width(r.source.as_str())).max().unwrap_or(6).max(6);

    // Also consider header widths
    let name_width = name_width.max(UnicodeWidthStr::width("名称"));
    let scope_width = scope_width.max(UnicodeWidthStr::width("作用域"));
    let version_width = version_width.max(UnicodeWidthStr::width("版本"));
    let source_width = source_width.max(UnicodeWidthStr::width("来源"));

    // Group by type for better readability
    let mut current_type = "";
    for row in &rows {
        // Print type header when type changes
        if row.resource_type != current_type {
            if !current_type.is_empty() {
                println!();
            }
            current_type = &row.resource_type;
            let type_header = match current_type {
                "skill" => style("Skills").cyan().bold(),
                "plugin" => style("Plugins").magenta().bold(),
                "mcp" => style("MCP Servers").yellow().bold(),
                _ => style(current_type).bold(),
            };
            println!("  {}", type_header);
            println!(
                "  {}  {}  {}  {}",
                style(pad_to_width("名称", name_width)).dim(),
                style(pad_to_width("作用域", scope_width)).dim(),
                style(pad_to_width("版本", version_width)).dim(),
                style(pad_to_width("来源", source_width)).dim(),
            );
        }

        // Color by source
        let source_display = if row.source == "global" {
            style(&row.source).green().to_string()
        } else if row.source.starts_with("project:") {
            style(&row.source).cyan().to_string()
        } else if row.source == "marketplace" {
            style(&row.source).magenta().to_string()
        } else if row.source == "config" {
            style(&row.source).yellow().to_string()
        } else {
            style(&row.source).dim().to_string()
        };

        println!(
            "  {}  {}  {}  {}",
            pad_to_width(&row.name, name_width),
            style(pad_to_width(&row.scope, scope_width)).dim(),
            style(pad_to_width(&row.version, version_width)).dim(),
            source_display,
        );
    }

    println!();
    println!(
        "  {}",
        t!(Msg::ListTotal { count: rows.len() })
    );

    Ok(())
}

/// Scan all local skills (global + project-level) not managed by registry.
fn list_local_skills() -> Result<()> {
    use crate::registry::discover::scan_all_local_skills;

    let discovered = match scan_all_local_skills() {
        Ok(s) => s,
        Err(_) => {
            println!("{}", t!(Msg::ListNoResources));
            println!("Run 'skillsync init' to initialize the registry.");
            return Ok(());
        }
    };

    if discovered.is_empty() {
        println!("{}", t!(Msg::ListNoResources));
        println!("Run 'skillsync init' to initialize the registry.");
        return Ok(());
    }

    println!("{}", t!(Msg::ListLocalSkillsFound { count: discovered.len() }));
    println!();

    for skill in &discovered {
        let location = if skill.project_path == dirs::home_dir().unwrap_or_default() {
            "(global)".to_string()
        } else {
            let project_name = skill.project_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            format!("(project: {})", project_name)
        };
        println!("  {} {}", skill.name, style(location).dim());
    }

    println!();
    println!("Use 'skillsync sync' to auto-discover and register these skills.");

    Ok(())
}
