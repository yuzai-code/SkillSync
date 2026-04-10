# Self Update Command

## Summary

提供 `skillsync self update` 命令，查询 GitHub Releases API 检查更新，自动下载并替换当前可执行文件。

## Requirements

### REQ-001: 版本检查
- **优先级**: P0
- **描述**: 查询 GitHub Releases API 获取最新版本
- **验收标准**:
  - 调用 `GET https://api.github.com/repos/yuzai-code/SkillSync/releases/latest`
  - 解析 `tag_name` 获取最新版本号
  - 与当前版本比较，已是最新则显示提示并退出
  - 有新版本则继续下载流程

### REQ-002: 平台检测
- **优先级**: P0
- **描述**: 自动检测当前平台并下载对应二进制
- **验收标准**:
  - 编译时确定平台：`aarch64-apple-darwin`、`x86_64-apple-darwin`、`x86_64-unknown-linux-gnu`、`aarch64-unknown-linux-gnu`
  - 从 release assets 中匹配正确的文件名
  - 不支持的平台编译时报错

### REQ-003: 下载与验证
- **优先级**: P0
- **描述**: 安全下载并验证二进制文件
- **验收标准**:
  - 下载到系统临时目录
  - 从 `checksums-sha256.txt` 读取对应 checksum
  - 验证下载文件的 SHA256 与 checksum 匹配
  - 验证失败则删除临时文件并报错

### REQ-004: 二进制替换
- **优先级**: P0
- **描述**: 替换当前可执行文件
- **验收标准**:
  - 使用 `std::fs::rename()` 原子替换（同文件系统内）
  - Unix 系统保留可执行权限
  - 权限不足时提示使用 `sudo`
  - 替换成功后显示新版本号

## CLI Interface

```
skillsync self update

检查并更新到最新版本。
```

## Examples

```bash
# 检查并更新
skillsync self update

# 权限不足时
sudo skillsync self update
```

## Output Examples

```
# 已是最新
✓ Already up to date (v0.2.0)

# 有更新
Checking for updates...
Current version: v0.1.0
Latest version:  v0.2.0

Downloading skillsync-aarch64-apple-darwin...
Verifying checksum...
✓ Updated to v0.2.0

# 权限不足
✗ Permission denied. Please run: sudo skillsync self update
```

## Error Cases

| 错误 | 处理 |
|------|------|
| 网络错误 | 提示 "Network error. Please check your connection and try again." |
| GitHub API 限流 | 提示 "GitHub API rate limit exceeded. Please try again later." |
| checksum 不匹配 | 删除临时文件，提示 "Download corrupted. Please try again." |
| 权限不足 | 提示使用 `sudo` |
| 找不到对应平台资源 | 提示 "No binary available for your platform." |

## Dependencies

- `ureq` crate — HTTP 客户端
- `sha2` crate — checksum 验证（已有）

## Security Considerations

1. **HTTPS Only**: 所有请求必须使用 HTTPS
2. **Checksum Verification**: 必须验证下载文件的 SHA256
3. **No Auto-Run**: 不自动执行更新，用户必须显式调用命令
4. **Atomic Replace**: 使用原子操作替换，避免损坏安装