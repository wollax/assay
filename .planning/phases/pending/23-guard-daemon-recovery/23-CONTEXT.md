# Phase 23: Guard Daemon & Recovery - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Background daemon with tiered threshold response, reactive overflow recovery, and circuit breaker for context protection. Watches Claude Code session files, triggers pruning strategies at configurable thresholds, and escalates through gentle/standard/aggressive prescriptions with a circuit breaker to prevent infinite recovery loops.

</domain>

<decisions>
## Implementation Decisions

### Daemon lifecycle
- Plugin hook auto-starts the daemon when a Claude Code session begins; CLI also available for manual control
- CLI subcommands: `assay context guard {start,stop,status,logs}`
- Auto-discovers active Claude Code session file by default; `--session <path>` flag to override

### Claude's Discretion: Daemon state location
- PID file and daemon state directory location (e.g., `.assay/guard/` vs XDG runtime dir) — Claude picks based on project-local vs system-level tradeoffs

### Threshold configuration
- Thresholds configured in `.assay/config.toml` under a `[guard]` section
- Both percentage-based (context window %) and file-size thresholds supported independently; whichever fires first triggers action
- Default thresholds: soft at 60%, hard at 80% of context window
- Default polling interval: 5 seconds

### Recovery feedback
- Log file for detailed output; short summary line to stderr when a prune triggers (visible if terminal is open)
- `assay context guard logs` subcommand supports `--level` flag for filtering by log level

### Claude's Discretion: Recovery feedback details
- Exact stderr summary format and verbosity level
- Log format choice (structured JSON lines vs human-readable text)
- Whether to checkpoint before soft threshold prune or only before hard threshold prune

### Session reload
- Claude researches what's actually possible with Claude Code's current plugin/hook system and picks the best mechanism for triggering session reload on hard threshold

### Circuit breaker
- Circuit breaker recovery limit and time window are configurable in config.toml (default: 3 recoveries in 10 minutes)

### Claude's Discretion: Circuit breaker behavior
- Whether circuit breaker halts the daemon entirely or enters a cooldown mode
- Whether escalating prescription level resets after a quiet period or persists until restart

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 23-guard-daemon-recovery*
*Context gathered: 2026-03-06*
