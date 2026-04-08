## ADDED Requirements

### Requirement: Language detection
系统 SHALL 按以下优先级检测语言：
1. 环境变量 `SKILLSYNC_LANG`（值为 `zh` 或 `en`）
2. `~/.skillsync/.lang` 配置文件（首次 `skillsync init` 时交互式生成）
3. 系统环境变量 `LANG` 或 `LC_ALL`（包含 `zh` 则为中文）
4. 默认为 `en`

语言检测 SHALL 在程序启动时执行一次，结果缓存供后续所有输出使用。

#### Scenario: Explicit SKILLSYNC_LANG=zh
- **WHEN** 环境变量 `SKILLSYNC_LANG` 设为 `zh`
- **THEN** 所有用户可见输出 SHALL 使用中文

#### Scenario: Explicit SKILLSYNC_LANG=en
- **WHEN** 环境变量 `SKILLSYNC_LANG` 设为 `en`
- **THEN** 所有用户可见输出 SHALL 使用英文

#### Scenario: System locale fallback
- **WHEN** `SKILLSYNC_LANG` 未设置且系统 `LANG` 为 `zh_CN.UTF-8`
- **THEN** 所有用户可见输出 SHALL 使用中文

#### Scenario: Default to English
- **WHEN** `SKILLSYNC_LANG` 未设置且系统 `LANG` 不包含 `zh`
- **THEN** 所有用户可见输出 SHALL 使用英文

#### Scenario: Interactive language selection during init
- **WHEN** 用户首次运行 `skillsync init` 且 `SKILLSYNC_LANG` 未设置
- **THEN** 系统显示语言选择提示 "请选择您偏好的语言："（中文）或 "Select your preferred language:"（英文）
- **AND** 用户选择后，语言偏好保存至 `~/.skillsync/.lang`

### Requirement: Translation macro
系统 SHALL 提供 `t!()` 宏用于获取当前语言的翻译文本。宏接受一个消息键枚举值，返回对应语言的 `String`。

#### Scenario: Basic translation
- **WHEN** 代码调用 `t!(Msg::InitSuccess { path })`
- **THEN** SHALL 返回当前语言对应的字符串（英文或中文）

#### Scenario: Compile-time safety
- **WHEN** 使用了不存在的消息键
- **THEN** 编译 SHALL 失败并报错

### Requirement: Parameterized messages
对于包含动态参数的消息，`t!()` 宏返回的字符串 SHALL 包含 `{}` 占位符，调用处使用 `format!()` 填充参数。中英文翻译的占位符数量和顺序 MUST 一致。

#### Scenario: Message with parameters
- **WHEN** 消息模板为 `"Added skill '{}' to registry"` / `"已添加 skill '{}' 到 registry"`
- **THEN** 两种语言的模板 SHALL 都包含一个 `{}` 占位符

### Requirement: Full text coverage
所有用户可见的输出文本 SHALL 通过 i18n 模块提供，包括：
- CLI 命令输出消息（success、error、warning）
- TUI 交互提示（问题、选项标签、帮助文本）
- 文件监控器状态消息
- 安装器进度消息

以下文本不在翻译范围内：
- clap `--help` 自动生成的帮助文本
- 调试日志
- 内部错误消息（panic 等）

#### Scenario: CLI output in Chinese
- **WHEN** 语言设为 `zh` 且用户运行 `skillsync init`
- **THEN** 成功消息 SHALL 显示中文（如 "已初始化 SkillSync registry"）

#### Scenario: TUI prompts in Chinese
- **WHEN** 语言设为 `zh` 且用户运行 `skillsync use`
- **THEN** 配置方式选择提示 SHALL 显示中文（如 "请选择配置方式"）

#### Scenario: Error messages in Chinese
- **WHEN** 语言设为 `zh` 且发生错误
- **THEN** 错误信息 SHALL 显示中文
