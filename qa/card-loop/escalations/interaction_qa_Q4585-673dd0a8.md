---
item: interaction:qa:Q4585
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T14:29:21.939057Z
state_before: ENCODE
reason: verifier disagrees: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.
---

# Escalated: `interaction:qa:Q4585`

**Why:** verifier disagrees: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `encoded` (attempt `20261006T142835073217Z-pilot-three-musketeers-51a1a5`)

- Citation: qa:Q4585
- Reasoning:
> The answer decides one thing: once the host (BT21-074 digivolved into EX7-048 Gundramon) no longer meets the link requirements, the linked BT21-054 Shotmon goes to the trash at rule check. The only observable is that BT21-054 ends up in the owner's (p0) trash. The answer says nothing about memory, hand, security, the opponent's side, or the digivolved Digimon's field entry, so I assert nothing on them. The `at` index of 17 is a placeholder. The supplied scenario (BT21-074-inherited0.yaml) only links from hand and never digivolves, so it has no step for this. The scenario author must set `at` to the step right after the digivolve into EX7-048 resolves and rule check runs. The trash list assumes nothing else has been trashed by that step, so the author should check that when placing the assertion.

### 2. codex: `does_not_encode` (attempt `20261006T142902985971Z-pilot-three-musketeers-c43bcf`)

- Citation: qa:Q4585
- Reasoning:
> The ruling concerns Shotmon becoming an invalid link when Satellamon digivolves into Gundramon. The scenario never establishes that situation: at step 17, Satellamon has linked to Scopemon and deleted the opponent's Monodramon. No Shotmon is linked and no Gundramon digivolution occurs, so asserting BT21-054 in p0.trash does not encode the ruling against this scenario. | quote: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T142753733441Z-pilot-three-musketeers-04e455` | classify_qa | claude | sonnet | accepted | 0.2472 |
| 2 | `20261006T142753733493Z-pilot-three-musketeers-905832` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T142835073217Z-pilot-three-musketeers-51a1a5` | encode_ruling | claude | sonnet | escalated | 0.2679 |
| 4 | `20261006T142902985971Z-pilot-three-musketeers-c43bcf` | encode_ruling | codex | (default) | escalated | unpriced |

## Item data

```json
{
  "card_ids": [
    "BT21-074"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT21-074#inherited#0"
    ],
    "q_id": "Q4585"
  },
  "history": [
    "20261006T142753733441Z-pilot-three-musketeers-04e455",
    "20261006T142753733493Z-pilot-three-musketeers-905832"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
