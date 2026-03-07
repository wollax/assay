//! Guard daemon configuration validation.

pub use assay_types::GuardConfig;

/// Validate a `GuardConfig`, returning a list of validation errors.
///
/// An empty vector means the configuration is valid.
pub fn validate(config: &GuardConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if !(0.0..=1.0).contains(&config.soft_threshold) {
        errors.push(format!(
            "soft_threshold must be in 0.0..=1.0, got {}",
            config.soft_threshold
        ));
    }

    if !(0.0..=1.0).contains(&config.hard_threshold) {
        errors.push(format!(
            "hard_threshold must be in 0.0..=1.0, got {}",
            config.hard_threshold
        ));
    }

    if config.soft_threshold >= config.hard_threshold {
        errors.push(format!(
            "soft_threshold ({}) must be less than hard_threshold ({})",
            config.soft_threshold, config.hard_threshold
        ));
    }

    if config.poll_interval_secs == 0 {
        errors.push("poll_interval_secs must be greater than 0".to_string());
    }

    if config.max_recoveries == 0 {
        errors.push("max_recoveries must be greater than 0".to_string());
    }

    if config.recovery_window_secs == 0 {
        errors.push("recovery_window_secs must be greater than 0".to_string());
    }

    if let (Some(soft_bytes), Some(hard_bytes)) =
        (config.soft_threshold_bytes, config.hard_threshold_bytes)
        && soft_bytes >= hard_bytes
    {
        errors.push(format!(
            "soft_threshold_bytes ({soft_bytes}) must be less than hard_threshold_bytes ({hard_bytes})"
        ));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> GuardConfig {
        serde_json::from_str("{}").unwrap()
    }

    #[test]
    fn valid_default_config_passes() {
        let errors = validate(&default_config());
        assert!(
            errors.is_empty(),
            "default config should be valid: {errors:?}"
        );
    }

    #[test]
    fn soft_gte_hard_fails() {
        let config = GuardConfig {
            soft_threshold: 0.8,
            hard_threshold: 0.6,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("soft_threshold") && e.contains("less than")),
            "should report soft >= hard: {errors:?}"
        );
    }

    #[test]
    fn threshold_out_of_range_fails() {
        let config = GuardConfig {
            soft_threshold: -0.1,
            hard_threshold: 1.5,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.len() >= 2,
            "should report both out-of-range: {errors:?}"
        );
    }

    #[test]
    fn zero_poll_interval_fails() {
        let config = GuardConfig {
            poll_interval_secs: 0,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.iter().any(|e| e.contains("poll_interval_secs")),
            "should report zero poll interval: {errors:?}"
        );
    }

    #[test]
    fn zero_max_recoveries_fails() {
        let config = GuardConfig {
            max_recoveries: 0,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.iter().any(|e| e.contains("max_recoveries")),
            "should report zero max_recoveries: {errors:?}"
        );
    }

    #[test]
    fn zero_recovery_window_fails() {
        let config = GuardConfig {
            recovery_window_secs: 0,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.iter().any(|e| e.contains("recovery_window_secs")),
            "should report zero recovery_window: {errors:?}"
        );
    }

    #[test]
    fn soft_bytes_gte_hard_bytes_fails() {
        let config = GuardConfig {
            soft_threshold_bytes: Some(1000),
            hard_threshold_bytes: Some(500),
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.iter().any(|e| e.contains("soft_threshold_bytes")),
            "should report soft_bytes >= hard_bytes: {errors:?}"
        );
    }

    #[test]
    fn one_byte_threshold_without_other_is_valid() {
        let config = GuardConfig {
            soft_threshold_bytes: Some(1000),
            hard_threshold_bytes: None,
            ..default_config()
        };
        let errors = validate(&config);
        assert!(
            errors.is_empty(),
            "single byte threshold should be valid: {errors:?}"
        );
    }
}
