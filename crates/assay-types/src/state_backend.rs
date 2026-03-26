//! Backend configuration for state persistence.

use crate::schema_registry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Backend configuration for state persistence.
///
/// Serializes with `snake_case` tag keys: `LocalFs` → `"local_fs"`,
/// `Custom` → `{"custom": {...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StateBackendConfig {
    /// Local filesystem backend (default). No additional config needed.
    LocalFs,
    /// Custom third-party backend identified by name.
    Custom {
        /// Identifier for the backend implementation.
        name: String,
        /// Backend-specific configuration payload.
        config: serde_json::Value,
    },
}

inventory::submit! {
    schema_registry::SchemaEntry {
        name: "state-backend-config",
        generate: || schemars::schema_for!(StateBackendConfig),
    }
}
