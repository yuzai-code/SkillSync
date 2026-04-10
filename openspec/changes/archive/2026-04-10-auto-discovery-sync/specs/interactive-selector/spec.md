## MODIFIED Requirements

### Requirement: Interactive project configuration
**FROM:** The system SHALL provide an interactive TUI via `skillsync use` that guides users through selecting resources for the current project.

**TO:** The system SHALL provide an interactive TUI via `skillsync use` and `skillsync sync` that guides users through selecting and installing remote-discovered skills.

#### Scenario: Interactive resource selection via use command
- **WHEN** user runs `skillsync use`
- **THEN** system displays a TUI for selecting resources from the registry
- **AND** user can choose configuration method: profile, manual, or copy from project
- **AND** system generates skillsync.yaml based on selections

## ADDED Requirements

### Requirement: Remote skills selection
The system SHALL display newly discovered remote skills after a successful sync.

#### Scenario: Show remote new skills after sync
- **WHEN** user runs `skillsync sync` and remote has new skills
- **THEN** system displays a TUI after sync listing all new remote skills
- **AND** each skill shows: name, source project, description (if available)
- **AND** user can multi-select which skills to install

#### Scenario: Select install scope
- **WHEN** user selects one or more skills to install
- **THEN** system prompts: "安装到全局还是项目？"
- **AND** user can choose: "全局 (~/.claude/skills/)" or "选择项目"
- **IF** "选择项目" is selected
- **THEN** system shows a list of known projects for installation target

#### Scenario: Skip remote selection
- **WHEN** user runs `skillsync sync --skip-select`
- **THEN** system performs sync without showing the remote selection TUI
- **AND** logs what was synced without prompting

### Requirement: Display skill source info
The system SHALL show which project a discovered skill came from.

#### Scenario: Show source project
- **WHEN** displaying a remotely discovered skill
- **THEN** system shows: "skill-name (来自 project-X)"
- **AND** shows the skill's content hash for verification

### Requirement: Confirm before install
The system SHALL show a dry-run preview before installing remote skills.

#### Scenario: Preview install
- **WHEN** user confirms skill selection
- **THEN** system shows: skill name, target path, files to be installed
- **AND** waits for final confirmation before copying files
