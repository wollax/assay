//! Session manifest, types, and scripted session support.

pub mod manifest;
pub mod script;
pub mod types;

pub use manifest::{
    FailureMode, FileChange, Manifest, ManifestMeta, ScriptDef, ScriptStep, SessionDef,
};
pub use script::ScriptExecutor;
pub use types::{SessionOutcome, SessionResult};
