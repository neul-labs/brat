---
name: brat-implementation
description: Implementation phase skill. Input: architecture design. Output: working code with tests, written in swimlanes.
version: 1.0.0
tags: [brat, implementation, tdd, swimlanes]
---

# brat-implementation Skill

## Purpose
Read architecture design and implement components in parallel using TDD. Write tests first, then implementation.

## Input Schema (JSON)
```json
{
  "architecture": {
    "components": [
      {
        "name": "string",
        "responsibility": "string",
        "interfaces": ["string"],
        "dependencies": ["string"],
        "file_paths": ["string"]
      }
    ],
    "test_strategy": {
      "unit_tests": ["string"],
      "integration_tests": ["string"]
    }
  },
  "swimlane": {
    "lane_id": "string",
    "engine": "claude|codex|aider",
    "worktree": "string"
  },
  "assigned_component": {
    "name": "string",
    "file_paths": ["string"]
  }
}
```

## Output Schema (JSON)
```json
{
  "implemented_files": [
    {
      "path": "string",
      "description": "string",
      "tests_passing": "boolean"
    }
  ],
  "tests_written": [
    {
      "path": "string",
      "test_type": "unit|integration",
      "status": "passing|failing"
    }
  ],
  "checkpoints": [
    {
      "timestamp": "string (ISO 8601)",
      "milestone": "string",
      "tests_run": "string (command + result)"
    }
  ],
  "memory": "string (findings to store in KB)",
  "state": "completed|blocked|failed"
}
```

## Guardrails
- ALWAYS write tests BEFORE implementation code
- Run tests after every milestone
- Post checkpoint comments with test results
- Acquire `path:` locks before editing files
- If blocked, post blocker comment and set state=blocked
- On completion, write memory note with findings
- NEVER modify architecture notes during implementation
- NEVER skip tests
