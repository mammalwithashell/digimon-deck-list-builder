# BT26 Data Squad (Rosemon / Ravemon) — exam notes

Source lists: DCG Nexus BT26 events (scraped 2026-09-30; archetype tag
"Data Squad", 34 lists containing BT26 cards). `bt26_dats_pool.json` carries the
1st-place list from Eagle's Nest S10W6.5 (2026-09-13,
https://dcg-nexus.com/decklist/ae29fdb6-8896-485d-adf8-6e2c65d7a697) plus the
shared `tm-quiet-red` opponent.

## State (2026-09-30)

14 BT26 cards implemented, 50 clauses. **All 50 are `unmeasured`** — no oracle
(Unity) pass has been run for any of them. Four scenarios are authored and pass
`--sim-only` (lowering + our-engine assertions only):

| Scenario | Clause |
|---|---|
| `BT26-065-effect1.yaml` | Falcomon [On Play] two-bucket reveal |
| `BT26-036-effect2.yaml` | Lalamon [On Play] reveal |
| `BT26-072-effect2.yaml` | Peckmon [On Play] hand-trash cost → delete Lv.≤4 |
| `BT26-094-effect0.yaml` | Keenan Crier [Start of Your Main Phase] |

Their `assert:` blocks were written from OUR engine's result, not backfilled from
DCGO, so they are unconfirmed until the oracle pass.

## Oracle pass 2026-10-05

Player `scripted-v18` (DCGO `b9a0638cd`, main's pinned commit; action space `711d23bf24fb`), run through `D:/card-loop-iter0/run_set.sh` (add-card-authoring-loop iteration 0). Divergences below are **untriaged**: `general_rule.pdf` outranks DCGO; read the card, the rule and the C# first.

For the four cards with scenarios (15 clauses): **3 confirmed, 1 diverged, 11 unmeasured** (no scenario). The wider 14-card / 50-clause pool still has 46 clauses without a scenario.

**Re-run on `scripted-v19` (same day):** DCGO `8b6c39ea1` + harness `99986fa96` resolve the battle-area permanents of attack / `[Main]` / digivolve rows by identity instead of by slot (add-card-authoring-loop 10.1). Unchanged: 3 confirmed, 1 diverged (BT26-065-effect1, reveal timing).

- BT26-036-effect2, BT26-072-effect2, BT26-094-effect0: confirmed.
- BT26-065-effect1 (Falcomon two-bucket reveal): diverged at step 3 on `p0.hand` only -- the reveal-bucket hand-timing pattern (DCGO moves each pick to hand as its prompt closes).


## Running the oracle pass (local, Unity in Play)

```bash
dcgo-harness --root "$ROOT" exam --scenario qa/dcgo-exams/BT26 --sim-only \
    --cards-json data/cards.json --decks qa/dcgo-exams/BT26/bt26_dats_pool.json \
    --emit-job jobs/
# drain the jobs with the phase-1 harness, then per scenario:
dcgo-harness --root "$ROOT" exam --scenario qa/dcgo-exams/BT26/<file>.yaml \
    --sidecar <recording>.state.jsonl --cards-json data/cards.json \
    --verdicts --clause-text-json <clause_coverage extract output>
```

## Expected divergences to triage (read before calling anything a bug)

- **BT26-044 Lilamon inherited** — DCGO evaluates "with [Rosemon] in its name or
  the [DATA SQUAD] trait" against the inherited Lilamon card itself (always true);
  we gate on the carrier per the printed text. A non-DS, non-Rosemon carrier
  diverges by design.
- **Mid-effect observers** — `OnDigivolutionCardTrashed` observers (Yoshino,
  Crowmon, Lilamon, Rosemon, Budmon) resolve inline, before the rest of the effect
  that trashed the source (engine EX10-036 contract); DCGO stacks them after.
- **Multi-suspend** — DCGO fires one `OnTappedAnyone` per suspend action; the
  engine fires `on_suspend` per permanent (G-ENGINE-ON-SUSPEND-BATCH).
- **Queen of Thorns** — DCGO lets the player order the two placed sources; we
  place Sunflowmon then Lilamon.
