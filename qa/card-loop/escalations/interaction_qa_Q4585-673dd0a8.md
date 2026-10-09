---
item: interaction:qa:Q4585
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T21:33:55.795437Z
state_before: FIX
reason: attempt cap: fix_card 2/2 spent; the last merge failed: scoped suite failed (CARGO_TARGET_DIR=D:/cargo-target\cl-pilot-three-musketeers-eng CARGO_TARGET_DIR_PINNED=1 RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8, rc 101): warning: unused `Result` that must be used --> code\digimon-engine\tests\cards_behavioral\p\p_151.rs:567:5 | 567 | runner.auto_resolve(); | ^^^^^^^^^^^^^^^^^^^^^ | = note: this `Result` may be an `Err` variant, which should be handled help: use `let _ = ...` to ignore the resulting value | 567 | let _ = runner.auto_resolve(); | +++++++ warning: unused `Result` that must be used --> code\digimon-engine\tests\cards_behavioral\p\p_169.rs:553:5 | 553 | runner.auto_resolve(); | ^^^^^^^^^^^^^^^^^^^^^ | = note: this `Result` may be an `Err` variant, which should be handled help: use `let _ = ...` to ignore the resulting value | 553 | let _ = runner.auto_resolve(); | +++++++ warning: unused `Result` that must be used --> code\digimon-engine\tests\cards_behavioral\st12\st12_12.rs:56:5 | 56 | runner.auto_resolve(); | ^^^^^^^^^^^^^^^^^^^^^ | = note: this `Result` may be an `Err` variant, which should be handled help: use `let _ = ...` to ignore the resulting value | 56 | let _ = runner.auto_resolve(); | +++++++ warning: `digimon-engine` (test "cards_behavioral") generated 118 warnings (7 duplicates) (run `cargo fix --test "cards_behavioral" -p digimon-engine` to apply 48 suggestions) Finished `test` profile [optimized + debuginfo] target(s) in 15m 28s Running tests\cards_behavioral\main.rs (D:/cargo-target\cl-pilot-three-musketeers-eng\debug\deps\cards_behavioral-7a0f9073ad31bfef.exe) error: test failed, to rerun pass `--test cards_behavioral`
---

# Escalated: `interaction:qa:Q4585`

**Why:** attempt cap: fix_card 2/2 spent; the last merge failed: scoped suite failed (CARGO_TARGET_DIR=D:/cargo-target\cl-pilot-three-musketeers-eng CARGO_TARGET_DIR_PINNED=1 RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8, rc 101): warning: unused `Result` that must be used
   --> code\digimon-engine\tests\cards_behavioral\p\p_151.rs:567:5
    |
567 |     runner.auto_resolve();
    |     ^^^^^^^^^^^^^^^^^^^^^
    |
    = note: this `Result` may be an `Err` variant, which should be handled
help: use `let _ = ...` to ignore the resulting value
    |
567 |     let _ = runner.auto_resolve();
    |     +++++++

warning: unused `Result` that must be used
   --> code\digimon-engine\tests\cards_behavioral\p\p_169.rs:553:5
    |
553 |     runner.auto_resolve();
    |     ^^^^^^^^^^^^^^^^^^^^^
    |
    = note: this `Result` may be an `Err` variant, which should be handled
help: use `let _ = ...` to ignore the resulting value
    |
553 |     let _ = runner.auto_resolve();
    |     +++++++

warning: unused `Result` that must be used
  --> code\digimon-engine\tests\cards_behavioral\st12\st12_12.rs:56:5
   |
56 |     runner.auto_resolve();
   |     ^^^^^^^^^^^^^^^^^^^^^
   |
   = note: this `Result` may be an `Err` variant, which should be handled
help: use `let _ = ...` to ignore the resulting value
   |
56 |     let _ = runner.auto_resolve();
   |     +++++++

warning: `digimon-engine` (test "cards_behavioral") generated 118 warnings (7 duplicates) (run `cargo fix --test "cards_behavioral" -p digimon-engine` to apply 48 suggestions)
    Finished `test` profile [optimized + debuginfo] target(s) in 15m 28s
     Running tests\cards_behavioral\main.rs (D:/cargo-target\cl-pilot-three-musketeers-eng\debug\deps\cards_behavioral-7a0f9073ad31bfef.exe)
error: test failed, to rerun pass `--test cards_behavioral`

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T142753733441Z-pilot-three-musketeers-04e455` | classify_qa | claude | sonnet | accepted | 0.2472 |
| 2 | `20261006T142753733493Z-pilot-three-musketeers-905832` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T142835073217Z-pilot-three-musketeers-51a1a5` | encode_ruling | claude | sonnet | escalated | 0.2679 |
| 4 | `20261006T142902985971Z-pilot-three-musketeers-c43bcf` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T160630995112Z-pilot-three-musketeers-c02269` | classify_qa | claude | sonnet | accepted | 0.2468 |
| 6 | `20261006T160630995295Z-pilot-three-musketeers-9702c1` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T160729661629Z-pilot-three-musketeers-d3179d` | author_interaction | claude | sonnet | accepted | 1.1926 |
| 8 | `20261006T161028124856Z-pilot-three-musketeers-5255aa` | encode_ruling | claude | sonnet | gate_failed | 0.2692 |
| 9 | `20261006T161050455967Z-pilot-three-musketeers-e2e111` | encode_ruling | codex | (default) | accepted | unpriced |
| 10 | `20261006T161152679697Z-pilot-three-musketeers-d8f511` | encode_ruling | claude | sonnet | accepted | 0.2666 |
| 11 | `20261006T161212538565Z-pilot-three-musketeers-d4b32f` | encode_ruling | codex | (default) | accepted | unpriced |
| 12 | `20261006T161241772226Z-pilot-three-musketeers-66a039` | author_interaction | claude | sonnet | gate_failed | 0.5325 |
| 13 | `20261006T161401965248Z-pilot-three-musketeers-6a4c3a` | author_interaction | claude | sonnet | gate_failed | 0.6834 |
| 14 | `20261006T164930093838Z-pilot-three-musketeers-348428` | classify_qa | claude | sonnet | accepted | 0.2435 |
| 15 | `20261006T164930093858Z-pilot-three-musketeers-b4d3e4` | classify_qa | codex | (default) | accepted | unpriced |
| 16 | `20261006T165014940036Z-pilot-three-musketeers-d0c03f` | author_interaction | claude | sonnet | accepted | 0.7991 |
| 17 | `20261006T165351160208Z-pilot-three-musketeers-775107` | triage | claude | sonnet | accepted | 0.2908 |
| 18 | `20261006T165514106618Z-pilot-three-musketeers-a83702` | fix_engine | codex | (default) | error | unpriced |
| 19 | `20261006T181804287409Z-pilot-three-musketeers-4ae4d1` | fix_card | claude | sonnet | gate_failed | 1.3311 |
| 20 | `20261006T190756805342Z-pilot-three-musketeers-617643` | classify_qa | claude | sonnet | accepted | 0.2456 |
| 21 | `20261006T190756805508Z-pilot-three-musketeers-828b79` | classify_qa | codex | (default) | accepted | unpriced |
| 22 | `20261006T190903494600Z-pilot-three-musketeers-993150` | encode_ruling | claude | sonnet | accepted | 0.2659 |
| 23 | `20261006T190926188052Z-pilot-three-musketeers-fe7994` | encode_ruling | codex | (default) | accepted | unpriced |
| 24 | `20261006T191422527616Z-pilot-three-musketeers-cff10b` | triage | claude | sonnet | accepted | 0.3630 |
| 25 | `20261006T191537900205Z-pilot-three-musketeers-bffee6` | fix_engine | codex | (default) | error | unpriced |
| 26 | `20261006T204955441646Z-pilot-three-musketeers-97244b` | fix_card | claude | sonnet | accepted | 0.6166 |
| 27 | `20261006T205100399863Z-pilot-three-musketeers-ee6f33` | fix_engine | codex | (default) | gate_failed | unpriced |

## Item data

```json
{
  "author_attempt": null,
  "author_family": null,
  "author_stage": null,
  "base_scenario": "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
  "card_ids": [
    "BT21-074"
  ],
  "citation": null,
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [],
    "q_id": "Q4585"
  },
  "covers": [
    "BT21-074#inherited#0"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml": "qa/dcgo-exams/BT21/tm_bt21_q4585_pool.json"
  },
  "encode_attempts": [
    "20261006T190903494600Z-pilot-three-musketeers-993150",
    "20261006T190926188052Z-pilot-three-musketeers-fe7994"
  ],
  "encode_feedback": null,
  "escalation": null,
  "escalation_reason": null,
  "expect_ruling": {
    "assert": [
      {
        "at": 23,
        "that": {
          "p0.trash": [
            "BT21-054"
          ]
        }
      }
    ],
    "q_id": "Q4585"
  },
  "history": [
    "20261006T142753733441Z-pilot-three-musketeers-04e455",
    "20261006T142753733493Z-pilot-three-musketeers-905832",
    "20261006T142835073217Z-pilot-three-musketeers-51a1a5",
    "20261006T142902985971Z-pilot-three-musketeers-c43bcf",
    "20261006T160630995112Z-pilot-three-musketeers-c02269",
    "20261006T160630995295Z-pilot-three-musketeers-9702c1",
    "20261006T160729661629Z-pilot-three-musketeers-d3179d",
    "20261006T161028124856Z-pilot-three-musketeers-5255aa",
    "20261006T161050455967Z-pilot-three-musketeers-e2e111",
    "20261006T161152679697Z-pilot-three-musketeers-d8f511",
    "20261006T161212538565Z-pilot-three-musketeers-d4b32f",
    "20261006T164930093838Z-pilot-three-musketeers-348428",
    "20261006T164930093858Z-pilot-three-musketeers-b4d3e4",
    "20261006T165014940036Z-pilot-three-musketeers-d0c03f",
    "20261006T165351160208Z-pilot-three-musketeers-775107",
    "20261006T165514106618Z-pilot-three-musketeers-a83702",
    "20261006T190756805342Z-pilot-three-musketeers-617643",
    "20261006T190756805508Z-pilot-three-musketeers-828b79",
    "20261006T190903494600Z-pilot-three-musketeers-993150",
    "20261006T190926188052Z-pilot-three-musketeers-fe7994",
    "20261006T191422527616Z-pilot-three-musketeers-cff10b",
    "20261006T191537900205Z-pilot-three-musketeers-bffee6"
  ],
  "merge": {
    "attempt_id": "20261006T165014940036Z-pilot-three-musketeers-d0c03f",
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
            "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "aee5ff908772e2e0f0f38fc03cf3b9dfbb006e63",
    "touched": [
      "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml"
    ]
  },
  "merge_error": [
    "scoped suite failed (CARGO_TARGET_DIR=D:/cargo-target\\cl-pilot-three-musketeers-eng CARGO_TARGET_DIR_PINNED=1 RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8, rc 101): warning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_151.rs:567:5\n    |\n567 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n567 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n   --> code\\digimon-engine\\tests\\cards_behavioral\\p\\p_169.rs:553:5\n    |\n553 |     runner.auto_resolve();\n    |     ^^^^^^^^^^^^^^^^^^^^^\n    |\n    = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n    |\n553 |     let _ = runner.auto_resolve();\n    |     +++++++\n\nwarning: unused `Result` that must be used\n  --> code\\digimon-engine\\tests\\cards_behavioral\\st12\\st12_12.rs:56:5\n   |\n56 |     runner.auto_resolve();\n   |     ^^^^^^^^^^^^^^^^^^^^^\n   |\n   = note: this `Result` may be an `Err` variant, which should be handled\nhelp: use `let _ = ...` to ignore the resulting value\n   |\n56 |     let _ = runner.auto_resolve();\n   |     +++++++\n\nwarning: `digimon-engine` (test \"cards_behavioral\") generated 118 warnings (7 duplicates) (run `cargo fix --test \"cards_behavioral\" -p digimon-engine` to apply 48 suggestions)\n    Finished `test` profile [optimized + debuginfo] target(s) in 15m 28s\n     Running tests\\cards_behavioral\\main.rs (D:/cargo-target\\cl-pilot-three-musketeers-eng\\debug\\deps\\cards_behavioral-7a0f9073ad31bfef.exe)\nerror: test failed, to rerun pass `--test cards_behavioral`"
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT21-074#inherited#0",
    "denominator": "compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable)",
    "divergence": {
      "dcgo": "[BT21-054]",
      "field": "p0.trash",
      "ours": "[]",
      "step": 28
    },
    "first_divergence": "DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))",
    "ids": [
      "qa:Q4585"
    ],
    "job_id": "exam-BT21-074-qa-Q4585",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "ours contradicts ruling qa:Q4585 (ours vs DCGO: diverge); DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))",
    "recorded": [
      "qa:Q4585"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T191121Z_9c61ea58bf5044aebd644ba34574d9b2.state.jsonl",
    "stall": null,
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT21-074#inherited#0",
      "denominator": "compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable)",
      "divergence": {
        "dcgo": "[BT21-054]",
        "field": "p0.trash",
        "ours": "[]",
        "step": 28
      },
      "first_divergence": "DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))",
      "ids": [
        "qa:Q4585"
      ],
      "job_id": "exam-BT21-074-qa-Q4585",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "ours contradicts ruling qa:Q4585 (ours vs DCGO: diverge); DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))",
      "recorded": [
        "qa:Q4585"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T191121Z_9c61ea58bf5044aebd644ba34574d9b2.state.jsonl",
      "stall": null,
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": null,
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "ours contradicts ruling qa:Q4585 (ours vs DCGO: diverge); DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))",
  "ruling_block_written": null,
  "ruling_contradicted": {
    "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml": [
      "RULING qa:Q4585 CONTRADICTED: at 23: p0.trash expected - BT21-054 but our engine has []",
      "ruling qa:Q4585: ours CONTRADICTS the ruling (1 check(s), 1 failed)"
    ]
  },
  "scenario_path": "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml": [
      "step 24 `choice: \"Bottom of deck\"` is branch 1 of [0: \"Top of deck\" | 1: \"Bottom of deck\"] on our EffectChoice prompt 'Place the remaining cards on the top or bottom of the deck' (sim-only; DCGO's zone/branch menu is its own row)",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "termination": null,
  "triage": null,
  "triage_feedback": null,
  "triage_first": {
    "attempt_id": "20261006T191422527616Z-pilot-three-musketeers-cff10b",
    "call": "ours_wrong",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q4585"
    },
    "family": "claude",
    "reasoning": "Official ruling Q4585 says that when a Digimon with BT21-054 Shotmon linked digivolves into EX7-048 Gundramon and no longer meets Shotmon's link requirement, Shotmon is trashed at rule-check timing. In the scenario Satellamon (BT21-074) has Shotmon linked and digivolves into Gundramon (Mega / Machine / Three Musketeers, not [Appmon]), which is the ruling's exact setup. The oracle result at step 28 shows DCGO with p0.trash = [BT21-054] and our engine with p0.trash = []. DCGO therefore follows the publisher's answer and our engine contradicts it. I did not run exam_probe, so I did not replay our prompt or board at the divergent step. The ruling and the oracle diff are enough. The expect_ruling asserts p0.trash: [BT21-054], which is what the answer decides, and I see no misreading in the scenario. Our engine fails to run the rule-check that trashes a link card whose link requirement the host stops meeting after a digivolve, so the gate should fix that."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q4585\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q4585` on BT21-074\n- Scenario(s): `qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT21-074#inherited#0\",\n  \"denominator\": \"compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable)\",\n  \"divergence\": {\n    \"dcgo\": \"[BT21-054]\",\n    \"field\": \"p0.trash\",\n    \"ours\": \"[]\",\n    \"step\": 28\n  },\n  \"first_divergence\": \"DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))\",\n  \"ids\": [\n    \"qa:Q4585\"\n  ],\n  \"job_id\": \"exam-BT21-074-qa-Q4585\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"ours contradicts ruling qa:Q4585 (ours vs DCGO: diverge); DIVERGED at step 28 (compared 24 of 29 ours / 27 dcgo steps (5 sim-only row(s) with no DCGO prompt + 3 DCGO intermediate row(s) not comparable))\",\n  \"recorded\": [\n    \"qa:Q4585\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T191121Z_9c61ea58bf5044aebd644ba34574d9b2.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT21\\Purple\\BT21_074.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT21-074.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO -- also when `ours vs DCGO:\n  agree` (both engines can be wrong against the publisher). `ours_wrong`\n  unless the `expect_ruling:` misreads the answer or asserts what it does not\n  decide: then `scenario_wrong`, naming the words. Never `dcgo_quirk` for a\n  line on which DCGO agreed with us.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `scenario_wrong`: the exam itself misreads the card or the ruling -- it picks\n  the wrong card, asserts at the wrong step, or its `expect_ruling:` claims\n  something the answer does not decide -- so neither engine is being judged.\n  Say exactly what to change; the exam goes back to its author with your words.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "4",
    "references": [
      "qa/dcgo-exams/BT21/BT21-074-qa-Q4585.yaml",
      "data/card_bundles/BT21-074.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT21\\Purple\\BT21_074.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T191121Z_9c61ea58bf5044aebd644ba34574d9b2.state.jsonl",
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
