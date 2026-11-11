# League deck audit (2026-11-10)

Five decks from the specialist league, audited two ways:

- **Engine faithfulness:** each deck's key cards were read against the official
  card text and DCGO, and its core line traced through the YAML, the engine and
  the tests. This was read-only: no tests were run and no files were edited.
- **Agent usage:** gen 5 matrix games were re-run with per-card event logging.
  The re-runs are deterministic: the action sequence matched the matrix game
  exactly. 300 games, 68 per audited deck.

## Summary

| Deck | Matrix score | Engine | What the agents do |
|---|---|---|---|
| TS Mervamon | 0.84 | No confirmed bug (the Homeros finding was withdrawn); strong printed cards | Won 85% of 68 games; Homeros re-use is its most frequent effect source |
| Three Musketeers | 0.35 | Core line correct by code but not tested end to end; Der Blitz bug | BeelStarmon BT25-085 played in 44% of games; P-180 mostly loaded under Digimon (39 of 100), its delete fired 68 times; free use from hand 11, paid 4 |
| Examon | 0.53 | All DNA lines work; Coredramon reactive digivolve offers illegal picks | DNA rarely: 0.16 per game; Examon was reached by DNA 11 times and by normal digivolve 38 times |
| Virus Busters | 0.49 | DNA works; MetalGarurumon and WarGreymon Counter bugs | DNA rarely: 0.12 per game, all at low levels; Omnimon/Proximamon reached 26 times, never by DNA |
| DATA SQUAD Ravemon ("Rose Rave") | 0.31 | Hand-trash chain correct; no confirmed bug | Tamers that fuel the deck land in 24-59% of games (Keenan Crier 24%) |

The opponent chooses which card they discard to hand-trash effects, in our engine
and in DCGO (printed "they trash 1 card in their hand").

## Confirmed bugs (fix list, user-confirmed 2026-11-10)

1. **MetalGarurumon (EX12-035)**: "Trash any 4 digivolution cards ... Then,
   return 1 ..." uses `min: 4` without `clamp_to_available`
   (`EX12-035.yaml:106`). With fewer than 4 sources the bottom-deck never
   happens; DCGO trashes up to 4 and always offers it (`EX12_035.cs:142-167`).
2. **WarGreymon (EX12-017) [Counter]**: the attack redirect only runs after a
   DNA (`EX12-017.yaml:157-170`); DCGO offers it either way. Its hand and pair
   picks also offer cards that can't DNA (DCGO filters with `CanPlayJogress`).

## Withdrawn findings

- **Homeros (BT24-102)**: the clause's `condition` does check for *any*
  unsuspended Homeros, but the engine also derives a cost guard from the
  leading `suspend: { target: source }` (`lower_triggered.rs:1153-1158`,
  checked at `:1340`), so a suspended Homeros is never offered the effect.
  With two Homeros and one upright, only the upright one gets a use, which is
  legal (user ruling). The redundant `condition` could be tightened to
  `source_is_unsuspended: true` for clarity; it changes no behaviour. Verified
  by reading the code, not by a test.
- **Lilamon (ST24-10)**: not a bug. The Tamer cards can be trashed even when no
  digivolution follows (user ruling).

3. **Der Blitz (EX7-070)** (user-confirmed): "While you have a [Three
   Musketeers] trait Digimon, you may ignore this card's color requirements" is
   lowered as a `flood_gate` (`EX7-070.yaml:153`) that may never reach an Option
   in hand; P-180 and EX7-071 use `use_requirement`. Low impact in practice: the
   deck mostly loads Der Blitz under a Digimon for its trash trigger.

4. **Coredramon (EX13-018, EX13-039) reactive digivolve** (user-confirmed):
   "[Your Turn] When any of your other Digimon with [Dracomon] or [Examon] in
   their texts are played, this Digimon may digivolve into a Digimon card with
   [Examon] in its text in the hand with the cost reduced by 2." Our hand pick
   filters only `kind: digimon` + `in_text_contains: Examon`, so cards it can't
   digivolve into are offered and the pick does nothing. DCGO
   (`EX13_018.cs:98-101`) requires `CanPlayCardTargetFrame` onto this Digimon,
   and its `CanActivateCondition` doesn't offer the effect at all without a
   legal card in hand. Fix: add `can_digivolve_onto: source` (as AD1-011 does)
   to both cards. The [On Play]/[When Digivolving] halves are correct.

## How 3M uses its Options (68 games)

Tracked by watching each Option leave the 3M hand (the event log has no
"Option used" event). Each time an Option left the hand, the step's memory change
and effect source say what took it:

| Option | Left hand | Placed under a Digimon (LadyDevimon, Sparrowmon, Scopemon, Satellamon) | Used free from hand (BeelStarmon BT25-085 / EX7-073) | Paid 6 memory | Hand cost / other |
|---|---|---|---|---|---|
| Bind Red Trigger (P-180) | 100 | 39 | 11 | 4 | 46 |
| Der Blitz (EX7-070) | 37 | 11 | 10 | 3 | 13 |
| Hurricane Screw Shot (EX7-071) | 82 | 27 | 7 | 15 | 33 |

- P-180's "trashed from digivolution cards → delete 7000 DP or lower" fired 68
  times, so loading and trashing it is the deck's main payoff, as intended.
- BeelStarmon's free use *from its own digivolution cards* isn't visible this
  way; only uses from hand are counted.
- Some "hand cost / other" rows are Asuna Shiroki (BT24-088/BT25-092) trashing
  an Option as a cost (9 of P-180's 46); most are hand picks the log can't
  attribute to a card.
- The waste is Hurricane Screw Shot: paid 6 memory 15 times.

## Suspicions (need a ruling or a test)

- **Aegiochusmon: Holy (BT26-029)**: we also block return-to-hand and
  de-digivolve; DCGO blocks DP reduction, stack trashing and return to deck
  only. Wording ("them") is arguable. Weakens bounce answers to TS Mervamon.
- **Coronamon (BT25-008)**: trashes and draws one at a time; DCGO picks both
  cards first. Small information advantage.
- **Dianamon (BT25-028)**: YAML colour blue/white; official bundle says
  blue/purple (missing purple digivolve path).
- **Ravemon (BT26-082)**: security placement doesn't check the card is still in
  the trash. Harmless today.
- **Free Option use skips colour entirely** (`options.rs:416`): matches DCGO,
  but rule 9-1-1 and rulings Q1500/Q2785 say colour still applies to effect
  uses. Affects 3M when no 3M-trait Digimon is out.
- **Asuna Shiroki (BT24-088), BeelStarmon (X Antibody) (EX7-073)**:
  "[Three Musketeers] in its text" checks effect text only; no current impact.

## Missing tests worth adding

- 3M: the real P-180 used from BeelStarmon's stack, then trashed by its unsuspend
  clause to delete a Digimon; P-180 used from hand and placed under BeelStarmon.
  No Three Musketeers archetype combo suite exists.
- Examon: the Groundramon + Wingdramon (as Lv.6) DNA mask bit; BT20-093 Delay
  with the real EX13-045.
- Virus Busters: WarGreymon Counter DNA into Omnimon during an attack;
  Proximamon DNA; wrong-colour rejection per DNA target.
- DATA SQUAD Ravemon: full Crowmon -> Ravemon -> security replay line.
- TS Mervamon: two Homeros on the field, one suspended (pins the cost guard).

## What it means for training

- **TS Mervamon's strength is printed** (free plays, Blocker/Reboot on every [Iliad] Digimon, Homeros re-use).
  Opponents' bounce is weaker than it should be if the Aegiochusmon reading is
  wrong.
- **The DNA decks rarely play DNA.** The engine supports every line and exposes
  it in the action space, but Examon is DNA-digivolved only 11 of 49 times and
  Virus Busters never DNAs into Omnimon/Proximamon. That is a policy/search gap, not engine
  support.
- **Rose Rave under-deploys its Tamers**, which are the deck's engine.
- **3M does make its key play** fairly often, so its weakness is more likely
  sequencing (re-using P-180 every turn, ordering triggers) or the Der Blitz bug
  than not finding the line.

Raw data: `D:/ex13-data/specialist-league/matrix/metrics-gen5/` (re-run games
with `card_metrics`); usage script in the session scratchpad (`usage.py`).

---

# Part 2: the other 13 league decks (2026-11-11)

Same method as part 1:
- **Engine faithfulness:** four read-only agents compared each deck's key cards
  against the official text, DCGO and the rules manual.
- **Agent usage:** 312 more gen 5 matchups were re-run with card logging, giving
  68 games per deck.

The engine findings below are the agents' reports, with their file and line
citations. I re-checked a sample. None of them is pinned by a test yet.

## Fixed during this audit (both parts)

| Bug | Effect on decks | Fix |
|---|---|---|
| DUAL cards checked from security did not battle (user report) | Weakened every deck with DUAL cards in security (Virus Busters, Three Musketeers, Glowing Dawn, TS decks) | `combat/mod.rs` battles a Digimon *or DUAL* security card using its Digimon-face DP. Test: `tests/security_battle_winner.rs` `checked_dual_card_battles_as_a_security_digimon` |
| MetalGarurumon (EX12-035) "trash any 4" with fewer sources | Virus Busters weaker | `clamp_to_available: true` |
| WarGreymon (EX12-017) [Counter] | Virus Busters weaker | Redirect offered without a DNA; OPT refunded if nothing happened; hand and pair picks offer only legal DNAs (new `dna_field_pair` predicate; the `select_dna_pair` left filter now sees bindings) |
| Der Blitz (EX7-070) colour bypass | Three Musketeers weaker | top-level `use_requirement` |
| Coredramon (EX13-018, EX13-039) reactive digivolve | Examon: dead choices | `can_digivolve_onto: source` |
| Marcus Damon & Thomas H. Norstein (ST24-13) missing name aliases | ShineGreymon Data Squad much weaker | `also_treated_as: [Marcus Damon, Thomas H. Norstein]`. ST23-13/14 print no such rule (the agent's claim was wrong) |
| Destromon (BT21-060) inherited "return 2 [Vemmon], end that attack" forced | Venmon weaker | `optional: true` (DCGO `SetUpActivateClass(..., true, ...)`). Its Vemmon count already reads only this Digimon's sources (the agent's second claim was wrong) |
| `<Delay>` usable the turn it was placed (would-leave family) | Toho Braves, Medusamon slightly stronger | `lower_replacement.rs`: a replacement that opens by trashing its own source is a `<Delay>` cost and is not offered the turn the Option was placed (CR 16-17-3). Covers EX12-070, BT19-099, BT17-095, BT20-093, BT20-100, ST20-14; the other 66 Delay cards already checked it |
| `<Use Req.>` counted Options on the field and some ignored breeding | SW, Toho Braves slightly stronger; Glowing Dawn, TS, DATA SQUAD weaker | The 31 YAMLs for DCGO's `CardEffectFactory.UseRequirements` cards now require your Digimon or Tamer in the battle area or breeding area. P-236 needed a [Glowing Dawn] Digimon/Tamer but was unconditional. P-206 really is unconditional (it prints "You can ignore this card's color requirements"). Mechanics test: `tests/dsl/use_req_kind_and_breeding.rs` |
| e-Pulse (ST23-15) from security went to the trash | Glowing Dawn weaker | Its [Security] now ends with `place_self_as_delay_option` (DCGO runs the whole [Main], ending in PlaceDelayOptionCards) |
| Raging Serpentine (BT21-093) Delay digivolve ignored requirements | Medusamon stronger | `ignore_requirements: false`, and the hand pick offers only cards that can digivolve onto the chosen Digimon |
| Medusamon (EX11-012) Token needed a deletable target; return forced | Medusamon weaker | The clause is mandatory with optional picks (DCGO): it is offered when there's a deletable target or a card in the opponent's trash, the delete and the return can each be declined, and the Token comes only with a return |
| Kaguyamon (EX12-065) could not revive Tamers | Toho Braves weaker | Filter is Digimon or Tamer ("card"; DCGO checks HasPlayCost) |
| Erlangmon (EX12-034) offered empty zones; OPT spent on nothing | SW weaker | Only zones with a playable card, plus "Don't play"; `refund_opt` when nothing played. Engine: `refund_opt` was a silent no-op in every replacement except the plural-prevention shape (`lower_replacement.rs`), so EX13-024's refunds were broken too |
| MachGaogamon (BT25-027) face-down trash forced | ShineGreymon Data Squad weaker | `optional: true` on the Tamer pick |
| Amaterasumon (EX12-047) "by returning 2" forced | Toho Braves (minor) | `optional_zero: true` |
| Galacticmon (EX11-046) with no-play-cost Digimon | Venmon weaker (edge) | Engine: play-cost selectors skip Tokens/DUAL cards (DCGO IsMaxCost); YAML: when none can be kept, all are deleted |
| Snatchmon (BT18-065) dead end-of-turn choice | Venmon (minor) | `can_digivolve_onto: source` |
| **DUAL cards played to the field** (game audit) | **TS Gear Forest, TS Mervamon, Glowing Dawn much stronger** | A DUAL card prints "Can't play to the field." The central play gate refuses it, a DUAL read as a Digimon has no play cost, and "play or use" uses it as an Option. Mechanics tests: `tests/dsl/play_and_attack_rule_guards.rs` |
| Effect attacks on the opponent's turn (game audit) | SW (1 game in 612) | `can_attack*` require the turn player (CR 11-1-2) |
| Free plays of DigiXros cards charged memory (game audit) | SW weaker | A `Free` play costs 0 even when a DigiXros transaction is pending (`game_actions/misc.rs`) |

## Summary

| Deck | Score | Win % (68 games) | Engine | Agents |
|---|---|---|---|---|
| Toho Braves | 0.77 | 84% | Same-turn `<Delay>` (stronger, but rare); Use Req counts Options (stronger, low); Kaguyamon can't revive Tamers (weaker) | Free Sanmyojin plays: 34 later own turn, 23 opponent's turn, only 3 on the turn Arrival was placed |
| TS Gear Forest | 0.73 | 72% | Clean | Ceresmon, Homeros, Deramon drive it |
| Dynasmon | 0.66 | 66% | Clean | Mistymon/Dynasmon effects most games |
| SW | 0.61 | 66% | Use Req counts placed Options (stronger, marginal); Erlangmon dead choices + no OPT refund (weaker) | Erlangmon and Sagomon top effect sources |
| TS Jupitermon | 0.59 | 59% | Clean | Inori Misono and Aegiochusmon: Holy |
| Chronomon | 0.58 | 57% | Clean | Holy Mode, Toya Kuga, Shota Kuroi chain |
| Glowing Dawn | 0.48 | 54% | e-Pulse Use Req ignores breeding (weaker); e-Pulse from security not placed (weaker); P-236 unconditional bypass (stronger, low) | DUAL Atratusmon a top effect source; also hit by the DUAL security bug |
| Chronicle | 0.43 | 35% | Clean | Alphamon effects in most games it lands |
| Medusamon | 0.40 | 41% | Raging Serpentine (BT21-093) ignores digivolve requirements + same-turn Delay (stronger); Medusamon (EX11-012) token needs a deletable target (weaker) | Gigimon, Medusamon, Owen |
| ShineGreymon Data Squad | 0.40 | 38% | **Marcus & Thomas (ST24-13) missing its name aliases** (much weaker); MachGaogamon forced trash (weaker) | ST24-13 still the top effect source |
| ADVENTURE | 0.35 | 28% | **About 20 cards have no card spec and play blank** | Plays the blank starter Digimon most games |
| Light Hybrid | 0.28 | 28% | Clean; one list only | Beowolfmon |
| Venmon | 0.21 | 26% | Destromon's inherited "end that attack" forced (weaker); Galacticmon keep-one with no play-cost Digimon (weaker, edge); Snatchmon dead choice | Galacticmon reached in 75% of games and still loses |

## Confirmed bugs, by impact

**Large (change how a deck plays):**
1. **ADVENTURE: about 20 cards blank.** ST20-02 Biyomon, ST20-03 Birdramon,
   ST20-05, ST20-06 Angewomon, ST21-01 Tsunomon, ST21-07 Palmon, ST21-08
   Togemon, ST21-09 Lillymon, ST21-10 Gabumon, ST21-14, AD1-022 Izzy & Tai,
   BT21-061 MetalGreymon, BT21-067 Garurumon, BT21-075 and others have only
   metadata. `data/card_admissibility.json` marks all of them inadmissible
   (`untested_lowering`). **The league's deck pool was not filtered by the
   training gate**, and ADVENTURE is the only deck it would have rejected.
   This deck's results say nothing about the agents.
2. **Marcus Damon & Thomas H. Norstein (ST24-13)** has no `also_treated_as`
   names (DCGO `ST24_13.cs:17-27` adds both). That breaks:
   - Gaogamon (BT25-023)'s Thomas search;
   - ShineGreymon: Burst Mode (BT25-104)'s "return 1 [Marcus Damon]" and
     "Marcus = 12000 DP with Rush";
   - ShineGreymon (AD1-016)'s free play of ST24-13.

   ST23-13 and ST23-14 have the same omission, and `st24_13.rs:12` documents
   it as intended.
3. **Destromon (BT21-060)'s inherited "by returning 2 [Vemmon] ... end that
   attack" is forced.** DCGO `BT21_060.cs:221` makes it optional. An opponent
   can attack with anything to strip the Vemmon that keep Galacticmon alive.
   Its condition also counts Vemmon on all your Digimon, not this one (`:251`).
4. **`<Delay>` can fire on the turn it was placed** through the play-from-hand
   delay path. `lower_replacement.rs:154-157` checks `source_delay_is_ready`
   only for DNA; the rule is CR 16-17-3, and DCGO checks it in
   `OptionEffect.cs:27-37`. `bt19_099.rs:166-172` pins the wrong behaviour.
   - Affects Sanmyojin Arrival (EX12-070), Raging Serpentine (BT21-093),
     BT19-099 and other Delay cards.
   - Measured impact in Toho Braves: at most 3 plays in 68 games.

**Smaller:**
- **`<Use Req.>` counts Option cards on the field.** CR 16-42-3, and DCGO
  `CardEffectFactory.cs:718`, require a Digimon or Tamer. About 86 YAMLs use
  `use_requirement`, so a central kind guard fixes them all. This affects
  Options in SW and Toho Braves, BT26-033, and the Option sides of DUAL cards.
- **Use Req ignores the breeding area** in about 28 YAMLs. e-Pulse (ST23-15)
  is the one that matters, since DCGO checks breeding
  (`CardEffectFactory.cs:706-726`).
- **e-Pulse (ST23-15) from security** isn't placed on the field (DCGO
  `ST23_15.cs:98,106`).
- **Raging Serpentine (BT21-093)**'s Delay digivolve has
  `ignore_requirements: true` (`:215`); DCGO keeps the requirements.
- **Medusamon (EX11-012)**'s Petrification token requires a deletable target
  (`:23`); DCGO only needs cards in the opponent's trash. Its trash pick should
  also be declinable.
- **Kaguyamon (EX12-065)**'s revive adds `kind: digimon`, but the card prints
  "card" and DCGO allows Tamers (Kunlun BT26-104).
- **Erlangmon (EX12-034)** always offers Hand/Sources and never refunds its OPT
  when nothing is played (DCGO `EX12_034.cs:158-172`, `RemoveUse`).
- **MachGaogamon (BT25-027)**'s face-down trash should be optional (DCGO
  `:100`).
- **Galacticmon (EX11-046)**'s "keep the highest play cost" ignores cards
  without a play cost (`selections.rs:162`). With only tokens or DUAL cards,
  all of them should be deleted (DCGO and the official Q&A).
- **P-236 / P-206** have an unconditional Use Req; DCGO needs a [Glowing Dawn]
  Digimon or Tamer.
- **Amaterasumon (EX12-047)**'s "by returning 2" can't be declined
  (`optional_zero: false`; DCGO `canNoSelect: true`).
- **Snatchmon (BT18-065)**'s end-of-turn digivolve offers illegal targets.

## Suspicions

- **DUAL Option-side [Security]:** "Option [Security] effects don't activate"
  doesn't block it (`effect_queue.rs:156` checks for exactly `Option`).
- **Ceresmon (BT26-032)** applies -5000 to Digimon suspended later. DCGO
  agrees; it needs a ruling on "all of X get".
- **Takutoumon (EX12-057):** the field [Counter] window, and the
  one-Counter-per-attack limit.
- **Medusamon:** BT21-029's OPT is split over two clauses; BT24-017's +2000 sits
  under the "returned 2" branch.
- **Alphamon (EX13-060)**'s end-of-turn play doesn't check affordability after
  the -6.
- **ADVENTURE cards:**
  - Matt & T.K. (AD1-019)'s hand pick is unfiltered.
  - Omnimon (AD1-025)'s Assembly uses `name_contains`.
  - MetalGarurumon (AD1-014) is missing purple.
- **Other DNA-from-hand cards** (AD1-009, AD1-012, BT20-016, BT20-093,
  BT24-035, EX13-070, EX9-013) can now use `dna_field_pair` to stop offering
  illegal picks.

## What it means for training

- **The league's deck pool needs the training gate.** ADVENTURE should not have
  been in it.
- **Several weak decks are weak partly because of the engine:**
  - ShineGreymon Data Squad: the ST24-13 aliases.
  - Venmon: Destromon.
  - ADVENTURE: the blank cards.
  - Glowing Dawn and Virus Busters: DUAL security.
  - Three Musketeers: Der Blitz and DUAL security.

  The fixes should move these up before the next matrix.
- **The strong decks are mostly clean.** Toho Braves, TS Gear Forest, Dynasmon
  and Chronomon have no confirmed bug that inflates them enough to matter.
  Their strength in our games comes from what they print plus how easy they
  are for the agents to pilot.

# Part 3: game audit (2026-11-11)

An agent scanned the 612 gen 5 matrix games (`D:/ex13-data/specialist-league/matrix/metrics-gen5*`)
for rule violations and confirmed three against the card faces, the rules
manual and DCGO. All three are fixed (table above).

1. **DUAL cards played to the field: 149 times in 109 games (18%).** Every DUAL
   card prints "Can't play to the field." (e.g. Ceresmon BT26-032), and CR 4-6-2
   lets a DUAL only be digivolved or used.
   - Sirenmon (BT25-039) played Ceresmon 43 times, often on turns 2-4, which
     gave a free Lv.6 13000 DP body.
   - Minervamon (BT24-041) played Jupitermon (BT26-033) 15 times, because it
     read the use cost as a play cost.
   - e-Pulse and the Murasamemons played Monarchlizamon, Atratusmon and
     Murasamemon.
   - Plays by deck: TS Gear Forest 54, TS Mervamon 39, Glowing Dawn 28, Virus
     Busters 10.
   - The two top league decks were inflated by this. Their win rates
     (0.73 / 0.84) are not deck strength.
   - **Related, not fixed:** after such a play, the Option [Main] also ran as an
     [On Play] (`lower_triggered.rs:1549` builds Option bodies as
     `Effect::on_play(card).option_main()`, which leaves `on_play` set). With
     DUAL plays refused, this path should no longer be reachable. It is worth a
     guard test.
2. **A Digimon attacked on the opponent's turn** (1 game). Gokuumon's [On Play]
   was resolved during the opponent's turn, and it let Sanzomon attack.
3. **Free plays of DigiXros cards charged memory** (8 plays, all SW). Erlangmon
   "without paying the cost" charged Sagomon 5 and Cho-Hakkaimon 7.

Checked and clean:
- same-turn attacks (every one had Rush or an attack effect);
- hand-play costs;
- digivolve level, colour and cost;
- security check counts;
- memory range;
- breeding and hatching.

Not checked: Once Per Turn overuse and mandatory effects that never fire. The
logs carry no clause ids, so these can't be checked from them.

# Part 4: suspicions checked (2026-11-11)

Each suspicion above was checked against the card face or official bundle, CR
4.3 and DCGO. This branch was merged with main first, so the ADVENTURE cards
are the current ones from PR #756.

**Fixed:**

| Card / area | Finding | Fix |
|---|---|---|
| Medusamon (BT21-029) | One printed [Once Per Turn] effect with two triggers was split into two clauses with separate counters, so it could play 2 Tokens a turn. Official Q1550 and CR 15-14-1: it is one effect whichever trigger fires. DCGO also plays 2; the manual governs. | New DSL clause key `opt_group`: clauses with the same label share one counter. Both arms use `opt_group: petrify`. |
| Alphamon (EX13-060) | The [End of Your Turn] play offered cards that can't be paid after -6 (memory would pass -10), and spent the [Once Per Turn] on nothing. | `affordable_with_cost_reduce: 6` on the pick (DCGO `CanPlayAsNewPermanent(fixedCost: cost - 6)`; CR 1-3-11-1). |
| Omnimon (AD1-025) | Assembly and Partition matched names by "contains", so BlackWarGreymon and the (X Antibody) cards qualified. A bracketed name alone is the exact name (CR 2-3-1-2; DCGO `EqualsCardName`). | `name_is` (the DNA line keeps "in name"). |
| DNA from hand: AD1-009, AD1-012, BT20-016, BT24-035, EX13-070, EX9-013 | Offered hand cards and field pairs that can't DNA digivolve (DCGO `CanPlayJogress(true)`). BT24-035 hard-coded one Silphymon's colours. AD1-009 and BT20-016 hand picks could not be declined. | `dna_field_pair` filters plus `binding_present` guards (WarGreymon EX12-017 pattern). BT20-093 already filters through its Delay flow. |
| EX9-013 [End of Your Turn] | Required an Omnimon Alter-S in hand and 2 Digimon, which skipped "Then, 1 of your Digimon may attack". | Condition is now only this Digimon on the field (DCGO). |
| Erlangmon (EX12-034) | Choosing "Hand"/"Sources" and then passing spent the [Once Per Turn] without the refund. | The pick inside a zone is mandatory; "Don't play" is the opt-out and refunds (same outcomes as DCGO). |
| Option [Main] bodies | Built with the `on_play` flag, so a played Option-face card also ran its [Main] as an [On Play]. | Built without it; a test checks every lowered card. |
| Takutoumon (EX12-057) | Attribute was Data; the card face reads Vaccine. | YAML and `card_overrides.json`. |
| MetalGarurumon (AD1-014) | YAML header colour missed purple (runtime data was right). | `color: [blue, purple]`. |

**Not bugs:**
- Ceresmon (BT26-032): "all of your opponent's suspended Digimon get -5000" is
  continuous overall processing (CR 15-11-2-2/3), so later-suspended Digimon
  are affected. DCGO agrees.
- Takutoumon (EX12-057) [Counter]: offered on the field to the defender, one
  [Counter] per attack across all sources (CR 11-3-2).
- BT24-017's +2000 under the "returned 2" branch matches official Q5591.
- Matt & T.K. (AD1-019)'s hand pick is filtered by [ADVENTURE].
- Tomoro & Kyo (ST23-13) and Reina & Makoto (ST23-14) print no name rule
  (Part 2 was wrong to list them with ST24-13).

**Needs a ruling:** a DUAL card checked from security against "doesn't
activate [Security] effects on Option cards" (BT20-015 and others). The only
DUAL card in the pool with a [Security] effect, ScourgeChiropmon (BT26-075),
prints it above the trait line, which is Digimon information (CR 4-6-5-3:
Digimon effects). DCGO blocks it because a DUAL card `IsOption`. Unchanged.

**Open, not fixed:**
- BT12-112's "All of your opponent's [Security] effects on Option cards don't
  activate" puts the modifier on Option permanents, so it never applies. It
  needs a player-level modifier that checks the checked card's kind.
- BT17-014, BT20-071, BT20-074, BT7-014, ST13-05 and BT3-097 have no YAML.
- The `digimon-dsl` crate's own test targets don't build on main (the
  implementation inventory lags `exclude_self`, `DigiEgg`,
  `SecurityTrashOne/Finalize` and `source_card_trash_protection`).
- Two `determinization` fixture replays fail
  (`closed_cloud::queued_and_optional_security_observers_*`,
  `closed_cloud::link_selection_continuations_*`). They replay recorded SW
  (Gokuumon EX12-015) and TS-link games whose plays are no longer legal after
  these fixes, and need re-recording. No generator is committed
  (26bfc3f79). This suite is in the PR gate.
- `test_alt_path_authoring_parity` fails the same way on main: Ceresmon
  (BT26-032) and Bacchusmon (BT26-080) have no alt_path for "[Digivolve]
  Play cost 12 [X]: Cost 2".

# Part 5: Rose Rave (DATA SQUAD Ravemon) games and cards (2026-11-11)

The 68 gen 5 matrix games with per-step records (`metrics-gen5/`) and all 24
cards in the four lists, checked against the card faces, CR 4.3 and DCGO.

**Fixed:**

| Card / area | Finding | Fix |
|---|---|---|
| Rosemon: Burst Mode (BT26-050), ShineGreymon: Burst Mode (BT25-104), BT13-020, BT13-060 | Every `burst_digivolve` alt path got the `<Blast Digivolve>` counter marker, so a printed [Burst Digivolve] (a Main-phase digivolve) was offered free in the counter window on the opponent's turn: 13 times in 68 Rose Rave games, once for ShineGreymon Data Squad. | The marker is emitted only for `marker: true` paths and Blast DNA. |
| Play from any zone | A Digi-Egg or Option could be played by an effect (no play cost; CR 7-1-1, DCGO `HasPlayCost`). Crowmon and Pinamon played DNA Charge and P-235 from trash in the games (put out and trashed at once). | `card_data_play_is_prohibited` refuses Digi-Eggs and Options. |
| Crowmon (BT26-076), Pinamon (BT26-005), Rosemon (BT26-049) | Their play picks offered Options / Digi-Eggs. | Filters exclude them. |
| DNA Charge (ST24-15) | Its [Security] ("activate this card's [Main]") never placed the card: 15 of 15 reveals ended in the trash. | Ends with `place_self_as_delay_option`. |
| Lilamon (ST24-10) | Its inherited "this Digimon would leave" protected any of your permanents; the trash-2 digivolve offered illegal targets (23 of 33 picks digivolved nothing) and refused to pay without a [DATA SQUAD] card in hand. | `is_source: true`; the trash-2 is now its own choice (paying alone is legal, per the earlier ruling), then only cards that can digivolve onto it. |
| Digital Accident Tactics Squad (P-235) | The add had a PASS; the card has no "may". | Mandatory (no candidates still skips). |
| Marcus Damon & Agumon (AD1-021) | The digivolve offered any own Digimon and any yellow Greymon. | `has_digivolve_candidate` / `can_digivolve_onto`; `[Marcus Damon]` matched by exact name. |
| Agumon (ST24-04), Rosemon (ST24-11) | Alt digivolve from "[Koromon]" / "[Lilamon]" matched by "contains". | `name_is` (no gameplay change today). |

**Checked and clean:** Ukkomon (BT16-082), ShineGreymon: Burst Mode
(BT25-104) effects, Rosemon: Burst Mode (BT26-050) effects, Falcomon
(BT26-065), Peckmon (BT26-072), Ravemon (BT26-082), Yoshino Fujieda
(BT26-091), Keenan Crier (BT26-094), Kudamon (EX13-026), Shadow Training
(LM-060), GeoGreymon (ST24-05), Lalamon (ST24-08), Sunflowmon (ST24-09),
Falcomon (ST24-12), Yoshino & Keenan (ST24-14). In the games: Crowmon and
Peckmon deletions, Ravemon targets, Tamer memory gains, Your-Turn timings and
effect digivolve costs were all right.

**Open:**
- The real [Burst Digivolve] is never offered (all 16 Rosemon: Burst Mode
  digivolves paid 5), gap G-BURST-ON-TURN-END-NOT-EXECUTED.
- Hero Tactics (AD1-023) and BT25-035 have the same patterns as Lilamon
  (`kind: digimon` without `is_source`; unchecked digivolve picks). Not checked.
