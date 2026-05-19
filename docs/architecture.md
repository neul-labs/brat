# Architecture

## Overview

Brat is a multi-agent software factory backed by Grite for task and memory storage, and zkb for knowledge base notes. The harness provides roles, swarming, orchestration UX, auto-bootstrap, and phased pipeline execution.

## Layers

1. **Harness layer (Brat)**
   - Roles: Meta Agent, Witness, Refinery, Deacon
   - Phased pipeline: Product → Architecture → Implementation → Review → Merge
   - Vertical swimlanes for parallel agent teams
   - Human-in-the-loop approval gates
   - Swarm orchestration and control room UX
   - Uses Grite issues, comments, labels, and locks for coordination

2. **Knowledge Base (zkb)**
   - Product notes: requirements, user stories, acceptance criteria
   - Architecture notes: components, ADRs, interfaces, design decisions
   - Memory notes: agent discoveries, conventions, lessons learned
   - Consistency checking across product/architecture dimensions
   - Full-text search via zkb-lib

3. **Grite substrate (source of truth)**
   - Append-only events in `refs/grite/wal`
   - Local materialized view in `.git/grite/actors/<actor_id>/sled/`
   - Deterministic projections from the WAL, values encoded with `rkyv`

4. **Grite daemon (optional, performance only)**
   - Background fetch/push
   - Warm cache and pub/sub notifications

Correctness never depends on `grited`; the CLI can always rebuild state from the WAL. `bratd` runs by default for UX but is not required for correctness.

## Components

- `libbrat-kb`: zkb integration (product, architecture, memory, consistency)
- `libbrat-engine`: codebase scanning, inference, bootstrap, consistency checking
- `libbrat-swimlane`: swimlane scheduler for parallel agent teams
- `libbrat-lib`: unified facade over all libbrat-* crates
- `brat-skills`: embedded Claude skills (bootstrap, product, architecture, implementation, review, memory)
- `brat-mcp`: MCP server exposing brat operations as tools
- `libgrite-core`: event types, hashing, projections, sled store
- `libgrite-git`: WAL commit read/write, snapshot handling, ref sync
- `libgrite-ipc`: shared IPC message schema (rkyv)
- `grite`: CLI frontend
- `grited`: daemon (optional)
- `brat`: harness CLI (roles, swarm, control room, bootstrap)
- `bratd`: harness daemon (role supervisor + tmux control room)
- `brat-ui`: web UI for bootstrap, consistency, pipeline, KB, review

## Phased Pipeline

All changes flow through a consistency-gated pipeline:

```
Product ──▶ Architecture ──▶ Implementation ──▶ Review ──▶ Merge
   │             │                │                 │         │
   ▼             ▼                ▼                 ▼         ▼
  KB            KB            Swimlanes          Human     KB/mem
 (auto)        (auto)        (parallel teams)  (approval) (auto)
```

Phase transitions are blocked by consistency gates:
- Product → Architecture: product_arch_coverage must be ≥ threshold
- Architecture → Implementation: arch_product_traceability must be ≥ threshold
- Implementation → Review: test_feature_coverage and doc_component_parity must be ≥ threshold
- Review → Merge: human approval required

## Auto-Bootstrap

On `brat init` for an existing git repository:

1. Meta Agent scans the codebase (files, README, tests, entry points)
2. Generates product notes (features, user stories, constraints)
3. Generates architecture notes (components, interfaces, ADRs)
4. Runs consistency check (score 0-100)
5. Auto-fixes some inconsistencies (missing architecture, orphan components)
6. Iterates up to 5 times, then surfaces remaining to human

## Data flow

1. The harness creates or updates Grite issues and comments.
2. Events are appended to the WAL ref as a new git commit.
3. The local materialized view is updated from new WAL events.
4. Meta Agent reads/writes KB notes via zkb-lib.
5. Consistency checker compares product and architecture notes.
6. `grite sync` pushes/pulls refs; the harness observes updates via the view.

## Storage footprint

Local state is scoped per actor. Each agent gets its own data directory to avoid multi-process writes to the same DB.

- `.git/grite/actors/<actor_id>/sled/`: local DB (per actor)
- `.git/grite/actors/<actor_id>/config.toml`: local config and actor identity
- `.git/grite/config.toml`: repo-level defaults (for example, default actor)
- `.brat/kb/`: knowledge base mirror (product.md, architecture.md)
- `.grite/`: optional export output (gitignored)
- `refs/grite/wal`: append-only event log
- `refs/grite/snapshots/*`: optional, monotonic snapshots
- `refs/grite/locks/*`: optional lease locks
