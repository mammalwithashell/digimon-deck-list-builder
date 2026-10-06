---
item: interaction:qa:Q2305
run_id: pilot-data-squad
escalated_at: 2026-10-06T18:18:02.727110Z
state_before: AUTHORING
reason: codex found no legal line for qa:Q2305: No current legal exam line reaches the Burst return choice. A temporary draft passed exam_validate, including the interaction denominator check. exam_probe(sim_only=true, inspect_step=11) with both players' Yoshinos present showed ordinary digivolution costing 5 memory (3 to -2), followed by OppField 'Suspend opponent Tamer'; no Burst return-cost prompt occurred. The exam vocabulary exposes no Burst declaration. No oracle ran; no repository files were left behind.
---

# Escalated: `interaction:qa:Q2305`

**Why:** codex found no legal line for qa:Q2305: No current legal exam line reaches the Burst return choice. A temporary draft passed exam_validate, including the interaction denominator check. exam_probe(sim_only=true, inspect_step=11) with both players' Yoshinos present showed ordinary digivolution costing 5 memory (3 to -2), followed by OppField 'Suspend opponent Tamer'; no Burst return-cost prompt occurred. The exam vocabulary exposes no Burst declaration. No oracle ran; no repository files were left behind.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. codex: `no_legal_line` (attempt `20261006T181339426659Z-pilot-data-squad-0ac085`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No current legal exam line reaches the Burst return choice. A temporary draft passed exam_validate, including the interaction denominator check. exam_probe(sim_only=true, inspect_step=11) with both players' Yoshinos present showed ordinary digivolution costing 5 memory (3 to -2), followed by OppField 'Suspend opponent Tamer'; no Burst return-cost prompt occurred. The exam vocabulary exposes no Burst declaration. No oracle ran; no repository files were left behind.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T142518308590Z-pilot-data-squad-ee5c47` | classify_qa | claude | sonnet | accepted | 0.2389 |
| 2 | `20261006T142518308653Z-pilot-data-squad-9cbd1b` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T142609475147Z-pilot-data-squad-e190a5` | encode_ruling | claude | sonnet | escalated | 0.3022 |
| 4 | `20261006T142656399478Z-pilot-data-squad-f1cc3d` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T181225778510Z-pilot-data-squad-63065a` | classify_qa | claude | sonnet | accepted | 0.2424 |
| 6 | `20261006T181225779007Z-pilot-data-squad-9d59ab` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T181339426659Z-pilot-data-squad-0ac085` | author_interaction | codex | (default) | escalated | unpriced |

## Item data

```json
{
  "card_ids": [
    "BT13-060"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT13-060#effect#0"
    ],
    "q_id": "Q2305"
  },
  "escalation": "interaction_qa_Q2305-700af984.md",
  "escalation_reason": "verifier disagrees: No, you can't. You can only return your own Tamer to the hand for Burst Digivolve.",
  "history": [
    "20261006T142518308590Z-pilot-data-squad-ee5c47",
    "20261006T142518308653Z-pilot-data-squad-9cbd1b",
    "20261006T142609475147Z-pilot-data-squad-e190a5",
    "20261006T142656399478Z-pilot-data-squad-f1cc3d",
    "20261006T181225778510Z-pilot-data-squad-63065a",
    "20261006T181225779007Z-pilot-data-squad-9d59ab"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
