# Phase 7 Context: AI Conflict Resolution

## Summary

Phase 7 adds an AI-assisted conflict resolver as the first attempt before the human fallback (Phase 6). The existing `ConflictHandler` trait provides the integration point — Phase 7 implements a new handler that calls an LLM, presents the proposed resolution to the user, and falls back to manual resolution if rejected or if the API fails.

## Decisions

### 1. LLM Provider & API Configuration

**Decision:** Pluggable provider abstraction from the start.

- Support multiple backends: Claude Code, OpenCode, Codex, direct API, subscription, proxy — modeled behind a trait/abstraction so the resolver doesn't care which provider is used.
- API key configuration uses layered resolution: environment variable → config file (`.smelt/config.toml`) → CLI flag. Each layer can override the previous.
- Model selection is user-configurable via config file, with a sensible default per provider (mid-range model, e.g., Sonnet-class rather than Opus-class).
- Rate limits and cost guardrails are user-configurable/definable (not hardcoded).

### 2. Conflict Context Construction

**Decision:** Rich context, send everything, let the model handle it.

- Send full 3-way merge view: base version + both branch versions (not just 2-way conflict markers).
- Include session task descriptions from the manifest (what each agent was trying to do).
- Include commit messages from both branches for additional intent signals.
- Include full file content (not just the conflicted region) so the model has surrounding context.
- No truncation — send everything and let the model manage its context window.
- **Deferred idea:** Shared context engine library (kata-context-style) usable by both Smelt and Assay. For Phase 7, build context construction inline. If a shared library emerges later, extract from here.

### 3. Resolution Presentation & Acceptance UX

**Decision:** Mode-dependent presentation with git tooling integration.

- **Diff display:** Open in user's configured git difftool/mergetool if set in gitconfig. Otherwise, fall back to colored unified diff in terminal. Configurable and overrideable.
- **Acceptance flow:** Accept / Edit / Reject (ternary, not binary). "Edit" lets the user tweak the AI's proposal before accepting.
- **Multi-file behavior depends on mode:**
  - **Autonomous mode:** Resolve all files, present together as a batch.
  - **Interactive mode:** Present and confirm one file at a time, sequentially.
- **Retry with feedback:** When the user rejects a proposal, they can provide textual feedback (e.g., "keep the imports from session-a") and the AI retries with that guidance.

### 4. Fallback Chain Behavior

**Decision:** AI-first with configurable retry, then manual fallback.

- **Resolution chain:** AI attempt → reject → retry with feedback (up to configurable max) → fall back to Phase 6 manual handler (resolve/skip/abort).
- **API failures** (network, rate limit, auth): Surface the error to the user and prompt with retry/manual/abort options. In autonomous mode, the orchestrator can have a policy (e.g., "retry once then skip").
- **Max retry count:** Configurable (default TBD during planning). After exhausting retries, go straight to manual.
- **Disable AI entirely:** `--no-ai` flag or config setting bypasses AI resolution completely, preserving Phase 6 behavior.
- **Autonomous mode policy:** Orchestrator can define fallback behavior (e.g., "retry once then skip", "retry once then manual then abort") — fits with mode-dependent presentation from area 3.

## Deferred Ideas

- **Shared context engine:** A kata-context-style library that both Smelt and Assay can use for rich context construction. Phase 7 builds inline; extract later if warranted.

## Constraints

- Must implement the existing `ConflictHandler` trait (no changes to the trait signature).
- `ResolutionMethod` enum needs new variant(s) for AI resolution (e.g., `AiAssisted`, `AiEdited`).
- Phase 6's `InteractiveConflictHandler` remains the fallback — AI handler delegates to it on rejection/failure.
- Provider abstraction lives in smelt-core; CLI-specific presentation (difftool integration, interactive prompts) lives in smelt-cli.

## Open Questions for Research

- Which provider SDKs/APIs to evaluate for the abstraction layer?
- How to invoke git difftool/mergetool programmatically and capture the result?
- How to construct the 3-way merge context efficiently (git show for base/ours/theirs)?
- What prompt structure produces the best conflict resolution results?
