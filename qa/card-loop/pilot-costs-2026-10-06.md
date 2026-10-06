# Card-loop pilots, 2026-10-06: cost and first scorecard (change task 8.2)

Two archetype runs of `python -m tools.card_loop` on one local node and one DCGO
oracle player (`scripted-v19`), config `runs/card-loop/pilot.toml`: Claude
`sonnet`, Codex `effort = high`, concurrency 3, worktree pool 3, `budget_usd =
150`, `wall_clock_hours = 10`, `plateau_attempts = 60`. Both ran from ~13:45Z;
the loop's own tooling defects were fixed from their ledgers as they appeared
(eight relaunches), so these numbers include the cost of finding those defects.
Claude costs are the CLI's reported USD; **Codex reports tokens but no price**,
so its share is unpriced until `[prices.codex]` is set (see the last section).

Sources: each run tree's `qa/card-loop/attempts.jsonl` (`report models` reads
it), `runs/card-loop/<run>/events.jsonl`, `report.md`.

## Outcomes

| Run | Work set | Adjudicated this run | Escalated | Attempts | Claude spend | Stop |
|---|---|---|---|---|---|---|
| `pilot-data-squad` (DS) | 19 cards, 69 clauses, 105 gating interactions; 46 clauses unexamined at start | 20 clauses + 12 interactions confirmed (+ 1 earlier) | 7 clauses, 6 interactions | 210 | **$76.23** | still running at the time of writing (restarted on each fix) |
| `pilot-three-musketeers` (TM) | 569 items: 235 clauses (already adjudicated by the earlier hand-driven campaign), 334 gating interactions | 63 interactions confirmed (293 adjudicated of 569 at the stop) | 16 interactions | 529 | **$151.60** | **budget**: "spent $150.56 reached the $150.00 cap" after 1.09 h of this session (ninth launch); 194 Codex attempts unpriced |

Oracle round trips: DS 64 (28 confirmed, 24 diverged, 12 other) = 2.29 trips per
confirmation; TM 126 (63 / 49 / 14) = 2.00.

## Cost per adjudicated unit

Track A (the hand-driven `/dcgo-exam` campaigns, Sonnet, ~70 % cache reads)
measured **$1.72 per adjudicated clause**. Two readings of the pilots, both
Claude-only:

| | DS clauses | DS interactions | TM interactions | both runs |
|---|---|---|---|---|
| **Marginal** -- spend on the items that ended adjudicated, per item | **$0.86** (16 items, $13.72) | **$1.02** (12, $12.24) | **$1.06** (63, $66.96) | ~$1.02 |
| **All-in** -- every dollar of the run, per adjudication | $2.76 per adjudication (28 for $77.25) | | **$2.41** (63 for $151.60) | **$2.51** (91 for $228.85) |

The marginal figure is the like-for-like with Track A and is 40-50 % under it.
The all-in figure is what a run actually costs per adjudication today: the
difference is work that did not adjudicate -- escalated items (TM spent $84 of
its $151 on items still open or escalated), fix attempts that failed their gate,
and the defect-finding relaunches. Both numbers should fall on the next runs:
the encode reorder alone took `encode_ruling` from 29 % first-pass (v3) to
79 % (v4), and the escalations that were tooling-caused are fixed.

## Where the money went (Claude USD, stage x run)

| Stage | DS | TM | Notes |
|---|---|---|---|
| author_interaction | 16.22 (26 calls) | 84.55 (132) | TM's main cost: 117 accepted of 132, $0.72 per accepted |
| author_clause | 24.81 (32) | -- | 25 accepted, $0.99 per accepted |
| encode_ruling | 5.55 (19) | 26.85 (97) | $0.39-0.50 per accepted; 14 TM escalations were all pre-reorder (v1-v3) |
| triage | 10.14 (23) | 19.22 (38) | $0.58-0.60 per accepted |
| classify_qa | 4.59 (19) | 14.57 (60) | $0.24 per ruling, both families always agreed |
| fix_card | 14.92 (16) | 4.43 (6) | DS: 7 accepted of 16, $2.13 per accepted, 988 s average; the hour-long cap killed one |
| fix_engine | -- (Codex) | 1.39 (1) | 0 fixes landed in either run: the two gaps need a human |

Codex tokens (unpriced): DS 4.03 M input / 68.8 M cached / 0.23 M output over 75
calls; TM 7.38 M / 59.1 M / 0.25 M over 194 calls.

**One finding changes these numbers for the next runs.** Both pilots' SIM and
ORACLE stages ran a `dcgo-harness` binary built from the developer's checkout,
not from the run tree where the loop's card fixes land (the lookup fell through
to it), so a landed fix never reached the oracle's engine: BT26-005#inherited#0
passed its fix gate, diverged again at the oracle and spent its fix cap, and
BT26-072#inherited#0's author probe (pool build) and the driver's SIM disagreed
three times. Fixed the same night (the driver builds the harness from the run
tree and rebuilds it after merges that touch engine inputs); the affected
escalations are re-entered. Every `fix_card` "gate passed, oracle diverged" row
above is suspect for this reason.

## First scorecard (`report models`, first-pass acceptance; n in brackets)

| Stage | Claude sonnet | Codex (default model) | Reading |
|---|---|---|---|
| classify_qa v2 | 79 % [58] TM, 100 % [19] DS, $0.24-0.30 per good | 79 % [58], 100 % [19] | identical calls; both families agree on every ruling |
| author_interaction v2 | 70 % [125] TM, 35 % [23] DS, $0.92 per good (TM) | 71 % [17] TM, 62 % [24] DS | Codex at least as good on the small samples; its share is the 20 % exploration lane |
| author_clause v2 | 68 % [28] DS, $1.00 per good | 100 % [3] | too few Codex samples |
| encode_ruling v4 | 79 % [75] TM, 67 % [12] DS, $0.35 per good | 76 % [68], 67 % [12] | encoder/verifier pairs; equal |
| triage v4 | 70 % [30] TM, 86 % [14] DS, $0.50-0.74 per good | 38 % [16] TM | Claude clearly better at triage so far |
| fix_card v3 | 67 % [6] TM, 14 % [14] DS; $6.70 per good on DS | 0 % [2] | the expensive stage; most failures are gate failures ("passes without the fix", "no regression test named") |
| fix_engine v3 | 0 % [1] | 0 % [5] | engine gaps are not a worker's job |

No late corrections yet (no human audit, no overturned verdict), so `wcorr` =
immediate corrections only. The routing override needs >= 30 samples per cell
with separated cost intervals; no cell qualifies yet, so the config routes stand.

## What escalated, and what it says

Tooling-caused escalations (all fixed during the pilots, re-entered by
withdrawing the file): stale `index.lock` after a `taskkill` restart, the gate
worktree collision, the hour-long fix worker retried as "transient", the gate
rejecting the absolute DCGO path the prompt itself hands out, encode anchoring
on a library line (the D7 reorder), shape-only ruling contradictions (lenient
checker). What remains is genuine:

- engine gaps a worker cannot fix: `G-ENGINE-LINK-BASE-CAPACITY` (linked
  Shotmon BT21-054 not trashed at the rule check, Q4578 / Q4585),
  `BREEDING-TRIGGER-DISPATCH-UNGATED` (Q6388);
- a rules question for a human: "[X] in its text" -- the publisher counts
  digivolution-requirement text, both engines do not (Q4577, Q6252);
- exam vocabulary: Burst Digivolve has no step verb (BT13-060#effect#0); the
  clause extractor counts a "ー" placeholder as an inherited clause (BT26-049);
- non-converging author/triage rounds (two) and fix attempts whose diff was
  empty or whose test passed without the fix.

## Pricing Codex

Set `[prices.codex]` (USD per million tokens: `input`, `cached_input`, `output`)
in the run config and the ledger prices every Codex call. For scale only: at a
GPT-5-class list price of $1.25 / $0.125 / $10 the two runs' Codex tokens come
to about $36 (DS ≈ $16, TM ≈ $19), i.e. roughly +16 % on the Claude total.

## What this means for the readiness backlog

The interaction gate (change group 7) holds 94 clause-ready cards behind 511
gating interactions (292 promoted probes, 219 Q&A rulings). At today's figures
that backlog is **≈ $520 marginal / ≈ $1,300 all-in** in Claude spend, before
Codex; the pilots' own 75 confirmed interactions reach the artifact once their
run branches are merged and `tools.clause_coverage.readiness` is regenerated.
