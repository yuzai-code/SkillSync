use std::collections::HashSet;

use anyhow::Result;
use console::style;
use git2::Repository;

use crate::claude::paths::{ClaudePaths, SkillSyncPaths};
use crate::registry::manifest::Manifest;

/// Print a pass/fail/warn line.
fn check_pass(label: &str) {
    println!("  {} {}", style("✓").green().bold(), label);
}

fn check_fail(label: &str) {
    println!("  {} {}", style("✗").red().bold(), label);
}

fn check_warn(label: &str) {
    println!("  {} {}", style("⚠").yellow().bold(), label);
}

pub fn run() -> Result<()> {
    println!("{}", style("SkillSync Doctor").bold().underlined());
    println!();

    let ss_paths = SkillSyncPaths::resolve()?;
    let claude_paths = ClaudePaths::global()?;

    let mut issues = 0u32;
    let mut warnings = 0u32;

    // 1. Registry exists?
    if ss_paths.registry_exists() {
        check_pass("Registry exists");
    } else {
        check_fail("Registry not found at ~/.skillsync/registry/");
        issues += 1;
        // If registry doesn't exist, remaining checks are moot for manifest.
        println!();
        println!(
            "  Run '{}' to initialize the registry.",
            style("skillsync init").cyan()
        );
        // Still check Claude Code paths.
        check_claude_paths(&claude_paths, &mut issues);
        return print_summary(issues, warnings);
    }

    // 2. manifest.yaml is parseable?
    let manifest = match Manifest::load(&ss_paths.manifest) {
        Ok(m) => {
            check_pass("manifest.yaml is parseable");
            Some(m)
        }
        Err(e) => {
            check_fail(&format!("manifest.yaml failed to parse: {}", e));
            issues += 1;
            None
        }
    };

    // 3. manifest.yaml passes validation?
    if let Some(ref m) = manifest {
        match m.validate() {
            Ok(()) => {
                check_pass("manifest.yaml passes validation");
            }
            Err(errors) => {
                check_warn(&format!(
                    "manifest.yaml has {} validation issue(s):",
                    errors.len()
                ));
                for err in &errors {
                    println!("    - {}", err);
                }
                warnings += 1;
            }
        }
    }

    // 4. Git remote is configured?
    match Repository::open(&ss_paths.registry) {
        Ok(repo) => match repo.find_remote("origin") {
            Ok(remote) => {
                let url = remote.url().unwrap_or("(no URL)");
                check_pass(&format!("Git remote 'origin' configured: {}", url));
            }
            Err(_) => {
                check_warn("Git remote 'origin' not configured (sync will not work)");
                warnings += 1;
            }
        },
        Err(_) => {
            check_fail("Registry is not a git repository");
            issues += 1;
        }
    }

    // 5. Claude Code home exists?
    check_claude_paths(&claude_paths, &mut issues);

    // 6. Check for orphaned resources (on disk but not in manifest).
    if let Some(ref m) = manifest {
        check_orphaned_resources(&ss_paths, m, &mut warnings);
    }

    // 7. Check for missing resources (in manifest but not on disk).
    if let Some(ref m) = manifest {
        check_missing_resources(&ss_paths, m, &mut warnings);
    }

    println!();
    print_summary(issues, warnings)
}

fn check_claude_paths(claude_paths: &ClaudePaths, issues: &mut u32) {
    if claude_paths.exists() {
        check_pass("Claude Code home (~/.claude/) exists");
    } else {
        check_fail("Claude Code home (~/.claude/) not found");
        *issues += 1;
    }
}

fn check_orphaned_resources(
    ss_paths: &SkillSyncPaths,
    manifest: &Manifest,
    warnings: &mut u32,
) {
    // Collect skill names from manifest.
    let manifest_skills: HashSet<&str> = manifest.skills.keys().map(|s| s.as_str()).collect();

    // Scan the resources/skills/ directory.
    if ss_paths.skills_dir.is_dir() {
        match std::fs::read_dir(&ss_paths.skills_dir) {
            Ok(entries) => {
                let mut orphaned = Vec::new();
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            if !manifest_skills.contains(name) {
                                orphaned.push(name.to_string());
                            }
                        }
                    }
                }
                if orphaned.is_empty() {
                    check_pass("No orphaned resources found");
                } else {
                    check_warn(&format!(
                        "{} orphaned skill(s) in resources/skills/ (not in manifest):",
                        orphaned.len()
                    ));
                    for name in &orphaned {
                        println!("    - {}", name);
                    }
                    *warnings += 1;
                }
            }
            Err(_) => {
                check_warn("Could not read resources/skills/ directory");
                *warnings += 1;
            }
        }
    } else {
        check_pass("No orphaned resources found");
    }
}

fn check_missing_resources(
    ss_paths: &SkillSyncPaths,
    manifest: &Manifest,
    warnings: &mut u32,
) {
    let mut missing = Vec::new();

    for (name, entry) in &manifest.skills {
        let skill_path = ss_paths.registry.join(&entry.path);
        if !skill_path.exists() {
            missing.push(name.clone());
        }
    }

    if missing.is_empty() {
        check_pass("All manifest skills have matching resource files");
    } else {
        check_warn(&format!(
            "{} skill(s) referenced in manifest but missing on disk:",
            missing.len()
        ));
        for name in &missing {
            println!("    - {}", name);
        }
        *warnings += 1;
    }
}

fn print_summary(issues: u32, warnings: u32) -> Result<()> {
    if issues == 0 && warnings == 0 {
        println!(
            "  {} All checks passed!",
            style("✓").green().bold()
        );
    } else {
        if issues > 0 {
            println!(
                "  {} {} issue(s) found",
                style("✗").red().bold(),
                issues
            );
        }
        if warnings > 0 {
            println!(
                "  {} {} warning(s)",
                style("⚠").yellow().bold(),
                warnings
            );
        }
    }
    Ok(())
}
