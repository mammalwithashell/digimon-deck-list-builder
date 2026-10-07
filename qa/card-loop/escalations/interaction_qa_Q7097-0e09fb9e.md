---
item: interaction:qa:Q7097
run_id: pilot-data-squad
escalated_at: 2026-10-07T01:04:51.357503Z
state_before: ENCODE
reason: attempt cap: encode_ruling 2/2 spent (state ENCODE)
---

# Escalated: `interaction:qa:Q7097`

**Why:** attempt cap: encode_ruling 2/2 spent (state ENCODE)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261007T005742643025Z-pilot-data-squad-f18307` | classify_qa | claude | sonnet | accepted | 0.2413 |
| 2 | `20261007T005742643152Z-pilot-data-squad-9a0903` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261007T005821561019Z-pilot-data-squad-fd173c` | author_interaction | codex | (default) | accepted | unpriced |
| 4 | `20261007T010256712130Z-pilot-data-squad-84e49c` | encode_ruling | claude | sonnet | gate_failed | 0.2646 |
| 5 | `20261007T010317684379Z-pilot-data-squad-7efc33` | encode_ruling | codex | (default) | accepted | unpriced |
| 6 | `20261007T010355452383Z-pilot-data-squad-5f075e` | encode_ruling | claude | sonnet | gate_failed | 0.2711 |
| 7 | `20261007T010417461302Z-pilot-data-squad-8cab72` | encode_ruling | codex | (default) | accepted | unpriced |

## Item data

```json
{
  "author_attempt": "20261007T005821561019Z-pilot-data-squad-fd173c",
  "author_family": "codex",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT26/BT26-072-qa-Q7097.yaml",
  "card_ids": [
    "BT26-072"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT26-072#effect#2"
    ],
    "q_id": "Q7097"
  },
  "covers": [
    "BT26-072#effect#2",
    "BT26-094#effect#0",
    "BT26-076#effect#1",
    "BT26-076#effect#2",
    "BT26-094#effect#1"
  ],
  "deck_books": null,
  "encode_feedback": "codex (verifier): Step 14 reaches the ruling situation by trashing BT26-082 from beneath the Tamer. However, p0.trash projects only card IDs, not face orientation. The candidate therefore passes both when BT26-082 enters trash face-up and under the opposite reading where it remains face-down. It checks the destination but does not discriminate the answer's face-up requirement. | quote: It's placed face-up in the trash.",
  "expect_ruling": null,
  "history": [
    "20261007T005742643025Z-pilot-data-squad-f18307",
    "20261007T005742643152Z-pilot-data-squad-9a0903",
    "20261007T005821561019Z-pilot-data-squad-fd173c",
    "20261007T010256712130Z-pilot-data-squad-84e49c",
    "20261007T010317684379Z-pilot-data-squad-7efc33",
    "20261007T010355452383Z-pilot-data-squad-5f075e",
    "20261007T010417461302Z-pilot-data-squad-8cab72"
  ],
  "merge": {
    "attempt_id": "20261007T005821561019Z-pilot-data-squad-fd173c",
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
            "qa/dcgo-exams/BT26/BT26-072-qa-Q7097.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT26/BT26-072-qa-Q7097.yaml",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "595f6cd0e9c5125fff0ed2a8a612cbb64bde0ff0",
    "touched": [
      "qa/dcgo-exams/BT26/BT26-072-qa-Q7097.yaml"
    ]
  },
  "oracle_results": null,
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT26/BT26-072-qa-Q7097.yaml"
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
