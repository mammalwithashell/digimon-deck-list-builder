---
item: interaction:qa:Q6388
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T17:08:55.021603Z
state_before: FIX
reason: fix_engine by claude still needs substrate: `BREEDING-TRIGGER-DISPATCH-UNGATED` (engine: enqueue_from_breeding_permanent still dispatches every timing-triggered effect for breeding-area cards. Only EX7-051 is now gated, via a per-card on_field condition. Other cards with start-of-main-phase or similar effects and no on_field gate would still fire from the breeding area. A general engine fix would need a per-effect flag for effects that are meant to work in breeding, since some (Training, breeding-area effects) legitimately do.)
---

# Escalated: `interaction:qa:Q6388`

**Why:** fix_engine by claude still needs substrate: `BREEDING-TRIGGER-DISPATCH-UNGATED` (engine: enqueue_from_breeding_permanent still dispatches every timing-triggered effect for breeding-area cards. Only EX7-051 is now gated, via a per-card on_field condition. Other cards with start-of-main-phase or similar effects and no on_field gate would still fire from the breeding area. A general engine fix would need a per-effect flag for effects that are meant to work in breeding, since some (Training, breeding-area effects) legitimately do.)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

### 1. claude: `fix_engine` (attempt `20261006T165429002366Z-pilot-three-musketeers-134735`)

- Citation: {'kind': 'dcgo', 'ref': 'DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_051.cs:51'}
- Reasoning:
> NOT VERIFIED: no test result was observed, so test_result_lines is empty. The cargo test run hit its background time limit during the build and was killed. I did not run the new test at base to confirm it fails, and I did not run it with the fix to confirm it passes. Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex7_051` from the repo root before trusting this.
>
> Changes: (1) EX7-051.yaml: the start_of_your_main_phase condition is now all_of [on_field: true, count_gte{...}], mirroring DCGO IsExistOnBattleAreaDigimon (EX7_051.cs:51, :77) and the Q6388 ruling. (2) ex7_051.rs: new test ex7_051_start_of_main_does_not_fire_from_breeding_area, which places Sparrowmon in breeding with a Three Musketeers Option in hand and asserts no pending selection and an unchanged hand.
>
> No new DSL vocabulary or engine primitive was added: the on_field predicate already exists (BT21-093 uses it). This is a card-spec fix, and the general engine gap is listed under gaps. Not checked: that on_field evaluates correctly for the breeding sentinel handle in a clause-level condition (src/dsl_cards/predicate.rs:674-695 suggests it does), and that the exam scenario BT25-082-qa-Q6388 now reaches the Q6388 clause. EX7-051.yaml had no produced_by header, so none was added.

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T145418117970Z-pilot-three-musketeers-f9f2a8` | classify_qa | claude | sonnet | accepted | 0.2406 |
| 2 | `20261006T145418118016Z-pilot-three-musketeers-12f52c` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T145449952857Z-pilot-three-musketeers-eb7b83` | encode_ruling | claude | sonnet | escalated | 0.2739 |
| 4 | `20261006T145535595512Z-pilot-three-musketeers-f77f6c` | encode_ruling | codex | (default) | escalated | unpriced |
| 5 | `20261006T163535234466Z-pilot-three-musketeers-aa3444` | classify_qa | claude | sonnet | accepted | 0.2415 |
| 6 | `20261006T163535234545Z-pilot-three-musketeers-b0fd8e` | classify_qa | codex | (default) | accepted | unpriced |
| 7 | `20261006T163745972402Z-pilot-three-musketeers-93a263` | author_interaction | claude | sonnet | accepted | 1.1994 |
| 8 | `20261006T164257044515Z-pilot-three-musketeers-46db15` | encode_ruling | claude | sonnet | accepted | 0.3585 |
| 9 | `20261006T164359316323Z-pilot-three-musketeers-678931` | encode_ruling | codex | (default) | accepted | unpriced |
| 10 | `20261006T164520407852Z-pilot-three-musketeers-f176c7` | triage | claude | sonnet | accepted | 0.4618 |
| 11 | `20261006T165401976400Z-pilot-three-musketeers-f8b748` | fix_card | claude | sonnet | accepted | 0.3350 |
| 12 | `20261006T165429002366Z-pilot-three-musketeers-134735` | fix_engine | claude | sonnet | escalated | 1.3902 |

## Item data

```json
{
  "author_attempt": "20261006T163745972402Z-pilot-three-musketeers-93a263",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
  "card_ids": [
    "BT25-082"
  ],
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT25-082#effect#2"
    ],
    "q_id": "Q6388"
  },
  "covers": [
    "BT25-082#effect#2"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml": "qa/dcgo-exams/EX7/three_musketeers_pool.json"
  },
  "encode_attempts": [
    "20261006T164257044515Z-pilot-three-musketeers-46db15",
    "20261006T164359316323Z-pilot-three-musketeers-678931"
  ],
  "encode_feedback": null,
  "escalation": "interaction_qa_Q6388-e12b8ee3.md",
  "escalation_reason": "verifier disagrees: No, you can't. The effect doesn\u2019t activate in the breeding area.",
  "expect_ruling": {
    "assert": [
      {
        "at": 16,
        "that": {
          "p0.hand": [
            "BT25-085",
            "EX7-073",
            "BT25-083",
            "BT21-071",
            "BT21-074",
            "BT25-083",
            "BT25-082"
          ]
        }
      }
    ],
    "q_id": "Q6388"
  },
  "history": [
    "20261006T145418117970Z-pilot-three-musketeers-f9f2a8",
    "20261006T145418118016Z-pilot-three-musketeers-12f52c",
    "20261006T145449952857Z-pilot-three-musketeers-eb7b83",
    "20261006T145535595512Z-pilot-three-musketeers-f77f6c",
    "20261006T163535234466Z-pilot-three-musketeers-aa3444",
    "20261006T163535234545Z-pilot-three-musketeers-b0fd8e",
    "20261006T163745972402Z-pilot-three-musketeers-93a263",
    "20261006T164257044515Z-pilot-three-musketeers-46db15",
    "20261006T164359316323Z-pilot-three-musketeers-678931",
    "20261006T164520407852Z-pilot-three-musketeers-f176c7"
  ],
  "merge": {
    "attempt_id": "20261006T163745972402Z-pilot-three-musketeers-93a263",
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
            "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "eff74705453be4336098a53072ad3b3e9ca07472",
    "touched": [
      "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml"
    ]
  },
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT25-082#effect#2",
    "denominator": "compared 9 of 17 ours / 9 dcgo steps",
    "divergence": null,
    "first_divergence": "TRUNCATED, no divergence found (compared 9 of 17 ours / 9 dcgo steps)",
    "ids": [
      "qa:Q6388"
    ],
    "job_id": "exam-BT25-082-qa-Q6388",
    "job_outcome": "failed",
    "mismatch": {
      "asked": "main_phase",
      "expected": "OptionalSkill",
      "row": 9,
      "step": 9
    },
    "reason": "DCGO job failed: prompt mismatch: step 9 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it",
    "recorded": [],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T164436Z_07f5b3d3d2dc49e6991a7e998ff161e5.state.jsonl",
    "stall": null,
    "verdict": "unmeasured"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT25-082#effect#2",
      "denominator": "compared 9 of 17 ours / 9 dcgo steps",
      "divergence": null,
      "first_divergence": "TRUNCATED, no divergence found (compared 9 of 17 ours / 9 dcgo steps)",
      "ids": [
        "qa:Q6388"
      ],
      "job_id": "exam-BT25-082-qa-Q6388",
      "job_outcome": "failed",
      "mismatch": {
        "asked": "main_phase",
        "expected": "OptionalSkill",
        "row": 9,
        "step": 9
      },
      "reason": "DCGO job failed: prompt mismatch: step 9 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it",
      "recorded": [],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T164436Z_07f5b3d3d2dc49e6991a7e998ff161e5.state.jsonl",
      "stall": null,
      "verdict": "unmeasured"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": {
    "dcgo_asked": "main_phase",
    "dcgo_row": 9,
    "expected": "OptionalSkill",
    "explanation": "the scenario expected 'OptionalSkill', DCGO asked 'main_phase', and our engine's UnionZone { zones: UnionZoneSet(3) } (no unambiguous DCGO prompt) has no unambiguous DCGO prompt mapping",
    "ours": null,
    "ours_snapshot": {
      "candidates": [
        [
          2,
          "BT25-085"
        ]
      ],
      "pending_kind": "UnionZone { zones: UnionZoneSet(3) }",
      "pending_optional": true,
      "pending_prompt": "Place 1 Option card with the [Three Musketeers] trait from your hand or trash as 1 of your Digimon's bottom digivolution card",
      "step": 9
    },
    "route": "undetermined",
    "scenario": "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
    "scenario_step": 9,
    "step_mapping": "harness"
  },
  "prompt_route": "undetermined",
  "ruling_block_written": [
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml"
  ],
  "ruling_contradicted": null,
  "scenario_paths": [
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml": [
      "step 9 expect.prompt 'OptionalSkill' not asserted sim-side (kind UnionZone { zones: UnionZoneSet(3) } has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "step 11 expect.prompt 'SelectCountEffect' not asserted sim-side (kind EffectChoice has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "triage_first": {
    "attempt_id": "20261006T164520407852Z-pilot-three-musketeers-f176c7",
    "call": "ours_wrong",
    "citation": {
      "kind": "dcgo",
      "ref": "DCGO/Assets/Scripts/CardEffect/EX7/Purple/EX7_051.cs:51"
    },
    "family": "claude",
    "reasoning": "The line stops at step 9, before the BT25-082 clause that Q6388 asks about. The stop comes from Sparrowmon (EX7-051), which the scenario digivolved in the breeding area at step 5 and leaves there (the board is empty at step 9). I replayed our side with exam_probe (inspect_step 9). Our engine raises an optional pending selection (\"Place 1 Option card with the [Three Musketeers] trait from your hand or trash ...\", candidate BT25-085, kind UnionZone). That is EX7-051's [Start of Your Main Phase] effect firing from the breeding area. DCGO does not raise it. EX7_051.cs gates both of its effect paths with IsExistOnBattleAreaDigimon(card) (lines 51 and 77), and DCGO goes straight to main_phase. Q6388 gives the publisher's ruling for BT25-082, \"The effect doesn't activate in the breeding area\". On that reasoning, effects on a Digimon that is still in the breeding area do not activate, and DCGO's gate agrees. The unmapped prompt from our engine is therefore a real divergence: it activates a battle-area-only effect from the breeding area. The engine-side fix is to stop activation of timing-triggered effects, such as EX7-051's start-of-main-phase effect, for a card in the breeding area. I did not read the rules PDF, and the evidence for the rule is the Q6388 ruling plus DCGO's gate. The Q6388 clause itself (BT25-082#effect#2) is not yet measured, because the line never reached it."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q6388\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q6388` on BT25-082\n- Scenario(s): `qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT25-082#effect#2\",\n  \"denominator\": \"compared 9 of 17 ours / 9 dcgo steps\",\n  \"divergence\": null,\n  \"first_divergence\": \"TRUNCATED, no divergence found (compared 9 of 17 ours / 9 dcgo steps)\",\n  \"ids\": [\n    \"qa:Q6388\"\n  ],\n  \"job_id\": \"exam-BT25-082-qa-Q6388\",\n  \"job_outcome\": \"failed\",\n  \"mismatch\": {\n    \"asked\": \"main_phase\",\n    \"expected\": \"OptionalSkill\",\n    \"row\": 9,\n    \"step\": 9\n  },\n  \"reason\": \"DCGO job failed: prompt mismatch: step 9 expected prompt 'OptionalSkill' but DCGO asked 'main_phase' -- stopped before the line finished, with no divergence before it\",\n  \"recorded\": [],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T164436Z_07f5b3d3d2dc49e6991a7e998ff161e5.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"unmeasured\"\n}\n```\n- Prompt evidence: at scenario step 9 the scenario expected `OptionalSkill`, DCGO asked `main_phase`, and our engine asked `None` (the scenario expected 'OptionalSkill', DCGO asked 'main_phase', and our engine's UnionZone { zones: UnionZoneSet(3) } (no unambiguous DCGO prompt) has no unambiguous DCGO prompt mapping).\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Purple\\BT25_082.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT25-082.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO. `ours_wrong` unless the ruling\n  is misread (then say exactly which words).\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "3",
    "references": [
      "qa/dcgo-exams/BT25/BT25-082-qa-Q6388.yaml",
      "data/card_bundles/BT25-082.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT25\\Purple\\BT25_082.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T164436Z_07f5b3d3d2dc49e6991a7e998ff161e5.state.jsonl",
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
