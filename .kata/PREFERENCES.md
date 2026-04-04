---
version: 1
always_use_skills: []
prefer_skills: []
avoid_skills: []
skill_rules: []
custom_instructions: []
pr:
  enabled: true
  auto_create: true
  base_branch: main
  review_on_create: true
models:
  review: claude-sonnet-4-6
  planner: claude-opus-4-6
skill_discovery: suggest
auto_supervisor: {}
symphony:
  url: http://127.0.0.1:8081
workflow:
  mode: linear
linear:
  teamKey: WOL
  projectId: fdb234ce-bc6e-47db-ba8f-2bc6df4fd39f
---

# Kata Skill Preferences

Project-specific guidance for skill selection and execution preferences.

See `~/.kata-cli/agent/extensions/kata/docs/preferences-reference.md` for full field documentation and examples.

## Fields

- `always_use_skills`: Skills that must be available during all Kata operations
- `prefer_skills`: Skills to prioritize when multiple options exist
- `avoid_skills`: Skills to minimize or avoid (with lower priority than prefer)
- `skill_rules`: Context-specific rules (e.g., "use tool X for Y type of work")
- `custom_instructions`: Append-only project guidance (do not override system rules)
- `models`: Model preferences for specific task types
- `skill_discovery`: Automatic skill detection preferences
- `auto_supervisor`: Supervision and gating rules for autonomous modes

## Examples

```yaml
prefer_skills:
  - playwright
  - resolve_library
avoid_skills:
  - subagent  # prefer direct execution in this project

custom_instructions:
  - "Always verify with browser_assert before marking UI work done"
  - "Use Context7 for all library/framework decisions"
```
