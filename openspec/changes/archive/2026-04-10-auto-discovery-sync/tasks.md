## 1. Registry Git Repo Structure

- [x] 1.1 Modify `SkillSyncPaths` to include `registry.git` as bare repo path
- [x] 1.2 Update `init` command to create `~/.skillsync/registry.git/` as bare git repo
- [x] 1.3 Link `~/.skillsync/registry/` as working tree of the bare repo
- [x] 1.4 Update `git_ops.rs` to support bare repo operations (stage only skills/)
- [x] 1.5 Add `skillsync remote add/remove` commands for registry remote management
- [x] 1.6 Update `config.yaml` schema to include `registry_remote` field

## 2. State Database - Discovered Projects

- [x] 2.1 Add `discovered_projects` table migration to `state/db.rs`
  - Columns: `id`, `project_path`, `git_remote`, `first_discovered_at`, `last_scanned_at`, `status`
- [x] 2.2 Add `insert_discovered_project()` method to `StateDb`
- [x] 2.3 Add `update_project_scan_time()` method to `StateDb`
- [x] 2.4 Add `get_discovered_projects()` method to `StateDb`
- [x] 2.5 Add `mark_project_removed()` method to `StateDb`

## 3. Project Skills Discovery

- [x] 3.1 Implement `scan_projects_skills()` in `registry/discover.rs`
  - Scan `~/projects/*/.claude/skills/` (respecting git-only filter)
  - Return `Vec<DiscoveredSkill>` with path and metadata
- [x] 3.2 Implement deduplication by content hash (multiple source paths → one manifest entry)
- [x] 3.3 Modify `init` command to call full project scan on new registry
- [x] 3.4 Modify `sync` command to call incremental project scan
- [x] 3.5 Copy discovered skill files to `~/.skillsync/registry/resources/skills/<name>/`
- [x] 3.6 Generate manifest entries for discovered skills with `source_path` metadata (added source_path field to SkillEntry)

## 4. Watcher - Project Directory Monitoring

- [x] 4.1 Implement `project_watch_dirs()` in `watcher/fs_watcher.rs`
  - Load project paths from `state.db`
  - Return `Vec<PathBuf>` for all `<project>/.claude/skills/` directories
- [x] 4.2 Extend `default_watch_dirs()` to include project paths (merge with global)
- [x] 4.3 Modify auto-push callback to stage only `resources/skills/` directory
- [x] 4.4 Add manifest sync to auto-push (skill file + manifest entry in same commit)

## 5. Watcher - Configuration and Control

- [x] 5.1 Add `auto_sync: bool` field to `~/.skillsync/config.yaml`
- [x] 5.2 Check `auto_sync` flag before auto-commit in watcher
- [x] 5.3 Add `--pause` flag to `watch` command (stops processing, keeps monitoring)
- [x] 5.4 Add `--resume` flag to `watch` command (resumes processing)
- [x] 5.5 Update watcher log messages for disabled/suspended states

## 6. TUI - Remote Skills Selection

- [x] 6.1 Extend `tui/selector.rs` with `select_remote_skills()` function
  - Display new remote skills with name, source project, description
  - Multi-select support
- [x] 6.2 Add install scope selection: "全局" or "选择项目"
- [x] 6.3 Implement project picker for install target selection
- [x] 6.4 Add dry-run preview before install (show files, target path)
- [x] 6.5 Add `--skip-select` flag to `sync` command

## 7. Sync Command Integration

- [x] 7.1 Modify `sync` command flow: pull → merge manifest → show TUI → install selected
- [x] 7.2 Implement LWW conflict resolution in `git_ops.rs`
  - Compare commit timestamps between local and remote
  - Auto-resolve with latest timestamp wins
- [x] 7.3 Handle deleted skills (manifest entry removal on file deletion)
- [x] 7.4 Ensure `sync` works without remote configured (local-only mode)

## 8. i18n Messages

- [x] 8.1 Add i18n keys for new TUI strings (remote skills selection, install scope)
- [x] 8.2 Add i18n keys for new watcher log messages (auto-sync disabled, paused, resumed)
- [x] 8.3 Add i18n keys for project discovery messages (scanned N projects, found M skills)
