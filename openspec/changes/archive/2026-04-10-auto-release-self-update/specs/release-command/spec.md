# Release Command

## Summary

提供 `skillsync release` 命令，实现版本发布的自动化流程：版本号计算、CHANGELOG 生成、git tag 创建与推送。

## Requirements

### REQ-001: 版本号自动计算
- **优先级**: P0
- **描述**: 根据选项自动递增版本号
- **验收标准**:
  - `--patch`（默认）：递增 patch 版本（0.1.0 → 0.1.1）
  - `--minor`：递增 minor 版本（0.1.0 → 0.2.0），patch 重置为 0
  - `--major`：递增 major 版本（0.1.0 → 1.0.0），minor 和 patch 重置为 0
  - 版本号从 `Cargo.toml` 读取，更新后写回

### REQ-002: CHANGELOG 自动生成
- **优先级**: P0
- **描述**: 从 git commits 自动生成 CHANGELOG
- **验收标准**:
  - 提取从上一个 tag 到 HEAD 的所有 commits
  - 按 conventional commits 类型分组（feat/fix/docs/refactor/test/chore）
  - 输出格式符合 [Keep a Changelog](https://keepachangelog.com/)
  - 新条目插入到 CHANGELOG.md 顶部

### REQ-003: Git 操作
- **优先级**: P0
- **描述**: 自动创建并推送 git tag
- **验收标准**:
  - 检查工作区必须 clean（无未提交更改）
  - 创建 tag 格式为 `v{version}`（如 `v0.2.0`）
  - 自动 commit Cargo.toml 和 CHANGELOG.md 的更改
  - 推送 commit 和 tag 到 origin

### REQ-004: Dry-run 模式
- **优先级**: P1
- **描述**: 预览将要执行的操作而不实际执行
- **验收标准**:
  - 显示新版本号
  - 显示生成的 CHANGELOG 内容
  - 显示将要创建的 tag 名称
  - 不修改任何文件

## CLI Interface

```
skillsync release [OPTIONS]

Options:
      --major     Bump major version (1.0.0 → 2.0.0)
      --minor     Bump minor version (0.1.0 → 0.2.0)
      --patch     Bump patch version (0.1.0 → 0.1.1) [default]
      --dry-run   Preview without making changes
```

## Examples

```bash
# 发布 patch 版本
skillsync release

# 发布 minor 版本
skillsync release --minor

# 预览发布操作
skillsync release --dry-run
```

## Error Cases

| 错误 | 处理 |
|------|------|
| 工作区有未提交更改 | 拒绝执行，提示 "Working tree is not clean. Please commit or stash changes first." |
| 无上一个 tag | 从第一个 commit 开始生成 CHANGELOG |
| Cargo.toml 格式错误 | 报错并退出 |
| git push 失败 | 回滚本地 tag 和 commit，提示网络错误 |

## Dependencies

- `semver` crate — 版本号解析
- `git2` crate — git 操作（已有）
- `toml` 解析 — 可使用 `toml_edit` 或手动解析