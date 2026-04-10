## Why

SkillSync 目前已有 GitHub Actions 自动发布流程（tag 触发），但版本发布和更新检测完全依赖手动操作：

1. **发布繁琐**：需要手动修改 `Cargo.toml` 版本号、创建 git tag、推送 tag
2. **无更新检测**：用户不知道有新版本发布，需要主动去 GitHub 查看
3. **更新麻烦**：发现新版本后需要手动下载对应平台的二进制文件并替换

核心问题：**发布流程手动且易出错，用户无法便捷地获取更新**。

## What Changes

- **`skillsync release` 命令**：手动触发发布，自动计算版本号、更新 `Cargo.toml`、生成 CHANGELOG、创建并推送 git tag
- **`skillsync self update` 命令**：查询 GitHub Releases API 检查更新，自动下载并替换当前可执行文件
- **版本号灵活控制**：支持 `--patch`（默认）、`--minor`、`--major` 选项
- **CHANGELOG 自动生成**：从 git commits 提取变更，按类型（feat/fix/docs 等）分组

## Capabilities

### New Capabilities

- **`release-command`**：提供 `skillsync release` 命令，实现版本号自动计算、CHANGELOG 生成、git tag 创建与推送。支持 dry-run 预览和递增级别选择。
- **`self-update-command`**：提供 `skillsync self update` 命令，查询 GitHub Releases API 获取最新版本，下载对应平台二进制文件，验证 checksum 后替换当前可执行文件。

### Modified Capabilities

- **`cli`**：新增 `release` 和 `self` 子命令（`self` 为命名空间，后续可扩展 `self status` 等）

## Impact

- **新依赖**：
  - `semver = "1"` — 版本号解析与计算
  - `ureq = "2"` — HTTP 客户端（同步、轻量）
- **新模块**：`cli/release.rs`、`cli/self_update.rs`
- **CHANGELOG.md**：项目根目录新增 CHANGELOG 文件，由 `release` 命令自动维护
- **GitHub Actions**：无需修改，现有 `release.yml` 已支持 tag 触发构建