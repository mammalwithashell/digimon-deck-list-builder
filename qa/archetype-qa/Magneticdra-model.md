# Magneticdra — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phases 0–3). Durable, reviewable system model of the **Magneticdra** archetype
> — the mono-Black **Mineral/Rock [LIBERATOR]** "Rocks" engine built around the
> Lv.7 apex **Magneticdramon (EX10-036)**. Sources cited inline: DCGO C# path
> (`$BASE_DCGO/Assets/Scripts/CardEffect/...`, resolve with
> `BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`)
> and/or `general_rule.pdf` §16 keyword rule numbers — DCGO + the PDF outrank the
> card-text JSON per CLAUDE.md source priority.
>
> Pool resolved with `python code/tools/resolve_deck.py "Magneticdra" --json`
> (33 decklists, 40 unique cards). Per-card DSL verdicts read from
> `qa/qa-reports/validated_cards_dsl.json`: the entire competitive core is
> **IMPLEMENTED** in the Rust DSL. NOT-IN-DSL splash cards (BT10-062 Golemon,
> EX1-071 Win Rate 60%!, LM-025 Cyberdramon, LM-033 Garnet Memory Boost!) and
> the PARTIAL BT23-058 Craniamon are **excluded** from authored combos.
>
> Magneticdra is a sibling/superset of the **Rocks** archetype
> (`qa/archetype-qa/Rocks-model.md`). This model is **Option-card-centric** per
> the task focus: the named combos prioritise interactions that route through an
> Option (EX10-069 Gravel Hearts, LM-031 Black Scramble, P-107 Defense Training,
> EX8-070 Zofr Kabus). Where a combo overlaps the Rocks model, the focus differs
> (Option enabler/chain rather than the Digimon-only payoff).

## The central engine (read this first)

Magneticdra is a **source-trash value engine**. Its Mineral/Rock Digimon carry
*inherited* effects that fire on the timing **"when effects trash this card from
a [Mineral] or [Rock] trait Digimon's digivolution cards"** (DCGO timing
`OnDigivolutionCardDiscarded`, gated on the *trashed-from* permanent's top card
carrying the Mineral/Rock trait). Mid/high Digimon **and Options** exist to
**trash those buried sources as a cost** — each trashed source simultaneously
"pays the cost" *and* fans out **one inherited trigger per trashed source**. So a
single activation chains into multiple deletes / de-digivolves / memory gains.

The **Option** half of the deck plugs into this engine from three directions:
1. **Enable / cheat-play the engine pieces** — Gravel Hearts (EX10-069) free-plays
   a [Sunarizamon] or [Close] from hand *or trash*; Black Scramble (LM-031),
   Defense Training (P-107), Gravel Hearts' Delay all give a **cost-reduced
   effect-initiated digivolve** up the Mineral/Rock spine.
2. **Pull the source-trash cost themselves** — Zofr Kabus (EX8-070) trashes a
   Mineral/Rock digivolution source as its own cost, **firing the inherited
   trigger of whatever it trashed** while also buffing the Digimon.
3. **Recur from trash** — Black Scramble's Delay returns a black Digimon from
   trash and can free-replay a small black Digimon; Gravel Hearts free-plays
   from trash.

**Inherited "when my source is trashed" payloads** (the fan-out targets):
| Source card | Inherited payload on trash |
|-------------|----------------------------|
| EX8-047 / EX10-025 / EX8-048 / BT21-055 Sunarizamon/Landramon | delete 1 opp Digimon with play cost ≤ 4 (`$BASE_DCGO/.../EX8/Black/EX8_047.cs`, `EX8_048.cs`) |
| EX8-051 / EX10-032 Proganomon, P-167 Landramon | ＜De-Digivolve 1＞ 1 opp Digimon (`$BASE_DCGO/.../EX8/Black/EX8_051.cs`; De-Dig §16-11) |
| EX8-005 / EX10-003 Tumblemon (DigiEgg) | gain 1 memory (`$BASE_DCGO/.../EX8/Black/EX8_005.cs`) |

**Source-trashing payoffs / activators:**
| Card | What it trashes / does |
|------|------------------------|
| EX10-036 Magneticdramon | WD/WA: trash **3** Mineral/Rock sources → delete 1 opp Digimon + trash their top security (`$BASE_DCGO/.../EX10/Black/EX10_036.cs`) |
| EX10-033 / EX8-055 Pyramidimon | WD/WA: trash up to 3 sources → reduce opp cost / unsuspend; re-bury 3 from trash |
| EX10-032 / EX8-051 Proganomon | WD/WA: trash 1 source → grant Collision/Piercing + DP |
| **EX8-070 Zofr Kabus (Option)** | **[Main] trash 1 source → grant Collision/Piercing/Reboot/anti-bounce/+3000** (the source-trash fan-out is the combo) |
| EX10-028 Landramon | OnPlay/WD: trash 1 source → Reboot+Blocker+DP buff |

**Re-bury / refuel engines** (recursion): EX8-067 / EX10-063 / P-169 Close
(Tamers, suspend → place sources from trash + memory), Magneticdramon and
Pyramidimon's own "place from trash as bottom sources" clauses, EX10-025
Sunarizamon `[On Play]`.

## Card pool & roles

| Card | Role (payoff/enabler/engine/tech) | One-line function |
|------|------|-------------------|
| EX10-036 Magneticdramon | payoff (Lv7) | Fragment(3); WD/WA trash 3 sources → delete 1 opp Digimon + trash opp top security; re-bury 3 → unsuspend |
| EX10-069 **Unique Emblem: Gravel Hearts** (Option) | enabler (Option) | [Main] free-play Sunarizamon/Close from hand or trash; Delay: when a Close suspends, cost-reduced (−3) digivolve into Mineral+LIBERATOR in hand |
| LM-031 **Black Scramble** (Option) | enabler (Option) | [Main] black Digimon digivolves into a black Digimon in hand cost −3; Delay returns/replays from trash; [Security] free-play small black from trash |
| P-107 **Defense Training** (Option) | enabler (Option) | [Main] reveal-2 add black + place self; Delay: a Digimon digivolves into a black Digimon in hand cost −2 |
| EX8-070 **Zofr Kabus** (Option) | engine (Option) | [Main] trash 1 Mineral/Rock source → that Digimon gains Collision/Piercing/Reboot/anti-bounce/+3000 until EoOT; [Security] delete opp lowest-cost |
| P-039 Black Memory Boost! (Option) | ramp (Option) | [Main] reveal-4 add black Digimon; Delay: +2 memory |
| P-206 Digital Gate Open (Option) | ramp (Option) | reveal-3 add Digimon+Tamer; Delay: play a Tamer cost −4 |
| BT9-103 Kongou (Option) | tech (Option) | opp cost ≤7 can't attack players; opp can't add security this turn |
| EX8-047 Sunarizamon | enabler+inherited | [On Play] reveal-3 add Mineral/Rock + LIBERATOR; **inherited trash → delete opp cost ≤4** |
| EX10-025 Sunarizamon | enabler+inherited | [On Play] place 2 Mineral/Rock from trash as a Digimon's bottom sources; **inherited trash → delete opp cost ≤4** |
| BT21-055 Sunarizamon | enabler+inherited | when digivolving into Mineral/Rock reduce cost by 1; **inherited trash → delete opp cost ≤4** |
| P-167 Landramon | engine+inherited | [SoMP]/[WD] trash 1 source → reveal-3 dig; **inherited trash → De-Digivolve 1** |
| EX8-048 Landramon | enabler+inherited | [WD] free-play [Close] if ≤1 Tamer; **inherited trash → delete opp cost ≤4** |
| EX10-028 Landramon | engine+inherited | OnPlay/WD trash 1 source → Reboot/Blocker/+3000; **inherited trash → delete opp cost ≤4** |
| EX10-032 Proganomon | engine+inherited | [Hand][Main] Close-gated cheat-evolve; WD/WA trash 1 source → Collision/Piercing/+3000; **inherited trash → De-Dig 1** |
| EX8-051 Proganomon | payoff+inherited | static Collision/Piercing/Fragment(3); **inherited trash → De-Dig 1** |
| EX10-033 Pyramidimon | payoff | Fragment(3); WD/WA re-bury 3 + trash ≤3 sources → reduce opp cost by 2/each |
| EX8-055 Pyramidimon | payoff | Fragment(3); WD/WA trash 3 sources → unsuspend + ＜Sec.A.+1＞; EoT re-bury 3 |
| EX8-067 Close (Tamer) | engine | [SoT] set memory to 3; on Mineral/Rock digivolve, suspend → place ≤2 sources from trash |
| EX10-063 Close (Tamer) | engine | [SoMP] cheat-play Close + Sunarizamon from trash; on source-trash, suspend → +1 memory |
| P-169 Close (Tamer) | engine | [SoMP] +1 memory if opp has Digimon; on source-trash, suspend → place a source from trash |
| EX8-005 Tumblemon (egg) | engine | **inherited trash → gain 1 memory** |
| EX10-003 Tumblemon (egg) | tech | inherited: opp attacks → trash 3 sources → end that attack |
| BT16-082 / P-123 Ukkomon | enabler | breeding→battle reveal-3 dig + re-hatch / +memory |
| EX10-034 Blastmon | payoff | Collision/Blocker/Fragment(3); forced-attack + ＜Sec.A.+1＞ |
| EX8-050 / BT4-072 Gogmamon | tech (Lv5 blocker) | [On Deletion] reveal-3 free-play a Mineral/Rock cost ≤5 |
| EX8-046 Gotsumon / BT14-009 Gotsumon | tech / lock | [On Deletion] draw / "players can't play Digimon by effects" |
| EX7-049 Metallicdramon, BT20-055 Invisimon, EX10-010 BlackWarGreymon | splash payoffs | de-digivolve / removal toolbox |

(NOT-IN-DSL / PARTIAL — **excluded from authored combos**: BT10-062 Golemon,
EX1-071 Win Rate: 60%!, LM-025 Cyberdramon, LM-033 Garnet Memory Boost!,
BT23-058 Craniamon (PARTIAL). P-130 Lui Ohwada (White Tamer) is IMPLEMENTED but
off the Black engine.)

## Digivolution lines

- **Tumblemon (Lv.2 egg, EX8-005/EX10-003) → Sunarizamon (Lv.3, EX8-047/EX10-025/
  BT21-055) → Landramon (Lv.4, P-167/EX8-048/EX10-028) → Proganomon (Lv.5,
  EX10-032/EX8-051) → Pyramidimon (Lv.6, EX10-033/EX8-055) → Magneticdramon
  (Lv.7, EX10-036)** — the mono-Black Mineral/Rock LIBERATOR spine. Every body in
  the line carries an inherited "when my source is trashed" payload, so evolving
  up the line *stacks more buried payloads* that the Options later trash.
- **Magneticdramon alt-req (EX10-036):** with a [Close] on field, digivolve from
  a Lv.6 black Digimon for cost **6** (`$BASE_DCGO/.../EX10/Black/EX10_036.cs`
  `AddSelfDigivolutionRequirementStaticEffect`, digivolutionCost 6). DSL
  `alt_paths: from {level_eq:6, color_is: black}, cost 6`.
- **Cheat line:** EX10-032 Proganomon `[Hand][Main]` (Close on field): place a
  [Landramon] from trash under a [Sunarizamon] → it digivolves into Proganomon
  for cost 3 ignoring requirements.
- **Option-fed cost reduction:** LM-031 (−3), Gravel Hearts Delay (−3),
  Defense Training Delay (−2) all drive `effect_initiated_digivolve` up this
  spine; with Close's "[SoT] set memory to 3" these chain in one turn.

## Named combos

### C1 — Zofr Kabus source-trash fan-out removal *(Option; rank: 1)*

- **Cards:** EX8-070 Zofr Kabus (Option, in hand) + 1 of your Mineral/Rock
  Digimon carrying an **inherited-delete source** beneath it (e.g. an EX8-047 /
  EX10-025 / BT21-055 Sunarizamon, or EX8-048 / EX10-028 Landramon, buried as a
  digivolution card) + an opponent Digimon with play cost ≤4 (the inherited
  target).
- **Expected mechanical outcome:** play Zofr Kabus `[Main]`. Its cost is "trash
  1 digivolution source of a chosen Mineral/Rock Digimon". Trashing that source
  **(a)** grants the chosen Digimon Collision + Piercing + Reboot +
  can't-be-returned + **+3000 DP until end of opponent's turn**, and **(b)**
  fires the trashed source's inherited "when trashed → delete 1 opp Digimon with
  play cost ≤4" trigger, **deleting a chosen opp Digimon (cost ≤4)**. Board diff:
  your chosen Digimon's source count −1; that Digimon gains Collision/Piercing/
  Reboot + ≥+3000 DP (expiry EoOT); **opp battle area −1 Digimon (cost ≤4)**, opp
  trash +the deleted body; Zofr Kabus → trash (Option resolved). Unhappy path: if
  the trashed source is **not** an inherited-delete card (e.g. a plain Tumblemon
  egg or a Proganomon → De-Dig instead), the cost-≤4 delete does **not** fire —
  the fan-out is source-specific.
- **Rules/keyword basis:** "By trashing … digivolution card" = cost paid before
  reward; source-trash dispatches the trashed source's inherited trigger
  (DCGO timing `OnDigivolutionCardDiscarded`). Keyword grants: Collision §16-29,
  Piercing §16-6, Reboot. DCGO/Impl:
  `code/digimon-engine/cards/ex8/EX8-070.yaml` (Main trash + buff clause);
  inherited delete `$BASE_DCGO/.../EX8/Black/EX8_047.cs` / `EX8_048.cs`.
- **Rank:** highest — the signature **Option-driven** removal swing: an Option
  that *itself* pays the source-trash cost and fans out an inherited delete while
  also buffing for the swing. Per-card tests can't see the cross-card fan-out.

### C2 — Gravel Hearts free-play → recur Sunarizamon search *(Option; rank: 2)*

- **Cards:** EX10-069 Unique Emblem: Gravel Hearts (Option, in hand) + a
  [Sunarizamon] (EX8-047 or EX10-025) available in **hand or trash**.
- **Expected mechanical outcome:** play Gravel Hearts `[Main]`. Choose "from
  hand" or "from trash", then **free-play 1 [Sunarizamon] (or [Close]) without
  paying its cost**; then Gravel Hearts is placed in the battle area as a Delay
  Option (`place_self_as_delay_option`). When the played card is EX8-047
  Sunarizamon, its `[On Play]` fires: **reveal top 3, add 1 Mineral/Rock + 1
  LIBERATOR card to hand, return the rest to the bottom**. Board diff: your
  battle area +1 Sunarizamon (cost paid: 0; memory unchanged by the play);
  hand +up to 2 cards (1 Mineral/Rock + 1 LIBERATOR) and −0 net from the free
  play if played from hand, **trash −1 Sunarizamon if played from trash** (trash
  recursion); deck top 3 → 1 added, 2 to bottom; Gravel Hearts → battle area
  (Delay armed). Unhappy path: with **no** Sunarizamon/Close in hand or trash the
  `[Main]` play step finds no legal target and only the self-placement happens.
- **Rules/keyword basis:** "play … without paying the cost" (free-play, no memory
  cost); Delay-Option placement §16-16. Impl:
  `code/digimon-engine/cards/ex10/EX10-069.yaml` (`play_from_hand_free` /
  `play_from_trash_free`); `$BASE_DCGO/.../EX10/Black/EX10_069.cs`
  (`CanSelectCardCondition` = Sunarizamon||Close, hand+trash selection). EX8-047
  On Play: `code/digimon-engine/cards/ex8/EX8-047.yaml`.
- **Rank:** high — Gravel Hearts is the deck's premier Option (freq 33, in every
  list); free-playing Sunarizamon from trash both recurs a body and chains its
  reveal-3 search. The hand-vs-trash branch is exactly a system-level edge.

### C3 — Black Scramble cost-reduced digivolve into the payoff *(Option; rank: 3)*

- **Cards:** LM-031 Black Scramble (Option, in hand) + a black Mineral/Rock
  Digimon already on field (e.g. a Lv.5 Proganomon) + a higher black Digimon
  card in hand to digivolve into (e.g. EX10-033 Pyramidimon, evo cost 3).
- **Expected mechanical outcome:** play Black Scramble `[Main]`. Choose the
  on-field black Digimon, then a black Digimon card in hand; it performs an
  **effect-initiated digivolve with the digivolution cost reduced by 3**
  (`effect_initiated_digivolve cost {reduce: 3}`), then Black Scramble is placed
  as a Delay Option. Board diff: the chosen permanent's top card becomes the
  hand card (its prior top card now a source beneath), hand −1 (the evo target),
  **memory paid = max(0, evoCost − 3)** (e.g. Pyramidimon evo cost 3 → 0 memory);
  Black Scramble → battle area (Delay armed). Because this is a digivolve, the
  new Digimon's `[When Digivolving]` triggers fire — so digivolving **into a
  Pyramidimon/Magneticdramon** chains the payoff's source-trash removal. Unhappy
  path: if the on-field Digimon or the hand target is **not black**, it is not a
  legal selection (filter `color_is: black`), so the cheat is off.
- **Rules/keyword basis:** effect-initiated digivolve + cost reduction; Delay
  §16-16. Impl: `code/digimon-engine/cards/lm/LM-031.yaml` (Clause A
  `effect_initiated_digivolve cost {reduce:3}`); `$BASE_DCGO/.../LM/.../LM_031.cs`
  (`DigivolveIntoHandOrTrashCard isHand reduceCost 3`).
- **Rank:** high — the Option-driven tempo line that *promotes into* the payoff
  WD triggers (C1/Magneticdramon). The black-filter gate and "digivolve fires WD"
  chain are system-level facts per-card tests miss.

### C4 — Defense Training Delay cost-reduced digivolve *(Option; rank: 4)*

- **Cards:** P-107 Defense Training (Option) **already placed in the battle area
  with its Delay armed** (placed a previous turn) + a black Digimon card in hand
  + any of your Digimon on field as the digivolve base.
- **Expected mechanical outcome:** on a later main phase, activate Defense
  Training's `<Delay>` by **trashing it**: choose 1 of your Digimon, then a
  **black** Digimon card in hand; it digivolves into that card for its
  digivolution cost **reduced by 2** (`effect_initiated_digivolve cost
  {reduce:2}`). Board diff: Defense Training → trash (Delay consumed); the chosen
  permanent's top card becomes the hand card with prior top beneath as a source;
  hand −1; memory paid = max(0, evoCost − 2); the new Digimon's `[When
  Digivolving]` fires. Unhappy path: the Delay **cannot** be activated the same
  turn it was placed ("after the placing turn", §16-16) — activating it the turn
  it enters is illegal.
- **Rules/keyword basis:** Delay §16-16 ("by trashing that card … after the
  placing turn"); effect-initiated digivolve −2. Impl:
  `code/digimon-engine/cards/p/P-107.yaml` (`kind: delay` clause,
  `effect_initiated_digivolve cost {reduce:2}`).
- **Rank:** medium-high — a second Option-fed cost-reduction line; the
  placing-turn Delay timing is a checkable system fact distinct from C3's
  in-hand [Main] form.

### C5 — Magneticdramon source-trash double removal *(payoff; rank: 5)*

- **Cards:** EX10-036 Magneticdramon (payoff) + ≥3 Mineral/Rock cards buried as
  digivolution sources across your Digimon, **with an inherited-delete
  Sunarizamon/Landramon among the 3 trashed** + an opp Digimon with play cost ≤4
  + the opp's top security.
- **Expected mechanical outcome:** on Magneticdramon's `[When Digivolving]`,
  trash exactly 3 Mineral/Rock sources → **delete 1 chosen opp Digimon** and
  **trash the opp's top security card** (active clause). *Then* the trashed
  inherited-delete source fires its "trash → delete opp cost ≤4" body, deleting a
  **second** opp Digimon. Board diff: opp battle area −2 Digimon, opp security
  −1, your stacks −3 sources → trash; a trashed Tumblemon among the 3 adds
  memory +1.
- **Rules/keyword basis:** "by trashing 3 … sources" = cost; source-trash
  dispatches each inherited trigger (`OnDigivolutionCardDiscarded`); Fragment(3)
  §16-36 (orthogonal survival). Impl:
  `code/digimon-engine/cards/ex10/EX10-036.yaml`;
  `$BASE_DCGO/.../EX10/Black/EX10_036.cs`; inherited `EX8_047.cs`.
- **Rank:** apex Digimon payoff, but **not Option-routed** — ranked below the
  four Option combos per the task focus. (Already the rank-1 combo in the
  sibling Rocks model; retained here for completeness.)

## Playstyle

- **Class:** midrange/combo-control on a Black memory engine. Close Tamers
  (EX8-067 "[SoT] set memory to 3", P-169/EX10-063 +memory) and the
  ramp/draw Options (P-039 Black Memory Boost!, P-206 Digital Gate Open,
  Defense Training reveal-2) hold the memory curve positive while the spine
  digivolves.
- **Tempo:** the deck wants buried Mineral/Rock sources, then converts them into
  removal through source-trash payoffs (Zofr Kabus, Magneticdramon, Pyramidimon).
  Options accelerate both the burying (Gravel Hearts free-play, Close refuel)
  and the cashing-in (Zofr Kabus, cost-reduced digivolve into the WD payoffs).
- **Memory curve:** Close resets to 3 each turn; Option Delays bank a turn of
  tempo (place this turn, fire next).

## Win conditions

- **Attrition removal:** repeated source-trash deletes (cost ≤4 fan-outs,
  De-Digivolve 1, Magneticdramon's delete + security trash, Zofr Kabus buff +
  delete) grind the opponent's board to nothing, then push damage with
  Collision/Piercing/Security-Attack-boosted attackers.
- **Magneticdramon close:** the Lv.7 with Collision/Piercing (granted by Zofr
  Kabus/Proganomon) plus its delete + security-trash on attack pressures the
  security stack while surviving via Fragment(3).

## Ranked interactions to test

1. **C1 — Zofr Kabus source-trash fan-out removal** (Option pays the inherited
   source-trash cost → delete opp ≤4 *and* buff): the highest-value Option combo;
   exercises the Option-as-cost → inherited-fan-out cross-card chain.
2. **C2 — Gravel Hearts free-play → Sunarizamon recur/search**: Option free-play
   from hand-or-trash → On-Play reveal-3 chain; hand-vs-trash branch.
3. **C3 — Black Scramble cost-reduced digivolve into payoff**: Option −3 digivolve
   that promotes into the WD removal triggers; black-filter gate.
4. **C4 — Defense Training Delay cost-reduced digivolve**: Option Delay −2
   digivolve; placing-turn Delay-timing gate (§16-16).
5. **C5 — Magneticdramon source-trash double removal**: apex payoff double-delete
   + security trash; retained but ranked below the Option combos per focus.

### Dropped under the cap (logged, not silently truncated)

- **Proganomon Close-gated cheat-evolve (EX10-032)** — Digimon-only cheat line,
  no Option; covered by the Rocks model (rank-2 there). Dropped: not Option-routed.
- **Pyramidimon trash-3 + re-bury recursion (EX10-033/EX8-055)** — Digimon-only
  payoff recursion; covered by the Rocks model. Dropped: not Option-routed.
- **Close suspend-refuel on Mineral/Rock digivolve (EX8-067)** — Tamer engine,
  no Option. Dropped: not Option-routed and covered by Rocks model C4.
- **Black Scramble [Security] free-play black from trash (LM-031 sec)** — strong
  but security-trigger, single-card; folded into C3's Option entry.
- **P-206 Digital Gate Open Delay play-Tamer −4 / P-039 Black Memory Boost! +2
  memory** — ramp Options, no removal/digivolve payoff chain; lower
  payoff-centrality than C1–C4.
- **EX10-003 Tumblemon end-attack (inherited trash-3 → end attack)** — defensive
  inherited, no Option. Dropped: not Option-routed, lower frequency.
