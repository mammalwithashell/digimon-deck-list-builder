---
item: clause:BT13-060#effect#0
run_id: pilot-data-squad
escalated_at: 2026-10-06T13:46:47.416115Z
state_before: AUTHORING
reason: claude found no legal line for BT13-060#effect#0: No legal scripted line reaches BT13-060#effect#0 ([Burst Digivolve]: cost 0 from [Rosemon] by returning 1 [Yoshino Fujieda] to hand; at end of burst turn trash this Digimon's top card). (1) The exam step vocabulary (STEP_VERBS in code/tools/dcgo-harness/src/exam/scenario.rs:309: hatch/pass/move/play/digivolve/dna/attack/main/link/select) has no burst-digivolve verb; `digivolve:{from,using}` cannot name the returned Tamer, and a plain digivolve takes the printed Lv.6 cost-5 path, which does not exercise this clause. (2) Our engine has no action path for burst: BurstDigivolve alt paths compile only to a 'Blast digivolve marker' (dsl_cards/mod.rs:101), alt paths with extra_cost/on_burst_turn_end are skipped by the digivolve lookup (dna_digivolve.rs:513), and nothing under src/ consumes extra_cost; bt13_020.rs:203-212 documents the same gap (no burst action helper). (3) DCGO records burst as a PlayCardAction BurstTamerFrameID decomposed into a SOURCE_SELECT row (ActionEncoder.DecomposePlayCardExtras) and asks through SelectBurstDigivolutionEffect, which is outside InputDriver's closed 13-kind prompt set (adapter.rs:1102). Also note: the dcgo-exam MCP failed to connect (CONNECTION_CLOSED) in this session, so exam_validate/exam_probe were unavailable; I relied on qa/exam-authoring-guide.json, existing scenarios and source reading. Recommendation: log an engine/harness gap (burst-digivolve action + a `burst:` step verb) in docs/RUST_ENGINE_GAPS.md and mark the clause unreachable/unavailable until it exists. No BT13 exam directory exists yet and BT13-060 has only a .json in cards/bt13 (the YAML exemplar is in cards/_examples/BT13-060.yaml).
---

# Escalated: `clause:BT13-060#effect#0`

**Why:** claude found no legal line for BT13-060#effect#0: No legal scripted line reaches BT13-060#effect#0 ([Burst Digivolve]: cost 0 from [Rosemon] by returning 1 [Yoshino Fujieda] to hand; at end of burst turn trash this Digimon's top card). (1) The exam step vocabulary (STEP_VERBS in code/tools/dcgo-harness/src/exam/scenario.rs:309: hatch/pass/move/play/digivolve/dna/attack/main/link/select) has no burst-digivolve verb; `digivolve:{from,using}` cannot name the returned Tamer, and a plain digivolve takes the printed Lv.6 cost-5 path, which does not exercise this clause. (2) Our engine has no action path for burst: BurstDigivolve alt paths compile only to a 'Blast digivolve marker' (dsl_cards/mod.rs:101), alt paths with extra_cost/on_burst_turn_end are skipped by the digivolve lookup (dna_digivolve.rs:513), and nothing under src/ consumes extra_cost; bt13_020.rs:203-212 documents the same gap (no burst action helper). (3) DCGO records burst as a PlayCardAction BurstTamerFrameID decomposed into a SOURCE_SELECT row (ActionEncoder.DecomposePlayCardExtras) and asks through SelectBurstDigivolutionEffect, which is outside InputDriver's closed 13-kind prompt set (adapter.rs:1102). Also note: the dcgo-exam MCP failed to connect (CONNECTION_CLOSED) in this session, so exam_validate/exam_probe were unavailable; I relied on qa/exam-authoring-guide.json, existing scenarios and source reading. Recommendation: log an engine/harness gap (burst-digivolve action + a `burst:` step verb) in docs/RUST_ENGINE_GAPS.md and mark the clause unreachable/unavailable until it exists. No BT13 exam directory exists yet and BT13-060 has only a .json in cards/bt13 (the YAML exemplar is in cards/_examples/BT13-060.yaml).

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `no_legal_line` (attempt `20261006T134445083580Z-pilot-data-squad-fd4558`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> No legal scripted line reaches BT13-060#effect#0 ([Burst Digivolve]: cost 0 from [Rosemon] by returning 1 [Yoshino Fujieda] to hand; at end of burst turn trash this Digimon's top card). (1) The exam step vocabulary (STEP_VERBS in code/tools/dcgo-harness/src/exam/scenario.rs:309: hatch/pass/move/play/digivolve/dna/attack/main/link/select) has no burst-digivolve verb; `digivolve:{from,using}` cannot name the returned Tamer, and a plain digivolve takes the printed Lv.6 cost-5 path, which does not exercise this clause. (2) Our engine has no action path for burst: BurstDigivolve alt paths compile only to a 'Blast digivolve marker' (dsl_cards/mod.rs:101), alt paths with extra_cost/on_burst_turn_end are skipped by the digivolve lookup (dna_digivolve.rs:513), and nothing under src/ consumes extra_cost; bt13_020.rs:203-212 documents the same gap (no burst action helper). (3) DCGO records burst as a PlayCardAction BurstTamerFrameID decomposed into a SOURCE_SELECT row (ActionEncoder.DecomposePlayCardExtras) and asks through SelectBurstDigivolutionEffect, which is outside InputDriver's closed 13-kind prompt set (adapter.rs:1102). Also note: the dcgo-exam MCP failed to connect (CONNECTION_CLOSED) in this session, so exam_validate/exam_probe were unavailable; I relied on qa/exam-authoring-guide.json, existing scenarios and source reading. Recommendation: log an engine/harness gap (burst-digivolve action + a `burst:` step verb) in docs/RUST_ENGINE_GAPS.md and mark the clause unreachable/unavailable until it exists. No BT13 exam directory exists yet and BT13-060 has only a .json in cards/bt13 (the YAML exemplar is in cards/_examples/BT13-060.yaml).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T134445083580Z-pilot-data-squad-fd4558` | author_clause | claude | sonnet | escalated | 0.7574 |

## Item data

```json
{
  "card_ids": [
    "BT13-060"
  ]
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
