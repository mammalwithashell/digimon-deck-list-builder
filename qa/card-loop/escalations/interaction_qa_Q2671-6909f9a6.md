---
item: interaction:qa:Q2671
run_id: pilot-data-squad
escalated_at: 2026-10-06T16:12:30.490576Z
state_before: AUTHORING
reason: claude found no legal line for qa:Q2671: No scenario written. I found no legal line that reaches Q2671, but I did not run a sim measurement (exam_probe) to confirm it, so treat this as unmeasured rather than proven unreachable. The ruling needs a DP-reducing effect already in force that takes the moving Ukkomon (BT16-082, 2000 DP) to 0 or below the moment it enters the battle area. A move from breeding to battle only happens in the breeding action at the start of a turn, so the reduction has to already be active then. What I searched in data/cards.json: - All "for the turn" mass -DP effects target the opponent's Digimon, and they come from [Main], [On Play] or [Security] effects. None can be in force during the opponent's breeding phase. - The only continuous (static) mass -DP effects I found hit opponent Security Digimon, or the opponent's *suspended* Digimon (BT16-101). A freshly moved Ukkomon is unsuspended, so neither applies. - The remaining hits were single-target -DP triggers. - I did not find an effect that lowers all of one's own Digimon's DP. - Searching code/digimon-engine/cards for a DSL aura with a negative amount returned nothing. My last two searches may have missed variants, and I did not exhaust every phrasing in the pool. Our engine matches the ruling's other half: a Digimon whose DP is 0 is deleted, so the pending effect cannot activate. The existing base line, qa/dcgo-exams/BT16/BT16-082-effect0.yaml, already covers the positive case where Ukkomon survives the move. If someone finds a card with a static or turn-long all-Digimon -DP effect that is active during a breeding phase, extend that base line with the setup and a move step, then stop right after the move.
---

# Escalated: `interaction:qa:Q2671`

**Why:** claude found no legal line for qa:Q2671: No scenario written. I found no legal line that reaches Q2671, but I did not run a sim measurement (exam_probe) to confirm it, so treat this as unmeasured rather than proven unreachable.

The ruling needs a DP-reducing effect already in force that takes the moving Ukkomon (BT16-082, 2000 DP) to 0 or below the moment it enters the battle area. A move from breeding to battle only happens in the breeding action at the start of a turn, so the reduction has to already be active then.

What I searched in data/cards.json:
- All "for the turn" mass -DP effects target the opponent's Digimon, and they come from [Main], [On Play] or [Security] effects. None can be in force during the opponent's breeding phase.
- The only continuous (static) mass -DP effects I found hit opponent Security Digimon, or the opponent's *suspended* Digimon (BT16-101). A freshly moved Ukkomon is unsuspended, so neither applies.
- The remaining hits were single-target -DP triggers.
- I did not find an effect that lowers all of one's own Digimon's DP.
- Searching code/digimon-engine/cards for a DSL aura with a negative amount returned nothing.

My last two searches may have missed variants, and I did not exhaust every phrasing in the pool.

Our engine matches the ruling's other half: a Digimon whose DP is 0 is deleted, so the pending effect cannot activate. The existing base line, qa/dcgo-exams/BT16/BT16-082-effect0.yaml, already covers the positive case where Ukkomon survives the move.

If someone finds a card with a static or turn-long all-Digimon -DP effect that is active during a breeding phase, extend that base line with the setup and a move step, then stop right after the move.

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `no_legal_line` (attempt `20261006T161001157337Z-pilot-data-squad-2c352f`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No scenario written. I found no legal line that reaches Q2671, but I did not run a sim measurement (exam_probe) to confirm it, so treat this as unmeasured rather than proven unreachable.
>
> The ruling needs a DP-reducing effect already in force that takes the moving Ukkomon (BT16-082, 2000 DP) to 0 or below the moment it enters the battle area. A move from breeding to battle only happens in the breeding action at the start of a turn, so the reduction has to already be active then.
>
> What I searched in data/cards.json:
> - All "for the turn" mass -DP effects target the opponent's Digimon, and they come from [Main], [On Play] or [Security] effects. None can be in force during the opponent's breeding phase.
> - The only continuous (static) mass -DP effects I found hit opponent Security Digimon, or the opponent's *suspended* Digimon (BT16-101). A freshly moved Ukkomon is unsuspended, so neither applies.
> - The remaining hits were single-target -DP triggers.
> - I did not find an effect that lowers all of one's own Digimon's DP.
> - Searching code/digimon-engine/cards for a DSL aura with a negative amount returned nothing.
>
> My last two searches may have missed variants, and I did not exhaust every phrasing in the pool.
>
> Our engine matches the ruling's other half: a Digimon whose DP is 0 is deleted, so the pending effect cannot activate. The existing base line, qa/dcgo-exams/BT16/BT16-082-effect0.yaml, already covers the positive case where Ukkomon survives the move.
>
> If someone finds a card with a static or turn-long all-Digimon -DP effect that is active during a breeding phase, extend that base line with the setup and a move step, then stop right after the move.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T144209073758Z-pilot-data-squad-92bda6` | classify_qa | claude | sonnet | accepted | 0.2397 |
| 2 | `20261006T144209073835Z-pilot-data-squad-edbcd4` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T144311299263Z-pilot-data-squad-965e41` | encode_ruling | codex | (default) | gate_failed | unpriced |
| 4 | `20261006T160917421167Z-pilot-data-squad-5a1bab` | classify_qa | claude | sonnet | accepted | 0.2393 |
| 5 | `20261006T160917421242Z-pilot-data-squad-5b78f2` | classify_qa | codex | (default) | accepted | unpriced |
| 6 | `20261006T161001157337Z-pilot-data-squad-2c352f` | author_interaction | claude | sonnet | escalated | 0.7067 |

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
    "q_id": "Q2671"
  },
  "escalation": "interaction_qa_Q2671-6909f9a6.md",
  "escalation_reason": "attempt cap: encode_ruling 1/1 spent; the last step was deferred: malformed expect_ruling block: author result has no expect_ruling block",
  "history": [
    "20261006T144209073758Z-pilot-data-squad-92bda6",
    "20261006T144209073835Z-pilot-data-squad-edbcd4",
    "20261006T144311299263Z-pilot-data-squad-965e41",
    "20261006T160917421167Z-pilot-data-squad-5a1bab",
    "20261006T160917421242Z-pilot-data-squad-5b78f2"
  ],
  "source": "qa"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
