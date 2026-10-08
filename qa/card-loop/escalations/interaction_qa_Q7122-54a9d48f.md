---
item: interaction:qa:Q7122
run_id: pilot-data-squad
escalated_at: 2026-10-08T01:13:31.154010Z
state_before: AUTHORING
reason: codex found no legal line for qa:Q7122: Blocked: exam_authoring_guide, exam_validate, and exam_probe were rejected with 'MCP tool call requires approval, but approval policy is never'. No scenario was delivered or oracle run. Legal reachability remains undetermined; this is not an unreachable finding.
---

# Escalated: `interaction:qa:Q7122`

**Why:** codex found no legal line for qa:Q7122: Blocked: exam_authoring_guide, exam_validate, and exam_probe were rejected with 'MCP tool call requires approval, but approval policy is never'. No scenario was delivered or oracle run. Legal reachability remains undetermined; this is not an unreachable finding.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. codex: `no_legal_line` (attempt `20261008T011149906271Z-pilot-data-squad-3da5c0`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> Blocked: exam_authoring_guide, exam_validate, and exam_probe were rejected with 'MCP tool call requires approval, but approval policy is never'. No scenario was delivered or oracle run. Legal reachability remains undetermined; this is not an unreachable finding.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261008T011024021035Z-pilot-data-squad-f7ad7f` | classify_qa | claude | sonnet | accepted | 0.3100 |
| 2 | `20261008T011024021271Z-pilot-data-squad-2f5942` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261008T011149906271Z-pilot-data-squad-3da5c0` | author_interaction | codex | (default) | escalated | unpriced |

## Item data

```json
{
  "card_ids": [
    "BT26-082"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT26-082#effect#1"
    ],
    "q_id": "Q7122"
  },
  "history": [
    "20261008T011024021035Z-pilot-data-squad-f7ad7f",
    "20261008T011024021271Z-pilot-data-squad-2f5942"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
