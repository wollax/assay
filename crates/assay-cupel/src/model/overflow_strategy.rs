/// Controls pipeline behavior when selected items exceed the token budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OverflowStrategy {
    /// Raise an error when selected items exceed `targetTokens` (default).
    #[default]
    Throw,
    /// Remove lowest-priority non-pinned items until the total fits.
    Truncate,
    /// Accept the over-budget selection and report overflow to an observer.
    Proceed,
}
