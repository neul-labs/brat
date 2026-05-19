---
name: brat-memory
description: Memory phase skill. Input: session findings. Output: structured memory notes in KB.
version: 1.0.0
tags: [brat, memory, knowledge-base]
---

# brat-memory Skill

## Purpose
After task completion or failure, write structured memory notes to the zkb knowledge base for future agents.

## Input Schema (JSON)
```json
{
  "session": {
    "task_id": "string",
    "component": "string",
    "engine": "string",
    "duration_ms": "number"
  },
  "findings": {
    "discoveries": ["string (new patterns, conventions)"],
    "conventions": ["string (coding conventions observed)"],
    "pitfalls": ["string (things to avoid)"],
    "useful_commands": ["string (commands that worked)"]
  },
  "outcome": {
    "status": "success|failure|partial",
    "reason": "string"
  }
}
```

## Output Schema (JSON)
```json
{
  "notes_created": [
    {
      "title": "string",
      "slug": "string",
      "type": "fleeting|permanent",
      "tags": ["string"]
    }
  ],
  "links_created": [
    {
      "from": "string",
      "to": "string",
      "type": "reference|continuation"
    }
  ]
}
```

## Guardrails
- ALWAYS create at least one memory note per session
- Tag notes with `memory`, `component:<name>`, `task:<id>`
- Link memory notes to related architecture and product notes
- Promote `fleeting` to `permanent` when a pattern is validated
- On failure, write note about what went wrong for future avoidance
