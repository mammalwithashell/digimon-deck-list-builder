---
item: interaction:qa:Q7147
run_id: pilot-data-squad
escalated_at: 2026-10-08T02:54:41.044572Z
state_before: ENCODE
reason: attempt cap: encode_ruling 2/2 spent (state ENCODE)
---

# Escalated: `interaction:qa:Q7147`

**Why:** attempt cap: encode_ruling 2/2 spent (state ENCODE)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261008T025123406613Z-pilot-data-squad-85e04e` | classify_qa | claude | sonnet | accepted | 0.2421 |
| 2 | `20261008T025123406661Z-pilot-data-squad-c026f3` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261008T025152544640Z-pilot-data-squad-a2e300` | author_interaction | claude | sonnet | accepted | 0.4467 |
| 4 | `20261008T025235919689Z-pilot-data-squad-5cec71` | encode_ruling | claude | sonnet | gate_failed | 0.3004 |
| 5 | `20261008T025305407051Z-pilot-data-squad-62478d` | encode_ruling | codex | (default) | accepted | unpriced |
| 6 | `20261008T025338719689Z-pilot-data-squad-6565f1` | encode_ruling | claude | sonnet | gate_failed | 0.2630 |
| 7 | `20261008T025358550276Z-pilot-data-squad-5efb35` | encode_ruling | codex | (default) | accepted | unpriced |

## Item data

```json
{
  "author_attempt": "20261008T025152544640Z-pilot-data-squad-a2e300",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT26/BT26-091-qa-Q7147.yaml",
  "card_ids": [
    "BT26-091"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT26-091#effect#0",
      "BT26-091#effect#1"
    ],
    "q_id": "Q7147"
  },
  "covers": [
    "BT26-091#effect#0",
    "BT26-091#effect#1"
  ],
  "deck_books": null,
  "encode_feedback": "codex (verifier): Step 8 trashes the face-down ST24-15 from under Yoshino, reaching the ruling's situation. However, p0.trash projects card IDs without face orientation. [\"ST24-15\"] therefore also passes under the opposite reading that the card remains face-down in the trash. The block checks its destination but cannot discriminate the answer's face-up requirement. | quote: It's placed face-up in the trash.",
  "expect_ruling": null,
  "history": [
    "20261008T025123406613Z-pilot-data-squad-85e04e",
    "20261008T025123406661Z-pilot-data-squad-c026f3",
    "20261008T025152544640Z-pilot-data-squad-a2e300",
    "20261008T025235919689Z-pilot-data-squad-5cec71",
    "20261008T025305407051Z-pilot-data-squad-62478d",
    "20261008T025338719689Z-pilot-data-squad-6565f1",
    "20261008T025358550276Z-pilot-data-squad-5efb35"
  ],
  "merge": {
    "attempt_id": "20261008T025152544640Z-pilot-data-squad-a2e300",
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
            "qa/dcgo-exams/BT26/BT26-091-qa-Q7147.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT26/BT26-091-qa-Q7147.yaml",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "45c1f42f18818e2e5ff51b1d2497397231d1fabd",
    "touched": [
      "qa/dcgo-exams/BT26/BT26-091-qa-Q7147.yaml"
    ]
  },
  "oracle_results": null,
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT26/BT26-091-qa-Q7147.yaml"
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
