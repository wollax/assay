# Feature Landscape: Gate Composability

**Domain:** Quality gate composability primitives and guided authoring wizard
**Researched:** 2026-04-11
**Milestone:** v0.7.0

## Context: What Already Exists

The evaluation engine, `GateKind`/`Criterion`/`GatesSpec` types, enforcement levels, `gate_evaluate`/`gate_report`/`gate_finalize` MCP tools, and the milestone/spec wizard (TUI + CLI + MCP) are all shipped. `GatesSpec` already carries a `depends` field (spec-level dependency ordering). The wizard creates milestones with chunks and `gates.toml` files.

The gap is: criteria and gate definitions cannot be shared across specs. Every `gates.toml` is self-contained. Teams copy-paste common criteria (e.g., "cargo fmt --check", "cargo test --workspace") into every spec.

---

## Table Stakes

Features users expect from any composable config system. Missing = the feature feels incomplete or unusable.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Criteria inheritance (`gate.extends`) | Every CI tool (GitLab CI `extends`, GitHub Actions composite actions, Ansible roles) provides some form of config inheritance. Without it, reuse requires copy-paste, which diverges over time. | Medium | Needs: a named parent gate definition, merge semantics (child criteria append or override), cycle detection (already exists for `depends`). The existing `depends` field is spec-level ordering, not criteria inheritance — these are distinct. |
| Criteria libraries (`include` field) | GitHub Actions reusable workflows, GitLab CI `!reference` tags, Ansible roles all use shared libraries. Users define "standard Rust project" criteria once and reference it by name. | Medium | Needs: a library file location convention (e.g., `.assay/libraries/*.toml`), a `GateCriterionLibrary` type, and resolution at gate-load time. Different from `extends`: a library is a named bag of criteria, not a full gate definition. |
| Validation of inheritance chains | SonarQube, Terraform, and pytest all give clear, structured errors when a referenced parent/precondition is missing or circular. Without this, broken references are silent data corruption. | Low | Extend `spec_validate` diagnostics. Cycle detection code exists for `depends`. |
| Wizard support for composability | Users need to reference libraries and parents during spec creation without manually editing TOML. If the wizard ignores the new fields, the feature is not discoverable. | Medium | Wizard already supports multiple surfaces (CLI, TUI, MCP). Add a step for "include criteria from library?" and "extend a parent gate?" |
| MCP tools for composability management | The existing gate and spec MCP tools are the primary agent interface. Agents need to list available libraries and resolve inheritance chains programmatically. | Low-Medium | `gate_library_list`, `gate_library_get` are the minimum. Alternatively, fold into `spec_get` with resolved flag. |

## Differentiators

Features that set Assay apart — not expected, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Spec preconditions | Prevents agents from running expensive gates when a required prior spec has not passed. Terraform and pytest both have precondition/skip-if patterns. In Assay this is stronger: a spec declares that its gates only make sense after another spec's gate has passed. | Medium | Needs a `[preconditions]` TOML section on `GatesSpec` referencing spec slugs by name plus a minimum pass state. Evaluate at `gate run` time before dispatching any criteria. Distinct from `depends` (which is ordering only, no state check). |
| Override semantics on inheritance | GitLab `extends` uses deep merge (child wins on conflict). GitHub composite actions do not support override — you get all steps from every referenced action. Assay can do better: explicit `override = true` on a criterion replaces the parent's criterion of the same name, while absent-override appends. This avoids hidden shadowing surprises. | Medium | Adds one field to `Criterion`. Merge logic lives in `assay-core::gates` at load time. |
| Dry-run / preview for wizard-applied libraries | Before the wizard writes files, show the user the resolved criteria list (what they inherit + what they add). Analogous to `terraform plan`. Aligns with existing dry-run convention in assay-core. | Low-Medium | Return `WizardPreview` struct from core before committing writes. Already a precedent (pruning dry-run). |
| Agent-driven library curation via MCP | An agent can call `gate_library_create` to extract repeated criteria patterns from existing specs into a named library. Surfaces the value of composability to agents (not just human authors). | Medium | Requires `gate_library_create` MCP tool. Wraps the same TOML-write path the wizard uses. |

## Anti-Features

Features to explicitly NOT build in this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Multi-level inheritance depth > 2 | GitLab recommends max 3 levels; real codebases rarely need more, and deep chains produce confusing merge behavior that is hard to debug. | Cap at 2 levels in the resolver; return a diagnostic error for deeper chains. |
| Runtime criteria composition (dynamic inheritance from gate results) | Premature abstraction. The existing gate system is static config; runtime composition belongs to the orchestrator (v0.8+). | Keep `extends` and `include` as static, load-time resolution only. |
| GUI library browser | Out of scope for v0.7.0 TUI work. TUI wizard covers authoring; browsing is a read operation. | Provide `spec show` and `gate_library_list` for navigation; full browser is post-v0.7.0. |
| Parameterized/template criteria (variable substitution) | Introduces a mini-language inside TOML. Over-engineered for the current problem. | Shell commands already support env vars at runtime. No Mustache/Jinja in TOML. |
| Cross-repository library references | Introduces network I/O, caching, versioning, and trust problems. | Libraries live in `.assay/libraries/` in the current repo only. Remote refs are a post-v1.0 concern. |

## Feature Dependencies

```
GateCriterionLibrary type
  └── library resolution at gate-load time
        ├── gate.extends (references another GatesSpec by slug)
        └── include field (references named libraries)
              └── spec_validate diagnostics extension (missing/circular refs)

spec preconditions
  └── gate run dispatch (check precondition state before criteria eval)
        └── gate_history query (existing tool, check prior pass/fail)

wizard composability steps (optional extends + include selection)
  └── library listing (must be able to enumerate .assay/libraries/)

MCP gate_library_list / gate_library_get
  └── GateCriterionLibrary type
```

**Existing dependencies satisfied:**
- `GatesSpec.depends` (ordering/cycle detection) — exists; preconditions build on top
- `spec_validate` with structured diagnostics — exists; extend for inheritance validation
- Wizard (CLI + TUI + MCP) — exists; add composability steps to `WizardInputs`
- Atomic TOML write pattern — exists; reuse for library file writes
- `gate_history` MCP tool — exists; precondition check queries this

## MVP Recommendation

Prioritize in this order:

1. **`GateCriterionLibrary` type + `.assay/libraries/` convention** — foundation everything else depends on. A `Vec<Criterion>` with a name and description, serializable to TOML. Zero new logic required beyond the existing `Criterion` type.
2. **`include` field on `GatesSpec`** — `include = ["rust-baseline", "security-checks"]`. Resolved at load time by appending the referenced library's criteria to the spec's criteria list. Extends `spec_validate` to report missing/circular includes.
3. **`gate.extends` on `GatesSpec`** — `extends = "parent-gate-slug"` where the parent is another `GatesSpec`. Resolved by prepending parent criteria; child criteria with `override = true` on a criterion replace the parent criterion of the same name.
4. **Spec preconditions** — `[preconditions]` section on `GatesSpec` referencing spec slugs and a minimum state (`passed`). Checked by `gate_evaluate` and `gate run` before dispatching any criteria. Returns a structured skip reason in the result, analogous to existing `advisory` enforcement output.
5. **Wizard composability steps** — Add optional "include libraries?" and "extend a parent gate?" steps to the existing multi-step wizard in CLI, TUI, and MCP surfaces using the existing `WizardInputs` pattern.

**Defer (post-MVP):**
- `gate_library_create` MCP tool (agent-driven curation): useful but not blocking. Add after the type system stabilizes.
- Dry-run preview for wizard: low effort; add if time permits before release.

## Confidence Assessment

| Area | Confidence | Rationale |
|------|------------|-----------|
| Table stakes scope | HIGH | Clear precedent from GitLab CI `extends`, GitHub composite actions, SonarQube quality profiles. The pattern is well-understood. |
| Criteria library pattern | HIGH | Directly analogous to GitLab `!reference` and GitHub reusable workflows. `Vec<Criterion>` + named file is the straightforward implementation. |
| Spec preconditions | HIGH | Terraform preconditions, pytest `skipif`/`pytest-dependency`, and CI `needs:` keyword all confirm this is the right primitive. Existing `depends` covers ordering; preconditions add state-checking. |
| Wizard extension complexity | MEDIUM | TUI wizard state machine exists and is multi-step (confirmed from source). Adding optional steps is additive but requires care to avoid state machine regressions. |
| Override semantics on extends | MEDIUM | GitLab deep-merge behavior is documented; name-keyed replacement is the simplest correct choice for Assay's dual-track criteria, but the decision needs to be made explicitly and documented. |

## Sources

- GitLab CI YAML optimization (extends, !reference, include) — https://docs.gitlab.com/ci/yaml/yaml_optimization/
- GitHub Actions composite actions vs. reusable workflows — https://dev.to/n3wt0n/composite-actions-vs-reusable-workflows-what-is-the-difference-github-actions-11kd
- SonarQube quality gates (confirmed: no inheritance, assignment model only) — https://docs.sonarsource.com/sonarqube-server/quality-standards-administration/managing-quality-gates/introduction-to-quality-gates
- Terraform preconditions and postconditions — https://spacelift.io/blog/terraform-precondition-postcondition
- pytest-dependency plugin — https://pytest-dependency.readthedocs.io/en/latest/usage.html
- Assay codebase: `crates/assay-types/src/gates_spec.rs`, `criterion.rs`, `gate.rs`; `crates/assay-core/src/wizard.rs`; `crates/assay-tui/src/wizard.rs`
