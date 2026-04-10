## Why

当前 `sync` 命令只扫描本地 skills，无法发现已安装的 plugins 和 MCP servers。用户通过插件市场安装的 plugins 存储在 `~/.claude/settings.json` 的 `enabledPlugins` 中，MCP servers 存储在 `~/.claude/.mcp.json` 中，这些资源无法被自动注册到 registry。

## What Changes

- 扩展 `sync` 命令的自动发现功能，支持扫描 plugins 和 MCP servers
- 从 `~/.claude/settings.json` 读取 `enabledPlugins` 和 `extraKnownMarketplaces`
- 从 `~/.claude/.mcp.json` 读取 MCP servers 配置
- 在 `list` 命令中显示 plugins 和 MCP servers 的来源
- 在 `use` 命令中支持选择 plugins 和 MCP servers

## Capabilities

### New Capabilities

无（功能扩展现有 capability）

### Modified Capabilities

- `registry-management`: 扩展自动发现功能，支持 plugins 和 MCP servers 的扫描与注册

## Impact

- `src/registry/discover.rs` — 新增 `scan_global_plugins()` 和 `scan_global_mcp()` 函数
- `src/cli/sync_cmd.rs` — 调用新的扫描函数
- `src/cli/list.rs` — 显示 plugins 和 MCP servers 来源
- `src/tui/selector.rs` — 选择时显示来源（已有实现，可能需要扩展）