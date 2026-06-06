# BEATBREAK (BT25 "Glowing Dawn") — Model

> Authored by `/archetype-interaction-test-author` for the **beatbreak** slice of
> BT25.
>
> **Pass 1 (earlier) scope:** `BT25-003, BT25-032, BT25-046, BT25-079` + the seven
> below; at that time only BT25-003/032/046/079 were implemented and combos B1/B2/B3
> were authored in `tests/archetypes/` against them.
>
> **Pass 2 (THIS pass, 2026-06-06) scope:** `BT25-088, BT25-090, BT25-035, BT25-049,
> BT25-081, BT25-041, BT25-057`. Implementation state **has changed** since Pass 1.
> Re-running the Phase-4 precondition gate against the live tree:
>
> | Card | YAML | per-card test | Status |
> |------|------|---------------|--------|
> | BT25-035 Cougarmon | ✅ `cards/bt25/BT25-035.yaml` | ✅ `bt25_035.rs` (green) | IMPLEMENTED (PARTIAL — see below) |
> | BT25-041 Murasamemon | ✅ `BT25-041.yaml` | ✅ `bt25_041.rs` (green) | IMPLEMENTED (PARTIAL) |
> | BT25-049 Armalizamon | ✅ `BT25-049.yaml` | ✅ `bt25_049.rs` (green) | IMPLEMENTED (PARTIAL) |
> | BT25-081 Fangmon | ✅ `BT25-081.yaml` | ✅ `bt25_081.rs` (green) | IMPLEMENTED (full) |
> | BT25-088 Kyo Sawashiro | ✅ `BT25-088.yaml` | ✅ `bt25_088.rs` (green) | IMPLEMENTED (PARTIAL) |
> | BT25-090 Tomoro Tenma | ✅ `BT25-090.yaml` | ✅ `bt25_090.rs` (green) | IMPLEMENTED (PARTIAL) |
> | BT25-057 Monarchlizamon | ❌ no YAML | ❌ 0-byte `bt25_057.rs` stub | **BLOCKED / unimplemented** |
>
> Verified 2026-06-06: the `cards_behavioral` binary now **compiles and runs green**
> (46/46 across the six implemented cards) — the Pass-1 caveat about it failing to
> compile is **stale**. The `archetypes` binary is also green (50/50 baseline).
>
> **The shared BLOCKED clause across the six PARTIAL cards** is the BEATBREAK
> cost-engine payoff — the cost-reduction / free-digivolve unlocked by *trashing
> face-down cards banked under your Tamers* (engine gap
> `G-COST-REDUCTION-INTERACTIVE-PAY-COST` + DSL-vocab gap
> `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER`). The IMPLEMENTED clauses are the
> board-affecting halves: −DP, suspend, memory gain, the face-down *banking*
> itself (Tomoro/Kyo), the inherited keyword grants, and Murasamemon's inherited
> [End of Attack] unsuspend (which spends a face-down as a **process** cost, not a
> cost-reduction pay_cost, so it is NOT blocked). The Pass-2 combos below test only
> these implemented clauses; combos whose payoff is the BLOCKED cost-engine, and
> any combo naming BT25-057, are dropped + logged.

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

---

## Pass-2 named combos (2026-06-06) — the now-implemented Lv.4/5 + Tamer slice

These exercise the cards that were BLOCKED in Pass 1 and are now IMPLEMENTED
(BT25-035/041/049/081/088/090). They live in
`code/digimon-engine/tests/archetypes/beatbreak_bt25.rs`. Each test fires only
the *implemented* clauses of these PARTIAL cards (never the BLOCKED cost-engine
payoff). Cross-set evolution prerequisites are synthesized via `make_test_card`
neutral fixtures; **no** cross-set card's printed effect is fired, so no
cross-set implementation is pulled (lazy closure honored).

### BB-IT1 — Armalizamon's suspend feeds Tomoro Tenma's face-down banker
- **Cards:** BT25-049 Armalizamon (suspends an opp Digimon) + BT25-090 Tomoro
  Tenma (banks 2 face-down on *any* Digimon suspend).
- **Expected mechanical outcome:** Armalizamon's `[On Play]/[When Digivolving]`
  *optional* suspend of an opponent Digimon is a **Digimon-suspend event**;
  Tomoro's `[All Turns] "When any Digimon suspend"` clause fires off it. Accepting
  Tomoro's optional clause **suspends Tomoro** (the activation cost) and
  **banks the top 2 deck cards face-down under Tomoro** (deck −2, Tomoro's source
  count 1→3, both placed sources face-down). The opponent Digimon is left
  suspended. The system fact a per-card test can't show: Tomoro's banker is fed
  by **another slice card's** suspend (the per-card 090 test calls
  `game.suspend` directly).
- **Rules/keyword basis:** DCGO `BT25_049.cs` `SelectPermanentEffect Mode.Tap`
  (opp Digimon); DCGO `BT25_090.cs` `OnTappedAnyone` gated on
  `IsPermanentExistsOnBattleAreaDigimon` (ANY Digimon) → `AddDigivolutionCardsBottom
  isFacedown:true`; `general_rule.pdf` suspend/trigger timing.
- **Rank:** HIGH — this *is* the deck's face-down resource engine (the suspend
  payoffs fuel the Tamer banker that the Lv.4/5 payoffs later spend).

### BB-IT2 — Tomoro banks the fuel that Murasamemon's inherited unsuspend spends
- **Cards:** BT25-090 Tomoro Tenma (banks a face-down under itself, a Tamer) +
  BT25-041 Murasamemon (inherited `[End of Attack][OPT]`: trash a bottom
  face-down under any Tamer → this `[Glowing Dawn]` Digimon unsuspends).
- **Expected mechanical outcome:** after Tomoro has banked face-down cards under
  itself (via BB-IT1's banker), a suspended Glowing-Dawn host carrying Murasamemon
  as an inherited source fires Murasamemon's `[End of Attack]` clause: it
  **trashes one bottom face-down under Tomoro** (trash +1) and **unsuspends the
  host**. Unhappy path: with **no** banked face-down anywhere, the trash cost is
  unpayable → the host stays suspended. The system fact: the producer (090) and
  consumer (041) of the face-down resource are **different cards** — the per-card
  041 test hand-sets `card_sources[0].face_down = true` on a synthetic Tamer; here
  the face-down is produced by Tomoro's *real* banking clause.
- **Rules/keyword basis:** DCGO `BT25_090.cs` `AddDigivolutionCardsBottom`
  (face-down) + DCGO `BT25_041.cs` inherited `EndOfAttack` `IsTamerWithFaceDownCard`
  → `TrashDigivolutionCardsFromTopOrBottom(isFromTop:false)` → unsuspend self;
  `general_rule.pdf` §16 inherited-effect / End-of-Attack timing.
- **Rank:** HIGH — closes the face-down engine loop (bank → spend → unsuspend for
  a second attack), the grind plan of the deck.

### BB-IT3 — Cougarmon −DP + Armalizamon suspend stack on one target (combined soften+tap)
- **Cards:** BT25-035 Cougarmon (`[On Play]/[WD]` −3000 DP to an opp Digimon) +
  BT25-049 Armalizamon (`[On Play]/[WD]` optional suspend of an opp Digimon).
- **Expected mechanical outcome:** both payoffs target the **same** opponent
  Digimon: after both resolve it is at **−3000 effective DP** (turn-scoped) **and
  suspended**. Net board: one opp Digimon softened (easier to out-DP / delete in
  battle) and tapped (can't block) — the combined removal-prep the deck plays for.
  Asserts the two independent debuffs **coexist** on one permanent and that the
  −3000 expires at end of turn while the suspend persists (different lifetimes).
- **Rules/keyword basis:** DCGO `BT25_035.cs` `-3000 DP UntilEachTurnEnd`; DCGO
  `BT25_049.cs` `Mode.Tap`; `general_rule.pdf` modifier-expiry vs suspend-state.
- **Rank:** MEDIUM — two payoffs converging on one target is the deck's standard
  removal setup; pins that a turn-scoped DP modifier and a suspend stack
  independently and expire independently.

### Pass-2 dropped (logged, not silently truncated)
- **Monarchlizamon (BT25-057)** any combo — BLOCKED: 0-byte test stub, no YAML.
  The "Tomoro banks → Monarchlizamon De-Digivolve by trashing a face-down" loop
  and "Armalizamon→Monarchlizamon green line + extra battle" are dropped.
- The **cost-engine payoff** of every PARTIAL card (Cougarmon free-digivolve into
  a Glowing-Dawn hand card by trashing 2 face-downs; Armalizamon/Tomoro/Kyo
  Option/play **cost −N** by trashing a face-down; Murasamemon play/use a Glowing
  Dawn at −3) — BLOCKED on `G-COST-REDUCTION-INTERACTIVE-PAY-COST` /
  `G-TRASH-N-BOTTOM-FACE-DOWN-UNDER-TAMER`. These are the deck's defining combos
  but the substrate cannot express them yet; not authored.
- **Kyo Sawashiro on-lose-security banker → spends under a Lv.4/5 payoff** —
  Kyo's banking IS implemented, but its only *implemented* downstream consumer in
  this slice is Murasamemon's inherited unsuspend, already covered by BB-IT2 using
  Tomoro (the more on-curve banker). A Kyo-fed variant would be a near-duplicate;
  dropped to avoid redundancy (BB-IT2 already pins "any Tamer's face-down is a
  legal Murasamemon cost", which subsumes Kyo).
