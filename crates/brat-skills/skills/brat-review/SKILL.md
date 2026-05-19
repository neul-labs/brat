---
name: brat-review
description: Review phase skill. Input: completed implementation artifact. Output: merge decision or human approval request.
version: 1.0.0
tags: [brat, review, merge, approval]
---

# brat-review Skill

## Purpose
Review completed implementation against architecture design and product requirements. Request human approval before merge.

## Input Schema (JSON)
```json
{
  "implementation": {
    "component": "string",
    "files_changed": ["string"],
    "diff_summary": "string",
    "tests": {
      "unit": {"passing": "number", "failing": "number"},
      "integration": {"passing": "number", "failing": "number"}
    }
  },
  "architecture": {
    "component": "string",
    "expected_interfaces": ["string"],
    "expected_file_paths": ["string"]
  },
  "product": {
    "requirement_title": "string",
    "acceptance_criteria": ["string"]
  },
  "policy": {
    "require_human_approval": "boolean",
    "required_checks": ["string"]
  }
}
```

## Output Schema (JSON)
```json
{
  "decision": "approve|request_changes|reject|escalate",
  "assessment": {
    "architecture_compliance": "compliant|partial|non_compliant",
    "test_coverage": "adequate|partial|inadequate",
    "acceptance_criteria_met": "all|partial|none"
  },
  "approval_request": {
    "rationale": "string (why this needs human eyes)",
    "artifact_summary": "string (what changed)",
    "risk_level": "low|medium|high",
    "suggested_reviewers": ["string"]
  },
  "required_changes": ["string (if decision=request_changes)"],
  "reason": "string"
}
```

## Guardrails
- If `require_human_approval` is true, ALWAYS request approval before merge
- Check architecture compliance before approval
- Check all acceptance criteria are met
- Tests must be passing (zero failures)
- On rejection, provide specific required changes
- Post review assessment as structured comment
- NEVER merge without approval when policy requires it
