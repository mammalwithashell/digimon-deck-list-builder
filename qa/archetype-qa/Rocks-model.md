# Rocks — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phases 0–3). Durable, reviewable system model of the **Rocks** (Mineral/Rock
> [LIBERATOR]) archetype. Sources cited inline: DCGO C# path
> (`$BASE_DCGO/Assets/Scripts/CardEffect/...`) and/or `general_rule.pdf` §16
> rule numbers (keyword semantics) — DCGO + the PDF outrank the card-text JSON
> per CLAUDE.md source priority. Pool resolved with
> `python code/tools/resolve_deck.py "Rocks" --json` (136 decklists, 56 unique
> cards). Per-card DSL verdicts read from `qa/qa-reports/validated_cards_dsl.json`
> — the entire competitive core is **IMPLEMENTED** in the Rust DSL.

## The central engine (read this first)

Rocks is a **source-trash value engine**. Its Mineral/Rock Digimon carry
*inherited* effects that fire on the timing **"when effects trash this card from
a [Mineral] or [Rock] trait Digimon's digivolution cards"** (DCGO timing
`OnDigivolutionCardDiscarded`, gated on the *trashed-from* permanent's top card
having the Mineral/Rock trait). The deck's mid/high Digimon and Options exist to
**trash those buried sources** as a cost — every source trashed simultaneously
"pays the cost" and **fans out one inherited trigger per trashed source**. So a
single payoff activation can chain into multiple deletes / de-digivolves / draws
/ memory gains. The Close Tamers and Magneticdramon/Pyramidimon re-bury sources
from trash, making the engine recur.

**Inherited "when my source is trashed" payloads** (the fan-out targets):
| Source card | Trait | Inherited payload on trash |
|-------------|-------|----------------------------|
| EX8-047 / EX10-025 / BT21-055 Sunarizamon | **Reptile**/LIBERATOR | delete 1 opp Digimon with play cost ≤ 4 (`$BASE_DCGO/.../EX8/Black/EX8_047.cs`) — **NOTE: Reptile trait only; cannot be trashed as cost by EX10-036 / EX11-044 (those require Mineral/Rock sources)** |
| EX8-048 / EX10-028 / P-167 Landramon | **Mineral**/LIBERATOR | delete 1 opp Digimon with play cost ≤ 4 (`$BASE_DCGO/.../EX8/Black/EX8_048.cs`) — qualifies as a Mineral/Rock trash cost for EX10-036 / EX11-044 |
| EX8-051 / EX10-032 Proganomon, P-167 Landramon | varies | ＜De-Digivolve 1＞ 1 opp Digimon (`$BASE_DCGO/.../EX8/Black/EX8_051.cs`) |
| EX11-038 Sunarizamon | Reptile/LIBERATOR | ＜Draw 1＞ (`$BASE_DCGO/.../EX11/Black/EX11_038.cs`) |
| EX8-005 / EX10-003 Tumblemon (DigiEgg) | — | gain 1 memory (`$BASE_DCGO/.../EX8/Black/EX8_005.cs`) |

**Source-trashing payoffs** (the activators / engines):
| Card | What it trashes / does |
|------|------------------------|
| EX10-036 Magneticdramon | WD/WA: trash **3** Mineral/Rock sources → delete 1 opp Digimon + trash their top security (`$BASE_DCGO/.../EX10/Black/EX10_036.cs`) |
| EX10-033 / EX8-055 / EX11-044 Pyramidimon | WD/WA: trash up to 3 sources → reduce opp cost / delete highest-cost; re-bury 3 from trash |
| EX10-032 / EX8-051 Proganomon | WD/WA: trash 1 source → grant Collision/Piercing + DP |
| EX8-070 Zofr Kabus (Option) | trash 1 source → grant a keyword bundle |
| EX10-028 Landramon | WD: trash 1 source → Reboot+Blocker+DP buff |

**Re-bury / refuel engines** (recursion): EX8-067 / EX10-063 / EX11-065 / P-169
Close (Tamers, suspend to place sources from trash + memory), Magneticdramon and
Pyramidimon's own "place from trash as bottom sources" clauses.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| EX8-047 Sunarizamon | enabler+inherited | [On Play] reveal-3 add Mineral/Rock + LIBERATOR; **inherited trash → delete opp cost ≤4** |
| EX10-025 Sunarizamon | enabler | [On Play] place 2 Mineral/Rock from trash as a Digimon's bottom sources (refuel) |
| EX11-038 Sunarizamon | engine+inherited | [On Play]/[When Moving] trash 1 source → Draw 1; **inherited trash → Draw 1** |
| BT21-055 Sunarizamon | enabler | when digivolving into Mineral/Rock, reduce digivolve cost by 1; **inherited trash → delete opp cost ≤4** |
| EX8-048 Landramon | enabler+inherited | [WD] play [Close] free if ≤1 Tamer; **inherited trash → delete opp cost ≤4** |
| EX10-028 Landramon | engine+inherited | [On Play]/[WD] trash 1 source → Reboot/Blocker/+3000; **inherited trash → delete opp cost ≤4** |
| P-167 Landramon | engine+inherited | [SoMP]/[WD] trash 1 source → reveal-3 dig; **inherited trash → De-Digivolve 1** |
| EX10-032 Proganomon | engine+inherited | [Hand][Main] Close-gated cheat-evolve; WD/WA trash 1 source → Collision/Piercing/+3000; **inherited trash → De-Dig 1** |
| EX8-051 Proganomon | payoff+inherited | static Collision/Piercing/Fragment(3); **inherited trash → De-Dig 1** |
| EX10-033 Pyramidimon | payoff | Fragment(3); WD/WA re-bury 3 + trash up to 3 sources → reduce opp cost by 2/each |
| EX11-044 Pyramidimon | payoff | Reboot/Fragment(3); WD/WA/OP trash 3 sources → delete opp highest-cost Digimon/Tamer; re-bury 3 on own trash |
| EX8-055 Pyramidimon | payoff | Fragment(3); WD/WA trash 3 sources → unsuspend + ＜Sec.A.+1＞; EoT re-bury 3 |
| EX10-036 Magneticdramon | payoff (Lv7) | Fragment(3); WD/WA trash 3 sources → **delete 1 opp Digimon + trash their top security**; re-bury 3 → unsuspend |
| EX10-034 Blastmon | payoff | Collision/Blocker/Fragment(3); WD give opp forced-attack; [All Turns][OPT] trash 2 sources → ＜Sec.A.+1＞/+3000 |
| EX8-050 Gogmamon | tech (Lv5 blocker) | [On Deletion] reveal-3 free-play a Mineral/Rock cost ≤5; inherited redirect attack |
| EX8-046 Gotsumon | tech | [On Deletion] trash 1 Mineral/Rock from hand → Draw 2; inherited Blocker |
| EX8-005 Tumblemon (egg) | engine | **inherited trash → gain 1 memory** |
| EX10-003 Tumblemon (egg) | tech | inherited: opp attacks → trash 3 Mineral/Rock sources → end that attack |
| EX8-067 Close | engine (Tamer) | [SoT] set memory to 3; on Mineral/Rock digivolve, suspend → place ≤2 sources from trash |
| EX10-063 Close | engine (Tamer) | [SoMP] cheat-play Close + Sunarizamon from trash; on source-trash, suspend → +1 memory |
| EX11-065 Close | engine (Tamer) | [SoMP] trash 1 source → +1 memory; on play/digivolve, suspend → place a source from trash |
| P-169 Close | engine (Tamer) | [SoMP] +1 memory if opp has Digimon; on source-trash, suspend → place a source from trash |
| EX10-069 Gravel Hearts | enabler (Option) | [Main] cheat-play Sunarizamon/Close; Delay: when Close suspends, cost-reduced digivolve into a Mineral+LIBERATOR |
| EX7-074 Vortex Resonance | enabler (Option) | reveal-3 add LIBERATOR + cost-reduced (−4) digivolve |
| BT16-082 Ukkomon | enabler (Lv3) | breeding-area-to-battle reveal-3 dig + re-hatch |
| LM-031 Black Scramble | enabler (Option) | cost-reduced (−3) black digivolve; Delay recursion |
| P-107 Defense Training | enabler (Option) | reveal-2 add black; Delay cost-reduced (−2) black digivolve |
| BT9-103 Kongou | tech (Option) | opp cost ≤7 can't attack players; opp can't add security |
| EX8-070 Zofr Kabus | tech (Option) | trash 1 source → grant Collision/Piercing/Reboot/anti-bounce/+3000 |

(Lower-frequency splash/tech: BT4-072 Gogmamon, BT2-105/BT5-105/ST15-16/BT23-096
de-digivolve removal Options, P-215 Icemon anti-bounce, P-186 Gallantmon,
EX7-049 Metallicdramon, plug-ins. Enumerated in the resolve output; not central
to the named combos below.)

## Digivolution lines

- **Tumblemon (Lv.2 egg, EX8-005/EX10-003) → Sunarizamon (Lv.3) → Landramon
  (Lv.4) → Proganomon (Lv.5) → Pyramidimon (Lv.6) → Magneticdramon (Lv.7)** —
  the mono-Black Mineral/Rock LIBERATOR spine. Every body in this line carries
  an inherited "when my source is trashed" payload, so the line *is* the engine:
  evolving up stacks more buried payloads.
- **Cheat lines:** EX10-032 Proganomon `[Hand][Main]` (with Close on field):
  place a [Landramon] from trash under a [Sunarizamon] → it digivolves into
  Proganomon for cost 3 ignoring requirements. EX10-036 Magneticdramon alt-req:
  with Close on field, digivolve from a [Proganomon] for cost 6.
- **Cost gates:** Black evo costs; Close (EX8-067) floors memory to 3 each turn,
  and the various Close Tamers + Gravel Hearts cheap-play the Lv.3 enablers.

## Named combos

### C1 — Magneticdramon source-trash double removal *(rank: 1)*

- **Cards:** EX10-036 Magneticdramon (payoff) + ≥3 **Mineral/Rock**-trait cards
  buried as digivolution sources across your Digimon (e.g. under Magneticdramon
  itself), with **EX8-048 Landramon (Mineral/LIBERATOR)** among the trashed
  sources; an opponent Digimon with play cost ≤4 (inherited target) and the
  opp's top security. Note: **Sunarizamon (Reptile/LIBERATOR) is NOT a
  Mineral/Rock-trait card and cannot serve as one of the trashed sources for
  EX10-036's "by trashing 3 Mineral/Rock sources" cost** — only Landramon
  (Mineral) or other Mineral/Rock-trait sources qualify.
- **Expected mechanical outcome:** on Magneticdramon's `[When Digivolving]`,
  trashing exactly 3 Mineral/Rock sources → **delete 1 chosen opp Digimon** and
  **trash the opp's top security card** (active clause). *Then*, because the
  trashed sources include an EX8-048 Landramon (inherited "trash → delete opp
  cost ≤4"), that inherited trigger **also fires**, deleting a *second* opp
  Digimon (cost ≤4). Board diff: opp battle area −2 Digimon (active delete +
  Landramon inherited delete), opp security −1 (active); your trash −3 sources;
  Tumblemon-among-the-3 would additionally swing memory +1.
- **Rules/keyword basis:** "by trashing 3 ... sources" = cost paid before reward;
  source-trash dispatches each trashed source's inherited trigger
  (`OnDigivolutionCardDiscarded`); EX10-036's cost filter requires Mineral/Rock
  trait on the trashed source. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_036.cs` (active
  delete+security + Mineral/Rock cost filter), `.../EX8/Black/EX8_048.cs`
  (Landramon inherited delete). Fragment(3) §16-36 (Magneticdramon's own
  survival is orthogonal).
- **Rank:** highest — Magneticdramon is the deck's apex (freq 135) and this is
  the signature multi-card removal swing per-card tests can't see. The
  active-delete + Landramon-inherited-delete = double removal (not triple);
  Sunarizamon provides no Mineral/Rock trash-cost contribution here.

### C2 — Proganomon cheat-evolve, Close-gated *(rank: 2)*

- **Cards:** EX10-032 Proganomon (in hand) + a [Close] Tamer on field (any of
  EX8-067/EX10-063/EX11-065/P-169) + a [Sunarizamon] on field + a [Landramon]
  in trash.
- **Expected mechanical outcome:** activate Proganomon's `[Hand][Main]`: place
  the [Landramon] from trash as the [Sunarizamon]'s bottom source, then that
  Sunarizamon **digivolves into Proganomon for digivolution cost 3, ignoring
  digivolution requirements**. Board diff: trash −1 Landramon; the Sunarizamon
  permanent's top card becomes Proganomon with the Sunarizamon (+ Landramon)
  beneath as sources; memory −3. **Without** a Close on field the `[Hand][Main]`
  is not usable (gated) — the cheat is off.
- **Rules/keyword basis:** cost-paid-by-placing + ignore-digivolution-requirement
  semantics. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_032.cs` (`CanUseCondition`
  requires Close + Sunarizamon on field + Landramon in trash).
- **Rank:** high — the defining tempo line (freq 136), deeply multi-card, and the
  Close-gate is exactly a system-level precondition.

### C3 — Pyramidimon trash-3 highest-cost delete + re-bury recursion *(rank: 3)*

- **Cards:** EX11-044 Pyramidimon (payoff) + ≥3 **Mineral/Rock**-trait sources
  buried (with at least one inherited-delete **Landramon (EX8-048/EX10-028/P-167,
  Mineral/LIBERATOR)** among them — **not Sunarizamon, which is Reptile/LIBERATOR
  and cannot be trashed as cost by EX11-044's Mineral/Rock filter**) + ≥3
  Mineral/Rock cards in trash + an opp board where the highest-cost Digimon is
  the intended target.
- **Expected mechanical outcome:** on `[On Play]/[When Digivolving]/[When
  Attacking][OPT]`, trash 3 Mineral/Rock sources → **delete 1 of opp's highest
  play-cost Digimon or Tamers**; the 3 trashed sources fan out their inherited
  triggers (e.g. extra delete cost ≤4 / De-Dig 1 / Draw 1 / +memory). Pyramidimon
  also has `[All Turns][OPT]` "when my sources are trashed → place 3 Mineral/Rock
  from trash as my bottom sources", **refueling itself** to enable the next
  activation. Board diff: opp field −1 (highest cost) + fan-out effects; your
  stacks −3 sources but +3 re-buried from trash (net source count restored);
  Fragment(3) keeps Pyramidimon alive through removal.
- **Rules/keyword basis:** Once-Per-Turn (§ keyword OPT) on the trash clause;
  De-Digivolve §16-11 (can't trash past Lv.3) for the inherited De-Dig fan-out;
  Fragment(3) §16-36. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX11/Black/EX11_044.cs`.
- **Rank:** high — Pyramidimon (freq 32+114+54 across prints) is the recurring
  Lv.6 grind engine; the re-bury-after-trash loop is the archetype's signature
  recursion.

### C4 — Close suspend-refuel on Mineral/Rock digivolve *(rank: 4)*

- **Cards:** EX8-067 Close (Tamer, unsuspended) + a Digimon digivolving into a
  Mineral/Rock Digimon + ≥1 Mineral/Rock card in trash.
- **Expected mechanical outcome:** when your Digimon digivolves into a
  Mineral/Rock Digimon on your turn, you may **suspend Close** to place up to 2
  Mineral/Rock cards from trash as that Digimon's bottom sources. Board diff:
  Close → suspended; trash −(1 or 2) Mineral/Rock; the digivolved Digimon's
  source count +(1 or 2). This **pre-loads** the fuel that C1/C3 then trash —
  the enabler half of the engine. Unhappy path: if Close is already suspended,
  the cost can't be paid and no sources move.
- **Rules/keyword basis:** "by suspending this Tamer" = suspend-as-cost; placing
  from trash as bottom digivolution sources. DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX8/Black/EX8_067.cs`.
- **Rank:** medium-high — Close is in every list (freq 136) and is the refuel
  half of the loop; the suspend-cost gating is a checkable system fact.

### C5 — Gravel Hearts cheat-play + Delay cost-reduced digivolve *(rank: 5)*

- **Cards:** EX10-069 Unique Emblem: Gravel Hearts (Option) + a [Sunarizamon] or
  [Close] in hand/trash; later, a Close suspending to arm the Delay + a
  Mineral/Rock Digimon to digivolve into a Mineral+LIBERATOR card in hand.
- **Expected mechanical outcome:** `[Main]`: play 1 [Sunarizamon] or [Close] from
  hand/trash **without paying cost**, then place Gravel Hearts in the battle area
  (Option permanent). The `[Your Turn]` Delay: when any of your [Close]s suspend
  (after the placing turn), by trashing Gravel Hearts, **1 Mineral/Rock Digimon
  digivolves into a Mineral+LIBERATOR card in hand with digivolution cost reduced
  by 3**. Board diff (Main): hand/trash −1 Sunarizamon/Close, that body enters
  play free, Gravel Hearts → battle area; (Delay): Gravel Hearts → trash, a
  Digimon digivolves up at −3 cost.
- **Rules/keyword basis:** ＜Delay＞ §16-16 (can't activate the turn it's placed;
  optional, by trashing the card). DCGO C#:
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX10/Black/EX10_069.cs`.
- **Rank:** medium — the primary ramp/enabler Option (freq 135); cross-turn Delay
  timing is a system fact, but its payoff is enabler-tempo rather than a board
  swing, so ranked below the removal/recursion combos.

## Playstyle

- **Class:** midrange grind/control with a removal-combo core. It is not a
  raw-aggro deck; it converts buried Mineral/Rock sources into repeated interaction
  (delete / de-digivolve) while staying alive through Fragment(3).
- **Tempo & memory:** Close (EX8-067) floors memory to 3 every turn and the Close
  Tamers + Tumblemon source-trash both generate memory, smoothing the curve. The
  deck builds a Mineral/Rock stack, then each turn trashes 3 sources (Magneticdramon
  / Pyramidimon) for a removal swing and re-buries from trash to do it again.

## Win conditions

1. **Grind-and-suppress:** repeatedly delete/de-digivolve the opponent's board via
   source-trash (Magneticdramon, Pyramidimon, the inherited cost-≤4 deletes), keep
   them off attackers, then push damage with Sec.A.+1 / Piercing payoffs through an
   empty board.
2. **Security pressure:** Magneticdramon trashes the opp's top security on each
   activation, and Pyramidimon/Blastmon/Proganomon grant ＜Security A. +1＞ and
   ＜Piercing＞ for extra checks.

## Ranked interactions to test

1. **C1 Magneticdramon source-trash double removal** — apex payoff; active
   delete+security *plus* the inherited fan-out delete from the trashed source.
   (Highest value: no per-card test asserts the active + inherited chain together.)
2. **C2 Proganomon cheat-evolve, Close-gated** — defining tempo line; the
   Close-on-field precondition flips the line on/off (test both branches).
3. **C3 Pyramidimon trash-3 highest-cost delete + re-bury recursion** — the
   grind engine and the re-bury-after-own-trash loop (OPT lockout matters).
4. **C4 Close suspend-refuel on Mineral/Rock digivolve** — the refuel half of the
   loop; suspend-cost gating (already-suspended ⇒ no fuel).
5. **C5 Gravel Hearts cheat-play + Delay cost-reduced digivolve** — cross-turn
   Delay timing (§16-16: not the placing turn) and free-play ramp.

**Dropped under the top-5 cap (logged, not silently truncated):**
- **Tumblemon memory-on-trash fan-out** (EX8-005 + any source-trash payoff) —
  real, but a single-source memory tick is largely subsumed by C1/C3's fan-out
  assertions; lower payoff-centrality.
- **EX10-003 Tumblemon "trash 3 sources to end an attack"** — defensive inherited;
  niche and opponent-turn-only, lower play frequency.
- **Gogmamon EX8-050 On-Deletion free-play** + **EX8-046 Gotsumon On-Deletion
  Draw 2** — value-on-death tech, not the core loop.
- **BT9-103 Kongou attack-lock / de-digivolve removal Options
  (BT2-105/BT5-105/ST15-16/BT23-096)** — single-card tech, no cross-card
  interaction to model.
- **EX10-032 Proganomon WD/WA "trash 1 source → grant Collision/Piercing"** —
  a payoff, but its source-trash fan-out is the same mechanism C1/C3 already
  exercise at higher centrality.
