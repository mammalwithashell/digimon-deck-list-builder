---
item: clause:BT26-050#effect#2
run_id: pilot-data-squad
escalated_at: 2026-10-06T19:08:04.190321Z
state_before: GATE
reason: fix gate failed and the fix_card cap 2/2 is spent: gate worktree: git checkout -q --force --detach 2ef0c12df5f80dfce025819fd2f707d1a3062f95 failed in D:\cl-ds\cl-pilot-data-squad-gate: fatal: Unable to create 'C:/Users/james/Documents/digimon-deck-list-builder-1/.git/worktrees/cl-pilot-data-squad-gate/index.lock': File exists. Another git process seems to be running in this repository, e.g. an editor opened by 'git commit'. Please make sure all processes are terminated then try again. If it still fails, a git process may have crashed in this repository earlier: remove the file manually to continue. -- free or remove D:\cl-ds\cl-pilot-data-squad-gate
---

# Escalated: `clause:BT26-050#effect#2`

**Why:** fix gate failed and the fix_card cap 2/2 is spent: gate worktree: git checkout -q --force --detach 2ef0c12df5f80dfce025819fd2f707d1a3062f95 failed in D:\cl-ds\cl-pilot-data-squad-gate: fatal: Unable to create 'C:/Users/james/Documents/digimon-deck-list-builder-1/.git/worktrees/cl-pilot-data-squad-gate/index.lock': File exists.

Another git process seems to be running in this repository, e.g.
an editor opened by 'git commit'. Please make sure all processes
are terminated then try again. If it still fails, a git process
may have crashed in this repository earlier:
remove the file manually to continue. -- free or remove D:\cl-ds\cl-pilot-data-squad-gate

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T154944348722Z-pilot-data-squad-260443` | author_clause | claude | sonnet | accepted | 1.0012 |
| 2 | `20261006T160111932839Z-pilot-data-squad-d4b78e` | triage | codex | (default) | accepted | unpriced |
| 3 | `20261006T163728362766Z-pilot-data-squad-2fd123` | fix_card | claude | sonnet | gate_failed | 0.3225 |
| 4 | `20261006T182556423083Z-pilot-data-squad-e41b1a` | fix_card | claude | sonnet | accepted | 1.6777 |

## Item data

```json
{
  "author_attempt": "20261006T154944348722Z-pilot-data-squad-260443",
  "author_family": "claude",
  "author_stage": "author_clause",
  "card_ids": [
    "BT26-050"
  ],
  "covers": [
    "BT26-050#effect#2"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT26/BT26-050-effect2.yaml": "qa/dcgo-exams/BT26/bt26_dats_pool.json"
  },
  "engine_fix": false,
  "engine_fix_reason": "card / YAML fix: triage names no engine or DSL gap",
  "fix_attempt": "20261006T182556423083Z-pilot-data-squad-e41b1a",
  "fix_citation": "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\ST24\\Purple\\ST24_12.cs:49",
  "fix_family": "claude",
  "fix_result": {
    "citation": {
      "kind": "dcgo",
      "ref": "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\ST24\\Purple\\ST24_12.cs:49"
    },
    "files": [
      "code/digimon-engine/cards/st24/ST24-12.yaml",
      "code/digimon-engine/tests/cards_behavioral/st24/st24_12.rs"
    ],
    "gaps": [],
    "notes": "The exam divergence is not in BT26-050. It stopped at step 12, where our engine offered ST24-12 Falcomon's optional [On Play] and DCGO did not. DCGO's CanActivateCondition (ST24_12.cs:49-53) requires an owned Tamer with a face-down source, and the scripted board has no Tamer. So I fixed ST24-12.yaml and left BT26-050.yaml unchanged. The fix adds `condition: face_down_sources_under_tamers_gte: 1` to the On Play clause. That predicate already exists, and BT26-026 uses it. The regression test is `st24_12_on_play_not_offered_without_face_down_source`. It lives in st24_12.rs, not bt26_050.rs, because the card under test is ST24-12. It failed before the fix and passes after. The final run covers the st24_12, bt26_050, bt26_002 and bt26_091 tests and passed 23 of 23. Gotcha: one intermediate run still failed after the YAML edit, because the embedded card pack was stale. Touching the YAML and build.rs forced a rebuild. If the fix gate sees an unexpected failure, rebuild with a clean pack before debugging the card. I did not capture dsl-lint's result line; the output was truncated and I only saw warnings. Please confirm it exits 0 on ST24-12.yaml. ST24-12.yaml has no `# produced_by` header. Add one if the merge requires it. I did not touch BT26-050.yaml's header.",
    "test_result_lines": [
      "BASE (before fix): test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 10895 filtered out; finished in 2.15s",
      "AFTER fix (st24_12 + bt26_050 + bt26_002 + bt26_091): test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 10879 filtered out; finished in 1.73s"
    ],
    "tests": [
      "st24::st24_12::st24_12_on_play_not_offered_without_face_down_source"
    ]
  },
  "fix_stage": "fix_card",
  "gap_id": null,
  "history": [
    "20261006T154944348722Z-pilot-data-squad-260443",
    "20261006T160111932839Z-pilot-data-squad-d4b78e",
    "20261006T182556423083Z-pilot-data-squad-e41b1a"
  ],
  "merge": {
    "attempt_id": "20261006T182556423083Z-pilot-data-squad-e41b1a",
    "branch": "card-loop/pilot-data-squad/run",
    "engine": false,
    "ok": true,
    "scope": {
      "cards": [
        "ST24-12"
      ],
      "cards_behavioral_filter": "ST24-12",
      "full_suite_required": false,
      "reasons": [],
      "side_binaries": [
        "cards_behavioral",
        "dsl"
      ],
      "suite": {
        "commands": [
          {
            "argv": [
              "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
              "-c",
              "import os,subprocess,sys;a=sys.argv[1:];i=a.index('--');os.environ.update(x.split('=',1) for x in a[:i]);sys.exit(subprocess.call(a[i+1:]))",
              "RUST_MIN_STACK=268435456",
              "--",
              "cargo",
              "test",
              "--manifest-path",
              "code/digimon-engine/Cargo.toml",
              "--test",
              "cards_behavioral",
              "--",
              "st24_12",
              "st24::st24_12",
              "--test-threads=8"
            ],
            "command": "RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- st24_12 st24::st24_12 --test-threads=8",
            "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
            "rc": 0,
            "tail": "\nwarning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_151.rs:567:5\n    |\n567 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n567 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_169.rs:553:5\n    |\n553 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n553 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n  --> code\\digimon-engine\\tests\\cards_behavioral\\st12\\st12_12.rs:56:5\n   |\n56 |     runner.auto_resolve();\n   |     ^^^^^^^^^^^^^^^^^^^^^\n   |\n   = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n   |\n56 |     let _ = runner.auto_resolve();\n   |     +++++++\n\nwarning: `digimon-engine` (test \"cards_behavioral\") generated 118 warnings (7 duplicates) (run `cargo fix --test \"cards_behavioral\" -p digimon-engine` to apply 48 suggestions)\n    Finished `test` profile [optimized + debuginfo] target(s) in 2m 57s\n     Running tests\\cards_behavioral\\main.rs (D:\\cargo-target/digimon-card-authoring-loop-1aab52\\debug\\deps\\cards_behavioral-7a0f9073ad31bfef.exe)"
          },
          {
            "argv": [
              "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
              "-c",
              "import os,subprocess,sys;a=sys.argv[1:];i=a.index('--');os.environ.update(x.split('=',1) for x in a[:i]);sys.exit(subprocess.call(a[i+1:]))",
              "RUST_MIN_STACK=268435456",
              "--",
              "cargo",
              "test",
              "--manifest-path",
              "code/digimon-engine/Cargo.toml",
              "--test",
              "dsl",
              "--",
              "--test-threads=8"
            ],
            "command": "RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- --test-threads=8",
            "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
            "rc": 0,
            "tail": "warning: field `chooser` is never read\n   --> code\\digimon-engine\\src\\resume.rs:836:16\n    |\n835 | pub struct TriggerOrderSelectionState {\n    |            -------------------------- field in this struct\n836 |     pub(crate) chooser: PlayerId,\n    |                ^^^^^^^\n    |\n    = note: `TriggerOrderSelectionState` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis\n\nwarning: field `owner` is never read\n   --> code\\digimon-engine\\src\\resume.rs:855:16\n    |\n852 | pub struct OptionTrashOrderState {\n    |            --------------------- field in this struct\n...\n855 |     pub(crate) owner: PlayerId,\n    |                ^^^^^\n    |\n    = note: `OptionTrashOrderState` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis\n\nwarning: `digimon-engine` (lib) generated 60 warnings (run `cargo fix --lib -p digimon-engine` to apply 22 suggestions)\n   Compiling digimon-engine v0.1.0 (D:\\cl-ds\\cl-pilot-data-squad-run\\code\\digimon-engine)\nwarning: unused import: `digimon_engine::permanent::PermanentHandle`\n    --> code\\digimon-engine\\tests\\dsl\\group6_auras.rs:2934:9\n     |\n2934 |     use digimon_engine::permanent::PermanentHandle;\n     |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n     |\n     = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default\n\nwarning: unused import: `digimon_engine::permanent::PermanentHandle`\n  --> code\\digimon-engine\\tests\\dsl\\trash_link_card_of_own_digimon.rs:34:5\n   |\n34 | use digimon_engine::permanent::PermanentHandle;\n   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n\nwarning: `digimon-engine` (test \"dsl\") generated 2 warnings (run `cargo fix --test \"dsl\" -p digimon-engine` to apply 2 suggestions)\n    Finished `test` profile [optimized + debuginfo] target(s) in 9.99s\n     Running tests\\dsl\\main.rs (D:\\cargo-target/digimon-card-authoring-loop-1aab52\\debug\\deps\\dsl-a2288996e44112c6.exe)"
          }
        ],
        "green": true,
        "impact_scope": {
          "argv": [
            "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
            "code/tools/impact_scope.py",
            "--json",
            "--path",
            "code/digimon-engine/cards/st24/ST24-12.yaml",
            "--path",
            "code/digimon-engine/tests/cards_behavioral/st24/st24_12.rs"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path code/digimon-engine/cards/st24/ST24-12.yaml --path code/digimon-engine/tests/cards_behavioral/st24/st24_12.rs",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [\n    \"ST24-12\"\n  ],\n  \"cards_behavioral_filter\": \"ST24-12\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [\n    \"cards_behavioral\",\n    \"dsl\"\n  ],\n  \"verbs\": []\n}"
        },
        "ran": true
      },
      "verbs": []
    },
    "sha": "2ef0c12df5f80dfce025819fd2f707d1a3062f95",
    "touched": [
      "code/digimon-engine/cards/st24/ST24-12.yaml",
      "code/digimon-engine/tests/cards_behavioral/st24/st24_12.rs"
    ]
  },
  "merge_error": [
    "diff unreadable: git apply --numstat -z C:\\Users\\james\\AppData\\Local\\Temp\\card-loop\\attempts\\20261006T163728362766Z-pilot-data-squad-2fd123\\call-1\\worktree.diff failed in D:\\cl-ds\\cl-pilot-data-squad-run: error: No valid patches in input (allow with \"--allow-empty\")"
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT26-050#effect#2",
    "denominator": "compared 12 of 17 ours / 12 dcgo steps",
    "divergence": null,
    "first_divergence": "TRUNCATED, no divergence found (compared 12 of 17 ours / 12 dcgo steps)",
    "ids": [
      "BT26-050#effect#2"
    ],
    "job_id": "exam-BT26-050-effect2",
    "job_outcome": "failed",
    "mismatch": {
      "asked": "main_phase",
      "expected": "OptionalSkill",
      "row": 12,
      "step": 12
    },
    "reason": "DCGO job failed: prompt mismatch: step 12 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it",
    "recorded": [],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT26/BT26-050-effect2.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T155844Z_866021f6389f40889dc88adb2f4d402a.state.jsonl",
    "stall": null,
    "verdict": "unmeasured"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT26/BT26-050-effect2.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT26-050#effect#2",
      "denominator": "compared 12 of 17 ours / 12 dcgo steps",
      "divergence": null,
      "first_divergence": "TRUNCATED, no divergence found (compared 12 of 17 ours / 12 dcgo steps)",
      "ids": [
        "BT26-050#effect#2"
      ],
      "job_id": "exam-BT26-050-effect2",
      "job_outcome": "failed",
      "mismatch": {
        "asked": "main_phase",
        "expected": "OptionalSkill",
        "row": 12,
        "step": 12
      },
      "reason": "DCGO job failed: prompt mismatch: step 12 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it",
      "recorded": [],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT26/BT26-050-effect2.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T155844Z_866021f6389f40889dc88adb2f4d402a.state.jsonl",
      "stall": null,
      "verdict": "unmeasured"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": {
    "dcgo_asked": "main_phase",
    "dcgo_row": 12,
    "expected": "OptionalSkill",
    "explanation": "the scenario expected 'OptionalSkill' and our engine asked Replacement (OptionalSkill), but DCGO asked 'main_phase'",
    "ours": "OptionalSkill",
    "ours_snapshot": {
      "candidates": [
        [
          59,
          null
        ]
      ],
      "pending_kind": "Replacement",
      "pending_optional": true,
      "pending_prompt": "You may activate ST24-12's triggered effect",
      "step": 12
    },
    "route": "engines_disagree",
    "scenario": "qa/dcgo-exams/BT26/BT26-050-effect2.yaml",
    "scenario_step": 12,
    "step_mapping": "harness"
  },
  "prompt_route": "engines_disagree",
  "scenario_paths": [
    "qa/dcgo-exams/BT26/BT26-050-effect2.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT26/BT26-050-effect2.yaml": [
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "triage_first": {
    "attempt_id": "20261006T160111932839Z-pilot-data-squad-d4b78e",
    "call": "ours_wrong",
    "citation": {
      "kind": "dcgo",
      "ref": "C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/ST24/Purple/ST24_12.cs:49"
    },
    "family": "codex",
    "reasoning": "The mismatch concerns ST24-12's On Play effect, before Rosemon digivolves. DCGO's activation condition requires an owned Tamer with a face-down source (lines 49\u201363), matching the printed prerequisite. The scripted board has no Tamer, so DCGO correctly resumes main_phase. Our unconditional optional activation exposes an unavailable choice. Fix ST24-12's activation gate; this run establishes nothing about Rosemon's target clause. The requested exam_probe replay was attempted but blocked by tool approval policy; our prompt is supported by the supplied evidence."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: clause:BT26-050#effect#2\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: clause `BT26-050#effect#2` of Rosemon: Burst Mode (BT26-050): You may suspend 2 Digimon or Tamers. Then, 2 of your opponent's Digimon or Tamers can't unsuspend until their turn ends.\n- Scenario(s): `qa/dcgo-exams/BT26/BT26-050-effect2.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT26-050#effect#2\",\n  \"denominator\": \"compared 12 of 17 ours / 12 dcgo steps\",\n  \"divergence\": null,\n  \"first_divergence\": \"TRUNCATED, no divergence found (compared 12 of 17 ours / 12 dcgo steps)\",\n  \"ids\": [\n    \"BT26-050#effect#2\"\n  ],\n  \"job_id\": \"exam-BT26-050-effect2\",\n  \"job_outcome\": \"failed\",\n  \"mismatch\": {\n    \"asked\": \"main_phase\",\n    \"expected\": \"OptionalSkill\",\n    \"row\": 12,\n    \"step\": 12\n  },\n  \"reason\": \"DCGO job failed: prompt mismatch: step 12 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it\",\n  \"recorded\": [],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT26/BT26-050-effect2.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T155844Z_866021f6389f40889dc88adb2f4d402a.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"unmeasured\"\n}\n```\n- Prompt evidence: at scenario step 12 the scenario expected `OptionalSkill`, DCGO asked `main_phase`, and our engine asked `OptionalSkill` (the scenario expected 'OptionalSkill' and our engine asked Replacement (OptionalSkill), but DCGO asked 'main_phase').\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT26\\Green\\BT26_050.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT26-050.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "2",
    "references": [
      "qa/dcgo-exams/BT26/BT26-050-effect2.yaml",
      "data/card_bundles/BT26-050.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT26\\Green\\BT26_050.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T155844Z_866021f6389f40889dc88adb2f4d402a.state.jsonl",
      "docs/digimon-rules/rules-index.json",
      "docs/digimon-rules/keyword-semantics.md"
    ]
  }
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
