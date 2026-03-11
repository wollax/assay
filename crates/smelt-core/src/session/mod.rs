//! Session manifest, types, scripted session support, and agent execution.

pub mod agent;
pub mod manifest;
pub mod process;
pub mod runner;
pub mod script;
pub mod types;

pub use agent::AgentExecutor;
pub use manifest::{
    FailureMode, FileChange, Manifest, ManifestMeta, ScriptDef, ScriptStep, SessionDef,
};
pub use process::ProcessGroup;
pub use runner::SessionRunner;
pub use script::ScriptExecutor;
pub use types::{SessionOutcome, SessionResult};
