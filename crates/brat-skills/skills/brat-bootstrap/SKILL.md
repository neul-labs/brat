---
name: brat-bootstrap
description: Auto-bootstrap skill. Input: codebase scan. Output: product/architecture notes + consistency score + inconsistencies.
version: 1.0.0
tags: [brat, bootstrap, scan, inference]
---

# brat-bootstrap Skill

## Purpose
Analyze user intent and produce structured product requirements stored in the zkb product knowledge base.

## Input Schema (JSON)
```json
{
  "codebase_scan": {
    "files": [{"path": "string", "language": "string", "exports": ["string"]}],
    "readme": "string",
    "docs": ["string"],
    "entry_points": ["string"],
    "dependencies": ["string"],
    "test_files": ["string"]
  },
  "existing_kb": {
    "product_notes": [{"title": "string", "body": "string"}],
    "arch_notes": [{"title": "string", "body": "string"}]
  }
}
```

## Output Schema (JSON)
```json
{
  "product_notes": [{"title": "string", "body": "string", "tags": ["string"]}],
  "architecture_notes": [{"title": "string", "body": "string", "tags": ["string"]}],
  "consistency_score": "number (0-100)",
  "inconsistencies": [
    {
      "kind": "MissingArchitecture|OrphanComponent|MissingTests|MissingDocs|Mismatch",
      "severity": "low|medium|high",
      "description": "string",
      "suggested_fix": "string"
    }
  ],
  "notes_to_create": [{"title": "string", "body": "string", "type": "permanent|structure"}]
}
```

## Guardrails
- ALWAYS scan all source files
- NEVER skip README or docs
- Link every product note to at least one architecture note
- Link every architecture note to at least one product note
- Flag inconsistencies, don't hide them
- Iterate up to 5 times, then surface remaining to human
