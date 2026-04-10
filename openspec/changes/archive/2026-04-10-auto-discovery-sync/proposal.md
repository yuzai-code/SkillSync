## Why

目前 SkillSync 需要手动添加 skills 到 registry，用户必须在项目 `.claude/skills/` 创建 skill 后，手动执行命令同步。用户希望**零操作**——创建/修改 skill 后自动同步到远程，其他设备通过 TUI 选择性安装。

核心问题：**registry 是中央仓库模式，但 skills 的发现和创作发生在分散的项目本地**。现有架构要求手动维护 manifest，违背了"最少命令操作"的设计目标。

## What Changes

- **自动发现**：首次 `init` 或 `sync` 时，自动扫描 `~/projects/*/.claude/skills/` 发现本地 skills，无需手动添加
- **零操作同步**：通过 `notify` 文件系统监控，在 skills 目录变动后自动 commit + push，不打扰用户
- **TUI 选择安装**：远程有新 skills 时，通过 TUI 交互选择要安装哪些、安装到哪（全局/项目）
- **冲突 LWW**：多设备同时修改同一 skill 时，以时间戳为准（Last Write Wins）
- **删除彻底清理**：删除 skill 时，manifest 条目一并移除

## Capabilities

### New Capabilities

- **`local-skill-discovery`**：自动扫描本地项目目录，发现 .claude/skills/ 下的 skills，解析其结构并生成 manifest 条目。支持首次 sync 时全量扫描 + 运行时增量监控。
- **`zero-operation-sync`**：基于 `notify` 的文件系统 watcher，监控 skills 目录变动，5秒防抖后自动 git commit + push。后台静默运行，失败重试不打扰用户。
- **`registry-git-repo`**：将 registry 从纯本地文件改为独立 git 仓库（`~/.skillsync/registry.git/`），支持 push/pull 同步。init 时 clone 或初始化该仓库。

### Modified Capabilities

- **`git-sync`**：扩展现有 git-sync 以支持 auto-commit 模式（无手动 commit）、LWW 冲突解决、增量 push（只 push skills 目录变动）
- **`interactive-selector`**：扩展 TUI 选择器，支持展示远程新发现的 skills 列表，支持"安装到全局"或"安装到指定项目"两种安装目标

## Impact

- **新模块**：`watcher/` 目录，引入 `notify` crate 实现文件系统监控
- **数据库变更**：`state.db` 新增 `discovered_projects` 表，记录已发现的项目路径
- **配置文件**：`~/.skillsync/config.yaml` 新增 `watcher` 配置节（监控路径列表、启用开关）
- **依赖变更**：`Cargo.toml` 新增 `notify` crate（已有一半实现，见 `watcher/fs_watcher.rs`）
- **init 流程变更**：首次 init 时自动扫描 ~/projects/ 建立初始项目列表
