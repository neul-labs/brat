---
name: brat-architecture
description: Architecture phase skill. Input: product requirements + current arch KB. Output: architecture design as zkb notes.
version: 1.0.0
tags: [brat, architecture, design]
---

# brat-architecture Skill

## Purpose
Read product requirements and produce architecture design decisions, component breakdown, and interface definitions.

## Input Schema (JSON)
```json
{
  "product_requirements": {
    "title": "string",
    "user_stories": [{"story": "string", "acceptance_criteria": ["string"]}],
    "acceptance_tests": ["string"]
  },
  "architecture_context": "string (relevant arch notes from KB)",
  "existing_components": ["string (known components from KB)"],
  "tech_stack": "string (language/framework info)"
}
```

## Output Schema (JSON)
```json
{
  "action": "create_design|update_design|answer",
  "design": {
    "title": "string",
    "components": [
      {
        "name": "string",
        "responsibility": "string",
        "interfaces": ["string (input/output contracts)"],
        "dependencies": ["string (other components)"],
        "file_paths": ["string (suggested files/modules)"]
      }
    ],
    "adr": {
      "title": "string",
      "context": "string",
      "decision": "string",
      "consequences": ["string"]
    },
    "test_strategy": {
      "unit_tests": ["string (what to test per component)"],
      "integration_tests": ["string (cross-component tests)"]
    },
    "notes_to_create": [
      {
        "title": "string",
        "body": "string",
        "tags": ["architecture", "adr", "component"],
        "type": "permanent|structure",
        "links_to": ["string (existing note slugs)"]
      }
    ]
  },
  "answer": "string (if action=answer)",
  "escalation_reason": "string (if requirements are unclear)"
}
```

## Guardrails
- ALWAYS read product requirements before designing
- Components must have clear single responsibilities
- Every component interface needs input/output contracts
- Create ADR (Architecture Decision Record) for significant choices
- Link architecture notes to product notes they implement
- Define test strategy BEFORE implementation (TDD)
- NEVER write implementation code in this phase
- NEVER proceed to implementation from this skill without explicit test strategy
