use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::CupelError;
use crate::model::{ContextKind, ContextSource};

/// An immutable record representing a single piece of context in the pipeline.
///
/// Constructed via [`ContextItemBuilder`]. All fields are private with public accessor methods.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextItem {
    content: String,
    tokens: i64,
    kind: ContextKind,
    source: ContextSource,
    priority: Option<i64>,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
    timestamp: Option<DateTime<Utc>>,
    future_relevance_hint: Option<f64>,
    pinned: bool,
    original_tokens: Option<i64>,
}

impl ContextItem {
    /// The textual content of this context item.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// The token count for this context item.
    pub fn tokens(&self) -> i64 {
        self.tokens
    }

    /// The kind of context item.
    pub fn kind(&self) -> &ContextKind {
        &self.kind
    }

    /// The origin of this context item.
    pub fn source(&self) -> &ContextSource {
        &self.source
    }

    /// Optional priority override. Higher values indicate greater importance.
    pub fn priority(&self) -> Option<i64> {
        self.priority
    }

    /// Descriptive tags for filtering and scoring.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Arbitrary key-value metadata, opaque to the pipeline.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// When this context item was created or observed.
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
    }

    /// Hint for future relevance scoring, conventionally in [0.0, 1.0].
    pub fn future_relevance_hint(&self) -> Option<f64> {
        self.future_relevance_hint
    }

    /// Whether this item is pinned (bypasses scoring and slicing).
    pub fn pinned(&self) -> bool {
        self.pinned
    }

    /// The original token count before any external summarization or truncation.
    pub fn original_tokens(&self) -> Option<i64> {
        self.original_tokens
    }
}

/// Builder for constructing [`ContextItem`] instances.
#[derive(Debug)]
pub struct ContextItemBuilder {
    content: String,
    tokens: i64,
    kind: ContextKind,
    source: ContextSource,
    priority: Option<i64>,
    tags: Vec<String>,
    metadata: HashMap<String, String>,
    timestamp: Option<DateTime<Utc>>,
    future_relevance_hint: Option<f64>,
    pinned: bool,
    original_tokens: Option<i64>,
}

impl ContextItemBuilder {
    /// Creates a new builder with the required `content` and `tokens` fields.
    pub fn new(content: impl Into<String>, tokens: i64) -> Self {
        Self {
            content: content.into(),
            tokens,
            kind: ContextKind::default(),
            source: ContextSource::default(),
            priority: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            timestamp: None,
            future_relevance_hint: None,
            pinned: false,
            original_tokens: None,
        }
    }

    /// Sets the kind of context item.
    pub fn kind(mut self, kind: ContextKind) -> Self {
        self.kind = kind;
        self
    }

    /// Sets the source of this context item.
    pub fn source(mut self, source: ContextSource) -> Self {
        self.source = source;
        self
    }

    /// Sets the priority override.
    pub fn priority(mut self, priority: i64) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the descriptive tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Sets the metadata map.
    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the timestamp.
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the future relevance hint.
    pub fn future_relevance_hint(mut self, hint: f64) -> Self {
        self.future_relevance_hint = Some(hint);
        self
    }

    /// Sets whether this item is pinned.
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Sets the original token count.
    pub fn original_tokens(mut self, tokens: i64) -> Self {
        self.original_tokens = Some(tokens);
        self
    }

    /// Builds the [`ContextItem`], validating that content is non-empty.
    pub fn build(self) -> Result<ContextItem, CupelError> {
        if self.content.is_empty() {
            return Err(CupelError::EmptyContent);
        }

        Ok(ContextItem {
            content: self.content,
            tokens: self.tokens,
            kind: self.kind,
            source: self.source,
            priority: self.priority,
            tags: self.tags,
            metadata: self.metadata,
            timestamp: self.timestamp,
            future_relevance_hint: self.future_relevance_hint,
            pinned: self.pinned,
            original_tokens: self.original_tokens,
        })
    }
}
