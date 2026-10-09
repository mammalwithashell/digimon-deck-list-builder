---
item: interaction:qa:Q2670
run_id: pilot-data-squad
escalated_at: 2026-10-06T16:09:17.402795Z
state_before: AUTHORING
reason: claude found no legal line for qa:Q2670: No scenario written. I found no legal line that reaches Q2670. This rests on searches of data/cards.json text only; I did not run exam_validate or exam_probe on any candidate line. The ruling needs a DP reduction already in force on P0's side at the moment Ukkomon moves. The move can only happen in P0's breeding phase, which is the start of P0's turn. Only P1 could have set that reduction up, and it would have to hit every Digimon, including one that arrives later. - **Static or [Opponent's Turn] reduction:** nothing in the pool lowers all of the opponent's Digimon DP as a standing effect. The only [Opponent's Turn] DP effects are single-target "for the turn" effects or protections. - **"For the turn" effects:** every "all of your opponent's Digimon -N DP for the turn" card expires at the end of P1's own turn (AD1-017, BT23-035, BT25-018, BT25-043, EX5-074, EX10-041, EX12-076, BT19-014). That is before P0's breeding phase. - **"Until their turn ends" effects:** these last through P0's turn, but every one is single-target (AD1-016, BT22-042, BT23-033, EX9-030, EX12-018, EX12-048, BT26-081, EX13-020, BT13-106). Breeding-area Digimon are not valid targets for them. - **Option effects:** BT11-101 and EX5-048 are "until the end of your opponent's turn" but pick 1-3 existing Digimon, so the incoming Ukkomon is not covered. - **P0's own side:** no card lowers all of P0's own Digimon DP in a way that could be set up before the move. If a card with such a standing effect is added later, build the line from qa/dcgo-exams/BT16/BT16-082-effect0.yaml and have P1 put that card in play on T4. The moved Ukkomon must have 2000 DP or less so it is deleted on arrival.
---

# Escalated: `interaction:qa:Q2670`

**Why:** claude found no legal line for qa:Q2670: No scenario written. I found no legal line that reaches Q2670. This rests on searches of data/cards.json text only; I did not run exam_validate or exam_probe on any candidate line.

The ruling needs a DP reduction already in force on P0's side at the moment Ukkomon moves. The move can only happen in P0's breeding phase, which is the start of P0's turn. Only P1 could have set that reduction up, and it would have to hit every Digimon, including one that arrives later.

- **Static or [Opponent's Turn] reduction:** nothing in the pool lowers all of the opponent's Digimon DP as a standing effect. The only [Opponent's Turn] DP effects are single-target "for the turn" effects or protections.
- **"For the turn" effects:** every "all of your opponent's Digimon -N DP for the turn" card expires at the end of P1's own turn (AD1-017, BT23-035, BT25-018, BT25-043, EX5-074, EX10-041, EX12-076, BT19-014). That is before P0's breeding phase.
- **"Until their turn ends" effects:** these last through P0's turn, but every one is single-target (AD1-016, BT22-042, BT23-033, EX9-030, EX12-018, EX12-048, BT26-081, EX13-020, BT13-106). Breeding-area Digimon are not valid targets for them.
- **Option effects:** BT11-101 and EX5-048 are "until the end of your opponent's turn" but pick 1-3 existing Digimon, so the incoming Ukkomon is not covered.
- **P0's own side:** no card lowers all of P0's own Digimon DP in a way that could be set up before the move.

If a card with such a standing effect is added later, build the line from qa/dcgo-exams/BT16/BT16-082-effect0.yaml and have P1 put that card in play on T4. The moved Ukkomon must have 2000 DP or less so it is deleted on arrival.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `no_legal_line` (attempt `20261006T160718659751Z-pilot-data-squad-87f2db`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No scenario written. I found no legal line that reaches Q2670. This rests on searches of data/cards.json text only; I did not run exam_validate or exam_probe on any candidate line.
>
> The ruling needs a DP reduction already in force on P0's side at the moment Ukkomon moves. The move can only happen in P0's breeding phase, which is the start of P0's turn. Only P1 could have set that reduction up, and it would have to hit every Digimon, including one that arrives later.
>
> - **Static or [Opponent's Turn] reduction:** nothing in the pool lowers all of the opponent's Digimon DP as a standing effect. The only [Opponent's Turn] DP effects are single-target "for the turn" effects or protections.
> - **"For the turn" effects:** every "all of your opponent's Digimon -N DP for the turn" card expires at the end of P1's own turn (AD1-017, BT23-035, BT25-018, BT25-043, EX5-074, EX10-041, EX12-076, BT19-014). That is before P0's breeding phase.
> - **"Until their turn ends" effects:** these last through P0's turn, but every one is single-target (AD1-016, BT22-042, BT23-033, EX9-030, EX12-018, EX12-048, BT26-081, EX13-020, BT13-106). Breeding-area Digimon are not valid targets for them.
> - **Option effects:** BT11-101 and EX5-048 are "until the end of your opponent's turn" but pick 1-3 existing Digimon, so the incoming Ukkomon is not covered.
> - **P0's own side:** no card lowers all of P0's own Digimon DP in a way that could be set up before the move.
>
> If a card with such a standing effect is added later, build the line from qa/dcgo-exams/BT16/BT16-082-effect0.yaml and have P1 put that card in play on T4. The moved Ukkomon must have 2000 DP or less so it is deleted on arrival.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T144103124993Z-pilot-data-squad-d83e8e` | classify_qa | claude | sonnet | accepted | 0.2390 |
| 2 | `20261006T144103125057Z-pilot-data-squad-8366fa` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T144136736840Z-pilot-data-squad-5d1eb0` | encode_ruling | claude | sonnet | escalated | 0.2608 |
| 4 | `20261006T144155190254Z-pilot-data-squad-7a0dfb` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T160630740775Z-pilot-data-squad-18e350` | classify_qa | claude | sonnet | accepted | 0.2388 |
| 6 | `20261006T160630740798Z-pilot-data-squad-61ce78` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T160718659751Z-pilot-data-squad-87f2db` | author_interaction | claude | sonnet | escalated | 0.7360 |

## Item data

```json
{
  "card_ids": [
    "BT16-082"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT16-082#effect#0"
    ],
    "q_id": "Q2670"
  },
  "escalation": "interaction_qa_Q2670-298d399c.md",
  "escalation_reason": "verifier disagrees: Yes, it activates.",
  "history": [
    "20261006T144103124993Z-pilot-data-squad-d83e8e",
    "20261006T144103125057Z-pilot-data-squad-8366fa",
    "20261006T144136736840Z-pilot-data-squad-5d1eb0",
    "20261006T144155190254Z-pilot-data-squad-7a0dfb",
    "20261006T160630740775Z-pilot-data-squad-18e350",
    "20261006T160630740798Z-pilot-data-squad-61ce78"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
