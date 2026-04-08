## Why

SkillSync 的目标用户包含中文用户，但目前所有 CLI 输出、TUI 交互提示、错误信息、帮助文本均为英文。用户已经要求中文输出，需要一个正式的 i18n 机制来支持中英文切换，而不是简单地把所有文本硬编码为中文。

## What Changes

- 引入轻量级 i18n 模块，管理所有用户可见文本的中英文翻译
- 支持通过环境变量 `SKILLSYNC_LANG` 或系统 locale 自动选择语言（`zh` / `en`，默认 `en`）
- 将现有 350+ 条硬编码英文字符串替换为 i18n 调用
- 涉及的模块：`cli/`（16 个子命令文件）、`tui/`（3 个文件）、`watcher/`（1 个文件）、`installer/`（1 个文件）

## Capabilities

### New Capabilities
- `i18n`: 国际化支持模块 — 语言检测、文本管理、中英文翻译映射

### Modified Capabilities
- `interactive-selector`: TUI 提示和选项标签需要支持多语言
- `registry-management`: CLI 输出消息需要支持多语言

## Impact

- **代码**：新增 `src/i18n/` 模块；修改 `src/cli/`、`src/tui/`、`src/watcher/`、`src/installer/` 中所有用户可见文本
- **依赖**：可能引入 `sys-locale` crate 用于系统语言检测（或手动读取 `LANG` 环境变量）
- **API**：无外部 API 变更
- **兼容性**：默认语言为英文，现有用户行为不变
