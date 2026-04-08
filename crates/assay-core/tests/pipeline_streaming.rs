//! Integration tests for `launch_agent_streaming`.
//!
//! Proves line-by-line delivery via mpsc channel and correct exit-code
//! reporting using real OS subprocess pipes. The channel now carries typed
//! [`AgentEvent`] values (S03/T01): plain-text stdout lines are forwarded
//! as synthetic `TextDelta { text: line, block_index: 0 }` events.

use std::sync::mpsc;

use assay_core::pipeline::launch_agent_streaming;
use assay_types::AgentEvent;

fn text_delta_texts(events: &[AgentEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            AgentEvent::TextDelta { text, .. } => text.clone(),
            other => panic!("expected TextDelta, got {other:?}"),
        })
        .collect()
}

/// Verify that `launch_agent_streaming` delivers every line to the receiver
/// channel in order, with no extra events. Plain-text lines become
/// synthetic `TextDelta` events via the relay fallback path.
#[test]
fn streaming_delivers_lines_to_receiver() {
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>();

    let handle = launch_agent_streaming(
        &[
            "sh".to_string(),
            "-c".to_string(),
            r#"printf "alpha\nbeta\ngamma\n"; exit 0"#.to_string(),
        ],
        std::path::Path::new("/tmp"),
        event_tx,
    );

    let events: Vec<AgentEvent> = event_rx.into_iter().collect();
    let exit_code = handle.relay.join().expect("thread should not panic");

    assert_eq!(text_delta_texts(&events), vec!["alpha", "beta", "gamma"]);
    assert_eq!(exit_code, 0);
}

/// Verify that the `JoinHandle` returned by `launch_agent_streaming` resolves
/// to the correct exit code.
#[test]
fn streaming_join_handle_returns_exit_code() {
    let (event_tx, _event_rx) = mpsc::channel::<AgentEvent>();

    let handle = launch_agent_streaming(
        &["sh".to_string(), "-c".to_string(), "exit 42".to_string()],
        std::path::Path::new("/tmp"),
        event_tx,
    );

    let exit_code = handle.relay.join().expect("thread should not panic");
    assert_eq!(exit_code, 42);
}

/// Verify that a process exiting with code 1 is reported as non-zero.
#[test]
fn streaming_failed_process_returns_nonzero() {
    let (event_tx, _event_rx) = mpsc::channel::<AgentEvent>();

    let handle = launch_agent_streaming(
        &[
            "sh".to_string(),
            "-c".to_string(),
            "echo err; exit 1".to_string(),
        ],
        std::path::Path::new("/tmp"),
        event_tx,
    );

    let exit_code = handle.relay.join().expect("thread should not panic");
    assert_eq!(exit_code, 1);
}

/// Verify that `launch_agent_streaming` returns -1 for a nonexistent binary.
#[test]
fn streaming_nonexistent_binary_returns_negative_one() {
    let (event_tx, _event_rx) = mpsc::channel::<AgentEvent>();

    let handle = launch_agent_streaming(
        &["__nonexistent_binary_assay_test__".to_string()],
        std::path::Path::new("/tmp"),
        event_tx,
    );

    let exit_code = handle.relay.join().expect("thread should not panic");
    assert_eq!(exit_code, -1, "expected -1 for nonexistent binary");
}
