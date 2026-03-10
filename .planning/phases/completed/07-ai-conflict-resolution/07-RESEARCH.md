# Phase 7: AI Conflict Resolution - Research

**Researched:** 2026-03-10
**Domain:** LLM-assisted merge conflict resolution in Rust
**Confidence:** MEDIUM

## Summary

Phase 7 adds an AI-assisted conflict resolver that implements the existing `ConflictHandler` trait. The resolver extracts 3-way merge context (base/ours/theirs) via git index stages, sends it alongside session task descriptions and commit messages to an LLM, then presents the proposed resolution as a colored unified diff for the user to accept, edit, or reject.

The Rust LLM client ecosystem has matured significantly. The `genai` crate (v0.5.x) provides a production-ready multi-provider abstraction that matches the project's requirements perfectly — pluggable backends, async/tokio, custom endpoint support via `ServiceTargetResolver`. For diff generation, the `similar` crate is the standard. No HTTP or LLM dependencies exist in the project today, so this phase introduces `reqwest` (transitively via `genai`), `genai`, and `similar` as new dependencies.

**Primary recommendation:** Use `genai` for the LLM provider abstraction and `similar` for unified diff display. Build the AI handler as a `ConflictHandler` implementation in `smelt-core` that delegates to a provider trait, with CLI-layer presentation in `smelt-cli`. Extract 3-way context via `git show :1:/:2:/:3:` (index stages) and send full file content with structured prompts.

## Standard Stack

### Core

| Library  | Version | Purpose                          | Why Standard                                                                                   |
| -------- | ------- | -------------------------------- | ---------------------------------------------------------------------------------------------- |
| genai    | 0.5.x   | Multi-provider LLM client        | 14+ providers (Anthropic, OpenAI, Gemini, Ollama, DeepSeek, etc.), async/tokio, custom targets |
| similar  | 2.x     | Diff computation & unified diffs | Standard Rust diff library (by mitsuhiko), Myers/Patience algorithms, zero dependencies        |
| reqwest  | 0.13.x  | HTTP client (transitive)         | Pulled in by genai; industry standard Rust HTTP client                                         |

### Supporting

| Library | Version | Purpose            | When to Use                                         |
| ------- | ------- | ------------------ | --------------------------------------------------- |
| serde   | 1.x     | Config serialization | Already in workspace; used for AI config in TOML    |
| toml    | 1.x     | Config file parsing  | Already in workspace; `.smelt/config.toml` for keys |
| console | 0.16    | Colored diff output  | Already in workspace; terminal styling              |

### Alternatives Considered

| Instead of | Could Use              | Tradeoff                                                                                                 |
| ---------- | ---------------------- | -------------------------------------------------------------------------------------------------------- |
| genai      | Raw reqwest + hand-roll | Full control but massive effort; genai already handles auth, streaming, model routing for 14+ providers  |
| genai      | rig-core               | Heavier framework (agents, RAG, vector stores); overkill for single-purpose conflict resolution          |
| genai      | llm crate              | Stripe-like API but less mature; genai has better provider coverage and active maintenance               |
| similar    | imara-diff             | Better worst-case perf; similar has better ergonomics and unified diff formatting built-in               |

**Installation (workspace Cargo.toml):**
```toml
# LLM
genai = "0.5"

# Diff display
similar = { version = "2", features = ["unicode"] }
```

## Architecture Patterns

### Recommended Module Structure

```
crates/smelt-core/src/
├── ai/                      # NEW: AI provider abstraction
│   ├── mod.rs               # AiProvider trait, AiConfig, prompt construction
│   ├── provider.rs          # genai-backed provider implementation
│   └── prompt.rs            # Prompt templates and context formatting
├── merge/
│   ├── mod.rs               # ConflictHandler trait (unchanged)
│   ├── conflict.rs          # ConflictScan (unchanged)
│   ├── types.rs             # + AiAssisted, AiEdited ResolutionMethod variants
│   └── ai_handler.rs        # NEW: AiConflictHandler implementing ConflictHandler
└── ...

crates/smelt-cli/src/
├── commands/
│   └── merge.rs             # InteractiveConflictHandler wraps AiConflictHandler
│                             # + diff display, accept/edit/reject UX, difftool integration
└── ...
```

### Pattern 1: Layered Handler Composition

**What:** The AI handler lives in `smelt-core` and handles the LLM call + file writing. The CLI layer wraps it to add interactive UX (diff display, accept/edit/reject prompting, difftool launching).

**When to use:** Always — this matches the existing core/cli split where `InteractiveConflictHandler` is in cli and `ConflictHandler` trait is in core.

```rust
// smelt-core: AI handler does the LLM work, returns proposed content
pub struct AiConflictHandler<P: AiProvider> {
    provider: P,
    config: AiConfig,
}

impl<P: AiProvider> ConflictHandler for AiConflictHandler<P> {
    async fn handle_conflict(
        &self,
        session_name: &str,
        files: &[String],
        scan: &ConflictScan,
        work_dir: &Path,
    ) -> Result<ConflictAction> {
        // 1. Extract 3-way context per file
        // 2. Build prompt with session descriptions + commit messages
        // 3. Call LLM via provider
        // 4. Parse response, write resolved files
        // 5. Return ConflictAction::Resolved
    }
}
```

### Pattern 2: Provider Trait Abstraction

**What:** A thin `AiProvider` trait in `smelt-core` that wraps the genai `Client`. This gives a test seam and future extensibility.

```rust
// smelt-core::ai
pub trait AiProvider: Send + Sync {
    /// Send a prompt and return the LLM's text response.
    fn complete(
        &self,
        model: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> impl Future<Output = Result<String>> + Send;
}

// Production implementation backed by genai::Client
pub struct GenAiProvider {
    client: genai::Client,
}
```

### Pattern 3: 3-Way Context Extraction via Git Index Stages

**What:** During a merge conflict, git stores three versions in the index. Extract them with `git show :1:file` (base), `:2:file` (ours/HEAD), `:3:file` (theirs/merging branch).

```rust
// New methods needed on GitOps trait:
async fn show_index_stage(
    &self,
    work_dir: &Path,
    stage: u8,  // 1=base, 2=ours, 3=theirs
    file: &str,
) -> Result<String>;

// Implementation: git show :N:path
async fn show_index_stage(&self, work_dir: &Path, stage: u8, file: &str) -> Result<String> {
    self.run_in(work_dir, &["show", &format!(":{stage}:{file}")]).await
}
```

### Pattern 4: Structured Prompt Template

**What:** The prompt includes the 3-way context, task descriptions, and clear instructions for the LLM.

```text
You are resolving a git merge conflict. Output ONLY the resolved file content, no explanations.

## Context
- Session: {session_name}
- Task description: {task_description}
- This session was implementing: {commit_messages}

## Base version (common ancestor)
```{language}
{base_content}
```

## Current version (ours — target branch)
```{language}
{ours_content}
```

## Incoming version (theirs — session branch)
```{language}
{theirs_content}
```

## Instructions
Merge both changes. Preserve all functionality from both versions.
If changes are to different parts of the file, include both.
If changes conflict on the same lines, integrate both intents.
Output the complete resolved file.
```

### Pattern 5: Retry with Feedback

**What:** When the user rejects a resolution, capture feedback text and append it to the prompt for the next attempt.

```rust
struct RetryContext {
    attempt: usize,
    max_attempts: usize,
    feedback_history: Vec<String>,  // accumulates across retries
}
```

### Anti-Patterns to Avoid

- **Sending only conflict markers:** Always send full 3-way content. Conflict markers lose context about non-conflicting changes that inform resolution.
- **Parsing LLM output as patches:** Have the LLM output the complete resolved file, not a patch/diff. Patches are fragile to generate correctly; full file content is unambiguous.
- **Blocking on LLM calls without timeout:** Always set a timeout on the genai `Client` to prevent indefinite hangs.
- **Coupling provider logic to CLI:** Keep LLM calls in `smelt-core`; only presentation (diff display, prompts) in `smelt-cli`.

## Don't Hand-Roll

| Problem                    | Don't Build              | Use Instead             | Why                                                                |
| -------------------------- | ------------------------ | ----------------------- | ------------------------------------------------------------------ |
| Multi-provider LLM client  | Custom HTTP + auth logic | `genai` crate           | Auth, streaming, model routing, 14+ providers already implemented  |
| Unified diff generation    | String manipulation      | `similar::TextDiff`     | Handles edge cases (empty files, binary, encoding) correctly       |
| Colored terminal diff      | Manual ANSI codes        | `console` + `similar`   | Already in workspace; consistent with existing conflict display    |
| HTTP client                | Raw TCP/TLS              | `reqwest` (via genai)   | TLS, connection pooling, proxy support, async                      |
| Config file parsing        | Custom parser            | `toml` + `serde`        | Already in workspace; `.smelt/config.toml` is the natural location |
| Retry/backoff              | Manual sleep loops       | Simple counter + loop   | genai handles transport-level retries; app-level retries are simple |

**Key insight:** The LLM client is the hardest thing to get right (auth flows, streaming, error handling per provider, model name mapping). `genai` handles all of this. The rest (prompt construction, diff display, UX flow) is straightforward application code.

## Common Pitfalls

### Pitfall 1: LLM Returns Partial or Wrapped Output

**What goes wrong:** The LLM wraps the resolved file in markdown code fences, adds explanatory text, or truncates long files.
**Why it happens:** Default LLM behavior is to be "helpful" with formatting.
**How to avoid:**
- System prompt: "Output ONLY the raw file content. No markdown fences. No explanations."
- Post-processing: Strip leading/trailing code fences if present (regex `^```\w*\n` and `\n```$`).
- Validation: Compare line count of output to expected range; reject wildly different lengths.
**Warning signs:** Resolved file has triple backticks at start/end, or is significantly shorter than the original.

### Pitfall 2: Sending Conflict Markers Instead of 3-Way Content

**What goes wrong:** Sending the working-tree file (with `<<<<<<<`/`=======`/`>>>>>>>` markers) instead of clean base/ours/theirs versions.
**Why it happens:** It's the path of least resistance — the conflicted file is right there on disk.
**How to avoid:** Always extract from git index stages (`:1:`, `:2:`, `:3:`). The conflict marker format loses information (no base version in default merge style) and is harder for the LLM to parse correctly.
**Warning signs:** Prompt contains `<<<<<<<` markers.

### Pitfall 3: Not Handling API Failures Gracefully

**What goes wrong:** Network timeout, rate limit (429), auth failure (401/403), or model not found causes a panic or cryptic error.
**Why it happens:** LLM API calls are inherently unreliable — network issues, provider outages, quota exhaustion.
**How to avoid:**
- Map genai errors to user-friendly `SmeltError` variants.
- On failure: surface the error message, then offer retry/manual/abort (same as Phase 6 interactive flow).
- Never let an API error abort the entire merge without user consent.
**Warning signs:** Unwrapped `.expect()` calls on LLM responses.

### Pitfall 4: Forgetting to Stage Resolved Files

**What goes wrong:** LLM writes resolved content to disk, but the merge loop's re-scan still finds markers or files aren't staged.
**Why it happens:** The existing merge loop (line 418-488 of `mod.rs`) re-scans after `ConflictAction::Resolved` and does `git add .` only after markers are gone.
**How to avoid:** The AI handler must:
1. Write resolved content to the conflicted files on disk.
2. Return `ConflictAction::Resolved`.
3. The existing loop handles re-scan and staging.
**Warning signs:** Infinite re-prompt loop after AI resolution.

### Pitfall 5: Config Key Exposure in Error Messages

**What goes wrong:** API keys or tokens appear in error messages, logs, or panic traces.
**Why it happens:** genai or reqwest may include request headers in error output.
**How to avoid:**
- Never log the full error chain from HTTP clients without sanitizing.
- Redact `Authorization` headers in any error display.
- Use `tracing` with appropriate levels (keys never at INFO or below).
**Warning signs:** grep for "Bearer" or "x-api-key" in log output.

### Pitfall 6: diff3 Conflict Style Assumption

**What goes wrong:** Code assumes `merge.conflictstyle = diff3` is set, expecting a `|||||||` base section in conflict markers.
**Why it happens:** Developer has diff3 configured locally, doesn't realize it's not default.
**How to avoid:** Don't parse conflict markers at all for context extraction. Use git index stages (`:1:/:2:/:3:`) which work regardless of merge.conflictstyle setting.
**Warning signs:** Base content is empty or parsing fails on some machines.

## Code Examples

### Extracting 3-Way Context from Git Index

```rust
// During a merge conflict, git stores three versions in the index:
// Stage 1 = base (common ancestor)
// Stage 2 = ours (HEAD / target branch)
// Stage 3 = theirs (source / session branch)

async fn extract_three_way(
    git: &impl GitOps,
    work_dir: &Path,
    file: &str,
) -> Result<(String, String, String)> {
    let base = git.show_index_stage(work_dir, 1, file).await
        .unwrap_or_default(); // base may not exist (new file on both sides)
    let ours = git.show_index_stage(work_dir, 2, file).await?;
    let theirs = git.show_index_stage(work_dir, 3, file).await?;
    Ok((base, ours, theirs))
}
```

### Generating Colored Unified Diff with `similar`

```rust
use similar::TextDiff;

fn format_colored_diff(original: &str, resolved: &str, filename: &str) -> String {
    let diff = TextDiff::from_lines(original, resolved);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let (sign, style) = match change.tag() {
            similar::ChangeTag::Delete => ("-", console::Style::new().red()),
            similar::ChangeTag::Insert => ("+", console::Style::new().green()),
            similar::ChangeTag::Equal  => (" ", console::Style::new().dim()),
        };
        output.push_str(&format!("{}", style.apply_to(format!("{sign}{change}"))));
    }
    output
}

// Or use built-in unified diff format:
fn unified_diff(original: &str, resolved: &str, filename: &str) -> String {
    TextDiff::from_lines(original, resolved)
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{filename}"), &format!("b/{filename}"))
        .to_string()
}
```

### genai Client Setup with Custom Config

```rust
use genai::Client;
use genai::chat::{ChatMessage, ChatRequest};

async fn resolve_with_llm(
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    let client = Client::default();
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(system_prompt),
        ChatMessage::user(user_prompt),
    ]);
    let response = client.exec_chat(model, chat_req, None).await
        .map_err(|e| SmeltError::AiResolution {
            message: format!("LLM call failed: {e}"),
        })?;
    response.first_text()
        .ok_or_else(|| SmeltError::AiResolution {
            message: "LLM returned empty response".to_string(),
        })
        .map(|s| s.to_string())
}
```

### Launching git difftool Programmatically

```rust
// Check if user has a difftool configured
async fn has_difftool(git: &impl GitOps) -> bool {
    // git config --get diff.tool
    git.config_get("diff.tool").await.is_ok()
}

// Launch difftool to show original vs resolved
async fn launch_difftool(
    work_dir: &Path,
    original_path: &Path,  // temp file with original content
    resolved_path: &Path,  // the actual file with AI resolution
) -> Result<()> {
    let status = tokio::process::Command::new("git")
        .args(["difftool", "--no-prompt", "--"])
        .arg(original_path)
        .arg(resolved_path)
        .current_dir(work_dir)
        .status()
        .await?;
    // difftool is fire-and-forget for display
    Ok(())
}
```

### AI Config in `.smelt/config.toml`

```toml
[ai]
# Enable/disable AI conflict resolution (overridden by --no-ai flag)
enabled = true

# Provider: "anthropic", "openai", "ollama", etc.
provider = "anthropic"

# Model override (default: provider-specific mid-range)
# model = "claude-sonnet-4-20250514"

# Max retry attempts before falling back to manual
max_retries = 2

# API key (prefer env var ANTHROPIC_API_KEY or OPENAI_API_KEY)
# api_key = "sk-..."

# Custom endpoint (for proxies, self-hosted)
# endpoint = "https://my-proxy.example.com/v1"
```

## State of the Art

| Old Approach                     | Current Approach                            | When Changed | Impact                                                     |
| -------------------------------- | ------------------------------------------- | ------------ | ---------------------------------------------------------- |
| 2-way conflict markers only      | 3-way merge (base + ours + theirs)          | Standard     | LLMs perform significantly better with base context        |
| Provider-specific SDKs           | Multi-provider abstractions (genai, rig)    | 2025-2026    | One integration supports all major providers               |
| Simple prompt → direct resolve   | Strategy classification + LLM (CHATMERGE)   | 2024         | Classify conflict type first, then apply appropriate strategy |
| No historical context            | RAG with past resolutions (LLMinus)         | 2026         | Similar past conflicts inform resolution; not needed for v1 |

**Deprecated/outdated:**
- `reqwest` 0.11/0.12 — current is 0.13.x (genai uses 0.13)
- Manual OpenAI/Anthropic HTTP clients — `genai` handles all providers uniformly
- `diff` crate — `similar` is the maintained successor with better API

## Open Questions

1. **genai `ChatOptions` for timeout/temperature control**
   - What we know: genai has a `ChatOptions` parameter on `exec_chat()`. Temperature/max_tokens likely configurable there.
   - What's unclear: Exact struct fields and whether timeout is per-request or client-wide.
   - Recommendation: Check genai docs at implementation time; this won't affect the plan structure.

2. **genai error types and retry semantics**
   - What we know: genai returns its own `Error` type. Rate limiting behavior is provider-dependent.
   - What's unclear: Whether genai surfaces HTTP status codes (429, 503) distinctly for retry logic.
   - Recommendation: Wrap genai errors into `SmeltError` variants; implement app-level retry with configurable count.

3. **Optimal prompt for conflict resolution**
   - What we know: Full 3-way context outperforms 2-way markers. Claude and DeepSeek V3 perform best. "Output only file content" instructions are critical.
   - What's unclear: Whether per-file or batch prompting produces better results.
   - Recommendation: Start with per-file prompting (simpler, more reliable). Batch optimization is a future enhancement.

4. **git difftool invocation from within a worktree**
   - What we know: `git difftool` uses `$LOCAL` and `$REMOTE` temp files. `--no-prompt` skips confirmation.
   - What's unclear: Whether difftool works correctly when run inside a temporary merge worktree (may inherit config from main repo).
   - Recommendation: Test during implementation. Fallback to colored terminal diff if difftool launch fails.

## Sources

### Primary (HIGH confidence)
- genai crate v0.5.x — [GitHub](https://github.com/jeremychone/rust-genai), [crates.io](https://crates.io/crates/genai) — API pattern, provider support, ServiceTargetResolver
- similar crate — [GitHub](https://github.com/mitsuhiko/similar), Context7 `/mitsuhiko/similar` — unified diff generation, TextDiff API
- reqwest 0.13.x — [crates.io](https://crates.io/crates/reqwest) — current version verified
- Git documentation — [git-merge-file](https://git-scm.com/docs/git-merge-file), [git-difftool](https://git-scm.com/docs/git-difftool), [git-mergetool](https://git-scm.com/docs/git-mergetool) — index stages, difftool invocation
- Existing smelt codebase — `ConflictHandler` trait, `InteractiveConflictHandler`, `GitOps` trait, merge loop

### Secondary (MEDIUM confidence)
- merde.ai blog (Sketch) — [blog post](https://sketch.dev/blog/merde) — ~50% resolution rate, Claude/DeepSeek best performers, full file output strategy
- LLMinus RFC v2 (LKML) — [announcement](https://lkml.org/lkml/2026/1/11/542) — adaptive RAG, token limit enforcement, semantic conflict detection
- CHATMERGE (IEEE) — strategy classification + LLM for complex conflicts

### Tertiary (LOW confidence)
- Exact genai `ChatOptions` field names — needs verification at implementation time
- Optimal temperature/token settings for conflict resolution — needs experimentation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — genai and similar are well-documented, active, and fit the requirements exactly
- Architecture: HIGH — follows existing codebase patterns (trait in core, UX in cli), established Rust patterns
- Pitfalls: MEDIUM — drawn from general LLM integration experience and merde.ai learnings; some are hypothetical
- Prompt structure: MEDIUM — no single authoritative source; based on multiple references and general best practices

**Research date:** 2026-03-10
**Valid until:** 2026-04-10 (30 days — ecosystem is stable; genai may release minor updates)
