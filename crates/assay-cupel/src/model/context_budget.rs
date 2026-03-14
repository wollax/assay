use std::collections::HashMap;

use crate::CupelError;
use crate::model::ContextKind;

/// Token budget constraints that control how much context the pipeline can select.
///
/// All fields are validated at construction time — no invalid budget can exist at runtime.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    max_tokens: i64,
    target_tokens: i64,
    output_reserve: i64,
    reserved_slots: HashMap<ContextKind, i64>,
    estimation_safety_margin_percent: f64,
}

impl ContextBudget {
    /// Creates a new `ContextBudget`, validating all spec constraints.
    ///
    /// # Validation Rules
    /// 1. `max_tokens >= 0`
    /// 2. `target_tokens >= 0`
    /// 3. `target_tokens <= max_tokens`
    /// 4. `output_reserve >= 0`
    /// 5. `output_reserve <= max_tokens`
    /// 6. `estimation_safety_margin_percent` in `[0.0, 100.0]`
    /// 7. All `reserved_slots` values `>= 0`
    pub fn new(
        max_tokens: i64,
        target_tokens: i64,
        output_reserve: i64,
        reserved_slots: HashMap<ContextKind, i64>,
        estimation_safety_margin_percent: f64,
    ) -> Result<Self, CupelError> {
        if max_tokens < 0 {
            return Err(CupelError::InvalidBudget(
                "maxTokens must be >= 0".to_owned(),
            ));
        }
        if target_tokens < 0 {
            return Err(CupelError::InvalidBudget(
                "targetTokens must be >= 0".to_owned(),
            ));
        }
        if target_tokens > max_tokens {
            return Err(CupelError::InvalidBudget(
                "targetTokens must be <= maxTokens".to_owned(),
            ));
        }
        if output_reserve < 0 {
            return Err(CupelError::InvalidBudget(
                "outputReserve must be >= 0".to_owned(),
            ));
        }
        if output_reserve > max_tokens {
            return Err(CupelError::InvalidBudget(
                "outputReserve must be <= maxTokens".to_owned(),
            ));
        }
        if !(0.0..=100.0).contains(&estimation_safety_margin_percent) {
            return Err(CupelError::InvalidBudget(
                "estimationSafetyMarginPercent must be in [0.0, 100.0]".to_owned(),
            ));
        }
        for (kind, &count) in &reserved_slots {
            if count < 0 {
                return Err(CupelError::InvalidBudget(format!(
                    "reserved slot count for kind '{}' must be >= 0",
                    kind,
                )));
            }
        }

        Ok(Self {
            max_tokens,
            target_tokens,
            output_reserve,
            reserved_slots,
            estimation_safety_margin_percent,
        })
    }

    /// Hard ceiling: the model's context window size.
    pub fn max_tokens(&self) -> i64 {
        self.max_tokens
    }

    /// Soft goal: the slicer aims for this token count.
    pub fn target_tokens(&self) -> i64 {
        self.target_tokens
    }

    /// Tokens reserved for model output generation.
    pub fn output_reserve(&self) -> i64 {
        self.output_reserve
    }

    /// Minimum guaranteed items per kind.
    pub fn reserved_slots(&self) -> &HashMap<ContextKind, i64> {
        &self.reserved_slots
    }

    /// Percentage buffer for token estimation error.
    pub fn estimation_safety_margin_percent(&self) -> f64 {
        self.estimation_safety_margin_percent
    }
}
