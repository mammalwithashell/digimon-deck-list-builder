---
item: interaction:probe:BT21-071#effect#1:scope:neg
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T13:55:15.458779Z
state_before: AUTHORING
reason: attempt cap: author_interaction 3/3 spent; the last merge failed: git apply --3way failed (rc 1): Performing three-way merge... Applied patch to 'qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml' with conflicts. U qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml
---

# Escalated: `interaction:probe:BT21-071#effect#1:scope:neg`

**Why:** attempt cap: author_interaction 3/3 spent; the last merge failed: git apply --3way failed (rc 1): Performing three-way merge...
Applied patch to 'qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml' with conflicts.
U qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T134840115383Z-pilot-three-musketeers-6004b7` | author_interaction | claude | sonnet | accepted | 0.7007 |
| 2 | `20261006T135135661963Z-pilot-three-musketeers-a63d71` | author_interaction | claude | sonnet | gate_failed | 0.5042 |
| 3 | `20261006T135349413214Z-pilot-three-musketeers-b1ea27` | author_interaction | claude | sonnet | gate_failed | 0.5946 |

## Item data

```json
{
  "author_attempt": "20261006T134840115383Z-pilot-three-musketeers-6004b7",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT21/BT21-071-effect1.yaml",
  "card_ids": [
    "BT21-071"
  ],
  "covers": [
    "BT21-071#effect#1"
  ],
  "deck_books": null,
  "history": [
    "20261006T134840115383Z-pilot-three-musketeers-6004b7"
  ],
  "merge": {
    "attempt_id": "20261006T134840115383Z-pilot-three-musketeers-6004b7",
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
            "qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "1a94ae911431b818379b0110bef66b6db1fda216",
    "touched": [
      "qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml"
    ]
  },
  "merge_error": [
    "git apply --3way failed (rc 1): Performing three-way merge...\nApplied patch to 'qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml' with conflicts.\nU qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml"
  ],
  "oracle_results": null,
  "oracle_retry_paths": null,
  "prompt_evidence": null,
  "prompt_route": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml"
  ],
  "sim_failure": {
    "qa/dcgo-exams/BT21/BT21-071-effect1-scope-neg.yaml": [
      "FAILED: stacked card ST1-02 is not in deck `three-musketeers` -- stacking it would silently change the deck list the scenario claims to use"
    ]
  },
  "source": "probe"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
