## 1. Plugin Discovery

- [x] 1.1 Add `scan_global_plugins()` function in `src/registry/discover.rs`
- [x] 1.2 Parse `enabledPlugins` from `~/.claude/settings.json`
- [x] 1.3 Parse `extraKnownMarketplaces` for marketplace source info
- [x] 1.4 Create `DiscoveredPlugin` struct with name, marketplace, source fields
- [x] 1.5 Add `register_discovered_plugins()` to add plugins to manifest

## 2. MCP Server Discovery

- [x] 2.1 Add `scan_global_mcp()` function in `src/registry/discover.rs`
- [x] 2.2 Parse `mcpServers` from `~/.claude/.mcp.json`
- [x] 2.3 Create `DiscoveredMcp` struct with name, command, args fields
- [x] 2.4 Add `register_discovered_mcp()` to add MCP servers to manifest

## 3. Integrate with sync Command

- [x] 3.1 Call `scan_global_plugins()` and `scan_global_mcp()` in `sync_cmd.rs`
- [x] 3.2 Merge discovered resources with skills in `scan_all_local_resources()`
- [x] 3.3 Add i18n messages for plugin/MCP discovery results
- [x] 3.4 Handle parsing errors gracefully with warnings

## 4. Display Source in list Command

- [x] 4.1 Update `list.rs` to show "marketplace" for plugins
- [x] 4.2 Update `list.rs` to show "config" for MCP servers
- [x] 4.3 Ensure source column width accommodates new source types

## 5. Display Source in use Command

- [x] 5.1 Update `selector.rs` `build_resource_options()` for plugins
- [x] 5.2 Update `selector.rs` `build_resource_options()` for MCP servers
- [x] 5.3 Update `confirm_preview()` to show source for all resource types

## 6. Testing

- [x] 6.1 Add unit tests for `scan_global_plugins()`
- [x] 6.2 Add unit tests for `scan_global_mcp()`
- [x] 6.3 Test sync command with plugins and MCP servers
- [x] 6.4 Verify list and use display correct source info