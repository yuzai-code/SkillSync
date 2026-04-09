# CLAUDE.md

本文件为 Claude Code 提供项目上下文和开发指引。

## 项目概述

SkillSync 是一个 Rust CLI 工具，用于跨机器、跨项目管理和同步 Claude Code 的 skills、plugins 和 MCP servers。使用 git 仓库同步配置，SQLite 跟踪本地状态。

## 构建与开发命令

```bash
cargo build                  # 构建项目
cargo build --release        # 构建优化版本（产物 3.7MB）
cargo check                  # 仅类型检查
cargo test                   # 运行全部 125 个测试（113 单元 + 12 集成）
cargo test manifest          # 运行匹配 "manifest" 的测试
cargo test -- --nocapture    # 运行测试并显示 stdout
cargo run -- <subcommand>    # 运行 CLI（如 cargo run -- init）
cargo clippy                 # 代码检查
cargo fmt                    # 格式化代码
```

注意：`rusqlite` 使用 bundled SQLite，无需系统安装。`git2` 需要 libgit2 构建工具（cmake 或 pkg-config）。

## 架构

### 模块结构

代码库采用分层架构，七个模块通过 `src/main.rs` 和 `src/lib.rs` 组织：

- **`cli/`** — 命令定义（clap derive）和分发。每个子命令一个文件（如 `cli/init.rs`、`cli/add.rs`）。`mod.rs` 定义 `Cli`、`Commands` 枚举，通过 `run()` 路由。共 16 个子命令全部实现。
- **`registry/`** — 核心数据层。`manifest.rs` 定义 YAML schema 类型（`Manifest`、`SkillEntry`、`PluginEntry`、`McpServerEntry`、`ProfileConfig`、`SkillSyncConfig`），提供 load/save/validate。`resource.rs` 提供确定性 SHA-256 哈希和深拷贝工具。`git_ops.rs` 封装 libgit2 操作（clone/fetch/merge/push/commit）。
- **`installer/`** — 将资源安装到 Claude Code 文件系统。`skill_installer.rs` 复制 skills 并基于哈希判断跳过/更新，`mcp_installer.rs` 合并 `.mcp.json`，`plugin_installer.rs` 更新 `settings.json` 插件配置，`settings_merger.rs` 生成 `skillsync.yaml` 和 `skillsync.lock`。
- **`claude/`** — Claude Code 集成。`paths.rs` 发现 `~/.claude/` 路径（`ClaudePaths`、`ProjectPaths`、`SkillSyncPaths` 三个结构体），`settings.rs` 操作 `settings.json`，`hooks.rs` 管理 SessionStart hook 注入/移除。
- **`state/`** — SQLite 状态数据库（`db.rs`），`StateDb` 跟踪已安装资源和同步历史。
- **`tui/`** — 交互式选择器（`inquire` crate）。`selector.rs` 多选/单选资源，`profile_picker.rs` Profile 选择，`diff_viewer.rs` 冲突差异查看。
- **`watcher/`** — 文件系统监控（`notify` crate）。`fs_watcher.rs` 实现 2 秒防抖监控和自动 push。
- **`i18n/`** — 国际化模块。`Lang` 枚举支持中英文，`Msg` 枚举包含 140+ 消息键，`t!()` 宏返回当前语言对应的翻译字符串。语言检测优先级：`SKILLSYNC_LANG` → `~/.skillsync/.lang` → 系统 locale → 默认英文。

### 核心数据流

1. Registry 在 `manifest.yaml`（YAML）中存储资源元数据，实际文件在 `resources/` 下
2. 项目配置在 `.claude/skillsync.yaml` 中声明需要的资源
3. `install` 命令读取项目配置，将资源复制/合并到 Claude Code 对应位置
4. `sync` 使用 git2 实现双向同步，支持冲突检测和交互式解决

### 配置文件

| 文件 | 格式 | 用途 |
|------|------|------|
| `manifest.yaml` | YAML | Registry 清单 — 所有 skills、plugins、MCP servers、profiles |
| `.claude/skillsync.yaml` | YAML | 项目资源声明（提交到 git） |
| `.claude/skillsync.lock` | YAML | 已安装的版本和哈希（gitignore） |
| `profiles/<name>.yaml` | YAML | 命名的资源捆绑包 |
| `~/.skillsync/state.db` | SQLite | 安装状态和同步历史 |

### 关键类型

| 类型 | 文件 | 说明 |
|------|------|------|
| `Manifest` | `registry/manifest.rs` | 顶层清单，含 skills/plugins/mcp_servers/profiles HashMap |
| `SkillEntry` | `registry/manifest.rs` | Skill 元数据（类型、作用域、版本、路径、哈希） |
| `SkillSyncConfig` | `registry/manifest.rs` | 项目声明文件结构 |
| `ProfileConfig` | `registry/manifest.rs` | Profile 定义 |
| `ClaudePaths` | `claude/paths.rs` | `~/.claude/` 下的全局路径 |
| `ProjectPaths` | `claude/paths.rs` | 项目内 `.claude/` 路径 |
| `SkillSyncPaths` | `claude/paths.rs` | `~/.skillsync/` 下的路径 |
| `StateDb` | `state/db.rs` | SQLite 连接封装 |

## 测试

项目有 117 个测试（105 单元 + 12 集成），全部通过：

- `registry/manifest.rs` — 20 个测试，覆盖解析、验证、序列化边界情况
- `registry/resource.rs` — 5 个测试，覆盖哈希和复制
- `registry/git_ops.rs` — 7 个测试，覆盖仓库操作
- `state/db.rs` — 10 个测试，覆盖 CRUD 和 upsert
- `claude/paths.rs` — 9 个测试，覆盖路径发现
- `claude/settings.rs` — 10 个测试，覆盖 JSON 操作
- `claude/hooks.rs` — 8 个测试，覆盖 hook 注入/移除
- `installer/` — 14 个测试，覆盖 skill 安装、MCP 合并、settings 生成
- `cli/update.rs` — 5 个测试
- `cli/resolve.rs` — 3 个测试
- `tui/diff_viewer.rs` — 4 个测试
- `tests/integration_test.rs` — 12 个集成测试，覆盖完整工作流

## OpenSpec

本项目使用 OpenSpec 进行 spec 驱动开发。设计文档和详细 spec 在 `openspec/` 目录：

- `openspec/specs/` — 主 specs（7 个 capability：registry-management、interactive-selector、sync-engine、doctor、profiles、i18n、hooks-integration）
- `openspec/changes/archive/` — 已归档的 change

## 开发注意事项

- 所有 CLI 命令依赖 `dirs::home_dir()` 获取 `~` 路径
- 错误处理统一使用 `anyhow::Result`、`anyhow::Context`、`anyhow::bail!`
- 彩色输出使用 `console::style()`
- 交互式 UI 使用 `inquire` crate（Select/MultiSelect/Confirm）
- Git 操作使用 `git2` crate，SSH 认证通过 ssh-agent
- 回复和输出使用中文
- `init` 命令首次运行时会交互式选择语言（中文/English），语言偏好保存至 `~/.skillsync/.lang`
