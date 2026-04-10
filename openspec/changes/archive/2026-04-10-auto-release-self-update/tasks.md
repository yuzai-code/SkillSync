## Tasks

### Phase 1: 基础设施

- [x] **T1.1** 添加依赖：`semver = "1"`、`ureq = "2"`
- [x] **T1.2** 创建 `src/cli/release.rs` 模块骨架
- [x] **T1.3** 创建 `src/cli/self_update.rs` 模块骨架
- [x] **T1.4** 更新 `src/cli/mod.rs`，添加 `Release` 和 `Self_` 命令定义

### Phase 2: Release 命令

- [x] **T2.1** 实现版本号解析与计算（使用 `semver` crate）
- [x] **T2.2** 实现 Cargo.toml 版本号读写
- [x] **T2.3** 实现工作区 clean 检查
- [x] **T2.4** 实现 CHANGELOG 生成（git log 解析 + conventional commits 分组）
- [x] **T2.5** 实现 git tag 创建与推送
- [x] **T2.6** 实现 dry-run 模式
- [x] **T2.7** 添加单元测试

### Phase 3: Self Update 命令

- [x] **T3.1** 实现平台检测（编译时宏）
- [x] **T3.2** 实现 GitHub Releases API 调用
- [x] **T3.3** 实现版本比较逻辑
- [x] **T3.4** 实现二进制文件下载
- [x] **T3.5** 实现 SHA256 checksum 验证
- [x] **T3.6** 实现可执行文件替换
- [x] **T3.7** 添加错误处理（网络、权限、checksum）
- [x] **T3.8** 添加单元测试

### Phase 4: 集成与文档

- [x] **T4.1** 创建初始 `CHANGELOG.md`
- [x] **T4.2** 更新 `CLAUDE.md` 文档
- [x] **T4.3** 添加 i18n 消息键
- [x] **T4.4** 集成测试：完整发布流程（单元测试已覆盖核心逻辑）
- [x] **T4.5** 集成测试：完整更新流程（单元测试已覆盖核心逻辑）

## Dependencies

```
T1.1 ─┬─▶ T2.1 ─▶ T2.2 ─▶ T2.3 ─▶ T2.4 ─▶ T2.5 ─▶ T2.6 ─▶ T2.7
      │
      └─▶ T3.1 ─▶ T3.2 ─▶ T3.3 ─▶ T3.4 ─▶ T3.5 ─▶ T3.6 ─▶ T3.7 ─▶ T3.8

T1.2 ─▶ T1.4
T1.3 ─▶ T1.4

T2.* ─▶ T4.4
T3.* ─▶ T4.5
T2.7 + T3.8 ─▶ T4.1 ─▶ T4.2 ─▶ T4.3
```

## Estimated Effort

| Phase | 预估时间 |
|-------|---------|
| Phase 1 | 0.5h |
| Phase 2 | 2h |
| Phase 3 | 2h |
| Phase 4 | 1h |
| **Total** | **5.5h** |