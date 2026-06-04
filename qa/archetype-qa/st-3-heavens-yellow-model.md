# ST-3 Heaven's Yellow — Model

Durable archetype model for the ST-3 starter deck. Drives the interaction tests in
`code/digimon-engine/tests/archetypes/st3.rs`. Per-card behavioral coverage lives in
`code/digimon-engine/tests/cards_behavioral/st3/st3_starter.rs`; this model captures the
**cross-card system** those per-card tests can't see.

## System summary

A Yellow control deck whose whole gameplan is **−DP removal**: stack enough negative-DP
modifiers on an opponent's Digimon to drive its DP to **0 or below**, which deletes it by
rule **17-1-3-1** (a Digimon with 0 DP is deleted). Those deletions are not merely removal —
they are the deck's **engine**: inherited "[Your Turn] [Once Per Turn] when an opponent's
Digimon is deleted by dropping to 0 DP" payoffs on Patamon (ST3-04 → +1 memory) and Tokomon
(ST3-01 → +1000 DP to the carrier) convert each kill into tempo. The defensive half is a
**Security-Digimon wall**: T.K. Takaishi (ST3-12) and Heaven's Gate's security side (ST3-13)
inflate the DP of Digimon revealed from security so they trade up against attackers.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST3-01 Tokomon (DigiEgg) | engine (payoff) | INH: on opp 0-DP deletion, carrier +1000 DP for the turn |
| ST3-02 Salamon (Rookie) | filler | vanilla Lv3 3000 |
| ST3-03 Tapirmon (Rookie) | filler | vanilla Lv3 4000 |
| ST3-04 Patamon (Rookie) | engine (payoff) | INH: on opp 0-DP deletion, gain 1 memory |
| ST3-05 Angemon (Champion) | engine | INH [When Attacking] if 4+ security, +1 memory |
| ST3-06 Gatomon (Champion) | filler | vanilla Lv4 5000 |
| ST3-07 Unimon (Champion) | tech (wall) | Blocker; [When Attacking] −2 memory |
| ST3-08 MagnaAngemon (Ultimate) | enabler (softener) | INH [When Attacking] 1 opp Digimon −1000 DP |
| ST3-09 Angewomon (Ultimate) | engine (defense) | [When Digivolving] if ≤3 security, Recovery +1 (Deck) |
| ST3-10 Magnadramon (Mega) | filler | vanilla Lv6 12000 |
| ST3-11 Seraphimon (Mega) | enabler (softener) | [When Attacking] 1 opp Digimon −4000 DP |
| ST3-12 T.K. Takaishi (Tamer) | tech (wall) | [Opp Turn] your Security Digimon +2000; Sec: play free |
| ST3-13 Heaven's Gate (Option 1) | enabler / tech | [Main] +3000 DP; Sec: all your Digimon AND Sec-Digimon +5000, return to hand |
| ST3-14 Heaven's Charm (Option 2) | enabler (removal) | [Main] 1 opp Digimon −2000 DP; Sec: to hand |
| ST3-15 Holy Flame (Option 2) | tech | [Main] 1 opp Digimon Security A. −3; Sec: all opp Sec A. −1 |
| ST3-16 Seven Heavens (Option 7) | payoff (removal) | [Main] 1 opp Digimon −10000 DP; Sec: activate Main |

## Digivolution lines

- Yellow Angel line: Salamon/Patamon (Lv3) → Angemon/Gatomon (Lv4) → MagnaAngemon /
  Angewomon (Lv5/Ult) → Seraphimon / Magnadramon (Mega). Tokomon (ST3-01) is the DigiEgg the
  breeding line hatches from.
- The two engine payoffs ride underneath as **inherited** effects: Tokomon (egg) and Patamon
  (Rookie) sit in the digivolution stack and observe deletions from beneath whatever is on top.
- The two softeners are [When Attacking]: MagnaAngemon's INH −1000 stacks from beneath, while
  Seraphimon's −4000 fires from the top when it itself attacks.

## Named combos

### 1. −DP-to-0 deletion → inherited payoff chain
- Cards: ST3-16 Seven Heavens (or ST3-14 Heaven's Charm) + ST3-04 Patamon + ST3-01 Tokomon
  (both inherited, stacked under one carrier) + a low-DP opponent Digimon.
- Expected mechanical outcome: the −DP option drives the opp Digimon to ≤0 → it is deleted
  (opp field −1, opp trash +1); Patamon's inherited fires (+1 memory) AND Tokomon's inherited
  fires (carrier +1000 DP for the turn) — both observers in one scenario.
- Unhappy contrast: a −DP that does NOT reach 0 (Heaven's Charm −2000 on a 5000-DP Digimon)
  deletes nothing and fires neither observer.
- Rules/keyword basis: 0-DP deletion = `general_rule.pdf` **17-1-3-1**; inherited effects
  resolve from digivolution sources; both clauses gate on `your_turn` + `once_per_turn`.
  DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST3/Yellow/ST3_01.cs`, `ST3_04.cs`, `ST3_16.cs`, `ST3_14.cs`.
- Rank: A (the deck's core engine loop).

### 2. Seraphimon −4000 softens a too-big Digimon into a deletion
- Cards: ST3-11 Seraphimon [When Attacking] −4000 (or ST3-08 MagnaAngemon INH −1000) + a
  large opp Digimon + a finishing −DP source.
- Expected mechanical outcome: the [When Attacking] −4000 lowers the target's effective DP,
  and a subsequent −DP source (Heaven's Charm) then reaches 0 and deletes it — a kill the
  finisher alone could not make. Assert the effective-DP step-down and the eventual deletion.
- Rank: B.

### 3. Security-Digimon defensive wall (T.K. + Heaven's Gate security)
- Cards: ST3-12 T.K. Takaishi ([Opp Turn] Sec-Digimon +2000) + ST3-13 Heaven's Gate security
  side (all your Digimon AND Sec-Digimon +5000 for the turn).
- Expected mechanical outcome: on the opponent's turn, the defender's Security-Digimon DP
  adjustment stacks T.K.'s +2000 with Heaven's Gate's +5000 = **+7000**, read via
  `defender_security_dp_adjustment(defender)`.
- Rank: B.

### 4. Seven Heavens reaches even a Mega (multi-source −DP stack → 0)
- Cards: ST3-16 Seven Heavens −10000 + ST3-14 Heaven's Charm −2000 (or ST3-08 MagnaAngemon
  INH −1000) on a 12000-DP Mega.
- Expected mechanical outcome: −10000 alone leaves the 12000 Mega at 2000 (survives); adding
  the −2000 pushes it to exactly 0 → deleted, firing the inherited observers (memory +1).
- Rank: B.

## Playstyle / Win conditions

Stall behind blockers (Unimon) and the Security-Digimon wall while assembling −DP removal;
each 0-DP kill refunds memory (Patamon) and grows the board (Tokomon), so the removal engine
snowballs into lethal Security Attacks from the Angel line. Angewomon's Recovery keeps the
security stack ≥4 for Angemon's memory trigger and the wall.

## Ranked interactions to test

1. −DP-to-0 deletion → inherited payoff chain — **#[test]** (happy: both observers; unhappy: no-reach). Rank A.
2. Seraphimon −4000 softens into a deletion — **#[test]**. Rank B.
3. Security-Digimon defensive wall (T.K. + Heaven's Gate) — **#[test]**. Rank B.
4. Seven Heavens + Charm reaches a Mega → 0 deletion — **#[test]**. Rank B.

### Blocked / dropped
- None. All four named combos became `#[test]`s.
- Note: the deletion-observers gate on `your_turn`, so the deletion scenarios run on player 0's
  turn (`turn_count = 0`, default first-player). The wall scenario flips to the opponent's turn
  (`turn_count = 1; turn_player_idx = 1`) because T.K.'s aura is `[Opponent's Turn]`.
