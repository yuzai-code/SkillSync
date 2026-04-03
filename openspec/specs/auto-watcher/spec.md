## ADDED Requirements

### Requirement: File watcher daemon
The system SHALL provide a background daemon via `skillsync watch` that monitors skill directories for changes and auto-syncs.

#### Scenario: Start watcher
- **WHEN** user runs `skillsync watch`
- **THEN** system starts monitoring `~/.claude/skills/` and all registered project skill directories
- **AND** runs in the foreground (or background with `--daemon`)

#### Scenario: Detect and auto-push
- **WHEN** a file change is detected in a monitored skill directory
- **THEN** system waits for a debounce period (2 seconds of no further changes)
- **AND** copies updated files to the registry
- **AND** runs `skillsync push --auto`

#### Scenario: Watcher error recovery
- **WHEN** auto-push fails (network error, conflict)
- **THEN** system logs the error
- **AND** retries on the next detected change
- **AND** does NOT crash the daemon

### Requirement: Claude Code hook integration
The system SHALL install a SessionStart hook in Claude Code to auto-pull on session start.

#### Scenario: Install hooks
- **WHEN** user runs `skillsync hook install`
- **THEN** system adds a SessionStart hook to `~/.claude/settings.json` that runs `skillsync pull --quiet --timeout 5`
- **AND** preserves existing hooks in the configuration

#### Scenario: Remove hooks
- **WHEN** user runs `skillsync hook remove`
- **THEN** system removes the skillsync-managed hook from `~/.claude/settings.json`
- **AND** does NOT affect other hooks

### Requirement: System service installation
The system SHALL support installing the watcher as a system service.

#### Scenario: Install as macOS launchd service
- **WHEN** user runs `skillsync watch --install` on macOS
- **THEN** system creates `~/Library/LaunchAgents/com.skillsync.watcher.plist`
- **AND** loads the service via `launchctl`
- **AND** the service starts automatically on login

#### Scenario: Install as Linux systemd service
- **WHEN** user runs `skillsync watch --install` on Linux
- **THEN** system creates `~/.config/systemd/user/skillsync-watcher.service`
- **AND** enables and starts the service via `systemctl --user`

#### Scenario: Uninstall service
- **WHEN** user runs `skillsync watch --uninstall`
- **THEN** system stops and removes the platform-specific service
