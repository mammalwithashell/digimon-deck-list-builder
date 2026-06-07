# Nokia AlterS — Model

> Archetype-model artifact produced by `/archetype-interaction-test-author`
> (Phases 0–3). Durable, reviewable system model of the **Nokia AlterS**
> (Agumon/Gabumon → WarGreymon/MetalGarurumon → Omnimon / Omnimon Alter-S)
> archetype. Sources cited inline: DCGO C# path
> (`$BASE_DCGO/Assets/Scripts/CardEffect/...`,
> `$BASE_DCGO = $(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO`)
> and/or `general_rule.pdf` §16 keyword/timing rules — DCGO + the PDF outrank
> the card-text JSON per CLAUDE.md source priority. Pool resolved with
> `python code/tools/resolve_deck.py "Nokia AlterS" --json` (2 decklists, 28
> unique cards). Per-card DSL verdicts read from
> `qa/qa-reports/validated_cards_dsl.json` — **all 28 pool cards are
> IMPLEMENTED** in the Rust DSL. (Pinecone retrieval was unavailable this run —
> no API key — so research is grounded in the local YAML specs + DCGO C# +
> printed text, the higher-trust sources.)

## The central engine (read this first)

Nokia AlterS is a **DNA-digivolve combo/tempo deck** that assembles
WarGreymon (Red) + MetalGarurumon (Blue) and fuses them into an **Omnimon**
finisher (BT17-078 / BT22-015 / EX9-021 Omnimon Alter-S / BT20-102 Omnimon
X-Antibody), generating board wipes and free security-stack damage. Three
sub-engines feed it:

1. **Tamer ramp + free-play.** Nokia Shiramine (BT22-084 / BT5-092) and the
   two Tai & Matt Ishida tamers (BT17-081 / EX9-066) cheat low Digimon into
   play, set/gain memory, and buff the Greymon/Garurumon/Omnimon name family.
   BT22-084 floors memory to 3 each turn and free-plays an Agumon/Gabumon if you
   have ≤1 Digimon; Tai&Matt gain +1 memory **per** Greymon **and per** Garurumon
   on every own Digimon play/digivolve (by suspending), which is what turns the
   double-evolve sequence into a same-turn Omnimon.

2. **Cross-evolve cheat lines.** The Lv6 WarGreymon/MetalGarurumon
   (BT17-015/BT17-027, BT22-013/BT22-026, BT15-101) each, [When Digivolving],
   can free-digivolve the *opposite* color's Lv5 line into its Lv6 form
   ("1 of your [Gabumon] may digivolve into [MetalGarurumon]…"). Combined with
   Nokia's cost-6 cheat-evolve from hand and the Tai-Kamiya alt-source paths,
   the deck reaches **both** Lv6 halves in one turn → DNA into Omnimon.

3. **Option enablers (the FOCUS).** Three Option cards extend the engine:
   - **BT17-095 Miraculous Mega Knight** — [Main] free-plays an Agumon/Gabumon
     from hand **or trash**, then seats itself as a Delay; its Delay turns a
     dying Lv6 Greymon/Garurumon into a hand-DNA Omnimon (a "second life"
     finisher trigger). Its Security clause free-plays a Tai/Matt tamer.
   - **P-206 Digital Gate Open** — [Main] reveal-3 digs a Digimon + a Tamer, then
     its Delay cheats a colour-matched Tamer in cost-4-reduced; its Security
     clause free-plays a cost ≤3 Digimon.
   - **ST20-15 Island of Adventure** — [Main] launders the top security into hand
     and reseats itself face-up as a security card granting **+2000 DP to all own
     Lv3+ Digimon**; its Security clause free-plays a Tamer.

   Every Option follows the **"Delay-Option / play-from-security" pattern** that
   `general_rule.pdf` §16 ＜Delay＞ and the Option-as-permanent rules describe:
   the [Main] play seats the card in the battle area as a Delay permanent
   (`place_self_as_delay_option`), and its trigger fires "after the placing turn".

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| BT22-084 Nokia Shiramine | engine (Tamer) | [SoT] memory→3 if ≤2; [SoMP]/[OP] free-play Agumon/Gabumon if ≤1 Digimon; [All Turns] +1000 DP to Greymon/Garurumon/Omnimon names |
| BT5-092 Nokia Shiramine | enabler (Tamer) | [On Play] free-play Agumon/Gabumon; on Greymon/Garurumon/Omnimon digivolve, suspend → digivolve cost −1 |
| BT17-081 Tai & Matt Ishida | engine (Tamer) | [All Turns] on own Digimon play/digivolve, suspend → +1 memory per Greymon **and** per Garurumon; [EoT][OPT] 1 own Omnimon may attack a player |
| EX9-066 Tai & Matt Ishida | engine (Tamer) | [On Play] return Greymon/Garurumon/Omnimon from trash (else Draw 1); [All Turns] same suspend→memory engine as BT17-081 |
| BT22-089 Mirei Mikagura | tech (Tamer) | [SoMP] return self to deck bottom → cheat a cost-4+ CS/Mirei tamer; [On Play] trash a CS/Angel card → Draw 2 |
| BT22-008 Agumon | enabler | [On Play] return Greymon/Garurumon/Omnimon from trash; inherited [EoT] DNA-digivolve into a hand card |
| EX4-038 Agumon | enabler | [On Play] reveal-3 add Greymon + Gabumon/Garurumon/Omnimon; inherited [OPT] +1 memory on own digivolve |
| EX4-039 Gabumon | enabler | [On Play] reveal-3 add Garurumon + Agumon/Greymon/Omnimon; inherited [OPT] +1 memory on own digivolve |
| BT12-059 Agumon | enabler | [On Play] reveal-4 add Greymon/Omnimon + Tai-Kamiya tamer |
| BT17-007 Agumon | enabler | [SoMP] if Tai-Kamiya tamer, return Garurumon/Greymon/Omnimon from trash; inherited [EoT] DNA-digivolve |
| BT17-019 Gabumon | enabler | [SoMP] if Matt-Ishida tamer, Draw 1; inherited [EoT] DNA-digivolve |
| BT22-017 Gabumon | enabler | [On Play] reveal-3 add Omnimon-text + CS-trait card; inherited [EoT] DNA-digivolve |
| ST20-10 Agumon | tech | [Your Turn] cheat-digivolve into WarGreymon (cost 4) vs big board / 3-colour tamers; inherited ＜Reboot＞ |
| BT14-001 Koromon | egg | inherited [Your Turn][OPT] on opp security removed → Draw 1 |
| BT17-102 Greymon | payoff (Lv4) | [WD] if Koromon-named +3000 then delete opp Digimon ≤ its DP; gains all sub-Lv3 names; [On Deletion] free-play Tai/Kari tamer or hatch |
| BT17-015 WarGreymon | payoff (Lv6) | cost −3 with Tai tamer; [OP]/[WD] delete opp ≤8000 **OR** cheat Gabumon→MetalGarurumon; inherited [WA][OPT] Omnimon-name → trash opp top security |
| BT22-013 WarGreymon | payoff (Lv6) | [Hand][Main] Nokia cheat-evolve Agumon (cost 6); [WD] cheat Gabumon→MetalGarurumon **OR** delete opp lowest-DP; inherited [WA][OPT] Omnimon → trash opp top security |
| BT17-027 MetalGarurumon | payoff (Lv6) | cost −3 with Matt tamer; [OP]/[WD] opp can't-suspend **OR** cheat Agumon→WarGreymon; inherited [WA][OPT] Omnimon → unsuspend self |
| BT22-026 MetalGarurumon | payoff (Lv6) | [Hand][Main] Nokia cheat-evolve Gabumon (cost 6); [WD] cheat Agumon→WarGreymon **OR** return opp lowest-level; inherited [WA][OPT] Omnimon → unsuspend |
| BT15-101 MetalGarurumon | payoff (Lv6) | Matt-gated cheat-evolve from Gabumon (cost 4); ＜Evade＞; [WD] 3 opp can't-suspend; [All Turns][OPT] on becoming suspended → unsuspend |
| BT17-078 Omnimon | finisher (Lv7) | ＜Blast DNA Digivolve＞; ＜Raid＞＜Blocker＞; [OP]/[WD] if DNA: bounce all opp same-level + delete 1 |
| BT22-015 Omnimon | finisher (Lv7) | ＜Blocker＞＜Decode＞; [OP]/[WA] delete opp lowest-DP; [WD] per-2-same-level bounce + may attack |
| EX9-021 Omnimon Alter-S | finisher (Lv7) | DNA from Blue Lv6 + Red Lv6; [WD] if DNA: immune to opp effects + delete all opp highest-level; [EoA] play Greymon/Ver.1 + Garurumon/Ver.2 from sources, then place self as top security |
| BT20-102 Omnimon X | finisher (Lv7) | ＜Raid＞＜Piercing＞＜Blocker＞; [OP]/[WD] choose 1 of both players' Digimon, delete all others, bounce 1 opp; [EoT][OPT] grant Rush |
| EX4-073 Omnimon Alter-B | finisher (Lv7) | [WD] ＜De-Digivolve 3＞ + delete ≤6 cost; [WA] trash sources → delete lowest-cost / trash 2 opp security |
| **BT17-095 Miraculous Mega Knight** | **Option (enabler/finisher)** | [Main] free-play Agumon/Gabumon from hand/trash, seat self as Delay; Delay: dying own Lv6 Greymon/Garurumon → DNA into hand Omnimon; [Security] free-play Tai/Matt tamer + add self to hand |
| **P-206 Digital Gate Open** | **Option (dig/ramp)** | [Main] reveal-3 add Digimon + Tamer, seat self as Delay; Delay: cheat a colour-matched Tamer cost −4; [Security] free-play Digimon ≤3 + add self to hand; ignore-color |
| **ST20-15 Island of Adventure** | **Option (security/buff)** | [Main] top-security → hand, reseat self face-up as security; [Security][All Turns] own Lv3+ Digimon +2000 DP; [Security] free-play 1 Tamer; ignore-color while no face-up copy |

## Digivolution lines

- **Red line:** Koromon (BT14-001) → Agumon (BT12-059 / BT17-007 / BT22-008 /
  EX4-038 / ST20-10, Lv3) → Greymon (BT17-102, Lv4, White) → WarGreymon
  (BT17-015 / BT22-013, Lv6/Red, cost 3–4 over Lv5).
- **Blue line:** Gabumon (BT17-019 / BT22-017 / EX4-039, Lv3) → … → MetalGarurumon
  (BT17-027 / BT22-026 / BT15-101, Lv6/Blue).
- **Cross-evolve shortcuts:** each Lv6's [When Digivolving] can free-evolve the
  *opposite* line's Lv3 base directly into its Lv6 form, **ignoring requirements
  and cost** (BT17-015 → Gabumon→MetalGarurumon; BT17-027 → Agumon→WarGreymon;
  BT22-013/026 mirror these). Nokia's [Hand][Main] cheat-evolve (BT22-013/026)
  pushes an Agumon/Gabumon to Lv6 for a flat cost-6.
- **Finisher fusions:**
  - **Omnimon** (BT17-078 / BT22-015): ＜Blast DNA Digivolve＞ WarGreymon +
    MetalGarurumon from hand/field; or normal Lv6 → Lv7.
  - **Omnimon Alter-S** (EX9-021): DNA digivolve **Blue Lv6 + Red Lv6**, cost 0
    (`code/digimon-engine/cards/ex9/EX9-021.yaml` `alt_paths`; DCGO
    `$BASE_DCGO/Assets/Scripts/CardEffect/EX9/.../EX9_021.cs` GetJogress).
  - **Omnimon X** (BT20-102), **Omnimon Alter-B** (EX4-073) cap the late game.

## Named combos

### C1 — Miraculous Mega Knight free-play recursion (Option)
- Cards: **BT17-095**, an Agumon/Gabumon (e.g. **BT22-008** / **BT22-017**)
  already in the trash.
- Expected mechanical outcome: play BT17-095 ([Main], cost 2). Before: target
  Agumon/Gabumon is in **trash**, BT17-095 in hand. After: that Agumon/Gabumon
  is a new **battle-area permanent** (played free; its own [On Play] resolves —
  e.g. BT22-008 returns a Greymon/Garurumon/Omnimon from trash to hand), trash
  count for that card −1, and **BT17-095 is now a battle-area Delay-Option
  permanent** (not in trash/hand). Net board: +1 own Digimon, +1 own Option
  permanent, no cost paid for the Digimon.
- Rules/keyword basis: `general_rule.pdf` §16 ＜Delay＞ + Option-permanent
  placement; DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/BT17/Red/BT17_095.cs`
  (OptionSkill: union-zone play hand/trash free, then `PlaceDelayOptionCards`).
  YAML `code/digimon-engine/cards/bt17/BT17-095.yaml` Clause A
  (`select_union_zone` + `play_union_bound_free` + `place_self_as_delay_option`).
- Rank: 1 (Option, runs 4 copies, central to the deck's grind plan).

### C2 — Mega Knight Delay "second-life" hand-DNA Omnimon (Option)
- Cards: **BT17-095** (already seated as a Delay permanent from a prior turn),
  a battle-area Lv6 **WarGreymon (BT17-015/BT22-013)** or **MetalGarurumon
  (BT17-027/BT22-026)**, an **Omnimon** (BT17-078) + a Lv6 partner **in hand**.
- Expected mechanical outcome: when the seated Lv6 Greymon/Garurumon would
  **leave the battle area outside of battle**, BT17-095's Delay fires: trash
  BT17-095 from the field (Delay cost), then the leaving Lv6 + the hand Lv6
  partner **DNA-digivolve into the hand Omnimon**. After: BT17-095 is in trash,
  the leaving Lv6 is consumed into a new Omnimon permanent whose stack is
  `[leaving Lv6 sources …, hand partner, Omnimon]`, and the leave is **cancelled**
  (the Lv6 does not reach the trash). If the player declines or no eligible hand
  Omnimon exists, the Lv6 leaves normally and BT17-095 stays.
- Rules/keyword basis: `general_rule.pdf` §16 ＜Delay＞ + DNA digivolve timing;
  DCGO `BT17_095.cs` Clause B (`WhenRemoveField` + `SetJogress` DNA merge,
  not by-battle). YAML Clause B (`kind: replacement`,
  `effect_initiated_dna_digivolve_hand_partner`, `cancel_replacement`).
- Rank: 2 (Option, highest-payoff Option interaction — a free finisher off a
  removed Lv6).

### C3 — Digital Gate Open dig + Delay Tamer cheat (Option)
- Cards: **P-206**, plus a board Digimon to define a colour, plus a Tamer in
  hand matching that colour (e.g. **BT22-084 Nokia** / **BT17-081 Tai&Matt**).
- Expected mechanical outcome: [Main] P-206 reveals top 3; player adds **1
  Digimon + 1 Tamer** to hand, rest to deck bottom; P-206 seats as a Delay
  permanent. After the placing turn the Delay cheats a hand Tamer whose colour
  matches a field Digimon into play with **cost reduced by 4**. Before/after: hand
  +2 (dig) then −1 (Tamer played), deck top 3 → bottom, P-206 hand→field→trash,
  +1 Tamer permanent, memory paid = `max(0, tamerCost − 4)`.
- Rules/keyword basis: DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/.../P_206...`
  (OptionSkill reveal-3 add Digimon+Tamer + `PlaceDelayOptionCards`;
  OnDeclaration cost −4 colour-matched). YAML `code/digimon-engine/cards/p/P-206.yaml`
  Clause 0 + Clause 1 (`color_matches_any_field_digimon`, `cost_delta: reduce 4`).
- Rank: 3 (Option, runs 2 copies; ramps the Tamer engine that powers Omnimon).

### C4 — Tai & Matt double-memory off the cross-evolve sequence
- Cards: **BT17-081** (or **EX9-066**) + an own **Greymon-named** Digimon + an
  own **Garurumon-named** Digimon, then any own Digimon play/digivolve.
- Expected mechanical outcome: on each own Digimon play or digivolve, by
  suspending Tai&Matt, gain **+1 memory if a Greymon-named Digimon is on field**
  and **+1 more if a Garurumon-named Digimon is on field** (two independent
  checks → +2 memory when both present). After: Tamer suspended, memory +2.
  Unhappy path: with only a Greymon (no Garurumon), the same trigger grants only
  +1.
- Rules/keyword basis: DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/BT17/.../BT17_081.cs`
  / `EX9_066.cs` (two independent memory grants, by suspending). YAML
  `code/digimon-engine/cards/bt17/BT17-081.yaml` Clause 1 (two independent
  `if any_permanent … name_contains` → `gain_memory: 1`).
- Rank: 4 (the memory engine that makes the same-turn double-evolve → Omnimon
  curve possible; fires constantly).

### C5 — Omnimon Alter-S DNA board wipe + security reset
- Cards: **EX9-021**, a Blue Lv6 (**BT22-026 MetalGarurumon**) + a Red Lv6
  (**BT22-013 WarGreymon**) on board as DNA materials.
- Expected mechanical outcome: DNA-digivolve the Blue Lv6 + Red Lv6 into EX9-021
  (cost 0, stack unsuspended). [When Digivolving], because DNA: EX9-021 becomes
  immune to opponent effects this turn, then **all opponent Digimon with the
  highest level are deleted**. After: both Lv6 materials are consumed into one
  EX9-021 permanent (unsuspended), and the opponent's highest-level Digimon are
  gone. [End of Attack] can later replay a Greymon/Ver.1 + Garurumon/Ver.2 from
  its sources and reseat EX9-021 as top security.
- Rules/keyword basis: `general_rule.pdf` §16 DNA digivolve; DCGO
  `$BASE_DCGO/Assets/Scripts/CardEffect/EX9/.../EX9_021.cs` (immunity inside the
  `IsJogress` block; unconditional delete-highest after). YAML
  `code/digimon-engine/cards/ex9/EX9-021.yaml` Clauses 1–2
  (`dna_origin` immunity + unconditional `for_each highest_level` delete).
- Rank: 5 (the deck's premier finisher fusion; central board-clear payoff).

### Dropped / lower-ranked (logged, not authored under the cap)
- **C6 — ST20-15 security-launder + +2000 DP aura (Option):** [Main] top-security
  → hand, reseat ST20-15 face-up as security granting own Lv3+ Digimon +2000 DP.
  Outcome is checkable (hand +1 from old top sec, ST20-15 now top face-up sec,
  every own Lv3+ Digimon +2000 DP). Ranked just under the cap; high Option value
  but a niche tech card (1 copy in the best list). Author next if cap raised.
- **C7 — WarGreymon cross-evolve → Gabumon becomes MetalGarurumon (BT17-015 /
  BT22-013 branch 0):** **BLOCKED — do not author.** The branch's
  `effect_initiated_digivolve` from a hand card with a permanent target is gated
  by `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-PERMANENT-TARGET` (the chained
  hand-pick prompt never installs; see `BT22-013.yaml`/`BT17-015.yaml` clause
  notes). Route to `docs/RUST_ENGINE_GAPS.md`; not a combo whose pieces are all
  functioning.
- **C8 — Nokia BT5-092 digivolve-cost −1 on Greymon/Garurumon/Omnimon evolve:**
  steady cost discount engine; real and implemented, but a single-card discount
  rather than a multi-card payoff — lower combo centrality than C1–C5.
- **C9 — WarGreymon/MetalGarurumon Omnimon-name inherited security trash/unsuspend
  (BT17-015 / BT22-013 inherited):** `source_name_contains: "Omnimon"` gate is a
  no-op under `G-DSL-SOURCE-NAME-CONTAINS` (the inherited clause degenerates to
  always-true); the underlying `trash_top_security` works but the name gate is
  unverifiable. Logged; not authored.

## Playstyle

- **Class:** combo/tempo midrange. Curve out Agumon/Gabumon → Lv6 → Omnimon,
  using Tamer ramp (Nokia memory-floor + Tai&Matt double-memory) to land a Lv7
  fusion ahead of curve, then close with Omnimon board wipes + free security
  damage.
- **Tempo/memory:** Nokia BT22-084 floors memory to 3 every turn; Tai&Matt grant
  +2 memory per trigger off the cross-evolve sequence, so a single turn can play
  two Lv6s and fuse to Omnimon while passing memory back near zero.
- **Options** smooth the curve: BT17-095 free-plays bases (incl. from trash) and
  banks a finisher Delay; P-206 digs Digimon+Tamer and cheats a Tamer; ST20-15
  buffs the whole board through security.

## Win conditions

1. **Omnimon fusion beatdown** — DNA/Blast into Omnimon (BT17-078 / BT22-015 /
   EX9-021 Alter-S / BT20-102 X), wipe/bounce the opponent's board, then push
   15000–16000 DP attackers with ＜Raid＞/＜Blocker＞/＜Piercing＞.
2. **Free security pressure** — Omnimon-name inherited [When Attacking] trashes
   the opponent's top security each turn; EX9-021 [End of Attack] reseats itself
   as security to grind; Tai&Matt BT17-081 grants an extra Omnimon attack at end
   of turn; Koromon (BT14-001) inherited draws on opponent-security removal.
3. **Tempo lockout** — MetalGarurumon's "can't-suspend" clauses (BT17-027 /
   BT15-101) and Omnimon bounces deny the opponent blockers/attackers while
   Omnimon races the security stack.

## Ranked interactions to test

1. **C1 — Mega Knight free-play recursion (Option)** — highest-frequency Option
   line; verifies free play from **trash** + the played base's own [On Play] +
   self-seating as a Delay permanent.
2. **C2 — Mega Knight Delay hand-DNA Omnimon (Option)** — highest-payoff Option
   interaction; verifies the cross-permanent ＜Delay＞ leave-watcher, the
   hand-partner DNA merge, and the leave-cancel.
3. **C3 — P-206 dig + Delay Tamer cheat (Option)** — verifies reveal-3 add
   Digimon+Tamer, Delay seating, and the colour-matched cost −4 Tamer cheat.
4. **C4 — Tai & Matt double-memory** — verifies the two **independent** memory
   grants (Greymon present and Garurumon present) and the suspend cost.
5. **C5 — Omnimon Alter-S DNA wipe** — verifies DNA-from-two-Lv6 (cost 0),
   DNA-gated immunity, and unconditional delete-all-highest-level.

Dropped under the cap: **C6** (ST20-15 security buff — author next if cap
raised), **C7** (BLOCKED on `G-EFFECT-INITIATED-DIGIVOLVE-FROM-HAND-WITH-
PERMANENT-TARGET`), **C8** (BT5-092 single-card discount), **C9** (Omnimon-name
inherited gate no-op under `G-DSL-SOURCE-NAME-CONTAINS`).
