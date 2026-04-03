# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SkillSync is a Rust CLI tool for managing and syncing Claude Code skills, plugins, and MCP servers across machines and projects. It uses a git-backed registry for synchronization and SQLite for local state tracking.

## Build & Development Commands

```bash
cargo build                  # Build the project
cargo check                  # Type-check without building
cargo test                   # Run all tests
cargo test manifest          # Run tests matching "manifest"
cargo test -- --nocapture    # Run tests with stdout visible
cargo run -- <subcommand>    # Run the CLI (e.g., cargo run -- init)
cargo clippy                 # Lint
cargo fmt                    # Format code
```

Note: `rusqlite` uses bundled SQLite — no system SQLite installation needed. The `git2` crate requires libgit2 build tools (cmake or pkg-config).

## Architecture

### Module Structure

The codebase follows a layered architecture with six modules, all wired through `src/main.rs`:

- **`cli/`** — Command definitions (clap derive) and dispatch. Each subcommand has its own file (e.g., `cli/init.rs`, `cli/add.rs`). The central `mod.rs` defines `Cli`, `Commands` enum, and routes via `run()`.
- **`registry/`** — Core data layer. `manifest.rs` defines the YAML schema types (`Manifest`, `SkillEntry`, `PluginEntry`, `McpServerEntry`, `ProfileConfig`, `SkillSyncConfig`) with load/save/validate. `resource.rs` provides deterministic SHA-256 hashing and deep-copy utilities. `git_ops.rs` wraps libgit2 operations.
- **`installer/`** — Installs resources into Claude Code's filesystem. Separate installers for skills (copy to `.claude/skills/`), plugins (modify `settings.json`), MCP servers (merge into `.mcp.json`), and settings merging.
- **`claude/`** — Claude Code integration. `paths.rs` discovers `~/.claude/` locations, `settings.rs` manipulates `settings.json`, `hooks.rs` manages SessionStart hook injection.
- **`state/`** — SQLite state database (`db.rs`) tracking installed resources and sync history.
- **`tui/`** — Interactive selection using `inquire`. Multi-select/single-select (`selector.rs`), profile picker, and conflict diff viewer.
- **`watcher/`** — File system monitoring with `notify` for auto-sync on changes.

### Key Data Flow

1. Registry stores resources in `manifest.yaml` (YAML) with actual skill files under `resources/`
2. Per-project config lives in `.claude/skillsync.yaml` declaring which resources to use
3. `install` command reads the project config and copies/merges resources into Claude Code's locations
4. `sync` uses git2 for bidirectional sync with conflict detection

### Configuration Files

| File | Format | Purpose |
|------|--------|---------|
| `manifest.yaml` | YAML | Registry manifest — all skills, plugins, MCP servers, profiles |
| `.claude/skillsync.yaml` | YAML | Per-project resource declarations |
| `profiles/<name>.yaml` | YAML | Named bundles of resources |

### Implementation Status

Phase 1 (scaffolding + data models) is complete. The manifest system, resource hashing/copying, and CLI routing are implemented. Most CLI command handlers, git operations, installers, TUI, state DB, and watcher are stubs awaiting implementation. See `openspec/changes/skillsync-cli/tasks.md` for the full task breakdown (10 phases, ~91 tasks).

## OpenSpec

This project uses OpenSpec for spec-driven development. Design docs and detailed specs live in `openspec/changes/skillsync-cli/` with subdirectories for each feature area (registry-management, project-installer, git-sync, profile-system, interactive-selector, auto-watcher).
