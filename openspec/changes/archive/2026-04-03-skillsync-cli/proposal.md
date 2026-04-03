## Why

Claude Code 的 skills、plugins 和 MCP servers 配置分散在 `~/.claude/` 和各项目的 `.claude/` 目录中，没有统一管理和同步机制。每次换机器或开新项目，都需要手动重新安装/复制这些配置，耗时且容易遗漏。需要一个工具将这些配置集中管理、跨机器同步、跨项目复用。

## What Changes

- 新增 `skillsync` Rust CLI 工具，提供完整的配置生命周期管理
- 引入 git-backed registry 作为配置的单一真实来源（single source of truth）
- 引入 `skillsync.yaml` 声明式项目配置文件（纳入项目 git）
- 引入 profile 机制，支持预设配置模板快速应用到项目
- 引入交互式 TUI 选择器，支持从全局/共享池中选择资源
- 引入自动同步机制（Claude Code hooks + 文件监控 daemon）
- 引入冲突检测与交互式解决流程
- 支持通过 `curl | sh` 一行命令安装

## Capabilities

### New Capabilities

- `registry-management`: Git-backed registry 的初始化、资源注册、manifest 管理，包括 skill/plugin/MCP server 三类资源的增删改查
- `interactive-selector`: 交互式 TUI，支持 profile 选择、skill/plugin/MCP 多选微调、项目配置导出
- `project-installer`: 根据 skillsync.yaml 声明或交互选择结果，将资源安装到 `~/.claude/` 或项目 `.claude/` 目录
- `git-sync`: 基于 git2 的双向同步（pull/push），包括冲突检测、交互式冲突解决、自动 commit
- `auto-watcher`: 文件变更监控 daemon + Claude Code hook 集成，实现无感自动同步
- `profile-system`: 可复用的项目配置模板，支持创建、应用、导出、从现有项目生成

### Modified Capabilities

（无已有 capabilities）

## Impact

- **新增依赖**: Rust 工具链（开发）、git（运行时）
- **文件系统**: 新增 `~/.skillsync/` 目录（registry 本地克隆 + SQLite 状态数据库）
- **Claude Code 配置**: 会修改 `~/.claude/settings.json`（注入 SessionStart hook）、`~/.claude/skills/`、`~/.claude/.mcp.json`
- **项目目录**: 新增 `.claude/skillsync.yaml`（声明文件，入 git）和 `.claude/skillsync.lock`（状态文件，gitignore）
- **系统服务**: 可选安装 macOS launchd / Linux systemd 服务用于文件监控
- **GitHub**: 需要创建 `skillsync` 仓库（CLI 代码）和 `skillsync-registry` 仓库（用户配置数据）
- **CI/CD**: GitHub Actions 交叉编译，发布预编译二进制到 GitHub Releases（darwin-aarch64, darwin-x86_64, linux-x86_64, linux-aarch64）
