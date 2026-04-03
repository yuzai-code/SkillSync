## ADDED Requirements

### Requirement: Install skills to project
The system SHALL physically copy skill source files from the registry to the project's `.claude/skills/` directory.

#### Scenario: Install declared skills
- **WHEN** `skillsync use` completes or `skillsync install` is run in a project with `skillsync.yaml`
- **THEN** system copies each declared skill's source files from `~/.skillsync/registry/resources/skills/<name>/` to `<project>/.claude/skills/<name>/`
- **AND** creates the `.claude/skills/` directory if it does not exist

#### Scenario: Skill already exists in project
- **WHEN** a skill being installed already exists in the project's `.claude/skills/`
- **THEN** system compares content hashes
- **AND** if different, overwrites with the registry version and logs the update
- **AND** if identical, skips with no output

### Requirement: Install global skills
The system SHALL install global-scope skills to `~/.claude/skills/`.

#### Scenario: Install global skills on init
- **WHEN** `skillsync init --from <url>` or `skillsync install --global` is run
- **THEN** system copies all global-scope skills from the registry to `~/.claude/skills/`

### Requirement: Configure MCP servers
The system SHALL merge MCP server declarations into the target `.mcp.json` file.

#### Scenario: Merge MCP config for project
- **WHEN** project declares MCP servers in `skillsync.yaml`
- **THEN** system reads existing `<project>/.mcp.json` (or creates it)
- **AND** merges declared MCP server entries into the `mcpServers` object
- **AND** does NOT remove existing MCP entries not managed by skillsync

#### Scenario: Merge global MCP config
- **WHEN** manifest declares global-scope MCP servers
- **THEN** system merges them into `~/.claude/.mcp.json`

### Requirement: Generate skillsync.yaml
The system SHALL write a `skillsync.yaml` declaration file after `skillsync use` completes.

#### Scenario: Create skillsync.yaml
- **WHEN** `skillsync use` confirms and applies a configuration
- **THEN** system writes `.claude/skillsync.yaml` listing all selected skills, plugins, and MCP servers
- **AND** includes the profile name if one was used as the base

### Requirement: Generate skillsync.lock
The system SHALL write a `.claude/skillsync.lock` file recording exact installed versions and hashes.

#### Scenario: Lock file creation
- **WHEN** resources are installed to a project
- **THEN** system writes `.claude/skillsync.lock` with each resource's name, version, and content hash
- **AND** this file SHALL NOT be committed to git (user should add to .gitignore)

### Requirement: Plugin enablement
The system SHALL enable declared plugins in Claude Code settings.

#### Scenario: Enable plugins
- **WHEN** project or global config declares plugins
- **THEN** system updates `~/.claude/settings.json` `enabledPlugins` to include declared plugins
- **AND** adds marketplace entries to `extraKnownMarketplaces` if needed
- **AND** if plugin is not installed locally, attempts `claude plugins install <name>` and reports result

### Requirement: Doctor diagnostics
The system SHALL provide `skillsync doctor` to verify environment health.

#### Scenario: Run doctor
- **WHEN** user runs `skillsync doctor`
- **THEN** system checks: registry exists, git remote reachable, Claude Code installed, skills directories valid, manifest parseable, no orphaned resources
- **AND** reports issues with suggested fixes
