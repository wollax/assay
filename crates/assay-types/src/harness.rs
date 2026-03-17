//! Harness profile types for agent configuration.
//!
//! A [`HarnessProfile`] defines the complete runtime configuration for an
//! agentic coding session: prompt layers, settings overrides, and lifecycle
//! hooks. These types are consumed by the prompt builder, settings merger,
//! Claude adapter, and RunManifest.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Category of a prompt layer, determining its priority tier in assembly.
///
/// Variants are ordered from highest built-in priority to custom:
/// - `System`: foundational agent instructions
/// - `Project`: project-level context and conventions
/// - `Spec`: spec-specific acceptance criteria and constraints
/// - `Custom`: user-defined layers injected at arbitrary priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PromptLayerKind {
    /// Foundational agent instructions (highest built-in priority).
    System,
    /// Project-level context and conventions.
    Project,
    /// Spec-specific acceptance criteria and constraints.
    Spec,
    /// User-defined layer injected at arbitrary priority.
    Custom,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "prompt-layer-kind",
        generate: || schemars::schema_for!(PromptLayerKind),
    }
}

/// A single prompt layer contributing to the assembled agent prompt.
///
/// Prompt layers are assembled in priority order (lowest `priority` value first)
/// to build the final system prompt for an agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PromptLayer {
    /// Category of this prompt layer.
    pub kind: PromptLayerKind,
    /// Human-readable name for identification and debugging.
    pub name: String,
    /// The prompt text content.
    pub content: String,
    /// Ordering hint for assembly (lower values assemble first).
    pub priority: i32,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "prompt-layer",
        generate: || schemars::schema_for!(PromptLayer),
    }
}

/// Settings overrides applied to an agent session.
///
/// All fields are optional or default-empty — only specified values override
/// the base configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsOverride {
    /// Model identifier to use (e.g., `"sonnet"`, `"opus"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Permissions granted to the agent (e.g., `"filesystem"`, `"network"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// Tools available to the agent (e.g., `"bash"`, `"browser"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// Maximum number of agent turns before forced stop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "settings-override",
        generate: || schemars::schema_for!(SettingsOverride),
    }
}

/// Lifecycle event that the harness can hook into.
///
/// Hooks fire at specific points during agent execution:
/// - `PreTool`: before a tool invocation
/// - `PostTool`: after a tool invocation completes
/// - `Stop`: when the agent session is stopping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HookEvent {
    /// Fires before a tool invocation.
    PreTool,
    /// Fires after a tool invocation completes.
    PostTool,
    /// Fires when the agent session is stopping.
    Stop,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "hook-event",
        generate: || schemars::schema_for!(HookEvent),
    }
}

/// A lifecycle hook that runs an external command at a specific event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HookContract {
    /// The lifecycle event that triggers this hook.
    pub event: HookEvent,
    /// Shell command to execute when the event fires.
    pub command: String,
    /// Maximum seconds to wait for the command to complete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "hook-contract",
        generate: || schemars::schema_for!(HookContract),
    }
}

/// Complete agent configuration profile for a harness session.
///
/// Combines prompt layers, settings overrides, and lifecycle hooks into a
/// single deployable configuration unit. Consumed by the harness runtime
/// to configure and launch agent sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HarnessProfile {
    /// Unique name identifying this profile.
    pub name: String,
    /// Prompt layers assembled into the agent's system prompt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_layers: Vec<PromptLayer>,
    /// Settings overrides applied to the agent session.
    pub settings: SettingsOverride,
    /// Lifecycle hooks triggered during agent execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<HookContract>,
    /// Working directory for agent execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "harness-profile",
        generate: || schemars::schema_for!(HarnessProfile),
    }
}
