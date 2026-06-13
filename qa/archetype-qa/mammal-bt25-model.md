# Mammal (BT25 slice) — Model

Scope: the **mammal** archetype slice of BT25 — four Lv.3 rookies that share the
`Mammal` / `Iliad` / `TS` trait spine and form the searchable rookie engine of
the BT25 "TS" (Tamer-Set / *Adventure*-crossover) decks. Capstone QA: cards
implemented + per-card behavioral tests green (28/28 in
`code/digimon-engine/tests/cards_behavioral/bt25/{bt25_022,bt25_030,bt25_031,bt25_078}.rs`).

Slice cards (from `--cards BT25-022,BT25-030,BT25-031,BT25-078`):

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| **BT25-022 Lunamon** (Blue, Data, Mammal/Iliad/TS, DP 2000) | engine (search) | `[On Play]` reveal 3, add 1 **[Iliad]** + 1 **[TS]** to hand (mandatory, no-duplicate), rest to bottom; inherited `<Jamming>`. |
| **BT25-030 Elecmon** (Yellow, Data, Mammal/Iliad/TS, DP 2000) | engine (memory/security) | `[Start of Your Main Phase]` (optional) by adding top security to hand, gain 1 memory; inherited `[When Attacking][OPT]` may add top security, then if 0 security `<Recovery +1>`. |
| **BT25-031 Patamon** (Yellow, Data, Mammal/Iliad/ADAMAS/TS, DP 2000) | engine (search) | `[On Play]` reveal 3, add 1 **[Angel]/[Archangel]/[Three Great Angels]/[Four Great Dragons]** + 1 **[TS]** to hand (mandatory, no-duplicate), rest to bottom; inherited `<Barrier>`. |
| **BT25-078 Gazimon** (Black/Purple, Virus, Mammal/Iliad/TS, DP 1000) | engine (search/tuck) | `[When Moving][On Play][OPT][OPT/turn]` reveal 3, **add 1 card with [Three Musketeers] in its text to hand** OR **place 1 [Three Musketeers]-trait card as this Digimon's bottom digivolution card**, rest to bottom; inherited `<Retaliation>`. |

Shared structural spine (the system glue, verified in each YAML + per-card test):
- **Cost-0 alt-digivolve from any Lv.2 with the `TS` trait**, in addition to the
  printed same-color Lv.2 path. Source: each card's `alt_paths` (`trait_has: TS`,
  `cost: 0`); DCGO `AddSelfDigivolutionRequirementStaticEffect(..., level:2)`
  gated on `TopCard.HasTSTraits`. This is the off-color splash enabler: a Yellow
  Patamon can sit on a Blue TS Lv.2, etc.
- **Every mammal carries both `Iliad` and `TS`** → each is a legal *target* of
  the others' reveal-search buckets. The search engine is self-feeding.

## Digivolution lines

These are Lv.3 rookies; the BT25 TS package digivolves them upward into the
crossover Lv.4+ payoffs (Angel line via Patamon, etc.). Within this slice the
load-bearing line fact is the **Lv.2 → Lv.3 entry**:

- any **Lv.2 [TS]** → (cost 0) → **Lunamon / Elecmon / Patamon / Gazimon**,
  regardless of the rookie's own color (off-color splash).
- printed same-color Lv.2 → rookie (cost 0) also registered.

Gazimon's entry is special: because its shared clause also fires `[On Play]`,
**digivolving a Lv.2 TS into Gazimon triggers its reveal-3** (the digivolve is
a play onto the field → `OnEnterFieldAnyone` / on_play). The free-digivolve is
thus also a search trigger.

## Named combos

### Combo MA1 — "Lunamon search adds a second mammal" (self-feeding engine)
- Cards: **BT25-022 Lunamon** + a second mammal in deck (e.g. **BT25-031 Patamon**
  as the `[Iliad]` pick) + any **[TS]** card (e.g. **BT25-030 Elecmon**).
- Expected mechanical outcome: Lunamon's `[On Play]` reveals 3; the `[Iliad]`
  bucket takes Patamon (Patamon has Iliad) and the `[TS]` bucket takes Elecmon
  (TS) — **both** go to hand; remainder to deck bottom. Hand +2, deck −2 net of
  the one returned filler. The system fact: a mammal is added to hand *as the
  Iliad pick of another mammal*, refueling the next rookie play.
- Unhappy variant: with only ONE mammal in the top-3 and no other Iliad/TS card,
  `no_duplicate_cards` forbids that one mammal filling both buckets, so only one
  bucket fills (a single card cannot be both the Iliad pick and the TS pick).
- Rules/keyword basis: DCGO `BT25_022.cs` `SimplifiedRevealDeckTopCardsAndSelect`
  with `mutualConditions: true` (= `no_duplicate_cards`), two `maxCount:1`
  buckets `HasIliadTraits` / `HasTSTraits`. `general_rule.pdf` reveal/return.
- Rank: HIGH (every-game [On Play], the engine's core loop).

### Combo MA2 — "off-color TS Lv.2 free-digivolves into a mammal"
- Cards: a synthetic **Lv.2 [TS]** in hand (cross-color) + **BT25-031 Patamon**
  (a Yellow rookie) standing on field as the Lv.2.
- Expected mechanical outcome: the TS-trait Lv.2 (any color) is a legal cost-0
  digivolution source for Patamon — the alt-digivolve recipe fires the
  digivolution at memory cost 0, even when the Lv.2's color ≠ Yellow. Confirms
  the off-color splash glue at the *recipe* level (the thing the slice is built
  around), beyond the structural `alt_paths.len()==2` per-card check.
- Rules/keyword basis: each YAML `alt_paths` `trait_has: TS, cost: 0`; DCGO
  `AddSelfDigivolutionRequirementStaticEffect(level:2)` gated on `HasTSTraits`.
- Rank: MEDIUM-HIGH (defines the deck's color identity; cross-card because the
  source is a *different* card than the mammal).

### Combo MA3 — "Gazimon [When Moving]/[On Play] tucks a [Three Musketeers] source under itself"
- Cards: **BT25-078 Gazimon** + a revealed **[Three Musketeers]-trait** card in
  deck top-3.
- Expected mechanical outcome: choosing the *place* branch tucks the
  [Three Musketeers]-trait card as Gazimon's **bottom** digivolution card (under
  its existing stack), NOT to hand; remainder to deck bottom. The alternative
  branch (text-match → hand) is the search mode. This is the dual-route engine.
- Unhappy variant: the whole clause is optional ("you may") — declining at the
  effect-choice / optional layer does nothing (no reveal consumed beyond return).
- Rules/keyword basis: DCGO `BT25_078.cs` `SetBoolSelection` add-to-hand vs
  `AddDigivolutionCardsBottom`; `canNoSelect: true` (optional). `general_rule.pdf`
  digivolution-card placement.
- Rank: MEDIUM (route-2 tuck is unique to Gazimon; partly covered per-card, but
  the *bottom-of-an-existing-stack* placement is a multi-source interaction).

### Combo MA4 — "Elecmon start-of-main converts security into memory ramp"
- Cards: **BT25-030 Elecmon** on field + ≥1 security.
- Expected mechanical outcome: optional `[Start of Your Main Phase]` — pay by
  moving top security to hand, gain +1 memory (security −1, hand +1, memory +1).
  Gated out at 0 security (cost unpayable). This is the slice's ramp engine.
- Rules/keyword basis: DCGO `BT25_030.cs` `isOptional:true`, gated on
  `SecurityCards.Count > 0`. Largely a single-card effect — covered well by the
  per-card test; included in the model for completeness but NOT re-authored as a
  multi-card interaction (see Plan / dropped list).
- Rank: LOW for *interaction* purposes (single-card; no cross-card surface).

## Playstyle

Tempo/combo midrange. The mammals are a **consistency engine**: cheap rookies
that dig 3 deep every play to assemble the TS crossover payoffs, splash colors
freely via the cost-0 TS alt-digivolve, and (Elecmon) trade security for memory
ramp. Memory curve is low — rookies are cost 3 / digivolve cost 0 from TS Lv.2.

## Win conditions

The slice itself does not close games; it feeds the BT25 TS Lv.4+ payoffs
(Angel/Three-Great-Angels line via Patamon's search, etc.) and ramps into them
(Elecmon). Inherited keywords harden attackers: `<Jamming>` (Lunamon),
`<Barrier>` (Patamon), `<Retaliation>` (Gazimon).

## Ranked interactions to test

1. **MA1** — Lunamon search adds a second mammal as the Iliad pick (+ TS pick) —
   HIGH: the self-feeding core loop; cross-card (one mammal searches up another).
   Includes the `no_duplicate_cards` unhappy path.
2. **MA2** — off-color TS Lv.2 → mammal cost-0 digivolve at the recipe level —
   MEDIUM-HIGH: the color-splash identity; the source is a different card.
3. **MA3** — Gazimon place-branch tucks a [Three Musketeers] source as the
   BOTTOM of an existing stack — MEDIUM: multi-source placement under a stack.

Dropped (logged, not silently truncated):
- **MA4 (Elecmon ramp)** — single-card effect, fully covered by the per-card
  test (`bt25_030_start_of_main_adds_top_security_and_gains_memory` + decline +
  zero-security gates). No cross-card surface → not re-authored as an
  interaction test.
- **Patamon Angel-bucket / Gazimon text-vs-trait routing** — single-card filter
  nuances already pinned per-card (`bt25_031_angel_bucket_rejects_non_angel_traits`,
  `bt25_078_add_to_hand_branch_moves_text_match_to_hand`). Not cross-card.
- **Elecmon inherited When-Attacking Recovery** — single-card, per-card-covered.

## Cross-set pulls (lazy — fired printed effects only)

Per the no-eager-closure rule, a cross-set card is pulled into a test ONLY if a
combo fires its *actual printed effect*. The three authored combos all run on the
four slice cards plus **synthetic neutral fixtures** (a generic Lv.2 TS source, a
[Three Musketeers]-trait filler, deck pad). No cross-set card's printed effect is
fired → **no cross-set implementation is pulled**. (Patamon-as-Iliad-target in
MA1 is a slice card, not cross-set; the Lv.2 TS source in MA2 is a synthetic
neutral whose only relevant property is the `TS` trait + level 2.)
