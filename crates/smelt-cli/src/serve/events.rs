//! Event types, bounded store, and broadcast bus for Assay event ingestion.

use std::collections::VecDeque;

/// Default capacity for per-job event ring buffers.
pub const DEFAULT_EVENT_STORE_CAPACITY: usize = 256;

/// An event received from an Assay session via the ingestion endpoint.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AssayEvent {
    /// Identifier of the job this event belongs to.
    pub job_id: String,
    /// Optional caller-supplied event identifier (informational; no server-side dedup).
    pub event_id: Option<String>,
    /// Unix epoch seconds when the server received this event.
    pub received_at: u64,
    /// Opaque JSON payload (typically `OrchestratorStatus`).
    ///
    /// Control fields (`job_id`, `event_id`) are stripped from the original
    /// POST body before storage — they live on the struct fields above.
    pub payload: serde_json::Value,
}

/// Bounded per-job ring buffer for received events.
///
/// When the buffer is full, the oldest event is dropped and `dropped` is
/// incremented so consumers can detect data loss.
///
/// Query methods (`iter`, `len`, `is_empty`, `dropped`) are consumed by tests
/// and upcoming S02 (SSE fan-out, TUI event pane).
#[derive(Debug)]
#[allow(dead_code)]
pub struct EventStore {
    events: VecDeque<AssayEvent>,
    capacity: usize,
    dropped: u64,
}

impl Default for EventStore {
    /// Create an `EventStore` with the default capacity (256).
    fn default() -> Self {
        Self::new(DEFAULT_EVENT_STORE_CAPACITY)
    }
}

#[allow(dead_code)]
impl EventStore {
    /// Create a new `EventStore` with the given maximum capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "EventStore capacity must be at least 1");
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
            dropped: 0,
        }
    }

    /// Push an event into the store, dropping the oldest if at capacity.
    pub fn push(&mut self, event: AssayEvent) {
        if self.events.len() >= self.capacity {
            self.events.pop_front();
            self.dropped += 1;
        }
        self.events.push_back(event);
    }

    /// Read-only iterator over all stored events (oldest first).
    pub fn iter(&self) -> impl Iterator<Item = &AssayEvent> {
        self.events.iter()
    }

    /// Number of events dropped due to overflow since creation.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Current number of events in the store.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the store contains no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Broadcast sender for real-time event fan-out.
///
/// Subscribers call `event_bus.subscribe()` to get a `Receiver<AssayEvent>`.
pub type EventBus = tokio::sync::broadcast::Sender<AssayEvent>;
