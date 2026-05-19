## Brat

This repository uses **Brat** for AI agent orchestration. Use brat commands for work management.

### Checking Status

```bash
# Overall status (convoys, tasks, sessions)
brat status --json

# List active sessions
brat session list --json
```

### Working on Tasks

When assigned a task, you'll have task context in `.claude/current_task.md`.

```bash
# Update task status as you progress
brat task update <task_id> --status running --json
brat task update <task_id> --status needs-review --json

# Add progress comments (recorded in gritee issue)
gritee issue comment <issue_id> --body "Checkpoint: implemented X, tests passing" --json
```

### Storing Context

Store architectural observations for future agents:

```bash
# Project-level context (conventions, patterns)
gritee context set test_command "cargo test"
gritee context set build_command "cargo build"
gritee context set api_pattern "REST /api/v1/"

# Discoveries as memory issues
gritee issue create --label memory --title "[Memory] Auth flow" --body "Uses JWT..." --json

# Index codebase symbols after significant changes
gritee context index --json
```

### Reading Context

Query existing knowledge before starting:

```bash
# Project conventions
gritee context project --json

# Previous discoveries
gritee issue list --label memory --json

# Code symbols
gritee context query <function_name> --json
```
