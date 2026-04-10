## ADDED Requirements

### Requirement: Scan projects for skills
The system SHALL scan `~/projects/*/.claude/skills/` directories to discover locally created skills.

#### Scenario: Scan during init
- **WHEN** user runs `skillsync init` and the registry is new
- **THEN** system scans `~/projects/*/.claude/skills/` for subdirectories
- **AND** for each skill found, generates a manifest entry with type=custom, scope=shared
- **AND** records the original project path as `source_path`

#### Scenario: Scan during sync
- **WHEN** user runs `skillsync sync`
- **THEN** system re-scans `~/projects/*/.claude/skills/` for newly created or deleted skills
- **AND** updates manifest entries accordingly

#### Scenario: Skip non-git directories
- **WHEN** scanning ~/projects/
- **THEN** system skips any project directory that is not a git repository
- **AND** only processes `.claude/skills/` within git-tracked project directories

#### Scenario: Deduplicate by content hash
- **WHEN** the same skill (by content hash) exists in multiple projects
- **THEN** system SHALL treat them as one skill with multiple `source_path` entries
- **AND** manifest entry contains a list of all source paths

### Requirement: Persist discovered projects
The system SHALL record discovered project paths in the state database to avoid repeated scanning.

#### Scenario: Store discovered project
- **WHEN** a new project with `.claude/skills/` is discovered
- **THEN** system inserts a row into `discovered_projects` table
- **AND** records project_path, first_discovered_at, last_scanned_at

#### Scenario: Update scan timestamp
- **WHEN** an already-discovered project is re-scanned
- **THEN** system updates the `last_scanned_at` timestamp
- **AND** does not re-process unchanged skills

#### Scenario: Remove deleted project
- **WHEN** a previously discovered project's `.claude/skills/` directory no longer exists
- **THEN** system marks the project as `removed` in the database
- **AND** retains the record for audit purposes

### Requirement: Manifest entry generation
The system SHALL generate manifest entries for discovered skills with correct metadata.

#### Scenario: Generate skill entry
- **WHEN** a skill directory is discovered at `~/projects/project-X/.claude/skills/my-tool/`
- **THEN** system creates manifest entry:
  ```
  skills:
    my-tool:
      type: custom
      scope: shared
      version: "1.0.0"
      path: resources/skills/my-tool  (copied to registry)
      description: "" (empty, auto-filled later)
      source_path: ~/projects/project-X/.claude/skills/my-tool
  ```

#### Scenario: Copy skill files to registry
- **WHEN** a skill is discovered from a project
- **THEN** system copies the skill directory to `~/.skillsync/registry/resources/skills/<name>/`
- **AND** computes SHA-256 content hash for the entry
