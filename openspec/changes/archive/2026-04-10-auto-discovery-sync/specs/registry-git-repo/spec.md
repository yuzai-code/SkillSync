## ADDED Requirements

### Requirement: Registry as git bare repo
The system SHALL store the registry as a bare git repository at `~/.skillsync/registry.git/`.

#### Scenario: Init new registry with git
- **WHEN** user runs `skillsync init` without `--from` flag
- **THEN** system creates `~/.skillsync/registry.git/` as a bare git repository
- **AND** creates working directory structure at `~/.skillsync/registry/` linked to the bare repo
- **AND** the registry directory is a git working tree pointing to registry.git

#### Scenario: Init from remote registry
- **WHEN** user runs `skillsync init --from <git-url>`
- **THEN** system clones `<git-url>` to `~/.skillsync/registry.git/` as bare repo
- **AND** checks out working tree to `~/.skillsync/registry/`

#### Scenario: Registry structure in git
- **THEN** the registry git repo contains:
  - `manifest.yaml` — skill/plugin/mcp declarations
  - `resources/skills/` — skill source files
  - `resources/plugins/` — plugin files
  - `resources/mcp/` — MCP server configs
  - `profiles/` — profile definitions

### Requirement: Auto-push to remote
The system SHALL push registry changes to configured remote on auto-sync.

#### Scenario: Configure remote
- **WHEN** user runs `skillsync remote add origin <git-url>`
- **THEN** system adds the remote to `~/.skillsync/registry.git/`
- **AND** stores the remote URL in `~/.skillsync/config.yaml`

#### Scenario: Push on change
- **WHEN** auto-sync commits a change
- **THEN** system pushes to the configured remote
- **AND** uses `git push origin main` (or current branch)

### Requirement: Pull remote changes
The system SHALL pull remote registry changes on `sync`.

#### Scenario: Pull during sync
- **WHEN** user runs `skillsync sync`
- **THEN** system fetches from remote
- **AND** merges/fast-forwards to local
- **AND** updates working tree in `~/.skillsync/registry/`

#### Scenario: Pull with conflicts (LWW)
- **WHEN** user runs `skillsync sync` and conflicts exist
- **THEN** system uses Last-Write-Wins strategy
- **AND** keeps the version with the latest commit timestamp
- **AND** logs conflict resolution details

### Requirement: Local-only registry (no remote)
The system SHALL work fully without a remote if none is configured.

#### Scenario: No remote configured
- **WHEN** no remote is configured in registry.git
- **THEN** auto-sync commits locally but does not push
- **AND** sync between machines requires manual `git clone`/`git push`/`git pull`
- **AND** user is notified of this limitation on init if no remote given
