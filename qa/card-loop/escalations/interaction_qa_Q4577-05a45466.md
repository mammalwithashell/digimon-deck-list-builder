---
item: interaction:qa:Q4577
run_id: pilot-three-musketeers
escalated_at: 2026-10-06T19:36:50.991101Z
state_before: AUTHORING
reason: attempt cap: author_interaction 3/3 spent (state AUTHORING)
---

# Escalated: `interaction:qa:Q4577`

**Why:** attempt cap: author_interaction 3/3 spent (state AUTHORING)

This item is **not adjudicated**: it is terminal for the run but blocks readiness until a human resolves it (design D4/D5).

## Arguments

No model arguments were recorded: the driver escalated this item itself (attempt cap, budget, or a missing second family).

## Attempt history (oldest first)

| # | attempt | stage | family | model | outcome | cost USD |
|---|---|---|---|---|---|---|
| 1 | `20261006T134839371258Z-pilot-three-musketeers-885ede` | classify_qa | claude | sonnet | accepted | 0.3140 |
| 2 | `20261006T134839371435Z-pilot-three-musketeers-0bf0eb` | classify_qa | codex | (default) | accepted | unpriced |
| 3 | `20261006T135031675461Z-pilot-three-musketeers-ea64da` | encode_ruling | claude | sonnet | accepted | 0.3318 |
| 4 | `20261006T135053635578Z-pilot-three-musketeers-f906e3` | encode_ruling | codex | (default) | accepted | unpriced |
| 5 | `20261006T135109582840Z-pilot-three-musketeers-df9a64` | author_interaction | claude | sonnet | accepted | 0.4096 |
| 6 | `20261006T135224269260Z-pilot-three-musketeers-9fcd43` | author_interaction | claude | sonnet | gate_failed | 0.4668 |
| 7 | `20261006T141214370951Z-pilot-three-musketeers-2fae5b` | author_interaction | claude | sonnet | accepted | 0.7039 |
| 8 | `20261006T153150290603Z-pilot-three-musketeers-4359f2` | classify_qa | claude | sonnet | accepted | 0.2438 |
| 9 | `20261006T153150290785Z-pilot-three-musketeers-fa454b` | classify_qa | codex | (default) | accepted | unpriced |
| 10 | `20261006T153234148951Z-pilot-three-musketeers-766f54` | encode_ruling | claude | sonnet | accepted | 0.2640 |
| 11 | `20261006T153256525989Z-pilot-three-musketeers-42b340` | encode_ruling | codex | (default) | accepted | unpriced |
| 12 | `20261006T153314961120Z-pilot-three-musketeers-517aa7` | author_interaction | claude | sonnet | accepted | 0.4524 |
| 13 | `20261006T153410919997Z-pilot-three-musketeers-12947f` | author_interaction | claude | sonnet | gate_failed | 0.4415 |
| 14 | `20261006T153444588010Z-pilot-three-musketeers-08d233` | author_interaction | claude | sonnet | accepted | 0.5524 |
| 15 | `20261006T160630995875Z-pilot-three-musketeers-f654fe` | classify_qa | claude | sonnet | accepted | 0.2435 |
| 16 | `20261006T160630995931Z-pilot-three-musketeers-313660` | classify_qa | codex | (default) | accepted | unpriced |
| 17 | `20261006T160732550874Z-pilot-three-musketeers-c568b5` | author_interaction | claude | sonnet | accepted | 0.6138 |
| 18 | `20261006T160850031588Z-pilot-three-musketeers-dd6b6e` | author_interaction | claude | sonnet | gate_failed | 0.3869 |
| 19 | `20261006T160923020647Z-pilot-three-musketeers-0661bd` | author_interaction | claude | sonnet | gate_failed | 0.3552 |
| 20 | `20261006T164930093451Z-pilot-three-musketeers-a1fa80` | classify_qa | claude | sonnet | accepted | 0.2431 |
| 21 | `20261006T164930093595Z-pilot-three-musketeers-3192ed` | classify_qa | codex | (default) | accepted | unpriced |
| 22 | `20261006T165011862477Z-pilot-three-musketeers-bc8156` | author_interaction | claude | sonnet | accepted | 0.4284 |
| 23 | `20261006T165145625619Z-pilot-three-musketeers-c42f4f` | triage | codex | (default) | accepted | unpriced |
| 24 | `20261006T165318770778Z-pilot-three-musketeers-d18865` | triage | claude | sonnet | escalated | 0.6674 |
| 25 | `20261006T181804290114Z-pilot-three-musketeers-0b7dd8` | triage | codex | (default) | escalated | unpriced |
| 26 | `20261006T190756806326Z-pilot-three-musketeers-f6e32d` | classify_qa | claude | sonnet | accepted | 0.2444 |
| 27 | `20261006T190756806350Z-pilot-three-musketeers-fe0b95` | classify_qa | codex | (default) | accepted | unpriced |
| 28 | `20261006T190858894249Z-pilot-three-musketeers-c6be06` | encode_ruling | claude | sonnet | accepted | 0.2632 |
| 29 | `20261006T190928066851Z-pilot-three-musketeers-a1ce9f` | encode_ruling | codex | (default) | accepted | unpriced |
| 30 | `20261006T191120644634Z-pilot-three-musketeers-83dd4e` | triage | codex | (default) | accepted | unpriced |
| 31 | `20261006T191435665176Z-pilot-three-musketeers-091027` | author_interaction | claude | sonnet | accepted | 0.3691 |
| 32 | `20261006T191537414271Z-pilot-three-musketeers-8fe618` | encode_ruling | claude | sonnet | accepted | 0.2567 |
| 33 | `20261006T191556417116Z-pilot-three-musketeers-76eda8` | encode_ruling | codex | (default) | accepted | unpriced |
| 34 | `20261006T191711367243Z-pilot-three-musketeers-92097c` | triage | codex | (default) | accepted | unpriced |
| 35 | `20261006T191940113013Z-pilot-three-musketeers-85d018` | author_interaction | claude | sonnet | accepted | 0.3569 |
| 36 | `20261006T192038824024Z-pilot-three-musketeers-69c6b4` | encode_ruling | claude | sonnet | accepted | 0.2556 |
| 37 | `20261006T192153864828Z-pilot-three-musketeers-9d274f` | encode_ruling | codex | (default) | accepted | unpriced |
| 38 | `20261006T192442010371Z-pilot-three-musketeers-f01fec` | triage | codex | (default) | accepted | unpriced |
| 39 | `20261006T192809856883Z-pilot-three-musketeers-5d31d2` | author_interaction | claude | sonnet | accepted | 0.3528 |
| 40 | `20261006T192859283333Z-pilot-three-musketeers-ddcd00` | encode_ruling | claude | sonnet | accepted | 0.2605 |
| 41 | `20261006T192924313232Z-pilot-three-musketeers-bb8a03` | encode_ruling | codex | (default) | accepted | unpriced |
| 42 | `20261006T193438391879Z-pilot-three-musketeers-2b204f` | triage | codex | (default) | accepted | unpriced |

## Item data

```json
{
  "author_attempt": "20261006T192809856883Z-pilot-three-musketeers-5d31d2",
  "author_family": "claude",
  "author_stage": "author_interaction",
  "base_scenario": "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
  "card_ids": [
    "BT21-071"
  ],
  "citation": null,
  "classification": {
    "agreed": true,
    "calls": {
      "claude": "behavioral",
      "codex": "behavioral"
    },
    "examined_clauses": [
      "BT21-071#effect#0"
    ],
    "q_id": "Q4577"
  },
  "covers": [
    "BT21-071#effect#0"
  ],
  "deck_books": {
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml": "qa/dcgo-exams/BT21/tm_bt21_071_pool.json"
  },
  "encode_attempts": [
    "20261006T192859283333Z-pilot-three-musketeers-ddcd00",
    "20261006T192924313232Z-pilot-three-musketeers-bb8a03"
  ],
  "encode_feedback": null,
  "escalation": null,
  "escalation_reason": null,
  "expect_ruling": null,
  "history": [
    "20261006T134839371258Z-pilot-three-musketeers-885ede",
    "20261006T134839371435Z-pilot-three-musketeers-0bf0eb",
    "20261006T135031675461Z-pilot-three-musketeers-ea64da",
    "20261006T135053635578Z-pilot-three-musketeers-f906e3",
    "20261006T135109582840Z-pilot-three-musketeers-df9a64",
    "20261006T141214370951Z-pilot-three-musketeers-2fae5b",
    "20261006T153150290603Z-pilot-three-musketeers-4359f2",
    "20261006T153150290785Z-pilot-three-musketeers-fa454b",
    "20261006T153234148951Z-pilot-three-musketeers-766f54",
    "20261006T153256525989Z-pilot-three-musketeers-42b340",
    "20261006T153314961120Z-pilot-three-musketeers-517aa7",
    "20261006T153444588010Z-pilot-three-musketeers-08d233",
    "20261006T160630995875Z-pilot-three-musketeers-f654fe",
    "20261006T160630995931Z-pilot-three-musketeers-313660",
    "20261006T160732550874Z-pilot-three-musketeers-c568b5",
    "20261006T164930093451Z-pilot-three-musketeers-a1fa80",
    "20261006T164930093595Z-pilot-three-musketeers-3192ed",
    "20261006T165011862477Z-pilot-three-musketeers-bc8156",
    "20261006T165145625619Z-pilot-three-musketeers-c42f4f",
    "20261006T165318770778Z-pilot-three-musketeers-d18865",
    "20261006T181804290114Z-pilot-three-musketeers-0b7dd8",
    "20261006T190756806326Z-pilot-three-musketeers-f6e32d",
    "20261006T190756806350Z-pilot-three-musketeers-fe0b95",
    "20261006T190858894249Z-pilot-three-musketeers-c6be06",
    "20261006T190928066851Z-pilot-three-musketeers-a1ce9f",
    "20261006T191120644634Z-pilot-three-musketeers-83dd4e",
    "20261006T191435665176Z-pilot-three-musketeers-091027",
    "20261006T191537414271Z-pilot-three-musketeers-8fe618",
    "20261006T191556417116Z-pilot-three-musketeers-76eda8",
    "20261006T191711367243Z-pilot-three-musketeers-92097c",
    "20261006T191940113013Z-pilot-three-musketeers-85d018",
    "20261006T192038824024Z-pilot-three-musketeers-69c6b4",
    "20261006T192153864828Z-pilot-three-musketeers-9d274f",
    "20261006T192442010371Z-pilot-three-musketeers-f01fec",
    "20261006T192809856883Z-pilot-three-musketeers-5d31d2",
    "20261006T192859283333Z-pilot-three-musketeers-ddcd00",
    "20261006T192924313232Z-pilot-three-musketeers-bb8a03",
    "20261006T193438391879Z-pilot-three-musketeers-2b204f"
  ],
  "merge": {
    "attempt_id": "20261006T192809856883Z-pilot-three-musketeers-5d31d2",
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
            "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml"
          ],
          "command": "C:\\Users\\james\\AppData\\Local\\Microsoft\\WindowsApps\\PythonSoftwareFoundation.Python.3.13_qbz5n2kfra8p0\\python.exe code/tools/impact_scope.py --json --path qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
          "cwd": "D:\\cl-tm\\cl-pilot-three-musketeers-run",
          "rc": 0,
          "tail": "{\n  \"cards\": [],\n  \"cards_behavioral_filter\": \"\",\n  \"full_suite_required\": false,\n  \"reasons\": [],\n  \"side_binaries\": [],\n  \"verbs\": []\n}"
        },
        "ran": false
      },
      "verbs": []
    },
    "sha": "6e41c18d33dafedeab421d028a082802d4c80de4",
    "touched": [
      "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml"
    ]
  },
  "merge_error": [
    "manifest paths have uncommitted changes in the run tree: qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml -- commit or discard them first"
  ],
  "oracle": {
    "backfill_note": null,
    "backfilled": false,
    "clause": "BT21-071#effect#0",
    "denominator": "compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)",
    "divergence": null,
    "first_divergence": null,
    "ids": [
      "qa:Q4577"
    ],
    "job_id": "exam-BT21-071-qa-Q4577",
    "job_outcome": "completed",
    "mismatch": null,
    "reason": "ours contradicts ruling qa:Q4577 (ours vs DCGO: agree); CLEAN (compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
    "recorded": [
      "qa:Q4577"
    ],
    "refused": [],
    "scenario": "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
    "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T193014Z_b5d869ae4ec9465098259626c0b66a00.state.jsonl",
    "stall": null,
    "verdict": "diverged"
  },
  "oracle_results": {
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml": {
      "backfill_note": null,
      "backfilled": false,
      "clause": "BT21-071#effect#0",
      "denominator": "compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)",
      "divergence": null,
      "first_divergence": null,
      "ids": [
        "qa:Q4577"
      ],
      "job_id": "exam-BT21-071-qa-Q4577",
      "job_outcome": "completed",
      "mismatch": null,
      "reason": "ours contradicts ruling qa:Q4577 (ours vs DCGO: agree); CLEAN (compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
      "recorded": [
        "qa:Q4577"
      ],
      "refused": [],
      "scenario": "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
      "sidecar": "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T193014Z_b5d869ae4ec9465098259626c0b66a00.state.jsonl",
      "stall": null,
      "verdict": "diverged"
    }
  },
  "oracle_retry_paths": null,
  "pool_files": [],
  "prompt_evidence": null,
  "prompt_route": null,
  "reason": "ours contradicts ruling qa:Q4577 (ours vs DCGO: agree); CLEAN (compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))",
  "ruling_block_written": [
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml"
  ],
  "ruling_contradicted": {
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml": [
      "RULING qa:Q4577 CONTRADICTED: at 9: p0.field expected - BT21-071 but our engine has - card_id: EX7-008   dp: 1000   suspended: false   sources:   - ST1-01",
      "ruling qa:Q4577: ours CONTRADICTS the ruling (1 check(s), 1 failed)"
    ]
  },
  "scenario_path": "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
  "scenario_paths": [
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml"
  ],
  "sim_failure": null,
  "sim_notes": {
    "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml": [
      "no `assert:` block, so this scenario checked NOTHING. Run the oracle pass and backfill before trusting it in CI."
    ]
  },
  "source": "qa",
  "termination": null,
  "triage": null,
  "triage_feedback": "codex (triage): Q4577 includes effects and digivolution requirements in \u201cX in its text,\u201d so EX7-008 qualifies for Scopemon\u2019s special digivolution. The assertion is one step early: run.rs:227-234 records the initial state at 0 and each action\u2019s result at i+1. Thus at: 9 is after promotion, with EX7-008 on the field; Scopemon\u2019s digivolution is action index 9 and its result is at: 10. Change expect_ruling.assert[0].at from 9 to 10, retaining p0.field: [BT21-071]. The supplied DCGO sidecar confirms the successful evolution. MCP replay was unavailable because its approval requirement conflicts with this session\u2019s never-approve policy.",
  "triage_first": {
    "attempt_id": "20261006T193438391879Z-pilot-three-musketeers-2b204f",
    "call": "scenario_wrong",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q4577"
    },
    "family": "codex",
    "reasoning": "Q4577 includes effects and digivolution requirements in \u201cX in its text,\u201d so EX7-008 qualifies for Scopemon\u2019s special digivolution. The assertion is one step early: run.rs:227-234 records the initial state at 0 and each action\u2019s result at i+1. Thus at: 9 is after promotion, with EX7-008 on the field; Scopemon\u2019s digivolution is action index 9 and its result is at: 10. Change expect_ruling.assert[0].at from 9 to 10, retaining p0.field: [BT21-071]. The supplied DCGO sidecar confirms the successful evolution. MCP replay was unavailable because its approval requirement conflicts with this session\u2019s never-approve policy."
  },
  "triage_packet": {
    "prompt": "# Triage a divergence: interaction:qa:Q4577\n\nYou are one stateless worker in the card-authoring loop. Read only: do not edit\nany file. Do not invoke slash-command skills and do not spawn sub-agents. Your\nanswer can end this item, so another model family answers the same question\nindependently: argue only from sources you can cite.\n\n## The divergence\nOur engine and DCGO ran the same scripted line and disagreed.\n- Exam: interaction `qa:Q4577` on BT21-071\n- Scenario(s): `qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml`\n- Oracle result:\n```json\n{\n  \"backfill_note\": null,\n  \"backfilled\": false,\n  \"clause\": \"BT21-071#effect#0\",\n  \"denominator\": \"compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable)\",\n  \"divergence\": null,\n  \"first_divergence\": null,\n  \"ids\": [\n    \"qa:Q4577\"\n  ],\n  \"job_id\": \"exam-BT21-071-qa-Q4577\",\n  \"job_outcome\": \"completed\",\n  \"mismatch\": null,\n  \"reason\": \"ours contradicts ruling qa:Q4577 (ours vs DCGO: agree); CLEAN (compared 12 of 13 ours / 13 dcgo steps (1 sim-only row(s) with no DCGO prompt + 1 DCGO intermediate row(s) not comparable))\",\n  \"recorded\": [\n    \"qa:Q4577\"\n  ],\n  \"refused\": [],\n  \"scenario\": \"qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml\",\n  \"sidecar\": \"C:/Users/james/AppData/LocalLow/DCGO/DCGO\\\\dcgo_recordings\\\\20261006T193014Z_b5d869ae4ec9465098259626c0b66a00.state.jsonl\",\n  \"stall\": null,\n  \"verdict\": \"diverged\"\n}\n```\n\n## Sources, in priority order\n1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the\n   pages). It outranks DCGO on rules.\n2. DCGO C#: `C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT21\\Purple\\BT21_071.cs` -- the authority for how a card resolves.\n3. Printed text: `data/card_bundles/BT21-071.md` (official Bandai DB). Official rulings:\n   `data/card_qa.json`.\nReplay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to\nsee our prompt and board at the divergent step.\n\nTwo shapes of finding carry no field divergence and are still divergences:\n- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate\n  mismatch; the prompt evidence above spells out who asked what): the engines\n  disagree about that prompt -- one asks it, the other does not, or they offer\n  different candidates. Decide who is right about asking it there. Do not\n  answer \"nothing to classify\": the stopped line is the finding.\n- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the\n  publisher's own answer, which outranks DCGO -- also when `ours vs DCGO:\n  agree` (both engines can be wrong against the publisher). `ours_wrong`\n  unless the `expect_ruling:` misreads the answer or asserts what it does not\n  decide: then `scenario_wrong`, naming the words. Never `dcgo_quirk` for a\n  line on which DCGO agreed with us.\n\n## Classify\n- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.\n- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the\n  two differ only in a rules-neutral way.\n- `unreachable`: no legal line reaches the clause the way the exam needs; state\n  what you measured.\n- `scenario_wrong`: the exam itself misreads the card or the ruling -- it picks\n  the wrong card, asserts at the wrong step, or its `expect_ruling:` claims\n  something the answer does not decide -- so neither engine is being judged.\n  Say exactly what to change; the exam goes back to its author with your words.\n- `undetermined`: the sources do not decide it.\nA `dcgo_quirk` or `unreachable` call without a citation is not accepted.\n\n## Result\nReturn only JSON matching `triage`:\n`{\"classification\": \"...\", \"citation\": {\"kind\": \"rule\" or \"ruling\" or \"dcgo\", \"ref\": \"16-36\" or \"qa:Q1601\" or \"<path>.cs:<line>\"} or null, \"reasoning\": \"...\"}`\n\n## Worktree hygiene\n\nKeep scratch out of the worktree: write temporary files (notes, probes, helper\nscripts) under the system temp directory, never inside the repository. Only\nthe files you deliver may be left behind; anything else is dropped at merge.\n",
    "prompt_version": "4",
    "references": [
      "qa/dcgo-exams/BT21/BT21-071-qa-Q4577.yaml",
      "data/card_bundles/BT21-071.md",
      "C:\\Users\\james\\Documents\\digimon-deck-list-builder-1\\DCGO\\Assets\\Scripts\\CardEffect\\BT21\\Purple\\BT21_071.cs",
      "C:/Users/james/AppData/LocalLow/DCGO/DCGO\\dcgo_recordings\\20261006T193014Z_b5d869ae4ec9465098259626c0b66a00.state.jsonl",
      "docs/digimon-rules/rules-index.json",
      "docs/digimon-rules/keyword-semantics.md"
    ]
  },
  "triage_second": {
    "attempt_id": "20261006T165318770778Z-pilot-three-musketeers-d18865",
    "call": "undetermined",
    "citation": {
      "kind": "ruling",
      "ref": "qa:Q4577"
    },
    "family": "claude",
    "reasoning": "Our engine follows Q4577 and agrees with DCGO. The failure comes from how the `expect_ruling` block was authored, not from engine behaviour, so I classed it undetermined.\n\n- **Ruling:** Q4577 says \"X in its text\" covers cards that name X in their effects. ToyAgumon EX7-008 is a black Lv.3 whose [On Play] names [Three Musketeers]. Scopemon BT21-071's [Digivolve] \"Lv.3 w/[Three Musketeers] in text: Cost 2\" therefore applies to it. The line reaches step 12 with Scopemon on ToyAgumon over Koromon and memory at -3. P0 digivolves 3\u21921 and passes, which puts 3 on P1's side, and -3 is P0's view of that. DCGO agrees (oracle compared 12 of 13 steps, diff CLEAN).\n- **Why the verdict reads \"ours contradicts ruling\":** `exam/assertions.rs::values_equal` compares sequences by exact element equality. The block under `p0.field` lists `{card_id: BT21-071, sources: [EX7-008, ST1-01]}` and omits `dp` and `suspended`, so it cannot match the full projected field entry. The oracle-confirmed `BT21-071-effect0.yaml` asserts the same line at step 12 with `dp: 4000`, `suspended: false`, `memory: -3` and the same `sources`. That makes the ruling block under-specified for the checker, not evidence that our engine breaks the ruling.\n- **What I could not confirm:** I probed a corrected block (with `dp` and `suspended` added). The sim-only probe returned no ruling check result, so that rests on reading the code and on the effect0 line, not on a passing run.\n- **Why not the other classes:** `ours_wrong` would send the item to an engine fix that has nothing to fix. `dcgo_quirk` does not apply because DCGO agrees with us. `unreachable` does not apply because the line is reachable.\n- **Fix:** add `dp: 4000, suspended: false` to the field entry in the `expect_ruling` block of `BT21-071-qa-Q4577.yaml` and re-run the oracle."
  },
  "verdict": "diverged"
}
```

## How to resolve

1. Check each argument's citation against the sources in priority order: `general_rule.pdf` for rules and timing, the DCGO C# for how a card resolves, the official Bandai DB (`data/card_bundles/<ID>.md`) for printed text (CLAUDE.md "Source priority"). A call with no citation carries no weight.
2. Record the decision on the verdict row: our engine right / DCGO differs -> `cargo run -p dcgo-harness -- verdict-triage --clause <clause id> --triage dcgo_quirk --citation "<source>"`; our engine wrong -> `--triage ours_wrong` with the citation, then fix it under the fix gate (or let the loop's fix stage take it on the next run).
3. Record which family's call was wrong as a late correction (`tools.card_loop.corrections.detect_escalation_resolution`) so the scorecard penalises it.
4. Delete this file. The next `python -m tools.card_loop resume` re-reads the committed ledgers; the row in `index.jsonl` stays as history.
