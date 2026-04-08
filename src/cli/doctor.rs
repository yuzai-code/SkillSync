use std::collections::HashSet;

use anyhow::Result;
use console::style;
use git2::Repository;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::{ClaudePaths, SkillSyncPaths};
use crate::i18n::Msg;
use crate::registry::manifest::Manifest;

/// Print a pass/fail/warn line.
fn check_pass(label: String) {
    println!("  {} {}", style("✓").green().bold(), label);
}

fn check_fail(label: String) {
    println!("  {} {}", style("✗").red().bold(), label);
}

fn check_warn(label: String) {
    println!("  {} {}", style("⚠").yellow().bold(), label);
}

pub fn run() -> Result<()> {
    println!("{}", style(t!(Msg::DoctorTitle)).bold().underlined());
    println!();

    let ss_paths = SkillSyncPaths::resolve()?;
    let claude_paths = ClaudePaths::global()?;

    let mut issues = 0;
    let mut warnings = 0;

    // 1. Registry exists?
    if ss_paths.registry_exists() {
        check_pass(t!(Msg::DoctorRegistryExists));
    } else {
        check_fail(t!(Msg::DoctorRegistryNotFound));
        issues += 1;
        // If registry doesn't exist, remaining checks are moot for manifest.
        println!();
        println!(
            "  {}",
            t!(Msg::DoctorRunInitHint { cmd: "skillsync init".to_string() })
        );
        // Still check Claude Code paths.
        check_claude_paths(&claude_paths, &mut issues);
        return print_summary(issues, warnings);
    }

    // 2. manifest.yaml is parseable?
    let manifest = match Manifest::load(&ss_paths.manifest) {
        Ok(m) => {
            check_pass(t!(Msg::DoctorManifestValid));
            Some(m)
        }
        Err(e) => {
            check_fail(t!(Msg::DoctorManifestParseFailed { error: e.to_string() }));
            issues += 1;
            None
        }
    };

    // 3. manifest.yaml passes validation?
    if let Some(ref m) = manifest {
        match m.validate() {
            Ok(()) => {
                check_pass(t!(Msg::DoctorManifestValid));
            }
            Err(errors) => {
                check_warn(t!(Msg::DoctorValidationIssues { count: errors.len() }));
                for err in &errors {
                    println!("{}", t!(Msg::DoctorValidationError { error: err.to_string() }));
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
                check_pass(t!(Msg::DoctorOriginConfigured { url: url.to_string() }));
            }
            Err(_) => {
                check_warn(t!(Msg::DoctorNoOrigin));
                warnings += 1;
            }
        },
        Err(_) => {
            check_fail(t!(Msg::DoctorNotGitRepo));
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

fn check_claude_paths(claude_paths: &ClaudePaths, issues: &mut usize) {
    if claude_paths.exists() {
        check_pass(t!(Msg::DoctorClaudeHomeExists));
    } else {
        check_fail(t!(Msg::DoctorClaudeHomeNotFound));
        *issues += 1;
    }
}

fn check_orphaned_resources(
    ss_paths: &SkillSyncPaths,
    manifest: &Manifest,
    warnings: &mut usize,
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
                    check_pass(t!(Msg::DoctorNoOrphaned));
                } else {
                    check_warn(t!(Msg::DoctorOrphanedSkills { count: orphaned.len() }));
                    for name in &orphaned {
                        println!("{}", t!(Msg::DoctorOrphanedSkill { name: name.to_string() }));
                    }
                    *warnings += 1;
                }
            }
            Err(_) => {
                check_warn(t!(Msg::DoctorOrphanedReadError));
                *warnings += 1;
            }
        }
    } else {
        check_pass(t!(Msg::DoctorNoOrphaned));
    }
}

fn check_missing_resources(
    ss_paths: &SkillSyncPaths,
    manifest: &Manifest,
    warnings: &mut usize,
) {
    let mut missing = Vec::new();

    for (name, entry) in &manifest.skills {
        let skill_path = ss_paths.registry.join(&entry.path);
        if !skill_path.exists() {
            missing.push(name.clone());
        }
    }

    if missing.is_empty() {
        check_pass(t!(Msg::DoctorManifestSkillsOk));
    } else {
        check_warn(t!(Msg::DoctorMissingSkills { count: missing.len() }));
        for name in &missing {
            println!("{}", t!(Msg::DoctorMissingSkill { name: name.to_string() }));
        }
        *warnings += 1;
    }
}

fn print_summary(issues: usize, warnings: usize) -> Result<()> {
    if issues == 0 && warnings == 0 {
        println!(
            "  {} {}",
            style("✓").green().bold(),
            t!(Msg::DoctorAllPassed)
        );
    } else {
        if issues > 0 {
            println!(
                "  {} {}",
                style("✗").red().bold(),
                t!(Msg::DoctorIssues { count: issues })
            );
        }
        if warnings > 0 {
            println!(
                "  {} {}",
                style("⚠").yellow().bold(),
                t!(Msg::DoctorWarnings { count: warnings })
            );
        }
    }
    Ok(())
}
