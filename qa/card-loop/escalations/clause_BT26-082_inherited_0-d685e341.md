---
item: clause:BT26-082#inherited#0
run_id: pilot-data-squad
escalated_at: 2026-10-07T01:54:36.933875Z
state_before: AUTHORING
reason: claude found no legal line for BT26-082#inherited#0: No scenario written. Ravemon (BT26-082) prints "ー" as its inherited effect, so the clause has no effect to fire. DCGO's BT26_082.cs agrees: it has no inherited-effect code, only the alternate digivolution requirements, the Security effect, the shared When Digivolving / End of Attack effect, and On Deletion. No line can make either engine resolve a clause with no text. This clause should be classed as unreachable or unavailable (no printed effect) rather than given a scenario.
---

# Escalated: `clause:BT26-082#inherited#0`

**Why:** claude found no legal line for BT26-082#inherited#0: No scenario written. Ravemon (BT26-082) prints "ー" as its inherited effect, so the clause has no effect to fire. DCGO's BT26_082.cs agrees: it has no inherited-effect code, only the alternate digivolution requirements, the Security effect, the shared When Digivolving / End of Attack effect, and On Deletion. No line can make either engine resolve a clause with no text. This clause should be classed as unreachable or unavailable (no printed effect) rather than given a scenario.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `no_legal_line` (attempt `20261007T015419786132Z-pilot-data-squad-f85fb3`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No scenario written. Ravemon (BT26-082) prints "ー" as its inherited effect, so the clause has no effect to fire. DCGO's BT26_082.cs agrees: it has no inherited-effect code, only the alternate digivolution requirements, the Security effect, the shared When Digivolving / End of Attack effect, and On Deletion. No line can make either engine resolve a clause with no text. This clause should be classed as unreachable or unavailable (no printed effect) rather than given a scenario.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261007T015419786132Z-pilot-data-squad-f85fb3` | author_clause | claude | sonnet | escalated | 0.2864 |

## Item data

```json
{
  "card_ids": [
    "BT26-082"
  ]
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
