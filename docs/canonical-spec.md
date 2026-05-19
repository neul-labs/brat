# Canonical spec (Brat + Grite)

This doc is the single source of truth for identifiers, labels, comment schemas, and file locations used by the Brat harness and Grite substrate.

## Identifiers

All IDs are ASCII and lowercase where applicable.

- `convoy_id`: `c-YYYYMMDD-<4hex>` (example `c-20250114-a2f9`)
- `task_id`: `t-YYYYMMDD-<4hex>` (example `t-20250114-3a2c`)
- `session_id`: `s-YYYYMMDD-<4hex>` (example `s-20250114-7b3d`)
- `actor_id`: 16-byte hex string (Grite actor ID)

## Label taxonomy

Canonical labels are defined in `docs/label-glossary.md`. The harness must only use labels from that list.

### Phase labels

- `phase:product`
- `phase:architecture`
- `phase:implementation`
- `phase:review`
- `phase:merge`
- `phase:memory`
- `gate:open`
- `gate:closed`

## Issue schemas

### Convoy issue

Required labels:

- `type:convoy`
- `convoy:<convoy_id>`
- `status:active|paused|complete|failed`

Body fields:

```
Title: <convoy title>
Goal: <one-line objective>
Base commit: <git sha>
Policy: <merge policy summary>
Owner: <actor_id or handle>
```

### Task issue

Required labels:

- `type:task`
- `task:<task_id>`
- `convoy:<convoy_id>`
- `status:queued|running|blocked|needs-review|merged|dropped`

Body fields:

```
Title: <task title>
Paths: <comma-separated paths>
Constraints: <brief constraints>
Acceptance: <tests or checks>
Notes: <extra context>
```

## Session comment schema

Session lifecycle is recorded via a structured comment block and labels:

- Labels: `session:spawned|ready|running|handoff|exit`, `session:polecat|crew`, `engine:<name>`
- Comment format (see `docs/session-event-schema.md`):

```
[session]
state = "running"
session_id = "s-20250114-7b3d"
role = "witness"
session_type = "polecat"
engine = "codex"
worktree = ".grite/worktrees/polecat-3"
pid = 12345
started_ts = 1700000000000
last_heartbeat_ts = 1700000005000
exit_code = null
exit_reason = null
last_output_ref = "sha256:..."
[/session]
```

## Handoff comment template

```
Summary:
Requested action:
Context:
Acceptance checks:
Deadline:
```

Use labels `to:<actor_id>`, `needs-ack`, `ack:<actor_id>`, `urgency:low|med|high`.

## Storage locations

- Grite WAL: `refs/grite/wal`
- Grite locks: `refs/grite/locks/*`
- Grite actors: `.git/grite/actors/<actor_id>/`
- Grite exports: `.grite/`
- Brat worktrees: `.grite/worktrees/polecat-<n>`
- Brat config: `.brat/config.toml`
- KB mirror: `.brat/kb/product.md`, `.brat/kb/architecture.md`
- Session logs: `.grite/logs/<session_id>.log` (hashed as `sha256:<hex>` in `last_output_ref`)

## Daemon semantics

- `grited` is optional and only accelerates substrate operations.
- `bratd` runs by default for UX but is not required for correctness.
