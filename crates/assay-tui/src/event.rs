//! TUI event type shared between `main.rs` and `app.rs`.

use assay_core::pr::PrStatusInfo;
use assay_types::AgentEvent;

/// Events dispatched through the TUI event loop.
///
/// The channel-based dispatch loop in `run()` receives these. `AgentEvent` and
/// `AgentDone` variants are sent by the agent background thread; `Key` and
/// `Resize` are sent by the crossterm event thread. `PrStatusUpdate` is sent
/// by the background PR polling thread.
pub enum TuiEvent {
    /// A keyboard event from crossterm.
    Key(crossterm::event::KeyEvent),
    /// A terminal resize event.
    Resize(u16, u16),
    /// A typed agent event from the relay-wrapper thread; reconstructed into
    /// display lines by `App::handle_agent_event`.
    AgentEvent(AgentEvent),
    /// The agent subprocess has exited with the given exit code.
    AgentDone { exit_code: i32 },
    /// Background PR status poll result for a milestone.
    PrStatusUpdate {
        /// Milestone slug this status belongs to.
        slug: String,
        /// Polled PR status info.
        info: PrStatusInfo,
    },
}
