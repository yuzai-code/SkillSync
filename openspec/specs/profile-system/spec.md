## ADDED Requirements

### Requirement: List profiles
The system SHALL display all available profiles via `skillsync profile list`.

#### Scenario: List profiles
- **WHEN** user runs `skillsync profile list`
- **THEN** system displays a table of profiles with name, description, and resource counts (skills, plugins, MCP servers)

### Requirement: Create profile
The system SHALL support creating new profiles via `skillsync profile create`.

#### Scenario: Create profile interactively
- **WHEN** user runs `skillsync profile create <name>`
- **THEN** system prompts for description
- **AND** presents multi-select of all registry resources to include
- **AND** writes the profile to `profiles/<name>.yaml` in the registry

### Requirement: Apply profile to project
The system SHALL apply a profile's configuration to the current project via `skillsync profile apply`.

#### Scenario: Apply profile
- **WHEN** user runs `skillsync profile apply <name>` in a project directory
- **THEN** system installs all resources declared in the profile
- **AND** generates `.claude/skillsync.yaml` with the profile name and resource list
- **AND** this is equivalent to running `skillsync use` and selecting the profile

### Requirement: Export project config as profile
The system SHALL support exporting the current project's configuration as a reusable profile.

#### Scenario: Export to profile
- **WHEN** user runs `skillsync profile export <name>` in a project with `.claude/skillsync.yaml`
- **THEN** system reads the project's skillsync.yaml
- **AND** prompts for a description
- **AND** writes `profiles/<name>.yaml` to the registry with the project's resource declarations

### Requirement: Profile YAML schema
Profiles SHALL use a defined YAML structure.

#### Scenario: Valid profile structure
- **WHEN** a profile is loaded
- **THEN** it SHALL contain: name (string), description (string), and at least one of: skills (array), plugins (array), mcp (array)
- **AND** skill entries reference registry resource names
- **AND** plugin entries use `<name>@<marketplace>` format
- **AND** MCP entries reference registry MCP server names
