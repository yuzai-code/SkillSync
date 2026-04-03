## ADDED Requirements

### Requirement: Initialize new registry
The system SHALL create a new git-backed registry at `~/.skillsync/registry/` with a valid `manifest.yaml` and `resources/` directory structure when the user runs `skillsync init`.

#### Scenario: Init with new registry
- **WHEN** user runs `skillsync init` and chooses "create new registry"
- **THEN** system creates `~/.skillsync/registry/` with scaffolded structure (manifest.yaml, resources/skills/, resources/plugins/, resources/mcp/, profiles/)
- **AND** initializes a git repository in the registry directory

#### Scenario: Init from remote
- **WHEN** user runs `skillsync init --from <git-url>`
- **THEN** system clones the remote repository to `~/.skillsync/registry/`
- **AND** validates that manifest.yaml exists and is parseable
- **AND** installs global resources declared in the manifest

#### Scenario: Init with auto-import
- **WHEN** user runs `skillsync init` and existing Claude Code skills/plugins/MCP configs are detected
- **THEN** system prompts user to import discovered resources into the registry
- **AND** copies selected skill source files to `resources/skills/`
- **AND** records plugin declarations in `resources/plugins/`
- **AND** records MCP server configs in `resources/mcp/`
- **AND** generates manifest.yaml entries for all imported resources

### Requirement: Add resource to registry
The system SHALL support adding skills, plugins, and MCP servers to the registry via `skillsync add`.

#### Scenario: Add a local skill
- **WHEN** user runs `skillsync add <path-to-skill-directory>`
- **THEN** system copies the skill source to `resources/skills/<name>/`
- **AND** updates manifest.yaml with the skill's metadata (name, type, scope, version, description)
- **AND** computes and stores the content hash

#### Scenario: Add a community plugin
- **WHEN** user runs `skillsync add --plugin <name>@<marketplace>`
- **THEN** system records the plugin declaration in manifest.yaml (name, marketplace, version, git_sha)
- **AND** backs up the plugin source to `resources/plugins/<name>/` if accessible

#### Scenario: Add an MCP server
- **WHEN** user runs `skillsync add --mcp <name> --command <cmd> --args <args>`
- **THEN** system records the MCP server config in manifest.yaml and `resources/mcp/<name>.json`

### Requirement: Remove resource from registry
The system SHALL support removing resources via `skillsync remove <name>`.

#### Scenario: Remove a skill
- **WHEN** user runs `skillsync remove <skill-name>`
- **THEN** system removes the skill entry from manifest.yaml
- **AND** deletes the skill source from `resources/skills/<name>/`
- **AND** warns if the skill is referenced by any profile or project skillsync.yaml

### Requirement: List registry contents
The system SHALL display all registered resources via `skillsync list`.

#### Scenario: List all resources
- **WHEN** user runs `skillsync list`
- **THEN** system displays a formatted table of all skills, plugins, and MCP servers
- **AND** shows name, type (custom/community), scope (global/shared), version, and usage count

#### Scenario: List filtered by type
- **WHEN** user runs `skillsync list --type skill`
- **THEN** system displays only skill resources

### Requirement: Show resource details
The system SHALL display detailed info for a specific resource via `skillsync info <name>`.

#### Scenario: Show skill info
- **WHEN** user runs `skillsync info <skill-name>`
- **THEN** system displays the skill's metadata, source path, content hash, which projects use it, and its profile memberships

### Requirement: Manifest schema validation
The system SHALL validate manifest.yaml against a defined schema on every read/write operation.

#### Scenario: Invalid manifest
- **WHEN** manifest.yaml contains invalid structure or missing required fields
- **THEN** system reports a clear error message indicating the validation failure
- **AND** refuses to proceed with the operation
