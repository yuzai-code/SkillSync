## 1. i18n 核心模块

- [x] 1.1 创建 `src/i18n/mod.rs`：定义 `Lang` 枚举（En/Zh）、语言检测函数（读取 `SKILLSYNC_LANG` → `LANG` → 默认 en）、全局语言缓存（`OnceLock<Lang>`）
- [x] 1.2 定义 `Msg` 枚举，包含所有消息键，按模块分组注释
- [x] 1.3 实现翻译映射：`Msg` 的 `fn en(&self)` 和 `fn zh(&self)` 返回 `String`
- [x] 1.4 实现 `t!()` 宏：`t!(Msg::Key)` 返回当前语言的 `String`
- [x] 1.5 在 `src/lib.rs` 中注册 `i18n` 模块
- [x] 1.6 为语言检测和翻译映射编写单元测试

## 2. TUI 模块迁移

- [x] 2.1 迁移 `src/tui/selector.rs`：配置方式提示、选项标签、资源选择提示、安装预览、确认提示
- [x] 2.2 迁移 `src/tui/profile_picker.rs`：Profile 选择提示、帮助文本、错误提示
- [x] 2.3 迁移 `src/tui/diff_viewer.rs`：冲突解决选项标签、提示文本

## 3. CLI 核心命令迁移

- [x] 3.1 迁移 `src/cli/init.rs`：初始化成功/失败消息、扫描结果提示
- [x] 3.2 迁移 `src/cli/add.rs`：添加成功/失败消息、用法提示
- [x] 3.3 迁移 `src/cli/remove.rs`：移除成功消息、引用警告
- [x] 3.4 迁移 `src/cli/list.rs`：列表表头，空结果提示、统计信息
- [x] 3.5 迁移 `src/cli/info.rs`：详情标签、未找到提示、模糊匹配建议
- [x] 3.6 迁移 `src/cli/search.rs`：搜索结果提示

## 4. CLI 同步命令迁移

- [x] 4.1 迁移 `src/cli/pull.rs`：拉取进度、超时错误、冲突提示、结果消息
- [x] 4.2 迁移 `src/cli/push.rs`：推送进度、提交消息、结果提示
- [x] 4.3 迁移 `src/cli/sync_cmd.rs`：同步进度、冲突提示，完成消息
- [x] 4.4 迁移 `src/cli/resolve.rs`：冲突解决进度、结果消息

## 5. CLI 其他命令迁移

- [x] 5.1 迁移 `src/cli/use_cmd.rs`：项目配置提示、取消消息，成功消息
- [x] 5.2 迁移 `src/cli/install.rs`：安装进度、警告，完成消息
- [x] 5.3 迁移 `src/cli/update.rs`：更新成功/失败消息
- [x] 5.4 迁移 `src/cli/profile.rs`：Profile 操作的所有消息
- [x] 5.5 迁移 `src/cli/doctor.rs`：所有检查项标题和结果消息
- [x] 5.6 迁移 `src/cli/watch.rs`：监控启动、守护进程、服务安装消息
- [x] 5.7 迁移 `src/cli/hook.rs`：Hook 安装/移除消息

## 6. 其他模块迁移

- [x] 6.1 迁移 `src/watcher/fs_watcher.rs`：监控状态消息、自动推送消息
- [x] 6.2 迁移 `src/installer/skill_installer.rs`：安装/更新/跳过消息
- [x] 6.3 迁移 `src/registry/resource.rs`：错误提示消息

## 7. 验证与收尾

- [x] 7.1 运行全部测试确保通过
- [x] 7.2 用 `grep` 扫描残留的硬编码用户可见英文字符串（已修复 profile.rs 表头等遗漏）
- [x] 7.3 手动测试中英文切换：`SKILLSYNC_LANG=zh cargo run -- doctor` 和 `SKILLSYNC_LANG=en cargo run -- doctor`
