## Context

SkillSync 目前已有基础的 git-sync 和 auto-watcher 能力：
- `git-sync/spec.md`：pull/push/sync，支持 "auto:" 前缀自动提交，LWW 冲突解决
- `auto-watcher/spec.md`：文件监控 + daemon 模式，macOS launchd / Linux systemd 服务
- 现有实现：`watcher/fs_watcher.rs`（2秒防抖），`cli/watch.rs`（完整服务安装）

**缺失的部分**：
1. **项目级 skills 发现**：只能扫描 `~/.claude/skills/`，无法发现 `~/projects/*/.claude/skills/` 下的 skills
2. **watcher 不监控项目路径**：只监控全局 skills 和 registry resources
3. **无 registry git repo**：registry 还是纯本地文件，没有独立的 git 仓库
4. **无 TUI 选择安装**：sync 后无法交互式选择要安装的 skills

## Goals / Non-Goals

**Goals:**
- 实现零操作自动同步：用户创建/修改 skill → 自动 commit → 自动 push
- 支持多项目 skills 自动发现：首次 init 或 sync 时扫描 ~/projects/*/.claude/skills/
- 支持 TUI 选择安装：sync 拉取远程变更后，交互式选择 skills 安装
- 独立 registry git repo：`~/.skillsync/registry.git/` 作为 registry 的 git 备份/同步方式
- LWW 冲突解决，多设备协同

**Non-Goals:**
- 不实现 skill 内容解析/验证（只记录路径和哈希）
- 不实现 skill 版本管理（SHA-256 哈希作为内容标识）
- 不实现 skill marketplace 或社区分享
- 不实现 skill 编辑器/UI（只处理文件变动同步）

## Decisions

### Decision 1: Registry 结构改为独立 git repo

**选择**：`~/.skillsync/registry.git/` 作为 bare git repo

**原因**：
- 现有 registry 是纯本地文件（`~/.skillsync/registry/`），无法直接 git sync 到其他机器
- 改为 bare repo 可以 `git push`/`git pull` 直接同步
- `git_ops.rs` 已有完整的 clone/fetch/merge/push 封装

**替代方案**：
- 把 registry 放在项目 `.claude/` 下（混在一起）→ 不干净，项目 repo 会膨胀
- 用文件同步（rsync/dropbox）→ 没有 git 的冲突解决能力

### Decision 2: 项目发现机制

**选择**：sync 时扫描 `~/projects/*/.claude/skills/`，结果缓存到 `state.db`

**原因**：
- 用户已有项目目录，不需要额外配置
- 扫描开销可控（只在 sync 时扫描）
- 缓存避免重复扫描

**实现细节**：
```
发现流程:
  1. glob ~/projects/*/.claude/skills/
  2. 对每个子目录，生成 skill entry
  3. 写入 manifest.yaml
  4. 记录项目路径到 state.db (discovered_projects 表)
```

### Decision 3: Watcher 监控范围扩展

**选择**：watcher 监控 `~/.claude/skills/` + 所有已发现的项目 skills 目录

**原因**：
- 用户在任意项目创建 skill，watcher 都能检测到
- 自动 push 到 registry.git
- 复用了现有的 watcher 实现

**实现细节**：
```rust
// fs_watcher.rs 扩展
pub fn project_watch_dirs() -> Result<Vec<PathBuf>> {
    // 从 state.db 读取 discovered_projects
    // 返回所有项目的 .claude/skills/ 路径
}
```

### Decision 4: TUI 选择安装

**选择**：扩展 `tui/selector.rs`，sync 完成后展示新增 skills 列表

**原因**：
- 复用现有 TUI 基础设施（inquire crate）
- 用户体验一致

**交互流程**：
```
sync → 发现远程新增 skill → TUI 多选 → 选择安装到全局/项目
```

### Decision 5: 冲突 LWW

**选择**：Last Write Wins，以 git commit 时间戳为准

**原因**：
- 最简单，不需要三方合并
- skill 创作场景下冲突罕见
- 符合"最少操作"目标

**实现**：使用 `git merge -X ours` 或以时间戳判断保留哪方

### Decision 6: 删除处理

**选择**：删除 skill 时，manifest 条目一并移除（不是标记 deleted）

**原因**：
- manifest 简洁，不需要 tombstone
- 用户明确删除 skill，保留历史没用

## Risks / Trade-offs

| 风险 | 缓解 |
|------|------|
| 多设备同时 push 产生 git 冲突 | LWW 自动解决，用户不需介入 |
| watcher 后台进程占用资源 | 只监控 skills 目录，不监控整个项目 |
| 首次 scan ~/projects/ 很慢 | 增量更新，已扫描项目记录到 db |
| registry.git 和本地 skills 不同步 | auto-push 确保本地变化及时推送 |
| Git 技能不足的用户 | 自动 commit/push 屏蔽了 git 复杂性 |

## Migration Plan

**Phase 1: Registry git repo**
1. 修改 `init` 命令，创建 bare repo `~/.skillsync/registry.git/`
2. 将现有 `~/.skillsync/registry/` 内容 init 并 push 到 repo
3. 更新 `SkillSyncPaths` 结构体

**Phase 2: 项目发现**
1. 添加 `discovered_projects` 表到 state.db
2. 实现 `scan_projects_skills()` 函数
3. 修改 `init` 和 `sync` 调用全量扫描

**Phase 3: Watcher 扩展**
1. 实现 `project_watch_dirs()` 读取已发现项目
2. 合并到 `default_watch_dirs()`
3. 修改 auto-push 只针对 skills 目录（不是整个 registry）

**Phase 4: TUI 选择安装**
1. 扩展 `tui/selector.rs` 支持远程新增 skills
2. sync 后自动展示选择界面
3. 支持安装到全局或指定项目

**Rollback**: 旧版本备份 `~/.skillsync/registry/` 到 `~/.skillsync/registry.backup/`

## Open Questions

1. **是否需要 git remote 配置 UI？** 首次 init 时让用户输入 registry remote URL？
2. **init 默认行为？** 是否自动 scan ~/projects/，还是需要 `--scan-projects` flag？
3. **安装时 skill 文件是否复制到 registry.git？** 还是 manifest 只记录原始路径引用？
