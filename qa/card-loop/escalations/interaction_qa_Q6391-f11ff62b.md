---
item: interaction:qa:Q6391
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T18:35:48.524814Z
state_before: ORACLE
reason: attempt cap: oracle_retry 3/3 spent (state ORACLE)
---

# Escalated: `interaction:qa:Q6391`

**Why:** attempt cap: oracle_retry 3/3 spent (state ORACLE)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T145721119385Z-pilot-three-musketeers-f0bb5a` | classify_qa | claude | sonnet | accepted | 0.2402 |
| 2 | `20261006T145721119444Z-pilot-three-musketeers-ff9ebe` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T153442526108Z-pilot-three-musketeers-2c5af7` | encode_ruling | codex | (default) | escalated | unpriced |
| 4 | `20261006T153505802879Z-pilot-three-musketeers-85d3e7` | encode_ruling | claude | sonnet | escalated | 0.2703 |
| 5 | `20261006T172004059689Z-pilot-three-musketeers-2f9680` | classify_qa | claude | sonnet | accepted | 0.2412 |
| 6 | `20261006T172004059757Z-pilot-three-musketeers-cfb39f` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T172217397006Z-pilot-three-musketeers-a3b8f7` | author_interaction | claude | sonnet | accepted | 0.9024 |
| 8 | `20261006T182707636109Z-pilot-three-musketeers-7b95ee` | encode_ruling | codex | (default) | accepted | unpriced |
| 9 | `20261006T183044846045Z-pilot-three-musketeers-e0f80b` | encode_ruling | claude | sonnet | accepted | 0.2651 |
| 10 | `20261006T183203383027Z-pilot-three-musketeers-c715d9` | author_interaction | claude | sonnet | accepted | 0.4243 |

## Item data

```json
{
  "author_attempt": "20261006T183203383027Z-pilot-three-musketeers-c715d9",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml",
  "card_ids": [
    "BT25-082"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT25-082#effect#2"
    ],
    "q_id": "Q6391"
  },
  "covers": [
    "BT25-082#effect#0",
    "BT25-082#effect#1",
    "BT25-082#effect#2",
    "BT25-092#effect#1",
    "EX7-059#effect#1",
    "EX7-059#effect#2"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml": "qa/dcgo-exams/BT25/tm_bt25_082_q6391_pool.json"
  },
  "encode_attempts": [
    "20261006T182707636109Z-pilot-three-musketeers-7b95ee",
    "20261006T183044846045Z-pilot-three-musketeers-e0f80b"
  ],
  "encode_feedback": null,
  "escalation": "interaction_qa_Q6391-f11ff62b.md",
  "escalation_reason": "verifier disagrees: Yes, you can. (digivolve BT25-082 into EX7-059 [BeelStarmon], ignoring conditions, without paying the cost, during my opponent's counter timing)",
  "expect_ruling": {
    "assert": [
      {
        "at": 20,
        "that": {
          "p0.field": [
            {
              "card_id": "EX7-059",
              "dp": 11000,
              "sources": [
                "BT25-082",
                "EX7-051",
                "ST6-01"
              ],
              "suspended": true
            },
            {
              "card_id": "BT25-092",
              "dp": -1,
              "sources": [],
              "suspended": false
            }
          ],
          "p0.memory": -3
        }
      }
    ],
    "q_id": "Q6391"
  },
  "history": [
    "20261006T145721119385Z-pilot-three-musketeers-f0bb5a",
    "20261006T145721119444Z-pilot-three-musketeers-ff9ebe",
    "20261006T153442526108Z-pilot-three-musketeers-2c5af7",
    "20261006T153505802879Z-pilot-three-musketeers-85d3e7",
    "20261006T172004059689Z-pilot-three-musketeers-2f9680",
    "20261006T172004059757Z-pilot-three-musketeers-cfb39f",
    "20261006T172217397006Z-pilot-three-musketeers-a3b8f7",
    "20261006T182707636109Z-pilot-three-musketeers-7b95ee",
    "20261006T183044846045Z-pilot-three-musketeers-e0f80b",
    "20261006T183203383027Z-pilot-three-musketeers-c715d9"
  ],
  "merge": {
    "attempt_id": "20261006T183203383027Z-pilot-three-musketeers-c715d9",
    "branch": "card-loop/pilot-three-musketeers/run",
    "engine": false,
    "ok": true,
    "scope": {
      "cards": [],
      "cards_behavioral_filter": "",
      "full_suite_required": false,
      "reasons": [],
      "side_binaries": [],
      "suite": {
        "commands": [],
        "green": true,
        "impact_scope": {
          "argv": [
            "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
            "code/tools/impact_scope.py",
            "--json",
            "--path",
            "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "adaf38e601975695457c7e5cdd10eedb5cf99231",
    "touched": [
      "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml"
    ]
  },
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT25-082#effect#2",
    "denominator": "compared 16 of 22 ours / 18 dcgo steps (5 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable)",
    "divergence": null,
    "first_divergence": "TRUNCATED, no divergence found (compared 16 of 22 ours / 18 dcgo steps (5 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))",
    "ids": [
      "qa:Q6391"
    ],
    "job_id": "exam-BT25-082-qa-Q6391",
    "job_outcome": "failed",
    "mismatch": null,
    "reason": "DCGO job failed: SelectHandEffect prompt needs select_card_ids or select_cancel, got: select_value=1 -- stopped before the line finished, with no divergence before it",
    "recorded": [],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T183519Z_3088f9e1316842bf9ab882db416b6b31.state.jsonl",
    "stall": null,
    "verdict": "unmeasured"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT25-082#effect#2",
      "denominator": "compared 16 of 22 ours / 18 dcgo steps (5 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable)",
      "divergence": null,
      "first_divergence": "TRUNCATED, no divergence found (compared 16 of 22 ours / 18 dcgo steps (5 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))",
      "ids": [
        "qa:Q6391"
      ],
      "job_id": "exam-BT25-082-qa-Q6391",
      "job_outcome": "failed",
      "mismatch": null,
      "reason": "DCGO job failed: SelectHandEffect prompt needs select_card_ids or select_cancel, got: select_value=1 -- stopped before the line finished, with no divergence before it",
      "recorded": [],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T183519Z_3088f9e1316842bf9ab882db416b6b31.state.jsonl",
      "stall": null,
      "verdict": "unmeasured"
    }
  },
  "oracle_retry_paths": [
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml"
  ],
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "ruling_block_written": null,
  "ruling_contradicted": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6391.yaml": [
      "step 9 expect.prompt 'OptionalSkill' not asserted sim-side (kind UnionZone { zones: UnionZoneSet(3) } has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "step 11 expect.prompt 'SelectCountEffect' not asserted sim-side (kind EffectChoice has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "triage_feedback": null
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
