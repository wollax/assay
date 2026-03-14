use std::collections::HashMap;

use crate::model::{ContextBudget, ContextItem, ContextKind, ScoredItem};
use crate::slicer::Slicer;
use crate::CupelError;

/// A single quota entry specifying require and cap percentages for a kind.
#[derive(Debug, Clone)]
pub struct QuotaEntry {
    /// The context kind this quota applies to.
    pub kind: ContextKind,
    /// Minimum guaranteed percentage of the budget. Range: [0.0, 100.0].
    pub require: f64,
    /// Maximum percentage of the budget. Range: [0.0, 100.0].
    pub cap: f64,
}

/// A decorator slicer that partitions items by [`ContextKind`], distributes the
/// token budget across kinds using configurable quotas, and delegates per-kind
/// selection to an inner slicer.
pub struct QuotaSlice {
    quotas: Vec<QuotaEntry>,
    inner: Box<dyn Slicer>,
}

impl QuotaSlice {
    /// Creates a new `QuotaSlice` with the given quota entries and inner slicer.
    ///
    /// # Validation
    ///
    /// - For each entry: `require <= cap`.
    /// - The sum of all `require` percentages must not exceed 100%.
    ///
    /// # Errors
    ///
    /// Returns `CupelError::SlicerConfig` if validation fails.
    pub fn new(quotas: Vec<QuotaEntry>, inner: Box<dyn Slicer>) -> Result<Self, CupelError> {
        let mut require_sum = 0.0;
        for q in &quotas {
            if q.require > q.cap {
                return Err(CupelError::SlicerConfig(format!(
                    "require ({}) must be <= cap ({}) for kind '{}'",
                    q.require, q.cap, q.kind,
                )));
            }
            require_sum += q.require;
        }
        if require_sum > 100.0 {
            return Err(CupelError::SlicerConfig(format!(
                "sum of require percentages ({require_sum}) must not exceed 100.0",
            )));
        }
        Ok(Self { quotas, inner })
    }

    fn get_cap(&self, kind: &ContextKind) -> f64 {
        self.quotas
            .iter()
            .find(|q| &q.kind == kind)
            .map_or(100.0, |q| q.cap)
    }
}

impl Slicer for QuotaSlice {
    fn slice(&self, sorted: &[ScoredItem], budget: &ContextBudget) -> Vec<ContextItem> {
        if sorted.is_empty() || budget.target_tokens() <= 0 {
            return Vec::new();
        }

        let target_tokens = budget.target_tokens();

        // Phase 1: Partition by ContextKind (case-insensitive via ContextKind's Eq/Hash)
        let mut partitions: HashMap<ContextKind, Vec<ScoredItem>> = HashMap::new();
        for si in sorted {
            partitions
                .entry(si.item.kind().clone())
                .or_default()
                .push(si.clone());
        }

        // Phase 2: Candidate token mass per kind
        let mut candidate_token_mass: HashMap<ContextKind, i64> = HashMap::new();
        for (kind, items) in &partitions {
            let mass: i64 = items.iter().map(|si| si.item.tokens()).sum();
            candidate_token_mass.insert(kind.clone(), mass);
        }

        // Phase 3: Budget distribution
        // Step 1: Compute require and cap token amounts
        let mut require_tokens: HashMap<ContextKind, i64> = HashMap::new();
        let mut cap_tokens: HashMap<ContextKind, i64> = HashMap::new();

        for q in &self.quotas {
            require_tokens.insert(
                q.kind.clone(),
                (q.require / 100.0 * target_tokens as f64).floor() as i64,
            );
            cap_tokens.insert(
                q.kind.clone(),
                (q.cap / 100.0 * target_tokens as f64).floor() as i64,
            );
        }

        // Step 2: Sum required tokens
        let total_required: i64 = require_tokens.values().sum();
        let unassigned_budget = (target_tokens - total_required).max(0);

        // Step 3: Compute distribution mass
        let mut total_mass_for_distribution: i64 = 0;
        for kind in partitions.keys() {
            let cap = cap_tokens.get(kind).copied().unwrap_or(target_tokens);
            let require = require_tokens.get(kind).copied().unwrap_or(0);
            if cap > require {
                total_mass_for_distribution +=
                    candidate_token_mass.get(kind).copied().unwrap_or(0);
            }
        }

        // Step 4: Distribute per kind
        let mut kind_budgets: HashMap<ContextKind, i64> = HashMap::new();
        for kind in partitions.keys() {
            let require = require_tokens.get(kind).copied().unwrap_or(0);
            let cap = cap_tokens.get(kind).copied().unwrap_or(target_tokens);

            let proportional = if total_mass_for_distribution > 0 && cap > require {
                let mass = candidate_token_mass.get(kind).copied().unwrap_or(0);
                (unassigned_budget as f64 * mass as f64 / total_mass_for_distribution as f64)
                    .floor() as i64
            } else {
                0
            };

            let mut kind_budget = require + proportional;
            if kind_budget > cap {
                kind_budget = cap;
            }

            kind_budgets.insert(kind.clone(), kind_budget);
        }

        // Phase 4: Per-kind slicing
        let mut all_selected: Vec<ContextItem> = Vec::new();
        for (kind, items) in &partitions {
            let kind_budget = kind_budgets.get(kind).copied().unwrap_or(0);
            if kind_budget <= 0 {
                continue;
            }

            let cap = (self.get_cap(kind) / 100.0 * target_tokens as f64).floor() as i64;

            // Create a sub-budget for the inner slicer.
            // Use unwrap since cap >= kind_budget >= 0 and target <= max.
            let sub_budget = ContextBudget::new(
                cap,
                kind_budget,
                0,
                HashMap::new(),
                0.0,
            )
            .expect("sub-budget should be valid since cap >= kind_budget >= 0");

            let selected = self.inner.slice(items, &sub_budget);
            all_selected.extend(selected);
        }

        all_selected
    }
}
