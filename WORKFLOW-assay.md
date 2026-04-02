---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  project_slug: assay-d9ecdc5c6203
  # assignee: alice
  active_states:
    - Todo
    - In Progress
    - Agent Review
    - Merging
    - Rework
  terminal_states:
    - Closed
    - Cancelled
    - Canceled
    - Duplicate
    - Done
polling:
  interval_ms: 30000
workspace:
  root: ~/assay-workspaces
  repo: /Users/wollax/Git/personal/assay
  git_strategy: worktree
  isolation: local
  cleanup_on_done: true
  branch_prefix: assay
  clone_branch: main
  base_branch: main
hooks:
  timeout_ms: 120000
agent:
  backend: kata-cli
  max_concurrent_agents: 4
  max_turns: 20
kata_agent:
  command: kata
  model: anthropic/claude-opus-4-6
  model_by_state:
    Agent Review: codex/gpt-5.4
    Merging: anthropic/claude-sonnet-4-6
  stall_timeout_ms: 900000
prompts:
  system: prompts/system.md
  repo: prompts/repo-sym.md
  by_state:
    Todo: prompts/in-progress.md
    In Progress: prompts/in-progress.md
    Agent Review: prompts/agent-review.md
    Merging: prompts/merging.md
    Rework: prompts/rework.md
  default: prompts/in-progress.md
server:
  port: 8080
  host: "127.0.0.1"
supervisor:
  enabled: true
  model: anthropic/claude-opus-4-6
  steer_cooldown_ms: 120000
---
