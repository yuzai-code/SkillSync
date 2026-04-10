use std::fs;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::{ClaudePaths, SkillSyncPaths};
use crate::i18n::Msg;
use crate::registry::discover;
use crate::registry::git_ops;
use crate::registry::manifest::Manifest;
use crate::tui::selector::{select_install_scope, select_remote_skills, InstallScope, RemoteSkillItem};

pub fn run(quiet: bool, skip_select: bool) -> Result<()> {
    let paths = SkillSyncPaths::resolve().context("Failed to resolve SkillSync paths")?;

    if !paths.registry_exists() {
        bail!("{}", t!(Msg::SyncRegistryNotFound { path: paths.registry.display().to_string() }));
    }

    let repo = git_ops::open_repo(&paths.registry)
        .context("Failed to open registry git repository")?;

    // Check if origin remote exists for remote sync
    let has_origin = repo.find_remote("origin").is_ok();

    if !quiet {
        println!(
            "{} {}",
            style("⟳").cyan().bold(),
            t!(Msg::SyncSyncing)
        );
    }

    // ---- Phase 1: Pull (fetch + merge) - only if origin exists ----
    if has_origin {
        if !quiet {
            println!(
                "{} {}",
                style("↓").cyan().bold(),
                t!(Msg::SyncFetching)
            );
        }

        git_ops::fetch_origin(&repo).context("Fetch from origin failed")?;
        let merge = git_ops::merge_origin(&repo).context("Merge failed")?;

        // Report pull result.
        if !merge.conflicts.is_empty() {
            eprintln!(
                "{} {}",
                style("✗").red().bold(),
                t!(Msg::SyncMergeConflicts { count: merge.conflicts.len() })
            );
            for path in &merge.conflicts {
                eprintln!("{}", t!(Msg::SyncConflictFile { file: path.clone() }));
            }
            eprintln!("{}", t!(Msg::SyncResolveHint));
            bail!("{}", t!(Msg::SyncAborted));
        }

        if merge.up_to_date {
            if !quiet {
                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    t!(Msg::SyncUpToDate)
                );
            }
        } else if merge.fast_forward {
            if !quiet {
                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    t!(Msg::SyncFastForwarded)
                );
            }
        } else {
            if !quiet {
                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    t!(Msg::SyncMerged)
                );
            }
        }
    } else if !quiet {
        println!(
            "{} {}",
            style("ℹ").dim(),
            "Local-only mode (no remote configured)"
        );
    }

    // ---- Phase 1b: Scan local projects for new/changed skills -------------
    let manifest_path = paths.registry.join("manifest.yaml");
    let mut new_skills_from_discovery: Vec<RemoteSkillItem> = Vec::new();

    if manifest_path.exists() {
        let mut manifest = match Manifest::load(&manifest_path) {
            Ok(m) => m,
            Err(e) => {
                let err_msg = format!("{:#}", e);
                eprintln!(
                    "{} {}",
                    style("⚠").yellow(),
                    t!(Msg::SyncManifestLoadError { error: err_msg })
                );
                return Ok(());
            }
        };

        // Clean up deleted skills (7.3)
        let removed = discover::cleanup_deleted_skills(&mut manifest, &paths.registry);
        if !removed.is_empty() && !quiet {
            println!(
                "  {} Removed {} deleted skill(s) from manifest",
                style("·").dim(),
                removed.len()
            );
            for name in &removed {
                println!("    {} {}", style("-").red(), name);
            }
        }

        match discover::scan_all_local_skills() {
            Ok(discovered) if !discovered.is_empty() => {
                // Copy discovered skills to registry and register in manifest
                let mut new_skills = 0;
                for skill in &discovered {
                    let dest_dir = paths.registry
                        .join("resources")
                        .join("skills")
                        .join(&skill.name);

                    // Check if this is a new skill
                    let is_new = !dest_dir.exists();
                    let content_changed = manifest.skills.get(&skill.name).map(|e| e.backup_hash.as_ref() != Some(&skill.content_hash)).unwrap_or(true);

                    // Copy skill files to registry if they don't exist or content changed
                    if is_new || content_changed {
                        if let Err(e) = copy_skill_dir(&skill.path, &dest_dir) {
                            eprintln!(
                                "{} {}",
                                style("⚠").yellow(),
                                t!(Msg::SyncCopySkillError { name: skill.name.clone(), error: e.to_string() })
                            );
                            continue;
                        }
                        new_skills += 1;

                        // Track new skills for TUI selection
                        if is_new {
                            new_skills_from_discovery.push(RemoteSkillItem {
                                name: skill.name.clone(),
                                source_project: skill.project_path.display().to_string(),
                                description: None,
                            });
                        }
                    }
                }

                // Register in manifest (handles deduplication)
                discover::register_discovered_skills(&mut manifest, &discovered);

                // Save updated manifest
                if let Err(e) = manifest.save(&manifest_path) {
                    let err_msg = format!("{:#}", e);
                    eprintln!(
                        "{} {}",
                        style("⚠").yellow(),
                        t!(Msg::SyncManifestSaveError { error: err_msg })
                    );
                } else if !quiet && new_skills > 0 {
                    println!(
                        "  {} {}",
                        style("·").dim(),
                        t!(Msg::SyncDiscoveredSkills { count: discovered.len(), new: new_skills })
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                if !quiet {
                    eprintln!(
                        "{} {}",
                        style("⚠").yellow(),
                        t!(Msg::SyncScanProjectsError { error: e.to_string() })
                    );
                }
            }
        }

        // Scan and register plugins
        match discover::scan_global_plugins() {
            Ok(plugins) if !plugins.is_empty() => {
                let new_count = plugins.iter().filter(|p| !manifest.plugins.contains_key(&p.name)).count();
                discover::register_discovered_plugins(&mut manifest, &plugins);
                if !quiet && new_count > 0 {
                    println!(
                        "  {} Discovered {} plugin(s), {} new",
                        style("·").dim(),
                        plugins.len(),
                        new_count
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                if !quiet {
                    eprintln!(
                        "{} Failed to scan plugins: {}",
                        style("⚠").yellow(),
                        e
                    );
                }
            }
        }

        // Scan and register MCP servers
        match discover::scan_global_mcp() {
            Ok(mcp_servers) if !mcp_servers.is_empty() => {
                let new_count = mcp_servers.iter().filter(|m| !manifest.mcp_servers.contains_key(&m.name)).count();
                discover::register_discovered_mcp(&mut manifest, &mcp_servers);
                if !quiet && new_count > 0 {
                    println!(
                        "  {} Discovered {} MCP server(s), {} new",
                        style("·").dim(),
                        mcp_servers.len(),
                        new_count
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                if !quiet {
                    eprintln!(
                        "{} Failed to scan MCP servers: {}",
                        style("⚠").yellow(),
                        e
                    );
                }
            }
        }

        // Save manifest after all discoveries
        if let Err(e) = manifest.save(&manifest_path) {
            let err_msg = format!("{:#}", e);
            eprintln!(
                "{} {}",
                style("⚠").yellow(),
                t!(Msg::SyncManifestSaveError { error: err_msg })
            );
        }
    }

    // ---- Phase 1c: TUI Selection for new skills (7.1) ----
    if !skip_select && !new_skills_from_discovery.is_empty() {
        println!();
        println!(
            "{} Found {} new skill(s) to install:",
            style("✦").cyan(),
            new_skills_from_discovery.len()
        );

        let selected = select_remote_skills(&new_skills_from_discovery)?;

        if !selected.is_empty() {
            let scope = select_install_scope()?;

            match scope {
                InstallScope::Global => {
                    // Install to ~/.claude/skills/
                    let claude_paths = ClaudePaths::global()
                        .context("Failed to resolve Claude paths")?;
                    let claude_skills = claude_paths.skills_dir;
                    for skill_name in &selected {
                        let src = paths.registry.join("resources").join("skills").join(skill_name);
                        let dst = claude_skills.join(skill_name);
                        if src.exists() {
                            if dst.exists() {
                                fs::remove_dir_all(&dst)
                                    .with_context(|| format!("Failed to remove existing skill: {}", dst.display()))?;
                            }
                            fs::create_dir_all(&dst)
                                .with_context(|| format!("Failed to create skill directory: {}", dst.display()))?;
                            copy_skill_dir(&src, &dst)?;
                            if !quiet {
                                println!("  {} Installed '{}' to ~/.claude/skills/", style("✓").green(), skill_name);
                            }
                        }
                    }
                }
                InstallScope::Project { path } => {
                    // Install to specified project
                    if path.is_empty() {
                        println!("  {} No project path specified, skipping install.", style("⚠").yellow());
                    } else {
                        let project_skills = std::path::PathBuf::from(&path).join(".claude").join("skills");
                        for skill_name in &selected {
                            let src = paths.registry.join("resources").join("skills").join(skill_name);
                            let dst = project_skills.join(skill_name);
                            if src.exists() {
                                if dst.exists() {
                                    fs::remove_dir_all(&dst)
                                        .with_context(|| format!("Failed to remove existing skill: {}", dst.display()))?;
                                }
                                fs::create_dir_all(&dst)
                                    .with_context(|| format!("Failed to create skill directory: {}", dst.display()))?;
                                copy_skill_dir(&src, &dst)?;
                                if !quiet {
                                    println!("  {} Installed '{}' to {}", style("✓").green(), skill_name, path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ---- Phase 2: Push (stage, commit, push) - only if origin exists ------
    let (has_changes, changed_files) =
        git_ops::repo_status(&repo).context("Failed to read repository status")?;

    if !has_changes {
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncNoLocalChanges)
            );
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncComplete)
            );
        }
        return Ok(());
    }

    if !quiet {
        println!(
            "{} {}",
            style("↑").cyan().bold(),
            t!(Msg::SyncPushing { count: changed_files.len() })
        );
    }

    git_ops::stage_all(&repo).context("Failed to stage changes")?;
    git_ops::commit(&repo, "sync: update registry")
        .context("Failed to create commit")?;

    // Only push if origin exists
    if has_origin {
        git_ops::push_origin(&repo).context("Failed to push to origin")?;
        if !quiet {
            println!(
                "{} {}",
                style("✓").green().bold(),
                t!(Msg::SyncPushed { count: changed_files.len() })
            );
            for f in &changed_files {
                println!("{}", t!(Msg::SyncCommitFile { file: f.clone() }));
            }
        }
    } else if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            format!("Committed {} change(s) locally (no remote to push)", changed_files.len())
        );
        for f in &changed_files {
            println!("{}", t!(Msg::SyncCommitFile { file: f.clone() }));
        }
    }

    if !quiet {
        println!(
            "{} {}",
            style("✓").green().bold(),
            t!(Msg::SyncComplete)
        );
    }

    Ok(())
}

/// Copy a skill directory recursively from source to destination.
fn copy_skill_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)
            .with_context(|| format!("Failed to remove existing skill directory: {}", dst.display()))?;
    }
    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create skill directory: {}", dst.display()))?;

    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read skill directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_skill_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("Failed to copy {} to {}", src_path.display(), dst_path.display()))?;
        }
    }

    Ok(())
}
