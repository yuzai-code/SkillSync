## ADDED Requirements

### Requirement: Auto-discover global plugins
The system SHALL scan `~/.claude/settings.json` for enabled plugins during `skillsync sync`.

#### Scenario: Scan enabled plugins
- **WHEN** user runs `skillsync sync`
- **THEN** system reads `enabledPlugins` from `~/.claude/settings.json`
- **AND** extracts plugin name and marketplace from format `name@marketplace`
- **AND** reads marketplace source from `extraKnownMarketplaces` if available
- **AND** registers each plugin in manifest.yaml with source_path set to "global"

#### Scenario: Plugin without marketplace info
- **WHEN** a plugin in `enabledPlugins` has no corresponding entry in `extraKnownMarketplaces`
- **THEN** system registers the plugin with marketplace extracted from the plugin key
- **AND** sets source to "unknown" if marketplace cannot be determined

### Requirement: Auto-discover global MCP servers
The system SHALL scan `~/.claude/.mcp.json` for MCP server configurations during `skillsync sync`.

#### Scenario: Scan MCP servers
- **WHEN** user runs `skillsync sync`
- **THEN** system reads `mcpServers` from `~/.claude/.mcp.json`
- **AND** registers each MCP server in manifest.yaml with command, args, and scope
- **AND** sets source_path to "global"

#### Scenario: MCP config file missing
- **WHEN** `~/.claude/.mcp.json` does not exist
- **THEN** system skips MCP discovery without error

### Requirement: Display resource source in list
The system SHALL display the source location for all resource types in `skillsync list`.

#### Scenario: List with source column
- **WHEN** user runs `skillsync list`
- **THEN** system displays a "来源" (source) column
- **AND** shows "global" for resources from `~/.claude/`
- **AND** shows "project: <name>" for resources from project directories
- **AND** shows "marketplace" for plugins installed from marketplace
- **AND** shows "config" for MCP servers from config file

### Requirement: Display resource source in use
The system SHALL display the source location during resource selection in `skillsync use`.

#### Scenario: Select skills with source
- **WHEN** user runs `skillsync use` and selects resources
- **THEN** system displays source in the selection list (e.g., `[skill] name (shared) [global]`)
- **AND** displays source in the confirmation preview