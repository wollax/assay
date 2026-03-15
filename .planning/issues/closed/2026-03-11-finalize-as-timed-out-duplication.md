# finalize_as_timed_out duplicates build_finalized_record

**Area:** crates/assay-core/src/gate/session.rs
**Severity:** suggestion
**Source:** PR review (phase 35)

## Description

`finalize_as_timed_out` is ~130 lines of near-identical code to `build_finalized_record`. The only difference is treatment of un-evaluated required criteria (fail vs skip). Now that `build_finalized_record` is extracted, a shared helper or parameter could reduce this duplication.
