//! i18n — Internationalization support for SkillSync.
//!
//! Provides language detection and translation for all user-visible messages.
//! Language priority: `SKILLSYNC_LANG` env var → `~/.skillsync/.lang` → system `LANG`/`LC_ALL` → default `en`.

use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Lang
// ---------------------------------------------------------------------------

/// Supported language codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
}

impl Lang {
    /// Detect the current language from environment and config file.
    ///
    /// Priority:
    /// 1. `SKILLSYNC_LANG` env var (`zh` or `en`)
    /// 2. `~/.skillsync/.lang` config file
    /// 3. System `LANG` / `LC_ALL` env vars (contains `zh` → `zh`)
    /// 4. Default: `en`
    pub fn detect() -> Lang {
        if let Some(val) = std::env::var_os("SKILLSYNC_LANG") {
            if let Some(s) = val.to_str() {
                let s = s.trim().to_lowercase();
                if s == "zh" {
                    return Lang::Zh;
                }
                if s == "en" {
                    return Lang::En;
                }
            }
        }

        if let Some(home) = dirs::home_dir() {
            let lang_file = home.join(".skillsync").join(".lang");
            if let Ok(content) = std::fs::read_to_string(&lang_file) {
                let content = content.trim().to_lowercase();
                if content == "zh" {
                    return Lang::Zh;
                }
                if content == "en" {
                    return Lang::En;
                }
            }
        }

        for var in &["LC_ALL", "LANG"] {
            if let Some(val) = std::env::var_os(var) {
                if let Some(s) = val.to_str() {
                    if s.to_lowercase().contains("zh") {
                        return Lang::Zh;
                    }
                }
            }
        }

        Lang::En
    }

    /// Returns the language tag string for this language.
    pub fn tag(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }

    /// Save the language preference to `~/.skillsync/.lang`.
    pub fn save_preference(self) -> std::io::Result<()> {
        if let Some(home) = dirs::home_dir() {
            let path = home.join(".skillsync").join(".lang");
            std::fs::write(path, self.tag())?;
        }
        Ok(())
    }
}

/// Global language cache — initialized once on first use.
static LANG_CACHE: OnceLock<Lang> = OnceLock::new();

/// Get the current language (cached after first call).
pub fn lang() -> Lang {
    *LANG_CACHE.get_or_init(Lang::detect)
}

// ---------------------------------------------------------------------------
// Msg
// ---------------------------------------------------------------------------

/// All user-visible message keys.
///
/// Each variant may carry data for parameterized messages.
/// The `en()` and `zh()` methods return the corresponding translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    // ── Shared / Context ─────────────────────────────────────────────────
    ContextFailedToLoadManifest,
    ContextFailedToSaveManifest,
    ContextFailedToOpenRepo,
    ContextHomeDir,
    ContextCurrentDir,
    ContextReadDir { path: String },
    ContextCreateDir { path: String },
    ContextResolvePaths,

    // ── cli::init ──────────────────────────────────────────────────────────
    InitRegistryExists { path: String },
    InitSuccess { path: String },
    InitLanguageSelect,
    InitLanguageSet { lang: String },
    InitCloned { url: String },
    InitScanResult { skills: usize, plugins: usize, mcp: usize, profiles: usize },
    InitScanSkillsError { error: String },
    InitScanMcpError { error: String },
    InitNoResourcesFound,
    InitFoundResources { count: usize },
    InitResourceItem { kind: String, name: String, detail: String },
    InitAddHint { cmd: String },

    // ── cli::add ───────────────────────────────────────────────────────────
    AddInvalidScope { scope: String },
    AddPathNotExist { path: String },
    AddSkillAlreadyExists { name: String },
    AddSkillSuccess { name: String },
    AddInvalidPluginFormat { input: String },
    AddPluginAlreadyExists { name: String },
    AddPluginSuccess { name: String },
    AddMcpRequiresCommand,
    AddMcpAlreadyExists { name: String },
    AddMcpSuccess { name: String },
    AddNothingToAdd,

    // ── cli::remove ───────────────────────────────────────────────────────
    RemoveResourceNotFound { name: String },
    RemoveReferencedByProfiles { name: String, profiles: String },
    RemoveUpdateProfilesHint,
    RemoveSuccess { kind: String, name: String },

    // ── cli::list ─────────────────────────────────────────────────────────
    ListInvalidTypeFilter { filter: String },
    ListNoResourcesOfType { kind: String },
    ListNoResources,
    ListUseAddHint { cmd: String },
    ListColName,
    ListColType,
    ListColScope,
    ListColVersion,
    ListTotal { count: usize },

    // ── cli::info ─────────────────────────────────────────────────────────
    InfoSkillLabel { name: String },
    InfoType,
    InfoScope,
    InfoVersion,
    InfoPath,
    InfoDescription,
    InfoTags,
    InfoSource,
    InfoMarketplace,
    InfoPlugin,
    InfoSkill,
    InfoHash,
    InfoProfiles,
    InfoPluginLabel { name: String },
    InfoPluginMarketplace,
    InfoGitSha,
    InfoRepo,
    InfoMcpLabel { name: String },
    InfoCommand,
    InfoArgs,
    InfoNotFound { name: String },
    InfoDidYouMean,
    InfoSuggestion { name: String },
    InfoUseListHint { cmd: String },

    // ── cli::search ────────────────────────────────────────────────────────
    SearchNoResults { query: String },
    SearchResults { count: usize, query: String },
    SearchResultRow { kind: String, name: String, desc: String },
    SearchMatchedOn { field: String },

    // ── cli::pull ─────────────────────────────────────────────────────────
    PullRegistryNotFound { path: String },
    PullNoOrigin,
    PullFetching,
    PullTimedOut { secs: u64 },
    PullMergeConflicts { count: usize },
    PullConflictFile { file: String },
    PullResolveHint,
    PullConflicts,
    PullUpToDate,
    PullFastForwarded,
    PullMerged,

    // ── cli::push ─────────────────────────────────────────────────────────
    PushRegistryNotFound { path: String },
    PushNoOrigin,
    PushNothingToPush,
    PushCommitting { count: usize },
    PushPushing,
    PushSuccess { count: usize },
    PushCommitFile { file: String },

    // ── cli::sync_cmd ─────────────────────────────────────────────────────
    SyncRegistryNotFound { path: String },
    SyncNoOrigin,
    SyncSyncing,
    SyncFetching,
    SyncMergeConflicts { count: usize },
    SyncConflictFile { file: String },
    SyncResolveHint,
    SyncAborted,
    SyncUpToDate,
    SyncFastForwarded,
    SyncMerged,
    SyncNoLocalChanges,
    SyncComplete,
    SyncPushing { count: usize },
    SyncPushed { count: usize },
    SyncCommitFile { file: String },

    // ── cli::resolve ──────────────────────────────────────────────────────
    ResolveNotInitialized,
    ResolveNoConflicts,
    ResolveFound { count: usize },
    ResolveConflictEntry { action: String, file: String },
    ResolveKeptLocal { file: String },
    ResolveUsedRemote { file: String },
    ResolveManuallyEdited { file: String },
    ResolveSuccess { count: usize },
    ResolveEditorLaunch { editor: String },
    ResolveEditorFailed { editor: String },

    // ── cli::use_cmd ─────────────────────────────────────────────────────
    UseConfiguring { path: String },
    UseCancelled,
    UseSuccess,
    UseProfilePreSelects { profile: String, count: usize },
    UseNoSiblingProjects,
    UseFallbackManual,
    UseSelectProject,
    UseLoadedResources { count: usize, project: String },
    UseMcpNotFound { name: String },
    UseMergedMcp { count: usize, path: String },
    UseWrote { path: String },

    // ── cli::install ─────────────────────────────────────────────────────
    InstallGlobal,
    InstallMcpMerged { count: usize, path: String },
    InstallGlobalComplete,
    InstallNoConfig { path: String },
    InstallProject { path: String },
    InstallMcpNotFound { name: String },
    InstallLockWritten { path: String },
    InstallProjectComplete,

    // ── cli::update ───────────────────────────────────────────────────────
    UpdateSourceNotExist { path: String },
    UpdateSkillSuccess { name: String, old_ver: String, new_ver: String },
    UpdatePluginSuccess { name: String, old_ver: String, new_ver: String },
    UpdateMcpAlreadyRegistered { name: String },
    UpdateNotFound { name: String },

    // ── cli::profile ─────────────────────────────────────────────────────
    ProfileListEmpty,
    ProfileListHint { cmd: String },
    ProfileErrorLoading { path: String },
    ProfileColName,
    ProfileColDesc,
    ProfileColSkills,
    ProfileColPlugins,
    ProfileColMcp,
    ProfileTotal { count: usize },
    ProfileAlreadyExists { name: String },
    ProfileCreateSuccess { name: String, path: String },
    ProfileEditHint { path: String },
    ProfileExportHint { cmd: String },
    ProfileNotFound { name: String },
    ProfileInstallSkill { name: String },
    ProfileSkillSourceNotFound { name: String, path: String },
    ProfileSkillNotFound { name: String },
    ProfileInstallMcp { name: String },
    ProfileMcpNotFound { name: String },
    ProfileApplySuccess { name: String, skills: usize, plugins: usize, mcp: usize },
    ProfileConfigWritten { path: String },
    ProfileExportNoConfig,
    ProfileExportSuccess { name: String },
    ProfileExportSummary { skills: usize, plugins: usize, mcp: usize },
    ProfileSavedTo { path: String },

    // ── cli::doctor ──────────────────────────────────────────────────────
    DoctorTitle,
    DoctorRegistryExists,
    DoctorRegistryNotFound,
    DoctorRunInitHint { cmd: String },
    DoctorManifestParseFailed { error: String },
    DoctorManifestValid,
    DoctorValidationIssues { count: usize },
    DoctorValidationError { error: String },
    DoctorOriginConfigured { url: String },
    DoctorNoOrigin,
    DoctorNotGitRepo,
    DoctorClaudeHomeExists,
    DoctorClaudeHomeNotFound,
    DoctorNoOrphaned,
    DoctorOrphanedSkills { count: usize },
    DoctorOrphanedSkill { name: String },
    DoctorOrphanedReadError,
    DoctorManifestSkillsOk,
    DoctorMissingSkills { count: usize },
    DoctorMissingSkill { name: String },
    DoctorAllPassed,
    DoctorIssues { count: usize },
    DoctorWarnings { count: usize },

    // ── cli::watch ───────────────────────────────────────────────────────
    WatchNoDirs,
    WatchStarting,
    WatchDaemonStarted { pid: u32 },
    WatchLogs { path: String },
    WatchErrors { path: String },
    WatchStopWith { cmd1: String, cmd2: String },
    WatchDaemonFailed { error: String },
    WatchTryManual { cmd: String },
    WatchServiceNotSupported,
    WatchWrotePlist { path: String },
    WatchServiceLoaded,
    WatchLaunchctlWarning,
    WatchLaunchctlHint { path: String },
    WatchWroteService { path: String },
    WatchSystemctlReloadFailed,
    WatchServiceEnabled,
    WatchSystemctlEnableFailed,
    WatchSystemctlHint,
    WatchNoPlist { path: String },
    WatchLaunchctlUnloadWarning,
    WatchServiceUnloaded { path: String },
    WatchNoServiceFile { path: String },
    WatchSystemctlDisableWarning,
    WatchServiceDisabled { path: String },

    // ── cli::hook ────────────────────────────────────────────────────────
    HookInstalled { path: String },
    HookAlreadyInstalled,
    HookRemoved { path: String },
    HookNotFound,

    // ── tui::selector ────────────────────────────────────────────────────
    SelectorConfigurePrompt,
    SelectorConfigureCancelled,
    SelectorFromProfile,
    SelectorManual,
    SelectorCopyProject,
    SelectorEmpty,
    SelectorResourcesPrompt,
    SelectorResourcesCancelled,
    SelectorPreviewHeader,
    SelectorSkillsLabel,
    SelectorPluginsLabel,
    SelectorMcpLabel,
    SelectorNoResources,
    SelectorTotal { count: usize },
    SelectorApplyPrompt,
    SelectorConfirmCancelled,

    // ── tui::profile_picker ─────────────────────────────────────────────
    ProfilePickerPrompt,
    ProfilePickerCancelled,
    ProfilePickerEmpty,
    ProfilePickerLoadError { error: String },

    // ── tui::diff_viewer ────────────────────────────────────────────────
    DiffConflictHeader { file: String },
    DiffLocalRemote,
    DiffResolvePrompt,
    DiffResolveCancelled,
    DiffKeepLocal,
    DiffUseRemote,
    DiffOpenEditor,

    // ── watcher::fs_watcher ─────────────────────────────────────────────
    WatcherStarted,
    WatcherWatching { path: String },
    WatcherDirNotExist { path: String },
    WatcherDetectedChanges { count: usize },
    WatcherEventPath { path: String },
    WatcherPanicked { error: String },
    WatcherRetry,
    WatcherError { error: String },
    WatcherChannelClosed { error: String },
    WatcherAutoPushFailed { error: String },
    WatcherWillRetry,
    WatcherRegistryNotInit,
    WatcherNoChanges,
    WatcherStaging { count: usize },
    WatcherCommitted { message: String, oid: String },
    WatcherPushed,
    WatcherPushFailed { error: String },
    WatcherPushLocal,

    // ── installer::skill_installer ──────────────────────────────────────
    InstallerInstalled { name: String },
    InstallerUpdated { name: String },
    InstallerSkipped { name: String },
    InstallerSkillPathNotExist { path: String },
    InstallerUpdateFailed { name: String, path: String },
    InstallerInstallFailed { name: String, path: String },
    InstallerNotInManifest { name: String },

    // ── registry::resource ──────────────────────────────────────────────
    ResourceTypeSkill,
    ResourceTypePlugin,
    ResourceTypeMcp,
    ResourceSourceNotExist { path: String },
    ResourceCopyDirFailed { from: String, to: String },
    ResourceCopyFileFailed { from: String, to: String },
}

// ---------------------------------------------------------------------------
// Translations
// ---------------------------------------------------------------------------

impl Msg {
    /// Returns the English translation for this message.
    pub fn en(&self) -> String {
        match self {
            Msg::ContextFailedToLoadManifest => "Failed to load manifest. Have you run 'skillsync init'?".into(),
            Msg::ContextFailedToSaveManifest => "Failed to save manifest".into(),
            Msg::ContextFailedToOpenRepo => "Failed to open registry repository".into(),
            Msg::ContextHomeDir => "Could not determine home directory".into(),
            Msg::ContextCurrentDir => "Failed to determine current directory".into(),
            Msg::ContextReadDir { path } => format!("Failed to read directory: {}", path),
            Msg::ContextCreateDir { path } => format!("Failed to create directory: {}", path),
            Msg::ContextResolvePaths => "Failed to resolve Claude paths".into(),

            Msg::InitRegistryExists { path } => format!("Registry already exists at {}\nUse `skillsync sync` to update, or remove the directory to start fresh.", path),
            Msg::InitSuccess { path } => format!("Initialized new SkillSync registry at {}", path),
            Msg::InitLanguageSelect => "Select your preferred language:".into(),
            Msg::InitLanguageSet { lang } => format!("Language set to {}. This can be changed via SKILLSYNC_LANG env var.", lang),
            Msg::InitCloned { url } => format!("Cloned SkillSync registry from {}", url),
            Msg::InitScanResult { skills, plugins, mcp, profiles } => format!("{} skill(s), {} plugin(s), {} MCP server(s), {} profile(s)", skills, plugins, mcp, profiles),
            Msg::InitScanSkillsError { error } => format!("Could not scan skills: {}", error),
            Msg::InitScanMcpError { error } => format!("Could not scan MCP servers: {}", error),
            Msg::InitNoResourcesFound => "No existing Claude Code resources found to import.".into(),
            Msg::InitFoundResources { count } => format!("Found {} existing resource(s) that could be imported:", count),
            Msg::InitResourceItem { kind, name, detail } => format!("[{}] {} — {}", kind, name, detail),
            Msg::InitAddHint { cmd } => format!("Use '{}' to add these to the registry.", cmd),

            Msg::AddInvalidScope { scope } => format!("Invalid scope '{}'. Must be 'global' or 'shared'.", scope),
            Msg::AddPathNotExist { path } => format!("Path '{}' does not exist. Check the path and try again.", path),
            Msg::AddSkillAlreadyExists { name } => format!("Skill '{}' already exists in the registry. Remove it first or choose a different name.", name),
            Msg::AddSkillSuccess { name } => format!("Added skill '{}' to registry", name),
            Msg::AddInvalidPluginFormat { input } => format!("Invalid plugin format '{}'. Expected 'name@marketplace' (e.g. superpowers@claude-plugins-official).", input),
            Msg::AddPluginAlreadyExists { name } => format!("Plugin '{}' already exists in the registry. Remove it first or choose a different name.", name),
            Msg::AddPluginSuccess { name } => format!("Added plugin '{}' to registry", name),
            Msg::AddMcpRequiresCommand => "MCP server requires --command. Usage: skillsync add --mcp <name> --command <cmd> [--args <args>...]".into(),
            Msg::AddMcpAlreadyExists { name } => format!("MCP server '{}' already exists in the registry. Remove it first or choose a different name.", name),
            Msg::AddMcpSuccess { name } => format!("Added MCP server '{}' to registry", name),
            Msg::AddNothingToAdd => "Nothing to add. Provide a path, --plugin, or --mcp.\nUsage:\n  skillsync add <path>                              # add a skill\n  skillsync add --plugin name@marketplace           # add a plugin\n  skillsync add --mcp <name> --command <cmd>        # add an MCP server".into(),

            Msg::RemoveResourceNotFound { name } => format!("Resource '{}' not found in the registry.\nUse 'skillsync list' to see all registered resources.", name),
            Msg::RemoveReferencedByProfiles { name, profiles } => format!("Resource '{}' is referenced by profile(s): {}", name, profiles),
            Msg::RemoveUpdateProfilesHint => "You may need to update these profiles after removal.".into(),
            Msg::RemoveSuccess { kind, name } => format!("Removed {} '{}' from registry", kind, name),

            Msg::ListInvalidTypeFilter { filter } => format!("Invalid type filter '{}'. Must be one of: skill, plugin, mcp", filter),
            Msg::ListNoResourcesOfType { kind } => format!("No {} resources found in the registry.", kind),
            Msg::ListNoResources => "No resources found in the registry.".into(),
            Msg::ListUseAddHint { cmd } => format!("Use '{}' to add resources.", cmd),
            Msg::ListColName => "Name".into(),
            Msg::ListColType => "Type".into(),
            Msg::ListColScope => "Scope".into(),
            Msg::ListColVersion => "Version".into(),
            Msg::ListTotal { count } => format!("{} resource(s) total", count),

            Msg::InfoSkillLabel { name } => format!("Skill: {}", name),
            Msg::InfoType => "Type:".into(),
            Msg::InfoScope => "Scope:".into(),
            Msg::InfoVersion => "Version:".into(),
            Msg::InfoPath => "Path:".into(),
            Msg::InfoDescription => "Description:".into(),
            Msg::InfoTags => "Tags:".into(),
            Msg::InfoSource => "Source:".into(),
            Msg::InfoMarketplace => "Marketplace:".into(),
            Msg::InfoPlugin => "Plugin:".into(),
            Msg::InfoSkill => "Skill:".into(),
            Msg::InfoHash => "Hash:".into(),
            Msg::InfoProfiles => "Profiles:".into(),
            Msg::InfoPluginLabel { name } => format!("Plugin: {}", name),
            Msg::InfoPluginMarketplace => "Marketplace:".into(),
            Msg::InfoGitSha => "Git SHA:".into(),
            Msg::InfoRepo => "Repo:".into(),
            Msg::InfoMcpLabel { name } => format!("MCP Server: {}", name),
            Msg::InfoCommand => "Command:".into(),
            Msg::InfoArgs => "Args:".into(),
            Msg::InfoNotFound { name } => format!("Resource '{}' not found in the registry.", name),
            Msg::InfoDidYouMean => "Did you mean one of these?".into(),
            Msg::InfoSuggestion { name } => format!("  - {}", name),
            Msg::InfoUseListHint { cmd } => format!("Use '{}' to see all registered resources.", cmd),

            Msg::SearchNoResults { query } => format!("No results for '{}'.", query),
            Msg::SearchResults { count, query } => format!("Found {} result(s) for '{}':", count, query),
            Msg::SearchResultRow { kind, name, desc } => format!("  {} {} {}", kind, name, desc),
            Msg::SearchMatchedOn { field } => format!("    matched on: {}", field),

            Msg::PullRegistryNotFound { path } => format!("Registry not found at {}.\nRun 'skillsync init' or 'skillsync init --from <url>' first.", path),
            Msg::PullNoOrigin => "No remote named 'origin' in the registry repository.\nIf this is a local-only registry, there is nothing to pull.".into(),
            Msg::PullFetching => "Fetching from origin...".into(),
            Msg::PullTimedOut { secs } => format!("Pull timed out after {} seconds. Check your network connection or increase the timeout with --timeout.", secs),
            Msg::PullMergeConflicts { count } => format!("Merge conflicts detected in {} file(s):", count),
            Msg::PullConflictFile { file } => format!("  - {}", file),
            Msg::PullResolveHint => "\nResolve conflicts manually, then run 'skillsync resolve'.".into(),
            Msg::PullConflicts => "Pull completed with conflicts".into(),
            Msg::PullUpToDate => "Already up to date.".into(),
            Msg::PullFastForwarded => "Fast-forwarded to latest changes.".into(),
            Msg::PullMerged => "Merged remote changes successfully.".into(),

            Msg::PushRegistryNotFound { path } => format!("Registry not found at {}.\nRun 'skillsync init' or 'skillsync init --from <url>' first.", path),
            Msg::PushNoOrigin => "No remote named 'origin' in the registry repository.\nIf this is a local-only registry, there is nothing to push.".into(),
            Msg::PushNothingToPush => "Nothing to push — registry is clean.".into(),
            Msg::PushCommitting { count } => format!("Committing {} changed file(s)...", count),
            Msg::PushPushing => "Pushing to origin...".into(),
            Msg::PushSuccess { count } => format!("Pushed {} change(s) to remote registry.", count),
            Msg::PushCommitFile { file } => format!("  - {}", file),

            Msg::SyncRegistryNotFound { path } => format!("Registry not found at {}.\nRun 'skillsync init' or 'skillsync init --from <url>' first.", path),
            Msg::SyncNoOrigin => "No remote named 'origin' in the registry repository.\nIf this is a local-only registry, there is nothing to sync.".into(),
            Msg::SyncSyncing => "Syncing registry with remote...\n".into(),
            Msg::SyncFetching => "Fetching from origin...".into(),
            Msg::SyncMergeConflicts { count } => format!("Merge conflicts detected in {} file(s):", count),
            Msg::SyncConflictFile { file } => format!("  - {}", file),
            Msg::SyncResolveHint => "\nResolve conflicts manually, then run 'skillsync resolve'.".into(),
            Msg::SyncAborted => "Sync aborted due to merge conflicts. Run 'skillsync resolve' to fix them, then retry.".into(),
            Msg::SyncUpToDate => "Already up to date with remote.".into(),
            Msg::SyncFastForwarded => "Fast-forwarded to latest remote changes.".into(),
            Msg::SyncMerged => "Merged remote changes successfully.".into(),
            Msg::SyncNoLocalChanges => "No local changes to push.".into(),
            Msg::SyncComplete => "\nSync complete.".into(),
            Msg::SyncPushing { count } => format!("\nPushing {} local change(s)...", count),
            Msg::SyncPushed { count } => format!("Pushed {} change(s) to remote.", count),
            Msg::SyncCommitFile { file } => format!("  - {}", file),

            Msg::ResolveNotInitialized => "Registry not initialized. Run 'skillsync init' first.".into(),
            Msg::ResolveNoConflicts => "No conflicts to resolve.".into(),
            Msg::ResolveFound { count } => format!("Found {} conflicting file(s):", count),
            Msg::ResolveConflictEntry { action, file } => format!("  {} {}", action, file),
            Msg::ResolveKeptLocal { file } => format!("kept local version of {}", file),
            Msg::ResolveUsedRemote { file } => format!("applied remote version of {}", file),
            Msg::ResolveManuallyEdited { file } => format!("manually edited {}", file),
            Msg::ResolveSuccess { count } => format!("Resolved {} conflict(s) and committed.", count),
            Msg::ResolveEditorLaunch { editor } => format!("Failed to launch editor '{}'", editor),
            Msg::ResolveEditorFailed { editor } => format!("Editor '{}' exited with non-zero status. Set the EDITOR environment variable to your preferred editor.", editor),

            Msg::UseConfiguring { path } => format!("Configuring project: {}", path),
            Msg::UseCancelled => "Cancelled — no changes applied.".into(),
            Msg::UseSuccess => "Project configured successfully!".into(),
            Msg::UseProfilePreSelects { profile, count } => format!("Profile {} pre-selects {} resources. Adjust if needed:", profile, count),
            Msg::UseNoSiblingProjects => "No sibling projects with skillsync.yaml found.".into(),
            Msg::UseFallbackManual => "Falling back to manual selection.".into(),
            Msg::UseSelectProject => "Select a project to copy configuration from:".into(),
            Msg::UseLoadedResources { count, project } => format!("Loaded {} resources from project {}. Adjust if needed:", count, project),
            Msg::UseMcpNotFound { name } => format!("MCP server '{}' not found in manifest, skipping", name),
            Msg::UseMergedMcp { count, path } => format!("merged {} MCP server(s) into {}", count, path),
            Msg::UseWrote { path } => format!("wrote {}", path),

            Msg::InstallGlobal => "Installing global resources...".into(),
            Msg::InstallMcpMerged { count, path } => format!("{} MCP server(s) merged into {}", count, path),
            Msg::InstallGlobalComplete => "Global install complete.".into(),
            Msg::InstallNoConfig { path } => format!("No skillsync.yaml found at {}\nRun 'skillsync use' to configure this project, or create the file manually.", path),
            Msg::InstallProject { path } => format!("Installing resources for project at {}", path),
            Msg::InstallMcpNotFound { name } => format!("MCP server '{}' not found in registry manifest — skipping", name),
            Msg::InstallLockWritten { path } => format!("Lock file written to {}", path),
            Msg::InstallProjectComplete => "Project install complete.".into(),

            Msg::UpdateSourceNotExist { path } => format!("Source path '{}' does not exist. Provide a valid path to the updated skill.", path),
            Msg::UpdateSkillSuccess { name, old_ver, new_ver } => format!("Updated skill '{}': {} -> {}", name, old_ver, new_ver),
            Msg::UpdatePluginSuccess { name, old_ver, new_ver } => format!("Updated plugin '{}': {} -> {}", name, old_ver, new_ver),
            Msg::UpdateMcpAlreadyRegistered { name } => format!("MCP server '{}' is already registered. Update its command/args via 'skillsync remove' + 'skillsync add'.", name),
            Msg::UpdateNotFound { name } => format!("Resource '{}' not found in the registry.\nUse 'skillsync list' to see all registered resources.", name),

            Msg::ProfileListEmpty => "No profiles found in the registry.".into(),
            Msg::ProfileListHint { cmd } => format!("Use '{}' to create one.", cmd),
            Msg::ProfileErrorLoading { path } => format!("(error loading {})", path),
            Msg::ProfileColName => "Profile".into(),
            Msg::ProfileColDesc => "Description".into(),
            Msg::ProfileColSkills => "Skills".into(),
            Msg::ProfileColPlugins => "Plugins".into(),
            Msg::ProfileColMcp => "MCP".into(),
            Msg::ProfileTotal { count } => format!("{} profile(s) total", count),
            Msg::ProfileAlreadyExists { name } => format!("Profile '{}' already exists. Remove it first with 'skillsync remove' or choose a different name.", name),
            Msg::ProfileCreateSuccess { name, path } => format!("Created profile '{}' at {}", name, path),
            Msg::ProfileEditHint { path } => format!("Edit {} to add skills, plugins, and MCP servers.", path),
            Msg::ProfileExportHint { cmd } => format!("Or use '{}' to populate from a project config.", cmd),
            Msg::ProfileNotFound { name } => format!("Profile '{}' not found in manifest.", name),
            Msg::ProfileInstallSkill { name } => format!("Installed skill '{}'", name),
            Msg::ProfileSkillSourceNotFound { name, path } => format!("Skill '{}' source not found at {}", name, path),
            Msg::ProfileSkillNotFound { name } => format!("Skill '{}' not found in manifest (skipped)", name),
            Msg::ProfileInstallMcp { name } => format!("Installed MCP server '{}'", name),
            Msg::ProfileMcpNotFound { name } => format!("MCP server '{}' not found in manifest (skipped)", name),
            Msg::ProfileApplySuccess { name, skills, plugins, mcp } => format!("Applied profile '{}': {} skill(s), {} plugin(s), {} MCP server(s)", name, skills, plugins, mcp),
            Msg::ProfileConfigWritten { path } => format!("Config written to {}", path),
            Msg::ProfileExportNoConfig => "No skillsync.yaml found in current project.\nUse 'skillsync use' to configure the project first.".into(),
            Msg::ProfileExportSuccess { name } => format!("Exported project config as profile '{}'", name),
            Msg::ProfileExportSummary { skills, plugins, mcp } => format!("{} skill(s), {} plugin(s), {} MCP server(s)", skills, plugins, mcp),
            Msg::ProfileSavedTo { path } => format!("Saved to {}", path),

            Msg::DoctorTitle => "SkillSync Doctor".into(),
            Msg::DoctorRegistryExists => "Registry found at ~/.skillsync/registry/".into(),
            Msg::DoctorRegistryNotFound => "Registry not found at ~/.skillsync/registry/".into(),
            Msg::DoctorRunInitHint { cmd } => format!("Run '{}' to initialize the registry.", cmd),
            Msg::DoctorManifestParseFailed { error } => format!("manifest.yaml failed to parse: {}", error),
            Msg::DoctorManifestValid => "manifest.yaml is valid".into(),
            Msg::DoctorValidationIssues { count } => format!("manifest.yaml has {} validation issue(s):", count),
            Msg::DoctorValidationError { error } => format!("    - {}", error),
            Msg::DoctorOriginConfigured { url } => format!("Git remote 'origin' configured: {}", url),
            Msg::DoctorNoOrigin => "Git remote 'origin' not configured (sync will not work)".into(),
            Msg::DoctorNotGitRepo => "Registry is not a git repository".into(),
            Msg::DoctorClaudeHomeExists => "Claude Code home (~/.claude/) exists".into(),
            Msg::DoctorClaudeHomeNotFound => "Claude Code home (~/.claude/) not found".into(),
            Msg::DoctorNoOrphaned => "No orphaned resources found".into(),
            Msg::DoctorOrphanedSkills { count } => format!("{} orphaned skill(s) in resources/skills/ (not in manifest):", count),
            Msg::DoctorOrphanedSkill { name } => format!("    - {}", name),
            Msg::DoctorOrphanedReadError => "Could not read resources/skills/ directory".into(),
            Msg::DoctorManifestSkillsOk => "All manifest skills have matching resource files".into(),
            Msg::DoctorMissingSkills { count } => format!("{} skill(s) referenced in manifest but missing on disk:", count),
            Msg::DoctorMissingSkill { name } => format!("    - {}", name),
            Msg::DoctorAllPassed => "All checks passed!".into(),
            Msg::DoctorIssues { count } => format!("{} issue(s) found", count),
            Msg::DoctorWarnings { count } => format!("{} warning(s)", count),

            Msg::WatchNoDirs => "No directories to watch. Run 'skillsync init' to initialize the registry first.".into(),
            Msg::WatchStarting => "Starting SkillSync file watcher (foreground)...".into(),
            Msg::WatchDaemonStarted { pid } => format!("Watcher daemon started (PID: {})", pid),
            Msg::WatchLogs { path } => format!("Logs: {}", path),
            Msg::WatchErrors { path } => format!("Errors: {}", path),
            Msg::WatchStopWith { cmd1, cmd2 } => format!("Stop with: {} or {}", cmd1, cmd2),
            Msg::WatchDaemonFailed { error } => format!("Failed to start daemon: {}", error),
            Msg::WatchTryManual { cmd } => format!("Try running manually: {}", cmd),
            Msg::WatchServiceNotSupported => "System service installation is not supported on this platform. Use 'skillsync watch --daemon' instead.".into(),
            Msg::WatchWrotePlist { path } => format!("Wrote plist: {}", path),
            Msg::WatchServiceLoaded => "Service loaded. The watcher will start automatically on login.".into(),
            Msg::WatchLaunchctlWarning => "launchctl load returned non-zero exit code. The plist has been written but the service may not be running.".into(),
            Msg::WatchLaunchctlHint { path } => format!("Try manually: launchctl load -w {}", path),
            Msg::WatchWroteService { path } => format!("Wrote service file: {}", path),
            Msg::WatchSystemctlReloadFailed => "systemctl daemon-reload failed.".into(),
            Msg::WatchServiceEnabled => "Service enabled and started. The watcher will start automatically on login.".into(),
            Msg::WatchSystemctlEnableFailed => "systemctl enable returned non-zero exit code.".into(),
            Msg::WatchSystemctlHint => "Try manually: systemctl --user enable --now skillsync-watcher.service".into(),
            Msg::WatchNoPlist { path } => format!("No plist found at {}. Service may not be installed.", path),
            Msg::WatchLaunchctlUnloadWarning => "launchctl unload returned non-zero exit code. Continuing with file removal.".into(),
            Msg::WatchServiceUnloaded { path } => format!("Service unloaded and plist removed: {}", path),
            Msg::WatchNoServiceFile { path } => format!("No service file found at {}. Service may not be installed.", path),
            Msg::WatchSystemctlDisableWarning => "systemctl disable returned non-zero exit code. Continuing with file removal.".into(),
            Msg::WatchServiceDisabled { path } => format!("Service disabled and removed: {}", path),

            Msg::HookInstalled { path } => format!("Installed SessionStart hook into {}", path),
            Msg::HookAlreadyInstalled => "SkillSync hook is already installed".into(),
            Msg::HookRemoved { path } => format!("Removed SkillSync hook from {}", path),
            Msg::HookNotFound => "No SkillSync hook found to remove".into(),

            Msg::SelectorConfigurePrompt => "How would you like to configure this project?".into(),
            Msg::SelectorConfigureCancelled => "Configuration method selection was cancelled".into(),
            Msg::SelectorFromProfile => "From profile  — start with a predefined bundle".into(),
            Msg::SelectorManual => "Manual        — pick resources one by one".into(),
            Msg::SelectorCopyProject => "Copy project  — reuse another project's config".into(),
            Msg::SelectorEmpty => "Registry is empty — nothing to select.".into(),
            Msg::SelectorResourcesPrompt => "Select resources to install:".into(),
            Msg::SelectorResourcesCancelled => "Resource selection was cancelled".into(),
            Msg::SelectorPreviewHeader => "=== Installation Preview ===".into(),
            Msg::SelectorSkillsLabel => "Skills:".into(),
            Msg::SelectorPluginsLabel => "Plugins:".into(),
            Msg::SelectorMcpLabel => "MCP servers:".into(),
            Msg::SelectorNoResources => "No resources selected.".into(),
            Msg::SelectorTotal { count } => format!("Total: {} resource(s)", count),
            Msg::SelectorApplyPrompt => "Apply these changes?".into(),
            Msg::SelectorConfirmCancelled => "Confirmation prompt was cancelled".into(),

            Msg::ProfilePickerPrompt => "Select a profile:".into(),
            Msg::ProfilePickerCancelled => "Profile selection was cancelled".into(),
            Msg::ProfilePickerEmpty => "No profiles found in the registry. Create one with 'skillsync profile create <name>'.".into(),
            Msg::ProfilePickerLoadError { error } => format!("could not load profile YAML: {}", error),

            Msg::DiffConflictHeader { file } => format!("--- Conflict: {}", file),
            Msg::DiffLocalRemote => "  local (ours)    remote (theirs)".into(),
            Msg::DiffResolvePrompt => "How would you like to resolve this conflict?".into(),
            Msg::DiffResolveCancelled => "Conflict resolution selection was cancelled".into(),
            Msg::DiffKeepLocal => "Keep local   — discard remote changes".into(),
            Msg::DiffUseRemote => "Use remote   — discard local changes".into(),
            Msg::DiffOpenEditor => "Open editor  — manually resolve in $EDITOR".into(),

            Msg::WatcherStarted => "File watcher started. Press Ctrl+C to stop.".into(),
            Msg::WatcherWatching { path } => format!("  Watching: {}", path),
            Msg::WatcherDirNotExist { path } => format!("  Directory does not exist, skipping: {}", path),
            Msg::WatcherDetectedChanges { count } => format!("\nDetected {} file change(s)", count),
            Msg::WatcherEventPath { path } => format!("  {}", path),
            Msg::WatcherPanicked { error } => format!("  on_change callback panicked: {:?}", error),
            Msg::WatcherRetry => "  Watcher continues running. Will retry on next change.".into(),
            Msg::WatcherError { error } => format!("  Watch error: {:?}", error),
            Msg::WatcherChannelClosed { error } => format!("  Watch channel closed: {}", error),
            Msg::WatcherAutoPushFailed { error } => format!("  Auto-push failed: {:#}", error),
            Msg::WatcherWillRetry => "  Will retry on next detected change.".into(),
            Msg::WatcherRegistryNotInit => "  Registry not initialized, skipping auto-push.".into(),
            Msg::WatcherNoChanges => "  No changes to push.".into(),
            Msg::WatcherStaging { count } => format!("  Staging {} file(s)...", count),
            Msg::WatcherCommitted { message, oid } => format!("  Committed: {} ({})", message, oid),
            Msg::WatcherPushed => "  Pushed to origin.".into(),
            Msg::WatcherPushFailed { error } => format!("  Push to origin failed: {:#}", error),
            Msg::WatcherPushLocal => "  Changes are committed locally. Push will be retried on next change.".into(),

            Msg::InstallerInstalled { name } => format!("installed {}", name),
            Msg::InstallerUpdated { name } => format!("updated {}", name),
            Msg::InstallerSkipped { name } => format!("skipped {} (up-to-date)", name),
            Msg::InstallerSkillPathNotExist { path } => format!("Registry skill path does not exist: {}\nThe skill may have been removed from the registry. Run 'skillsync doctor' to check.", path),
            Msg::InstallerUpdateFailed { name, path } => format!("Failed to update skill '{}' at {}", name, path),
            Msg::InstallerInstallFailed { name, path } => format!("Failed to install skill '{}' to {}", name, path),
            Msg::InstallerNotInManifest { name } => format!("Skill '{}' not found in registry manifest", name),

            Msg::ResourceTypeSkill => "skill".into(),
            Msg::ResourceTypePlugin => "plugin".into(),
            Msg::ResourceTypeMcp => "mcp_server".into(),
            Msg::ResourceSourceNotExist { path } => format!("Source path does not exist: {}. Check the path and ensure the resource has not been deleted.", path),
            Msg::ResourceCopyDirFailed { from, to } => format!("Failed to copy directory from {} to {}", from, to),
            Msg::ResourceCopyFileFailed { from, to } => format!("Failed to copy file from {} to {}", from, to),
        }
    }

    /// Returns the Chinese translation for this message.
    pub fn zh(&self) -> String {
        match self {
            Msg::ContextFailedToLoadManifest => "无法加载 manifest。是否已运行 'skillsync init'？".into(),
            Msg::ContextFailedToSaveManifest => "无法保存 manifest".into(),
            Msg::ContextFailedToOpenRepo => "无法打开 registry 仓库".into(),
            Msg::ContextHomeDir => "无法确定主目录".into(),
            Msg::ContextCurrentDir => "无法确定当前目录".into(),
            Msg::ContextReadDir { path } => format!("无法读取目录：{}", path),
            Msg::ContextCreateDir { path } => format!("无法创建目录：{}", path),
            Msg::ContextResolvePaths => "无法解析 Claude 路径".into(),

            Msg::InitRegistryExists { path } => format!("Registry 已存在：{}\n使用 `skillsync sync` 更新，或删除该目录后重新初始化。", path),
            Msg::InitSuccess { path } => format!("已初始化 SkillSync registry：{}", path),
            Msg::InitLanguageSelect => "请选择您偏好的语言：".into(),
            Msg::InitLanguageSet { lang } => format!("语言已设置为 {}。可通过 SKILLSYNC_LANG 环境变量修改。", lang),
            Msg::InitCloned { url } => format!("已克隆 SkillSync registry：{}", url),
            Msg::InitScanResult { skills, plugins, mcp, profiles } => format!("{} 个 skill、{} 个 plugin、{} 个 MCP server、{} 个 profile", skills, plugins, mcp, profiles),
            Msg::InitScanSkillsError { error } => format!("扫描 skills 失败：{}", error),
            Msg::InitScanMcpError { error } => format!("扫描 MCP servers 失败：{}", error),
            Msg::InitNoResourcesFound => "未找到任何已安装的 Claude Code 资源。".into(),
            Msg::InitFoundResources { count } => format!("找到 {} 个已安装的资源，可能需要导入：", count),
            Msg::InitResourceItem { kind, name, detail } => format!("[{}] {} — {}", kind, name, detail),
            Msg::InitAddHint { cmd } => format!("使用 '{}' 将这些资源添加到 registry。", cmd),

            Msg::AddInvalidScope { scope } => format!("无效的 scope '{}'。必须为 'global' 或 'shared'。", scope),
            Msg::AddPathNotExist { path } => format!("路径 '{}' 不存在。请检查路径后重试。", path),
            Msg::AddSkillAlreadyExists { name } => format!("Skill '{}' 已存在于 registry 中。请先移除或使用其他名称。", name),
            Msg::AddSkillSuccess { name } => format!("已添加 skill '{}' 到 registry", name),
            Msg::AddInvalidPluginFormat { input } => format!("无效的 plugin 格式 '{}'。期望格式为 'name@marketplace'（例如 superpowers@claude-plugins-official）。", input),
            Msg::AddPluginAlreadyExists { name } => format!("Plugin '{}' 已存在于 registry 中。请先移除或使用其他名称。", name),
            Msg::AddPluginSuccess { name } => format!("已添加 plugin '{}' 到 registry", name),
            Msg::AddMcpRequiresCommand => "MCP server 需要 --command 参数。用法：skillsync add --mcp <name> --command <cmd> [--args <args>...]".into(),
            Msg::AddMcpAlreadyExists { name } => format!("MCP server '{}' 已存在于 registry 中。请先移除或使用其他名称。", name),
            Msg::AddMcpSuccess { name } => format!("已添加 MCP server '{}' 到 registry", name),
            Msg::AddNothingToAdd => "没有可添加的内容。请提供路径、--plugin 或 --mcp。\n用法：\n  skillsync add <path>                              # 添加 skill\n  skillsync add --plugin name@marketplace           # 添加 plugin\n  skillsync add --mcp <name> --command <cmd>        # 添加 MCP server".into(),

            Msg::RemoveResourceNotFound { name } => format!("资源 '{}' 在 registry 中未找到。\n使用 'skillsync list' 查看所有已注册的资源。", name),
            Msg::RemoveReferencedByProfiles { name, profiles } => format!("资源 '{}' 被以下 profile 引用：{}", name, profiles),
            Msg::RemoveUpdateProfilesHint => "移除后可能需要更新这些 profile。".into(),
            Msg::RemoveSuccess { kind, name } => format!("已从 registry 中移除 {} '{}'", kind, name),

            Msg::ListInvalidTypeFilter { filter } => format!("无效的类型过滤器 '{}'。必须为：skill、plugin、mcp", filter),
            Msg::ListNoResourcesOfType { kind } => format!("registry 中没有 {} 资源。", kind),
            Msg::ListNoResources => "registry 中没有资源。".into(),
            Msg::ListUseAddHint { cmd } => format!("使用 '{}' 添加资源。", cmd),
            Msg::ListColName => "名称".into(),
            Msg::ListColType => "类型".into(),
            Msg::ListColScope => "作用域".into(),
            Msg::ListColVersion => "版本".into(),
            Msg::ListTotal { count } => format!("共 {} 个资源", count),

            Msg::InfoSkillLabel { name } => format!("Skill：{}", name),
            Msg::InfoType => "类型：".into(),
            Msg::InfoScope => "作用域：".into(),
            Msg::InfoVersion => "版本：".into(),
            Msg::InfoPath => "路径：".into(),
            Msg::InfoDescription => "描述：".into(),
            Msg::InfoTags => "标签：".into(),
            Msg::InfoSource => "来源：".into(),
            Msg::InfoMarketplace => "市场：".into(),
            Msg::InfoPlugin => "Plugin：".into(),
            Msg::InfoSkill => "Skill：".into(),
            Msg::InfoHash => "哈希：".into(),
            Msg::InfoProfiles => "Profile：".into(),
            Msg::InfoPluginLabel { name } => format!("Plugin：{}", name),
            Msg::InfoPluginMarketplace => "市场：".into(),
            Msg::InfoGitSha => "Git SHA：".into(),
            Msg::InfoRepo => "仓库：".into(),
            Msg::InfoMcpLabel { name } => format!("MCP Server：{}", name),
            Msg::InfoCommand => "命令：".into(),
            Msg::InfoArgs => "参数：".into(),
            Msg::InfoNotFound { name } => format!("资源 '{}' 在 registry 中未找到。", name),
            Msg::InfoDidYouMean => "您是否想找：".into(),
            Msg::InfoSuggestion { name } => format!("  - {}", name),
            Msg::InfoUseListHint { cmd } => format!("使用 '{}' 查看所有已注册的资源。", cmd),

            Msg::SearchNoResults { query } => format!("'{}' 没有搜索结果。", query),
            Msg::SearchResults { count, query } => format!("'{}' 找到 {} 个结果：", query, count),
            Msg::SearchResultRow { kind, name, desc } => format!("  {} {} {}", kind, name, desc),
            Msg::SearchMatchedOn { field } => format!("    匹配字段：{}", field),

            Msg::PullRegistryNotFound { path } => format!("未找到 registry：{}。\n请先运行 'skillsync init' 或 'skillsync init --from <url>'。", path),
            Msg::PullNoOrigin => "Registry 仓库中没有名为 'origin' 的远程仓库。\n如果是本地 registry，则没有可拉取的内容。".into(),
            Msg::PullFetching => "正在从 origin 拉取...".into(),
            Msg::PullTimedOut { secs } => format!("拉取超时（{} 秒）。请检查网络连接或使用 --timeout 增加超时时间。", secs),
            Msg::PullMergeConflicts { count } => format!("检测到 {} 个文件存在合并冲突：", count),
            Msg::PullConflictFile { file } => format!("  - {}", file),
            Msg::PullResolveHint => "\n请手动解决冲突，然后运行 'skillsync resolve'。".into(),
            Msg::PullConflicts => "拉取完成，但存在冲突".into(),
            Msg::PullUpToDate => "已是最新版本。".into(),
            Msg::PullFastForwarded => "已快进到最新版本。".into(),
            Msg::PullMerged => "已成功合并远程更改。".into(),

            Msg::PushRegistryNotFound { path } => format!("未找到 registry：{}。\n请先运行 'skillsync init' 或 'skillsync init --from <url>'。", path),
            Msg::PushNoOrigin => "Registry 仓库中没有名为 'origin' 的远程仓库。\n如果是本地 registry，则没有可推送的内容。".into(),
            Msg::PushNothingToPush => "没有需要推送的内容 — registry 是干净的。".into(),
            Msg::PushCommitting { count } => format!("正在提交 {} 个更改的文件...", count),
            Msg::PushPushing => "正在推送到 origin...".into(),
            Msg::PushSuccess { count } => format!("已推送 {} 个更改到远程 registry。", count),
            Msg::PushCommitFile { file } => format!("  - {}", file),

            Msg::SyncRegistryNotFound { path } => format!("未找到 registry：{}。\n请先运行 'skillsync init' 或 'skillsync init --from <url>'。", path),
            Msg::SyncNoOrigin => "Registry 仓库中没有名为 'origin' 的远程仓库。\n如果是本地 registry，则没有可同步的内容。".into(),
            Msg::SyncSyncing => "正在同步 registry 与远程...\n".into(),
            Msg::SyncFetching => "正在从 origin 获取...".into(),
            Msg::SyncMergeConflicts { count } => format!("检测到 {} 个文件存在合并冲突：", count),
            Msg::SyncConflictFile { file } => format!("  - {}", file),
            Msg::SyncResolveHint => "\n请手动解决冲突，然后运行 'skillsync resolve'。".into(),
            Msg::SyncAborted => "因合并冲突中止同步。请运行 'skillsync resolve' 解决冲突后重试。".into(),
            Msg::SyncUpToDate => "已与远程版本同步。".into(),
            Msg::SyncFastForwarded => "已快进到最新远程版本。".into(),
            Msg::SyncMerged => "已成功合并远程更改。".into(),
            Msg::SyncNoLocalChanges => "没有本地更改需要推送。".into(),
            Msg::SyncComplete => "\n同步完成。".into(),
            Msg::SyncPushing { count } => format!("\n正在推送 {} 个本地更改...", count),
            Msg::SyncPushed { count } => format!("已推送 {} 个更改到远程。", count),
            Msg::SyncCommitFile { file } => format!("  - {}", file),

            Msg::ResolveNotInitialized => "Registry 未初始化。请先运行 'skillsync init'。".into(),
            Msg::ResolveNoConflicts => "没有需要解决的冲突。".into(),
            Msg::ResolveFound { count } => format!("发现 {} 个冲突文件：", count),
            Msg::ResolveConflictEntry { action, file } => format!("  {} {}", action, file),
            Msg::ResolveKeptLocal { file } => format!("保留了本地版本：{}", file),
            Msg::ResolveUsedRemote { file } => format!("应用了远程版本：{}", file),
            Msg::ResolveManuallyEdited { file } => format!("手动编辑了：{}", file),
            Msg::ResolveSuccess { count } => format!("已解决 {} 个冲突并提交。", count),
            Msg::ResolveEditorLaunch { editor } => format!("无法启动编辑器 '{}'", editor),
            Msg::ResolveEditorFailed { editor } => format!("编辑器 '{}' 退出码非零。请设置 EDITOR 环境变量指定您偏好的编辑器。", editor),

            Msg::UseConfiguring { path } => format!("正在配置项目：{}", path),
            Msg::UseCancelled => "已取消 — 未应用任何更改。".into(),
            Msg::UseSuccess => "项目配置成功！".into(),
            Msg::UseProfilePreSelects { profile, count } => format!("Profile {} 预选了 {} 个资源。可根据需要调整：", profile, count),
            Msg::UseNoSiblingProjects => "未找到包含 skillsync.yaml 的同级项目。".into(),
            Msg::UseFallbackManual => "回退到手动选择。".into(),
            Msg::UseSelectProject => "选择要复制配置的项目：".into(),
            Msg::UseLoadedResources { count, project } => format!("从项目 {} 加载了 {} 个资源。可根据需要调整：", project, count),
            Msg::UseMcpNotFound { name } => format!("MCP server '{}' 在 manifest 中未找到，跳过", name),
            Msg::UseMergedMcp { count, path } => format!("已合并 {} 个 MCP server 到 {}", count, path),
            Msg::UseWrote { path } => format!("已写入 {}", path),

            Msg::InstallGlobal => "正在安装全局资源...".into(),
            Msg::InstallMcpMerged { count, path } => format!("已合并 {} 个 MCP server 到 {}", count, path),
            Msg::InstallGlobalComplete => "全局安装完成。".into(),
            Msg::InstallNoConfig { path } => format!("在 {} 未找到 skillsync.yaml\n请运行 'skillsync use' 配置项目，或手动创建该文件。", path),
            Msg::InstallProject { path } => format!("正在为项目 {} 安装资源", path),
            Msg::InstallMcpNotFound { name } => format!("MCP server '{}' 在 registry manifest 中未找到 — 跳过", name),
            Msg::InstallLockWritten { path } => format!("Lock file 已写入：{}", path),
            Msg::InstallProjectComplete => "项目安装完成。".into(),

            Msg::UpdateSourceNotExist { path } => format!("源路径 '{}' 不存在。请提供有效的 skill 路径。", path),
            Msg::UpdateSkillSuccess { name, old_ver, new_ver } => format!("已更新 skill '{}'：{} → {}", name, old_ver, new_ver),
            Msg::UpdatePluginSuccess { name, old_ver, new_ver } => format!("已更新 plugin '{}'：{} → {}", name, old_ver, new_ver),
            Msg::UpdateMcpAlreadyRegistered { name } => format!("MCP server '{}' 已注册。请通过 'skillsync remove' + 'skillsync add' 更新其 command/args。", name),
            Msg::UpdateNotFound { name } => format!("资源 '{}' 在 registry 中未找到。\n使用 'skillsync list' 查看所有已注册的资源。", name),

            Msg::ProfileListEmpty => "registry 中没有 profile。".into(),
            Msg::ProfileListHint { cmd } => format!("使用 '{}' 创建一个。", cmd),
            Msg::ProfileErrorLoading { path } => format!("（加载 {} 失败）", path),
            Msg::ProfileColName => "Profile".into(),
            Msg::ProfileColDesc => "描述".into(),
            Msg::ProfileColSkills => "Skills".into(),
            Msg::ProfileColPlugins => "Plugins".into(),
            Msg::ProfileColMcp => "MCP".into(),
            Msg::ProfileTotal { count } => format!("共 {} 个 profile", count),
            Msg::ProfileAlreadyExists { name } => format!("Profile '{}' 已存在。请先使用 'skillsync remove' 移除或选择其他名称。", name),
            Msg::ProfileCreateSuccess { name, path } => format!("已创建 profile '{}'：{}", name, path),
            Msg::ProfileEditHint { path } => format!("编辑 {} 添加 skills、plugins 和 MCP servers。", path),
            Msg::ProfileExportHint { cmd } => format!("或使用 '{}' 从项目配置填充。", cmd),
            Msg::ProfileNotFound { name } => format!("Profile '{}' 在 manifest 中未找到。", name),
            Msg::ProfileInstallSkill { name } => format!("已安装 skill '{}'", name),
            Msg::ProfileSkillSourceNotFound { name, path } => format!("Skill '{}' 源文件不存在：{}", name, path),
            Msg::ProfileSkillNotFound { name } => format!("Skill '{}' 在 manifest 中未找到（跳过）", name),
            Msg::ProfileInstallMcp { name } => format!("已安装 MCP server '{}'", name),
            Msg::ProfileMcpNotFound { name } => format!("MCP server '{}' 在 manifest 中未找到（跳过）", name),
            Msg::ProfileApplySuccess { name, skills, plugins, mcp } => format!("已应用 profile '{}'：{} 个 skill、{} 个 plugin、{} 个 MCP server", name, skills, plugins, mcp),
            Msg::ProfileConfigWritten { path } => format!("配置已写入：{}", path),
            Msg::ProfileExportNoConfig => "当前项目中未找到 skillsync.yaml。\n请先使用 'skillsync use' 配置项目。".into(),
            Msg::ProfileExportSuccess { name } => format!("已将项目配置导出为 profile '{}'", name),
            Msg::ProfileExportSummary { skills, plugins, mcp } => format!("{} 个 skill、{} 个 plugin、{} 个 MCP server", skills, plugins, mcp),
            Msg::ProfileSavedTo { path } => format!("已保存到：{}", path),

            Msg::DoctorTitle => "SkillSync 诊断".into(),
            Msg::DoctorRegistryExists => "已找到 registry：~/.skillsync/registry/".into(),
            Msg::DoctorRegistryNotFound => "未找到 registry：~/.skillsync/registry/".into(),
            Msg::DoctorRunInitHint { cmd } => format!("运行 '{}' 初始化 registry。", cmd),
            Msg::DoctorManifestParseFailed { error } => format!("manifest.yaml 解析失败：{}", error),
            Msg::DoctorManifestValid => "manifest.yaml 格式正确".into(),
            Msg::DoctorValidationIssues { count } => format!("manifest.yaml 有 {} 个验证问题：", count),
            Msg::DoctorValidationError { error } => format!("    - {}", error),
            Msg::DoctorOriginConfigured { url } => format!("Git remote 'origin' 已配置：{}", url),
            Msg::DoctorNoOrigin => "Git remote 'origin' 未配置（同步功能不可用）".into(),
            Msg::DoctorNotGitRepo => "Registry 不是 git 仓库".into(),
            Msg::DoctorClaudeHomeExists => "Claude Code 主目录 (~/.claude/) 存在".into(),
            Msg::DoctorClaudeHomeNotFound => "Claude Code 主目录 (~/.claude/) 未找到".into(),
            Msg::DoctorNoOrphaned => "未发现孤立资源".into(),
            Msg::DoctorOrphanedSkills { count } => format!("{} 个孤立 skill（在 resources/skills/ 中但不在 manifest 中）：", count),
            Msg::DoctorOrphanedSkill { name } => format!("    - {}", name),
            Msg::DoctorOrphanedReadError => "无法读取 resources/skills/ 目录".into(),
            Msg::DoctorManifestSkillsOk => "所有 manifest 中的 skills 都有对应的资源文件".into(),
            Msg::DoctorMissingSkills { count } => format!("{} 个 skill 在 manifest 中引用但磁盘上不存在：", count),
            Msg::DoctorMissingSkill { name } => format!("    - {}", name),
            Msg::DoctorAllPassed => "所有检查均已通过！".into(),
            Msg::DoctorIssues { count } => format!("发现 {} 个问题", count),
            Msg::DoctorWarnings { count } => format!("{} 个警告", count),

            Msg::WatchNoDirs => "没有要监控的目录。请先运行 'skillsync init' 初始化 registry。".into(),
            Msg::WatchStarting => "正在启动 SkillSync 文件监控（前台）...".into(),
            Msg::WatchDaemonStarted { pid } => format!("监控守护进程已启动（PID：{}）", pid),
            Msg::WatchLogs { path } => format!("日志：{}", path),
            Msg::WatchErrors { path } => format!("错误日志：{}", path),
            Msg::WatchStopWith { cmd1, cmd2 } => format!("停止命令：{} 或 {}", cmd1, cmd2),
            Msg::WatchDaemonFailed { error } => format!("守护进程启动失败：{}", error),
            Msg::WatchTryManual { cmd } => format!("尝试手动运行：{}", cmd),
            Msg::WatchServiceNotSupported => "此平台不支持系统服务安装。请使用 'skillsync watch --daemon'。".into(),
            Msg::WatchWrotePlist { path } => format!("已写入 plist：{}", path),
            Msg::WatchServiceLoaded => "服务已加载。监控将在登录时自动启动。".into(),
            Msg::WatchLaunchctlWarning => "launchctl load 返回非零退出码。plist 已写入但服务可能未运行。".into(),
            Msg::WatchLaunchctlHint { path } => format!("请手动执行：launchctl load -w {}", path),
            Msg::WatchWroteService { path } => format!("已写入服务文件：{}", path),
            Msg::WatchSystemctlReloadFailed => "systemctl daemon-reload 失败。".into(),
            Msg::WatchServiceEnabled => "服务已启用并启动。监控将在登录时自动运行。".into(),
            Msg::WatchSystemctlEnableFailed => "systemctl enable 返回非零退出码。".into(),
            Msg::WatchSystemctlHint => "请手动执行：systemctl --user enable --now skillsync-watcher.service".into(),
            Msg::WatchNoPlist { path } => format!("未在 {} 找到 plist。服务可能未安装。", path),
            Msg::WatchLaunchctlUnloadWarning => "launchctl unload 返回非零退出码。继续删除文件。".into(),
            Msg::WatchServiceUnloaded { path } => format!("服务已卸载，plist 已删除：{}", path),
            Msg::WatchNoServiceFile { path } => format!("未在 {} 找到服务文件。服务可能未安装。", path),
            Msg::WatchSystemctlDisableWarning => "systemctl disable 返回非零退出码。继续删除文件。".into(),
            Msg::WatchServiceDisabled { path } => format!("服务已禁用并删除：{}", path),

            Msg::HookInstalled { path } => format!("已将 SessionStart hook 安装到 {}", path),
            Msg::HookAlreadyInstalled => "SkillSync hook 已安装".into(),
            Msg::HookRemoved { path } => format!("已从 {} 移除 SkillSync hook", path),
            Msg::HookNotFound => "未找到 SkillSync hook，无法移除".into(),

            Msg::SelectorConfigurePrompt => "您想如何配置此项目？".into(),
            Msg::SelectorConfigureCancelled => "配置方式选择已取消".into(),
            Msg::SelectorFromProfile => "从 profile 开始 — 使用预定义的资源包".into(),
            Msg::SelectorManual => "手动选择    — 逐个选择资源".into(),
            Msg::SelectorCopyProject => "复制项目    — 复用其他项目的配置".into(),
            Msg::SelectorEmpty => "Registry 为空 — 没有可选择的内容。".into(),
            Msg::SelectorResourcesPrompt => "选择要安装的资源：".into(),
            Msg::SelectorResourcesCancelled => "资源选择已取消".into(),
            Msg::SelectorPreviewHeader => "=== 安装预览 ===".into(),
            Msg::SelectorSkillsLabel => "Skills：".into(),
            Msg::SelectorPluginsLabel => "Plugins：".into(),
            Msg::SelectorMcpLabel => "MCP servers：".into(),
            Msg::SelectorNoResources => "未选择任何资源。".into(),
            Msg::SelectorTotal { count } => format!("共 {} 个资源", count),
            Msg::SelectorApplyPrompt => "应用这些更改？".into(),
            Msg::SelectorConfirmCancelled => "确认提示已取消".into(),

            Msg::ProfilePickerPrompt => "选择 profile：".into(),
            Msg::ProfilePickerCancelled => "Profile 选择已取消".into(),
            Msg::ProfilePickerEmpty => "registry 中没有 profile。使用 'skillsync profile create <name>' 创建一个。".into(),
            Msg::ProfilePickerLoadError { error } => format!("无法加载 profile YAML：{}", error),

            Msg::DiffConflictHeader { file } => format!("--- 冲突：{}", file),
            Msg::DiffLocalRemote => "  本地（ ours ）    远程（ theirs ）".into(),
            Msg::DiffResolvePrompt => "您想如何解决此冲突？".into(),
            Msg::DiffResolveCancelled => "冲突解决选择已取消".into(),
            Msg::DiffKeepLocal => "保留本地   — 丢弃远程更改".into(),
            Msg::DiffUseRemote => "使用远程   — 丢弃本地更改".into(),
            Msg::DiffOpenEditor => "打开编辑器 — 在 $EDITOR 中手动解决".into(),

            Msg::WatcherStarted => "文件监控已启动。按 Ctrl+C 停止。".into(),
            Msg::WatcherWatching { path } => format!("  监控中：{}", path),
            Msg::WatcherDirNotExist { path } => format!("  目录不存在，跳过：{}", path),
            Msg::WatcherDetectedChanges { count } => format!("\n检测到 {} 个文件更改", count),
            Msg::WatcherEventPath { path } => format!("  {}", path),
            Msg::WatcherPanicked { error } => format!("  on_change 回调 panic：{:?}", error),
            Msg::WatcherRetry => "  监控继续运行。下次更改时将重试。".into(),
            Msg::WatcherError { error } => format!("  监控错误：{:?}", error),
            Msg::WatcherChannelClosed { error } => format!("  监控通道关闭：{}", error),
            Msg::WatcherAutoPushFailed { error } => format!("  自动推送失败：{:#}", error),
            Msg::WatcherWillRetry => "  下次检测到更改时将重试。".into(),
            Msg::WatcherRegistryNotInit => "  Registry 未初始化，跳过自动推送。".into(),
            Msg::WatcherNoChanges => "  没有需要推送的更改。".into(),
            Msg::WatcherStaging { count } => format!("  正在暂存 {} 个文件...", count),
            Msg::WatcherCommitted { message, oid } => format!("  已提交：{} ({})", message, oid),
            Msg::WatcherPushed => "  已推送到 origin。".into(),
            Msg::WatcherPushFailed { error } => format!("  推送到 origin 失败：{:#}", error),
            Msg::WatcherPushLocal => "  更改已在本地提交。将在下次更改时重试推送。".into(),

            Msg::InstallerInstalled { name } => format!("已安装 {}", name),
            Msg::InstallerUpdated { name } => format!("已更新 {}", name),
            Msg::InstallerSkipped { name } => format!("跳过 {}（已是最新）", name),
            Msg::InstallerSkillPathNotExist { path } => format!("Registry skill 路径不存在：{}\n该 skill 可能已从 registry 中移除。请运行 'skillsync doctor' 检查。", path),
            Msg::InstallerUpdateFailed { name, path } => format!("更新 skill '{}' 失败：{}", name, path),
            Msg::InstallerInstallFailed { name, path } => format!("安装 skill '{}' 失败：{}", name, path),
            Msg::InstallerNotInManifest { name } => format!("Skill '{}' 在 registry manifest 中未找到", name),

            Msg::ResourceTypeSkill => "skill".into(),
            Msg::ResourceTypePlugin => "plugin".into(),
            Msg::ResourceTypeMcp => "mcp_server".into(),
            Msg::ResourceSourceNotExist { path } => format!("源路径不存在：{}。请检查路径并确保资源未被删除。", path),
            Msg::ResourceCopyDirFailed { from, to } => format!("复制目录失败：{} → {}", from, to),
            Msg::ResourceCopyFileFailed { from, to } => format!("复制文件失败：{} → {}", from, to),
        }
    }

    /// Returns the translation for the current language.
    pub fn get(&self) -> String {
        match lang() {
            Lang::En => self.en(),
            Lang::Zh => self.zh(),
        }
    }
}

// ---------------------------------------------------------------------------
// t!() macro
// ---------------------------------------------------------------------------

/// Returns the translation of the given message key for the current language.
///
/// Usage:
/// ```ignore
/// println!("{}", t!(Msg::InitSuccess { path: "/path" }));
/// ```
///
/// For parameterized messages, the format string contains `{}` placeholders:
/// ```ignore
/// println!("{}", format!(t!(Msg::InitSuccess { path: "/path" })));
/// ```
#[macro_export]
macro_rules! t {
    ($key:expr) => { $key.get() };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lang_zh_override() {
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LANG");
        std::env::set_var("SKILLSYNC_LANG", "zh");

        let detected = Lang::detect();
        assert_eq!(detected, Lang::Zh);

        std::env::set_var("SKILLSYNC_LANG", "en");
        let detected = Lang::detect();
        assert_eq!(detected, Lang::En);

        std::env::remove_var("SKILLSYNC_LANG");
    }

    #[test]
    fn test_lang_system_fallback() {
        std::env::remove_var("SKILLSYNC_LANG");
        std::env::remove_var("LC_ALL");
        std::env::set_var("LANG", "zh_CN.UTF-8");

        let detected = Lang::detect();
        assert_eq!(detected, Lang::Zh);

        std::env::set_var("LANG", "en_US.UTF-8");
        let detected = Lang::detect();
        assert_eq!(detected, Lang::En);

        std::env::remove_var("LANG");
    }

    #[test]
    fn test_msg_translations_differ() {
        let key = Msg::InitSuccess {
            path: "/test".into(),
        };
        assert_ne!(key.en(), key.zh());
    }

    #[test]
    fn test_msg_get_uses_lang_directly() {
        // Test that .en() and .zh() work correctly without cache interference
        let key = Msg::InitSuccess {
            path: "/test".into(),
        };
        assert_eq!(key.en(), "Initialized new SkillSync registry at /test");
        assert_eq!(key.zh(), "已初始化 SkillSync registry：/test");
    }
}
