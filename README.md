# SkillSync

一个用 Rust 编写的 CLI 工具，用于跨机器、跨项目管理和同步 Claude Code 的 skills、plugins 和 MCP servers。

SkillSync 使用 git 仓库作为配置的唯一真实来源，实现一行命令恢复完整环境、声明式项目配置和自动同步。

## 安装

### 从源码编译

```bash
git clone https://github.com/OWNER/skillsync.git
cd skillsync
cargo install --path .
```

### 从 Release 下载

```bash
curl -fsSL https://raw.githubusercontent.com/OWNER/skillsync/main/install.sh | sh
```

脚本会自动检测操作系统和架构（macOS/Linux, x86_64/aarch64），下载对应的预编译二进制文件。

## 快速上手

### 首次使用

```bash
# 1. 初始化一个新的 registry
skillsync init

# 2. 把现有的 skill 添加到 registry
skillsync add ~/.claude/skills/my-skill

# 3. 添加插件
skillsync add --plugin superpowers@claude-plugins-official

# 4. 添加 MCP server
skillsync add --mcp openspec --command npx --args -y @fission-ai/openspec-mcp

# 5. 在项目目录下，交互式选择需要的资源
skillsync use

# 6. 或者从已有的 skillsync.yaml 安装
skillsync install
```

### 换机器 / 新环境

```bash
# 从远程克隆你的 registry
skillsync init --from git@github.com:you/skillsync-registry.git

# 安装全局 skills 和 MCP servers
skillsync install --global

# 设置自动同步 hook（每次打开 Claude Code 自动拉取更新）
skillsync hook install
```

## 命令参考

### Registry 管理

| 命令 | 说明 |
|------|------|
| `skillsync init` | 创建新的本地 registry |
| `skillsync init --from <url>` | 从远程克隆已有 registry |
| `skillsync add <path>` | 添加本地 skill 目录到 registry |
| `skillsync add --plugin <name>@<marketplace>` | 注册插件 |
| `skillsync add --mcp <name> --command <cmd> [--args ...]` | 注册 MCP server |
| `skillsync remove <name>` | 从 registry 移除资源 |
| `skillsync update <name>` | 更新资源版本 |
| `skillsync list [--type skill\|plugin\|mcp]` | 列出所有已注册资源 |
| `skillsync search <query>` | 按名称、描述、标签搜索资源 |
| `skillsync info <name>` | 显示资源详细信息 |

### 项目配置

| 命令 | 说明 |
|------|------|
| `skillsync use` | 交互式 TUI 配置当前项目 |
| `skillsync install` | 安装 `.claude/skillsync.yaml` 中声明的资源 |
| `skillsync install --global` | 安装所有全局作用域资源到 `~/.claude/` |

`skillsync use` 提供三种配置方式：

- **从 Profile 选择** — 选择预定义的配置模板，然后微调具体资源
- **手动选择** — 从 registry 全量资源中多选
- **从项目复制** — 以其他项目的配置为起点调整

### 同步

| 命令 | 说明 |
|------|------|
| `skillsync pull` | 拉取远程 registry 变更 |
| `skillsync push` | 提交并推送本地变更到远程 |
| `skillsync sync` | 双向同步（先 pull 后 push） |
| `skillsync resolve` | 交互式解决合并冲突 |

```bash
# 静默拉取（用于 hook/自动化）
skillsync pull --quiet --timeout 5

# 自动提交推送（用于文件监控守护进程）
skillsync push --auto
```

### Profile 管理

| 命令 | 说明 |
|------|------|
| `skillsync profile list` | 列出所有 profile 及资源数量 |
| `skillsync profile create <name>` | 创建新 profile |
| `skillsync profile apply <name>` | 将 profile 应用到当前项目 |
| `skillsync profile export <name>` | 将当前项目配置导出为 profile |

### 自动同步

| 命令 | 说明 |
|------|------|
| `skillsync watch` | 前台启动文件监控 |
| `skillsync watch --daemon` | 后台启动文件监控 |
| `skillsync watch --install` | 安装为系统服务（macOS launchd / Linux systemd） |
| `skillsync watch --uninstall` | 卸载系统服务 |
| `skillsync hook install` | 注入 Claude Code SessionStart hook |
| `skillsync hook remove` | 移除 SessionStart hook |

### 环境诊断

```bash
skillsync doctor
```

检查 registry 状态、manifest 有效性、git remote、Claude Code 安装、资源一致性，并给出修复建议。

### 语言切换

SkillSync 支持中文和英文界面。

首次运行 `skillsync init` 时会交互式选择语言偏好，之后的语言检测优先级：

1. `SKILLSYNC_LANG` 环境变量（`zh` 或 `en`）
2. `~/.skillsync/.lang` 配置文件（`skillsync init` 时生成）
3. 系统 `LANG` / `LC_ALL` 环境变量（包含 `zh` 则为中文）
4. 默认为英文

```bash
# 手动切换语言
SKILLSYNC_LANG=zh skillsync doctor
SKILLSYNC_LANG=en skillsync doctor
```

## 配置文件

| 文件 | 位置 | 用途 |
|------|------|------|
| `manifest.yaml` | `~/.skillsync/registry/` | Registry 清单 — 所有资源和 profile |
| `skillsync.yaml` | `<project>/.claude/` | 项目资源声明（应提交到 git） |
| `skillsync.lock` | `<project>/.claude/` | 已安装的版本和哈希（加入 .gitignore） |
| `state.db` | `~/.skillsync/` | SQLite 数据库，跟踪安装状态 |

### skillsync.yaml 示例

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

### manifest.yaml 示例

```yaml
version: 1
skills:
  openspec-expert:
    type: custom
    scope: shared
    version: "1.0.0"
    path: resources/skills/openspec-expert
    description: OpenSpec 工作流专家
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

## 目录结构

```
~/.skillsync/
  registry/                    # Git 仓库（本地克隆）
    manifest.yaml              # 资源清单
    resources/
      skills/                  # Skill 源文件
      plugins/                 # 插件备份
      mcp/                     # MCP server 配置
    profiles/                  # Profile YAML 文件
  state.db                     # SQLite 状态数据库

~/.claude/
  skills/                      # 全局 skills（由 skillsync 管理）
  settings.json                # Claude Code 设置（hooks、插件）
  .mcp.json                    # 全局 MCP server 配置

<project>/
  .claude/
    skills/                    # 项目 skills（由 skillsync 安装）
    skillsync.yaml             # 项目资源声明
    skillsync.lock             # 已安装版本（gitignore）
  .mcp.json                   # 项目 MCP 配置
```

## 自动同步架构

SkillSync 提供三层同步机制：

```
第一层：Claude Code Hook（SessionStart）
  → skillsync pull --quiet --timeout 5
  → 每次启动 Claude Code 自动拉取远程更新

第二层：文件监控守护进程（可选）
  → 监控 ~/.claude/skills/ 的文件变更
  → 变更后自动 commit + push 到 registry
  → 2 秒防抖，避免频繁触发

第三层：手动同步（兜底）
  → skillsync sync
```

设置方法：

```bash
# 安装 SessionStart hook
skillsync hook install

# 可选：安装文件监控为系统服务
skillsync watch --install
```

## 全局参数

| 参数 | 说明 |
|------|------|
| `-q, --quiet` | 静默输出 |
| `-v, --verbose` | 详细输出 |
| `--dry-run` | 预览变更但不执行 |

## 从源码构建

```bash
cargo build --release       # 构建优化版本
cargo test                  # 运行全部 125 个测试
cargo clippy                # 代码检查
cargo fmt                   # 格式化
```

依赖：Rust 工具链、cmake 或 pkg-config（用于编译 libgit2）。

## License

MIT
