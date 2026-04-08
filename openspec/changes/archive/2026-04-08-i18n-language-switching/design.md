## Context

SkillSync 当前有 350+ 条用户可见的英文字符串，分布在 `cli/`（16 个文件）、`tui/`（3 个文件）、`watcher/`（1 个文件）、`installer/`（1 个文件）中。字符串直接硬编码在代码中，无任何 i18n 基础设施。

项目是一个 Rust CLI 工具，单二进制分发，当前编译产物 3.7MB。需要在不显著增加体积和复杂度的前提下支持中英文切换。

## Goals / Non-Goals

**Goals:**
- 支持英文（en）和中文（zh）两种语言
- 通过 `SKILLSYNC_LANG` 环境变量或系统 locale 自动检测语言
- 默认英文，确保向后兼容
- 所有用户可见文本（CLI 输出、TUI 提示、错误信息）均支持翻译
- 翻译文本编译时嵌入二进制，零运行时文件依赖

**Non-Goals:**
- 不支持第三方语言包或动态加载翻译文件
- 不支持 en/zh 以外的语言（但架构应允许未来扩展）
- 不翻译 clap 的 `--help` 输出（clap derive 宏限制，且 help 通常保持英文）
- 不翻译日志级别的调试信息

## Decisions

### 1. i18n 实现方案：宏 + 静态映射

**选择**：自定义 `t!()` 宏 + 枚举键 + 静态翻译映射

**替代方案**：
- `rust-i18n` crate：功能全面但依赖重，引入 YAML/JSON 翻译文件和 proc macro
- `fluent-rs`：Mozilla 的方案，对 CLI 工具来说太重
- 纯字符串 HashMap：运行时开销，无编译时检查

**理由**：CLI 工具只需两种语言，350 条文本。宏方案零运行时开销，编译时类型检查，代码改动小（`println!("msg")` → `println!("{}", t!(msg_key))`）。翻译直接在 Rust 源码中维护，无需额外工具链。

### 2. 语言检测优先级

```
SKILLSYNC_LANG=zh  >  系统 LANG/LC_ALL  >  默认 en
```

**理由**：环境变量最高优先级让用户显式控制；系统 locale 作为 fallback 提供开箱即用的体验。不引入 `sys-locale` crate，直接读取 `LANG` 环境变量解析即可（检查是否包含 `zh`）。

### 3. 翻译文本组织

翻译按模块分组，使用枚举键：

```rust
// src/i18n/mod.rs
pub enum Msg {
    // cli/init
    InitRegistryExists,
    InitSuccess,
    // tui/selector
    SelectorConfigPrompt,
    SelectorFromProfile,
    // ...
}
```

每个枚举值对应 `(en, zh)` 文本对。使用 `match` 分发，编译器保证所有键都有翻译。

### 4. 带参数的翻译

对于包含 `{}` 占位符的消息，`t!()` 宏返回模板字符串，调用处使用 `format!`：

```rust
// 之前
println!("Added skill '{}' to registry", name);
// 之后
println!("{}", format!(t!(AddSkillSuccess), name));
```

### 5. clap 描述文本的处理

clap derive 宏的 `about`/`help` 属性要求编译时字面量，无法使用运行时函数。

**选择**：保持 clap help 为英文，不翻译。

**理由**：CLI help 文本是开发者约定（`--help` 输出保持英文是行业惯例），且 clap derive 的技术限制使得翻译成本很高（需要改为 builder API）。

## Risks / Trade-offs

- **风险**：350+ 条文本的翻译工作量大，可能出现遗漏 → 分批迁移，先覆盖 TUI 和核心 CLI 命令，用 `grep` 扫描残留硬编码字符串
- **风险**：枚举键命名不一致 → 建立命名规范：`模块_动作_细节`（如 `InitSuccess`、`SelectorConfigPrompt`）
- **权衡**：编译时嵌入翻译增加二进制体积（~20KB 文本数据）→ 可接受，远小于当前 3.7MB
- **权衡**：不翻译 `--help` → 用户体验略有不一致，但避免了大量 clap 重构
