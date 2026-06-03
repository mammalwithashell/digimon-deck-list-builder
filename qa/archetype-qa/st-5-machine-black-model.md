# ST-5 Starter Deck Machine Black — Model

Durable system model for the ST-5 Machine Black starter, written for the
`/archetype-interaction-test-author` capstone. Per-card behavioral coverage
lives in `code/digimon-engine/tests/cards_behavioral/st5/`; the cross-card
SYSTEM combos are pinned in `code/digimon-engine/tests/archetypes/st5.rs`.

ST-5 is a Black / Cyborg-Machine **control wall**: it stalls behind a stack of
Blockers, keeps those blockers up (by granting Blocker, granting Reboot, and
re-using the Tai-Kamiya unsuspend-on-block loop), buffs a blocker with
Digi-Burst so it survives a swing, and grinds the opponent's board down with
De-Digivolve (Laser Eye) and a cost-gated delete (Dark Side Attack). The
end-of-opponent's-turn draw triggers (ToyAgumon / Greymon) reward the opponent
for *not* attacking into the wall.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST5-01 Kapurimon (DigiEgg) | engine (inherited aura) | [Your Turn] while the carrier has Blocker it gets +1000 DP — rewards the wall plan |
| ST5-02 Jazamon (Rookie) | filler | vanilla |
| ST5-03 Agumon (Rookie) | enabler (blocker) | native ＜Blocker＞ on a Lv.3 body |
| ST5-04 ToyAgumon (Rookie) | engine (draw) | INH [End of Opp Turn] if opp didn't attack with a Digimon, ＜Draw 1＞ |
| ST5-05 Commandramon (Rookie) | filler | vanilla |
| ST5-06 Greymon (Champion) | engine (draw) | INH [End of Opp Turn] if opp didn't attack with a Digimon, ＜Draw 1＞ |
| ST5-07 Jazardmon (Champion) | filler | vanilla |
| ST5-08 DarkTyrannomon (Champion) | enabler (blocker) | ＜Blocker＞; [When Attacking] lose 2 memory (down-side) |
| ST5-09 MetalGreymon (Ultimate) | enabler (blocker grant) | [When Digivolving] 1 of your Digimon gains ＜Blocker＞ until end of opp's next turn |
| ST5-10 MetalTyrannomon (Ultimate) | filler | vanilla |
| ST5-11 Megadramon (Ultimate) | enabler (blocker source) | INH ＜Blocker＞ — any carrier over it is a blocker |
| ST5-12 Machinedramon (Mega) | engine (Reboot wall) | [When Digivolving] up to 2 of your Digimon gain ＜Reboot＞ until end of opp's next turn |
| ST5-13 BlitzGreymon (Mega) | payoff (buff) | ＜Security A.+1＞; [Main] ＜Digi-Burst 2＞ · 1 of your Digimon +4000 DP until end of opp's next turn |
| ST5-14 Tai Kamiya (Tamer) | engine (blocker reuse) | [Opp Turn] when you use ＜Blocker＞ to suspend a Digimon, you may suspend Tai to unsuspend 1 of your Digimon; Security: play free |
| ST5-15 Laser Eye (Option, cost 4) | tech (removal) | [Main] ＜De-Digivolve 1＞ up to 2 opp Digimon (can't trash past Lv.3); Security: activate Main |
| ST5-16 Dark Side Attack (Option, cost 5) | tech (removal) | [Main] delete 1 opp Digimon with play cost ≤ 7; Security: activate Main |

## Digivolution lines

- **Egg → wall**: Kapurimon (ST5-01) → Agumon (ST5-03, native Blocker) →
  Greymon (ST5-06) → MetalGreymon (ST5-09, grants Blocker) → BlitzGreymon
  (ST5-13) / Machinedramon (ST5-12). The Kapurimon source keeps feeding +1000
  to whichever carrier is currently a Blocker.
- **Machine line**: Commandramon → Jazardmon (ST5-07) → MetalTyrannomon
  (ST5-10) → Machinedramon (ST5-12) → grants Reboot to the wall.
- **Megadramon (ST5-11)** as a digivolution *source* makes any carrier above it
  a Blocker by inheritance.

## Named combos

### 1. Blocker + Tai unsuspend reuse (real granted blocker)
- Cards: ST5-14 Tai Kamiya + a REAL blocker source (ST5-09 MetalGreymon grant /
  ST5-03 Agumon native / ST5-11 Megadramon inherited).
- Expected mechanical outcome: on the opponent's turn, the opponent attacks; you
  declare the blocker (it suspends), Tai's optional response is offered, you
  accept (Tai suspends), and you unsuspend the blocker — leaving the blocker
  READY to block again while Tai is now suspended. The wall blocks twice per
  opponent turn off one Tamer.
- Rules/keyword basis: `general_rule.pdf` §16 ＜Blocker＞ (suspend-to-block) +
  the opponent-turn trigger window; DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_14.cs`.
- Rank: A (the deck's signature loop).

### 2. MetalGreymon grants Blocker → Kapurimon inherited +1000 turns on
- Cards: ST5-01 Kapurimon (source) under a carrier; ST5-09 MetalGreymon as the
  carrier that grants Blocker.
- Expected mechanical outcome: with no Blocker on the carrier, Kapurimon's
  inherited "+1000 while has Blocker" aura is inactive (effective_dp = base).
  Once MetalGreymon's [When Digivolving] grants Blocker to that carrier (on your
  turn), the aura turns ON → effective_dp += 1000 AND the carrier has ＜Blocker＞.
  Unhappy: without the grant the +1000 never applies.
- Rules/keyword basis: `general_rule.pdf` §11 (DP — auras sum), §16 ＜Blocker＞,
  inherited effects resolve from below the top card; DCGO
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_09.cs`, `ST5_01.cs`.
- Rank: A (a keyword grant flips a conditional inherited aura — invisible to
  either card's per-card test).

### 3. Machinedramon Reboot keeps the wall up
- Cards: ST5-12 Machinedramon, two own Digimon (one chosen for Reboot, one not).
- Expected mechanical outcome: Machinedramon's [When Digivolving] grants ＜Reboot＞
  to up to 2 chosen Digimon; the chosen ones gain Reboot (unsuspend during the
  opponent's unsuspend phase so they stay up as blockers across turns) and a
  non-chosen one does not.
- Rules/keyword basis: `general_rule.pdf` §16 ＜Reboot＞; DCGO
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_12.cs`.
- Rank: B (effect; per-card test already pins the selection, this asserts the
  wall-persistence framing).

### 4. BlitzGreymon Digi-Burst buffs a blocker
- Cards: ST5-13 BlitzGreymon over 2+ sources, buffing a separate Blocker.
- Expected mechanical outcome: [Main] ＜Digi-Burst 2＞ trashes 2 of BlitzGreymon's
  digivolution sources and gives +4000 DP to one of your Digimon — directed onto
  a Blocker so it survives blocking a big attacker. Assert the 2 sources are
  trashed AND the +4000 ChangeDp modifier sits on the chosen blocker.
- Rules/keyword basis: `general_rule.pdf` §16 ＜Digi-Burst＞ (pay by trashing
  digivolution cards) + §11 (DP); DCGO
  `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_13.cs`.
- Rank: B (resource conversion into a defensive DP window).

### 5. Laser Eye De-Digivolve + Dark Side removal package
- Cards: ST5-15 Laser Eye, ST5-16 Dark Side Attack.
- Expected mechanical outcome: Laser Eye De-Digivolves up to 2 opp Digimon (trims
  one digivolution source each, never below Lv.3); Dark Side Attack deletes one
  opp Digimon with play cost ≤ 7 (a cost-8 body is outside the window and
  survives). Run as the deck's removal package — Laser Eye shrinks stacks, Dark
  Side finishes the body — asserting the source-trim and the cost gate.
- Rules/keyword basis: `general_rule.pdf` §16 ＜De-Digivolve＞ + deletion
  semantics; DCGO `$BASE_DCGO/Assets/Scripts/CardEffect/ST5/Black/ST5_15.cs`,
  `ST5_16.cs`.
- Rank: C (mostly single-option behavior, but the removal package is a real
  sequencing line).

## Playstyle / Win conditions

ST-5 does not race; it *outlasts*. It plants Blockers, layers Blocker/Reboot
grants and the Tai loop so the wall keeps absorbing attacks, denies the opponent
free swings (and draws off ToyAgumon/Greymon when the opponent declines to
attack), shrinks opposing threats with De-Digivolve, and removes a key body with
Dark Side Attack. Damage accrues slowly through blockers that survive and the
occasional ＜Security A.+1＞ swing from BlitzGreymon. Win condition: grind the
opponent out of board presence and chip security behind an unbreakable wall.

## Ranked interactions to test

| # | Combo | Status |
|---|-------|--------|
| 1 | Blocker + Tai unsuspend reuse (real ST5-09 granted blocker) | ✅ `#[test]` happy + unhappy (decline) |
| 2 | MetalGreymon grant → Kapurimon inherited +1000 turns on | ✅ `#[test]` happy + unhappy (no grant) |
| 3 | Machinedramon Reboot keeps wall up | ✅ `#[test]` |
| 4 | BlitzGreymon Digi-Burst buffs a blocker | ✅ `#[test]` |
| 5 | Laser Eye De-Digivolve + Dark Side removal package | ✅ `#[test]` (one per option) |

### Blocked / dropped
- None. All 16 cards are implemented in the DSL and every combo above maps to
  real implemented behavior. No combo depends on an unimplemented card or a
  missing engine primitive.
