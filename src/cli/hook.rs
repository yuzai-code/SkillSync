use anyhow::{Context, Result};
use console::style;

use crate::claude::hooks;
use crate::claude::paths::ClaudePaths;
use crate::claude::settings;
use crate::cli::HookAction;

pub fn run(action: HookAction) -> Result<()> {
    let claude_paths = ClaudePaths::global()
        .context("Failed to discover Claude Code paths")?;

    let settings_path = &claude_paths.settings_json;

    match action {
        HookAction::Install {} => {
            let mut settings_val = settings::load_settings(settings_path)
                .context("Failed to load Claude Code settings")?;

            let added = hooks::install_hook(&mut settings_val);

            if added {
                settings::save_settings(settings_path, &settings_val)
                    .context("Failed to save Claude Code settings")?;

                println!(
                    "{} Installed SessionStart hook into {}",
                    style("✓").green().bold(),
                    style(settings_path.display().to_string()).cyan(),
                );
            } else {
                println!(
                    "{} SkillSync hook is already installed",
                    style("·").dim(),
                );
            }
        }
        HookAction::Remove {} => {
            let mut settings_val = settings::load_settings(settings_path)
                .context("Failed to load Claude Code settings")?;

            let removed = hooks::remove_hook(&mut settings_val);

            if removed {
                settings::save_settings(settings_path, &settings_val)
                    .context("Failed to save Claude Code settings")?;

                println!(
                    "{} Removed SkillSync hook from {}",
                    style("✓").green().bold(),
                    style(settings_path.display().to_string()).cyan(),
                );
            } else {
                println!(
                    "{} No SkillSync hook found to remove",
                    style("·").dim(),
                );
            }
        }
    }

    Ok(())
}
