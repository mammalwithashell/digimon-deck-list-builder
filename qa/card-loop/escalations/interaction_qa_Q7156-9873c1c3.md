---
item: interaction:qa:Q7156
run_id: pilot-data-squad
escalated_at: 2026-10-08T03:51:03.889832Z
state_before: ENCODE
reason: attempt cap: encode_ruling 2/2 spent (state ENCODE)
---

# Escalated: `interaction:qa:Q7156`

**Why:** attempt cap: encode_ruling 2/2 spent (state ENCODE)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261008T032835347115Z-pilot-data-squad-e5e028` | classify_qa | claude | sonnet | accepted | 0.2429 |
| 2 | `20261008T032835347165Z-pilot-data-squad-08d6d9` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261008T032908920533Z-pilot-data-squad-57ad22` | author_interaction | claude | sonnet | accepted | 0.3827 |
| 4 | `20261008T034934775142Z-pilot-data-squad-61c276` | encode_ruling | codex | (default) | gate_failed | unpriced |
| 5 | `20261008T035013212374Z-pilot-data-squad-436c0c` | encode_ruling | codex | (default) | gate_failed | unpriced |

## Item data

```json
{
  "author_attempt": "20261008T032908920533Z-pilot-data-squad-57ad22",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT26/BT26-094-effect0.yaml",
  "card_ids": [
    "BT26-094"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT26-094#effect#0"
    ],
    "q_id": "Q7156"
  },
  "covers": [
    "BT26-094#effect#0"
  ],
  "deck_books": null,
  "encode_feedback": null,
  "history": [
    "20261008T032835347115Z-pilot-data-squad-e5e028",
    "20261008T032835347165Z-pilot-data-squad-08d6d9",
    "20261008T032908920533Z-pilot-data-squad-57ad22",
    "20261008T034934775142Z-pilot-data-squad-61c276",
    "20261008T035013212374Z-pilot-data-squad-436c0c"
  ],
  "merge": {
    "attempt_id": "20261008T032908920533Z-pilot-data-squad-57ad22",
    "branch": "card-loop/pilot-data-squad/run",
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
            "qa/dcgo-exams/BT26/BT26-094-qa-Q7156.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT26/BT26-094-qa-Q7156.yaml",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "302f64b6e7dad1e142e9b593ae528ebb78e3a354",
    "touched": [
      "qa/dcgo-exams/BT26/BT26-094-qa-Q7156.yaml"
    ]
  },
  "oracle_results": null,
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT26/BT26-094-qa-Q7156.yaml"
  ],
  "sim_failure": null,
  "source": "qa",
  "triage_feedback": null
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
