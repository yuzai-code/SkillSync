pub mod manifest;
pub mod git_ops;
pub mod resource;
pub mod config;
pub mod discover;

// Re-export core types for convenient access via `crate::registry::*`.
#[allow(unused_imports)]
pub use manifest::{
    Manifest, McpServerEntry, PluginEntry, ProfileConfig, ProfileRef,
    ResourceScope, SkillEntry, SkillSyncConfig, SkillType,
};
#[allow(unused_imports)]
pub use resource::{compute_hash, copy_resource};
#[allow(unused_imports)]
pub use config::GlobalConfig;
