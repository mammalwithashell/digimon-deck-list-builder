---
item: clause:BT13-060#effect#2
run_id: pilot-data-squad
escalated_at: 2026-10-06T18:12:24.328390Z
state_before: GATE
reason: fix gate failed and the fix_card cap 2/2 is spent: citation malformed: dcgo ref 'C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs:218' is not <File>.cs:<line> or a C# path -- cite a general_rule.pdf section (e.g. 16-36), an official ruling (qa:Q1234) or a DCGO file:line (BT21_029.cs:45); a fix that cannot cite a source is a finding -- escalate the item
---

# Escalated: `clause:BT13-060#effect#2`

**Why:** fix gate failed and the fix_card cap 2/2 is spent: citation malformed: dcgo ref 'C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs:218' is not <File>.cs:<line> or a C# path -- cite a general_rule.pdf section (e.g. 16-36), an official ruling (qa:Q1234) or a DCGO file:line (BT21_029.cs:45); a fix that cannot cite a source is a finding -- escalate the item

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T134445083832Z-pilot-data-squad-34087d` | author_clause | claude | sonnet | gate_failed | 1.4629 |
| 2 | `20261006T134928466659Z-pilot-data-squad-de2b9b` | author_clause | claude | sonnet | accepted | 1.9492 |
| 3 | `20261006T141212077944Z-pilot-data-squad-7621c5` | author_clause | claude | sonnet | accepted | 0.6125 |
| 4 | `20261006T141443956202Z-pilot-data-squad-f3b8ec` | triage | claude | sonnet | accepted | 0.6055 |
| 5 | `20261006T141632165072Z-pilot-data-squad-c921ec` | fix_card | claude | sonnet | gate_failed | 1.1257 |
| 6 | `20261006T142356248864Z-pilot-data-squad-08ac83` | fix_card | claude | sonnet | accepted | 0.3909 |
| 7 | `20261006T142436193849Z-pilot-data-squad-c84d06` | fix_engine | codex | (default) | gate_failed | unpriced |
| 8 | `20261006T160630812683Z-pilot-data-squad-2a991c` | triage | claude | sonnet | accepted | 0.6494 |
| 9 | `20261006T160755605142Z-pilot-data-squad-90fdcf` | fix_card | claude | sonnet | gate_failed | 1.2631 |
| 10 | `20261006T164919922300Z-pilot-data-squad-ecf874` | fix_card | claude | sonnet | accepted | 1.4493 |

## Item data

```json
{
  "author_attempt": "20261006T141212077944Z-pilot-data-squad-7621c5",
  "author_family": "claude",
  "author_stage": "author_clause",
  "card_ids": [
    "BT13-060"
  ],
  "citation": null,
  "covers": [
    "BT13-060#effect#2"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT13/BT13-060-effect2.yaml": "qa/dcgo-exams/BT13/tm_bt13_060_pool.json"
  },
  "engine_fix": false,
  "engine_fix_reason": "card / YAML fix: triage names no engine or DSL gap",
  "escalation": "clause_BT13-060_effect_2-3117b387.md",
  "escalation_reason": "attempt cap: fix_card 2/2 spent; the last merge failed: qa/dsl-vocab-gaps.md: gap-tracker writes are orchestrator-only -- the worker must report gaps in its result's `gaps`, and the driver records them",
  "fix_attempt": "20261006T164919922300Z-pilot-data-squad-ecf874",
  "fix_citation": "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs:218",
  "fix_family": "claude",
  "fix_result": {
    "citation": {
      "kind": "dcgo",
      "ref": "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs:218"
    },
    "files": [
      "code/digimon-engine/cards/bt13/BT13-060.yaml",
      "code/digimon-engine/tests/cards_behavioral/bt13/bt13_060.rs",
      "code/digimon-engine/tests/cards_behavioral/bt13/mod.rs"
    ],
    "gaps": [],
    "notes": "No DSL gap: trash_top_security already takes a `count` formula and card_count_in_zone accepts a filter (is_suspended + any_of digimon/tamer), so the [When Attacking] clause is plain DSL. The canonical cards/bt13/BT13-060.yaml did not exist (the spec lived only in cards/_examples/ with the clause omitted), so I created it with the provenance header, the [When Digivolving] clause and alt_paths copied unchanged, plus the new when_attacking clause. I also added the bt13_060.rs test module and registered it in bt13/mod.rs. `cargo run -p dsl-lint -- code/digimon-engine/cards/bt13/BT13-060.yaml` completed with exit code 0 and printed no result lines. The tests fail at base because the YAML is absent (include_str! breaks); they were not run against the old _examples spec. Not done: the card's other bt13 tests were not re-run, and the DCGO exam scenario was not re-run to confirm the divergence is gone.",
    "test_result_lines": [
      "test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 10897 filtered out; finished in 0.56s"
    ],
    "tests": [
      "bt13::bt13_060::bt13_060_when_attacking_trashes_one_security_per_two_suspended_digimon_and_tamers",
      "bt13::bt13_060::bt13_060_when_attacking_scales_and_ignores_unsuspended",
      "bt13::bt13_060::bt13_060_when_attacking_odd_count_rounds_down",
      "bt13::bt13_060::bt13_060_when_attacking_trashes_nothing_below_two"
    ]
  },
  "fix_stage": "fix_card",
  "gap_id": null,
  "history": [
    "20261006T134928466659Z-pilot-data-squad-de2b9b",
    "20261006T141212077944Z-pilot-data-squad-7621c5",
    "20261006T141443956202Z-pilot-data-squad-f3b8ec",
    "20261006T160630812683Z-pilot-data-squad-2a991c",
    "20261006T164919922300Z-pilot-data-squad-ecf874"
  ],
  "merge": {
    "attempt_id": "20261006T164919922300Z-pilot-data-squad-ecf874",
    "branch": "card-loop/pilot-data-squad/run",
    "engine": false,
    "ok": true,
    "scope": {
      "cards": [
        "BT13-060"
      ],
      "cards_behavioral_filter": "BT13-060",
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
              "bt13_060",
              "bt13::bt13_060",
              "bt13::",
              "--test-threads=8"
            ],
            "command": "RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt13_060 bt13::bt13_060 bt13:: --test-threads=8",
            "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
            "rc": 0,
            "tail": "\nwarning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_151.rs:567:5\n    |\n567 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n567 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_169.rs:553:5\n    |\n553 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n553 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n  --> code\\digimon-engine\\tests\\cards_behavioral\\st12\\st12_12.rs:56:5\n   |\n56 |     runner.auto_resolve();\n   |     ^^^^^^^^^^^^^^^^^^^^^\n   |\n   = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n   |\n56 |     let _ = runner.auto_resolve();\n   |     +++++++\n\nwarning: `digimon-engine` (test \"cards_behavioral\") generated 118 warnings (7 duplicates) (run `cargo fix --test \"cards_behavioral\" -p digimon-engine` to apply 48 suggestions)\n    Finished `test` profile [optimized + debuginfo] target(s) in 19m 06s\n     Running tests\\cards_behavioral\\main.rs (D:\\cargo-target/digimon-card-authoring-loop-1aab52\\debug\\deps\\cards_behavioral-7a0f9073ad31bfef.exe)"
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
            "tail": "warning: field `chooser` is never read\n   --> code\\digimon-engine\\src\\resume.rs:836:16\n    |\n835 | pub struct TriggerOrderSelectionState {\n    |            -------------------------- field in this struct\n836 |     pub(crate) chooser: PlayerId,\n    |                ^^^^^^^\n    |\n    = note: `TriggerOrderSelectionState` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis\n\nwarning: field `owner` is never read\n   --> code\\digimon-engine\\src\\resume.rs:855:16\n    |\n852 | pub struct OptionTrashOrderState {\n    |            --------------------- field in this struct\n...\n855 |     pub(crate) owner: PlayerId,\n    |                ^^^^^\n    |\n    = note: `OptionTrashOrderState` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis\n\nwarning: `digimon-engine` (lib) generated 60 warnings (run `cargo fix --lib -p digimon-engine` to apply 22 suggestions)\n   Compiling digimon-engine v0.1.0 (D:\\cl-ds\\cl-pilot-data-squad-run\\code\\digimon-engine)\nwarning: unused import: `digimon_engine::permanent::PermanentHandle`\n    --> code\\digimon-engine\\tests\\dsl\\group6_auras.rs:2934:9\n     |\n2934 |     use digimon_engine::permanent::PermanentHandle;\n     |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n     |\n     = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default\n\nwarning: unused import: `digimon_engine::permanent::PermanentHandle`\n  --> code\\digimon-engine\\tests\\dsl\\trash_link_card_of_own_digimon.rs:34:5\n   |\n34 | use digimon_engine::permanent::PermanentHandle;\n   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n\nwarning: `digimon-engine` (test \"dsl\") generated 2 warnings (run `cargo fix --test \"dsl\" -p digimon-engine` to apply 2 suggestions)\n    Finished `test` profile [optimized + debuginfo] target(s) in 6m 48s\n     Running tests\\dsl\\main.rs (D:\\cargo-target/digimon-card-authoring-loop-1aab52\\debug\\deps\\dsl-a2288996e44112c6.exe)"
          }
        ],
        "green": true,
        "impact_scope": {
          "argv": [
            "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe",
            "code/tools/impact_scope.py",
            "--json",
            "--path",
            "code/digimon-engine/cards/bt13/BT13-060.yaml",
            "--path",
            "code/digimon-engine/tests/cards_behavioral/bt13/bt13_060.rs",
            "--path",
            "code/digimon-engine/tests/cards_behavioral/bt13/mod.rs"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path code/digimon-engine/cards/bt13/BT13-060.yaml --path code/digimon-engine/tests/cards_behavioral/bt13/bt13_060.rs --path code/digimon-engine/tests/cards_behavioral/bt13/mod.rs",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [\n    \"BT13-060\"\n  ],\n  \"cards_behavioral_filter\": \"BT13-060\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [\n    \"cards_behavioral\",\n    \"dsl\"\n  ],\n  \"verbs\": []\n}"
        },
        "ran": true
      },
      "verbs": []
    },
    "sha": "708a0aa235d7a007b4574036593c9cab85f7e7e7",
    "touched": [
      "code/digimon-engine/cards/bt13/BT13-060.yaml",
      "code/digimon-engine/tests/cards_behavioral/bt13/bt13_060.rs",
      "code/digimon-engine/tests/cards_behavioral/bt13/mod.rs"
    ]
  },
  "merge_error": [
    "apply failed: git add -- code/digimon-engine/tests/cards_behavioral/bt13/mod.rs failed in D:\\cl-ds\\cl-pilot-data-squad-run: fatal: Unable to create 'C:/Users/james/Documents/digimon-deck-list-builder-1/.git/worktrees/cl-pilot-data-squad-run/index.lock': File exists.\n\nAnother git process seems to be running in this repository, e.g.\nan editor opened by 'git commit'. Please make sure all processes\nare terminated then try again. If it still fails, a git process\nmay have crashed in this repository earlier:\nremove the file manually to continue."
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT13-060#effect#2",
    "denominator": "compared 18 of 18 ours / 18 dcgo steps",
    "divergence": {
      "dcgo": "3",
      "field": "p1.security",
      "ours": "4",
      "step": 16
    },
    "first_divergence": "DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)",
    "ids": [
      "BT13-060#effect#2"
    ],
    "job_id": "exam-BT13-060-effect2",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)",
    "recorded": [
      "BT13-060#effect#2"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT13/BT13-060-effect2.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T141401Z_7e77baabcee24f66a26bbeb3a1044858.state.jsonl",
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT13/BT13-060-effect2.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT13-060#effect#2",
      "denominator": "compared 18 of 18 ours / 18 dcgo steps",
      "divergence": {
        "dcgo": "3",
        "field": "p1.security",
        "ours": "4",
        "step": 16
      },
      "first_divergence": "DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)",
      "ids": [
        "BT13-060#effect#2"
      ],
      "job_id": "exam-BT13-060-effect2",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)",
      "recorded": [
        "BT13-060#effect#2"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT13/BT13-060-effect2.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T141401Z_7e77baabcee24f66a26bbeb3a1044858.state.jsonl",
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)",
  "scenario_path": "qa/dcgo-exams/BT13/BT13-060-effect2.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT13/BT13-060-effect2.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT13/BT13-060-effect2.yaml": [
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "triage": null,
  "triage_first": {
    "attempt_id": "20261006T160630812683Z-pilot-data-squad-2a991c",
    "call": "ours_wrong",
    "citation": {
      "kind": "dcgo",
      "ref": "DCGO/Assets/Scripts/CardEffect/BT13/Green/BT13_060.cs:218"
    },
    "family": "claude",
    "reasoning": "DCGO's OnAllyAttack effect (BT13_060.cs:218-266) is mandatory. It counts opposing suspended Digimon and Tamers, divides by 2, and trashes that many security cards from the top. It matches the printed [When Attacking] text (data/card_bundles/BT13-060.md). The scenario (qa/dcgo-exams/BT13/BT13-060-effect2.yaml) suspends Biyomon and Tai Kamiya with the card's own [When Digivolving], so count = 2/2 = 1 at the attack declaration.\n\nDCGO's final state shows both cards gone from P1's security: security 5 to 3 and P1 trash [ST1-04, BT1-009]. That is the clause trashing the top card, BT1-009, then the security check destroying Dracomon ST1-04.\n\nI replayed our side sim-only with `exam_probe`. P1 security went 5 to 4 and trash holds only BT1-009. That is consistent with the clause never firing and the security check alone destroying the top card. Both opposing permanents were suspended in our projection, so the trigger condition was met.\n\nThe cause is in our spec. `code/digimon-engine/cards/_examples/BT13-060.yaml` (lines 49-52) deliberately omits the [When Attacking] clause (\"lose_count_bound ... OMITTED\"), and the `bt13` card JSON has no effect either. The clause is simply unimplemented, which breaks the no-approximations policy. The fix is to widen the DSL with a count-bounded security-trash step, \"for every 2 suspended Digimon/Tamers\", and author the effect."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: clause:BT13-060#effect#2\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: clause `BT13-060#effect#2` of Rosemon: Burst Mode (BT13-060): Trash the top card of your opponent\u2019s security stack for every 2 of your opponent's suspended Digimon and/or Tamers.\n- Scenario(s): `qa/dcgo-exams/BT13/BT13-060-effect2.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT13-060#effect#2\",\n  \"denominator\": \"compared 18 of 18 ours / 18 dcgo steps\",\n  \"divergence\": {\n    \"dcgo\": \"3\",\n    \"field\": \"p1.security\",\n    \"ours\": \"4\",\n    \"step\": 16\n  },\n  \"first_divergence\": \"DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)\",\n  \"ids\": [\n    \"BT13-060#effect#2\"\n  ],\n  \"job_id\": \"exam-BT13-060-effect2\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"DIVERGED at step 16 (compared 18 of 18 ours / 18 dcgo steps)\",\n  \"recorded\": [\n    \"BT13-060#effect#2\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT13/BT13-060-effect2.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T141401Z_7e77baabcee24f66a26bbeb3a1044858.state.jsonl\",\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT13-060.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO. `ours_wrong` unless the ruling\n  is misread (then say exactly which words).\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "3",
    "references": [
      "qa/dcgo-exams/BT13/BT13-060-effect2.yaml",
      "data/card_bundles/BT13-060.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT13\\Green\\BT13_060.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T141401Z_7e77baabcee24f66a26bbeb3a1044858.state.jsonl",
      "docs/digimon-rules/rules-index.json",
      "docs/digimon-rules/keyword-semantics.md"
    ]
  },
  "verdict": "diverged"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
