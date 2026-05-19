# Roles

Brat preserves the Gastown role topology with additions for the software factory. Roles are behavioral conventions that emit explicit actions via Grite issues, comments, labels, and locks.

## Meta Agent (control plane)

- Replaces the former "Mayor" role
- Creates convoys and tasks as Grite issues
- Assigns tasks and updates labels/state
- Monitors status via Grite queries
- **Auto-bootstraps** existing repositories by scanning code and generating KB notes
- Runs **consistency checks** and surfaces inconsistencies to humans
- Never manages processes directly beyond user UX

## Witness (worker controller)

- Turns intent into worker sessions:
  - decides how many polecats
  - spawns via the engine adapter
  - monitors heartbeats and progress
  - posts lifecycle updates as Grite comments or labels
- If `bratd` is absent: `brat witness run` can execute the controller once
- During **Implementation** phase, spawns parallel **swimlane** teams

## Refinery (integration controller)

- Consumes completed task outputs and manages the merge queue
- Owns merge policy (parallelism, rebase strategy, required checks)
- Posts merge results as Grite updates (labels, comments, links)
- **Enforces human approval gates** before merge
- Tracks review state (approved / rejected / pending)

## Deacon (janitor/reconciler)

- Expires or cleans up stale locks
- Detects orphan sessions (no heartbeat)
- Rebuilds projections if needed
- Syncs refs with remotes
- Emits periodic health summaries

## Bootstrap Agent (auto-discovery)

- Scans codebase on `brat init`
- Infers product notes from entry points, README, API routes
- Infers architecture notes from module structure, crate graph
- Runs consistency check and iterates until converged
- Surfaces unresolvable inconsistencies to humans

## Session types (non-roles)

Session types describe how a running process is managed. They are not roles.

- Polecat: ephemeral worker session managed by Witness
- Crew: user-owned persistent session with user-controlled lifecycle
