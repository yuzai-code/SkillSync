## ADDED Requirements

### Requirement: Watch project skill directories
The system SHALL monitor all discovered project `.claude/skills/` directories for file changes.

#### Scenario: Watch discovered projects
- **WHEN** the watcher daemon starts
- **THEN** system loads all project paths from `discovered_projects` table
- **AND** monitors each `<project>/.claude/skills/` directory recursively

#### Scenario: Watch global skills
- **WHEN** the watcher daemon starts
- **THEN** system monitors `~/.claude/skills/` in addition to project directories

#### Scenario: Ignore non-skill files
- **WHEN** a file change is detected outside of skill subdirectories
- **THEN** system ignores the event if the changed path does not contain a valid skill structure

### Requirement: Auto-commit on change detection
The system SHALL automatically commit skill changes with an "auto:" prefixed message.

#### Scenario: Auto-commit with changes
- **WHEN** a skill file change is detected and debounce period (5 seconds) has elapsed
- **THEN** system stages only the changed skill files (not entire registry)
- **AND** creates a commit with message format: "auto-sync: <skill-name> updated (<count> file(s))"

#### Scenario: Auto-commit on skill deletion
- **WHEN** a skill directory is deleted
- **THEN** system stages the removal
- **AND** creates a commit with message: "auto-sync: <skill-name> deleted"

#### Scenario: Auto-commit on new skill
- **WHEN** a new skill directory appears in a watched location
- **THEN** system copies it to registry
- **AND** creates a commit with message: "auto-sync: <skill-name> added"

#### Scenario: No commit when no changes
- **WHEN** debounce period elapses but no actual file content changed
- **THEN** system does not create a commit
- **AND** logs "No changes detected"

### Requirement: Silent background push
The system SHALL push commits to remote registry without user interaction or output.

#### Scenario: Silent push success
- **WHEN** auto-commit succeeds and remote is configured
- **THEN** system pushes silently to `origin`
- **AND** logs are written to `~/.skillsync/watcher.log` only

#### Scenario: Silent push failure
- **WHEN** auto-commit succeeds but push fails (network error)
- **THEN** system logs the error to watcher log
- **AND** retries on next change detection
- **AND** does NOT interrupt the watcher process

#### Scenario: No remote configured
- **WHEN** auto-commit succeeds but no remote is configured
- **THEN** system logs a warning
- **AND** continues watching without pushing
- **AND** does NOT error out

### Requirement: Configurable enable/disable
The system SHALL provide a way to enable or disable auto-sync via configuration file.

#### Scenario: Disable auto-sync
- **WHEN** `~/.skillsync/config.yaml` contains `auto_sync: false`
- **THEN** watcher detects changes but does NOT auto-commit or push
- **AND** logs "Auto-sync disabled, skipping"

#### Scenario: Enable auto-sync
- **WHEN** `~/.skillsync/config.yaml` contains `auto_sync: true` or is absent (default)
- **THEN** watcher behaves as specified above

#### Scenario: Runtime toggle
- **WHEN** user runs `skillsync watch --pause`
- **THEN** watcher stops processing events but keeps monitoring
- **WHEN** user runs `skillsync watch --resume`
- **THEN** watcher resumes processing events
