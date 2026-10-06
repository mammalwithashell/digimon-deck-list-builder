---
item: interaction:qa:Q4578
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T13:52:39.786770Z
state_before: ENCODE
reason: verifier disagrees: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.
---

# Escalated: `interaction:qa:Q4578`

**Why:** verifier disagrees: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `encoded` (attempt `20261006T135132251645Z-pilot-three-musketeers-afa0d2`)

- Citation: qa:Q4578
- Reasoning:
> The answer decides one thing: once the host digivolves into EX7-044 [Gigadramon] and the link requirement fails, the linked BT21-054 [Shotmon] goes to the trash at rule check. So the only observable is p0.trash holding BT21-054. I assert nothing else (memory, hand, field DP and so on), since the answer says nothing on them. Caveat: the given scenario (BT21-071-inherited0.yaml) never digivolves into EX7-044 and uses BT21-071 as the link card, not BT21-054 as the host. It has no step after the digivolve and no BT21-054 in its stacks. So `at: 16` is only a placeholder for the final step. The trash value ["BT21-054"] assumes a scenario where nothing else has been trashed. The block must be re-indexed, and the trash list adjusted for any other trashed cards, once a scenario that performs the digivolve exists.

### 2. codex: `does_not_encode` (attempt `20261006T135219192275Z-pilot-three-musketeers-c9b3b8`)

- Citation: qa:Q4578
- Reasoning:
> The scenario links another Scopemon, never links Shotmon, and never digivolves the host into EX7-044 Gigadramon. At step 16, the trash contains BT24-088 and P-180 from Scopemon's When Linking effect. Requiring exactly ["BT21-054"] neither matches this scenario nor tests the ruling's loss-of-link-requirements condition. | quote: The BT21-054 [Shotmon] linked with this card is trashed upon the rule check timing.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T135031174290Z-pilot-three-musketeers-7c109e` | classify_qa | claude | sonnet | accepted | 0.2444 |
| 2 | `20261006T135031174364Z-pilot-three-musketeers-79b704` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T135132251645Z-pilot-three-musketeers-afa0d2` | encode_ruling | claude | sonnet | escalated | 0.2623 |
| 4 | `20261006T135219192275Z-pilot-three-musketeers-c9b3b8` | encode_ruling | codex | (default) | escalated | unpriced |

## Item data

```json
{
  "card_ids": [
    "BT21-071"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT21-071#inherited#0"
    ],
    "q_id": "Q4578"
  },
  "history": [
    "20261006T135031174290Z-pilot-three-musketeers-7c109e",
    "20261006T135031174364Z-pilot-three-musketeers-79b704"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
