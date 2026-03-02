//! Auto-discovery registry for JSON Schema generation.
//!
//! Types register themselves via `inventory::submit!` at their definition site.
//! The generator binary iterates all entries to produce schema files.

use schemars::Schema;

/// A registered schema-generating type.
///
/// Each entry maps a kebab-case name to a function that generates the root
/// JSON Schema for that type.
pub struct SchemaEntry {
    /// Kebab-case name for the output file (e.g., "gate-result").
    pub name: &'static str,
    /// Function that generates the root schema for this type.
    pub generate: fn() -> Schema,
}

inventory::collect!(SchemaEntry);

/// Iterate all registered schema entries.
pub fn all_entries() -> impl Iterator<Item = &'static SchemaEntry> {
    inventory::iter::<SchemaEntry>.into_iter()
}
