# NOTES — Dorumon (BT7-056)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT7-056`):
**2 clauses** — `effect#0`, `inherited#0`.
DCGO script: `BT7/Black/BT7_056.cs` exists — the card is **not** `unavailable`.
Both clauses have a scenario; nothing is `unreachable`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT7-056#effect#0` | `[On Play] Reveal the top 3 cards of your deck. Add 1 [Kota Domoto] and 1 card with the [X Antibody] trait among them to the hand. Return the rest to the bottom of the deck.` | `BT7-056-effect0.yaml` | lowers, 7/7 asserts pass |
| `BT7-056#inherited#0` | `[Your Turn] [Once Per Turn] When your effects add to this Digimon's digivolution cards, gain 1 memory.` | `BT7-056-inherited0.yaml` | lowers, 7/7 asserts pass |

Both use book `tm_bt7_056_pool.json`, deck `tm-dorumon`: the shared 50 with
BT7-056 ×4 and Kota Domoto BT7-090 ×2 swapped in for EX7-071 ×3, EX7-070 ×2
and one P-180 (Kota Domoto is not in the shared pool), egg deck Kapurimon
EX7-005 ×4. Both `BT7_090.cs` and `EX7_073.cs` exist in DCGO, so every card
the lines name has an oracle-side script.

## `effect#0` — the EX7-008-effect1 shape, with disjoint witnesses

`SimplifiedRevealDeckTopCardsAndSelect(revealCount 3, [HasXAntibodyTraits
→ AddHand ×1, CardNames.Contains("Kota Domoto") → AddHand ×1],
DeckBottom)`. Re-derived from `RevealLibrary.cs`:

- `canNoAction` is not passed → the "≥ 2 conditions" `generic_bool`
  ("Will you select cards?") gate is **not** asked.
- One `SelectCardEffect` per condition with `maxCount = min(1,
  candidates)` ≥ 1, in **C# condition order** — `[X Antibody]` trait
  FIRST, `[Kota Domoto]` name SECOND (the printed sentence lists them the
  other way round; `BT7-056.yaml` authors its buckets in the C# order, and
  the lowered wire shows `EX7-073` then `BT7-090`). `canNoSelect: false`.
  Under the harness `SelectCardEffect.Activate()` routes both seats to
  `AutoSelect()` → `SetTargetCardAndIndicies` (`SelectCardEffect.cs:383`),
  so each single-candidate pick still consumes one wire row.
- The chosen card is removed from `revealedCards` before the next
  condition scans, so the witnesses were chosen **disjoint** anyway:
  BeelStarmon (X Antibody) EX7-073 (trait `X Antibody`, matched by
  `DataBase.IsXAntibodyString` after stripping spaces/hyphens; name ≠ Kota
  Domoto) and Kota Domoto BT7-090 (no traits — its text mentions
  [X-Antibody], but both engines scan the TRAIT line). Titamon +
  SkullBaluchimon BT24-081 (Shaman/Olympos XII/Titan/TS/Demon) matches
  neither and is the lone remainder.
- One remaining card → DCGO's deck-bottom ordering asks nothing
  (`remainingCards.Count == 1`); ours parks the documented
  `OrderedPermutation { remaining: 1 }` one-choice prompt → `sim_only`
  row, zero wire rows (the "OrderedPermutation at N=1 — ours" finding in
  `docs/DCGO_EXAM.md`).
- Memory 3 → 0 on the play; the trailing pass is asserted at 0, still
  P0's seat — the same edge `EX7-008-effect1.yaml` sits on.

## `inherited#0` — Dorumon must be BURIED, and the placer must be YOURS

Inherited effects are live only from the digivolution cards, so Dorumon is
breeding-digivolved onto Kapurimon (its only circle, Black Lv.2 / 0; no
`[On Play]` on a digivolve) and BlackGatomon BT25-082 is digivolved onto
it on T5 through its **black** Lv.3 / 3 circle — the purple circle fails
on colour and the "[Three Musketeers] in text or [TS] trait" alt-circle
fails on Dorumon (Beast / X Antibody, no such text), so exactly one route
opens and no count row is asked. The placer is LadyDevimon BT25-083's
`[On Play]` cost on T7 (the `BT25-083-effect1.yaml` rows), which puts
**BeelStarmon (X Antibody) EX7-073** from the hand under the BlackGatomon
stack. Dorumon's trigger then fires: `EffectSourceCard != null &&
EffectSourceCard.Owner == card.Owner` (LadyDevimon is P0's), `cardCondition:
null` (any card kind), `IsOwnerTurn`; ours `event_host_permanent_is_source`
+ `event_caused_by_own_effect`. Mandatory (`SetUpActivateClass(..., 1,
FALSE, ...)`) → no prompt on either side; the observable is **−4 → −3**.

Why EX7-073 and not P-180: the egg under Dorumon is Kapurimon, whose own
inherited (`EX7-005#inherited#0`) fires on a `[Three Musketeers]`
**Option** placed under the same stack. That would stack two mandatory
triggers on one event and bury the clause under a `MultipleSkills` /
`TriggerOrder` row (identity-matchable, but a prompt this clause does not
need). EX7-073 carries the trait (LadyDevimon's `HasThreeMusketeersTraits`
accepts it; our union pick `trait_has` is kind-agnostic) but is a Digimon
card, so Kapurimon stays silent. Pagumon BT25-005 was not used as the egg
for the same reason (it fires on ANY trait card).

Memory is engineered away from zero exactly as `BT21-071-effect1.yaml`
does: P1 overspends on T4 (Biyomon ST1-02 for 2, Garudamon ST1-08 for 6:
3 → 1 → −5), P0 starts T5 at 5, BlackGatomon takes it to 2, P0 passes.
Neither P1 play prompts (Biyomon: nothing; Garudamon: `[When Digivolving]`
only). Biyomon's DP is 3000 (the first draft asserted 2000; corrected at
lowering — the only edit the line needed).

BlackGatomon's `[When Digivolving]` on T5 is DCGO-gated off
(`AdditionalActivateCondition`: a TM-text Tamer in hand — none is ever
drawn); our `BT25-082.yaml` carries only the "≤1 Tamers" half of that gate
but voids the optional trigger with zero hand candidates, so the line
lowered with no compensation row (same observation as
`NOTES-BT25-005.md` §4).

## Not measured by these lines

- `effect#0` with a bucket that has **no** candidate (the official Q&A:
  "you still add that card") — DCGO skips that condition's
  `SelectCardEffect` (`maxCount >= 1` guard); ours skips the empty
  bucket. Not authored; the shape is a strict subset of this line.
- `effect#0` with a card that fits **both** buckets — `no_duplicate_cards`
  on our side vs DCGO removing the chosen card from `revealedCards`.
  Kota Domoto printings carry no `[X Antibody]` trait and BT20-087 (Kota
  Domoto & Yuji Musya) is `Chronicle`, so no legal card in the pool can
  exercise it.
- `inherited#0`'s **owner gate** (an opponent's effect placing under the
  stack must NOT fire) — the quiet-opponent pool has no such effect.
- `inherited#0`'s rule 15-5-3 self-placement case (Dorumon itself placed
  by your effect) — Dorumon has no `[Three Musketeers]` trait, so none of
  this pool's placers can move it from hand.

## Oracle verdicts (2026-09-18, job three-musketeers-1)

- `inherited#0` — **confirmed** (CLEAN, 19 of 20 steps compared).
- `effect#0` — **diverged** at step 7, single diff under `--all-diffs`:
  `p0.hand` before the SECOND bucket's `SelectCardEffect` — DCGO already holds
  EX7-073, ours does not. `RevealLibrary.cs` runs `AddHandCards` inside the
  per-condition loop (each bucket's pick is added to hand before the next
  bucket is asked); our reveal-bucket step parks all bucket picks and moves
  the cards after the last one. End state converges (no later diff: both
  hands hold EX7-073 + BT7-090, BT24-081 to the bottom). Finding family:
  mid-effect staging order of multi-bucket reveal (`RevealBucketStep`), not a
  BT7-056 card-spec error — any "when a card is added to your hand" observer
  or hand-count-dependent second bucket would see it. Not fixed here (exam
  stage does not edit the engine).
  **TRIAGED 2026-09-18: DCGO QUIRK, no fix.** The printed text is ONE "Add"
  with two targets -- `general_rule.pdf` 15-15-10-1 ("a single effect allows
  you to select multiple targets with different conditions") plus 15-1-2
  (processed in printed order: both targets are selected, then the single add
  is performed). DCGO's per-condition `SelectCardEffect(Mode.AddHand)` `foreach`
  (`RevealLibrary.cs:291-336`; its own comment says the conditions are "chosen
  at once (per card game rules)") moves each pick when its prompt closes -- an
  implementation artefact. Outcome-neutral: nothing can trigger or resolve
  mid-effect and neither bucket reads the hand. `--all-diffs` re-run against the
  preserved sidecar: the step-7 `p0.hand` field is the ONLY diff. Same family
  as `EX7-008#effect#1`; see `docs/DCGO_EXAM.md` "RevealBucket add timing". The
  verdict stays `diverged` (triaged); the engine is NOT changed to add per
  bucket.
