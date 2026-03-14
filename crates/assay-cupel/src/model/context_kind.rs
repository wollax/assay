use std::fmt;
use std::hash::{Hash, Hasher};

use crate::CupelError;

/// An extensible string enumeration classifying the type of a context item.
///
/// Comparison is case-insensitive using ASCII case folding.
#[derive(Debug, Clone)]
pub struct ContextKind(String);

impl ContextKind {
    /// Well-known kind: conversational message (default).
    pub const MESSAGE: &str = "Message";
    /// Well-known kind: document or file content.
    pub const DOCUMENT: &str = "Document";
    /// Well-known kind: output from a tool invocation.
    pub const TOOL_OUTPUT: &str = "ToolOutput";
    /// Well-known kind: stored memory or fact.
    pub const MEMORY: &str = "Memory";
    /// Well-known kind: system-level instruction.
    pub const SYSTEM_PROMPT: &str = "SystemPrompt";

    /// Creates a new `ContextKind` from the given string.
    ///
    /// Rejects empty or whitespace-only strings.
    pub fn new(value: impl Into<String>) -> Result<Self, CupelError> {
        let s = value.into();
        if s.trim().is_empty() {
            return Err(CupelError::EmptyKind);
        }
        Ok(Self(s))
    }

    /// Creates a `ContextKind` from a well-known constant, bypassing validation.
    /// Only used internally for statically-known non-empty strings.
    pub(crate) fn from_static(value: &str) -> Self {
        Self(value.to_owned())
    }

    /// Returns the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ContextKind {
    fn default() -> Self {
        Self(Self::MESSAGE.to_owned())
    }
}

impl PartialEq for ContextKind {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for ContextKind {}

impl Hash for ContextKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.0.bytes() {
            state.write_u8(byte.to_ascii_lowercase());
        }
    }
}

impl fmt::Display for ContextKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
