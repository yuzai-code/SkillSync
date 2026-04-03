// Integration tests for the SkillSync CLI workflow.
//
// These tests exercise the core data layer functions in combination,
// simulating the init -> add -> list -> use -> install flow without
// relying on `dirs::home_dir()` or the real filesystem.

use std::collections::HashMap;
use std::fs;

use tempfile::tempdir;

use skillsync::installer::mcp_installer::merge_mcp_config;
use skillsync::installer::settings_merger::{
    read_lock_file, write_lock_file, write_skillsync_config, LockEntry,
};
use skillsync::installer::skill_installer::{
    install_project_skills, install_skill, InstallResult,
};
use skillsync::registry::{
    compute_hash, copy_resource, Manifest, McpServerEntry, PluginEntry, ProfileConfig, ProfileRef,
    ResourceScope, SkillEntry, SkillSyncConfig, SkillType,
};
use skillsync::claude::paths::{ProjectPaths, SkillSyncPaths};
use skillsync::claude::settings::{enable_plugin, load_settings, save_settings};
use skillsync::claude::hooks::{has_hook, install_hook, remove_hook};
use skillsync::state::StateDb;

// =========================================================================
// 10.3 -- Full workflow integration test
// =========================================================================

/// Simulate the complete init -> add -> list -> install workflow using
/// the underlying library functions instead of the CLI entry points
/// (which depend on `dirs::home_dir()`).
#[test]
fn test_full_workflow_init_add_list_install() {
    let base = tempdir().unwrap();

    // --- Phase 1: Init -- create registry structure ---
    let ss_paths = SkillSyncPaths::with_root(base.path().join(".skillsync"));
    ss_paths.ensure_registry_dirs().unwrap();

    let mut manifest = Manifest::default_empty();
    manifest.save(&ss_paths.manifest).unwrap();

    assert!(ss_paths.registry_exists());

    // --- Phase 2: Add a skill ---
    let skill_src = base.path().join("my-skills").join("code-review");
    fs::create_dir_all(&skill_src).unwrap();
    fs::write(
        skill_src.join("CLAUDE.md"),
        "# Code Review\nYou are a code review expert.",
    )
    .unwrap();
    fs::write(
        skill_src.join("prompts.md"),
        "Review for bugs and style issues.",
    )
    .unwrap();

    // Copy into registry resources.
    let dest = ss_paths.skills_dir.join("code-review");
    copy_resource(&skill_src, &dest).unwrap();

    let hash = compute_hash(&dest).unwrap();

    manifest.skills.insert(
        "code-review".into(),
        SkillEntry {
            skill_type: SkillType::Custom,
            scope: ResourceScope::Shared,
            version: "0.1.0".into(),
            path: "resources/skills/code-review".into(),
            description: Some("Code review skill".into()),
            tags: vec!["review".into(), "quality".into()],
            source: None,
            backup_hash: Some(hash.clone()),
        },
    );
    manifest.save(&ss_paths.manifest).unwrap();

    // --- Phase 3: Add an MCP server ---
    manifest.mcp_servers.insert(
        "openspec".into(),
        McpServerEntry {
            command: "npx".into(),
            args: vec!["-y".into(), "@fission-ai/openspec-mcp".into()],
            scope: ResourceScope::Shared,
        },
    );
    manifest.save(&ss_paths.manifest).unwrap();

    // --- Phase 4: Add a plugin ---
    manifest.plugins.insert(
        "superpowers".into(),
        PluginEntry {
            marketplace: "claude-plugins-official".into(),
            version: "latest".into(),
            git_sha: None,
            repo: None,
        },
    );
    manifest.save(&ss_paths.manifest).unwrap();

    // --- Phase 5: List -- verify all resources are in the manifest ---
    let loaded = Manifest::load(&ss_paths.manifest).unwrap();
    assert!(loaded.validate().is_ok());
    assert_eq!(loaded.skills.len(), 1);
    assert_eq!(loaded.plugins.len(), 1);
    assert_eq!(loaded.mcp_servers.len(), 1);
    assert!(loaded.skills.contains_key("code-review"));
    assert!(loaded.plugins.contains_key("superpowers"));
    assert!(loaded.mcp_servers.contains_key("openspec"));

    // --- Phase 6: Use -- write project skillsync.yaml ---
    let project_dir = base.path().join("my-project");
    let project = ProjectPaths::new(&project_dir);
    project.ensure_dirs().unwrap();

    let config = SkillSyncConfig {
        profile: None,
        skills: vec!["code-review".into()],
        plugins: vec!["superpowers@claude-plugins-official".into()],
        mcp: vec!["openspec".into()],
    };
    write_skillsync_config(&config, &project.skillsync_yaml).unwrap();

    // Verify round-trip.
    let loaded_config = SkillSyncConfig::load(&project.skillsync_yaml).unwrap();
    assert_eq!(loaded_config.skills, vec!["code-review"]);
    assert_eq!(loaded_config.mcp, vec!["openspec"]);

    // --- Phase 7: Install skills ---
    let skill_results = install_project_skills(
        &config.skills,
        &loaded,
        &ss_paths.registry,
        &project.skills_dir,
    )
    .unwrap();

    assert_eq!(skill_results.len(), 1);
    assert!(matches!(
        skill_results[0],
        InstallResult::Installed(ref n) if n == "code-review"
    ));

    // Verify the skill was copied.
    assert!(project
        .skills_dir
        .join("code-review")
        .join("CLAUDE.md")
        .exists());
    let content =
        fs::read_to_string(project.skills_dir.join("code-review").join("CLAUDE.md")).unwrap();
    assert!(content.contains("code review expert"));

    // --- Phase 8: Install MCP servers ---
    let mcp_servers: HashMap<String, McpServerEntry> = config
        .mcp
        .iter()
        .filter_map(|name| loaded.mcp_servers.get(name).map(|e| (name.clone(), e.clone())))
        .collect();

    merge_mcp_config(&mcp_servers, &project.mcp_json).unwrap();

    // Verify .mcp.json was created.
    assert!(project.mcp_json.exists());
    let mcp_content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project.mcp_json).unwrap()).unwrap();
    assert!(mcp_content["mcpServers"]["openspec"].is_object());
    assert_eq!(mcp_content["mcpServers"]["openspec"]["command"], "npx");

    // --- Phase 9: Re-install should skip (already up-to-date) ---
    let second_install = install_project_skills(
        &config.skills,
        &loaded,
        &ss_paths.registry,
        &project.skills_dir,
    )
    .unwrap();

    assert!(matches!(second_install[0], InstallResult::Skipped(_)));
}

// =========================================================================
// Manifest round-trip with save + load + validate
// =========================================================================

#[test]
fn test_manifest_save_load_validate_cycle() {
    let dir = tempdir().unwrap();
    let manifest_path = dir.path().join("manifest.yaml");

    let mut manifest = Manifest::default_empty();

    // Add various resources.
    manifest.skills.insert(
        "skill-a".into(),
        SkillEntry {
            skill_type: SkillType::Custom,
            scope: ResourceScope::Global,
            version: "1.0.0".into(),
            path: "resources/skills/skill-a".into(),
            description: Some("Skill A".into()),
            tags: vec!["test".into()],
            source: None,
            backup_hash: Some("sha256:abc".into()),
        },
    );

    manifest.plugins.insert(
        "plugin-x".into(),
        PluginEntry {
            marketplace: "official".into(),
            version: "2.0.0".into(),
            git_sha: Some("deadbeef".into()),
            repo: Some("https://github.com/example/plugin-x".into()),
        },
    );

    manifest.mcp_servers.insert(
        "mcp-tool".into(),
        McpServerEntry {
            command: "node".into(),
            args: vec!["server.js".into(), "--port".into(), "3000".into()],
            scope: ResourceScope::Shared,
        },
    );

    manifest.profiles.insert(
        "dev".into(),
        ProfileRef {
            path: "profiles/dev.yaml".into(),
        },
    );

    // Save, load, validate.
    manifest.save(&manifest_path).unwrap();
    let loaded = Manifest::load(&manifest_path).unwrap();
    assert!(loaded.validate().is_ok());

    // Verify all data came back.
    assert_eq!(loaded.version, 1);
    let skill = &loaded.skills["skill-a"];
    assert_eq!(skill.version, "1.0.0");
    assert_eq!(skill.description.as_deref(), Some("Skill A"));
    assert_eq!(skill.backup_hash.as_deref(), Some("sha256:abc"));

    let plugin = &loaded.plugins["plugin-x"];
    assert_eq!(plugin.marketplace, "official");
    assert_eq!(plugin.git_sha.as_deref(), Some("deadbeef"));
    assert_eq!(
        plugin.repo.as_deref(),
        Some("https://github.com/example/plugin-x")
    );

    let mcp = &loaded.mcp_servers["mcp-tool"];
    assert_eq!(mcp.command, "node");
    assert_eq!(mcp.args, vec!["server.js", "--port", "3000"]);
}

// =========================================================================
// SkillSyncConfig write -> load round-trip
// =========================================================================

#[test]
fn test_skillsync_config_write_load_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(".claude").join("skillsync.yaml");

    let config = SkillSyncConfig {
        profile: Some("my-profile".into()),
        skills: vec!["a".into(), "b".into(), "c".into()],
        plugins: vec!["p1@market".into()],
        mcp: vec!["server-1".into(), "server-2".into()],
    };

    write_skillsync_config(&config, &path).unwrap();
    let loaded = SkillSyncConfig::load(&path).unwrap();

    assert_eq!(loaded.profile, Some("my-profile".into()));
    assert_eq!(loaded.skills.len(), 3);
    assert_eq!(loaded.plugins, vec!["p1@market"]);
    assert_eq!(loaded.mcp.len(), 2);
}

// =========================================================================
// MCP merge does not delete existing entries
// =========================================================================

#[test]
fn test_mcp_merge_preserves_all_existing_entries() {
    let dir = tempdir().unwrap();
    let mcp_path = dir.path().join(".mcp.json");

    // Write three existing servers.
    let existing = serde_json::json!({
        "mcpServers": {
            "server-1": {"command": "cmd1", "args": []},
            "server-2": {"command": "cmd2", "args": ["--flag"]},
            "server-3": {"command": "cmd3", "args": ["a", "b"]}
        }
    });
    fs::write(
        &mcp_path,
        serde_json::to_string_pretty(&existing).unwrap(),
    )
    .unwrap();

    // Merge one new server.
    let mut new_servers = HashMap::new();
    new_servers.insert(
        "server-new".to_string(),
        McpServerEntry {
            command: "new-cmd".into(),
            args: vec!["--new".into()],
            scope: ResourceScope::Shared,
        },
    );

    merge_mcp_config(&new_servers, &mcp_path).unwrap();

    let content: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&mcp_path).unwrap()).unwrap();
    let servers = content["mcpServers"].as_object().unwrap();

    // All four servers should be present.
    assert_eq!(servers.len(), 4);
    assert!(servers.contains_key("server-1"));
    assert!(servers.contains_key("server-2"));
    assert!(servers.contains_key("server-3"));
    assert!(servers.contains_key("server-new"));
}

// =========================================================================
// Skill install + update detection
// =========================================================================

#[test]
fn test_skill_install_detects_update() {
    let dir = tempdir().unwrap();

    // Create source v1.
    let src = dir.path().join("src-skill");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("CLAUDE.md"), "# Version 1").unwrap();

    let target = dir.path().join("target-skills");

    // First install.
    let r1 = install_skill(&src, &target, "my-skill").unwrap();
    assert!(matches!(r1, InstallResult::Installed(_)));

    // Same content -> skip.
    let r2 = install_skill(&src, &target, "my-skill").unwrap();
    assert!(matches!(r2, InstallResult::Skipped(_)));

    // Modify source -> update.
    fs::write(src.join("CLAUDE.md"), "# Version 2").unwrap();
    let r3 = install_skill(&src, &target, "my-skill").unwrap();
    assert!(matches!(r3, InstallResult::Updated(_)));

    // Verify target has updated content.
    let content = fs::read_to_string(target.join("my-skill").join("CLAUDE.md")).unwrap();
    assert_eq!(content, "# Version 2");
}

// =========================================================================
// Lock file round-trip
// =========================================================================

#[test]
fn test_lock_file_full_roundtrip() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join(".claude").join("skillsync.lock");

    let entries = vec![
        LockEntry {
            name: "skill-a".into(),
            resource_type: "skill".into(),
            version: "1.0.0".into(),
            hash: "sha256:aaa".into(),
        },
        LockEntry {
            name: "mcp-b".into(),
            resource_type: "mcp_server".into(),
            version: "0.0.0".into(),
            hash: "cmd:npx -y @fission-ai/openspec-mcp".into(),
        },
    ];

    write_lock_file(&entries, &lock_path).unwrap();

    let lock = read_lock_file(&lock_path).unwrap().unwrap();
    assert_eq!(lock.version, 1);
    assert_eq!(lock.resources.len(), 2);
    assert_eq!(lock.resources[0].name, "skill-a");
    assert_eq!(lock.resources[0].hash, "sha256:aaa");
    assert_eq!(lock.resources[1].name, "mcp-b");
    assert!(lock.resources[1].hash.starts_with("cmd:"));
}

// =========================================================================
// Settings + hooks integration
// =========================================================================

#[test]
fn test_settings_and_hooks_integration() {
    let dir = tempdir().unwrap();
    let settings_path = dir.path().join("settings.json");

    // Start with empty settings.
    let mut settings = load_settings(&settings_path).unwrap();
    assert_eq!(settings, serde_json::json!({}));

    // Enable plugins.
    enable_plugin(&mut settings, "plugin-a");
    enable_plugin(&mut settings, "plugin-b");
    save_settings(&settings_path, &settings).unwrap();

    // Install hook.
    assert!(!has_hook(&settings));
    install_hook(&mut settings);
    assert!(has_hook(&settings));
    save_settings(&settings_path, &settings).unwrap();

    // Reload and verify everything is present.
    let reloaded = load_settings(&settings_path).unwrap();
    let plugins = reloaded["enabledPlugins"].as_array().unwrap();
    assert_eq!(plugins.len(), 2);
    assert!(has_hook(&reloaded));

    // Remove hook.
    let mut reloaded = reloaded;
    remove_hook(&mut reloaded);
    assert!(!has_hook(&reloaded));
}

// =========================================================================
// State DB integration
// =========================================================================

#[test]
fn test_state_db_integration_with_install_workflow() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let db = StateDb::open(&db_path).unwrap();

    // Record installations that would happen during `skillsync install`.
    db.record_install(
        "code-review",
        "skill",
        "shared",
        "0.1.0",
        Some("sha256:abc123"),
        "/project/.claude/skills/code-review",
        Some("/project"),
    )
    .unwrap();

    db.record_install(
        "openspec",
        "mcp_server",
        "shared",
        "1.0.0",
        None,
        "/project/.mcp.json",
        Some("/project"),
    )
    .unwrap();

    // List project resources.
    let resources = db.list_installed(Some("/project")).unwrap();
    assert_eq!(resources.len(), 2);

    // Update one resource (simulating re-install with new version).
    db.record_install(
        "code-review",
        "skill",
        "shared",
        "0.2.0",
        Some("sha256:def456"),
        "/project/.claude/skills/code-review",
        Some("/project"),
    )
    .unwrap();

    // Should still be 2 resources (upsert, not duplicate).
    let resources = db.list_installed(Some("/project")).unwrap();
    assert_eq!(resources.len(), 2);

    let cr = db
        .get_installed("code-review", Some("/project"))
        .unwrap()
        .unwrap();
    assert_eq!(cr.version, "0.2.0");
    assert_eq!(cr.content_hash.as_deref(), Some("sha256:def456"));

    // Record sync.
    db.record_sync("install", "success", Some("Installed 2 resources"))
        .unwrap();
    let syncs = db.recent_syncs(10).unwrap();
    assert_eq!(syncs.len(), 1);
    assert_eq!(syncs[0].operation, "install");
}

// =========================================================================
// Profile save + load + apply workflow
// =========================================================================

#[test]
fn test_profile_create_and_apply_workflow() {
    let dir = tempdir().unwrap();
    let ss_paths = SkillSyncPaths::with_root(dir.path().join(".skillsync"));
    ss_paths.ensure_registry_dirs().unwrap();

    // Create a manifest with some resources.
    let mut manifest = Manifest::default_empty();
    manifest.skills.insert(
        "skill-a".into(),
        SkillEntry {
            skill_type: SkillType::Custom,
            scope: ResourceScope::Shared,
            version: "1.0.0".into(),
            path: "resources/skills/skill-a".into(),
            description: None,
            tags: vec![],
            source: None,
            backup_hash: None,
        },
    );
    manifest.mcp_servers.insert(
        "mcp-a".into(),
        McpServerEntry {
            command: "node".into(),
            args: vec!["serve.js".into()],
            scope: ResourceScope::Shared,
        },
    );

    // Create the skill files in the registry.
    let skill_dir = ss_paths.skills_dir.join("skill-a");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("CLAUDE.md"), "# Skill A").unwrap();

    // Create a profile.
    let profile = ProfileConfig {
        name: "my-profile".into(),
        description: Some("Test profile".into()),
        skills: vec!["skill-a".into()],
        plugins: vec![],
        mcp: vec!["mcp-a".into()],
    };

    let profile_path = ss_paths.profiles_dir.join("my-profile.yaml");
    profile.save(&profile_path).unwrap();

    manifest.profiles.insert(
        "my-profile".into(),
        ProfileRef {
            path: "profiles/my-profile.yaml".into(),
        },
    );
    manifest.save(&ss_paths.manifest).unwrap();

    // Load it back and verify.
    let loaded_manifest = Manifest::load(&ss_paths.manifest).unwrap();
    assert!(loaded_manifest.validate().is_ok());
    assert!(loaded_manifest.profiles.contains_key("my-profile"));

    let loaded_profile = ProfileConfig::load(&profile_path).unwrap();
    assert_eq!(loaded_profile.name, "my-profile");
    assert_eq!(loaded_profile.skills, vec!["skill-a"]);
    assert_eq!(loaded_profile.mcp, vec!["mcp-a"]);

    // Simulate applying the profile to a project.
    let project_dir = dir.path().join("project");
    let project = ProjectPaths::new(&project_dir);
    project.ensure_dirs().unwrap();

    let project_config = SkillSyncConfig {
        profile: Some("my-profile".into()),
        skills: loaded_profile.skills.clone(),
        plugins: loaded_profile.plugins.clone(),
        mcp: loaded_profile.mcp.clone(),
    };
    project_config.save(&project.skillsync_yaml).unwrap();

    assert!(project.has_config());

    // Install skills from the profile.
    let results = install_project_skills(
        &project_config.skills,
        &loaded_manifest,
        &ss_paths.registry,
        &project.skills_dir,
    )
    .unwrap();
    assert_eq!(results.len(), 1);
    assert!(matches!(
        results[0],
        InstallResult::Installed(ref n) if n == "skill-a"
    ));

    // Install MCP servers.
    let mcp_to_install: HashMap<String, McpServerEntry> = project_config
        .mcp
        .iter()
        .filter_map(|n| {
            loaded_manifest
                .mcp_servers
                .get(n)
                .map(|e| (n.clone(), e.clone()))
        })
        .collect();
    merge_mcp_config(&mcp_to_install, &project.mcp_json).unwrap();

    // Verify.
    assert!(project
        .skills_dir
        .join("skill-a")
        .join("CLAUDE.md")
        .exists());
    let mcp_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project.mcp_json).unwrap()).unwrap();
    assert_eq!(mcp_data["mcpServers"]["mcp-a"]["command"], "node");
}

// =========================================================================
// SkillSyncPaths helper tests
// =========================================================================

#[test]
fn test_skillsync_paths_registry_lifecycle() {
    let dir = tempdir().unwrap();
    let paths = SkillSyncPaths::with_root(dir.path().to_path_buf());

    // Not initialized yet.
    assert!(!paths.registry_exists());

    // Create directories.
    paths.ensure_registry_dirs().unwrap();
    assert!(!paths.registry_exists()); // still no manifest

    // Write manifest.
    let manifest = Manifest::default_empty();
    manifest.save(&paths.manifest).unwrap();
    assert!(paths.registry_exists());

    // Verify directory structure.
    assert!(paths.skills_dir.is_dir());
    assert!(paths.plugins_dir.is_dir());
    assert!(paths.mcp_dir.is_dir());
    assert!(paths.profiles_dir.is_dir());
}

// =========================================================================
// Hash consistency across install
// =========================================================================

#[test]
fn test_hash_consistent_after_copy() {
    let dir = tempdir().unwrap();

    // Create source.
    let src = dir.path().join("source");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.txt"), "hello").unwrap();
    fs::write(src.join("sub/b.txt"), "world").unwrap();

    let src_hash = compute_hash(&src).unwrap();

    // Copy to destination.
    let dst = dir.path().join("dest");
    copy_resource(&src, &dst).unwrap();

    let dst_hash = compute_hash(&dst).unwrap();

    // Hashes should match.
    assert_eq!(src_hash, dst_hash);
    assert!(src_hash.starts_with("sha256:"));
}

// =========================================================================
// Install project skills with missing skill name
// =========================================================================

#[test]
fn test_install_project_skills_missing_name_errors() {
    let dir = tempdir().unwrap();
    let manifest = Manifest::default_empty();

    let result = install_project_skills(
        &["nonexistent-skill".to_string()],
        &manifest,
        dir.path(),
        &dir.path().join("skills"),
    );

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("nonexistent-skill"));
}
