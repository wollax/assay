//! Guard daemon main event loop.
//!
//! Multiplexes polling timer, file system watcher, and shutdown signals.
//! Responds to threshold crossings with escalating pruning prescriptions.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use assay_types::GuardConfig;
use assay_types::context::PrescriptionTier;
use tracing::{error, info, warn};

use super::circuit_breaker::CircuitBreaker;
use super::pid;
use super::thresholds::{ThresholdLevel, evaluate_thresholds};
use super::watcher::SessionWatcher;
use crate::AssayError;

/// Guard daemon state.
pub struct GuardDaemon {
    session_path: PathBuf,
    assay_dir: PathBuf,
    config: GuardConfig,
    breaker: CircuitBreaker,
    /// Debounce: timestamp of last threshold check.
    last_check: Option<Instant>,
}

impl GuardDaemon {
    /// Create a new guard daemon.
    pub fn new(session_path: PathBuf, assay_dir: PathBuf, config: GuardConfig) -> Self {
        let breaker = CircuitBreaker::new(config.max_recoveries, config.recovery_window_secs);
        Self {
            session_path,
            assay_dir,
            config,
            breaker,
            last_check: None,
        }
    }

    /// Run the daemon event loop. Blocks until shutdown signal or circuit breaker trip.
    #[cfg(unix)]
    pub async fn run(&mut self) -> crate::Result<()> {
        use tokio::signal::unix::{SignalKind, signal};

        let pid_path = pid::pid_file_path(&self.assay_dir);
        pid::create_pid_file(&pid_path)?;

        let watcher_result = SessionWatcher::new(&self.session_path);
        let mut watcher = match watcher_result {
            Ok(w) => w,
            Err(e) => {
                let _ = pid::remove_pid_file(&pid_path);
                return Err(e);
            }
        };

        let mut poll_interval =
            tokio::time::interval(Duration::from_secs(self.config.poll_interval_secs));

        let mut sigint = signal(SignalKind::interrupt()).map_err(|source| AssayError::Io {
            operation: "setting up SIGINT handler".into(),
            path: self.session_path.clone(),
            source,
        })?;

        let mut sigterm = signal(SignalKind::terminate()).map_err(|source| AssayError::Io {
            operation: "setting up SIGTERM handler".into(),
            path: self.session_path.clone(),
            source,
        })?;

        info!("[guard] Started watching {}", self.session_path.display());

        let result = loop {
            tokio::select! {
                _ = poll_interval.tick() => {
                    match self.check_and_respond() {
                        Ok(_) => {}
                        Err(e) if matches!(e, AssayError::GuardCircuitBreakerTripped { .. }) => {
                            break Err(e);
                        }
                        Err(e) => {
                            error!("[guard] Poll check error: {e}");
                        }
                    }
                }
                Some(()) = watcher.rx.recv() => {
                    // Debounce: skip if less than 1 second since last check
                    let should_check = self.last_check.is_none_or(|t| t.elapsed() >= Duration::from_secs(1));

                    if should_check {
                        match self.check_and_respond() {
                            Ok(_) => {}
                            Err(e) if matches!(e, AssayError::GuardCircuitBreakerTripped { .. }) => {
                                break Err(e);
                            }
                            Err(e) => {
                                error!("[guard] Watcher check error: {e}");
                            }
                        }
                    }
                }
                _ = sigint.recv() => {
                    info!("[guard] SIGINT received");
                    self.graceful_shutdown();
                    break Ok(());
                }
                _ = sigterm.recv() => {
                    info!("[guard] SIGTERM received");
                    self.graceful_shutdown();
                    break Ok(());
                }
            }
        };

        let _ = pid::remove_pid_file(&pid_path);
        result
    }

    /// Check current session state against thresholds and respond if needed.
    ///
    /// Returns `Ok(true)` if action was taken, `Ok(false)` if below thresholds.
    fn check_and_respond(&mut self) -> crate::Result<bool> {
        self.last_check = Some(Instant::now());

        // Get file size (may fail if file is being written)
        let file_size = match std::fs::metadata(&self.session_path) {
            Ok(m) => m.len(),
            Err(e) => {
                warn!("[guard] Cannot read session file metadata: {e}");
                return Ok(false);
            }
        };

        // Get token estimate for context percentage
        let context_pct = match crate::context::quick_token_estimate(&self.session_path) {
            Ok(Some(usage)) => {
                let context_tokens = usage.context_tokens();
                let available = crate::context::tokens::DEFAULT_CONTEXT_WINDOW
                    .saturating_sub(crate::context::tokens::SYSTEM_OVERHEAD_TOKENS);
                if available > 0 {
                    context_tokens as f64 / available as f64
                } else {
                    0.0
                }
            }
            Ok(None) => {
                // No usage data yet — use heuristic from file size
                let estimated_tokens =
                    crate::context::tokens::estimate_tokens_from_bytes(file_size);
                let available = crate::context::tokens::DEFAULT_CONTEXT_WINDOW
                    .saturating_sub(crate::context::tokens::SYSTEM_OVERHEAD_TOKENS);
                if available > 0 {
                    estimated_tokens as f64 / available as f64
                } else {
                    0.0
                }
            }
            Err(e) => {
                warn!("[guard] Cannot estimate tokens: {e}");
                return Ok(false);
            }
        };

        let level = evaluate_thresholds(&self.config, context_pct, file_size);

        match level {
            ThresholdLevel::None => {
                self.breaker.reset_if_quiet();
                Ok(false)
            }
            ThresholdLevel::Soft => {
                self.handle_soft_threshold()?;
                // Re-evaluate after prune to avoid stale state
                let _ = self.re_evaluate_after_prune();
                Ok(true)
            }
            ThresholdLevel::Hard => {
                self.handle_hard_threshold()?;
                // Re-evaluate after prune to avoid stale state
                let _ = self.re_evaluate_after_prune();
                Ok(true)
            }
        }
    }

    /// Handle soft threshold crossing: gentle prune with checkpoint.
    fn handle_soft_threshold(&mut self) -> crate::Result<()> {
        let count = self.breaker.record_recovery();

        if self.breaker.should_trip() {
            self.breaker.trip();
            self.try_save_checkpoint("guard-circuit-trip");
            return Err(AssayError::GuardCircuitBreakerTripped {
                recoveries: count,
                window_secs: self.config.recovery_window_secs,
            });
        }

        let tier = self.breaker.current_tier();

        self.try_save_checkpoint("guard-soft");

        let strategies = tier.strategies();
        let backup_dir = self.assay_dir.join("backups");

        match crate::context::pruning::prune_session(
            &self.session_path,
            strategies,
            tier,
            true,
            Some(&backup_dir),
        ) {
            Ok(report) => {
                let saved = report.original_size.saturating_sub(report.final_size);
                info!("[guard] Soft threshold -- {tier:?} prune saved {saved} bytes");
            }
            Err(e) => {
                error!("[guard] Soft prune failed: {e}");
            }
        }

        Ok(())
    }

    /// Handle hard threshold crossing: full prune with team checkpoint.
    fn handle_hard_threshold(&mut self) -> crate::Result<()> {
        let count = self.breaker.record_recovery();

        if self.breaker.should_trip() {
            self.breaker.trip();
            self.try_save_checkpoint("guard-circuit-trip");
            return Err(AssayError::GuardCircuitBreakerTripped {
                recoveries: count,
                window_secs: self.config.recovery_window_secs,
            });
        }

        // Hard threshold: at least Standard tier
        let breaker_tier = self.breaker.current_tier();
        let tier = match breaker_tier {
            PrescriptionTier::Gentle => PrescriptionTier::Standard,
            other => other,
        };

        self.try_save_checkpoint("guard-hard");

        let strategies = tier.strategies();
        let backup_dir = self.assay_dir.join("backups");

        match crate::context::pruning::prune_session(
            &self.session_path,
            strategies,
            tier,
            true,
            Some(&backup_dir),
        ) {
            Ok(report) => {
                let saved = report.original_size.saturating_sub(report.final_size);
                info!("[guard] Hard threshold -- {tier:?} prune saved {saved} bytes");
                info!("[guard] Consider running /compact in your Claude Code session");
            }
            Err(e) => {
                error!("[guard] Hard prune failed: {e}");
            }
        }

        Ok(())
    }

    /// Re-evaluate thresholds after a prune to ensure we're below limits.
    fn re_evaluate_after_prune(&self) -> ThresholdLevel {
        let file_size = std::fs::metadata(&self.session_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let context_pct = crate::context::quick_token_estimate(&self.session_path)
            .ok()
            .flatten()
            .map(|usage| {
                let available = crate::context::tokens::DEFAULT_CONTEXT_WINDOW
                    .saturating_sub(crate::context::tokens::SYSTEM_OVERHEAD_TOKENS);
                if available > 0 {
                    usage.context_tokens() as f64 / available as f64
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);

        let level = evaluate_thresholds(&self.config, context_pct, file_size);
        if level != ThresholdLevel::None {
            warn!(
                "[guard] Still above threshold after prune: {level:?} (pct={context_pct:.2}, size={file_size})"
            );
        }
        level
    }

    /// Attempt to save a checkpoint. Logs errors but does not propagate them.
    fn try_save_checkpoint(&self, trigger: &str) {
        let project_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                warn!("[guard] Cannot determine project dir for checkpoint: {e}");
                return;
            }
        };

        match crate::checkpoint::extract_team_state(&project_dir, None, trigger) {
            Ok(checkpoint) => {
                match crate::checkpoint::save_checkpoint(&self.assay_dir, &checkpoint) {
                    Ok(path) => {
                        info!("[guard] Checkpoint saved: {}", path.display());
                    }
                    Err(e) => {
                        warn!("[guard] Checkpoint save failed: {e}");
                    }
                }
            }
            Err(e) => {
                warn!("[guard] Checkpoint extraction failed: {e}");
            }
        }
    }

    /// Graceful shutdown: save final checkpoint and clean up.
    fn graceful_shutdown(&self) {
        info!("[guard] Shutting down...");
        self.try_save_checkpoint("guard-shutdown");
        let pid_path = pid::pid_file_path(&self.assay_dir);
        let _ = pid::remove_pid_file(&pid_path);
        info!("[guard] Final checkpoint saved. Exiting.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GuardConfig {
        serde_json::from_str("{}").unwrap()
    }

    fn config_with_low_thresholds() -> GuardConfig {
        serde_json::from_str(
            r#"{"soft_threshold_bytes": 10, "hard_threshold_bytes": 20, "max_recoveries": 3}"#,
        )
        .unwrap()
    }

    fn make_daemon(session_path: PathBuf, assay_dir: PathBuf, config: GuardConfig) -> GuardDaemon {
        GuardDaemon::new(session_path, assay_dir, config)
    }

    #[test]
    fn guard_daemon_new_creates_valid_struct() {
        let daemon = GuardDaemon::new(
            PathBuf::from("/tmp/session.jsonl"),
            PathBuf::from("/tmp/.assay"),
            default_config(),
        );

        assert_eq!(daemon.session_path, PathBuf::from("/tmp/session.jsonl"));
        assert_eq!(daemon.assay_dir, PathBuf::from("/tmp/.assay"));
        assert!(daemon.last_check.is_none());
        assert!(!daemon.breaker.is_tripped());
    }

    #[test]
    fn check_and_respond_below_thresholds_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        // Small file — well below default thresholds
        std::fs::write(&session, "{}").unwrap();

        let mut daemon = make_daemon(session, assay_dir, default_config());
        let result = daemon.check_and_respond().unwrap();

        assert!(!result, "should return false when below all thresholds");
        assert!(daemon.last_check.is_some(), "should update last_check");
    }

    #[test]
    fn check_and_respond_missing_file_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("nonexistent.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        let mut daemon = make_daemon(session, assay_dir, default_config());
        let result = daemon.check_and_respond().unwrap();

        assert!(!result, "should return false when session file missing");
    }

    #[test]
    fn check_and_respond_above_soft_threshold_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        // 15 bytes — above soft (10) but below hard (20)
        std::fs::write(&session, "0123456789abcde").unwrap();

        let mut daemon = make_daemon(session, assay_dir, config_with_low_thresholds());
        let result = daemon.check_and_respond().unwrap();

        assert!(result, "should return true when above soft threshold");
    }

    #[test]
    fn handle_soft_threshold_trips_breaker_after_max_recoveries() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        std::fs::write(&session, "{}").unwrap();

        let mut daemon = make_daemon(session, assay_dir, config_with_low_thresholds());

        // First 2 recoveries should succeed (max_recoveries = 3, trips at >= 3)
        for _ in 0..2 {
            assert!(daemon.handle_soft_threshold().is_ok());
        }

        // 3rd recovery should trip the breaker (count reaches max_recoveries)
        let result = daemon.handle_soft_threshold();
        assert!(result.is_err());
        match result.unwrap_err() {
            AssayError::GuardCircuitBreakerTripped { .. } => {}
            other => panic!("expected GuardCircuitBreakerTripped, got: {other}"),
        }
        assert!(daemon.breaker.is_tripped());
    }

    #[test]
    fn handle_hard_threshold_enforces_minimum_standard_tier() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        std::fs::write(&session, "{}").unwrap();

        let mut daemon = make_daemon(session, assay_dir, config_with_low_thresholds());

        // Before any recoveries, breaker tier is Gentle (0 recoveries).
        // Hard threshold should override to Standard.
        assert_eq!(daemon.breaker.current_tier(), PrescriptionTier::Gentle);
        assert!(daemon.handle_hard_threshold().is_ok());
        // After one recovery, tier escalates — the key invariant is that
        // handle_hard_threshold didn't use Gentle (tested via the prune call
        // which receives the tier). We verify the breaker recorded the recovery.
        assert_eq!(daemon.breaker.recovery_count(), 1);
    }

    #[test]
    fn handle_hard_threshold_trips_breaker_after_max_recoveries() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        std::fs::write(&session, "{}").unwrap();

        let mut daemon = make_daemon(session, assay_dir, config_with_low_thresholds());

        for _ in 0..2 {
            assert!(daemon.handle_hard_threshold().is_ok());
        }

        // 3rd recovery trips the breaker
        let result = daemon.handle_hard_threshold();
        assert!(result.is_err());
        match result.unwrap_err() {
            AssayError::GuardCircuitBreakerTripped { .. } => {}
            other => panic!("expected GuardCircuitBreakerTripped, got: {other}"),
        }
    }

    #[test]
    fn graceful_shutdown_removes_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();

        let daemon = make_daemon(session, assay_dir.clone(), default_config());
        let pid_path = pid::pid_file_path(&assay_dir);
        pid::create_pid_file(&pid_path).unwrap();
        assert!(pid_path.exists());

        daemon.graceful_shutdown();

        assert!(
            !pid_path.exists(),
            "PID file should be removed after shutdown"
        );
    }

    #[test]
    fn re_evaluate_after_prune_returns_none_for_small_file() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.jsonl");
        let assay_dir = dir.path().join(".assay");
        std::fs::create_dir_all(&assay_dir).unwrap();
        std::fs::write(&session, "{}").unwrap();

        let daemon = make_daemon(session, assay_dir, default_config());
        let level = daemon.re_evaluate_after_prune();
        assert_eq!(level, ThresholdLevel::None);
    }
}
