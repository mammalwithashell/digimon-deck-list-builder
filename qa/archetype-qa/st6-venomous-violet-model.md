# ST-6 Venomous Violet — Model

Scout artifact for `/archetype-interaction-test-author`. Set `st6`, slug
`st6-venomous-violet`, color **Purple**, deck_library key
"Starter Deck Venomous Violet". Pool = ST6-01 … ST6-16, all DSL YAML, all
`AUDITED-OK` in `qa/qa-reports/validated_cards_dsl.json` (per the 2026-05-29
faithfulness audit `qa/qa-reports/2026-05-29-starter-decks-st1-6-faithfulness-audit.md`).

The deck is a **trash-matters value engine**: *fill* the trash (mill + attack
discard + self-deletion), then *cash it in* (recur Digimon from trash, threshold
DP buff, memory off deletions) and close with Retaliation-backed Mega bodies.

Sources cited inline: printed text (`code/digimon-engine/cards/st6/<ID>.json`),
DCGO C# (`DCGO/Assets/Scripts/CardEffect/ST6/Purple/ST6_<NN>.cs` in the **base**
repo), `general_rule.pdf` §16 keyword semantics (Retaliation §16-12, Digi-Burst
§16-13). YAML implementation in `code/digimon-engine/cards/st6/<ID>.yaml`.

> **Audit note carried forward (test-coverage gap, not a faithfulness defect):**
> the prior audit found ST6 per-card behavioral tests are *mostly structural IR
> shape assertions, not end-to-end DebugRunner play-throughs*. The cross-card
> interaction tests planned below are exactly the end-to-end coverage that gap
> calls for.

## Card pool & roles

| Card | Role | One-line function |
|---|---|---|
| ST6-01 Pagumon (Egg, Lv2) | engine (fill) | Inherited `[On Deletion]` Trash top 2 of your deck — mills as a digivolution source dies. DCGO gates on deck ≥1 (`ST6_01.cs` `CanActivateCondition`). |
| ST6-02 DemiDevimon (Lv3) | body | Vanilla Rookie (4000 DP, cost 2). Cheap purple Lv3 fodder for recursion targets. |
| ST6-03 Gabumon (Lv3) | engine (fill) | Inherited `[When Attacking]` Draw 1, then trash 1 from hand — net mill+filter on every inheriting attacker. |
| ST6-04 Dracmon (Lv3) | engine (cash-in) | `[On Play]` may return a purple Option of cost 1 **or** 7 (Death Claw / Nail Bone) from trash to hand. Optional. |
| ST6-05 Elecmon (Lv3) | body | Vanilla Rookie (5000 DP). Beatstick / recursion target. |
| ST6-06 Garurumon (Lv4) | engine (fill) | Same inherited `[When Attacking]` Draw 1 + trash 1 as ST6-03 — the Lv4 copy of the fill engine. |
| ST6-07 Youkomon (Lv4) | body | Vanilla Champion (6000 DP). Lv4 recursion target for Nail Bone / Digi-Burst. |
| ST6-08 Devimon (Lv4) | tech (blocker) | `<Blocker>`; `[When Attacking]` Lose 2 memory. Defensive wall on the Devimon line. |
| ST6-09 Kyukimon (Lv5) | body | Vanilla Ultimate (9000 DP). Beatstick into the Mega line. |
| ST6-10 SkullSatamon (Lv5) | engine (cash-in) | `[When Digivolving]` may return 1 purple **Digimon** from trash to hand. Optional (`ST6_10.cs` `canNoSelect:true`). |
| ST6-11 WereGarurumon (Lv5) | **payoff** | Inherited `[Your Turn]` +2000 DP while **5+ cards in your trash** (DCGO `ST6_11.cs` `TrashCards.Count>=5`). The deck's threshold payoff. |
| ST6-12 VenomMyotismon (Lv6 Mega) | **payoff** | `[When Digivolving]` up to 2 of your Digimon gain `<Retaliation>` until end of opp's next turn (§16-12). |
| ST6-13 CresGarurumon (Lv6 Mega) | **payoff** | `<Security A.+1>`; `[Main] <Digi-Burst 2>` play a purple Lv3 from trash **free, On Play DOES fire** (DCGO `activateETB:true`). |
| ST6-14 Matt Ishida (Tamer) | engine (cash-in) | `[Your Turn]` when one of YOUR Digimon is deleted, may suspend this Tamer → gain 1 memory. `[Security]` play free. |
| ST6-15 Death Claw (Option, cost 1) | tech (removal/fill) | `[Main]` may delete 1 of YOUR Digimon to delete 1 opp Lv4-or-lower; `[Security]` delete 1 opp Lv4-or-lower. |
| ST6-16 Nail Bone (Option, cost 7) | **payoff** (cash-in) | `[Main]` play a purple Lv3 **and** a purple Lv4 from trash free, **On Play suppressed** (DCGO `activateETB:false`); `[Security]` play a purple Lv4-or-lower free, On Play suppressed. |

## Digivolution lines

All purple. Egg = **ST6-01 Pagumon** (Lv2 In-Training).

- **Devimon control line:** Pagumon → DemiDevimon/Dracmon/Elecmon (Lv3) →
  **Devimon** (Lv4, blocker; cost 1 over Lv3) → SkullSatamon (Lv5, cost 3) →
  **VenomMyotismon** (Lv6 Mega, cost 3 over Lv5).
- **Garurumon value line:** Pagumon → Gabumon (Lv3, fill) → **Garurumon**
  (Lv4, fill; cost 2) → WereGarurumon (Lv5, cost 3) / Youkomon → **CresGarurumon**
  (Lv6 Mega, cost 4 over Lv5).
- Cross-pollination: the two Lv5s (SkullSatamon/WereGarurumon) both digivolve
  from any purple Lv4, and both Megas digivolve from any purple Lv5 — so the
  lines interleave freely. Inherited fill effects (Pagumon On-Deletion mill,
  Gabumon/Garurumon attack-draw-trash) ride **under** whatever sits on top, so a
  tall stack carries multiple trash-fillers.

## Named combos

### Trash-threshold beatdown (WereGarurumon online)
- Cards: **ST6-11 WereGarurumon** + any fillers (**ST6-01 Pagumon** On-Deletion,
  **ST6-03 Gabumon** / **ST6-06 Garurumon** When-Attacking, **ST6-15 Death Claw**
  self-delete).
- Expected mechanical outcome: with **≥5 cards in your trash on your turn**,
  WereGarurumon's carrier reads base 7000 **+2000 = 9000** effective DP; the
  buff is conditional and **falls off** the instant the trash drops below 5 (or
  on the opponent's turn). A before/after test seeds trash at 4 → asserts base
  DP, mills/discards to 5 → asserts +2000, then shrinks below 5 → asserts the
  buff is gone.
- Rules/keyword basis: inherited static DP modifier gated on a live trash count;
  DCGO `ST6_11.cs` (`IsOwnerTurn` + `TrashCards.Count>=5` → `ChangeSelfDPStaticEffect(+2000, isInheritedEffect:true)`). Engine read via `DebugRunner::effective_dp`.
- Rank: **1** (freq 1 × payoff-central; the deck's defining threshold and the
  clearest cross-card "fill ⇒ payoff" claim).

### Death Claw → Matt Ishida memory loop
- Cards: **ST6-15 Death Claw** + **ST6-14 Matt Ishida** (+ an expendable own
  Digimon, ideally one carrying **ST6-01 Pagumon** for extra mill).
- Expected mechanical outcome: on your turn, Death Claw `[Main]` deletes 1 of
  YOUR Digimon (the cost) to delete 1 opp Lv4-or-lower; the **own** deletion
  triggers Matt Ishida → may suspend to **+1 memory** (and if the sacrificed
  Digimon carried Pagumon, +2 cards to trash). Net: 1 opp body removed,
  +1 memory refunded, trash filled — for 1 cost. Unhappy path: if you decline /
  can't pay the self-delete, **no opponent deletion happens** (DCGO `ST6_15.cs`
  gates the opp deletion on the self-delete's `successProcess`); and Matt's
  trigger only fires on YOUR turn and only if an unsuspended Matt is in play.
- Rules/keyword basis: Death Claw is a self-pay-to-remove (`ST6_15.cs`,
  YAML `if: { binding_present: sacrifice }`); Matt Ishida `[Your Turn]`
  on-deletion suspend-for-memory (`ST6_14.cs` `OnDestroyedAnyone` + `IsOwnerTurn`
  + suspend cost). General deletion timing.
- Rank: **2** (two engine cards whose value only exists *together*; the
  self-delete cost is the bridge that feeds both Matt and the trash).

### Nail Bone double-recur (trash cash-in)
- Cards: **ST6-16 Nail Bone** + 2 purple Digimon in trash (one Lv3, one Lv4 —
  e.g. **ST6-04 Dracmon** + **ST6-06 Garurumon**).
- Expected mechanical outcome: `[Main]` plays a purple Lv3 **and** a purple Lv4
  from trash, **free**, with their `[On Play]` effects **suppressed** (so
  Dracmon's `[On Play]` return-an-Option does NOT fire). Net: trash −2, field +2,
  no memory paid, no On-Play. A test seeds two eligible Digimon + a third
  ineligible (wrong level/color) → asserts exactly the Lv3 + Lv4 are recurred,
  the ineligible one stays, and Dracmon's On-Play does not re-return an Option.
- Rules/keyword basis: DCGO `ST6_16.cs` (`PlayPermanentCards(..., payCost:false,
  activateETB:false)`, level filters 3 and 4, max 1 of each via
  `CanTargetCondition_ByPreSelecetedList`). Contrast §16-13 Digi-Burst recursion
  below, which does NOT suppress On Play.
- Rank: **3** (the deck's biggest trash payoff, but single-card-dominant; the
  cross-card nuance is the On-Play *suppression*, which a per-card test could in
  principle pin alone — so it ranks just under the two genuinely two-card loops).

### CresGarurumon Digi-Burst recur (On-Play fires) — *contrast / candidate*
- Cards: **ST6-13 CresGarurumon** (with ≥2 digivolution cards) + a purple Lv3 in
  trash (ideally **ST6-04 Dracmon**).
- Expected mechanical outcome: `[Main] <Digi-Burst 2>` trashes 2 of CresGarurumon's
  own digivolution cards, then plays a purple Lv3 from trash free — and unlike
  Nail Bone its `[On Play]` **DOES** fire (DCGO `activateETB:true`). So recurring
  Dracmon via Digi-Burst additionally triggers Dracmon's Option-return. This is
  the explicit Digi-Burst-vs-Nail-Bone divergence.
- Rules/keyword basis: §16-13 (Digi-Burst X, optional); DCGO `ST6_13.cs`
  (`IDigiBurst(...,2,...)` then `PlayPermanentCards(..., activateETB:true)`).
- Rank: **4** (high cost — a 12-cost Mega + Digi-Burst — so low play frequency;
  valuable as the *paired contrast* to Nail Bone's suppression, but ranked below
  the top three).

### VenomMyotismon Retaliation wall — *candidate*
- Cards: **ST6-12 VenomMyotismon** + up to 2 of your Digimon.
- Expected mechanical outcome: `[When Digivolving]`, up to 2 chosen own Digimon
  gain `<Retaliation>` until end of opp's next turn; when such a Digimon is
  deleted after losing a battle, the Digimon it battled is deleted too (§16-12).
- Rules/keyword basis: §16-12 (Retaliation: trigger-type, mandatory); DCGO
  `ST6_12.cs` (`GainRetaliation(..., UntilOpponentTurnEnd)`). YAML grants
  `Retaliation` with `expiry: end_of_opponents_next_turn`.
- Rank: **5** (single-card grant; the combat-interrupt is the hard part to
  exercise end-to-end and is largely a keyword test, not a cross-card combo —
  candidate, dropped from the top tier).

## Playstyle

- **Class:** midrange value/control. Not a fast aggro deck — it grinds card
  advantage through the trash and removes the opponent's early board with Death
  Claw / Devimon's blocker while assembling the Mega payoffs.
- **Tempo:** slow-to-medium. The fill engine (mill on deletion, attack
  draw-trash) front-loads trash so the WereGarurumon threshold and the
  Nail-Bone/Digi-Burst recursion come online by mid-game.
- **Memory curve:** purple's classic low-cost digivolutions (Lv3→Lv4 cost 2,
  →Lv5 cost 3) plus Matt Ishida memory refunds and Death Claw's 1-cost removal
  let it stabilize without over-extending; the Megas (cost 3–4 over Lv5) are the
  expensive turns.

## Win conditions

1. **Threshold beatdown:** WereGarurumon (9000 on your turn) and the Mega bodies
   push through security while removal keeps the opponent's board thin.
2. **Recursion grind:** Nail Bone / CresGarurumon Digi-Burst keep re-deploying
   purple bodies from the trash faster than the opponent can answer; SkullSatamon
   and Dracmon refill hand from trash, so the deck rarely runs out of gas.
3. **Attrition removal:** Death Claw (×main + ×security), Devimon blocker, and
   VenomMyotismon Retaliation trade up on the opponent's attackers; Matt Ishida
   refunds memory off every own-Digimon deletion (including the Death Claw cost).

## Ranked interactions to test

1. **Trash-threshold beatdown (WereGarurumon online)** — the archetype's
   defining "fill ⇒ payoff" claim; assert the +2000 DP window flips on at 5 and
   off below 5 / off-turn. High-value: it's the one combo where the *whole* fill
   engine's job (cross multiple cards) is observable as a single DP delta, and
   the prior audit flagged ST6 lacks end-to-end play-throughs for exactly this.
2. **Death Claw → Matt Ishida memory loop** — two engine cards whose value only
   exists together; assert opp body removed **and** +1 memory **and** the
   unhappy path (no self-delete ⇒ no opp deletion; Matt only on your turn).
   High-value: catches a cross-card trigger-ordering / gating bug a per-card test
   can't see.
3. **Nail Bone double-recur** — biggest trash payoff; assert Lv3+Lv4 recurred
   free with On-Play **suppressed**, ineligible target untouched. Pairs as a
   deliberate **contrast** with CresGarurumon Digi-Burst (On-Play *fires*) — the
   suppression divergence is the system-level fact worth pinning.

**Dropped from the top tier (logged, not silently truncated):**
- **CresGarurumon Digi-Burst recur (On-Play fires)** — rank 4. Valuable only as
  the paired contrast to Nail Bone's suppression; low play frequency (12-cost
  Mega + Digi-Burst). Recommend authoring *if* the implementer wave covers the
  Nail-Bone contrast, else drop. Also note the audit's faithfulness call-out:
  the engine `[Main]` gates on a valid recur target existing (DCGO lets you whiff
  the Digi-Burst) — more player-friendly, hides no choice, not a bug.
- **VenomMyotismon Retaliation wall** — rank 5. Mostly a keyword test (combat
  interrupt), not a cross-card combo; hard to exercise end-to-end without a full
  attack/battle sequence. Per-card keyword coverage is the better home.
- **SkullSatamon / Dracmon trash-to-hand refills** — single-card cash-in
  effects with no second card required to express their value; per-card
  behavioral tests cover them. Mentioned in the model for completeness; not
  interaction-test candidates.
