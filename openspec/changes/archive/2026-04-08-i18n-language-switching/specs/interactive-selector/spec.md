## MODIFIED Requirements

### Requirement: Configuration method selection
TUI 配置方式选择 SHALL 通过 i18n 模块显示提示和选项标签。

#### Scenario: Chinese TUI prompts
- **WHEN** 语言设为 `zh` 且用户运行 `skillsync use`
- **THEN** 提示问题 SHALL 显示 "请选择配置方式"
- **AND** 选项 SHALL 显示为 "从 Profile 选择"、"手动选择"、"从项目复制"

#### Scenario: English TUI prompts
- **WHEN** 语言设为 `en` 且用户运行 `skillsync use`
- **THEN** 提示问题 SHALL 显示 "How would you like to configure this project?"
- **AND** 选项 SHALL 显示为 "From profile"、"Manual"、"Copy project"

### Requirement: Resource selection
资源多选界面 SHALL 通过 i18n 模块显示提示和帮助文本。

#### Scenario: Chinese resource selection
- **WHEN** 语言设为 `zh`
- **THEN** 提示 SHALL 显示 "选择要安装的资源："
- **AND** 帮助文本 SHALL 显示 "输入筛选，空格切换，回车确认"

### Requirement: Conflict resolution
冲突解决界面 SHALL 通过 i18n 模块显示选项标签。

#### Scenario: Chinese conflict resolution
- **WHEN** 语言设为 `zh` 且存在合并冲突
- **THEN** 选项 SHALL 显示为 "保留本地"、"使用远程"、"手动编辑"
