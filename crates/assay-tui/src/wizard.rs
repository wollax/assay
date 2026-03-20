/// Multi-step wizard state for in-TUI milestone authoring.
///
/// Stub implementation — full logic lands in T02.
pub struct WizardState {
    pub step: usize,
    pub fields: Vec<Vec<String>>,
    pub cursor: usize,
    pub chunk_count: usize,
    pub error: Option<String>,
}

/// The kind of input expected at the current wizard step.
pub enum StepKind {
    Name,
    Description,
    ChunkCount,
    ChunkName(usize),
    Criteria(usize),
}

/// The result of processing a key event in the wizard.
pub enum WizardAction {
    Continue,
    Submit(assay_core::wizard::WizardInputs),
    Cancel,
}

impl WizardState {
    pub fn new() -> Self {
        todo!("T02")
    }

    pub fn current_step_kind(&self) -> StepKind {
        todo!("T02")
    }
}

/// Drive the wizard forward with a single key event.
///
/// Returns [`WizardAction::Submit`] when the user completes all steps,
/// [`WizardAction::Cancel`] on Escape, and [`WizardAction::Continue`] otherwise.
pub fn handle_wizard_event(
    _state: &mut WizardState,
    _event: crossterm::event::KeyEvent,
) -> WizardAction {
    todo!("T02")
}
