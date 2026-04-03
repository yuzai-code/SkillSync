// Claude Code hook management
// Implements: tasks 7.5, 7.6

use serde_json::{json, Value};

/// The command string that the skillsync hook runs.
const SKILLSYNC_HOOK_COMMAND: &str = "skillsync pull --quiet --timeout 5";

/// Substring used to identify skillsync hooks when searching/removing.
const SKILLSYNC_HOOK_MATCHER: &str = "skillsync pull";

/// Install the skillsync `SessionStart` hook into settings.
///
/// Appends to the existing `hooks.SessionStart` array without overwriting
/// other hooks. Returns `true` if the hook was added, `false` if it was
/// already present.
pub fn install_hook(settings: &mut Value) -> bool {
    if has_hook(settings) {
        return false;
    }

    let obj = settings
        .as_object_mut()
        .expect("settings must be a JSON object");

    // Ensure `hooks` key exists as an object.
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .expect("hooks must be a JSON object");

    // Ensure `SessionStart` key exists as an array.
    let session_start = hooks
        .entry("SessionStart")
        .or_insert_with(|| json!([]));
    let session_start = session_start
        .as_array_mut()
        .expect("SessionStart must be an array");

    // Append our hook entry.
    session_start.push(json!({
        "type": "command",
        "command": SKILLSYNC_HOOK_COMMAND,
    }));

    true
}

/// Remove the skillsync `SessionStart` hook from settings.
///
/// Identifies the hook by checking if the `command` field contains
/// "skillsync pull". Preserves all other hooks. Returns `true` if a hook
/// was removed, `false` if no matching hook was found.
pub fn remove_hook(settings: &mut Value) -> bool {
    let session_start = settings
        .pointer_mut("/hooks/SessionStart")
        .and_then(|v| v.as_array_mut());

    let session_start = match session_start {
        Some(arr) => arr,
        None => return false,
    };

    let original_len = session_start.len();

    session_start.retain(|entry| {
        let cmd = entry
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        !cmd.contains(SKILLSYNC_HOOK_MATCHER)
    });

    session_start.len() != original_len
}

/// Check if the skillsync hook is already installed.
pub fn has_hook(settings: &Value) -> bool {
    let session_start = settings
        .pointer("/hooks/SessionStart")
        .and_then(|v| v.as_array());

    match session_start {
        Some(arr) => arr.iter().any(|entry| {
            entry
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains(SKILLSYNC_HOOK_MATCHER)
        }),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_install_hook_empty_settings() {
        let mut settings = json!({});

        assert!(!has_hook(&settings));
        assert!(install_hook(&mut settings));
        assert!(has_hook(&settings));

        let hooks = settings["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0]["type"], "command");
        assert_eq!(hooks[0]["command"], SKILLSYNC_HOOK_COMMAND);
    }

    #[test]
    fn test_install_hook_preserves_existing() {
        let mut settings = json!({
            "hooks": {
                "SessionStart": [
                    {
                        "type": "command",
                        "command": "some-existing-hook"
                    }
                ]
            }
        });

        assert!(install_hook(&mut settings));

        let hooks = settings["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(hooks.len(), 2);
        assert_eq!(hooks[0]["command"], "some-existing-hook");
        assert_eq!(hooks[1]["command"], SKILLSYNC_HOOK_COMMAND);
    }

    #[test]
    fn test_install_hook_idempotent() {
        let mut settings = json!({});

        assert!(install_hook(&mut settings));
        assert!(!install_hook(&mut settings)); // second call returns false

        let hooks = settings["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(hooks.len(), 1); // not duplicated
    }

    #[test]
    fn test_remove_hook() {
        let mut settings = json!({
            "hooks": {
                "SessionStart": [
                    {
                        "type": "command",
                        "command": "some-existing-hook"
                    },
                    {
                        "type": "command",
                        "command": SKILLSYNC_HOOK_COMMAND
                    }
                ]
            }
        });

        assert!(has_hook(&settings));
        assert!(remove_hook(&mut settings));
        assert!(!has_hook(&settings));

        // The other hook is preserved.
        let hooks = settings["hooks"]["SessionStart"].as_array().unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0]["command"], "some-existing-hook");
    }

    #[test]
    fn test_remove_hook_not_present() {
        let mut settings = json!({
            "hooks": {
                "SessionStart": [
                    {
                        "type": "command",
                        "command": "some-other-hook"
                    }
                ]
            }
        });

        assert!(!remove_hook(&mut settings));
    }

    #[test]
    fn test_remove_hook_empty_settings() {
        let mut settings = json!({});
        assert!(!remove_hook(&mut settings));
    }

    #[test]
    fn test_has_hook_matches_substring() {
        // Even if the command has extra flags, we match on "skillsync pull".
        let settings = json!({
            "hooks": {
                "SessionStart": [
                    {
                        "type": "command",
                        "command": "skillsync pull --quiet --timeout 10"
                    }
                ]
            }
        });

        assert!(has_hook(&settings));
    }

    #[test]
    fn test_has_hook_no_hooks_key() {
        let settings = json!({"enabledPlugins": []});
        assert!(!has_hook(&settings));
    }
}
