## Context

Claude Code 配置散布在多个位置：`~/.claude/skills/`（全局 skills）、`<project>/.claude/skills/`（项目 skills）、`~/.claude/plugins/`（插件）、`~/.claude/.mcp.json`（MCP servers）、`~/.claude/settings.json`（设置）。没有统一的管理层，导致跨机器迁移和跨项目复用都是手动操作。

SkillSync 是一个 Rust CLI 工具，通过 git-backed registry 集中管理这些配置，提供声明式项目配置、交互式选择器和自动同步。

## Goals / Non-Goals

**Goals:**

- 一行命令在新机器上恢复完整的 Claude Code 配置环境
- 通过声明式 `skillsync.yaml` 让项目的 AI 工具配置可复现
- 交互式 TUI 让用户从全局/共享池中选择资源到项目
- 文件变更后自动同步到 registry，新机器/新会话自动拉取
- 支持社区 skill 的离线备份
- 单二进制分发，`curl | sh` 安装

**Non-Goals:**

- 不做 skill marketplace / 社区分享平台
- 不管理 Claude Code 本身的版本或更新
- 不处理 `settings.json` 中与 skills/plugins/MCP 无关的配置（如 theme、model）
- 不做实时多机协同编辑，冲突通过 git 机制解决
- 不替代 `claude plugins install`，而是编排它

## Decisions

### D1: 使用 Git 作为同步后端

**选择**: git2 (libgit2) 操作本地 git 仓库，推送到 GitHub/GitLab 等远程。

**替代方案**:
- 自建云服务: 需要维护服务器，增加复杂度
- S3/R2 对象存储: 无版本历史，无合并机制
- rsync: 无冲突检测，覆盖即丢失

**理由**: Git 提供版本历史、分支、合并、冲突检测，生态成熟，用户已有 GitHub 账号。Registry 本质上是配置文件仓库，git 是天然选择。

### D2: 声明式项目配置 (skillsync.yaml)

**选择**: 项目根目录下 `.claude/skillsync.yaml` 声明所需资源，`skillsync` 负责安装。

```yaml
# .claude/skillsync.yaml
profile: agent-dev          # 可选，基于 profile
skills:
  - openspec-expert
  - workflow-debugger
  - prompt-optimizer
mcp:
  - openspec
plugins:
  - superpowers@claude-plugins-official
```

**替代方案**:
- 符号链接: git 不跟踪链接内容，跨机器需重建
- 物理复制（当前做法）: 更新一处不同步到其他

**理由**: 声明意图而非管理文件，与 package.json / .tool-versions 模式一致。入 git 让团队成员也能受益。

### D3: Manifest 使用 YAML 格式

**选择**: `manifest.yaml` 作为 registry 的主清单。

**替代方案**:
- TOML: Rust 生态偏好，但嵌套结构不如 YAML 直观
- JSON: 无注释支持，手动编辑不友好

**理由**: YAML 支持注释、可读性强，与 Claude Code 生态中的其他配置格式一致。

### D4: SQLite 做本地状态追踪

**选择**: `~/.skillsync/state.db`（SQLite via rusqlite）记录安装状态、版本、同步时间。

**替代方案**:
- 纯文件 (JSON/YAML): 并发写入不安全
- 无状态: 每次全量对比，慢

**理由**: SQLite 是零配置嵌入式数据库，单文件，事务安全，rusqlite bundled 模式无外部依赖。

### D5: inquire crate 做交互式 TUI

**选择**: `inquire` crate 实现多选、单选、确认等交互。

**替代方案**:
- dialoguer: 功能类似但 API 不如 inquire 直观
- ratatui: 全屏 TUI，本场景过重
- 纯 CLI flags: 无交互体验

**理由**: inquire 提供开箱即用的多选/单选/搜索过滤，样式美观，维护活跃。

### D6: 资源安装策略

安装 skill 到项目时，物理复制文件到 `.claude/skills/`（而非符号链接）。

**理由**:
- Claude Code 期望 skills 在 `.claude/skills/` 中是真实文件
- 物理复制保证项目独立性，不依赖外部路径
- `skillsync.yaml` 声明 + `skillsync use` 重建 = 等效于符号链接的便利性

### D7: 自动同步分层设计

```
Layer 1: Claude Code Hook (SessionStart)
  → skillsync pull --quiet --timeout 5
  → 每次打开 Claude Code 自动拉取远程更新

Layer 2: File Watcher Daemon (可选)
  → 监控 ~/.claude/skills/ 变更
  → 自动 commit + push 到 registry
  → macOS: launchd plist
  → Linux: systemd user service

Layer 3: 手动同步 (兜底)
  → skillsync sync
```

## Risks / Trade-offs

**[Git 冲突阻塞同步]** → 自动同步遇到冲突时暂停推送，发送终端通知，等待 `skillsync resolve` 手动解决。不做自动合并策略，避免静默覆盖。

**[Claude Code 内部结构变更]** → Claude Code 的 `settings.json`、`skills/` 目录结构可能在更新中改变。Mitigation: 将路径发现集中在 `claude/paths.rs` 模块，便于适配。

**[git2 crate 体积]** → libgit2 静态链接会增大二进制体积（~5-10MB）。可接受，因为单二进制分发本身就有这个代价。

**[Plugins 安装依赖 Claude CLI]** → 无法直接安装 plugins，需要调用 `claude plugins install` 命令。如果用户未安装 Claude Code CLI，plugins 部分会降级为"提示用户手动安装"。

**[File watcher 资源消耗]** → notify crate 的 FSEvents (macOS) / inotify (Linux) 后端很轻量。只监控 `~/.claude/skills/` 和已注册项目的 skills 目录，不做全盘监控。

## Open Questions

- Claude Code 是否提供了编程式的 plugin 安装 API？目前假设只能通过 CLI 子进程调用。
- Registry 仓库是否需要支持私有仓库（SSH key / token 认证）？初始设计支持，但不做额外的凭证管理。
