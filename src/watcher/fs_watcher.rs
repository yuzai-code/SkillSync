// File system watcher for auto-sync
// Implements: tasks 7.1-7.3

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

#[allow(unused_imports)]
use crate::t;
use crate::claude::paths::SkillSyncPaths;
use crate::i18n::Msg;
use crate::registry::git_ops;
use crate::state::StateDb;

/// Start watching the given directories for file changes.
///
/// Uses `notify_debouncer_mini` with a 2-second debounce window.
/// When changes are detected, the `on_change` callback is invoked.
/// This function blocks the calling thread until an unrecoverable error
/// occurs or the process is terminated.
pub fn watch_directories(
    dirs: Vec<PathBuf>,
    on_change: impl Fn() + Send + 'static,
) -> Result<()> {
    // Channel to receive debounced events
    let (tx, rx) = mpsc::channel();

    // Create a debouncer with 2-second timeout
    let mut debouncer = new_debouncer(Duration::from_secs(2), tx)?;

    // Watch each directory recursively
    for dir in &dirs {
        if dir.exists() {
            debouncer
                .watcher()
                .watch(dir, RecursiveMode::Recursive)
                .with_context(|| t!(Msg::WatcherWatching { path: dir.display().to_string() }))?;
            eprintln!(
                "  {}",
                t!(Msg::WatcherWatching { path: dir.display().to_string() })
            );
        } else {
            eprintln!(
                "  {}",
                t!(Msg::WatcherDirNotExist { path: dir.display().to_string() })
            );
        }
    }

    eprintln!("{}", t!(Msg::WatcherStarted));

    // Block on the receiver, processing events as they arrive
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                if events.is_empty() {
                    continue;
                }

                // Log detected changes
                eprintln!("{}", t!(Msg::WatcherDetectedChanges { count: events.len() }));
                for event in &events {
                    eprintln!("{}", t!(Msg::WatcherEventPath { path: event.path.display().to_string() }));
                }

                // Invoke the callback, catching any panic to keep the watcher alive
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    on_change();
                }));

                if let Err(e) = result {
                    eprintln!("{}", t!(Msg::WatcherPanicked { error: format!("{:?}", e) }));
                    eprintln!("{}", t!(Msg::WatcherRetry));
                }
            }
            Ok(Err(errors)) => {
                // Debouncer reported errors — log them and continue
                eprintln!("{}", t!(Msg::WatcherError { error: format!("{:?}", errors) }));
                eprintln!("{}", t!(Msg::WatcherRetry));
            }
            Err(e) => {
                // The channel was disconnected — the debouncer was dropped
                eprintln!("{}", t!(Msg::WatcherChannelClosed { error: e.to_string() }));
                anyhow::bail!("{}", t!(Msg::WatcherStarted));
            }
        }
    }
}

/// Auto-push callback: stages, commits, and pushes any changes in the registry.
///
/// This function is intended to be used as the `on_change` callback for
/// `watch_directories`. Errors are logged but never propagated — the watcher
/// must not crash due to a failed push.
pub fn auto_push() {
    if let Err(e) = auto_push_inner() {
        eprintln!("{}", t!(Msg::WatcherAutoPushFailed { error: format!("{:#}", e) }));
        eprintln!("{}", t!(Msg::WatcherWillRetry));
    }
}

/// Check if auto-sync is enabled in the global config.
fn is_auto_sync_enabled() -> bool {
    match crate::registry::config::GlobalConfig::load() {
        Ok(config) => config.auto_sync,
        Err(_) => true, // Default to enabled if config can't be read
    }
}

/// Inner implementation of auto_push that returns Result for ergonomic error handling.
fn auto_push_inner() -> Result<()> {
    let ss_paths = SkillSyncPaths::resolve()?;

    if !ss_paths.registry_exists() {
        eprintln!(
            "  {} Registry not initialized, skipping auto-push.",
            console::style("Warning:").yellow()
        );
        return Ok(());
    }

    // Check if auto-sync is enabled
    if !is_auto_sync_enabled() {
        eprintln!("{}", t!(Msg::WatcherAutoSyncDisabled));
        return Ok(());
    }

    let repo = git_ops::open_repo(&ss_paths.registry)?;
    let (has_changes, changed_files) = git_ops::repo_status(&repo)?;

    if !has_changes {
        eprintln!("{}", t!(Msg::WatcherNoChanges));
        return Ok(());
    }

    eprintln!("{}", t!(Msg::WatcherStaging { count: changed_files.len() }));

    git_ops::stage_skills_only(&repo)?;

    let message = format!(
        "auto-sync: {} file(s) changed",
        changed_files.len()
    );
    let oid = git_ops::commit(&repo, &message)?;

    eprintln!(
        "  {}",
        t!(Msg::WatcherCommitted {
            message,
            oid: oid.to_string()[..8].to_string()
        })
    );

    // Attempt to push — this may fail if there's no remote configured
    match git_ops::push_origin(&repo) {
        Ok(()) => {
            eprintln!("{}", t!(Msg::WatcherPushed));
        }
        Err(e) => {
            eprintln!("{}", t!(Msg::WatcherPushFailed { error: format!("{:#}", e) }));
            eprintln!("{}", t!(Msg::WatcherPushLocal));
        }
    }

    Ok(())
}

/// Collect the directories that should be watched.
///
/// Returns the global skills directory (`~/.claude/skills/`), the
/// registry resources directory (`~/.skillsync/registry/resources/`), and
/// all discovered project skills directories.
pub fn default_watch_dirs() -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    // Global Claude skills directory
    let claude_paths = crate::claude::paths::ClaudePaths::global()?;
    dirs.push(claude_paths.skills_dir);

    // Registry resources directory
    let ss_paths = SkillSyncPaths::resolve()?;
    if ss_paths.registry_exists() {
        dirs.push(ss_paths.resources);
    }

    // Project skills directories from state.db
    if let Ok(project_dirs) = project_watch_dirs() {
        dirs.extend(project_dirs);
    }

    Ok(dirs)
}

/// Collect project skills directories from the discovered projects in state.db.
///
/// Reads the `discovered_projects` table and returns the `.claude/skills/`
/// path for each active project.
pub fn project_watch_dirs() -> Result<Vec<PathBuf>> {
    let ss_paths = SkillSyncPaths::resolve()?;
    let db_path = &ss_paths.state_db;

    if !db_path.exists() {
        return Ok(Vec::new());
    }

    let db = StateDb::open(&db_path)?;
    let projects = db.get_discovered_projects(false)?;

    let mut dirs = Vec::new();
    for project in projects {
        let skills_dir = Path::new(&project.project_path).join(".claude").join("skills");
        if skills_dir.is_dir() {
            dirs.push(skills_dir);
        }
    }

    Ok(dirs)
}
