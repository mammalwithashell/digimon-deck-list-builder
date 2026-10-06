---
item: clause:BT26-049#inherited#0
run_id: pilot-data-squad
escalated_at: 2026-10-06T15:40:39.459965Z
state_before: AUTHORING
reason: claude found no legal line for BT26-049#inherited#0: No scenario written. Rosemon (BT26-049) has no inherited effect: the printed text is the placeholder "ー" (data/card_bundles/BT26-049.md, "Inherited Effect"). The DCGO script BT26_049.cs defines nothing for inherited timing. It has only the two alternate digivolution requirements, the shared [When Digivolving]/[When Attacking] suspend, and the [All Turns] reactive play. With no effect behind it, no legal line can make either engine resolve BT26-049#inherited#0. Any scenario would fire nothing and cover nothing. I did not run exam_validate or exam_probe. This clause is best classed `unreachable` (or dropped from the denominator as a "ー" placeholder), not `unmeasured`.
---

# Escalated: `clause:BT26-049#inherited#0`

**Why:** claude found no legal line for BT26-049#inherited#0: No scenario written. Rosemon (BT26-049) has no inherited effect: the printed text is the placeholder "ー" (data/card_bundles/BT26-049.md, "Inherited Effect"). The DCGO script BT26_049.cs defines nothing for inherited timing. It has only the two alternate digivolution requirements, the shared [When Digivolving]/[When Attacking] suspend, and the [All Turns] reactive play. With no effect behind it, no legal line can make either engine resolve BT26-049#inherited#0. Any scenario would fire nothing and cover nothing. I did not run exam_validate or exam_probe. This clause is best classed `unreachable` (or dropped from the denominator as a "ー" placeholder), not `unmeasured`.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `no_legal_line` (attempt `20261006T154024241496Z-pilot-data-squad-7b4873`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No scenario written. Rosemon (BT26-049) has no inherited effect: the printed text is the placeholder "ー" (data/card_bundles/BT26-049.md, "Inherited Effect"). The DCGO script BT26_049.cs defines nothing for inherited timing. It has only the two alternate digivolution requirements, the shared [When Digivolving]/[When Attacking] suspend, and the [All Turns] reactive play. With no effect behind it, no legal line can make either engine resolve BT26-049#inherited#0. Any scenario would fire nothing and cover nothing. I did not run exam_validate or exam_probe. This clause is best classed `unreachable` (or dropped from the denominator as a "ー" placeholder), not `unmeasured`.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T154024241496Z-pilot-data-squad-7b4873` | author_clause | claude | sonnet | escalated | 0.2911 |

## Item data

```json
{
  "card_ids": [
    "BT26-049"
  ]
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
