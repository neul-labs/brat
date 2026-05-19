---
id: 01KQPMB8NYFSP78WB9372GN00E
title: Brat
slug: brat
note_type: permanent
tags:
- overview
- product
created_at: 2026-05-03T09:57:37.092439+00:00
updated_at: 2026-05-04T09:26:44.690452+00:00
---

# Brat

**Multi-agent harness for AI coding tools. Crash-safe state, parallel execution, one CLI.**

[![Crates.io](https://img.shields.io/crates/v/brat.svg)](https://crates.io/crates/brat)
[![Documentation](https://img.shields.io/badge/docs-neullabs.com-blue)](https://docs.neullabs.com/brat)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

Brat is a multi-agent harness that coordinates AI coding tools (Claude Code, Aider, Codex, and more) working in parallel on your codebase. Built on [Gritee](https://github.com/neul-labs/grite), an append-only event log, Brat ensures that even if agents crash, your coordination state is always recoverable and auditable.

---

## See It In Action

```bash
# Clone and run the full demo with web UI
git clone https://github.com/neul-labs/brat && cd brat
cargo build --release
./scripts/mayor-demo.sh --with-ui
```

**What the demo shows:**
1. Creates a sample Python project with intentional bugs
2. Starts the **Mayor** (AI orchestrator powered by Claude)
3. Mayor analyzes the codebase and identifies issues by severity
4. Mayor creates a **Convoy** with **Tasks** for bug fixes
5. View everything in the web dashboard at **http://localhost:5173**

---

## Supported AI Coding Engines

Brat works with your preferred AI coding tool:

| Engine | Command | Highlights |
|--------|---------|------------|
| **Claude Code** | `claude` | Native Anthropic integration, session resume |
| **Aider** | `aider` | Multi-model support (GPT-4, Claude, Gemini, local LLMs) |
| **OpenCode** | `opencode` | 75+ LLM providers, open-source Claude Code alternative |
| **Codex** | `codex` | Structured JSON output for parsing |
| **Continue** | `cn` | IDE integration, CI/CD pipelines |
| **Gemini** | `gemini` | Google's free tier |
| **GitHub Copilot** | `gh copilot` | Shell/git command suggestions |

Configure your engine in `.brat/config.toml` and Brat handles the rest.

---

## How It Works

```
┌──────────┐         ┌──────────┐         ┌──────────┐
│  Mayor   │─creates─▶│  Convoy  │─contains─▶│  Tasks   │
│  (AI)    │         │  (group) │         │  (work)  │
└──────────┘         └──────────┘         └────┬─────┘
                                               │
                     ┌─────────────────────────┘
                     ▼
              ┌─────────────┐      ┌─────────────┐
              │   Witness   │─────▶│  Refinery   │
              │(spawn agents)│      │(merge work) │
              └─────────────┘      └─────────────┘
```

| Role | What It Does |
|------|--------------|
| **Mayor** | AI orchestrator that analyzes codebases, breaks down work, and creates convoys/tasks |
| **Convoy** | A group of related tasks (think: sprint, epic, or feature branch) |
| **Task** | Individual work item assigned to an AI coding agent |
| **Witness** | Spawns and monitors coding agent sessions ("polecats") |
| **Refinery** | Manages the merge queue, runs CI checks, handles integration |
| **Deacon** | Background janitor: cleans locks, syncs state, detects orphans |

---

## Web UI Dashboard

Brat includes a real-time web dashboard for monitoring and control:

- **Dashboard** - Task status cards (queued, running, blocked, merged)
- **Convoys** - Create and manage work groups
- **Tasks** - Filter, assign, and track individual work items
- **Sessions** - Monitor active AI agents with live log viewer
- **Mayor Chat** - Interactive interface to communicate with the AI orchestrator

```bash
# Start the UI
./scripts/ui-demo.sh
# Opens http://localhost:5173
```

---

## Quick Start

### 1. Install

```bash
# One-line install
curl -fsSL https://raw.githubusercontent.com/neul-labs/brat/main/install.sh | bash

# Or build from source
cargo install --path crates/brat
```

**Prerequisite:** Install [Gritee](https://github.com/neul-labs/grite) first:
```bash
cargo install --git https://github.com/neul-labs/grite grite
```

### 2. Initialize Your Repo

```bash
cd your-project
grite init     # Initialize Gritee substrate
brat init      # Initialize Brat harness
```

### 3. Start the Mayor

```bash
# Start AI orchestrator
brat mayor start

# Ask it to analyze your code
brat mayor ask "Analyze src/ and create tasks for any bugs you find"

# Check what it created
brat status
```

### 4. Run Agents on Tasks

```bash
# Spawn AI agents for queued tasks
brat witness run --once

# Watch progress
brat status --watch
```

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `brat init` | Initialize harness in current repo |
| `brat status` | View convoys, tasks, and sessions |
| `brat mayor start` | Start AI orchestrator session |
| `brat mayor ask "..."` | Send prompt to Mayor |
| `brat mayor stop` | Stop Mayor session |
| `brat convoy create` | Create a new convoy |
| `brat convoy list` | List all convoys |
| `brat task add` | Add a task to a convoy |
| `brat task list` | List all tasks |
| `brat witness run` | Spawn agents for queued tasks |
| `brat refinery run` | Process merge queue |
| `brat daemon start` | Start HTTP API daemon in background |
| `brat daemon stop` | Stop the daemon |
| `brat daemon status` | Check if daemon is running |

See [docs/brat-cli.md](docs/brat-cli.md) for the complete reference.

---

## Daemon (bratd)

Brat includes an HTTP API daemon for the web UI and multi-session coordination:

```bash
# Start daemon (auto-starts on most commands anyway)
brat daemon start

# Check status
brat daemon status

# Stop daemon
brat daemon stop
```

**Features:**
- **Auto-start** - Daemon starts automatically when you run commands that need it
- **Idle shutdown** - Shuts down after 15 minutes of inactivity (configurable)
- **Multi-repo** - Single daemon manages multiple repositories
- **Standalone binary** - Also available as `bratd` for direct invocation

```bash
# Disable auto-start for scripting
brat --no-daemon status

# Custom port and longer timeout
brat daemon start --port 8080 --idle-timeout 3600

# Run standalone daemon
bratd --port 3000 --idle-timeout 900
```

See [docs/bratd.md](docs/bratd.md) for full documentation.

---

## Why Brat?

### Problems It Solves

| Problem | How Brat Fixes It |
|---------|-------------------|
| **Dirty working trees** | Metadata lives in `refs/grite/*`, never in tracked files |
| **Silent failures** | All state changes recorded as Grite events, fully observable |
| **Crash recovery** | Append-only log enables deterministic rebuild from any point |
| **Daemon dependency** | CLI commands are complete transactions; daemons are optional |
| **Merge chaos** | Refinery manages queue with configurable policy (rebase/squash/merge) |

### What It Doesn't Solve

We believe in honest positioning:

- **Engine reliability** - API rate limits, auth issues, and vendor outages are outside Brat's control
- **Merge conflicts** - Real code conflicts still need human judgment
- **Prompt quality** - Brat orchestrates agents; prompt engineering is your job
- **CI/CD setup** - Brat integrates with your existing CI, doesn't replace it

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       Your Repository                        │
├──────────────────────────┬──────────────────────────────────┤
│  .brat/                  │  refs/grite/wal                  │
│  ├─ config.toml          │  └─ append-only event log        │
│  └─ workflows/           │                                  │
│     ├─ feature.yaml      │  .git/grite/actors/<id>/sled/    │
│     ├─ fix-bug.yaml      │  └─ local materialized view      │
│     └─ code-review.yaml  │                                  │
├──────────────────────────┴──────────────────────────────────┤
│                    Brat Harness Layer                        │
│  ┌────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐            │
│  │ Mayor  │ │ Witness │ │ Refinery │ │ Deacon │            │
│  └────────┘ └─────────┘ └──────────┘ └────────┘            │
├─────────────────────────────────────────────────────────────┤
│                  Gritee Substrate Layer                       │
│  Events • Issues • Labels • Comments • Locks • Sync         │
├─────────────────────────────────────────────────────────────┤
│                    AI Engine Adapters                        │
│  Claude │ Aider │ OpenCode │ Codex │ Continue │ Gemini      │
└─────────────────────────────────────────────────────────────┘
```

**Key design principles:**
- **Append-only correctness** - WAL is immutable; state rebuilds from events
- **Actor isolation** - Each agent gets its own data directory
- **Bounded timeouts** - All engine operations have configurable timeouts
- **Lock discipline** - Resource coordination with TTL-based leases

---

## Workflow Templates

Define reusable workflows in `.brat/workflows/`:

**Sequential workflow** (`feature.yaml`):
```yaml
name: feature
type: workflow
steps:
  - id: design
    title: "Design {{feature}}"
  - id: implement
    needs: [design]
    title: "Implement {{feature}}"
  - id: test
    needs: [implement]
    title: "Test {{feature}}"
```

**Parallel convoy** (`code-review.yaml`):
```yaml
name: code-review
type: convoy
legs:
  - id: correctness
    title: "Review correctness"
  - id: security
    title: "Review security"
  - id: performance
    title: "Review performance"
synthesis:
  title: "Synthesize review findings"
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System design and data flow |
| [CLI Reference](docs/brat-cli.md) | Complete command documentation |
| [Daemon (bratd)](docs/bratd.md) | HTTP API daemon and auto-start |
| [State Machine](docs/state-machine.md) | Session lifecycle and transitions |
| [Roles](docs/roles.md) | Mayor, Witness, Refinery, Deacon |
| [Engine Integration](docs/engine.md) | Adding new AI engines |
| [Workflows](docs/convoy-task-schema.md) | Convoy and task schemas |
| [Roadmap](docs/roadmap.md) | Current status and planned features |

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Development setup
git clone https://github.com/neul-labs/brat
cd brat
cargo build
cargo test
```

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <b>Brat</b> is built on <a href="https://github.com/neul-labs/grite">Gritee</a> - the append-only substrate for deterministic collaboration.
</p>


## Acceptance Criteria


P0
P0

## Hand-edited section
This was added by hand.
HANDEDIT-TEST-12345
