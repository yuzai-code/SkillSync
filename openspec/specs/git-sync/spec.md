## ADDED Requirements

### Requirement: Pull remote changes
The system SHALL fetch and apply remote registry changes via `skillsync pull`.

#### Scenario: Pull with no conflicts
- **WHEN** user runs `skillsync pull` and remote has new commits
- **THEN** system performs git pull (fast-forward or merge)
- **AND** applies any new/updated global skills to `~/.claude/skills/`
- **AND** reports summary of changes (N skills updated, M plugins changed)

#### Scenario: Pull with conflicts
- **WHEN** user runs `skillsync pull` and merge conflicts occur
- **THEN** system aborts the merge
- **AND** reports conflicting files
- **AND** instructs user to run `skillsync resolve`

#### Scenario: Pull with quiet mode
- **WHEN** `skillsync pull --quiet` is run (e.g., from hook)
- **THEN** system produces no output on success
- **AND** only outputs errors or conflict warnings

### Requirement: Push local changes
The system SHALL commit and push local registry changes via `skillsync push`.

#### Scenario: Push with changes
- **WHEN** user runs `skillsync push` and local registry has uncommitted changes
- **THEN** system stages all changes in the registry directory
- **AND** creates a commit with descriptive message (e.g., "update: yuque skill v1.0.5")
- **AND** pushes to the configured remote

#### Scenario: Push with nothing to push
- **WHEN** user runs `skillsync push` and no local changes exist
- **THEN** system reports "Nothing to push, registry is up to date"

#### Scenario: Auto push
- **WHEN** `skillsync push --auto` is run (e.g., from file watcher)
- **THEN** system generates an automatic commit message prefixed with "auto:"
- **AND** pushes silently

### Requirement: Bidirectional sync
The system SHALL perform pull then push in sequence via `skillsync sync`.

#### Scenario: Full sync
- **WHEN** user runs `skillsync sync`
- **THEN** system first pulls remote changes (handling conflicts if any)
- **AND** then pushes local changes
- **AND** reports combined summary

### Requirement: Conflict resolution
The system SHALL provide interactive conflict resolution via `skillsync resolve`.

#### Scenario: Resolve with options
- **WHEN** user runs `skillsync resolve` and conflicts exist
- **THEN** system lists each conflicting file
- **AND** for each file, offers: "Open editor", "Keep local", "Use remote", "Side-by-side compare"

#### Scenario: Side-by-side compare
- **WHEN** user selects "Side-by-side compare" for a conflict
- **THEN** system displays local and remote versions in parallel columns with diff highlighting
- **AND** allows user to choose which version to keep or open editor

#### Scenario: After resolution
- **WHEN** all conflicts are resolved
- **THEN** system completes the merge
- **AND** pushes the resolved state to remote

### Requirement: Scan local changes
The system SHALL detect when local Claude Code skill files differ from the registry.

#### Scenario: Detect modified skill
- **WHEN** a skill file in `~/.claude/skills/` or project `.claude/skills/` has been modified outside of skillsync
- **THEN** `skillsync push` SHALL detect the change by comparing content hashes
- **AND** copy the updated file into the registry before committing
