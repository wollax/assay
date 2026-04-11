# Requirements: Assay v0.7.0 Gate Composability

**Defined:** 2026-04-11
**Core Value:** Dual-track quality gates (deterministic + agent-evaluated) for AI coding agents

## v0.7.0 Requirements

Requirements for gate composability milestone. Each maps to roadmap phases.

### Gate Inheritance

- [x] **INHR-01**: User can define a gate that extends another gate via `gate.extends` field
- [x] **INHR-02**: Extended gate inherits parent criteria with own-wins merge semantics
- [x] **INHR-03**: Circular `extends` chains are detected and reported as validation errors
- [x] **INHR-04**: Gate run output shows per-criterion source annotation (parent vs own)

### Criteria Libraries

- [x] **CLIB-01**: User can define shared criteria sets in `.assay/criteria/<slug>.toml`
- [x] **CLIB-02**: User can reference criteria libraries via `include` field in gate definitions
- [x] **CLIB-03**: Core API supports load, save, and scan operations for criteria libraries
- [ ] **CLIB-04**: Agent can create criteria libraries programmatically via `criteria_create` MCP tool

### Spec Preconditions

- [ ] **PREC-01**: User can define `[preconditions].requires` — gate skipped unless named spec's last gate run passed
- [ ] **PREC-02**: User can define `[preconditions].commands` — shell commands that must succeed before gate evaluation
- [ ] **PREC-03**: Precondition failures produce distinct `PreconditionFailed` result (blocked != failed)

### Wizard — CLI

- [ ] **WIZC-01**: User can create new gate definitions via `assay gate wizard` interactive flow
- [ ] **WIZC-02**: User can edit existing gate definitions via the wizard
- [ ] **WIZC-03**: User can manage criteria libraries via `assay criteria list/new` commands

### Wizard — MCP

- [ ] **WIZM-01**: Agent can drive gate wizard via `gate_wizard` MCP tool
- [ ] **WIZM-02**: Agent can discover criteria libraries via `criteria_list` and `criteria_get` MCP tools
- [ ] **WIZM-03**: Agent can resolve a spec's effective criteria via `spec_resolve` MCP tool

### Wizard — TUI

- [ ] **WIZT-01**: User can create and edit gate definitions via TUI wizard screen
- [ ] **WIZT-02**: TUI wizard delegates all validation to core (no surface-specific logic)

### Validation & Safety

- [ ] **SAFE-01**: `spec_validate` detects composability errors (missing parents, missing libraries, cycle detection)
- [ ] **SAFE-02**: `extends` and `include` values are slug-validated to prevent path traversal
- [x] **SAFE-03**: All new `GatesSpec` fields are backward-compatible (existing TOML files parse without error)

## Future Requirements

### Gate Composability Extensions

- **INHR-05**: Multi-level inheritance (> 2 levels) with configurable depth limit
- **CLIB-05**: Parameterized/template criteria with variable substitution
- **CLIB-06**: Cross-repo library references
- **PREC-04**: Precondition staleness window (gate must have passed within N minutes)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Runtime/dynamic composition | Load-time static resolution is simpler and sufficient for v0.7.0 |
| GUI library browser | TUI wizard is sufficient; dedicated browser is over-scoped |
| `toml_edit` for comment preservation | Wizard writes new files; comment preservation adds complexity with low value |
| Multi-level inheritance > 2 | Adds transitive cycle complexity; depth 1 covers all practical cases |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| INHR-01 | Phase 64 | Complete |
| INHR-02 | Phase 64 | Complete |
| INHR-03 | Phase 65 | Complete |
| INHR-04 | Phase 65 | Complete |
| CLIB-01 | Phase 65 | Complete |
| CLIB-02 | Phase 65 | Complete |
| CLIB-03 | Phase 65 | Complete |
| CLIB-04 | Phase 68 | Pending |
| PREC-01 | Phase 66 | Pending |
| PREC-02 | Phase 66 | Pending |
| PREC-03 | Phase 66 | Pending |
| WIZC-01 | Phase 67 | Pending |
| WIZC-02 | Phase 67 | Pending |
| WIZC-03 | Phase 67 | Pending |
| WIZM-01 | Phase 68 | Pending |
| WIZM-02 | Phase 68 | Pending |
| WIZM-03 | Phase 68 | Pending |
| WIZT-01 | Phase 69 | Pending |
| WIZT-02 | Phase 69 | Pending |
| SAFE-01 | Phase 66 | Pending |
| SAFE-02 | Phase 66 | Pending |
| SAFE-03 | Phase 64 | Complete |

**Coverage:**
- v0.7.0 requirements: 22 total
- Mapped to phases: 22
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-11*
*Last updated: 2026-04-11 after roadmap creation*
