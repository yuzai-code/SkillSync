use anyhow::{Context, Result};
use console::style;

#[allow(unused_imports)]
use crate::t;
use crate::claude::hooks;
use crate::claude::paths::ClaudePaths;
use crate::claude::settings;
use crate::cli::HookAction;
use crate::i18n::Msg;

pub fn run(action: HookAction) -> Result<()> {
    let claude_paths = ClaudePaths::global()
        .with_context(|| t!(Msg::ContextResolvePaths))?;

    let settings_path = &claude_paths.settings_json;

    match action {
        HookAction::Install {} => {
            let mut settings_val = settings::load_settings(settings_path)
                .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

            let added = hooks::install_hook(&mut settings_val);

            if added {
                settings::save_settings(settings_path, &settings_val)
                    .with_context(|| t!(Msg::ContextFailedToSaveManifest))?;

                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    t!(Msg::HookInstalled { path: settings_path.display().to_string() }),
                );
            } else {
                println!(
                    "{} {}",
                    style("·").dim(),
                    t!(Msg::HookAlreadyInstalled),
                );
            }
        }
        HookAction::Remove {} => {
            let mut settings_val = settings::load_settings(settings_path)
                .with_context(|| t!(Msg::ContextFailedToLoadManifest))?;

            let removed = hooks::remove_hook(&mut settings_val);

            if removed {
                settings::save_settings(settings_path, &settings_val)
                    .with_context(|| t!(Msg::ContextFailedToSaveManifest))?;

                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    t!(Msg::HookRemoved { path: settings_path.display().to_string() }),
                );
            } else {
                println!(
                    "{} {}",
                    style("·").dim(),
                    t!(Msg::HookNotFound),
                );
            }
        }
    }

    Ok(())
}
