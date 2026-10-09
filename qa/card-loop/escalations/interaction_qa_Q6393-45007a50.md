---
item: interaction:qa:Q6393
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T22:07:25.383554Z
state_before: TERMINATION_CHECK
reason: families disagree: claude dcgo_quirk vs codex scenario_wrong
---

# Escalated: `interaction:qa:Q6393`

**Why:** families disagree: claude dcgo_quirk vs codex scenario_wrong

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `dcgo_quirk` (attempt `20261006T220151667346Z-pilot-three-musketeers-bc38f1`)

- Citation: {'kind': 'rule', 'ref': '14-2-1-3'}
- Reasoning:
> Our engine follows Q6393. I replayed the scenario sim-only with inspect_step 24, which shows our board after the revive pick. Impmon BT6-068 is on p0's field and p0.field is [BT6-068], so the expect_ruling assertion (at 23) holds on our side. The printed [Three Musketeers] appears only in Impmon's [On Play] effect text, and Q6393 says "X in its text" includes effects. DCGO agrees on the ruling: BT25_083.cs:423 filters with HasText("Three Musketeers"), which reads the effect text, and its recording shows Impmon revived (field [BT6-068], step 28). The only field divergence is p1.trash at step 24. Ours is [ST1-05, ST1-10], DCGO's is [ST1-05]. This is timing, not a rules difference. On the tied attack (Phoenixmon ST1-10 vs BeelStarmon, equal DP), both lose at the same time (14-2-1-3), so by the time LadyDevimon's [On Deletion] resolves, ST1-10 is already in the trash. DCGO trashes the security Digimon only after the revive resolves: its recording has ST1-10 in the trash by step 30, but not in the rows before. The revive outcome is identical, and only the trash-update timing differs. The "ours contradicts ruling" label does not match my replay of our side. The rest of the divergence is the trash-timing difference above, so I am treating that as the finding and not as a ruling violation.

### 2. codex: `scenario_wrong` (attempt `20261006T220526270042Z-pilot-three-musketeers-71f854`)

- Citation: {'kind': 'ruling', 'ref': 'qa:Q6393'}
- Reasoning:
> Q6393 includes effect text, so BT6-068 is a valid revival target. However, expect_ruling asserts revival too early: at: 23 is before the card-selection action at zero-based step 23; its resulting state is at: 24 (run.rs:227-234). Also, p0.field requires permanent objects, not card-ID strings (assertions.rs:103-104,120-138). Change the assertion to at: 24 with p0.field: [{card_id: BT6-068, dp: 2000, suspended: false, sources: []}]. Q6393 does not decide the separate ST1-10 trash-timing difference; DCGO's sidecar eventually shows ST1-10 in trash. Correct the ruling assertion and re-evaluate that difference separately. The requested exam_probe was blocked by the approval policy, so no fresh replay was obtained.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T154633307107Z-pilot-three-musketeers-287117` | classify_qa | claude | sonnet | accepted | 0.2451 |
| 2 | `20261006T154633307171Z-pilot-three-musketeers-7b9c45` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T154747264372Z-pilot-three-musketeers-11ef51` | encode_ruling | claude | sonnet | escalated | 0.2726 |
| 4 | `20261006T154810124706Z-pilot-three-musketeers-f83fc6` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T183515683910Z-pilot-three-musketeers-707f2a` | classify_qa | claude | sonnet | accepted | 0.2450 |
| 6 | `20261006T183515683967Z-pilot-three-musketeers-e9d3d5` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T183558696188Z-pilot-three-musketeers-48d4ef` | author_interaction | claude | sonnet | accepted | 1.0021 |
| 8 | `20261006T183925549764Z-pilot-three-musketeers-aa8224` | encode_ruling | claude | sonnet | accepted | 0.2655 |
| 9 | `20261006T183947322377Z-pilot-three-musketeers-d540da` | encode_ruling | codex | (default) | accepted | unpriced |
| 10 | `20261006T184340389359Z-pilot-three-musketeers-f65bb7` | triage | claude | sonnet | accepted | 0.6325 |
| 11 | `20261006T184456923566Z-pilot-three-musketeers-8a5b59` | triage | codex | (default) | escalated | unpriced |
| 12 | `20261006T210318437921Z-pilot-three-musketeers-ae22a5` | classify_qa | claude | sonnet | accepted | 0.2434 |
| 13 | `20261006T210318437980Z-pilot-three-musketeers-810e4c` | classify_qa | codex | (default) | accepted | unpriced |
| 14 | `20261006T210426714383Z-pilot-three-musketeers-106262` | encode_ruling | claude | sonnet | accepted | 0.2626 |
| 15 | `20261006T210445818001Z-pilot-three-musketeers-12690d` | encode_ruling | codex | (default) | accepted | unpriced |
| 16 | `20261006T214118340779Z-pilot-three-musketeers-227c6f` | triage | claude | sonnet | accepted | 0.7209 |
| 17 | `20261006T214515576623Z-pilot-three-musketeers-28edb0` | triage | codex | (default) | accepted | unpriced |
| 18 | `20261006T214736802846Z-pilot-three-musketeers-a7fbd1` | author_interaction | claude | sonnet | accepted | 0.3993 |
| 19 | `20261006T214832122249Z-pilot-three-musketeers-b7ae97` | encode_ruling | claude | sonnet | accepted | 0.2631 |
| 20 | `20261006T214853100745Z-pilot-three-musketeers-93c9b4` | encode_ruling | codex | (default) | accepted | unpriced |
| 21 | `20261006T215328279776Z-pilot-three-musketeers-5bc3e0` | triage | claude | sonnet | accepted | 0.5692 |
| 22 | `20261006T215506295484Z-pilot-three-musketeers-1a31ef` | author_interaction | claude | sonnet | accepted | 0.3774 |
| 23 | `20261006T215804171617Z-pilot-three-musketeers-ae8e9a` | encode_ruling | claude | sonnet | accepted | 0.2619 |
| 24 | `20261006T215921298215Z-pilot-three-musketeers-3018d5` | encode_ruling | codex | (default) | accepted | unpriced |
| 25 | `20261006T220151667346Z-pilot-three-musketeers-bc38f1` | triage | claude | sonnet | accepted | 0.6567 |
| 26 | `20261006T220526270042Z-pilot-three-musketeers-71f854` | triage | codex | (default) | escalated | unpriced |

## Item data

```json
{
  "author_attempt": "20261006T215506295484Z-pilot-three-musketeers-1a31ef",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
  "card_ids": [
    "BT25-083"
  ],
  "citation": null,
  "classification": {
    "agreed": false,
    "calls": {
      "claude": "behavioral",
      "codex": "textual"
    },
    "examined_clauses": [
      "BT25-083#effect#0",
      "BT25-083#inherited#0"
    ],
    "q_id": "Q6393"
  },
  "covers": [
    "BT25-083#inherited#0"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml": "qa/dcgo-exams/BT25/tm_bt25_083_q6393_pool.json"
  },
  "encode_attempts": [
    "20261006T215804171617Z-pilot-three-musketeers-ae8e9a",
    "20261006T215921298215Z-pilot-three-musketeers-3018d5"
  ],
  "encode_feedback": null,
  "escalation": null,
  "escalation_reason": null,
  "expect_ruling": {
    "assert": [
      {
        "at": 23,
        "that": {
          "p0.field": [
            "BT6-068"
          ]
        }
      }
    ],
    "q_id": "Q6393"
  },
  "history": [
    "20261006T154633307107Z-pilot-three-musketeers-287117",
    "20261006T154633307171Z-pilot-three-musketeers-7b9c45",
    "20261006T154747264372Z-pilot-three-musketeers-11ef51",
    "20261006T154810124706Z-pilot-three-musketeers-f83fc6",
    "20261006T183515683910Z-pilot-three-musketeers-707f2a",
    "20261006T183515683967Z-pilot-three-musketeers-e9d3d5",
    "20261006T183558696188Z-pilot-three-musketeers-48d4ef",
    "20261006T183925549764Z-pilot-three-musketeers-aa8224",
    "20261006T183947322377Z-pilot-three-musketeers-d540da",
    "20261006T184340389359Z-pilot-three-musketeers-f65bb7",
    "20261006T184456923566Z-pilot-three-musketeers-8a5b59",
    "20261006T210318437921Z-pilot-three-musketeers-ae22a5",
    "20261006T210318437980Z-pilot-three-musketeers-810e4c",
    "20261006T210426714383Z-pilot-three-musketeers-106262",
    "20261006T210445818001Z-pilot-three-musketeers-12690d",
    "20261006T214118340779Z-pilot-three-musketeers-227c6f",
    "20261006T214515576623Z-pilot-three-musketeers-28edb0",
    "20261006T214736802846Z-pilot-three-musketeers-a7fbd1",
    "20261006T214832122249Z-pilot-three-musketeers-b7ae97",
    "20261006T214853100745Z-pilot-three-musketeers-93c9b4",
    "20261006T215328279776Z-pilot-three-musketeers-5bc3e0",
    "20261006T215506295484Z-pilot-three-musketeers-1a31ef",
    "20261006T215804171617Z-pilot-three-musketeers-ae8e9a",
    "20261006T215921298215Z-pilot-three-musketeers-3018d5",
    "20261006T220151667346Z-pilot-three-musketeers-bc38f1"
  ],
  "merge": {
    "attempt_id": "20261006T215506295484Z-pilot-three-musketeers-1a31ef",
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
            "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "efa373d061f7d0863560d1c545412cb501dc5104",
    "touched": [
      "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml"
    ]
  },
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT25-083#inherited#0",
    "denominator": "compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)",
    "divergence": {
      "dcgo": "[ST1-05]",
      "field": "p1.trash",
      "ours": "[ST1-05, ST1-10]",
      "step": 24
    },
    "first_divergence": "DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
    "ids": [
      "qa:Q6393"
    ],
    "job_id": "exam-BT25-083-qa-Q6393",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "ours contradicts ruling qa:Q6393 (ours vs DCGO: diverge); DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
    "recorded": [
      "qa:Q6393"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T220000Z_09bdccbbefb44dea841a3255ab303c15.state.jsonl",
    "stall": null,
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT25-083#inherited#0",
      "denominator": "compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)",
      "divergence": {
        "dcgo": "[ST1-05]",
        "field": "p1.trash",
        "ours": "[ST1-05, ST1-10]",
        "step": 24
      },
      "first_divergence": "DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
      "ids": [
        "qa:Q6393"
      ],
      "job_id": "exam-BT25-083-qa-Q6393",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "ours contradicts ruling qa:Q6393 (ours vs DCGO: diverge); DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
      "recorded": [
        "qa:Q6393"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T220000Z_09bdccbbefb44dea841a3255ab303c15.state.jsonl",
      "stall": null,
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "ours contradicts ruling qa:Q6393 (ours vs DCGO: diverge); DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
  "ruling_block_written": [
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml"
  ],
  "ruling_contradicted": {
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml": [
      "RULING qa:Q6393 CONTRADICTED: at 23: p0.field expected - BT6-068 but our engine has []",
      "ruling qa:Q6393: ours CONTRADICTS the ruling (1 check(s), 1 failed)"
    ]
  },
  "scenario_path": "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml"
  ],
  "scenario_wrong_rounds": 2,
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml": [
      "step 17 expect.prompt 'SelectCountEffect' not asserted sim-side (kind EffectChoice has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "step 24 expect.prompt 'OptionalSkill' cannot be asserted sim-side -- our engine auto-resolved the prompt; DCGO will assert it strictly",
      "step 24 select answered no live prompt -- our engine auto-resolved it; the row is kept for the DCGO wire",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "termination": null,
  "triage": null,
  "triage_feedback": null,
  "triage_first": {
    "attempt_id": "20261006T220151667346Z-pilot-three-musketeers-bc38f1",
    "call": "dcgo_quirk",
    "citation": {
      "kind": "rule",
      "ref": "14-2-1-3"
    },
    "family": "claude",
    "reasoning": "Our engine follows Q6393. I replayed the scenario sim-only with inspect_step 24, which shows our board after the revive pick. Impmon BT6-068 is on p0's field and p0.field is [BT6-068], so the expect_ruling assertion (at 23) holds on our side. The printed [Three Musketeers] appears only in Impmon's [On Play] effect text, and Q6393 says \"X in its text\" includes effects. DCGO agrees on the ruling: BT25_083.cs:423 filters with HasText(\"Three Musketeers\"), which reads the effect text, and its recording shows Impmon revived (field [BT6-068], step 28). The only field divergence is p1.trash at step 24. Ours is [ST1-05, ST1-10], DCGO's is [ST1-05]. This is timing, not a rules difference. On the tied attack (Phoenixmon ST1-10 vs BeelStarmon, equal DP), both lose at the same time (14-2-1-3), so by the time LadyDevimon's [On Deletion] resolves, ST1-10 is already in the trash. DCGO trashes the security Digimon only after the revive resolves: its recording has ST1-10 in the trash by step 30, but not in the rows before. The revive outcome is identical, and only the trash-update timing differs. The \"ours contradicts ruling\" label does not match my replay of our side. The rest of the divergence is the trash-timing difference above, so I am treating that as the finding and not as a ruling violation."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q6393\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q6393` on BT25-083\n- Scenario(s): `qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT25-083#inherited#0\",\n  \"denominator\": \"compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)\",\n  \"divergence\": {\n    \"dcgo\": \"[ST1-05]\",\n    \"field\": \"p1.trash\",\n    \"ours\": \"[ST1-05, ST1-10]\",\n    \"step\": 24\n  },\n  \"first_divergence\": \"DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))\",\n  \"ids\": [\n    \"qa:Q6393\"\n  ],\n  \"job_id\": \"exam-BT25-083-qa-Q6393\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"ours contradicts ruling qa:Q6393 (ours vs DCGO: diverge); DIVERGED at step 24 (compared 25 of 26 ours / 26 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))\",\n  \"recorded\": [\n    \"qa:Q6393\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T220000Z_09bdccbbefb44dea841a3255ab303c15.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Purple\\BT25_083.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT25-083.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO -- also when `ours vs DCGO:\n  agree` (both engines can be wrong against the publisher). `ours_wrong`\n  unless the `expect_ruling:` misreads the answer or asserts what it does not\n  decide: then `scenario_wrong`, naming the words. Never `dcgo_quirk` for a\n  line on which DCGO agreed with us.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `scenario_wrong`: the exam itself misreads the card or the ruling -- it picks\n  the wrong card, asserts at the wrong step, or its `expect_ruling:` claims\n  something the answer does not decide -- so neither engine is being judged.\n  Say exactly what to change; the exam goes back to its author with your words.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "4",
    "references": [
      "qa/dcgo-exams/BT25/BT25-083-qa-Q6393.yaml",
      "data/card_bundles/BT25-083.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Purple\\BT25_083.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T220000Z_09bdccbbefb44dea841a3255ab303c15.state.jsonl",
      "docs/digimon-rules/rules-index.json",
      "docs/digimon-rules/keyword-semantics.md"
    ]
  },
  "triage_second": {
    "attempt_id": "20261006T214515576623Z-pilot-three-musketeers-28edb0",
    "call": "scenario_wrong",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q6393"
    },
    "family": "codex",
    "reasoning": "Q6393 confirms that effect text counts, so BT6-068 is eligible. However, expect_ruling asserts revival at 23, before the step-23 card selection executes: run.rs:227-234 assigns post-action projections index i+1. Move the assertion to at: 25, after revival and declining Impmon's On Play. Also replace p0.field: [BT6-068] with the projected permanent object [{card_id: BT6-068, dp: 2000, suspended: false, sources: []}]; assertions.rs compares complete objects, not card IDs. The ST1-10 trash timing difference does not establish a contradiction of Q6393 and needs separate evaluation. Replay inspection was unavailable because the tool required approval under policy never."
  },
  "verdict": "diverged"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
