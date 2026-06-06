# Machine (BT25 slice) — Model

Scope: the **BT25 "machine" slice** as dispatched by the author-set workflow —
cards **BT25-062 Kokuwamon**, **BT25-066 Guardromon**, **BT25-074 Tankdramon**.
This is a 3-card slice, not a meta deck; it is not present in
`data/deck_library.json` (so the `archetype-static-tests` CLI cannot run the
deck-legality / smoke-games invariants for it — see Phase-4 note below).

This is the capstone interaction pass: cards are assumed implemented and their
per-card behavioral tests green. The pass discovers that **2 of the 3 slice
cards are not implemented** (BLOCKED on a DSL vocab gap), which gates most of
the slice's natural combos out — see "Precondition gating" below.

## Card pool & roles

| Card | Status | Role | One-line function |
|------|--------|------|-------------------|
| BT25-062 Kokuwamon | **IMPLEMENTED** (DSL) | enabler / ramp | [SOMP] at ≤4 memory may free-digivolve self into a [Machine]/[Cyborg]/[TS] Digimon from hand; inherited +1000 DP; TS Lv2 cost-0 alt-path |
| BT25-066 Guardromon | **BLOCKED (dsl gap)** | wall | ＜Blocker＞; [All Turns] would-leave replacement by trashing 1 of its link cards; inherited +1000 DP |
| BT25-074 Tankdramon | **BLOCKED (dsl gap)** | payoff | [WD][WA][OPT] reveal top 3, play a cost≤12 [D-Brigade]/[ACCEL] Digimon among them at −5 cost, trash rest; [All Turns][OPT] on ally [D-Brigade]/[ACCEL] played, 1 opp Digimon can't digivolve; inherited ＜Reboot＞+＜Blocker＞ |

Verdicts: `qa/qa-reports/validated_cards_dsl.json` —
BT25-062 IMPLEMENTED; BT25-066 BLOCKED (gap_kind `dsl`); BT25-074 BLOCKED
(gap_kind `dsl`).

Cross-set partner used by the one authored combo (already implemented, not
pulled): **BT24-035 Gatomon** (Lv4 Yellow, traits Holy Beast/**Iliad/TS**) —
`[On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for the
turn.` YAML `cards/bt24/BT24-035.yaml`, test `cards_behavioral/bt24/bt24_035.rs`.

## Digivolution lines

- **Kokuwamon (Lv3 Black, Machine/Iliad/TS)** is the slice's only implemented
  line piece. Its `[Start of Your Main Phase]` clause free-digivolves *itself*
  (the source permanent) into any **[Machine] / [Cyborg] / [TS]** Lv-appropriate
  Digimon in hand, paying no cost, when the controller has **≤4 memory** on their
  turn. This is the engine of the line: it skips a turn of memory and a manual
  digivolve, and — crucially for combos — it is an **effect-initiated
  digivolution**, which the engine fires `WhenDigivolving` off of
  (`game_actions.rs::effect_initiated_digivolve_from_source_inner`, step 5).
- Guardromon (Lv4) and Tankdramon (Lv5) would extend the line, but both are
  unimplemented; the Lv3→Lv4→Lv5 machine line cannot be exercised end-to-end.

## Named combos

### Combo M1 — "Kokuwamon free-digivolve fires a [When Digivolving] payoff" (Kokuwamon → Gatomon)
- Cards: **BT25-062 Kokuwamon** (enabler), **BT24-035 Gatomon** (TS-trait
  payoff, cross-set, already implemented).
- Setup: Kokuwamon standalone on P0 field; Gatomon in P0 hand; ≤4 memory on
  P0's turn; an opponent Digimon present.
- Expected mechanical outcome: P0's `[Start of Your Main Phase]` prompt installs
  (the printed "may"); on accept + pick-Gatomon, Kokuwamon digivolves into
  Gatomon **for free** (no memory paid, hand −1, the stack now [Kokuwamon,
  Gatomon]); the effect-initiated digivolve fires Gatomon's `[When Digivolving]`,
  which debuffs **1 opponent Digimon by −3000 DP for the turn**. The net,
  system-level fact: a *free* Kokuwamon digivolve is also a *removal/debuff*
  enabler — the −3000 DP appears even though no manual digivolve happened.
- Rules/keyword basis:
  - `[Start of Your Main Phase]` is a triggered timing; "may … without paying
    the cost" → optional, cost-0 effect-initiated digivolution
    (DCGO `BT25/Black/BT25_062.cs`, `DigivolveIntoHandOrTrashCard(payCost:false)`).
  - An effect-driven digivolution **is** a digivolution and triggers
    `[When Digivolving]` (`general_rule.pdf` digivolution timing; engine:
    `effect_initiated_digivolve_from_source_inner` enqueues
    `EffectTiming::WhenDigivolving` then drains — `game_actions.rs:7088+`).
  - Gatomon's −3000 DP for the turn (DCGO `BT24/Yellow/BT24_035.cs`; YAML
    `add_dp_modifier value:-3000 expiry:end_of_turn`).
  - Trait gate: Gatomon carries the **TS** trait, satisfying Kokuwamon's
    "[Machine], [Cyborg] or [TS]" hand filter.
- Rank: HIGH — the only combo in the slice whose pieces are *both implemented*;
  exercises the cross-card trigger chain (enabler clause → partner's WhenDigivolving)
  that the two per-card tests cannot see in isolation.

### Combo M2 — "Machine line wall" (Kokuwamon → Guardromon) — BLOCKED
- Cards: BT25-062, **BT25-066 Guardromon** (unimplemented).
- Would be: Kokuwamon free-digivolves into Guardromon (Machine/TS) → a ＜Blocker＞
  wall that refuses to leave by trashing a link card.
- **Blocked on BT25-066** (DSL gap: no verb selects/trashes a permanent's own
  link card as a would-leave replacement cost). Interaction test **not
  authored**; routed to the implementation backlog + DSL-vocab-gap tracker.

### Combo M3 — "D-Brigade ramp payoff" (Kokuwamon → … → Tankdramon) — BLOCKED
- Cards: BT25-062, **BT25-074 Tankdramon** (unimplemented). Note Tankdramon is
  D-Brigade/ACCEL, not Machine-line-reachable from Kokuwamon's hand filter except
  via its Machine trait at the wrong level; the real payoff is its reveal-play
  clause.
- **Blocked on BT25-074** (DSL gap: reveal-pool play with a non-zero reduced
  cost is not expressible). Interaction test **not authored**; routed to backlog
  + DSL-vocab-gap tracker.

## Playstyle
- Class: midrange / tempo. Kokuwamon is a cost-cheating ramp piece that converts
  a low-memory board state into a free Lv4 with an immediate `[When Digivolving]`
  payoff, then sticks around as a +1000-DP inherited source. Guardromon (when
  implemented) is the defensive wall; Tankdramon (when implemented) is the
  top-end reveal-play engine.
- Memory curve: Kokuwamon's clause is **gated to ≤4 memory** — it rewards
  passing the turn near the floor, not after a big ramp.

## Win conditions
- Not a standalone deck at this slice size. The implemented piece (Kokuwamon)
  contributes free tempo + a trigger-platform for `[When Digivolving]` payoffs
  (here Gatomon's debuff; in a full machine build, Guardromon's wall / Tankdramon's
  removal). The deck would close via the Lv5+ payoffs once unblocked.

## Ranked interactions to test
1. **M1 Kokuwamon → Gatomon** (HIGH) — both pieces implemented; authored.
   - Happy path: ≤4 memory + Gatomon in hand → accept → free digivolve →
     −3000 DP on an opponent Digimon.
   - Unhappy path: same board but the only hand card is a non-TS/Machine/Cyborg
     Digimon → SOMP clause has no legal pick, never fires, no debuff appears (the
     payoff is gated on the *enabler reaching a valid partner*).
2. M2 Kokuwamon → Guardromon (BLOCKED on BT25-066) — **dropped**, logged.
3. M3 Kokuwamon → Tankdramon (BLOCKED on BT25-074) — **dropped**, logged.

## Precondition gating (Phase 4) — summary
- `archetype-static-tests -- "machine"` errors: archetype not in
  `deck_library.json`. The slice is a `--cards` list, not a library deck, so the
  CLI's deck-legality + smoke-games invariants are **not applicable** to this
  slice (recorded as `not_applicable`, not a failure).
- Coverage / combo-presence gate (from the DSL verdict tracker): only BT25-062
  is implemented. Combos M2/M3 name unimplemented cards → **blocked on the
  missing card**, not authored. Only M1 (both pieces present) is authored.
