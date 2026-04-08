## ADDED Requirements

### Requirement: TUI i18n support
The system SHALL support displaying TUI prompts and option labels in the user's preferred language via the i18n module.

#### Scenario: Chinese TUI prompts
- **WHEN** language is set to `zh` and user runs `skillsync use`
- **THEN** the prompt question SHALL display "请选择配置方式"
- **AND** options SHALL display as "从 profile 开始 — 使用预定义的资源包", "手动选择 — 逐个选择资源", "复制项目 — 复用其他项目的配置"

#### Scenario: English TUI prompts
- **WHEN** language is set to `en` and user runs `skillsync use`
- **THEN** the prompt question SHALL display "How would you like to configure this project?"
- **AND** options SHALL display as "From profile  — start with a predefined bundle", "Manual        — pick resources one by one", "Copy project  — reuse another project's config"

### Requirement: Resource selection i18n
The resource multi-select interface SHALL display prompts and help text via the i18n module.

#### Scenario: Chinese resource selection
- **WHEN** language is set to `zh`
- **THEN** the prompt SHALL display "选择要安装的资源："
- **AND** the help text SHALL display "Type to filter, Space to toggle, Enter to confirm" (clap help remains in English)

### Requirement: Conflict resolution i18n
The conflict resolution interface SHALL display option labels via the i18n module.

#### Scenario: Chinese conflict resolution
- **WHEN** language is set to `zh` and merge conflicts exist
- **THEN** options SHALL display as "保留本地   — 丢弃远程更改", "使用远程   — 丢弃本地更改", "打开编辑器 — 在 $EDITOR 中手动解决"

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
