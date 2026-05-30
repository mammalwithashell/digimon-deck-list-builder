# ST-2 Cocytus Blue — Model

Scope: the 16 unique cards of the ST-2 *Cocytus Blue* starter deck (ST2-01 … ST2-16), all
mono-Blue, all implemented as DSL YAML and audited faithful (`status: IMPLEMENTED`,
report `implement-st2-cocytus-blue-substrate`, 2026-05-29, in
`qa/qa-reports/validated_cards_dsl.json`). This is the SCOUT model that drives the
interaction-test author; it does not re-implement cards.

Engine semantic that underpins the whole archetype (verified, not assumed):
`materials_count = perm.card_sources.len().saturating_sub(1)`
(`code/digimon-engine/src/dsl_cards/predicate.rs:2511`). So `materials_count == 0` means
"top card only, **no digivolution cards underneath**" — the rules definition of a
source-less Digimon and the engine equivalent of DCGO's `Permanent.HasNoDigivolutionCards`
(`ST2_01.cs`, `ST2_08.cs`, `ST2_12.cs`). A Digimon played straight from hand starts at
`materials_count == 0`; one that digivolved naturally has its lower cards as sources and is
NOT source-less until those are stripped. This single fact is the hinge of every combo
below.

Rules basis for "trash the bottom digivolution card": removing the bottom of the
digivolution stack (general_rule.pdf §6-8 / §6-9 digivolution-card handling; trashing
sources does not delete the Digimon, only thins its stack). "No digivolution cards" status
is re-evaluated continuously, so stripping a source flips the no-source payoffs ON the
instant the last source leaves (static-ability re-check, general_rule.pdf §16 keyword/static
timing; DCGO models the payoffs as `ChangeSelf*StaticEffect` with a live `Condition()`).

## Card pool & roles

| Card | Name | Lvl | Role | Mechanically relevant text | Source refs |
|------|------|-----|------|----------------------------|-------------|
| ST2-01 | Tsunomon | 2 (egg) | **No-source payoff (egg)** | Inherited [Your Turn] +1000 DP when battling an opponent Digimon with no digivolution cards | `cards/st2/ST2-01.yaml` (`battle_opponent_no_sources`); `DCGO/.../ST2/Blue/ST2_01.cs` |
| ST2-02 | Gomamon | 3 | Vanilla Rookie (Ikkakumon/Zudomon line bottom) | none | `ST2-02.json` |
| ST2-03 | Gabumon | 3 | **Strip engine (lvl ≤5 gated)** | Inherited [When Attacking] Trash bottom source of 1 opp Digimon, level 5 or less | `ST2-03.yaml`; `ST2_03.cs` (`permanent.Level <= 5`) |
| ST2-04 | Bearmon | 3 | Vanilla Rookie (4000 DP body) | none | `ST2-04.json` |
| ST2-05 | Ikkakumon | 4 | Vanilla Champion (→ Zudomon) | none | `ST2-05.json` |
| ST2-06 | Garurumon | 4 | **Strip engine (uncapped)** | Inherited [When Attacking] Trash bottom source of 1 opp Digimon (no level cap) | `ST2-06.yaml`; `ST2_06.cs` |
| ST2-07 | Grizzlymon | 4 | Defensive body | ＜Blocker＞; [When Attacking] Lose 2 memory | `ST2-07.json` |
| ST2-08 | WereGarurumon | 5 | **No-source payoff (pressure)** | Inherited [Your Turn] while opp has a no-source Digimon, gains ＜Security A.+1＞ | `ST2-08.yaml`; `ST2_08.cs` |
| ST2-09 | Zudomon | 5 | **Strip burst (2 sources, on-evo)** | [When Digivolving] Trash 2 bottom sources of 1 opp Digimon | `ST2-09.yaml`; `ST2_09.cs` (`trashCount: 2`) |
| ST2-10 | Plesiomon | 6 | Big vanilla Mega (12000 DP) | none | `ST2-10.json` |
| ST2-11 | MetalGarurumon | 6 | **Repeated-attack finisher** | [When Attacking][Once Per Turn] Unsuspend this Digimon | `ST2-11.yaml`; `ST2_11.cs` (`SetHashString("Unsuspend_ST2_11")`, OPT) |
| ST2-12 | Matt Ishida | — (Tamer) | **No-source payoff (memory engine)** | [Start of Your Turn] if opp has a no-source Digimon, gain 1 memory; [Security] play free | `ST2-12.yaml`; `ST2_12.cs` |
| ST2-13 | Hammer Spark | — (Option) | Ramp | [Main] gain 1 memory; [Security] gain 2 memory | `ST2-13.json` |
| ST2-14 | Sorrow Blue | — (Option) | **No-source lockdown** | [Main]/[Security] a **no-source** opp Digimon can't attack/block until end of next turn | `ST2-14.yaml` |
| ST2-15 | Kaiser Nail | — (Option) | Source recursion / tempo | [Main]/[Security] play a Digimon source from under one of your Digimon, free | `ST2-15.yaml` |
| ST2-16 | Cocytus Breath | — (Option) | Bounce removal | [Main]/[Security] return 1 opp Digimon to hand | `ST2-16.yaml` |

## Digivolution lines

- **Garurumon → MetalGarurumon (the namesake line, Blue tempo):**
  Tsunomon (ST2-01 egg) → Gabumon (ST2-03, lvl3) → Garurumon (ST2-06, lvl4) →
  WereGarurumon (ST2-08, lvl5) → MetalGarurumon (ST2-11, lvl6). This line carries the strip
  inheritables (Gabumon/Garurumon trash a bottom source [When Attacking]) up through the
  stack, then caps with WereGarurumon (no-source Security pressure) and MetalGarurumon
  (double-attack finisher). The Tsunomon egg under it provides the +1000-vs-source-less
  inheritable at the very bottom of the stack.
- **Gomamon → Zudomon (the burst-strip line):**
  Gomamon (ST2-02, lvl3) → Ikkakumon (ST2-05, lvl4) → Zudomon (ST2-09, lvl5) → Plesiomon
  (ST2-10, lvl6 Mega). Zudomon's [When Digivolving] strips **2** sources at once — the
  fastest way to flip an opponent Digimon source-less in one play.
- Cross-line: Bearmon (ST2-04) / Grizzlymon (ST2-07) are Blue lvl3/4 filler bodies that any
  lvl2/lvl3 source can climb; Grizzlymon adds a ＜Blocker＞ defensive body and Bearmon a
  4000-DP wall. Kaiser Nail (ST2-15) can re-play a Digimon source out from under a stack
  (note: this is your OWN stack, raising one of your sources back to the field — a tempo /
  re-trigger tool, not part of the opponent-strip engine).

## Named combos

### Combo A — Strip → no-source payoff stack (the archetype's core engine)
- **Cards:** ST2-09 Zudomon (or ST2-06 Garurumon / ST2-03 Gabumon as the strip)
  **+** ST2-01 Tsunomon (egg inheritable) **+** ST2-08 WereGarurumon **+** ST2-12 Matt Ishida.
- **Expected mechanical outcome:** Take an opponent Digimon that digivolved naturally (so
  it has ≥1 source, `materials_count ≥ 1`). Strip its sources to zero — Zudomon does it in
  one shot ([When Digivolving] trash 2; if the target had exactly 2 sources it is now
  source-less; `ST2_09.cs` `trashCount: 2`), or Garurumon/Gabumon do it one-per-attack.
  The instant `materials_count` hits 0 on that opponent Digimon, ALL three payoffs flip ON
  simultaneously: (1) any of YOUR Digimon carrying Tsunomon's inheritable gets +1000 DP
  *when battling that target* (`battle_opponent_no_sources`, `ST2-01.yaml`); (2) a Digimon
  carrying WereGarurumon's inheritable gains ＜Security A.+1＞ for the rest of your turn
  (`ST2-08.yaml`, `any_permanent of: opponent materials_count_lte: 0`); (3) on YOUR next
  Start of Turn, Matt Ishida nets +1 memory (`ST2-12.yaml`, same `materials_count_lte: 0`
  condition; `ST2_12.cs` `HasNoDigivolutionCards`).
- **Rules/keyword basis:** general_rule.pdf §6-8/§6-9 (trashing bottom digivolution cards
  thins the stack without deleting), §16 static-ability continuous re-evaluation (payoffs
  read live no-source status). DCGO: `ST2_09.cs`, `ST2_01.cs` (`enemy.HasNoDigivolutionCards`
  in the DP `Condition`), `ST2_08.cs` (`HasMatchConditionOpponentsPermanent … HasNoDigivolutionCards`),
  `ST2_12.cs`. Engine hinge: `materials_count = card_sources.len()-1` (`predicate.rs:2511`).
- **Rank: 1.** This is the deck's reason to exist and spans 4 distinct cards across both
  digivolution lines; it exercises the exact boolean (`materials_count_lte: 0`) that three
  separate effects share, so a single off-by-one in source counting would silently break
  all three at once.

### Combo B — WereGarurumon Security pressure under a fresh strip
- **Cards:** ST2-08 WereGarurumon (carrying the line, or as inheritable under MetalGarurumon)
  **+** any strip (ST2-09 Zudomon / ST2-06 Garurumon) **+** opponent with a single
  source-less Digimon.
- **Expected mechanical outcome:** While the opponent has at least one no-source Digimon,
  WereGarurumon's attacker checks **+1 extra security card** this turn — turning a normal
  Security attack into a 2-card check. The test asserts: with the opponent at `materials_count
  ≥ 1` everywhere, a WereGarurumon attack checks 1 security; after a strip drops one
  opponent Digimon to `materials_count == 0`, the SAME WereGarurumon (or a Digimon carrying
  its inheritable) now checks 2 security cards on attack — and crucially the bonus keys off
  ANY opponent no-source Digimon, not the one being attacked (`ST2-08.yaml`
  `any_permanent of: opponent`; `ST2_08.cs` `HasMatchConditionOpponentsPermanent`).
- **Rules/keyword basis:** glossary.pdf ＜Security Attack +N＞ keyword (check N additional
  security); general_rule.pdf §16 keyword grant via static condition. DCGO `ST2_08.cs`
  (`ChangeSelfSAttackStaticEffect(changeValue: 1)` gated on owner-turn + opponent has
  no-source Digimon).
- **Rank: 2.** Real cross-card interaction (strip enables the keyword grant), distinct from
  Combo A's DP/memory payoffs, and the "any opponent no-source Digimon, not the battle
  target" scoping is a subtle edge worth a dedicated assertion.

### Combo C — MetalGarurumon double-attack as a strip multiplier / closer
- **Cards:** ST2-11 MetalGarurumon **+** an inherited strip in its stack
  (ST2-06 Garurumon and/or ST2-03 Gabumon as digivolution sources) **±** ST2-08 WereGarurumon
  inheritable for ＜Security A.+1＞.
- **Expected mechanical outcome:** MetalGarurumon attacks; [When Attacking][Once Per Turn]
  it unsuspends itself (`ST2-11.yaml` `unsuspend target: source`; `ST2_11.cs`, OPT via
  `SetHashString("Unsuspend_ST2_11")`). If Garurumon/Gabumon's [When Attacking] strip is in
  its digivolution stack, each attack also fires that inheritable — but the unsuspend is
  Once Per Turn, so the test must assert exactly **two attacks** in one turn (one base, one
  from the single unsuspend), NOT an infinite loop, and that the OPT flag blocks a second
  unsuspend. With WereGarurumon's inheritable present, both attacks can be ＜Security A.+1＞
  while the opponent stays source-less — a fast clock.
- **Rules/keyword basis:** general_rule.pdf §16 [Once Per Turn] timing-restriction (one
  activation per turn regardless of re-attacks); §16 [When Attacking] inherited-effect
  resolution per attack. DCGO `ST2_11.cs` (OPT-tagged unsuspend), `ST2_06.cs`/`ST2_03.cs`.
- **Rank: 3.** Genuine multi-card interaction (finisher + inherited strips + OPT gating) but
  more of a tempo/closer than the engine; its highest-value assertion is the **OPT boundary**
  (exactly one unsuspend → exactly two attacks), which is a correctness guard rather than a
  novel payoff.

## Playstyle

Mid-speed Blue tempo/control. The deck does not race on raw DP — it **degrades** the
opponent's board. It climbs the Garurumon line (Gabumon → Garurumon → WereGarurumon →
MetalGarurumon) and/or the Gomamon → Zudomon line, attacking with strip-inheritables in the
stack so that every attack peels a bottom source off an opponent Digimon. Zudomon's
on-digivolve double-strip is the burst enabler. Once an opponent Digimon is source-less, the
deck's three payoffs (Tsunomon +1000 DP in battle, WereGarurumon ＜Security A.+1＞, Matt +1
memory/turn) all turn on, giving favorable trades, faster security pressure, and a memory
lead. Sorrow Blue locks a no-source Digimon out of attacking/blocking; Cocytus Breath
bounces a problem body; Kaiser Nail/Hammer Spark provide tempo and ramp. MetalGarurumon
closes with its OPT self-unsuspend for two attacks a turn.

## Win conditions

1. **Security pressure under the no-source engine:** strip an opponent Digimon, then attack
   with WereGarurumon (＜Security A.+1＞) and/or a double-attacking MetalGarurumon to chew
   through security faster than the opponent rebuilds, then land the lethal direct attack.
2. **Tempo + memory snowball:** Matt Ishida's recurring +1 memory (while any opponent
   Digimon is source-less) plus Hammer Spark ramp fund repeated strips and re-deploys,
   denying the opponent stable digivolution while the Garurumon line out-trades on the
   +1000-DP-vs-source-less Tsunomon inheritable.
3. **Disruption-to-attrition:** Cocytus Breath bounce + Sorrow Blue lockdown neutralize the
   opponent's best body each turn so the strip engine and security checks finish the game.

## Ranked interactions to test

1. **(Rank 1, Combo A) Strip → triple no-source payoff flip.** Zudomon [When Digivolving]
   trashes both sources of an opponent Digimon that had exactly 2; assert that Digimon is now
   `materials_count == 0`, that a Tsunomon-inheritable attacker gets +1000 DP **only** while
   battling it, that a WereGarurumon-inheritable Digimon now has ＜Security A.+1＞, and that
   Matt Ishida gains +1 memory on the controller's next Start of Turn. (Variant: same flip via
   Garurumon ST2-06 [When Attacking] single-strip across two attacks.) Cross-card, shared
   `materials_count_lte: 0` boolean — highest leverage.
2. **(Rank 2, Combo B) Security Attack +1 keys off any opponent no-source Digimon.** Before a
   strip, a WereGarurumon attack checks 1 security; after a strip drops a (possibly
   non-attacked) opponent Digimon to source-less, the same attack checks 2. Assert the bonus
   reads the *board*, not the battle target.
3. **(Rank 3, Combo C) MetalGarurumon OPT self-unsuspend = exactly two attacks, strip fires
   each.** Assert exactly one unsuspend per turn (OPT), exactly two attacks, and that an
   inherited Garurumon/Gabumon strip in the stack fires on each attack — no infinite loop.

### Dropped candidates (not promoted to interaction tests)
- **Gabumon ST2-03 level-cap edge vs Garurumon ST2-06 uncapped:** Gabumon can only target
  opp Digimon **level ≤ 5** (`ST2_03.cs` `permanent.Level <= 5`), Garurumon any level. Real,
  but a single-card targeting-filter detail better covered by the per-card behavioral test;
  not a multi-card interaction.
- **Kaiser Nail ST2-15 source recursion:** plays a Digimon source from under YOUR OWN
  Digimon — a self-tempo/re-trigger tool, orthogonal to the opponent-strip no-source engine.
  No cross-card synergy with the payoffs; drop.
- **Cocytus Breath / Sorrow Blue removal:** bounce and attack-lock are strong but
  single-target removal/control with no dependency on the no-source engine for Cocytus
  Breath (Sorrow Blue *does* require a no-source target, but that's a one-card precondition,
  not a combo). Covered by per-card tests.
- **Hammer Spark ramp, vanilla bodies (ST2-02/04/05/07/10):** no interaction surface.
