# Harness state machine

This document specifies the lifecycle state machines used by the Brat harness and how they are persisted in Grite.

## Goals

- Explicit lifecycle states for roles and sessions
- Idempotent transitions (safe to replay)
- Recoverable after crash or restart
- Observable in the control room via Grite queries
- Consistency-gated phase transitions for the software factory pipeline

## Session lifecycle

### States

- `spawned`: session created, not yet ready
- `ready`: engine healthy, initial prompt delivered
- `running`: actively executing task work
- `handoff`: waiting for review or merge
- `exit`: session terminated (success or failure)

### Session types

- Polecat: ephemeral worker session managed by Witness
- Crew: user-owned persistent session (manual lifecycle control)

### Transitions

- `spawned -> ready`: engine health check passes
- `ready -> running`: first task action begins
- `running -> handoff`: task ready for review or merge
- `running -> exit`: failure, timeout, or user stop
- `handoff -> exit`: task closed or session stopped

### Persistence in Grite

- Each transition is recorded as a Grite comment on the task issue
- Labels are updated to reflect the current state
- Exit transition includes exit code, reason, last output hash or snippet

## Pipeline phase lifecycle

### Phases

- `product`: Meta agent writing product notes
- `architecture`: Meta agent writing architecture notes
- `implementation`: Witness spawning agents in swimlanes
- `review`: Refinery assessing, requesting human approval
- `merge`: Refinery merging approved work
- `memory`: Agents writing memory notes

### Phase transitions (consistency-gated)

- `product -> architecture`: allowed when `product_arch_coverage` ≥ threshold
- `architecture -> implementation`: allowed when `arch_product_traceability` ≥ threshold
- `implementation -> review`: allowed when `test_feature_coverage` and `doc_component_parity` ≥ threshold
- `review -> merge`: allowed when human approves (via UI or MCP)
- `merge -> memory`: automatic after successful merge

### Gate failure behavior

- Gate closed: phase remains in current state
- Inconsistencies are surfaced to humans via UI and MCP
- Humans edit KB notes; Meta Agent re-runs consistency check
- Score updates in real-time; gate opens when threshold met

### Persistence in Grite

- Phase transitions recorded as comments on the convoy issue
- Consistency score recorded at each transition
- Gate status (open/closed) stored as a label

## Role lifecycle

### States

- `idle`: not actively coordinating
- `active`: role is executing normal duties
- `degraded`: errors detected, partial capability
- `recovering`: reconciling state or restarting sessions

### Transitions

- `idle -> active`: role is invoked or scheduled
- `active -> degraded`: failed health check or missing resources
- `degraded -> recovering`: reconciliation begins
- `recovering -> active`: state is consistent again

### Persistence in Grite

- Role state transitions are recorded in a dedicated issue or log thread
- Health summaries are posted at bounded intervals
