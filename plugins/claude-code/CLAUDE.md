# Assay Workflow

This project uses Assay for spec-driven development with quality gates.

## Workflow

1. **Read the spec first.** Before writing code, always read the relevant spec:
   - Use `/assay:spec-show <spec-name>` to see all criteria
   - Understand what "done" means before starting

2. **Implement against criteria.** Each criterion in the spec defines a verifiable requirement. Write code that satisfies each one.

3. **Verify with gates.** After making changes, run the quality gates:
   - Use `/assay:gate-check <spec-name>` to run all executable criteria
   - Fix any failures before moving on

4. **Iterate until all gates pass.** Do not consider work complete until all quality gates pass.

## Commands

| Command | Description |
| --- | --- |
| `/assay:spec-show [name]` | Display a spec's criteria and details |
| `/assay:gate-check [name]` | Run quality gates and report results |

## MCP Tools

The Assay MCP server provides these tools directly:

| Tool | Description |
| --- | --- |
| `spec_list` | Discover available specs in the project |
| `spec_get` | Get a spec's full definition with all criteria |
| `gate_run` | Run quality gates for a spec and get pass/fail results |
