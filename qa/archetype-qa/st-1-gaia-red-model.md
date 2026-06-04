# ST-1 Starter Deck Gaia Red — Model

Durable archetype model for the worldwide **ST-1 Gaia Red** starter deck. Drives
the multi-card interaction tests in
`code/digimon-engine/tests/archetypes/st1.rs`. Per-card behavioral coverage lives
in `code/digimon-engine/tests/cards_behavioral/st1/gaia_red.rs`; this model
captures the cross-card SYSTEM that per-card TDD cannot see.

Red mono-color aggro: a DP-pumping, Security-Attack–stacking beatdown that races
the opponent's security stack while using cheap-ish Red removal options to clear
blockers. All 16 cards are implemented in the DSL with green per-card tests.

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST1-01 Koromon (DigiEgg Lv2) | enabler (inherited) | INH [Your Turn] +1000 DP while carrier has 4+ digivolution cards |
| ST1-02 Biyomon (Rookie Lv3, 3000) | body | vanilla rookie |
| ST1-03 Agumon (Rookie Lv3, 2000) | enabler (inherited) | INH [Your Turn] +1000 DP — flat inherited DP source |
| ST1-04 Dracomon (Rookie Lv3, 4000) | body | vanilla rookie (4000 base — Giga Destroyer window edge) |
| ST1-05 Birdramon (Champion Lv4, 5000) | body | vanilla champion (5000 base — survives Giga Destroyer) |
| ST1-06 Coredramon (Champion Lv4, 6000) | tech | Blocker; [When Attacking] lose 2 memory |
| ST1-07 Greymon (Champion Lv4, 4000) | enabler (inherited) | INH flat `<Security A. +1>` keyword grant to carrier |
| ST1-08 Garudamon (Ultimate Lv5, 7000) | payoff | [When Digivolving] 1 of your Digimon +3000 DP for the turn |
| ST1-09 MetalGreymon (Ultimate Lv5, 7000) | tech (inherited) | INH [Your Turn] when this Digimon is blocked, gain 3 memory |
| ST1-10 Phoenixmon (Mega Lv6, 12000) | body | vanilla mega |
| ST1-11 WarGreymon (Mega Lv6, 12000) | payoff | [Your Turn] +1 `<Security A.>` per 2 digivolution cards (dynamic aura) |
| ST1-12 Tai Kamiya (Tamer) | engine | [Your Turn] all YOUR Digimon +1000 DP; INH/Sec: play free from security |
| ST1-13 Shadow Wing (Option cost1) | enabler / pump | [Main] 1 of your Digimon +3000 DP; [Security] all your Digimon `<Sec A.+1>` |
| ST1-14 Starlight Explosion (Option cost2) | defensive | [Main] all your Security Digimon +7000 DP until end of opp's next turn |
| ST1-15 Giga Destroyer (Option cost6) | removal | [Main] delete up to 2 opp Digimon with ≤4000 DP; [Sec] activate Main |
| ST1-16 Gaia Force (Option cost8) | removal | [Main] delete 1 opp Digimon; [Sec] activate Main |

## Digivolution lines

- Red Agumon line → Greymon (ST1-07) → MetalGreymon (ST1-09) → WarGreymon (ST1-11),
  bred from Koromon (ST1-01). As the stack grows, the inherited DP sources
  (Koromon @ 4+ sources, Agumon flat) and the inherited `<Security A. +1>`
  (Greymon) accumulate UNDER the active top, so a tall WarGreymon stack carries
  BOTH WarGreymon's own dynamic per-2-sources `<Security A.>` aura AND Greymon's
  flat keyword grant simultaneously.
- Red Biyomon → Birdramon (ST1-05) → Garudamon (ST1-08) → Phoenixmon (ST1-10).
  Garudamon's When-Digivolving +3000 is the burst node on this line.

## Named combos

### Security-Attack stacking (Greymon source + WarGreymon top)
- Cards: ST1-11 WarGreymon (active top), ST1-07 Greymon (digivolution source).
- Expected mechanical outcome: the WarGreymon top simultaneously carries the
  flat inherited `<Security A. +1>` from the Greymon source AND its own dynamic
  `<Security A. +1>`-per-2-sources aura. With 4 digivolution cards the dynamic
  bonus is +2 (4 ÷ 2). The two bonuses are independent and both present.
- Rules basis: `general_rule.pdf` §16 (Security Attack keyword); inherited effects
  resolve from below the top card. DCGO: `$BASE_DCGO/Assets/Scripts/CardEffect/ST1/Red/ST1_11.cs`,
  `ST1_07.cs`.
- Rank: A (the deck's primary win-condition lever — extra security checks).

### Tai + Shadow Wing lethal DP push
- Cards: ST1-12 Tai Kamiya (tamer, +1000 aura), ST1-13 Shadow Wing ([Main] +3000),
  on one own attacker.
- Expected mechanical outcome: an attacker's `effective_dp` = base + 1000 (Tai
  aura) + 3000 (Shadow Wing modifier). Unhappy path: an opponent Digimon is NOT
  buffed by either (Tai is own-only; Shadow Wing targets your Digimon).
- Rules basis: `general_rule.pdf` §11 (DP), aura vs. one-shot DP modifier stacking.
  DCGO: `ST1_12.cs`, `ST1_13.cs`.
- Rank: A (combat-trade math — survives a same-DP counterattack and trades up).

### Garudamon When-Digivolving + Tai
- Cards: ST1-08 Garudamon ([When Digivolving] +3000 to one), ST1-12 Tai (+1000 aura),
  on the same Digimon.
- Expected mechanical outcome: the +3000 one-shot modifier and the +1000 aura
  stack additively on the chosen target's `effective_dp` (base + 4000).
- Rules basis: `general_rule.pdf` §11 (DP) + §16 When-Digivolving timing. DCGO:
  `ST1_08.cs`, `ST1_12.cs`.
- Rank: B (the burst node — pushes an Ultimate to lethal-trade range for a turn).

### Starlight Explosion security wall
- Cards: ST1-14 Starlight Explosion ([Main]).
- Expected mechanical outcome: playing the [Main] installs the player-level
  `ChangeOwnSecurityDigimonDp = 7000` modifier — every Security Digimon the
  attacker hits is +7000 DP, blunting an incoming rush. Composed with Shadow
  Wing's [Security] `<Security A. +1>` theme conceptually but mechanically the
  modifier value is the assertion.
- Rules basis: `general_rule.pdf` §11 (DP), security-Digimon DP windows. DCGO:
  `ST1_14.cs`.
- Rank: B (defensive pivot — the deck's only stall tool).

### Giga Destroyer DP-window removal (Tai does not widen it)
- Cards: ST1-15 Giga Destroyer ([Main]), opponent Digimon at 2000 / 4000 / 5000 DP,
  ST1-12 Tai on YOUR field.
- Expected mechanical outcome: Giga Destroyer deletes only opponent Digimon at
  ≤4000 effective DP. A 5000-DP opponent Digimon survives. Tai buffs YOUR
  Digimon, not the opponent's, so the opponent's DP window is unchanged — the
  same two ≤4000 targets are deletable and the 5000 one survives whether or not
  Tai is on your field.
- Rules basis: `general_rule.pdf` §11 (DP comparison), deletion semantics. DCGO:
  `ST1_15.cs`.
- Rank: B (removal — important the filter reads the opponent's DP, not yours).

## Playstyle / Win conditions

Red Gaia aggro applies early pressure with cheap rookies, builds a tall Agumon
line, and wins by over-checking security: WarGreymon + an inherited Greymon
source delivers multiple `<Security A.>` checks per swing while Tai (+1000 to all
own Digimon) and Shadow Wing (+3000 one-shot) keep attackers ahead in combat
math. Garudamon's When-Digivolving +3000 is a burst node for a lethal trade.
Removal (Giga Destroyer for ≤4000 swarms, Gaia Force for anything) clears
blockers. Starlight Explosion is the lone defensive pivot, buffing security
Digimon by +7000 to survive a return rush.

## Ranked interactions to test

| # | Interaction | Rank | Test? |
|---|-------------|------|-------|
| 1 | Security-Attack stacking (Greymon src + WarGreymon dynamic) | A | ✅ `wargreymon_over_greymon_source_stacks_both_security_attack_bonuses` + unhappy `wargreymon_fewer_sources_lowers_dynamic_security_attack_bonus` |
| 2 | Tai + Shadow Wing DP push | A | ✅ `tai_plus_shadow_wing_stacks_dp_on_own_attacker` + unhappy `tai_and_shadow_wing_do_not_buff_opponent_digimon` |
| 3 | Garudamon WD + Tai | B | ✅ `garudamon_when_digivolving_buff_stacks_additively_with_tai_aura` |
| 4 | Starlight Explosion security wall | B | ✅ `starlight_explosion_installs_own_security_digimon_dp_window` |
| 5 | Giga Destroyer DP-window removal | B | ✅ `giga_destroyer_deletes_only_le_4000_opponents_and_tai_does_not_widen_window` |

### Blocked / dropped

None — all five ranked interactions became `#[test]`s. (Combo #5 was marked
optional in the brief and is included.)
