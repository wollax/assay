//! TUI event type shared between `main.rs` and `app.rs`.

use assay_core::pr::PrStatusInfo;

/// Events dispatched through the TUI event loop.
///
/// The channel-based dispatch loop in `run()` receives these. `AgentLine` and
/// `AgentDone` variants are sent by the agent background thread; `Key` and
/// `Resize` are sent by the crossterm event thread. `PrStatusUpdate` is sent
/// by the background PR polling thread.
pub enum TuiEvent {
    /// A keyboard event from crossterm.
    Key(crossterm::event::KeyEvent),
    /// A terminal resize event.
    Resize(u16, u16),
    /// A single line of stdout from the agent subprocess.
    AgentLine(String),
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
