// Watch command — file watcher daemon, system service install/uninstall
// Implements: tasks 7.4, 7.7, 7.8, 7.9

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::i18n::Msg;
use crate::watcher::fs_watcher;

pub fn run(daemon: bool, install: bool, uninstall: bool, pause: bool, resume: bool) -> Result<()> {
    if install {
        return install_service();
    }

    if uninstall {
        return uninstall_service();
    }

    if pause {
        return pause_sync();
    }

    if resume {
        return resume_sync();
    }

    if daemon {
        return run_daemon();
    }

    // Foreground mode — block on the file watcher
    run_foreground()
}

// ---------------------------------------------------------------------------
// Foreground mode (7.1-7.3)
// ---------------------------------------------------------------------------

/// Run the file watcher in the foreground, blocking the current process.
fn run_foreground() -> Result<()> {
    let dirs = fs_watcher::default_watch_dirs()?;

    if dirs.is_empty() {
        bail!("{}", t!(Msg::WatchNoDirs));
    }

    eprintln!(
        "{}",
        console::style(t!(Msg::WatchStarting)).bold()
    );

    fs_watcher::watch_directories(dirs, || {
        fs_watcher::auto_push();
    })
}

// ---------------------------------------------------------------------------
// Daemon mode (7.4)
// ---------------------------------------------------------------------------

/// Launch the watcher as a background process.
///
/// Re-spawns the current binary with `watch` (without `--daemon`) and
/// detaches it. On failure, suggests using `nohup` as a fallback.
fn run_daemon() -> Result<()> {
    let binary = std::env::current_exe().with_context(|| t!(Msg::ContextCurrentDir))?;

    // Ensure log directory exists
    let log_dir = skillsync_log_dir()?;
    fs::create_dir_all(&log_dir)
        .with_context(|| t!(Msg::ContextCreateDir { path: log_dir.display().to_string() }))?;

    let stdout_log = log_dir.join("watcher.log");
    let stderr_log = log_dir.join("watcher.err.log");

    let stdout_file = fs::File::create(&stdout_log)
        .with_context(|| t!(Msg::ContextCreateDir { path: stdout_log.display().to_string() }))?;
    let stderr_file = fs::File::create(&stderr_log)
        .with_context(|| t!(Msg::ContextCreateDir { path: stderr_log.display().to_string() }))?;

    let child = Command::new(&binary)
        .arg("watch")
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn();

    match child {
        Ok(child) => {
            eprintln!(
                "{} {}",
                console::style("[daemon]").green().bold(),
                t!(Msg::WatchDaemonStarted { pid: child.id() })
            );
            eprintln!("  {}", t!(Msg::WatchLogs { path: stdout_log.display().to_string() }));
            eprintln!("  {}", t!(Msg::WatchErrors { path: stderr_log.display().to_string() }));
            eprintln!(
                "  {}",
                t!(Msg::WatchStopWith {
                    cmd1: "kill".to_string(),
                    cmd2: "skillsync watch --uninstall".to_string()
                })
            );
            Ok(())
        }
        Err(e) => {
            eprintln!(
                "{} {}",
                console::style("Error:").red(),
                t!(Msg::WatchDaemonFailed { error: e.to_string() })
            );
            eprintln!(
                "  {}",
                t!(Msg::WatchTryManual {
                    cmd: format!("nohup {} watch &", binary.display())
                })
            );
            bail!("{}", t!(Msg::WatchDaemonFailed { error: e.to_string() }));
        }
    }
}

// ---------------------------------------------------------------------------
// Service install (7.7, 7.8)
// ---------------------------------------------------------------------------

/// Install the watcher as a system service (launchd on macOS, systemd on Linux).
fn install_service() -> Result<()> {
    if cfg!(target_os = "macos") {
        install_launchd_service()
    } else if cfg!(target_os = "linux") {
        install_systemd_service()
    } else {
        bail!("{}", t!(Msg::WatchServiceNotSupported));
    }
}

/// Generate and load a macOS launchd plist.
fn install_launchd_service() -> Result<()> {
    let binary = std::env::current_exe().with_context(|| t!(Msg::ContextCurrentDir))?;
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;

    let plist_dir = home.join("Library/LaunchAgents");
    fs::create_dir_all(&plist_dir)
        .with_context(|| t!(Msg::ContextCreateDir { path: plist_dir.display().to_string() }))?;

    let plist_path = plist_dir.join("com.skillsync.watcher.plist");

    // Ensure log directory exists
    let log_dir = skillsync_log_dir()?;
    fs::create_dir_all(&log_dir)
        .with_context(|| t!(Msg::ContextCreateDir { path: log_dir.display().to_string() }))?;

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.skillsync.watcher</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>watch</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{}/watcher.log</string>
    <key>StandardErrorPath</key>
    <string>{}/watcher.err.log</string>
</dict>
</plist>"#,
        binary.display(),
        log_dir.display(),
        log_dir.display()
    );

    fs::write(&plist_path, &plist_content)
        .with_context(|| t!(Msg::ContextCreateDir { path: plist_path.display().to_string() }))?;

    eprintln!(
        "{} {}",
        console::style("[install]").green().bold(),
        t!(Msg::WatchWrotePlist { path: plist_path.display().to_string() })
    );

    // Load the service via launchctl
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if status.success() {
        eprintln!(
            "{} {}",
            console::style("[install]").green().bold(),
            t!(Msg::WatchServiceLoaded)
        );
    } else {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchLaunchctlWarning)
        );
        eprintln!(
            "  {}",
            t!(Msg::WatchLaunchctlHint { path: plist_path.display().to_string() })
        );
    }

    Ok(())
}

/// Generate and enable a Linux systemd user service.
fn install_systemd_service() -> Result<()> {
    let binary = std::env::current_exe().with_context(|| t!(Msg::ContextCurrentDir))?;
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;

    let service_dir = home.join(".config/systemd/user");
    fs::create_dir_all(&service_dir)
        .with_context(|| t!(Msg::ContextCreateDir { path: service_dir.display().to_string() }))?;

    let service_path = service_dir.join("skillsync-watcher.service");

    let service_content = format!(
        r#"[Unit]
Description=SkillSync File Watcher
After=network.target

[Service]
ExecStart={} watch
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target"#,
        binary.display()
    );

    fs::write(&service_path, &service_content)
        .with_context(|| t!(Msg::ContextCreateDir { path: service_path.display().to_string() }))?;

    eprintln!(
        "{} {}",
        console::style("[install]").green().bold(),
        t!(Msg::WatchWroteService { path: service_path.display().to_string() })
    );

    // Reload systemd daemon and enable the service
    let reload_status = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if !reload_status.success() {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchSystemctlReloadFailed)
        );
    }

    let enable_status = Command::new("systemctl")
        .args(["--user", "enable", "--now", "skillsync-watcher.service"])
        .status()
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if enable_status.success() {
        eprintln!(
            "{} {}",
            console::style("[install]").green().bold(),
            t!(Msg::WatchServiceEnabled)
        );
    } else {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchSystemctlEnableFailed)
        );
        eprintln!(
            "  {}",
            t!(Msg::WatchSystemctlHint)
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Service uninstall (7.9)
// ---------------------------------------------------------------------------

/// Uninstall the watcher system service.
fn uninstall_service() -> Result<()> {
    if cfg!(target_os = "macos") {
        uninstall_launchd_service()
    } else if cfg!(target_os = "linux") {
        uninstall_systemd_service()
    } else {
        bail!("{}", t!(Msg::WatchServiceNotSupported));
    }
}

/// Unload and remove the macOS launchd plist.
fn uninstall_launchd_service() -> Result<()> {
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;
    let plist_path = home.join("Library/LaunchAgents/com.skillsync.watcher.plist");

    if !plist_path.exists() {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchNoPlist { path: plist_path.display().to_string() })
        );
        return Ok(());
    }

    // Unload the service
    let status = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&plist_path)
        .status()
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if !status.success() {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchLaunchctlUnloadWarning)
        );
    }

    // Remove the plist file
    fs::remove_file(&plist_path)
        .with_context(|| t!(Msg::ContextCreateDir { path: plist_path.display().to_string() }))?;

    eprintln!(
        "{} {}",
        console::style("[uninstall]").green().bold(),
        t!(Msg::WatchServiceUnloaded { path: plist_path.display().to_string() })
    );

    Ok(())
}

/// Disable and remove the Linux systemd user service.
fn uninstall_systemd_service() -> Result<()> {
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;
    let service_path = home.join(".config/systemd/user/skillsync-watcher.service");

    if !service_path.exists() {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchNoServiceFile { path: service_path.display().to_string() })
        );
        return Ok(());
    }

    // Disable and stop the service
    let status = Command::new("systemctl")
        .args([
            "--user",
            "disable",
            "--now",
            "skillsync-watcher.service",
        ])
        .status()
        .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

    if !status.success() {
        eprintln!(
            "{} {}",
            console::style("Warning:").yellow(),
            t!(Msg::WatchSystemctlDisableWarning)
        );
    }

    // Remove the service file
    fs::remove_file(&service_path)
        .with_context(|| t!(Msg::ContextCreateDir { path: service_path.display().to_string() }))?;

    // Reload daemon to pick up the removal
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    eprintln!(
        "{} {}",
        console::style("[uninstall]").green().bold(),
        t!(Msg::WatchServiceDisabled { path: service_path.display().to_string() })
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the SkillSync log directory (`~/.skillsync/`).
fn skillsync_log_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().with_context(|| t!(Msg::ContextHomeDir))?;
    Ok(home.join(".skillsync"))
}

// ---------------------------------------------------------------------------
// Pause / Resume (5.3, 5.4)
// ---------------------------------------------------------------------------

/// Pause auto-sync by setting auto_sync=false in config.
fn pause_sync() -> Result<()> {
    let mut config = crate::registry::config::GlobalConfig::load()
        .unwrap_or_else(|_| crate::registry::config::GlobalConfig::default());

    if !config.auto_sync {
        eprintln!(
            "{} {}",
            style("ℹ").blue(),
            t!(Msg::WatchAlreadyPaused)
        );
        return Ok(());
    }

    config.set_auto_sync(false);
    config.save()?;

    eprintln!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::WatchPaused)
    );
    Ok(())
}

/// Resume auto-sync by setting auto_sync=true in config.
fn resume_sync() -> Result<()> {
    let mut config = crate::registry::config::GlobalConfig::load()
        .unwrap_or_else(|_| crate::registry::config::GlobalConfig::default());

    if config.auto_sync {
        eprintln!(
            "{} {}",
            style("ℹ").blue(),
            t!(Msg::WatchAlreadyRunning)
        );
        return Ok(());
    }

    config.set_auto_sync(true);
    config.save()?;

    eprintln!(
        "{} {}",
        style("✓").green().bold(),
        t!(Msg::WatchResumed)
    );
    Ok(())
}
