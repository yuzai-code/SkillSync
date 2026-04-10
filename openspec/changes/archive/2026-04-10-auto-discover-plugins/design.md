## Context

当前 `sync` 命令通过 `scan_all_local_skills()` 扫描 skills，但 plugins 和 MCP servers 没有类似的发现机制。这些资源存储在：
- Plugins: `~/.claude/settings.json` 的 `enabledPlugins` 和 `extraKnownMarketplaces`
- MCP Servers: `~/.claude/.mcp.json` 的 `mcpServers`

## Goals / Non-Goals

**Goals:**
- 从 `settings.json` 扫描已启用的 plugins
- 从 `.mcp.json` 扫描已配置的 MCP servers
- 将发现的资源注册到 manifest
- 在 `list` 和 `use` 命令中显示来源

**Non-Goals:**
- 不扫描项目级的 plugins/MCP（暂不支持）
- 不自动安装 plugins/MCP 到其他机器（仅注册元数据）

## Decisions

### 1. 扫描时机
**决定**: 在 `sync` 命令中与 skills 扫描并行执行
**原因**: 用户习惯 `sync` 作为"同步所有"命令，保持一致性

### 2. Plugin 元数据存储
**决定**: 从 `extraKnownMarketplaces` 提取 marketplace 信息，存储为 `PluginEntry`
**原因**: `enabledPlugins` 格式为 `name@marketplace`，需要关联 marketplace 信息用于后续安装

### 3. MCP Server 元数据存储
**决定**: 从 `.mcp.json` 提取 command/args，存储为 `McpServerEntry`
**原因**: MCP 配置本身就是完整定义，直接复制即可

### 4. 来源标识
**决定**: 使用 `source_path` 字段存储 `"global"` 作为来源标识
**原因**: 与 skills 的来源标识一致，便于统一显示

## Risks / Trade-offs

- **风险**: `settings.json` 格式变化 → 缓解：使用 JSON 解析容错，缺失字段时跳过
- **风险**: 用户手动编辑配置导致格式错误 → 缓解：捕获解析错误，打印警告继续执行