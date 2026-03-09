# Phase 27: Types Hygiene - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Bring all public types to production quality — Eq derives where safe, Display impls on key enums, doc comments on all public items, Default on GateSection, and structural dedup on Criterion types. Requirements: TYPE-01 through TYPE-06.

</domain>

<decisions>
## Implementation Decisions

### Display format style
- Claude's discretion on whether Display output is human-readable or matches serde form
- Claude's discretion on which enums get Display (key user-facing enums vs all public enums)
- Claude's discretion on derive macro (strum) vs hand-written impls
- Claude's discretion on whether data variants include key data or just variant name

### Doc comment depth
- Claude's discretion on depth per item (terse one-liners for obvious fields, explanatory for complex types)
- Claude's discretion on cross-references between related types
- **Locked: `#![deny(missing_docs)]` at the crate level** in `assay-types` to enforce documentation going forward

### Criterion dedup strategy
- Claude's discretion on approach: merge into single Criterion type (with optional `requirements: Vec<String>`) vs shared base struct
- Claude's discretion on whether GateCriterion becomes a type alias or is fully removed
- Claude's discretion on whether `requirements` field becomes available in flat-file specs
- **Locked: Review CriterionResult** as part of this dedup — check if naming/structure can improve alongside the criterion type consolidation

### Eq derive policy
- Claude's discretion on blanket-add vs selective Eq derives
- Claude's discretion on adding Hash alongside Eq
- Claude's discretion on adding PartialEq + Eq to types that currently lack both
- **Locked: `#[deny(clippy::derive_partial_eq_without_eq)]`** at the crate level to enforce Eq alongside PartialEq going forward

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The two concrete enforcement decisions (`deny(missing_docs)` and `deny(clippy::derive_partial_eq_without_eq)`) ensure these hygiene standards are maintained after this phase.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 27-types-hygiene*
*Context gathered: 2026-03-09*
