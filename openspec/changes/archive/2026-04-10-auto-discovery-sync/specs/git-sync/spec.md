## MODIFIED Requirements

### Requirement: Conflict resolution
**FROM:** The system SHALL provide interactive conflict resolution via `skillsync resolve`.

**TO:** The system SHALL automatically resolve conflicts using Last-Write-Wins (LWW) strategy, falling back to interactive resolution only when LWW fails.

#### Scenario: LWW conflict resolution
- **WHEN** user runs `skillsync sync` and merge conflicts occur
- **THEN** system compares local HEAD commit timestamp with FETCH_HEAD timestamp
- **AND** keeps the version with the latest timestamp
- **AND** stages resolved files automatically
- **AND** continues with the sync operation

#### Scenario: LWW with equal timestamps
- **WHEN** local and remote commits have identical timestamps
- **THEN** system keeps the local version (tie-breaker)
- **AND** logs: "Conflict tie-breaker: keeping local version"

#### Scenario: Interactive fallback
- **WHEN** LWW resolution fails or user runs `skillsync resolve`
- **THEN** system provides interactive conflict resolution as before

#### Scenario: Resolve with LWW
- **WHEN** user runs `skillsync sync` and conflicts exist
- **THEN** system compares local and remote commit timestamps
- **AND** keeps the version with the latest timestamp
- **AND** creates a merge commit with resolved content
- **AND** logs: "Conflict resolved: using <local|remote> version of <file> (timestamp: <ts>)"

#### Scenario: LWW with equal timestamps
- **WHEN** local and remote commits have identical timestamps (rare edge case)
- **THEN** system keeps the local version
- **AND** logs: "Conflict tie-breaker: keeping local version"

#### Scenario: LWW on deleted files
- **WHEN** one side deleted a file and the other modified it
- **THEN** the deletion wins (deleted file is removed)
- **AND** logs: "Conflict resolved: <file> deleted (deletion takes precedence)"

## ADDED Requirements

### Requirement: Incremental skills-only push
The system SHALL only push changes to skills directories, not the entire registry.

#### Scenario: Push only changed skills
- **WHEN** auto-sync detects changes in `<project>/.claude/skills/my-tool/`
- **THEN** system stages only the skill files under `resources/skills/my-tool/`
- **AND** does not stage changes to `manifest.yaml` alone
- **AND** does not stage changes to plugins or MCP directories

#### Scenario: Manifest updated automatically
- **WHEN** skill files are staged and committed
- **THEN** system also stages and commits the corresponding manifest.yaml entry update
- **AND** the manifest update is part of the same commit

### Requirement: Manifest entry sync
The system SHALL keep manifest entries synchronized with skill files.

#### Scenario: Sync deleted skill
- **WHEN** a skill is deleted from the filesystem
- **THEN** manifest entry is removed in the same commit
- **AND** git tracks this as a single deletion operation

#### Scenario: Sync new skill
- **WHEN** a new skill is copied to registry
- **THEN** manifest entry is added in the same commit
- **AND** git tracks this as a single addition operation
