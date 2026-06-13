# ST-6 Venomous Violet — Model

Durable archetype model for the **ST-6 Venomous Violet** Purple starter deck.
Interaction tests live in `code/digimon-engine/tests/archetypes/st6.rs`; per-card
(mostly structural) tests live in `code/digimon-engine/tests/cards_behavioral/st6/st6_cards.rs`.

The deck is a Purple **trash engine**: it deliberately FILLS the trash (draw-then-trash
attackers, On-Deletion mill) and then RECURS cards back out of it (free plays via Nail
Bone / CresGarurumon Digi-Burst, hand recursion via Dracmon / SkullSatamon). A
sacrifice/deletion sub-theme — Death Claw + Matt-Ishida memory-on-deletion + Pagumon
On-Deletion mill — both removes opposing Lv4-or-lower Digimon and refuels the engine.
VenomMyotismon adds a Retaliation trade-threat top end. WereGarurumon pays you for a
full trash (+2000 DP while 5+ cards in trash on your turn).

## Card pool & roles

| Card | Role | One-line function |
|------|------|-------------------|
| ST6-01 Pagumon (Digi-Egg) | engine | INH [On Deletion] trash top 2 of your deck (mill / fuel) |
| ST6-02 DemiDevimon (Rookie) | filler | vanilla |
| ST6-03 Gabumon (Rookie) | engine | INH [When Attacking] Draw 1, then trash 1 hand card (filter + fuel) |
| ST6-04 Dracmon (Rookie) | recursion | [On Play] may return 1 purple Option (cost 1 or 7) from trash to hand |
| ST6-05 Elecmon (Rookie) | filler | vanilla |
| ST6-06 Garurumon (Champion) | engine | INH [When Attacking] Draw 1, then trash 1 hand card |
| ST6-07 Youkomon (Champion) | filler | vanilla |
| ST6-08 Devimon (Champion) | tech | Blocker; [When Attacking] lose 2 memory |
| ST6-09 Kyukimon (Ultimate) | filler | vanilla |
| ST6-10 SkullSatamon (Ultimate) | recursion | [When Digivolving] may return 1 purple Digimon from trash to hand |
| ST6-11 WereGarurumon (Ultimate) | payoff | INH [Your Turn] while 5+ in your trash, carrier +2000 DP |
| ST6-12 VenomMyotismon (Mega) | payoff/tech | [When Digivolving] up to 2 of your Digimon gain Retaliation (EOON-turn) |
| ST6-13 CresGarurumon (Mega) | payoff | Security A.+1; [Main] Digi-Burst 2 → play 1 purple Lv3 Digimon from trash free |
| ST6-14 Matt Ishida (Tamer) | engine | [Your Turn] when your Digimon deleted, may suspend to gain 1 memory; Sec: play free |
| ST6-15 Death Claw (Option, cost 1) | enabler/removal | [Main] may delete YOUR Digimon to delete 1 opp Lv4-or-lower; Sec: delete opp Lv4-or-lower |
| ST6-16 Nail Bone (Option, cost 7) | payoff/recursion | [Main] play 1 purple Lv3 AND 1 purple Lv4 from trash free, On-Play suppressed; Sec: play 1 Lv4-or-lower free |

## Digivolution lines

- Purple Rookie (DemiDevimon/Gabumon/Dracmon/Elecmon) → Champion (Garurumon/Youkomon/Devimon)
  → Ultimate (Kyukimon/SkullSatamon/WereGarurumon) → Mega (VenomMyotismon/CresGarurumon).
- Pagumon (Digi-Egg, In-Training) sits at the bottom of stacks as an inherited On-Deletion
  source — its mill fires whenever the *carrier* it is under is deleted.
- CresGarurumon / VenomMyotismon both alt-digivolve from a Lv5 purple at reduced cost.

## Named combos

### 1. Death Claw sacrifice → Matt memory + Pagumon On-Deletion mill
- Cards: ST6-15 Death Claw, ST6-14 Matt Ishida, ST6-01 Pagumon (as a digivolution source), an opp Lv4-or-lower Digimon.
- Expected mechanical outcome: On your turn, play Death Claw [Main] → choose to delete YOUR
  Pagumon-carrying Digimon → delete the opp Lv4-or-lower Digimon. Chain: your field −1; opp
  field −1; Pagumon INH [On Deletion] trashes top 2 of your deck (your deck −2, trash grows);
  Matt's [Your Turn] optional "one of your Digimon deleted → suspend Matt to gain 1 memory"
  is offered → accept → memory +1, Matt suspended.
- Rules/keyword basis: On-Deletion handlers fire **post-trash** (CLAUDE.md rule 25;
  `general_rule.pdf` deletion lifecycle). Death Claw text gates target to opp Lv4-or-lower.
  DCGO C# `$BASE_DCGO/Assets/Scripts/CardEffect/ST6/Purple/ST6_15.cs`,
  `.../ST6_14.cs`, `.../ST6_01.cs`.
- Rank: signature engine. **#[test]** (happy).

### 2. Death Claw is gated to opp Lv4-or-lower
- Cards: ST6-15 Death Claw, an opp Lv4 + an opp Lv5 Digimon.
- Expected outcome: after sacrificing, the opponent-target selection offers ONLY the Lv4
  Digimon (one valid action id), never the Lv5.
- Rules/keyword basis: Death Claw `level_lte: 4` filter on the opponent select.
  DCGO `.../ST6_15.cs`.
- Rank: gate/unhappy. **#[test]**.

### 3. Fill trash → Nail Bone double recursion
- Cards: ST6-16 Nail Bone, a purple Lv3 Digimon + a purple Lv4 Digimon in trash (plus filler).
- Expected outcome: Nail Bone [Main] plays BOTH from trash to your field for free (your field
  +2, your trash −2); their [On Play] effects are suppressed.
- Rules/keyword basis: Nail Bone two `play_from_trash_free { suppress_on_play: true }` steps.
  DCGO `.../ST6_16.cs`.
- Rank: payoff. **#[test]**.

### 4. Trash threshold → WereGarurumon inherited +2000
- Cards: ST6-11 WereGarurumon (carrier).
- Expected outcome: with 4 cards in trash on your turn, no buff (effective_dp == base 7000);
  raise to 5 and re-tick declarative effects → +2000 (effective_dp == 9000).
- Rules/keyword basis: INH aura `active_when { your_turn, count_gte trash n:5 }`, `dp_modifier 2000`.
  DCGO `.../ST6_11.cs`.
- Rank: payoff/threshold. **#[test]** (both sides of the threshold).

### 5. CresGarurumon Digi-Burst → play a Lv3 purple from trash
- Cards: ST6-13 CresGarurumon with ≥2 sources, a purple Lv3 Digimon in trash.
- Expected outcome: activate the field [Main] Digi-Burst 2 → trash 2 sources (stack sources −2,
  trash +2) → play the Lv3 from trash free (your field +1, that Lv3 leaves trash).
- Rules/keyword basis: `digi_burst count:2` then `select_trash` + `play_from_trash_free`.
  `general_rule.pdf` §16 Digi-Burst. DCGO `.../ST6_13.cs`.
- Rank: payoff. **#[test]**.

### 6. VenomMyotismon Retaliation trade
- Cards: ST6-12 VenomMyotismon, ≥1 of your Digimon, an opp Digimon.
- Expected outcome: [When Digivolving] choose up to 2 of your Digimon → they gain
  <Retaliation>; assert `has_keyword(Retaliation)`. Then a losing attack into a bigger opp
  Digimon: the chosen Digimon is deleted but takes the attacker with it.
- Rules/keyword basis: `general_rule.pdf` §16 Retaliation (mandatory; fires on Battle-cause
  deletion). DCGO `.../ST6_12.cs`.
- Rank: top-end tech. **#[test]** (keyword grant + a confirming combat trade).

## Playstyle / Win conditions

Mid-range Purple value/control. Attack with Gabumon/Garurumon to draw and self-mill, dump
recurable purple Digimon into the trash, then reuse them for free via Nail Bone and
CresGarurumon Digi-Burst while Matt converts every sacrifice/trade into memory. Death Claw
clears blockers and small threats; VenomMyotismon's Retaliation deters profitable attacks.
WereGarurumon turns a full trash into board dominance. Win by chipping security with the
recurred bodies plus CresGarurumon's Security A.+1.

## Ranked interactions to test

1. Death Claw → Matt + Pagumon mill chain — **#[test]** `death_claw_sacrifice_chains_pagumon_mill_and_matt_memory`.
2. Death Claw Lv4-or-lower gate — **#[test]** `death_claw_only_targets_opponent_level_4_or_lower`.
3. Nail Bone double recursion — **#[test]** `nail_bone_double_recursion_plays_lv3_and_lv4_free`.
4. WereGarurumon trash threshold — **#[test]** `weregarurumon_aura_flips_at_five_card_trash_threshold`.
5. CresGarurumon Digi-Burst recursion — **#[test]** `cresgarurumon_digiburst_plays_lv3_purple_from_trash`.
6. VenomMyotismon Retaliation — **#[test]** `venommyotismon_grants_retaliation_and_trades_in_battle`.

### Blocked / dropped / uncertain

- None dropped for missing cards/primitives. See the test-file header for assertions flagged
  UNCERTAIN (Matt-vs-Pagumon trigger ordering in combo 1, and whether Matt's optional trigger
  surfaces as a pending selection vs. an auto-resolved optional after Death Claw resolves).
