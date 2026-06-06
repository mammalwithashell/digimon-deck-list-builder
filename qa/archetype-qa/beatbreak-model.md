# BEATBREAK (BT25 "Glowing Dawn") — Model

> Authored by `/archetype-interaction-test-author` for the **beatbreak** slice of
> BT25. Scope (as given): `BT25-003, BT25-032, BT25-046, BT25-079, BT25-035,
> BT25-049, BT25-081, BT25-041, BT25-057, BT25-088, BT25-090`.
>
> **Implementation status (Phase-4 precondition gate, 2026-06-06):** only 4 of the
> 11 slice cards are implemented (YAML present + loads into the embedded DSL
> pack): **BT25-003 Frimon, BT25-032 Liollmon, BT25-046 Gekkomon, BT25-079
> Hyemon**. The other 7 (BT25-035 Cougarmon, BT25-041 Murasamemon, BT25-049
> Armalizamon, BT25-057 Monarchlizamon, BT25-081 Fangmon, BT25-088 Kyo Sawashiro,
> BT25-090 Tomoro Tenma) have **empty (0-line) per-card test stubs and no YAML** —
> they are **BLOCKED / unimplemented**. Per the skill's gating, combos naming any
> of those 7 are **dropped** (logged below), not authored.
>
> Caveat carried forward from per-card authoring: the BT25 per-card behavioral
> files (`cards_behavioral/bt25/bt25_00{3,32,46,79}.rs`) were authored but **never
> run green** (the shared `cards_behavioral` binary currently fails to compile on
> stale API: `GameEventKind`, `CompiledCost::Reduce`, `event.kind`,
> `has_player_modifier`). The card **YAML** themselves compile and load (verified:
> the `archetypes` binary loads BT25 DSL cards). The interaction tests below are
> authored against the **current** `archetypes`/`support.rs` API and run in that
> separate, green binary.

## Card pool & roles

| Card | Impl | Role | One-line function |
|------|------|------|-------------------|
| BT25-003 Frimon (Lv.2 Y, Lesser/Glowing Dawn/BEATBREAK) | ✅ YAML | enabler (inherited) | Inherited [When Attacking][OPT] **may** trash top security to digivolve into a [Glowing Dawn] hand card, **cost −1**. |
| BT25-032 Liollmon (Lv.3 Y, Holy Beast/Glowing Dawn/BEATBREAK, DP2000) | ✅ YAML | engine | [On Play] reveal 3, add 1 [Glowing Dawn] + 1 **yellow** [BEATBREAK] to hand, bottom rest. Inherited `<Barrier>`. Alt-path Lv.2 Y cost-0. |
| BT25-046 Gekkomon (Lv.3 **G**, Reptile/Glowing Dawn/BEATBREAK, DP2000) | ✅ YAML | engine | [On Play] reveal 3, add 1 [Glowing Dawn] + 1 **green** [BEATBREAK] to hand, bottom rest. Inherited `<Piercing>`. Alt-path Lv.2 G cost-0. |
| BT25-079 Hyemon (Lv.3 **P**, Beast/BEATBREAK, DP2000) | ✅ YAML | tech / floodgate | [All Turns] **both players** can't gain memory except by Tamer effects. Inherited `<Retaliation>`. Alt-path Lv.2 P cost-2. |
| BT25-035 Cougarmon (Lv.4 Y) | ❌ BLOCKED | payoff | [On Play/WhenDigi] −3000 DP; trash 2 face-downs under a Tamer to free-digivolve into a [Glowing Dawn] card. |
| BT25-049 Armalizamon (Lv.4 G) | ❌ BLOCKED | payoff | [WhenDigi] suspend opp; [Your Turn][OPT] cost −3 on [Glowing Dawn] Options by trashing a face-down. |
| BT25-081 Fangmon (Lv.4 P) | ❌ BLOCKED | tech | [WhenDigi] suspend non-purple Tamer; gain 1 memory on opp Tamer suspend. |
| BT25-041 Murasamemon (Lv.5 Y) | ❌ BLOCKED | payoff | `<Alliance>`; [WhenDigi/WhenAtk][OPT] play/use a [Glowing Dawn] card at −3. |
| BT25-057 Monarchlizamon (Lv.5 G/P) | ❌ BLOCKED | payoff | [WhenDigi/WhenAtk][OPT] De-Digivolve 1 by trashing a Tamer face-down; extra battle. |
| BT25-088 Kyo Sawashiro (Tamer Y) | ❌ BLOCKED | tamer | [Security] play for free. |
| BT25-090 Tomoro Tenma (Tamer G) | ❌ BLOCKED | tamer / engine | memory-set-to-3 floor; bank 2 face-downs under Tamer on any suspend; cost −1 on [Glowing Dawn] Options. |

**System shape:** a multi-color (Y/G/P) "Glowing Dawn" midrange engine whose
recurring resource is **face-down cards banked under Tamers** (Tomoro Tenma banks
them; the Lv.4/5 payoffs spend them) plus **trashing the top security card** for
tempo digivolves (Frimon). The yellow line revolves around cheap aggressive
digivolves (Frimon → Liollmon → Murasamemon/Cougarmon), green around Option
cost-reduction (Gekkomon → Armalizamon + Tomoro), and purple as a memory
floodgate / Retaliation tech (Hyemon → Fangmon). Inherited keywords (Barrier,
Piercing, Retaliation) ride up the digivolution stack.

## Digivolution lines

- **Yellow:** Frimon (Lv.2, alt nothing) → **Liollmon** (Lv.2 Y cost-0 alt-path)
  → Cougarmon (Lv.4) → Murasamemon (Lv.5). Evo-cost gates: Liollmon `{Lv2,Y,0}`.
- **Green:** (Lv.2) → **Gekkomon** (Lv.2 G cost-0 alt-path) → Armalizamon (Lv.4)
  → Monarchlizamon (Lv.5 G/P). Evo-cost gate: Gekkomon `{Lv2,G,0}`.
- **Purple:** (Lv.2) → **Hyemon** (Lv.2 P cost-2 alt-path) → Fangmon (Lv.4).
  Evo-cost gate: Hyemon `{Lv2,P,2}`.

Frimon's inherited clause performs an **effect-initiated digivolve** that *does
not* bypass requirements (it uses `effect_initiated_digivolve`, not the
`_ignore_requirements` variant), so the engine still demands a matching evo cost
(level **and** colour) on the target — see
`game_actions.rs::effect_initiated_digivolve_from_source_inner` (the
`matching_memory_cost` find over `evo_costs`, `general_rule.pdf` digivolution
requirements).

## Named combos

### B1 — Frimon tempo-digivolve into Liollmon (security-trash, cost −1)
- **Cards:** BT25-003 Frimon (inherited carrier) + BT25-032 Liollmon (yellow
  Glowing Dawn target in hand).
- **Expected mechanical outcome:** on Frimon's attack, accepting the inherited
  `[When Attacking]` clause **trashes the top security card** (security −1) and
  **digivolves Frimon into Liollmon** (hand −1; carrier top becomes Liollmon;
  the stack now holds Frimon under Liollmon). Liollmon's `[On Play]` reveal
  **does NOT fire** — this is a *digivolve*, not a *play* (`OnPlay` ≠
  `WhenDigivolving`/`OnDigivolve` in the lowering, `lower_triggered.rs`), so no
  reveal/add happens and hand does not gain +2. Liollmon's inherited `<Barrier>`
  is now the top card's keyword carrier.
- **Rules/keyword basis:** Frimon text (cards.json BT25-003); Liollmon evo-cost
  `{level:2, card_color:2(yellow), 0}` matches Frimon (Lv.2 yellow) →
  digivolve legal. `general_rule.pdf` §16 (`<Barrier>` glossary); digivolution
  timing. DCGO: BT25_032.cs reveal is gated by `CanTriggerOnPlay` (play-only).
- **Rank:** HIGH — Frimon is the deck's core tempo enabler and Liollmon its
  cheapest Glowing Dawn payoff; this is the line the yellow build opens on.

### B2 — Frimon's trait-picker over-offers a colour-illegal target (Gekkomon)
- **Cards:** BT25-003 Frimon (yellow) + BT25-046 Gekkomon (**green** Glowing
  Dawn target in hand).
- **Expected mechanical outcome:** Frimon's `select_hand` picker filters by
  **trait only** (`trait_has: Glowing Dawn`), so it **offers Gekkomon** even
  though Gekkomon is green. But the digivolve substrate enforces colour: green
  Gekkomon's evo cost is `{level:2, card_color:3(green)}`, which does **not**
  match Frimon's yellow → the `effect_initiated_digivolve` is **rejected**
  ("no matching evo cost"). Net: **no colour-illegal digivolve happens** — Frimon
  stays Frimon (carrier top unchanged), no Gekkomon enters the field.
- **Why it's a system-level fact:** the per-card test only ever puts a *same
  colour* Glowing Dawn in hand, so it never exercises the gap between Frimon's
  trait-only *picker* and the digivolve's colour *requirement*. This combo pins
  that the colour gate (not just the trait gate) governs the actual evolution.
- **Source / candidate-finding note:** the load-bearing detail to confirm is
  whether the **security cost is paid (top security trashed) even when the
  digivolve is then rejected** for colour. If security is consumed with no
  digivolve, that is a candidate faithfulness finding (cost paid for nothing) and
  must be confirmed against DCGO (BT25_003.cs is ABSENT — authored from text) +
  `general_rule.pdf` cost/effect ordering before filing. The test records the
  observed behavior either way.
- **Rules/keyword basis:** `game_actions.rs` evo-cost colour match; Gekkomon
  evo-cost (cards.json BT25-046 `evo_costs`).
- **Rank:** MEDIUM — exercises the substrate edge between picker and requirement.

### B3 — Hyemon floodgate blocks a non-Tamer effect memory gain (both players)
- **Cards:** BT25-079 Hyemon (floodgate) + a neutral Digimon whose `[On Play]`
  gains memory (synthetic probe: the engine's `TEST-001` "On Play: gain 1
  memory", a **non-Tamer** source).
- **Expected mechanical outcome:** while Hyemon is on the field, the
  `CannotGainMemoryExceptFromTamers` player-modifier is installed for **both**
  players; playing the non-Tamer +1-memory Digimon yields **no memory gain**
  (`ctx.gain_memory` early-returns because the source is not a Tamer —
  `effect_context/mod.rs:2238`). The **control**: with Hyemon absent, the same
  play **does** gain +1 memory.
- **Why it's a system-level fact:** the per-card test
  (`bt25_079.rs::..._installs_for_both_players`) only asserts the *modifier is
  installed*; it explicitly leaves the *actual block* as a follow-up. This combo
  drives a real non-Tamer memory-gain effect through the floodgate and asserts
  the gain is suppressed — the behavior the deck actually relies on.
- **Rules/keyword basis:** Hyemon text (cards.json BT25-079); DCGO BT25_079.cs
  `CannotAddMemoryClass` with `PlayerCondition: player==true` (both) and
  `CardEffectCondition: !IsTamerEffect`. Engine consult site
  `effect_context/mod.rs::gain_memory`.
- **Rank:** HIGH — Hyemon is the purple tech floodgate; its both-player,
  Tamer-only gate is the card's whole point and the most disruptive interaction.

## Playstyle

- **Class:** multi-color midrange/tempo "combo-lite". Tempo from cheap
  cost-reduced digivolves (Frimon, alt-paths) and security-trash acceleration;
  grind from Tamer face-down banking + Option cost reduction. Memory curve sits
  low (Tomoro Tenma floors you at 3) with Hyemon as a disruptive floodgate.

## Win conditions

- Climb the Glowing Dawn line to Lv.5 (Murasamemon `<Alliance>` /
  Monarchlizamon extra-battle) and push with inherited Piercing/Barrier riding
  the stack, trading security via Frimon while Hyemon starves the opponent's
  memory. (Most of this win path is **BLOCKED on unimplemented Lv.4/5 payoffs**.)

## Ranked interactions to test (Phase-3 selection)

Authored (all pieces implemented):
1. **B1** — Frimon → Liollmon tempo digivolve (HIGH).
2. **B3** — Hyemon floodgate blocks a non-Tamer memory gain (HIGH).
3. **B2** — Frimon picker over-offers colour-illegal Gekkomon (MEDIUM).

**Dropped (BLOCKED on unimplemented cards — logged, not silently truncated):**
- Frimon/Liollmon → **Cougarmon** free-digivolve −3000 DP chain — blocked on
  BT25-035.
- **Murasamemon** `<Alliance>` + play a Glowing Dawn at −3 — blocked on BT25-041.
- **Tomoro Tenma** banks face-downs → **Armalizamon/Monarchlizamon** spend them
  (the deck's core face-down engine) — blocked on BT25-049/057/090.
- **Fangmon** punishes opp-Tamer suspends for memory while **Hyemon** floodgates —
  blocked on BT25-081.
- **Kyo Sawashiro** [Security] free-play tempo — blocked on BT25-088.
- Hyemon floodgate vs a **Tamer-sourced** gain (the "allowed" half of B3) —
  needs a Tamer card with a memory-gain effect; deferred (no slice Tamer is
  implemented, and using an out-of-slice Tamer adds non-slice surface).
