---
item: interaction:qa:Q2669
run_id: pilot-data-squad
escalated_at: 2026-10-06T21:43:46.942276Z
state_before: TRIAGE
reason: the exam was judged wrong 3 times in a row and re-authored each time without converging; last: The ruling (qa:Q2669) says Ukkomon's trigger activates during the breeding phase, and the main phase comes only after all effects have resolved. Our engine and DCGO agree on the line: 12 of 12 steps compared, no field divergence. The only failure is the exam's own `expect_ruling` assert, `at: 9, tha
---

# Escalated: `interaction:qa:Q2669`

**Why:** the exam was judged wrong 3 times in a row and re-authored each time without converging; last: The ruling (qa:Q2669) says Ukkomon's trigger activates during the breeding phase, and the main phase comes only after all effects have resolved. Our engine and DCGO agree on the line: 12 of 12 steps compared, no field divergence. The only failure is the exam's own `expect_ruling` assert, `at: 9, tha

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `scenario_wrong` (attempt `20261006T214127360269Z-pilot-data-squad-4e58df`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> The ruling (qa:Q2669) says Ukkomon's trigger activates during the breeding phase, and the main phase comes only after all effects have resolved. Our engine and DCGO agree on the line: 12 of 12 steps compared, no field divergence. The only failure is the exam's own `expect_ruling` assert, `at: 9, that: {phase: breeding}`. I replayed the line with exam_probe sim-only and `inspect_step: 9`. At that step the projection reports `phase: "SelectReveal"`, turn 5, with pending_kind Reveal and candidates BT25-090, ST23-12 and BT25-057. The engine's `GamePhase` enum is a prompt-state label, so it reads `SelectReveal` while the trigger's reveal selection is pending (enums.rs:123, 166). A `phase: breeding` assert can therefore never hold at a step where the effect is parked on a selection. It does not show the engine leaving the breeding phase, and it would not discriminate even if the engine did resolve the trigger in the main phase. So the assertion measures the wrong observable, and neither engine is being judged. I did not find evidence that our engine resolves the trigger outside the breeding phase. Both engines offered the same reveal and hatch prompts, and the post-move prompt is `breeding_action` in both.
>
> Fix for the author: drop the `phase` assert at step 9 and assert the ruling through something discriminating. Options are the step-8 expectation `prompt: breeding_action` after the move (that the next prompt is still a breeding-phase action), or an assert after the hatch is accepted that the next prompt is `main_phase` and a `phase` read after the effect has fully resolved. This ruling also decides only the timing relative to the main phase, so do not assert hand or board outcomes as the ruling's content.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T143801776153Z-pilot-data-squad-ae8615` | classify_qa | claude | sonnet | accepted | 0.2374 |
| 2 | `20261006T143801776221Z-pilot-data-squad-c23f42` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T143925767429Z-pilot-data-squad-d69577` | encode_ruling | claude | sonnet | escalated | 0.2618 |
| 4 | `20261006T143945749640Z-pilot-data-squad-b72fb2` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T160630740545Z-pilot-data-squad-6390bb` | classify_qa | claude | sonnet | accepted | 0.2384 |
| 6 | `20261006T160630740665Z-pilot-data-squad-b150e5` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T164919195147Z-pilot-data-squad-ea0642` | classify_qa | claude | sonnet | accepted | 0.2379 |
| 8 | `20261006T164919195275Z-pilot-data-squad-bf92e1` | classify_qa | codex | (default) | accepted | unpriced |
| 9 | `20261006T164959400420Z-pilot-data-squad-d75605` | author_interaction | claude | sonnet | accepted | 0.3703 |
| 10 | `20261006T165042356908Z-pilot-data-squad-4ebcaa` | encode_ruling | claude | sonnet | gate_failed | 0.2573 |
| 11 | `20261006T165102022269Z-pilot-data-squad-c47a34` | encode_ruling | codex | (default) | accepted | unpriced |
| 12 | `20261006T165120345769Z-pilot-data-squad-3cd6d7` | encode_ruling | claude | sonnet | accepted | 0.2603 |
| 13 | `20261006T165152396316Z-pilot-data-squad-0087d2` | encode_ruling | codex | (default) | accepted | unpriced |
| 14 | `20261006T165245776316Z-pilot-data-squad-cd6277` | triage | claude | sonnet | escalated | 0.3655 |
| 15 | `20261006T181226687622Z-pilot-data-squad-b501a6` | triage | claude | sonnet | accepted | 0.4008 |
| 16 | `20261006T181322693667Z-pilot-data-squad-0bb498` | author_interaction | claude | sonnet | accepted | 0.3516 |
| 17 | `20261006T181409336433Z-pilot-data-squad-b0bb3f` | encode_ruling | claude | sonnet | gate_failed | 0.3230 |
| 18 | `20261006T181454886297Z-pilot-data-squad-06ee00` | encode_ruling | codex | (default) | accepted | unpriced |
| 19 | `20261006T181604605633Z-pilot-data-squad-919412` | encode_ruling | claude | sonnet | gate_failed | 0.2615 |
| 20 | `20261006T181637874642Z-pilot-data-squad-b05b65` | encode_ruling | codex | (default) | accepted | unpriced |
| 21 | `20261006T190752335446Z-pilot-data-squad-33fee2` | classify_qa | claude | sonnet | accepted | 0.2395 |
| 22 | `20261006T190752335559Z-pilot-data-squad-b7268d` | classify_qa | codex | (default) | accepted | unpriced |
| 23 | `20261006T190834765732Z-pilot-data-squad-01a169` | encode_ruling | claude | sonnet | accepted | 0.3232 |
| 24 | `20261006T190922303253Z-pilot-data-squad-87c908` | encode_ruling | codex | (default) | accepted | unpriced |
| 25 | `20261006T204956329894Z-pilot-data-squad-c546c6` | triage | claude | sonnet | accepted | 0.4322 |
| 26 | `20261006T205103434839Z-pilot-data-squad-61330d` | author_interaction | claude | sonnet | accepted | 0.3732 |
| 27 | `20261006T205138915192Z-pilot-data-squad-d7219d` | encode_ruling | claude | sonnet | accepted | 0.2623 |
| 28 | `20261006T205159432926Z-pilot-data-squad-ed3c6d` | encode_ruling | codex | (default) | accepted | unpriced |
| 29 | `20261006T211231419222Z-pilot-data-squad-845831` | triage | claude | sonnet | accepted | 0.4094 |
| 30 | `20261006T211315863653Z-pilot-data-squad-096130` | author_interaction | claude | sonnet | accepted | 0.3448 |
| 31 | `20261006T211341876056Z-pilot-data-squad-0cc968` | encode_ruling | claude | sonnet | gate_failed | 0.2585 |
| 32 | `20261006T211400735670Z-pilot-data-squad-f61b4c` | encode_ruling | codex | (default) | accepted | unpriced |
| 33 | `20261006T213528114005Z-pilot-data-squad-419a5c` | encode_ruling | claude | sonnet | accepted | 0.3310 |
| 34 | `20261006T213629585190Z-pilot-data-squad-236ef5` | encode_ruling | codex | (default) | accepted | unpriced |
| 35 | `20261006T214127360269Z-pilot-data-squad-4e58df` | triage | claude | sonnet | escalated | 0.4206 |

## Item data

```json
{
  "author_attempt": "20261006T211315863653Z-pilot-data-squad-096130",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
  "card_ids": [
    "BT16-082"
  ],
  "citation": null,
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT16-082#effect#0"
    ],
    "q_id": "Q2669"
  },
  "covers": [
    "BT16-082#effect#0"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml": "qa/dcgo-exams/ST23/glowing_dawn_pool.json"
  },
  "encode_attempts": [
    "20261006T213528114005Z-pilot-data-squad-419a5c",
    "20261006T213629585190Z-pilot-data-squad-236ef5"
  ],
  "encode_feedback": null,
  "escalation": null,
  "escalation_reason": null,
  "expect_ruling": {
    "assert": [
      {
        "at": 9,
        "that": {
          "phase": "breeding"
        }
      }
    ],
    "q_id": "Q2669"
  },
  "history": [
    "20261006T143801776153Z-pilot-data-squad-ae8615",
    "20261006T143801776221Z-pilot-data-squad-c23f42",
    "20261006T143925767429Z-pilot-data-squad-d69577",
    "20261006T143945749640Z-pilot-data-squad-b72fb2",
    "20261006T160630740545Z-pilot-data-squad-6390bb",
    "20261006T160630740665Z-pilot-data-squad-b150e5",
    "20261006T164919195147Z-pilot-data-squad-ea0642",
    "20261006T164919195275Z-pilot-data-squad-bf92e1",
    "20261006T164959400420Z-pilot-data-squad-d75605",
    "20261006T165042356908Z-pilot-data-squad-4ebcaa",
    "20261006T165102022269Z-pilot-data-squad-c47a34",
    "20261006T165120345769Z-pilot-data-squad-3cd6d7",
    "20261006T165152396316Z-pilot-data-squad-0087d2",
    "20261006T165245776316Z-pilot-data-squad-cd6277",
    "20261006T181226687622Z-pilot-data-squad-b501a6",
    "20261006T181322693667Z-pilot-data-squad-0bb498",
    "20261006T181409336433Z-pilot-data-squad-b0bb3f",
    "20261006T181454886297Z-pilot-data-squad-06ee00",
    "20261006T181604605633Z-pilot-data-squad-919412",
    "20261006T181637874642Z-pilot-data-squad-b05b65",
    "20261006T190752335446Z-pilot-data-squad-33fee2",
    "20261006T190752335559Z-pilot-data-squad-b7268d",
    "20261006T190834765732Z-pilot-data-squad-01a169",
    "20261006T190922303253Z-pilot-data-squad-87c908",
    "20261006T204956329894Z-pilot-data-squad-c546c6",
    "20261006T205103434839Z-pilot-data-squad-61330d",
    "20261006T205138915192Z-pilot-data-squad-d7219d",
    "20261006T205159432926Z-pilot-data-squad-ed3c6d",
    "20261006T211231419222Z-pilot-data-squad-845831",
    "20261006T211315863653Z-pilot-data-squad-096130",
    "20261006T211341876056Z-pilot-data-squad-0cc968",
    "20261006T211400735670Z-pilot-data-squad-f61b4c",
    "20261006T213528114005Z-pilot-data-squad-419a5c",
    "20261006T213629585190Z-pilot-data-squad-236ef5"
  ],
  "merge": {
    "attempt_id": "20261006T211315863653Z-pilot-data-squad-096130",
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
            "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
          "cwd": "D:\\cl-ds\\cl-pilot-data-squad-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "69ed354b89abffa6fef4aa36514439ab2627f159",
    "touched": [
      "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml"
    ]
  },
  "merge_error": [
    "git apply --3way failed (rc 128): fatal: Unable to create 'C:/Users/james/Documents/digimon-deck-list-builder-1/.git/worktrees/cl-pilot-data-squad-run/index.lock': File exists.\n\nAnother git process seems to be running in this repository, e.g.\nan editor opened by 'git commit'. Please make sure all processes\nare terminated then try again. If it still fails, a git process\nmay have crashed in this repository earlier:\nremove the file manually to continue."
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT16-082#effect#0",
    "denominator": "compared 12 of 12 ours / 12 dcgo steps",
    "divergence": null,
    "first_divergence": null,
    "ids": [
      "qa:Q2669"
    ],
    "job_id": "exam-BT16-082-qa-Q2669",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "ours contradicts ruling qa:Q2669 (ours vs DCGO: agree); CLEAN (compared 12 of 12 ours / 12 dcgo steps)",
    "recorded": [
      "qa:Q2669"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T213812Z_8aef3880eb9344dca5ff0278b6d88085.state.jsonl",
    "stall": null,
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT16-082#effect#0",
      "denominator": "compared 12 of 12 ours / 12 dcgo steps",
      "divergence": null,
      "first_divergence": null,
      "ids": [
        "qa:Q2669"
      ],
      "job_id": "exam-BT16-082-qa-Q2669",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "ours contradicts ruling qa:Q2669 (ours vs DCGO: agree); CLEAN (compared 12 of 12 ours / 12 dcgo steps)",
      "recorded": [
        "qa:Q2669"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T213812Z_8aef3880eb9344dca5ff0278b6d88085.state.jsonl",
      "stall": null,
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "ours contradicts ruling qa:Q2669 (ours vs DCGO: agree); CLEAN (compared 12 of 12 ours / 12 dcgo steps)",
  "ruling_block_written": [
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml"
  ],
  "ruling_contradicted": {
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml": [
      "RULING qa:Q2669 CONTRADICTED: at 9: phase is not observable while a selection is open (our engine reports `SelectReveal` there, not the turn phase) -- expected breeding; assert a zone or a count at this step, or the phase at a step with no selection open",
      "ruling qa:Q2669: ours CONTRADICTS the ruling (1 check(s), 1 failed)"
    ]
  },
  "scenario_path": "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml"
  ],
  "scenario_wrong_rounds": 2,
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml": [
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "termination": null,
  "triage": null,
  "triage_feedback": null,
  "triage_first": {
    "attempt_id": "20261006T211231419222Z-pilot-data-squad-845831",
    "call": "scenario_wrong",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q2669"
    },
    "family": "claude",
    "reasoning": "Q2669 says the trigger activates during the breeding phase, and the main phase comes only after all effects have activated. The exam's `expect_ruling` asserts `at: 11 \u2192 phase: main`. I probed our side with `inspect_step: 11`. That is the state before step 11, the `select: {yes: true}` answer to \"You may hatch in your breeding area\". The hatch prompt is the last part of the Ukkomon (BT16-082) trigger's own resolution. Ours is parked in `SelectEffectChoice` there, a Select* phase that stands for the interrupted Breeding phase. The ruling says the phase must still be breeding at that point, so asserting `main` misreads the answer. The ruling is not violated by our engine. The line also stops right after the hatch answer, so it never reaches the point where main begins. The `at: 9` assert fails for another reason. Our engine is in Select* phases (SelectReveal at 9, SelectPermutation at 10) during selection prompts, and the guide treats `phase` at a selection boundary as representation, not rules state. Fix: drop the `at: 11 phase main` assert. Append a step after the hatch answer, for example the actor's `pass` at `main_phase`, which needs the hatch resolution to finish and so shows main is reached. Then assert `phase: main` at that new later index. Keep a breeding-phase assert only at a non-selection boundary, such as `at: 8` before the move or the state immediately after the move resolves, and do not assert `breeding` at Select* prompts. The line did not run on DCGO's side beyond the mismatch (12 of 12 steps compared, clean), so DCGO and ours agree."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q2669\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q2669` on BT16-082\n- Scenario(s): `qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT16-082#effect#0\",\n  \"denominator\": \"compared 12 of 12 ours / 12 dcgo steps\",\n  \"divergence\": null,\n  \"first_divergence\": null,\n  \"ids\": [\n    \"qa:Q2669\"\n  ],\n  \"job_id\": \"exam-BT16-082-qa-Q2669\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"ours contradicts ruling qa:Q2669 (ours vs DCGO: agree); CLEAN (compared 12 of 12 ours / 12 dcgo steps)\",\n  \"recorded\": [\n    \"qa:Q2669\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T205422Z_ed158ca39f634e9b8e6ea619099e1d7b.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT16\\White\\BT16_082.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT16-082.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO -- also when `ours vs DCGO:\n  agree` (both engines can be wrong against the publisher). `ours_wrong`\n  unless the `expect_ruling:` misreads the answer or asserts what it does not\n  decide: then `scenario_wrong`, naming the words. Never `dcgo_quirk` for a\n  line on which DCGO agreed with us.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `scenario_wrong`: the exam itself misreads the card or the ruling -- it picks\n  the wrong card, asserts at the wrong step, or its `expect_ruling:` claims\n  something the answer does not decide -- so neither engine is being judged.\n  Say exactly what to change; the exam goes back to its author with your words.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "4",
    "references": [
      "qa/dcgo-exams/BT16/BT16-082-qa-Q2669.yaml",
      "data/card_bundles/BT16-082.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT16\\White\\BT16_082.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T205422Z_ed158ca39f634e9b8e6ea619099e1d7b.state.jsonl",
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

## Owner decision (2026-10-10)
Approved: replace the `phase: breeding` assert. It can never hold while the reveal prompt is open, and it would not discriminate even if it could. Nothing observable changes between the end of the breeding phase and the start of the main phase, so a zone or count check can't witness Q2669 either.
The witness needs ordering. Put a [Start of Your Main Phase] trigger on the board (e.g. Tomoro & Kyo ST23-13), then show that Ukkomon's (BT16-082) effect fully resolves before that trigger is offered.
This needs re-authoring and an oracle run, so the escalation stays open until then.
