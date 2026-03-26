//! Backend configuration for state persistence.

use crate::schema_registry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Backend configuration for state persistence.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
