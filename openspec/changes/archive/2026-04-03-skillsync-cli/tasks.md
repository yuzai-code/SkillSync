## 1. Project Scaffolding

- [x] 1.1 Initialize Rust project with `cargo init skillsync` and configure Cargo.toml with all dependencies (clap, inquire, git2, notify, rusqlite, serde_yaml, serde_json, walkdir, sha2, fs_extra, anyhow, thiserror, console, indicatif)
- [x] 1.2 Set up module structure: cli/, registry/, installer/, watcher/, state/, tui/, claude/
- [x] 1.3 Define CLI command structure with clap derive macros (all subcommands: init, sync, pull, push, use, add, remove, update, list, search, info, profile, resolve, doctor, watch, hook)
- [x] 1.4 Implement `claude/paths.rs` — discover Claude Code paths (~/.claude/, skills/, plugins/, .mcp.json, settings.json)


## 2. Data Models & Manifest

- [x] 2.1 Define manifest.yaml schema as Rust structs (Manifest, SkillEntry, PluginEntry, McpEntry) with serde derive
- [x] 2.2 Implement manifest parser and validator (read, write, validate manifest.yaml)
- [x] 2.3 Define skillsync.yaml schema (project declaration file) with serde structs
- [x] 2.4 Define profile YAML schema (name, description, skills, plugins, mcp) with serde structs
- [x] 2.5 Implement SQLite state database schema and init (tables: installed_resources, sync_history) via rusqlite

## 3. Registry Management (Phase 1)

- [x] 3.1 Implement `skillsync init` — new registry creation (scaffold directories, init git, create empty manifest)
- [x] 3.2 Implement `skillsync init --from <url>` — clone remote registry, validate manifest, install globals
- [x] 3.3 Implement auto-import during init — scan ~/.claude/skills/, plugins/, .mcp.json and offer to import
- [x] 3.4 Implement `skillsync add <path>` — copy skill to registry, update manifest, compute hash
- [x] 3.5 Implement `skillsync add --plugin <name>@<marketplace>` — record plugin declaration in manifest
- [x] 3.6 Implement `skillsync add --mcp <name> --command <cmd> --args <args>` — record MCP config
- [x] 3.7 Implement `skillsync remove <name>` — remove from manifest and resources/, warn if referenced
- [x] 3.8 Implement `skillsync list` — formatted table output of all resources (with --type filter)
- [x] 3.9 Implement `skillsync info <name>` — detailed resource view (metadata, hash, usage, profiles)

## 4. Interactive Selector (Phase 2)

- [x] 4.1 Implement TUI configuration method chooser (from profile / manual / copy from project)
- [x] 4.2 Implement profile selection screen with inquire Select (name, description, resource count)
- [x] 4.3 Implement multi-select resource screen with inquire MultiSelect (grouped by category, pre-checked for profile)
- [x] 4.4 Implement type-ahead search filtering in multi-select screens
- [x] 4.5 Implement "copy from project" flow — list projects with skillsync.yaml, load as starting config
- [x] 4.6 Implement dry-run preview — show summary of changes before applying, wait for confirmation

## 5. Project Installer (Phase 2)

- [x] 5.1 Implement skill installer — copy skill files from registry to target .claude/skills/, with hash comparison for skip/overwrite
- [x] 5.2 Implement global skill installer — copy global-scope skills to ~/.claude/skills/
- [x] 5.3 Implement MCP merger — read/create .mcp.json, merge declared servers, preserve unmanaged entries
- [x] 5.4 Implement plugin enabler — update settings.json enabledPlugins and extraKnownMarketplaces, attempt `claude plugins install` for missing plugins
- [x] 5.5 Implement skillsync.yaml generator — write declaration file after `skillsync use`
- [x] 5.6 Implement skillsync.lock generator — write installed versions and hashes
- [x] 5.7 Implement `skillsync install` — read project skillsync.yaml and install all declared resources
- [x] 5.8 Implement `skillsync doctor` — verify registry, remote, Claude Code, manifest, orphaned resources

## 6. Git Sync (Phase 3)

- [x] 6.1 Implement git operations module (git2) — clone, pull, push, status, stage, commit, conflict detection
- [x] 6.2 Implement `skillsync pull` — fetch + merge, detect conflicts, apply updated globals, quiet mode support
- [x] 6.3 Implement `skillsync push` — scan local skill changes vs registry, copy diffs into registry, commit, push
- [x] 6.4 Implement `skillsync sync` — pull then push, combined summary
- [x] 6.5 Implement local change scanner — compare ~/.claude/skills/ content hashes against registry, detect external modifications
- [x] 6.6 Implement `skillsync resolve` — list conflicts, offer choices (editor, keep local, use remote, side-by-side diff), complete merge after resolution

## 7. Auto Watcher (Phase 4)

- [x] 7.1 Implement file watcher with notify crate — monitor ~/.claude/skills/ and registered project skills directories
- [x] 7.2 Implement debounce logic (2s) and auto-push trigger
- [x] 7.3 Implement watcher error recovery — log errors, retry on next change, don't crash
- [x] 7.4 Implement `skillsync watch --daemon` — background mode
- [x] 7.5 Implement `skillsync hook install` — inject SessionStart hook into ~/.claude/settings.json (preserve existing hooks)
- [x] 7.6 Implement `skillsync hook remove` — remove skillsync hook only
- [x] 7.7 Implement `skillsync watch --install` for macOS launchd plist generation and loading
- [x] 7.8 Implement `skillsync watch --install` for Linux systemd user service generation and enabling
- [x] 7.9 Implement `skillsync watch --uninstall` — stop and remove platform-specific service

## 8. Profile System

- [x] 8.1 Implement `skillsync profile list` — table of profiles with resource counts
- [x] 8.2 Implement `skillsync profile create <name>` — interactive resource selection, write to profiles/<name>.yaml
- [x] 8.3 Implement `skillsync profile apply <name>` — install profile resources + generate skillsync.yaml
- [x] 8.4 Implement `skillsync profile export <name>` — read project skillsync.yaml, prompt description, write profile

## 9. Distribution & Install Script

- [x] 9.1 Create GitHub Actions CI workflow — cross-compile for darwin-aarch64, darwin-x86_64, linux-x86_64, linux-aarch64
- [x] 9.2 Create install.sh — detect OS/arch, download binary from GitHub Releases, install to PATH, verify
- [x] 9.3 Create release automation — tag-triggered build + release with pre-compiled binaries
- [x] 9.4 Write README.md with installation instructions, quick start guide, and command reference

## 10. Testing & Polish

- [x] 10.1 Write unit tests for manifest parsing and validation
- [x] 10.2 Write unit tests for MCP/settings.json merging logic
- [x] 10.3 Write integration tests for init → add → list → use → install flow
- [x] 10.4 Write integration tests for pull → push → sync → resolve flow
- [x] 10.5 Test cross-platform builds (macOS ARM, macOS Intel, Linux)
- [x] 10.6 Error message polish — ensure all user-facing errors are clear with actionable suggestions
