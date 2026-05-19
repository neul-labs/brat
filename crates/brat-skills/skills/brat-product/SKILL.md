---
name: brat-product
description: Product phase skill. Input: user intent + current product KB. Output: structured product requirements as zkb notes.
version: 1.0.0
tags: [brat, product, requirements]
---

# brat-product Skill

## Purpose
Analyze user intent and produce structured product requirements stored in the zkb product knowledge base.

## Input Schema (JSON)
```json
{
  "intent": "string (user's natural language request)",
  "product_context": "string (relevant product notes from KB)",
  "existing_features": ["string (list of known features from KB)"],
  "constraints": "string (business/technical constraints)"
}
```

## Output Schema (JSON)
```json
{
  "action": "create_requirements|update_requirements|answer",
  "requirements": {
    "title": "string",
    "user_stories": [
      {
        "story": "string (As a X, I want Y, so that Z)",
        "acceptance_criteria": ["string"],
        "priority": "P0|P1|P2"
      }
    ],
    "acceptance_tests": ["string (test descriptions)"],
    "notes_to_create": [
      {
        "title": "string",
        "body": "string",
        "tags": ["string"],
        "type": "fleeting|permanent|structure"
      }
    ]
  },
  "answer": "string (if action=answer)",
  "escalation_reason": "string (if unclear)"
}
```

## Guardrails
- ALWAYS query KB for existing product context before writing
- User stories must follow "As a... I want... so that..." format
- Every requirement needs at least one acceptance criterion
- Create `fleeting` notes for raw ideas, `permanent` for validated requirements
- Link new notes to existing product structure note
- NEVER proceed to architecture phase from this skill
