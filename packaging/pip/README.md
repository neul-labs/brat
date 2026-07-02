# Brat

**Multi-agent harness for AI coding tools — crash-safe state, parallel execution, one CLI.**

[![PyPI version](https://img.shields.io/pypi/v/brat-cli.svg)](https://pypi.org/project/brat-cli/)
[![Documentation](https://img.shields.io/badge/docs-neullabs.com-blue)](https://docs.neullabs.com/brat)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/neul-labs/brat)

Brat coordinates AI coding tools (Claude Code, Aider, Codex, and more) working in parallel on your codebase. Built on an append-only event log, Brat ensures that even if agents crash, your coordination state is always recoverable and auditable.

- **Site:** https://brat.neullabs.com
- **Docs:** https://docs.neullabs.com/brat
- **Repo:** https://github.com/neul-labs/brat

## Install

```bash
pip install brat-cli
```

This installs the `brat` command-line tool (a native binary is fetched on install).

## Quick start

```bash
cd your-project
brat init                 # Initialize the Brat harness
brat mayor start          # Start the AI orchestrator
brat mayor ask "Analyze src/ and create tasks for any bugs you find"
brat status               # View convoys, tasks, and sessions
```

See the [full documentation](https://docs.neullabs.com/brat) for the complete CLI reference, workflows, and architecture.

## Part of the Neul Labs toolchain

Brat is part of the Neul Labs orchestration toolchain:

| Project | Description |
|---------|-------------|
| [ringlet](https://github.com/neul-labs/ringlet) | One CLI to rule all your coding agents. |
| [fastworker](https://github.com/neul-labs/fastworker) | Background tasks in Python with zero infrastructure — no Redis, no RabbitMQ. |
| [m9m](https://github.com/neul-labs/m9m) | The n8n alternative without the bugs — one Go binary. |
| [conductor](https://github.com/neul-labs/conductor) | Multi-agent CLI orchestrator for AI coding agents. |

Learn more at [neullabs.com](https://www.neullabs.com).
