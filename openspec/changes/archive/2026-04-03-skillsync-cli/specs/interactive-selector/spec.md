## ADDED Requirements

### Requirement: Interactive project configuration
The system SHALL provide an interactive TUI via `skillsync use` that guides users through selecting resources for the current project.

#### Scenario: Choose configuration method
- **WHEN** user runs `skillsync use` in a project directory
- **THEN** system presents three options: "From profile", "Manual selection", "Copy from another project"

#### Scenario: Configure from profile
- **WHEN** user selects "From profile"
- **THEN** system displays available profiles with name, description, and resource count
- **AND** after profile selection, shows a multi-select screen listing all profile resources (pre-checked) plus additional available resources (unchecked)
- **AND** user can toggle individual resources before confirming

#### Scenario: Manual selection
- **WHEN** user selects "Manual selection"
- **THEN** system displays all available resources grouped by category (global skills, shared skills, plugins, MCP servers)
- **AND** each resource shows name, type, scope, and description
- **AND** user can toggle resources with spacebar and confirm with Enter

#### Scenario: Copy from another project
- **WHEN** user selects "Copy from another project"
- **THEN** system lists projects that have a `.claude/skillsync.yaml`
- **AND** after project selection, loads that project's configuration as the starting point for further adjustment

### Requirement: Search and filter in selector
The system SHALL support type-ahead search filtering in all multi-select screens.

#### Scenario: Filter resources by keyword
- **WHEN** user starts typing while in a multi-select screen
- **THEN** the visible options are filtered to match the typed keyword against resource name, description, and tags

### Requirement: Dry-run preview
The system SHALL show a summary of changes before applying any configuration.

#### Scenario: Preview before apply
- **WHEN** user confirms their selection in `skillsync use`
- **THEN** system displays a summary showing: skills to install, plugins to enable, MCP servers to configure
- **AND** waits for final confirmation before making changes
