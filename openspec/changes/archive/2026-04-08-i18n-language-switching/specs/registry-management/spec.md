## MODIFIED Requirements

### Requirement: CLI output messages
所有 CLI 命令的用户可见输出消息 SHALL 通过 i18n 模块提供，支持中英文切换。

#### Scenario: Init command Chinese output
- **WHEN** 语言设为 `zh` 且 `skillsync init` 成功
- **THEN** SHALL 输出 "已初始化 SkillSync registry 至 {path}"

#### Scenario: Add command Chinese output
- **WHEN** 语言设为 `zh` 且 `skillsync add` 成功添加 skill
- **THEN** SHALL 输出 "已添加 skill '{name}' 到 registry"

#### Scenario: Error messages Chinese output
- **WHEN** 语言设为 `zh` 且 registry 未初始化时运行命令
- **THEN** SHALL 输出 "Registry 未找到。请先运行 'skillsync init'。"

### Requirement: Doctor output
`skillsync doctor` 的检查结果 SHALL 通过 i18n 模块显示。

#### Scenario: Doctor Chinese output
- **WHEN** 语言设为 `zh` 且运行 `skillsync doctor`
- **THEN** 检查项标题和结果 SHALL 显示中文（如 "Registry 存在"、"manifest.yaml 解析正常"）
