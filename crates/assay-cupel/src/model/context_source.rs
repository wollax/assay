use std::fmt;
use std::hash::{Hash, Hasher};

use crate::CupelError;

/// An extensible string enumeration identifying the origin of a context item.
///
/// Comparison is case-insensitive using ASCII case folding.
#[derive(Debug, Clone)]
pub struct ContextSource(String);

impl ContextSource {
    /// Well-known source: user chat interaction (default).
    pub const CHAT: &str = "Chat";
    /// Well-known source: tool or function call.
    pub const TOOL: &str = "Tool";
    /// Well-known source: retrieval-augmented generation.
    pub const RAG: &str = "Rag";

    /// Creates a new `ContextSource` from the given string.
    ///
    /// Rejects empty or whitespace-only strings.
    pub fn new(value: impl Into<String>) -> Result<Self, CupelError> {
        let s = value.into();
        if s.trim().is_empty() {
            return Err(CupelError::EmptySource);
        }
        Ok(Self(s))
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ContextSource {
    fn default() -> Self {
        Self(Self::CHAT.to_owned())
    }
}

impl PartialEq for ContextSource {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for ContextSource {}

impl Hash for ContextSource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.0.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for ContextSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
