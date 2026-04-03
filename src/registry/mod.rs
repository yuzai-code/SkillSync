pub mod manifest;
pub mod git_ops;
pub mod resource;

// Re-export core types for convenient access via `crate::registry::*`.
pub use manifest::{
    CommunitySource, Manifest, McpServerEntry, PluginEntry, ProfileConfig, ProfileRef,
    ResourceScope, SkillEntry, SkillSyncConfig, SkillType,
};
pub use resource::{compute_hash, copy_resource, ResourceType};
