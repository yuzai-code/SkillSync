# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0]

### Features

- add release and self-update commands

Release command (skillsync release):
- Auto bump version (patch/minor/major)
- Generate CHANGELOG from conventional commits
- Create and push git tag
- Dry-run mode for preview

Self-update command (skillsync self update):
- Query GitHub Releases API for latest version
- Auto-detect platform and download binary
- SHA256 checksum verification
- Atomic executable replacement

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- auto-discover plugins and MCP servers from global config

- Add scan_global_plugins() to read enabledPlugins from settings.json
- Add scan_global_mcp() to read mcpServers from .mcp.json
- Register discovered plugins/MCP servers in manifest during sync
- Improve list output: group by type, color coding, better alignment
- Update use command to show source for all resource types

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- auto-discover skills from global and multiple project directories

- Add scan_global_skills() to scan ~/.claude/skills/
- Add get_project_dirs() to support multiple project locations:
  - ~/projects/
  - ~/Desktop/project/
- Add scan_all_local_skills() to combine global + project skills
- Show skill source in list command (global/project: xxx)
- Show skill source in use command during selection and preview

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- scan local skills when registry is empty

When `skillsync list` finds no resources in registry (uninitialized or
empty), it now scans `~/.claude/skills/` and displays discovered local
skills to help users get started.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- add interactive language selection during init

When `SKILLSYNC_LANG` is not set, `skillsync init` now prompts the user
to select their preferred language and saves it to `~/.skillsync/.lang`.

Language detection priority updated:
1. SKILLSYNC_LANG env var
2. ~/.skillsync/.lang config file
3. System LANG/LC_ALL
4. Default: en

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- implement i18n with Chinese/English language switching

Add `SKILLSYNC_LANG` environment variable support (zh/en), with system
locale fallback. All CLI output, TUI prompts, and error messages now
go through the new `i18n` module with 140+ translated message variants.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- implement SkillSync CLI — full Rust tool for managing Claude Code skills, plugins, and MCP servers

Complete implementation of all 61 tasks across 10 phases:
- CLI with 16 subcommands (init, add, remove, list, info, use, install, pull, push, sync, resolve, profile, doctor, watch, hook, search)
- Git-backed registry with manifest.yaml schema, resource hashing, and bidirectional sync
- Interactive TUI selector with inquire (profile picker, multi-select, diff viewer)
- Installer module for skills, plugins, MCP servers with hash-based skip/update
- SQLite state database for tracking installations and sync history
- File watcher daemon with debounce and auto-push
- Claude Code hook integration (SessionStart auto-pull)
- Profile system for reusable configuration templates
- GitHub Actions CI/CD with cross-compilation for 4 platforms
- 125 tests (113 unit + 12 integration), all passing

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>

### Bug Fixes

- use unicode-width for proper column alignment in list output

- Add unicode-width dependency for CJK character width calculation
- Use UnicodeWidthStr::width() instead of str.len() for column widths
- Add pad_to_width() helper for proper padding with unicode chars

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>

### Documentation

- update CLAUDE.md with plugins/MCP discovery and unicode-width note

- Document plugins discovery from settings.json
- Document MCP servers discovery from .mcp.json
- Add source color coding explanation
- Add unicode-width crate note for CJK character handling

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- update CLAUDE.md with auto-discovery details

- Document scanned directories: global, ~/projects/, ~/Desktop/project/
- Explain skill source display in list/use commands
- Note source_path field in manifest.yaml
- Add --skip-select tip for non-interactive environments
- Update module structure to include discover.rs and config.rs

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 更新 README 和 CLAUDE.md 为中文，反映完整实现状态

- README.md 改写为中文，包含安装、快速上手、命令参考、配置文件、目录结构、自动同步架构
- CLAUDE.md 更新为中文，补充完整的模块说明、关键类型、测试覆盖、开发注意事项

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- add README with installation, quick start, and command reference

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>

### Chores

- add MemPalace temp files to gitignore

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- fix all compiler warnings

- Add #[allow(dead_code)] to unused fields and functions
- Fields: enabled_plugins, repo, source, description
- Functions: open_bare_repo, resolve_conflicts_lww,
  set_registry_remote, is_auto_sync_enabled,
  get_project_skills_dirs, select_target_project, show_dry_run_preview
- Struct: ProjectItem

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
- 清理所有编译警告

为公共 API 类型和函数添加 #[allow(dead_code)]，移除未使用的 import，
消除 cargo check 的全部 27 个警告。

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>


- Cross-machine sync for Claude Code skills, plugins, and MCP servers
- Git-based registry with bidirectional sync
- Interactive TUI for resource selection
- Profile system for resource bundles
- Auto-discovery of local skills, plugins, and MCP servers
- File watcher for automatic sync
- i18n support (Chinese/English)
- SessionStart hook integration

### Chores

- Initial release