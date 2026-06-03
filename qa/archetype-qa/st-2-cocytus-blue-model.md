# ST-2 Cocytus Blue — Model

Interaction tests: `code/digimon-engine/tests/archetypes/st2.rs`
Per-card behavioral tests: `code/digimon-engine/tests/cards_behavioral/st2/st2_cards.rs`

**System summary:** A Blue control deck whose core loop is **stripping digivolution
cards off the bottom of the opponent's Digimon** until a Digimon has *zero*
digivolution cards ("bare" / "no digivolution cards"). That bare state is a
board-wide trigger condition that three separate payoffs key off — WereGarurumon's
Security Attack window, Matt's start-of-turn memory engine, and Sorrow Blue's lock.
The unifying mechanical fact: **stripping the LAST source flips a Digimon bare, and
that single flip simultaneously enables every "no digivolution cards" payoff.**

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST2-01 Tsunomon (DigiEgg) | payoff (inherited) | [Your Turn] +1000 DP when battling an opp Digimon with no digivolution cards |
| ST2-02 Gomamon (Rookie) | body | vanilla Rookie |
| ST2-03 Gabumon (Rookie) | enabler (inherited) | [When Attacking] trash bottom source of 1 opp Digimon level ≤5 |
| ST2-04 Bearmon (Rookie) | body | vanilla Rookie |
| ST2-05 Ikkakumon (Champion) | body | vanilla Champion |
| ST2-06 Garurumon (Champion) | enabler (inherited) | [When Attacking] trash bottom source of 1 opp Digimon (no level cap) |
| ST2-07 Grizzlymon (Champion) | tech | Blocker; [When Attacking] lose 2 memory |
| ST2-08 WereGarurumon (Ultimate) | payoff (inherited) | [Your Turn] while opp has a bare Digimon, this Digimon gains <Security A. +1> |
| ST2-09 Zudomon (Ultimate) | enabler | [When Digivolving] trash 2 bottom sources of 1 opp Digimon |
| ST2-10 Plesiomon (Mega) | body | vanilla Mega |
| ST2-11 MetalGarurumon (Mega) | finisher | [When Attacking] [Once Per Turn] unsuspend this Digimon |
| ST2-12 Matt Ishida (Tamer) | engine | [Start of Your Turn] gain 1 memory if opp has a bare Digimon; Security: play free |
| ST2-13 Hammer Spark (Option c0) | tempo | [Main] +1 memory; Security: +2 memory |
| ST2-14 Sorrow Blue (Option c2) | payoff/lock | [Main] a bare opp Digimon can't attack/block until end of opp's next turn |
| ST2-15 Kaiser Nail (Option c4) | recursion | [Main] play a Digimon digivolution card under YOUR Digimon for free |
| ST2-16 Cocytus Breath (Option c7) | removal | [Main] return 1 opp Digimon to hand |

## Digivolution lines

- Blue Bukamon/Tsunomon egg → Gomamon (ST2-02) → Ikkakumon (ST2-05) → Zudomon
  (ST2-09) → Plesiomon (ST2-10). The Sea-Beast line; Zudomon's digivolve trigger
  is a strip-2.
- Tsunomon egg → Gabumon (ST2-03) → Garurumon (ST2-06) → WereGarurumon (ST2-08) →
  MetalGarurumon (ST2-11). The Garuru line; Gabumon/Garurumon carry the
  When-Attacking strip inherited effects, and the WereGarurumon payoff sits on top.
- Bearmon (ST2-04) → Grizzlymon (ST2-07) is the Blocker side-branch.

The crucial structure: the **strip enablers are inherited effects** (ST2-03,
ST2-06) carried by under-cards in a stack, so a tall Garuru stack keeps stripping
on every attack while the top body advances. The **payoffs are also inherited**
(ST2-01, ST2-08), so they fire from anywhere in the line.

## Named combos

### 1. Strip → WereGarurumon Security-Attack flips on
- Cards: ST2-08 WereGarurumon (carrier) + a stripped (bare) opp Digimon.
- Expected mechanical outcome: `security_attack_keyword_bonus(carrier) == 1` while
  the opponent has any bare Digimon; `== 0` when every opp Digimon still has ≥1
  source. The flip is driven entirely by the opponent's source-count state.
- Rules/keyword basis: "Security Attack +1" (<S・A・+1>) = `general_rule.pdf` §16
  (keyword effects, attack-step / checking-security count); digivolution-card =
  source under a Digimon. DCGO C# `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_08.cs`.
- Rank: A (the headline payoff; per-card test `st2_08_…` pins the read, this asserts
  the strip-driven *flip* across before/after).

### 2. Strip → Sorrow Blue lock (gated)
- Cards: ST2-14 Sorrow Blue + 1 bare opp Digimon + 1 sourced opp Digimon.
- Expected mechanical outcome: Sorrow Blue's [Main] target prompt offers **exactly
  the bare Digimon** (count 1); after resolving, the bare one carries
  `ModifierType::CannotAttack` + `CannotBlock` and the sourced one carries neither.
- Rules/keyword basis: card text gates the target on "with no digivolution cards";
  can't-attack/can't-block are turn-window restriction modifiers
  (`general_rule.pdf` §6 attack declaration / §8 block timing). DCGO C#
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_14.cs`.
- Rank: A.

### 3. Strip → Matt memory engine (gated)
- Cards: ST2-12 Matt Ishida + opp Digimon, sourced then stripped bare.
- Expected mechanical outcome: firing `StartOfYourTurn` with only sourced opp
  Digimon gains **0** memory; after stripping one to bare, firing again gains **+1**.
- Rules/keyword basis: card text conditions the gain on "if your opponent has a
  Digimon with no digivolution cards"; Start-of-Your-Turn timing
  (`general_rule.pdf` §15 turn structure / start-of-turn timing). DCGO C#
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_12.cs`.
- Rank: A.

### 4. Garurumon strip → WereGarurumon turns on (real strip, end to end)
- Cards: ST2-06 Garurumon (inherited strip carrier) + ST2-08 WereGarurumon (payoff
  carrier) + a 1-source opp Digimon.
- Expected mechanical outcome: a 1-source opp Digimon leaves WereGarurumon's bonus
  at 0; firing Garurumon's [When Attacking] trashes that last source → opp Digimon
  is now bare → re-ticking declaratives, `security_attack_keyword_bonus` becomes 1.
  Strips the actual source rather than modeling the bare state by hand.
- Rules/keyword basis: [When Attacking] inherited timing + bottom-source trash
  (`general_rule.pdf` §16 inherited effects / §6 attack step). DCGO C#
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_06.cs` + `ST2_08.cs`.
- Rank: A (the canonical full loop — proves the strip→bare→payoff chain mechanically,
  not just the bare-state read).

### 5. Zudomon strip-2 reaches bare on a tall stack
- Cards: ST2-09 Zudomon ([When Digivolving] strip-2) + a 2-source opp Digimon +
  ST2-12 Matt as the bare-state observer.
- Expected mechanical outcome: firing Zudomon's digivolve trigger trashes both
  bottom sources of the opp Digimon, leaving it bare; Matt's start-of-turn gain then
  reads 1 (it read 0 while the Digimon still had its two sources).
- Rules/keyword basis: [When Digivolving] timing + 2× bottom-source trash; same
  bare-state condition feeding ST2-12. DCGO C#
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_09.cs`.
- Rank: B (a second strip path reaching the same shared bare-state trigger).

### 6. Kaiser Nail recurs a digivolution source from your own stack
- Cards: ST2-15 Kaiser Nail + your own Blue stack (ST2-03 under ST2-05).
- Expected mechanical outcome: play Kaiser Nail, select the under-card source
  (ST2-03), assert it is played to the field for free — your battle-area count +1,
  the source leaves the host stack, memory unchanged (free play).
- Rules/keyword basis: card text "play it without paying the cost"; playing a
  digivolution card from under a Digimon (`general_rule.pdf` §16 / play rules).
  DCGO C# `$BASE_DCGO/Assets/Scripts/CardEffect/ST2/Blue/ST2_15.cs`.
- Rank: B (the deck's recursion line — distinct from the strip system but a core
  multi-card interaction with the player's own stack).

## Playstyle / Win conditions

Blue tempo-control: defend with Grizzlymon's Blocker and Cocytus Breath bounce, grind
the opponent's board into bareness with the inherited strip effects, then convert the
bare state into pressure (WereGarurumon Security Attack +1, MetalGarurumon
unsuspend re-attacks) and inevitability (Matt's recurring memory, Sorrow Blue locking
their best Digimon out of attacking/blocking). The deck rarely "removes" — it
*disarms*: a bare Digimon can't digivolve-back its protection, so stripping is a
permanent tempo tax. Kaiser Nail recurs your own buried bodies to rebuild after trades.

## Ranked interactions to test

| # | Combo | Became #[test]? |
|---|-------|-----------------|
| 1 | Strip → WereGarurumon Security-Attack flip | YES — `strip_flips_weregarurumon_security_attack_bonus` |
| 2 | Strip → Sorrow Blue lock (gated) | YES — `sorrow_blue_locks_only_the_bare_opponent_digimon` |
| 3 | Strip → Matt memory engine (gated) | YES — `matt_memory_engine_gated_on_bare_opponent` |
| 4 | Garurumon real strip → WereGarurumon on | YES — `garurumon_strip_turns_weregarurumon_security_attack_on` |
| 5 | Zudomon strip-2 → bare → Matt | YES — `zudomon_strip_two_reaches_bare_and_feeds_matt` |
| 6 | Kaiser Nail recurs own source | YES — `kaiser_nail_recurs_own_digivolution_source_for_free` |

### Blocked / dropped
- None. All six ranked interactions became tests. MetalGarurumon's unsuspend (ST2-11)
  and Tsunomon's battle-DP buff (ST2-01) are pinned at the per-card level
  (`st2_11_…`, `st2_01_…`) and were not promoted to a separate interaction test
  because they do not depend on the bare-state system the way combos 1–5 do, and
  combining them adds no cross-card signal the per-card tests miss.
