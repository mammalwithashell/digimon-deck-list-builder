---
item: interaction:qa:Q6252
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T21:03:15.390690Z
state_before: TRIAGE
reason: the exam was judged wrong 3 times in a row and re-authored each time without converging; last: Our engine is not contradicting Q6252. The exam's `expect_ruling` asserts at the wrong step. Q6252 says a card with "X in its text" includes cards whose effects or digivolution requirements contain X. Scopemon BT21-071 has no [TS] trait and matches only through its "[Three Musketeers]" text, so it m
---

# Escalated: `interaction:qa:Q6252`

**Why:** the exam was judged wrong 3 times in a row and re-authored each time without converging; last: Our engine is not contradicting Q6252. The exam's `expect_ruling` asserts at the wrong step. Q6252 says a card with "X in its text" includes cards whose effects or digivolution requirements contain X. Scopemon BT21-071 has no [TS] trait and matches only through its "[Three Musketeers]" text, so it m

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `scenario_wrong` (attempt `20261006T210125094612Z-pilot-three-musketeers-526585`)

- Citation: **no citation given** (a terminating call needs one, design D5)
- Reasoning:
> Our engine is not contradicting Q6252. The exam's `expect_ruling` asserts at the wrong step. Q6252 says a card with "X in its text" includes cards whose effects or digivolution requirements contain X. Scopemon BT21-071 has no [TS] trait and matches only through its "[Three Musketeers]" text, so it must be a legal pick for the clause. Our engine does offer it, and the hand ends up exactly as the exam expects.
>
> The mismatch is in the `at:` index. A projection at step N is the board BEFORE step N runs, and `exam_probe inspect_step` documents that reading. Step 14 is the Scopemon pick (steps are 0-indexed: 0 hatch, 1 pass, 2 pass, 3 pass, 4 digivolve, 5 pass, 6 pass, 7 pass, 8 move, 9 play, 10 zone, 11 P-180, 12 own.field.0, 13 yes, 14 Scopemon, 15 decline, 16 pass).
>
> At `at: 14` the Scopemon pick and its digivolve draw have not happened, so p0.hand still holds BT21-071 and lacks BT25-058. That is [BT24-081, BT21-071, BT25-078 x4], which is not the asserted [BT24-081, BT25-078 x4, BT25-058]. `check_rows` compares hands as multisets (`assertions.rs`), so ordering is not the cause.
>
> A sim-only probe with `inspect_step: 15`, the board after the pick, shows p0.hand = [BT24-081, BT25-058, BT25-078 x4], memory -4, and the field BT21-071 over BT25-005, EX7-008 and P-180. That equals the asserted hand.
>
> Fix: change `at: 14` to `at: 15` in `qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml`. Keep the hand list. The asserted hand still discriminates, because without Scopemon leaving the hand (a text-only match) it would still contain BT21-071. I did not rerun the corrected assertion, so confirm it passes.
>
> Both engines already agree on the prompt sequence (compared 15 of 17 ours and 17 DCGO steps, CLEAN), so DCGO is not at fault.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T144601438121Z-pilot-three-musketeers-8c0e10` | classify_qa | claude | sonnet | accepted | 0.2395 |
| 2 | `20261006T144601438173Z-pilot-three-musketeers-a2737a` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T144648771981Z-pilot-three-musketeers-be1435` | encode_ruling | claude | sonnet | escalated | 0.2788 |
| 4 | `20261006T144711198902Z-pilot-three-musketeers-911899` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T162553524643Z-pilot-three-musketeers-512931` | classify_qa | claude | sonnet | accepted | 0.2412 |
| 6 | `20261006T162553524722Z-pilot-three-musketeers-cfa8f3` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T163535469261Z-pilot-three-musketeers-92d8a3` | author_interaction | claude | sonnet | accepted | 0.7733 |
| 8 | `20261006T164141272280Z-pilot-three-musketeers-b47536` | encode_ruling | claude | sonnet | accepted | 0.2665 |
| 9 | `20261006T164217759321Z-pilot-three-musketeers-96a065` | encode_ruling | codex | (default) | accepted | unpriced |
| 10 | `20261006T182022997176Z-pilot-three-musketeers-8fc77c` | classify_qa | claude | sonnet | accepted | 0.2420 |
| 11 | `20261006T182022997231Z-pilot-three-musketeers-0f9304` | classify_qa | codex | (default) | accepted | unpriced |
| 12 | `20261006T182112323756Z-pilot-three-musketeers-f36bea` | encode_ruling | claude | sonnet | accepted | 0.2620 |
| 13 | `20261006T182130635963Z-pilot-three-musketeers-e7ee4c` | encode_ruling | codex | (default) | accepted | unpriced |
| 14 | `20261006T182408484113Z-pilot-three-musketeers-f3d20c` | triage | claude | sonnet | accepted | 0.6700 |
| 15 | `20261006T182533502792Z-pilot-three-musketeers-a5c2ee` | author_interaction | claude | sonnet | accepted | 0.4599 |
| 16 | `20261006T182623919644Z-pilot-three-musketeers-ea5caa` | encode_ruling | claude | sonnet | accepted | 0.2730 |
| 17 | `20261006T182649524340Z-pilot-three-musketeers-2cbdb6` | encode_ruling | codex | (default) | accepted | unpriced |
| 18 | `20261006T190947100270Z-pilot-three-musketeers-32bcdd` | classify_qa | claude | sonnet | accepted | 0.2412 |
| 19 | `20261006T190947100325Z-pilot-three-musketeers-6b7e6a` | classify_qa | codex | (default) | accepted | unpriced |
| 20 | `20261006T191040840134Z-pilot-three-musketeers-527ed1` | encode_ruling | claude | sonnet | accepted | 0.2670 |
| 21 | `20261006T191122627790Z-pilot-three-musketeers-4bb7e0` | encode_ruling | codex | (default) | accepted | unpriced |
| 22 | `20261006T193834646294Z-pilot-three-musketeers-eb634b` | triage | claude | sonnet | accepted | 0.3805 |
| 23 | `20261006T194023475672Z-pilot-three-musketeers-a4b03a` | author_interaction | claude | sonnet | accepted | 0.3978 |
| 24 | `20261006T204954845893Z-pilot-three-musketeers-aca13a` | encode_ruling | claude | sonnet | accepted | 0.3385 |
| 25 | `20261006T205029559880Z-pilot-three-musketeers-f3cbdf` | encode_ruling | codex | (default) | accepted | unpriced |
| 26 | `20261006T205150291822Z-pilot-three-musketeers-4b14a4` | triage | claude | sonnet | accepted | 0.5763 |
| 27 | `20261006T205355914713Z-pilot-three-musketeers-1fcf61` | author_interaction | claude | sonnet | accepted | 0.4452 |
| 28 | `20261006T205509120296Z-pilot-three-musketeers-4ffcff` | encode_ruling | claude | sonnet | accepted | 0.2722 |
| 29 | `20261006T205540245281Z-pilot-three-musketeers-4e8eef` | encode_ruling | codex | (default) | accepted | unpriced |
| 30 | `20261006T205730463375Z-pilot-three-musketeers-d6a5bf` | triage | claude | sonnet | accepted | 0.3602 |
| 31 | `20261006T205838850296Z-pilot-three-musketeers-c3f968` | author_interaction | claude | sonnet | accepted | 0.4161 |
| 32 | `20261006T205921709310Z-pilot-three-musketeers-80d745` | encode_ruling | claude | sonnet | accepted | 0.2679 |
| 33 | `20261006T205942130839Z-pilot-three-musketeers-b43bab` | encode_ruling | codex | (default) | accepted | unpriced |
| 34 | `20261006T210125094612Z-pilot-three-musketeers-526585` | triage | claude | sonnet | escalated | 0.5281 |

## Item data

```json
{
  "author_attempt": "20261006T205838850296Z-pilot-three-musketeers-c3f968",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
  "card_ids": [
    "BT25-005"
  ],
  "citation": null,
  "classification": {
    "agreed": false,
    "calls": {
      "claude": "behavioral",
      "codex": "textual"
    },
    "examined_clauses": [
      "BT25-005#inherited#0"
    ],
    "q_id": "Q6252"
  },
  "covers": [
    "BT25-005#inherited#0"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml": "qa/dcgo-exams/BT25/tm_bt25_005_pool.json"
  },
  "encode_attempts": [
    "20261006T205921709310Z-pilot-three-musketeers-80d745",
    "20261006T205942130839Z-pilot-three-musketeers-b43bab"
  ],
  "encode_feedback": null,
  "escalation": null,
  "escalation_reason": null,
  "expect_ruling": {
    "assert": [
      {
        "at": 14,
        "that": {
          "p0.hand": [
            "BT24-081",
            "BT25-078",
            "BT25-078",
            "BT25-078",
            "BT25-078",
            "BT25-058"
          ]
        }
      }
    ],
    "q_id": "Q6252"
  },
  "history": [
    "20261006T144601438121Z-pilot-three-musketeers-8c0e10",
    "20261006T144601438173Z-pilot-three-musketeers-a2737a",
    "20261006T144648771981Z-pilot-three-musketeers-be1435",
    "20261006T144711198902Z-pilot-three-musketeers-911899",
    "20261006T162553524643Z-pilot-three-musketeers-512931",
    "20261006T162553524722Z-pilot-three-musketeers-cfa8f3",
    "20261006T163535469261Z-pilot-three-musketeers-92d8a3",
    "20261006T164141272280Z-pilot-three-musketeers-b47536",
    "20261006T164217759321Z-pilot-three-musketeers-96a065",
    "20261006T182022997176Z-pilot-three-musketeers-8fc77c",
    "20261006T182022997231Z-pilot-three-musketeers-0f9304",
    "20261006T182112323756Z-pilot-three-musketeers-f36bea",
    "20261006T182130635963Z-pilot-three-musketeers-e7ee4c",
    "20261006T182408484113Z-pilot-three-musketeers-f3d20c",
    "20261006T182533502792Z-pilot-three-musketeers-a5c2ee",
    "20261006T182623919644Z-pilot-three-musketeers-ea5caa",
    "20261006T182649524340Z-pilot-three-musketeers-2cbdb6",
    "20261006T190947100270Z-pilot-three-musketeers-32bcdd",
    "20261006T190947100325Z-pilot-three-musketeers-6b7e6a",
    "20261006T191040840134Z-pilot-three-musketeers-527ed1",
    "20261006T191122627790Z-pilot-three-musketeers-4bb7e0",
    "20261006T193834646294Z-pilot-three-musketeers-eb634b",
    "20261006T194023475672Z-pilot-three-musketeers-a4b03a",
    "20261006T204954845893Z-pilot-three-musketeers-aca13a",
    "20261006T205029559880Z-pilot-three-musketeers-f3cbdf",
    "20261006T205150291822Z-pilot-three-musketeers-4b14a4",
    "20261006T205355914713Z-pilot-three-musketeers-1fcf61",
    "20261006T205509120296Z-pilot-three-musketeers-4ffcff",
    "20261006T205540245281Z-pilot-three-musketeers-4e8eef",
    "20261006T205730463375Z-pilot-three-musketeers-d6a5bf",
    "20261006T205838850296Z-pilot-three-musketeers-c3f968",
    "20261006T205921709310Z-pilot-three-musketeers-80d745",
    "20261006T205942130839Z-pilot-three-musketeers-b43bab"
  ],
  "merge": {
    "attempt_id": "20261006T205838850296Z-pilot-three-musketeers-c3f968",
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
            "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "a6fe1b75bf8a050e1b24c96ed1cf103d1e3a4d09",
    "touched": [
      "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml"
    ]
  },
  "merge_error": [
    "manifest paths have uncommitted changes in the run tree: qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml -- commit or discard them first"
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT25-005#inherited#0",
    "denominator": "compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable)",
    "divergence": null,
    "first_divergence": null,
    "ids": [
      "qa:Q6252"
    ],
    "job_id": "exam-BT25-005-qa-Q6252",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "ours contradicts ruling qa:Q6252 (ours vs DCGO: agree); CLEAN (compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))",
    "recorded": [
      "qa:Q6252"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T210010Z_c5a02ef495a241a1bbb418c23f9ea383.state.jsonl",
    "stall": null,
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT25-005#inherited#0",
      "denominator": "compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable)",
      "divergence": null,
      "first_divergence": null,
      "ids": [
        "qa:Q6252"
      ],
      "job_id": "exam-BT25-005-qa-Q6252",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "ours contradicts ruling qa:Q6252 (ours vs DCGO: agree); CLEAN (compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))",
      "recorded": [
        "qa:Q6252"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T210010Z_c5a02ef495a241a1bbb418c23f9ea383.state.jsonl",
      "stall": null,
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "ours contradicts ruling qa:Q6252 (ours vs DCGO: agree); CLEAN (compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))",
  "ruling_block_written": [
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml"
  ],
  "ruling_contradicted": {
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml": [
      "RULING qa:Q6252 CONTRADICTED: at 14: p0.hand expected - BT24-081 - BT25-078 - BT25-078 - BT25-078 - BT25-078 - BT25-058 but our engine has - BT21-071 - BT24-081 - BT25-078 - BT25-078 - BT25-078 - BT25-078",
      "ruling qa:Q6252: ours CONTRADICTS the ruling (1 check(s), 1 failed)"
    ]
  },
  "scenario_path": "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml"
  ],
  "scenario_wrong_rounds": 2,
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml": [
      "step 11 expect.prompt 'SelectHandEffect' not asserted sim-side (kind UnionZone { zones: UnionZoneSet(3) } has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "termination": null,
  "triage": null,
  "triage_feedback": null,
  "triage_first": {
    "attempt_id": "20261006T205730463375Z-pilot-three-musketeers-d6a5bf",
    "call": "scenario_wrong",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q6252"
    },
    "family": "claude",
    "reasoning": "Our engine does not contradict Q6252; the exam's assertion is mis-indexed. Q6252 says a card with \"X in its text\" includes cards with X in their name, traits, effects, inherited effects, (Rule) or digivolution requirements. Scopemon BT21-071 has [Three Musketeers] in its text, so it must be an eligible pick for BT25-005's clause. Ours offers it: an exam_probe with inspect_step 14 shows a pending optional SelectHand with candidates [[0,\"BT21-071\"]] and the prompt \"Digivolve this Digimon into a Digimon card with [Three Musketeers] in its text or the [TS] trait from your hand, with the cost reduced by 2\". The board is memory -4, EX7-008 on the field with sources [BT25-005, P-180], and hand [BT21-071, BT24-081, BT25-078 x4]. That is the state BEFORE step 14, the Scopemon pick. The `expect_ruling` assert `at: 14` expects hand [BT24-081, BT25-078 x4, BT25-058]. That is the state AFTER the pick: Scopemon has left the hand and the digivolve draw BT25-058 has arrived. The assert therefore reads the hand one step too early, sees Scopemon still in hand and no BT25-058, and reports a false contradiction. Both engines agree (ours vs DCGO: agree, clean compare). Fix: change `at: 14` to the index of the post-pick state (`at: 15`, assuming `at` means \"before step N\", as inspect_step does). Keep the hand list unchanged. Also confirm the assert's index convention against the exam docs. The ruling does not decide hand contents at an arbitrary step, so the assert should pin only the post-pick state."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q6252\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q6252` on BT25-005\n- Scenario(s): `qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT25-005#inherited#0\",\n  \"denominator\": \"compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable)\",\n  \"divergence\": null,\n  \"first_divergence\": null,\n  \"ids\": [\n    \"qa:Q6252\"\n  ],\n  \"job_id\": \"exam-BT25-005-qa-Q6252\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"ours contradicts ruling qa:Q6252 (ours vs DCGO: agree); CLEAN (compared 15 of 17 ours / 17 dcgo steps (2 sim-only row(s) with no DCGO prompt + 2 DCGO intermediate row(s) not comparable))\",\n  \"recorded\": [\n    \"qa:Q6252\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T205609Z_ac7c84589412440cab1591ea3a1cad8a.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Black\\BT25_005.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT25-005.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO -- also when `ours vs DCGO:\n  agree` (both engines can be wrong against the publisher). `ours_wrong`\n  unless the `expect_ruling:` misreads the answer or asserts what it does not\n  decide: then `scenario_wrong`, naming the words. Never `dcgo_quirk` for a\n  line on which DCGO agreed with us.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `scenario_wrong`: the exam itself misreads the card or the ruling -- it picks\n  the wrong card, asserts at the wrong step, or its `expect_ruling:` claims\n  something the answer does not decide -- so neither engine is being judged.\n  Say exactly what to change; the exam goes back to its author with your words.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "4",
    "references": [
      "qa/dcgo-exams/BT25/BT25-005-qa-Q6252.yaml",
      "data/card_bundles/BT25-005.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Black\\BT25_005.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T205609Z_ac7c84589412440cab1591ea3a1cad8a.state.jsonl",
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
