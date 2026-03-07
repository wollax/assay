//! Threshold evaluation for guard daemon.

use assay_types::GuardConfig;

/// Result of threshold evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdLevel {
    /// Below all thresholds.
    None,
    /// Soft threshold crossed (gentle prune).
    Soft,
    /// Hard threshold crossed (full prune + checkpoint).
    Hard,
}

/// Evaluate current session state against configured thresholds.
///
/// Checks both token percentage and file size (if byte thresholds are configured).
/// Returns the highest triggered level — hard takes precedence over soft.
pub fn evaluate_thresholds(
    config: &GuardConfig,
    context_pct: f64,
    file_size_bytes: u64,
) -> ThresholdLevel {
    // Check hard thresholds first (either metric can trigger)
    if context_pct >= config.hard_threshold {
        return ThresholdLevel::Hard;
    }
    if let Some(hard_bytes) = config.hard_threshold_bytes
        && file_size_bytes >= hard_bytes
    {
        return ThresholdLevel::Hard;
    }

    // Check soft thresholds
    if context_pct >= config.soft_threshold {
        return ThresholdLevel::Soft;
    }
    if let Some(soft_bytes) = config.soft_threshold_bytes
        && file_size_bytes >= soft_bytes
    {
        return ThresholdLevel::Soft;
    }

    ThresholdLevel::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GuardConfig {
        serde_json::from_str("{}").unwrap()
    }

    #[test]
    fn below_both_returns_none() {
        let config = default_config();
        assert_eq!(evaluate_thresholds(&config, 0.3, 0), ThresholdLevel::None);
    }

    #[test]
    fn above_soft_below_hard_returns_soft() {
        let config = default_config();
        // Default soft=0.6, hard=0.8
        assert_eq!(evaluate_thresholds(&config, 0.65, 0), ThresholdLevel::Soft);
    }

    #[test]
    fn at_soft_threshold_returns_soft() {
        let config = default_config();
        assert_eq!(evaluate_thresholds(&config, 0.6, 0), ThresholdLevel::Soft);
    }

    #[test]
    fn above_hard_returns_hard() {
        let config = default_config();
        assert_eq!(evaluate_thresholds(&config, 0.85, 0), ThresholdLevel::Hard);
    }

    #[test]
    fn at_hard_threshold_returns_hard() {
        let config = default_config();
        assert_eq!(evaluate_thresholds(&config, 0.8, 0), ThresholdLevel::Hard);
    }

    #[test]
    fn file_size_triggers_soft_while_token_pct_below() {
        let config = GuardConfig {
            soft_threshold_bytes: Some(1_000_000),
            hard_threshold_bytes: Some(2_000_000),
            ..default_config()
        };
        // Token pct below soft, but file size above soft
        assert_eq!(
            evaluate_thresholds(&config, 0.3, 1_500_000),
            ThresholdLevel::Soft
        );
    }

    #[test]
    fn token_pct_triggers_hard_while_file_size_below() {
        let config = GuardConfig {
            soft_threshold_bytes: Some(1_000_000),
            hard_threshold_bytes: Some(2_000_000),
            ..default_config()
        };
        // Token pct above hard, file size below soft
        assert_eq!(
            evaluate_thresholds(&config, 0.9, 500_000),
            ThresholdLevel::Hard
        );
    }

    #[test]
    fn file_size_triggers_hard() {
        let config = GuardConfig {
            soft_threshold_bytes: Some(1_000_000),
            hard_threshold_bytes: Some(2_000_000),
            ..default_config()
        };
        assert_eq!(
            evaluate_thresholds(&config, 0.3, 2_500_000),
            ThresholdLevel::Hard
        );
    }

    #[test]
    fn no_byte_thresholds_ignores_file_size() {
        let config = default_config();
        // Large file size but no byte thresholds configured
        assert_eq!(
            evaluate_thresholds(&config, 0.3, 999_999_999),
            ThresholdLevel::None
        );
    }
}
