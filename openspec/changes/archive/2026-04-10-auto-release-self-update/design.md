## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              发布与更新流程                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   skillsync release                         skillsync self update        │
│        ↓                                          ↓                       │
│   ┌─────────────────┐                      ┌─────────────────┐          │
│   │ 版本计算模块     │                      │ 更新检测模块     │          │
│   │ (semver)        │                      │ (GitHub API)    │          │
│   └────────┬────────┘                      └────────┬────────┘          │
│            ↓                                        ↓                    │
│   ┌─────────────────┐                      ┌─────────────────┐          │
│   │ CHANGELOG 生成   │                      │ 平台检测模块     │          │
│   │ (git log 解析)  │                      │ (target_os/arch)│          │
│   └────────┬────────┘                      └────────┬────────┘          │
│            ↓                                        ↓                    │
│   ┌─────────────────┐                      ┌─────────────────┐          │
│   │ Git 操作模块     │                      │ 下载与验证模块   │          │
│   │ (tag + push)    │                      │ (checksum)      │          │
│   └────────┬────────┘                      └────────┬────────┘          │
│            ↓                                        ↓                    │
│   GitHub Actions 构建                        替换可执行文件              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Module Design

### 1. `cli/release.rs` — 发布命令

```rust
/// Release 命令选项
pub struct ReleaseOptions {
    /// 递增级别：patch（默认）、minor、major
    pub level: BumpLevel,
    /// 预览模式，不实际执行
    pub dry_run: bool,
}

pub enum BumpLevel {
    Patch,  // 0.1.0 → 0.1.1
    Minor,  // 0.1.0 → 0.2.0
    Major,  // 0.1.0 → 1.0.0
}

/// 执行发布流程
pub fn run(options: ReleaseOptions) -> Result<()> {
    // 1. 检查工作区状态（必须 clean）
    // 2. 读取当前版本号
    // 3. 计算新版本号
    // 4. 更新 Cargo.toml
    // 5. 生成 CHANGELOG.md
    // 6. git add + commit
    // 7. git tag v{version}
    // 8. git push origin main --tags
}
```

**版本号计算逻辑：**
```rust
fn bump_version(current: &str, level: BumpLevel) -> Result<String> {
    let mut semver = Version::parse(current)?;
    match level {
        BumpLevel::Patch => semver.patch += 1,
        BumpLevel::Minor => { semver.minor += 1; semver.patch = 0; }
        BumpLevel::Major => { semver.major += 1; semver.minor = 0; semver.patch = 0; }
    }
    Ok(semver.to_string())
}
```

**CHANGELOG 生成规则：**
- 从上一个 tag 到 HEAD 的 commits
- 按 conventional commits 类型分组：`feat`、`fix`、`docs`、`refactor`、`test`、`chore`
- 格式参考 [Keep a Changelog](https://keepachangelog.com/)

### 2. `cli/self_update.rs` — 自更新命令

```rust
/// 平台信息（编译时确定）
pub struct Platform {
    pub target: &'static str,  // e.g., "aarch64-apple-darwin"
    pub binary_name: &'static str,  // e.g., "skillsync-aarch64-apple-darwin"
}

/// 执行自更新
pub fn run() -> Result<()> {
    // 1. 获取当前版本
    // 2. 查询 GitHub Releases API
    // 3. 比较版本，无更新则退出
    // 4. 检测当前平台
    // 5. 下载对应二进制文件
    // 6. 验证 SHA256 checksum
    // 7. 替换当前可执行文件
}
```

**GitHub API 调用：**
```
GET https://api.github.com/repos/yuzai-code/SkillSync/releases/latest

Response:
{
  "tag_name": "v0.2.0",
  "assets": [
    { "name": "skillsync-aarch64-apple-darwin", "browser_download_url": "..." },
    { "name": "skillsync-x86_64-apple-darwin", "browser_download_url": "..." },
    ...
  ]
}
```

**平台映射：**
```rust
const CURRENT_PLATFORM: Platform = match (cfg!(target_arch), cfg!(target_os)) {
    (true, true) if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") => 
        Platform { target: "aarch64-apple-darwin", ... },
    (true, true) if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") => 
        Platform { target: "x86_64-apple-darwin", ... },
    (true, true) if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") => 
        Platform { target: "x86_64-unknown-linux-gnu", ... },
    (true, true) if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") => 
        Platform { target: "aarch64-unknown-linux-gnu", ... },
    _ => compile_error!("Unsupported platform"),
};
```

**二进制替换流程：**
1. 下载到临时目录（`std::env::temp_dir()`）
2. 验证 checksum（从 `checksums-sha256.txt` 获取）
3. 设置可执行权限（Unix）
4. `std::fs::rename()` 原子替换（同文件系统内）

### 3. CLI 命令结构变更

```rust
// cli/mod.rs

#[derive(Subcommand)]
pub enum Commands {
    // ... existing commands ...
    
    /// Release a new version
    Release {
        /// Bump major version
        #[arg(long)]
        major: bool,
        /// Bump minor version
        #[arg(long)]
        minor: bool,
        /// Preview without making changes
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Self-management commands
    Self_ {
        #[command(subcommand)]
        action: SelfAction,
    },
}

#[derive(Subcommand)]
pub enum SelfAction {
    /// Update to the latest version
    Update {},
}
```

**注意：** `self` 是 Rust 关键字，使用 `Self_` 作为命令名，clap 会自动处理为 `self`。

## Error Handling

| 错误场景 | 处理方式 |
|---------|---------|
| 工作区有未提交更改 | `release` 拒绝执行，提示先提交 |
| 无网络连接 | `self update` 提示网络错误，建议稍后重试 |
| GitHub API 限流 | 显示友好提示，包含重试时间 |
| 权限不足 | 提示使用 `sudo skillsync self update` |
| checksum 验证失败 | 删除临时文件，提示下载损坏 |
| 不支持的平台 | 编译时报错 |

## Dependencies

```toml
[dependencies]
# 新增依赖
semver = "1"      # 版本号解析与计算
ureq = "2"        # HTTP 客户端（同步、轻量）

# 已有依赖（复用）
sha2 = "0.10"     # checksum 验证
git2 = "0.19"     # git 操作
```

## File Changes

| 文件 | 操作 | 说明 |
|------|------|------|
| `src/cli/mod.rs` | 修改 | 新增 `Release`、`Self_` 命令 |
| `src/cli/release.rs` | 新增 | 发布命令实现 |
| `src/cli/self_update.rs` | 新增 | 自更新命令实现 |
| `Cargo.toml` | 修改 | 新增 `semver`、`ureq` 依赖 |
| `CHANGELOG.md` | 新增 | 由 `release` 命令自动维护 |
| `.github/workflows/release.yml` | 无需修改 | 已支持 tag 触发 |