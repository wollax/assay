//! Circuit breaker preventing infinite recovery loops.
//!
//! Tracks recovery timestamps in a sliding window. Trips when
//! max_recoveries is exceeded within recovery_window_secs.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use assay_types::context::PrescriptionTier;

/// Circuit breaker state machine for guard daemon recovery limiting.
///
/// Tracks recovery attempts in a sliding time window and trips when
/// the maximum number of recoveries is exceeded. Escalation maps
/// recovery count to prescription tier (gentle -> standard -> aggressive).
pub struct CircuitBreaker {
    /// Maximum recoveries allowed before tripping.
    max_recoveries: u32,
    /// Time window for counting recoveries.
    window: Duration,
    /// Timestamps of recent recovery attempts (oldest first).
    recoveries: VecDeque<Instant>,
    /// Whether the breaker has tripped.
    tripped: bool,
}

impl CircuitBreaker {
    /// Create a new circuit breaker.
    ///
    /// # Arguments
    /// - `max_recoveries`: Maximum recoveries allowed within the window before tripping.
    /// - `window_secs`: Duration of the sliding window in seconds.
    pub fn new(max_recoveries: u32, window_secs: u64) -> Self {
        Self {
            max_recoveries,
            window: Duration::from_secs(window_secs),
            recoveries: VecDeque::new(),
            tripped: false,
        }
    }

    /// Record a recovery attempt and return the current count of recoveries in the window.
    ///
    /// Prunes expired entries first, then adds the current timestamp.
    /// The caller is responsible for checking [`should_trip`] and calling [`trip`] if needed.
    pub fn record_recovery(&mut self) -> u32 {
        self.prune_old();
        self.recoveries.push_back(Instant::now());
        self.recovery_count()
    }

    /// Whether the circuit breaker should trip (recoveries in window >= max).
    pub fn should_trip(&self) -> bool {
        self.recovery_count() >= self.max_recoveries
    }

    /// Whether the circuit breaker has been tripped.
    pub fn is_tripped(&self) -> bool {
        self.tripped
    }

    /// Trip the circuit breaker.
    pub fn trip(&mut self) {
        self.tripped = true;
    }

    /// Count of recoveries currently within the sliding window.
    pub fn recovery_count(&self) -> u32 {
        self.recoveries.len() as u32
    }

    /// Map the current recovery count to an escalating prescription tier.
    ///
    /// - 0 or 1 recovery: [`PrescriptionTier::Gentle`]
    /// - 2 recoveries: [`PrescriptionTier::Standard`]
    /// - 3+ recoveries: [`PrescriptionTier::Aggressive`]
    pub fn current_tier(&self) -> PrescriptionTier {
        match self.recovery_count() {
            0 | 1 => PrescriptionTier::Gentle,
            2 => PrescriptionTier::Standard,
            _ => PrescriptionTier::Aggressive,
        }
    }

    /// Reset the circuit breaker if no recoveries remain in the window.
    ///
    /// Prunes expired entries first. If the window is empty, resets
    /// the tripped state, allowing the daemon to resume.
    pub fn reset_if_quiet(&mut self) {
        self.prune_old();
        if self.recoveries.is_empty() {
            self.tripped = false;
        }
    }

    /// Remove entries from the front of the deque that have expired past the window.
    fn prune_old(&mut self) {
        while let Some(front) = self.recoveries.front() {
            if front.elapsed() > self.window {
                self.recoveries.pop_front();
            } else {
                break;
            }
        }
    }

    /// Record a recovery at a specific instant (test helper).
    #[cfg(test)]
    fn record_recovery_at(&mut self, instant: Instant) {
        self.prune_old();
        self.recoveries.push_back(instant);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_breaker_not_tripped() {
        let cb = CircuitBreaker::new(3, 60);
        assert_eq!(cb.recovery_count(), 0);
        assert!(!cb.is_tripped());
        assert!(!cb.should_trip());
    }

    #[test]
    fn single_recovery_not_tripped() {
        let mut cb = CircuitBreaker::new(3, 60);
        let count = cb.record_recovery();
        assert_eq!(count, 1);
        assert!(!cb.should_trip());
        assert!(!cb.is_tripped());
    }

    #[test]
    fn max_recoveries_trips() {
        let mut cb = CircuitBreaker::new(3, 60);
        cb.record_recovery();
        cb.record_recovery();
        cb.record_recovery();
        assert!(cb.should_trip());

        // Caller is responsible for tripping
        assert!(!cb.is_tripped());
        cb.trip();
        assert!(cb.is_tripped());
    }

    #[test]
    fn old_recoveries_pruned() {
        let mut cb = CircuitBreaker::new(3, 60);

        // Insert a recovery that's already expired (70 seconds ago)
        let old = Instant::now() - Duration::from_secs(70);
        cb.record_recovery_at(old);
        assert_eq!(cb.recovery_count(), 1);

        // Recording a new recovery prunes the old one first
        let count = cb.record_recovery();
        assert_eq!(count, 1); // old one pruned, only new one remains
    }

    #[test]
    fn escalation_gentle() {
        let mut cb = CircuitBreaker::new(3, 60);
        assert_eq!(cb.current_tier(), PrescriptionTier::Gentle); // 0 recoveries

        cb.record_recovery();
        assert_eq!(cb.current_tier(), PrescriptionTier::Gentle); // 1 recovery
    }

    #[test]
    fn escalation_standard() {
        let mut cb = CircuitBreaker::new(3, 60);
        cb.record_recovery();
        cb.record_recovery();
        assert_eq!(cb.current_tier(), PrescriptionTier::Standard);
    }

    #[test]
    fn escalation_aggressive() {
        let mut cb = CircuitBreaker::new(5, 60);
        cb.record_recovery();
        cb.record_recovery();
        cb.record_recovery();
        assert_eq!(cb.current_tier(), PrescriptionTier::Aggressive);

        // 4+ also aggressive
        cb.record_recovery();
        assert_eq!(cb.current_tier(), PrescriptionTier::Aggressive);
    }

    #[test]
    fn reset_after_quiet() {
        let mut cb = CircuitBreaker::new(3, 60);

        // Insert expired recoveries and trip
        let old = Instant::now() - Duration::from_secs(70);
        cb.record_recovery_at(old);
        cb.record_recovery_at(old);
        cb.record_recovery_at(old);
        cb.trip();
        assert!(cb.is_tripped());

        // After quiet period (all entries expired), reset
        cb.reset_if_quiet();
        assert!(!cb.is_tripped());
        assert_eq!(cb.recovery_count(), 0);
    }

    #[test]
    fn reset_does_not_clear_if_active() {
        let mut cb = CircuitBreaker::new(3, 60);
        cb.record_recovery();
        cb.record_recovery();
        cb.record_recovery();
        cb.trip();

        // Active recoveries still in window — should stay tripped
        cb.reset_if_quiet();
        assert!(cb.is_tripped());
        assert_eq!(cb.recovery_count(), 3);
    }

    #[test]
    fn recovery_count_accurate() {
        let mut cb = CircuitBreaker::new(5, 60);

        // Start with an old recovery, then add fresh ones
        let old = Instant::now() - Duration::from_secs(70);
        cb.record_recovery_at(old);
        assert_eq!(cb.recovery_count(), 1);

        // Next record_recovery prunes the old entry (it's at the front), then adds new
        let count = cb.record_recovery();
        assert_eq!(count, 1); // old pruned, only new remains

        cb.record_recovery();
        cb.record_recovery();
        assert_eq!(cb.recovery_count(), 3); // 3 fresh recoveries
    }

    #[test]
    fn trips_at_exactly_max() {
        let mut cb = CircuitBreaker::new(2, 60);
        cb.record_recovery();
        assert!(!cb.should_trip()); // 1 < 2

        cb.record_recovery();
        assert!(cb.should_trip()); // 2 >= 2
    }
}
