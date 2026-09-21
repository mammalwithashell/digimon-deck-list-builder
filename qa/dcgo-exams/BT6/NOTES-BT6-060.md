# NOTES — Deputymon (BT6-060)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT6-060`):
**2 clauses** — `effect#0`, `effect#1`.
DCGO script: `BT6/Black/BT6_060.cs` exists — the card is **not** `unavailable`.
Both clauses have a scenario; neither is `unreachable`.
Book: `qa/dcgo-exams/BT6/tm_bt6_060_pool.json` (decks `tm-deputy-black`, `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT6-060#effect#0` | `[On Play] Reveal the top 4 cards of your deck. Add 1 Digimon card with [Three Musketeers] in its type and/or 1 Option card with a memory cost of 7 among them to your hand. Trash the remaining cards.` | `BT6-060-effect0.yaml` | lowers, asserts pass |
| `BT6-060#effect#1` | `[Your Turn] This Digimon can digivolve into a Digimon card with [Three Musketeers] in its type from your hand for a memory cost of 6, ignoring its digivolution requirements.` | `BT6-060-effect1.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

Both lowered and asserted as found. Prompt sequences re-derived from
`BT6_060.cs`:

- `effect#0`: `SetUpActivateClass(..., -1, false, ...)` — mandatory;
  `SimplifiedRevealDeckTopCardsAndSelect(revealCount 4, two conditions,
  RemainingCardsPlace.Trash)`, `mutualConditions` default false,
  `canNoSelect` false → ONE `SelectCardEffect` per condition in order
  ([Three Musketeers]-trait Digimon, then cost-7 Option), then the rest
  trashed with no prompt. The reveal is engineered as two P-170 + two
  BT6-105 so both picks are two-candidate decisions. Ours: two
  `select_reveal_buckets` picks in the same order, then `per_selected
  trash_from_reveal`. 1:1.
- `effect#1`: `AddSelfDigivolutionRequirementStaticEffect(this permanent,
  cost 6, ignoreDigivolutionRequirement: true, condition: IsOwnerTurn &&
  on battle area, cardCondition: own HAND Digimon with the trait)`. The
  destination is AvengeKidmon P-170 — the only [Three Musketeers]-trait
  Digimon in the pool with no `[When Digivolving]` (its cost reduction is a
  `BeforePayCost` play-time effect; <Raid>/<Blocker>/<Retaliation> are
  passive; its `[On Deletion]` never fires) — and neither its printed
  circles (Red/Purple Lv.5) nor its own alt route (`P_170.cs`: Lv.5 with
  [Three Musketeers] in text) fits a black Lv.4 Deputymon, so the grant is
  the only route: one cost, no `SelectCountEffect`. Deputymon reaches the
  field by digivolving (over ToyAgumon BT25-064 through its printed Black
  Lv.3 / 2 circle) so its `[On Play]` never opens on this line.

## Second resume (2026-09-18): re-lowered; slot hygiene; one PREDICTED oracle diff

Both files re-lowered unchanged against the current harness. Slot hygiene
(`../BT25/NOTES-BT25-083.md`): P0 never holds more than one permanent and P1
holds none, so `field.0` is the same permanent on both engines.

**Predicted for `effect#0` — the `RevealBucket` add-timing quirk.**
`docs/DCGO_EXAM.md` ("Rows where one side looks rules-wrong") records that on
a ≥2-condition `SimplifiedRevealDeckTopCardsAndSelect` DCGO moves each
bucket's pick to the hand when THAT prompt closes, while ours collects every
pick and adds once (§15-15-10-1: one "Add" with two targets). Expect a
one-field `p0.hand` divergence on the 2nd pick row (DCGO already shows P-170
in hand) with no downstream rows; triage it as the documented DCGO quirk
(measured on EX7-008#effect#1 and BT7-056#effect#0), not as an engine bug.

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#0` re-measured: the PREDICTED quirk, verdict stays **diverged**

Re-drained on oracle build `scripted-v16` (DCGO `fc67f9ae6`) against a fresh sidecar
and re-diffed with `--all-diffs`.

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `BT6-060#effect#0` | `20260921T042614Z_c72aa5cc` | DIVERGED at step 3, compared 5 of 5 ours / 5 dcgo | **diverged** |

The prediction written in "Second resume" above is exactly what the oracle produced —
one field, on the 2nd bucket pick row:
`p0.hand: ours=[BT2-052, BT2-056, BT3-059, BT3-067] dcgo=[…, P-170]`. `--all-diffs`
confirms it is the LEAD **and the only row**; step 4 (after resolution: hand, trash,
field, memory) matches.

**DCGO quirk, no fix.** The printed text is ONE "Add" with two targets
(`general_rule.pdf` 15-15-10-1 — a single effect selecting multiple targets with
different conditions; 15-1-2 — processed in printed order, and the one "Add" follows
both selections). DCGO's `RevealDeckTopCardsAndSelect` runs one
`SelectCardEffect(mode: Mode.AddHand)` per condition in a `foreach`
(`DCGO/Assets/Scripts/Script/CardEffectCommons/RevealLibrary.cs:291-336`, whose own
comment says the conditions are "chosen at once (per card game rules)"), so each pick
moves to hand as its prompt closes; ours collects both and adds once. Outcome-neutral:
revealed cards are in no area while revealed (15-15-3-2) and neither bucket reads the
hand. Adjudicated under `G-EXAM-REVEAL-BUCKET-ADD-TIMING` in
`docs/RUST_ENGINE_GAPS.md`, which already names this clause as a driver.

`diverged` is the honest verdict — the exam compares every row and these rows differ —
but it is a DCGO-quirk finding, not an engine bug, and no further action follows.

**2 clauses: 1 confirmed, 1 diverged (adjudicated DCGO quirk), 0 unreachable, 0 unavailable, 0 unmeasured.**
