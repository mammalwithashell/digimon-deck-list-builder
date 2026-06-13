# Burst Digivolve — implementation plan (judge-quiz Q8 / G-BURST-ON-TURN-END-NOT-EXECUTED)

**Status:** scoped & de-risked 2026-06-02. NOT a "re-model" — Burst Digivolve is
an **unimplemented greenfield mechanic**. The DSL parses/compiles the path
(`kind: burst_digivolve`, `extra_cost`, `on_burst_turn_end`) but the engine has
**no burst-digivolve resolution path at all**, so the teardown, DP-less
rules-check, and `extra_cost` all hang off a resolution that does not exist.

## What Burst Digivolve is (BT13-020 ShineGreymon: Burst Mode, BT13-060 Rosemon: Burst Mode)

Printed: `[Burst Digivolve] [ShineGreymon]: By returning 1 [Marcus Damon] to
hand, cost 0. At the end of the burst digivolution turn, trash this Digimon's top
card.` A **main-phase digivolve** from the *named* Lv.6 prior form, paying an
extra cost (return a Tamer to hand), cost 0, with an end-of-turn self-teardown
that trashes the just-played top card — revealing the prior form (or, after a
de-digivolve, a sub-Lv3 card that then can't remain).

## Current (broken) state

- `dsl_cards/mod.rs:97-108` lumps `CompiledAltPathKind::BurstDigivolve` with
  `BlastDnaDigivolve` and lowers BOTH to a `blast_digivolve()` declarative marker
  (the combat **counter-window** blast). That is wrong for Burst — it is not a
  counter blast.
- `dna_digivolve.rs` digivolve-path matching (line ~250) and the literal-cost
  helpers (~825-855) **exclude** any path with `extra_cost` or `on_burst_turn_end`
  → the burst path is never offered as a digivolve option and never resolves.
- `extra_cost` is compiled (`CompiledAltPath::extra_cost`) but **never executed**
  anywhere in the engine (only referenced in the exclusion checks above).
- `on_burst_turn_end` is compiled (`CompiledAltPath::on_burst_turn_end`) but
  **never scheduled/executed** (only `!is_empty()` path-detection checks).
- BT13-020 / BT13-060 tests are **structural-only** for the burst clause; the
  behavioral test is explicitly deferred (see `bt13_020.rs:198-206`) pending "a
  burst-digivolve action helper" + engine teardown.

## Confirmed enablers (verified 2026-06-02)

- **Scheduling:** `EffectContext::schedule_delayed(when: EffectTiming, body:
  Vec<CompiledStep>, bindings)` → `Game.scheduled_effects`; drained by
  `scheduled_effects::fire_scheduled_for_timing(self, EffectTiming::EndOfYourTurn)`
  at the player's own turn-end (`game_phases.rs:794`). `on_burst_turn_end` is
  already `Vec<CompiledStep>` → schedule it with `when: EndOfYourTurn`.
- **Alt-path access:** `Game.alt_path_registry: HashMap<card_id, Vec<CompiledAltPath>>`
  (`pub(crate)`) exposes the compiled burst path (with `extra_cost` +
  `on_burst_turn_end`) per card.
- **Stack mutation:** `Permanent::digivolve(card: CardSource, turn)` (the move
  used by `execute_blast_digivolve`).

## Build plan (each step TDD; the whole thing wants its own OpenSpec change)

1. **Lowering split** — in `dsl_cards/mod.rs`, stop emitting the blast marker for
   `BurstDigivolve`; only `BlastDnaDigivolve` keeps the counter-window marker.
   (BT13-020 tests are structural and don't assert the marker — verify no combat
   counter-window test exercises a burst card.)
2. **`extra_cost` execution primitive** — run a path's `extra_cost` steps (e.g.
   `return_to_hand` a selected [Marcus Damon]) through an `EffectContext` as part
   of the burst-digivolve cost. Surfaces a selection (which Tamer) → no
   auto-select (rule 17). Likely reusable for `ActivatedDigivolve` (also
   unimplemented).
3. **Action-space exposure** — offer the burst-digivolve as a digivolve option
   from the *named* prior form in the action mask + decoder (so an RL agent can
   choose it — rule 17). This is the largest piece; model on the existing
   alt-path digivolve generation but route burst paths (currently excluded at
   `dna_digivolve.rs:250`) into their own handler that applies `extra_cost` + cost 0.
4. **Resolution + teardown scheduling** — on burst-digivolve resolution: pay
   `extra_cost`, `Permanent::digivolve` the card on, fire `WhenDigivolving`, then
   `schedule_delayed(EndOfYourTurn, on_burst_turn_end, …)`.
5. **Teardown execution** — at `EndOfYourTurn` the scheduled `on_burst_turn_end`
   (`trash_top_source`) runs, trashing the burst card's top.
6. **DP-less / sub-Lv3 "can't remain" rules-check** — after the teardown trash,
   if the revealed top card is below Lv3 / DP-less it cannot remain in the battle
   area → delete it. (Check whether a related rule already exists for
   de-digivolve "can't trash past Lv3"; this is the *reveal-then-can't-remain*
   variant.)
7. **DebugRunner burst-digivolve driver** + **full Q8 test**
   (`q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot`): Comet Hammer
   (BT23-096) de-digivolves the burst stack to Agumon (EX4-005) → at EoT the burst
   teardown trashes Agumon → the revealed DP-less Koromon (BT21-004) can't remain.
   All Q8 cards are already implemented.

## Recommendation

Scope as a dedicated OpenSpec change (`add-burst-digivolve`). Steps 2 + 3
(`extra_cost` execution and action-space exposure) are the substantial,
design-bearing pieces; 4-6 are mechanical once 1-3 land. Pin BT13-020 / BT13-060
behavioral teardown tests alongside the judge-quiz Q8 scenario.
