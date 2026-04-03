# SkillSync

Rust CLI tool for managing and syncing Claude Code skills, plugins, and MCP servers across machines and projects.

SkillSync uses a git-backed registry as the single source of truth, enabling one-command environment restoration, declarative project configuration, and automatic synchronization.

## Installation

### From source

```bash
git clone https://github.com/OWNER/skillsync.git
cd skillsync
cargo install --path .
```

### From release

```bash
curl -fsSL https://raw.githubusercontent.com/OWNER/skillsync/main/install.sh | sh
```

## Quick Start

```bash
# 1. Initialize a new registry
skillsync init

# 2. Add your existing skills to the registry
skillsync add ~/.claude/skills/my-skill

# 3. Add a plugin
skillsync add --plugin superpowers@claude-plugins-official

# 4. Add an MCP server
skillsync add --mcp openspec --command npx --args -y @fission-ai/openspec-mcp

# 5. In a project directory, interactively select resources
skillsync use

# 6. Or install from an existing skillsync.yaml
skillsync install
```

### On a new machine

```bash
# Clone your registry from remote
skillsync init --from git@github.com:you/skillsync-registry.git

# Install global skills and MCP servers
skillsync install --global

# Set up auto-sync hook for Claude Code
skillsync hook install
```

## Commands

### Registry Management

| Command | Description |
|---------|-------------|
| `skillsync init` | Create a new local registry |
| `skillsync init --from <url>` | Clone an existing registry from remote |
| `skillsync add <path>` | Add a local skill directory to the registry |
| `skillsync add --plugin <name>@<marketplace>` | Register a plugin |
| `skillsync add --mcp <name> --command <cmd> [--args ...]` | Register an MCP server |
| `skillsync remove <name>` | Remove a resource from the registry |
| `skillsync update <name>` | Bump a resource's version |
| `skillsync list [--type skill\|plugin\|mcp]` | List all registered resources |
| `skillsync search <query>` | Search resources by name, description, or tags |
| `skillsync info <name>` | Show detailed info for a resource |

### Project Configuration

| Command | Description |
|---------|-------------|
| `skillsync use` | Interactive TUI to configure the current project |
| `skillsync install` | Install resources declared in `.claude/skillsync.yaml` |
| `skillsync install --global` | Install all global-scope resources to `~/.claude/` |

`skillsync use` provides three configuration methods:

- **From profile** -- select a predefined profile, then fine-tune individual resources
- **Manual selection** -- pick resources from the full registry with multi-select
- **Copy from project** -- use another project's configuration as a starting point

### Synchronization

| Command | Description |
|---------|-------------|
| `skillsync pull` | Fetch and merge remote registry changes |
| `skillsync push` | Commit and push local changes to remote |
| `skillsync sync` | Bidirectional sync (pull then push) |
| `skillsync resolve` | Interactively resolve merge conflicts |

```bash
# Quiet pull (for hooks/automation)
skillsync pull --quiet --timeout 5

# Auto-commit push (for watcher daemon)
skillsync push --auto
```

### Profiles

| Command | Description |
|---------|-------------|
| `skillsync profile list` | List all profiles with resource counts |
| `skillsync profile create <name>` | Create a new profile |
| `skillsync profile apply <name>` | Apply a profile to the current project |
| `skillsync profile export <name>` | Export current project config as a profile |

### Auto-Sync

| Command | Description |
|---------|-------------|
| `skillsync watch` | Start file watcher in foreground |
| `skillsync watch --daemon` | Start watcher in background |
| `skillsync watch --install` | Install as system service (launchd/systemd) |
| `skillsync watch --uninstall` | Remove system service |
| `skillsync hook install` | Add SessionStart hook to Claude Code |
| `skillsync hook remove` | Remove the SessionStart hook |

### Diagnostics

```bash
skillsync doctor
```

Checks registry health, manifest validity, git remote, Claude Code installation, and resource consistency.

## Configuration Files

| File | Location | Purpose |
|------|----------|---------|
| `manifest.yaml` | `~/.skillsync/registry/` | Registry manifest -- all resources and profiles |
| `skillsync.yaml` | `<project>/.claude/` | Per-project resource declarations (commit to git) |
| `skillsync.lock` | `<project>/.claude/` | Installed versions and hashes (add to .gitignore) |
| `state.db` | `~/.skillsync/` | SQLite database tracking installations |

### skillsync.yaml example

```yaml
profile: agent-dev
skills:
  - openspec-expert
  - workflow-debugger
plugins:
  - superpowers@claude-plugins-official
mcp:
  - openspec
```

### manifest.yaml example

```yaml
version: 1
skills:
  openspec-expert:
    type: custom
    scope: shared
    version: "1.0.0"
    path: resources/skills/openspec-expert
    description: OpenSpec workflow expertise
    tags:
      - openspec
      - workflow
plugins:
  superpowers:
    marketplace: claude-plugins-official
    version: "72b975468071"
mcp_servers:
  openspec:
    command: npx
    args:
      - "-y"
      - "@fission-ai/openspec-mcp"
    scope: global
profiles:
  agent-dev:
    path: profiles/agent-dev.yaml
```

## Directory Structure

```
~/.skillsync/
  registry/                    # Git-backed registry (local clone)
    manifest.yaml              # Resource manifest
    resources/
      skills/                  # Skill source files
      plugins/                 # Plugin backups
      mcp/                     # MCP server configs
    profiles/                  # Profile YAML files
  state.db                     # SQLite state database

~/.claude/
  skills/                      # Global skills (managed by skillsync)
  settings.json                # Claude Code settings (hooks, plugins)
  .mcp.json                    # Global MCP server config

<project>/
  .claude/
    skills/                    # Project skills (installed by skillsync)
    skillsync.yaml             # Project resource declarations
    skillsync.lock             # Installed versions (gitignored)
  .mcp.json                   # Project MCP config
```

## Auto-Sync Architecture

SkillSync provides three layers of synchronization:

```
Layer 1: Claude Code Hook (SessionStart)
  skillsync pull --quiet --timeout 5
  Every time Claude Code starts, it pulls remote updates.

Layer 2: File Watcher Daemon (optional)
  Monitors ~/.claude/skills/ for changes.
  Auto-commits and pushes to registry on file changes.
  Debounced at 2 seconds.

Layer 3: Manual sync (fallback)
  skillsync sync
```

Setup:

```bash
# Install the SessionStart hook
skillsync hook install

# Optionally install the file watcher as a system service
skillsync watch --install
```

## Global Flags

| Flag | Description |
|------|-------------|
| `-q, --quiet` | Suppress output |
| `-v, --verbose` | Verbose output |
| `--dry-run` | Preview changes without applying |

## Building from Source

```bash
cargo build --release       # Build optimized binary
cargo test                  # Run all 125 tests
cargo clippy                # Lint
cargo fmt                   # Format
```

Requirements: Rust toolchain, cmake or pkg-config (for libgit2).

## License

MIT
