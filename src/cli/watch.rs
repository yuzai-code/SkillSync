// Watch command — file watcher daemon, system service install/uninstall
// Implements: tasks 7.4, 7.7, 7.8, 7.9

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::watcher::fs_watcher;

pub fn run(daemon: bool, install: bool, uninstall: bool) -> Result<()> {
    if install {
        return install_service();
    }

    if uninstall {
        return uninstall_service();
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
        bail!(
            "No directories to watch. Run 'skillsync init' to initialize the registry first."
        );
    }

    eprintln!(
        "{}",
        console::style("Starting SkillSync file watcher (foreground)...").bold()
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
    let binary = std::env::current_exe().context("Failed to determine current executable path")?;

    // Ensure log directory exists
    let log_dir = skillsync_log_dir()?;
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

    let stdout_log = log_dir.join("watcher.log");
    let stderr_log = log_dir.join("watcher.err.log");

    let stdout_file = fs::File::create(&stdout_log)
        .with_context(|| format!("Failed to create log file: {}", stdout_log.display()))?;
    let stderr_file = fs::File::create(&stderr_log)
        .with_context(|| format!("Failed to create log file: {}", stderr_log.display()))?;

    let child = Command::new(&binary)
        .arg("watch")
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn();

    match child {
        Ok(child) => {
            eprintln!(
                "{} Watcher daemon started (PID: {})",
                console::style("[daemon]").green().bold(),
                child.id()
            );
            eprintln!("  Logs: {}", stdout_log.display());
            eprintln!("  Errors: {}", stderr_log.display());
            eprintln!(
                "  Stop with: {} or {}",
                console::style("kill").yellow(),
                console::style("skillsync watch --uninstall").yellow()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!(
                "{} Failed to start daemon: {}",
                console::style("Error:").red(),
                e
            );
            eprintln!(
                "  Try running manually: {} {} {}",
                console::style("nohup").yellow(),
                binary.display(),
                console::style("watch &").yellow()
            );
            bail!("Failed to start watcher daemon: {}", e);
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
        bail!(
            "System service installation is not supported on this platform. \
             Use `skillsync watch --daemon` instead."
        );
    }
}

/// Generate and load a macOS launchd plist.
fn install_launchd_service() -> Result<()> {
    let binary = std::env::current_exe().context("Failed to determine current executable path")?;
    let home = dirs::home_dir().context("Could not determine home directory")?;

    let plist_dir = home.join("Library/LaunchAgents");
    fs::create_dir_all(&plist_dir)
        .with_context(|| format!("Failed to create {}", plist_dir.display()))?;

    let plist_path = plist_dir.join("com.skillsync.watcher.plist");

    // Ensure log directory exists
    let log_dir = skillsync_log_dir()?;
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

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
        .with_context(|| format!("Failed to write plist to {}", plist_path.display()))?;

    eprintln!(
        "{} Wrote plist: {}",
        console::style("[install]").green().bold(),
        plist_path.display()
    );

    // Load the service via launchctl
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .context("Failed to run launchctl")?;

    if status.success() {
        eprintln!(
            "{} Service loaded. The watcher will start automatically on login.",
            console::style("[install]").green().bold()
        );
    } else {
        eprintln!(
            "{} launchctl load returned non-zero exit code. \
             The plist has been written but the service may not be running.",
            console::style("Warning:").yellow()
        );
        eprintln!(
            "  Try manually: launchctl load -w {}",
            plist_path.display()
        );
    }

    Ok(())
}

/// Generate and enable a Linux systemd user service.
fn install_systemd_service() -> Result<()> {
    let binary = std::env::current_exe().context("Failed to determine current executable path")?;
    let home = dirs::home_dir().context("Could not determine home directory")?;

    let service_dir = home.join(".config/systemd/user");
    fs::create_dir_all(&service_dir)
        .with_context(|| format!("Failed to create {}", service_dir.display()))?;

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
        .with_context(|| format!("Failed to write service file to {}", service_path.display()))?;

    eprintln!(
        "{} Wrote service file: {}",
        console::style("[install]").green().bold(),
        service_path.display()
    );

    // Reload systemd daemon and enable the service
    let reload_status = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .context("Failed to run systemctl daemon-reload")?;

    if !reload_status.success() {
        eprintln!(
            "{} systemctl daemon-reload failed.",
            console::style("Warning:").yellow()
        );
    }

    let enable_status = Command::new("systemctl")
        .args(["--user", "enable", "--now", "skillsync-watcher.service"])
        .status()
        .context("Failed to run systemctl enable")?;

    if enable_status.success() {
        eprintln!(
            "{} Service enabled and started. The watcher will start automatically on login.",
            console::style("[install]").green().bold()
        );
    } else {
        eprintln!(
            "{} systemctl enable returned non-zero exit code.",
            console::style("Warning:").yellow()
        );
        eprintln!(
            "  Try manually: systemctl --user enable --now skillsync-watcher.service"
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
        bail!(
            "System service uninstallation is not supported on this platform. \
             Use 'skillsync watch --daemon' to run the watcher manually."
        );
    }
}

/// Unload and remove the macOS launchd plist.
fn uninstall_launchd_service() -> Result<()> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let plist_path = home.join("Library/LaunchAgents/com.skillsync.watcher.plist");

    if !plist_path.exists() {
        eprintln!(
            "{} No plist found at {}. Service may not be installed.",
            console::style("Warning:").yellow(),
            plist_path.display()
        );
        return Ok(());
    }

    // Unload the service
    let status = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&plist_path)
        .status()
        .context("Failed to run launchctl unload")?;

    if !status.success() {
        eprintln!(
            "{} launchctl unload returned non-zero exit code. Continuing with file removal.",
            console::style("Warning:").yellow()
        );
    }

    // Remove the plist file
    fs::remove_file(&plist_path)
        .with_context(|| format!("Failed to remove plist: {}", plist_path.display()))?;

    eprintln!(
        "{} Service unloaded and plist removed: {}",
        console::style("[uninstall]").green().bold(),
        plist_path.display()
    );

    Ok(())
}

/// Disable and remove the Linux systemd user service.
fn uninstall_systemd_service() -> Result<()> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let service_path = home.join(".config/systemd/user/skillsync-watcher.service");

    if !service_path.exists() {
        eprintln!(
            "{} No service file found at {}. Service may not be installed.",
            console::style("Warning:").yellow(),
            service_path.display()
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
        .context("Failed to run systemctl disable")?;

    if !status.success() {
        eprintln!(
            "{} systemctl disable returned non-zero exit code. Continuing with file removal.",
            console::style("Warning:").yellow()
        );
    }

    // Remove the service file
    fs::remove_file(&service_path)
        .with_context(|| format!("Failed to remove service file: {}", service_path.display()))?;

    // Reload daemon to pick up the removal
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    eprintln!(
        "{} Service disabled and removed: {}",
        console::style("[uninstall]").green().bold(),
        service_path.display()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the SkillSync log directory (`~/.skillsync/`).
fn skillsync_log_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".skillsync"))
}
