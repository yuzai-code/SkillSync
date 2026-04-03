pub mod init;
pub mod add;
pub mod remove;
pub mod list;
pub mod info;
pub mod sync_cmd;
pub mod pull;
pub mod push;
pub mod use_cmd;
pub mod profile;
pub mod resolve;
pub mod doctor;
pub mod watch;
pub mod hook;
pub mod search;
pub mod update;
pub mod install;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "skillsync", about = "Sync Claude Code skills, plugins, and MCP servers across machines and projects")]
#[command(version, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Suppress output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Preview changes without applying
    #[arg(long, global = true)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize registry (new or clone from remote)
    Init {
        /// Clone from existing remote registry
        #[arg(long)]
        from: Option<String>,
    },

    /// Add a resource to the registry
    Add {
        /// Path to skill directory, or resource identifier
        path: Option<String>,

        /// Add a plugin (format: name@marketplace)
        #[arg(long)]
        plugin: Option<String>,

        /// Add an MCP server
        #[arg(long)]
        mcp: Option<String>,

        /// MCP server command
        #[arg(long)]
        command: Option<String>,

        /// MCP server arguments
        #[arg(long)]
        args: Option<Vec<String>>,

        /// Resource scope (global or shared)
        #[arg(long, default_value = "shared")]
        scope: String,
    },

    /// Remove a resource from the registry
    Remove {
        /// Resource name
        name: String,
    },

    /// Update a resource to latest version
    Update {
        /// Resource name
        name: String,
    },

    /// List all registered resources
    List {
        /// Filter by type: skill, plugin, mcp
        #[arg(long, short)]
        r#type: Option<String>,
    },

    /// Search resources in the registry
    Search {
        /// Search query
        query: String,
    },

    /// Show detailed resource info
    Info {
        /// Resource name
        name: String,
    },

    /// Interactive project configuration
    Use {},

    /// Install resources from skillsync.yaml
    Install {
        /// Install global resources only
        #[arg(long)]
        global: bool,
    },

    /// Pull remote changes
    Pull {
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// Push local changes to remote
    Push {
        /// Auto-generated commit message
        #[arg(long)]
        auto: bool,
    },

    /// Bidirectional sync (pull + push)
    Sync {},

    /// Resolve sync conflicts
    Resolve {},

    /// Profile management
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Diagnose environment issues
    Doctor {},

    /// File watcher daemon
    Watch {
        /// Run as background daemon
        #[arg(long)]
        daemon: bool,

        /// Install as system service
        #[arg(long)]
        install: bool,

        /// Uninstall system service
        #[arg(long)]
        uninstall: bool,
    },

    /// Manage Claude Code hooks
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand)]
pub enum ProfileAction {
    /// List all profiles
    List {},
    /// Create a new profile
    Create {
        /// Profile name
        name: String,
    },
    /// Apply a profile to current project
    Apply {
        /// Profile name
        name: String,
    },
    /// Export current project config as profile
    Export {
        /// Profile name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum HookAction {
    /// Install Claude Code hooks
    Install {},
    /// Remove Claude Code hooks
    Remove {},
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { from } => init::run(from, cli.quiet),
        Commands::Add { path, plugin, mcp, command, args, scope } => {
            add::run(path, plugin, mcp, command, args, scope)
        }
        Commands::Remove { name } => remove::run(&name),
        Commands::Update { name } => update::run(&name),
        Commands::List { r#type } => list::run(r#type.as_deref()),
        Commands::Search { query } => search::run(&query),
        Commands::Info { name } => info::run(&name),
        Commands::Use {} => use_cmd::run(),
        Commands::Install { global } => install::run(global),
        Commands::Pull { timeout } => pull::run(timeout, cli.quiet),
        Commands::Push { auto } => push::run(auto, cli.quiet),
        Commands::Sync {} => sync_cmd::run(cli.quiet),
        Commands::Resolve {} => resolve::run(),
        Commands::Profile { action } => profile::run(action),
        Commands::Doctor {} => doctor::run(),
        Commands::Watch { daemon, install, uninstall } => watch::run(daemon, install, uninstall),
        Commands::Hook { action } => hook::run(action),
    }
}
