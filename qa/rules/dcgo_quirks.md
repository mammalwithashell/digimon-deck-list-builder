# DCGO quirks held against the rules

The pre-registered list the engine-rules-conformance design (§6, "No training-pool shrinkage from conformance fixes") asks for: each entry is a family of places where DCGO, the exam oracle, answers differently from the Comprehensive Rules Ver. 4.3 (`docs/digimon-rules/cr-4.3.txt`) or an owner ruling, or differs from our engine only in a rules-neutral way. An exam verdict row that diverges for one of these reasons is triaged `dcgo_quirk` with a citation naming the rule and this file's entry id, so re-triage after a conformance fix is a lookup.

Each entry: the id, what DCGO does (with the C# location), what the rules say, how the exam shows it, and the verdict rows that cite it. A row is cited here only after its divergence was re-diffed against the recorded DCGO sidecar and found to be this family and nothing else (no downstream rows).

Older families documented elsewhere and not repeated here: `docs/DCGO_EXAM.md` "Rows where one side looks rules-wrong" (the `UnionZone` zone menu among others; the RevealBucket bullet there is restated below with a 4.3 citation), and the held quirks in `qa/rules/mechanics-gaps-2026-10.md` "Residuals left open" (`<Piercing>` not pooled; a win trigger not offered with no card to play; Q3551) and owner sign-off item 18 (DCGO raises one trash event per Tamer for a multi-card cost).

## Q-REVEAL-BUCKET-ADD-TIMING: one "Add X and Y" is performed per bucket

- **DCGO:** `SimplifiedRevealDeckTopCardsAndSelect` → `RevealDeckTopCardsAndSelect` (`Assets/Scripts/Script/CardEffectCommons/RevealLibrary.cs:291`) runs one `SelectCardEffect` per condition, and each one in `Mode.AddHand` moves its pick to the hand when that prompt closes. The state row at the second pick already shows the first pick in hand.
- **Rules:** the printed text is ONE add instruction with two targets ("Add 1 [Iliad] trait card and 1 [TS] trait card among them to the hand"). §15-1-2 processes a single effect in text order, and §3-1-3-3 moves cards that leave an area together at the same time. So both targets are chosen and then both cards move. (`docs/DCGO_EXAM.md` cites §15-15-10-1 for this; that section is 3.6 numbering and was deleted in 4.0, see `docs/digimon-rules/changelog-3.6-to-4.3.md`.)
- **Outcome:** neutral. No triggered effect or rule check runs inside the resolving effect (§15-8-3-2, §17-1-2-2), the second bucket's candidates do not read the hand, and the end state matches.
- **Exam signature:** one `p0.hand` field differs on the second bucket-pick row (DCGO holds the first pick, we do not) and there are no downstream rows.
- **Rows (2026-10-21):** BT24-031#effect#1, BT24-031#inherited#0, BT24-034#effect#0, BT24-101#effect#4, BT25-022#effect#1, BT25-022#inherited#0, BT25-032#effect#1, BT25-046#effect#1 (the last six through an Elecmon BT24-031 / Lunamon BT25-022 / Liollmon BT25-032 / Gekkomon BT25-046 reveal in the line). Earlier: BT26-065#effect#1, EX7-008#effect#1, BT7-056.

## Q-REVEAL-ADD-THEN-PLACE: "add 1 to the hand and place 1 under a Tamer" (rules-neutral)

- **DCGO:** Gekkomon ST23-06 (`ST23_06.cs`) uses the same per-condition loop with an `AddHand` bucket followed by a custom "place under a Tamer" bucket, so the hand pick is in the hand before the place prompt opens.
- **Ours:** both picks are made, then the card is added and the other placed.
- **Rules:** this text has two verbs ("add ... and place ..."), so the §3-1-3-3 reading behind Q-REVEAL-BUCKET-ADD-TIMING does not apply as directly, and DCGO's order is the literal §15-1-2 text order. The difference is still not observable: nothing triggers or rule-checks inside the resolving effect (§15-8-3-2, §17-1-2-2), the place prompt's candidates (the remaining revealed [Glowing Dawn] cards) are the same either way, and the end state matches. Triaged `dcgo_quirk` because the verdict store's `DcgoQuirk` class covers a rules-neutral difference.
- **Owner question:** for a reveal effect with two different verbs, should the engine move the first card before the second pick, to match the text order exactly? If yes, this entry becomes an engine fix and the rows below become `ours_wrong`.
- **Exam signature:** one `p0.hand` field on the place-pick row, no downstream rows.
- **Rows (2026-10-21):** ST23-06#effect#1, ST23-06#effect#2. The same row is the only compared divergence in the prefix of BT25-035#effect#1, BT25-049#effect#2 and ST23-03#effect#2, which were triaged `stale` for other reasons.

## Q-SELECT-INDEX-NULL-ON-AI-PATH: an effect read back through selected indices does nothing on the AI/harness path

- **DCGO:** `SelectCardEffect.SetTargetCardAndIndicies` receives `Indicies = null` from the AI's `AutoSelect` (`Assets/Scripts/Script/SelectCardEffect.cs:684`) and from the exam harness hook (`:982`, "Indicies stays null, exactly as the AI's AutoSelect passes it"), so `_slectedInexesInList` stays empty. A card that reads its choice only through `SetUpAfterSelectIndexCoroutine` gets an empty list. Homeros BT24-102 (`BT24_102.cs:209-230`, "Select 1 effect to activate.") then sets no `selectedEffect` and re-activates nothing. Only the human-UI path (`:601-610`) passes indices.
- **Rules:** BT24-102's [End of Your Turn] activates the chosen [On Play] / [When Digivolving] effect; the official Q&A assumes it does (Q6945; Q5721 and Q6029 rule on which effects it may choose).
- **Exam signature:** every row matches until the end state; DCGO's end state lacks everything the re-activated effect does.
- **Rows (2026-10-21):** BT24-102#effect#1 (Jupitermon BT24-101's [On Play] re-activated: ours trashes a second security card, DCGO does not).
- **Other cards:** any DCGO script that reads its choice through `SetUpAfterSelectIndexCoroutine` is affected the same way under the harness. Not audited.

## Q-IN-TEXT-BATTLE-TRIGGER-MISSED: EX13-041's "[Dracomon] or [Examon] in their texts" battle trigger does not fire for an effect-text match

- **DCGO:** Groundramon EX13-041's inherited "[All Turns] [Once Per Turn] When any of your Digimon with [Dracomon] or [Examon] in their texts delete your opponent's Digimon in battle, trash their top security card" (`EX13_041.cs:182-213`, `WinnerRealCondition` on `permanent.TopCard.HasText`) did not trash a security card when Slayerdramon EX3-024 won the battle. EX3-024 names [Examon] only in its effect text. DCGO asked no trigger-order prompt either (only BT20-044's own trigger pended). Its `HasText` does scan effect text (`CardSource.cs:2149`), so the miss is somewhere in the trigger path. Not traced further.
- **Rules:** official Q&A Q7335 ("X in its text" includes effects) and Q7336 ("[AA] or [BB] in its text" = either). Ours fires and trashes the card, and agrees with both rulings (expect_ruling 6/6).
- **Exam signature:** one LEAD row at the trailing step: `p1.security` ours 4 / DCGO 5 and `p1.trash` one card longer in ours.
- **Rows (2026-10-21):** EX13-041 `qa:Q7335`, `qa:Q7336`. (`qa:Q7339` on the same line is `stale`: the line does not exercise that ruling.)

## Q-DCGO-TAMER-PHANTOM-DIGIVOLVE: the DCGO AI digivolves onto a target the card cannot digivolve onto (a level-less Tamer, or a Digimon that fails every requirement) at cost 0

- **DCGO:** in a bot-vs-bot recording (`20261005T144037Z_0b812114...`, BT26-089 Kyo Sawashiro on the board beside BT26-025) the AI plays BT25-049 from hand as a digivolve onto the Tamer's frame (action 401). The row's `action_detail` reads `alt_path: cost_modifier`, `cost_paid: 0`: with no level to match, `CostList` is empty, `baseCost` stays at its initial `-1`, `GetPayingCostWithBaseCost` clamps it up to 0, and `DetermineAltPath` labels the difference `cost_modifier`. No DCGO effect reduces BT25-049's printed digivolution cost of 2.
- **Rules:** a Tamer has no level and no digivolution requirement is met; it is not a legal digivolution target (CR 4.3 §6-5 digivolution requirements). Our engine has no such move, so the replay driver, finding the remapped id illegal, falls back to the recorded id, which is the same hand card onto the real Digimon (BT26-025) at the printed cost 2.
- **Replay signature:** `memory_mismatch` at the next step, recorded 8 vs engine 6 (DCGO charged 0, we charged 2). Not fixable on our side without implementing an illegal move.
- **Triage (2026-11-08):** `dcgo-replay` recognises this signature in `code/tools/dcgo-replay/src/triage.rs` and reports the game under `quirk` (with this id and the matched rows), NOT under `pass` and NOT under `fail`. The match requires ALL of: a `memory_mismatch`; a digivolve-range action whose `action_detail` has `cost_paid == 0` and `alt_path == cost_modifier`; a played card whose printed digivolution cost is > 0; exactly one card leaving the actor's board across that action, and it is a Tamer; and the mismatch's recorded memory equal to that `action_detail`'s `memory_after`. A mismatch that merely sits near a quirk stays a `fail`. Unit tests cover each element failing in turn.
- **Rows (2026-11-07):** 1 of 176 sampled corpus games. Other BT26-089 / BT25-032 games in the sample replay clean once the operand-order fallback applies.

### Generalisation (2026-11-08, full-corpus replay)

The same mechanism is not limited to Tamers. In `DCGO/Assets/Scripts/Script/CardController.cs` `PlayCard()`, `isEvolution` becomes false when `CanEvolve` fails, so the `CanPlayCardTargetFrame` legality guard is skipped; `SelectCost()` finds an empty `CostList`, `baseCost` stays -1, `GetPayingCostWithBaseCost` clamps it to 0, and `DetermineAltPath` labels the difference `cost_modifier`. Corpus examples: EX12-036 Ryugumon (Lv6) onto EX12-061 (a Lv3) in six games, where the same card onto a legal EX12-063 paid 3 in 17 games; BT19-075 onto BT6-012 in one game, where onto a legal BT18-019 it paid 2 in 8 games.

`dcgo-replay` now also recognises the Digimon-target form: the digivolve action must have replayed ONLY through the recorded-operand fallback (the engine's own mask rejected the board-remapped target), recorded in `ReplaySession::digivolve_operand_fallbacks()`. The engine's legality is the proof; the tool does not re-derive card requirements.

## R-RECORDER-MISSING-ACTIVATION-ACTION: `[Main]` ability executed with no activation action row

Recordings made before DCGO `e69f10a38` (2026-08-24, CLAUDE.md rule 27, `LogAction`'s `UseCardEffect` mirror) never logged the decision to activate a `[Main]` ability, `<Delay>` or `<Training>`. The effect shows only as `selection` + `effect_activation` rows, so a replay cannot activate it and the memory it moved diverges (EX12-043 granting EX12-039 at cost -2: engine 3, recorded 2). `dcgo-replay` triages a `memory_mismatch` as this when an executed `[Main]` `effect_activation` has no field-effect or hand-effect `action` row as the nearest preceding action of that actor. Re-recording on the current mod removes the class.

## R-RECORDER-STALE-MULTIPLESKILLS-INDEX: final `MultipleSkills` index outside DCGO's own list

Three exam recordings of EX12-047 (2026-08-22/23) end on a `MultipleSkills` row whose `int_value` is 5 with two effects on offer (the exam harness passed the sim engine's trigger slot as a raw value), then `game_end` reason `disconnect`. DCGO commit `617cf381b` (2026-08-23) made such an index refused; before it the stack was cleared and the game aborted. `dcgo-replay` triages an `EngineError` "int payload N out of range for TriggerOrder" as this only when that row is the LAST decision row and the end reason is `disconnect`.

## R-RECORDER-UNRECORDED-STACK-ORDER: DCGO orders simultaneous deck placements without recording it

BT20-045's "return all of your opponent's highest-DP Digimon to the bottom of the deck" with a tie (two 7000-DP Digimon) leaves the engine asking the effect's owner which card goes to the bottom first (`replacement_departure_batch`, an `AnyField` prompt "Choose the bottommost remaining card"). DCGO places them in an order of its own and writes neither a prompt nor a selection row, so the true order is not in the recording. The replay driver answers that prompt with the lowest board slot (DCGO's frame order) when no recorded row answers it. This is an ASSUMPTION about unrecorded information: it affects only which returned card would be drawn first from that deck. It is the one place the driver fills a real engine choice from a guess, so it is limited to this prompt (bottom/top "remaining card" ordering) and to DCGO-oracle replays.

## R-RECORDER-ATTACKER-INDEX-NOT-DIGIMON: the final attack's attacker index contradicts its own board snapshot

Three corpus games (`395415c3`, `f574326348564fceb9fc89b2a11cac31`, `f1285790f6b04e8b81ca84caf6197792`) reach the same 9-permanent board (four EX12-061, EX12-076 Susanoomon, four EX12-070 Delay options) and end on an attack whose attacker index, read against that SAME row's `board_p0`, names an EX12-070 (an Option, which cannot attack; `Permanent.CanAttackTargetDigimon` starts `if (!IsDigimon) return false`). The recorder logs `GetFieldPermanents().IndexOf(AttackingPermanent)` and snapshots `GetBattleAreaPermanents()`; both iterate the same array, so they should agree, and here they do not. The engine was checked against DCGO's per-step state sidecar and is in step up to that row. Several Digimon could legally have attacked, so engine legality cannot repair the operand and the driver must not guess one. `dcgo-replay` triages an `illegal_action` as this only when it is on an attack-range action, the LAST `action` row of the recording is an attack by the same actor, and its attacker slot in its own snapshot is an Option or Tamer. The cause of the index drift on that board is unexplained.

## Q-DCGO-OPTION-WITHOUT-COLOR-SOURCE: DCGO completes an Option play with no Digimon or Tamer on the field

`cab6f074`: DCGO's bot plays the Option face of the DUAL card BT25-085 (cost 6 paid, its `[Main]` executed) and then Red Option P-180, for a player whose battle area is empty. An Option's colour requirement needs a Digimon or Tamer of each of its colours on your field (CR 4.3 4-22-2, 4-22-3) and `<Use Req.>` only waives it "with specific Digimon and/or Tamers on the field" (16-42-3); BT25-085 needs a [Three Musketeers] permanent and P-180 ignores its colour only "while you have a [Three Musketeers] trait Digimon". The engine models both correctly. The DCGO mechanism that let it through is unresolved (its `CardSource` colour and use-requirement code reads as if it should block this). `dcgo-replay` triages an `illegal_action` on a play-hand action as this only when the played card is an Option or DUAL card, the actor's board in that row is empty, and DCGO resolved the play (a paid `action_detail`).

## Q-DCGO-RULE-TRAIT-MISSING: DCGO ignores a printed "(Rule) Trait: Has [X] Type" grant

- **DCGO:** Sunarizamon EX8-047 (`Assets/Scripts/CardEffect/EX8/Black/EX8_047.cs`) adds no trait, and its card
  data carries only Reptile/LIBERATOR. Every [Mineral]/[Rock] filter (`EqualsTraits("Mineral")`) therefore leaves it
  out, e.g. Magneticdramon EX10-036's "trash 3 [Mineral] or [Rock] trait cards from your Digimon's digivolution
  cards" offers it nowhere.
- **Rules / printed data:** the card prints "(Rule) Trait: Has [Mineral] Type." (official DB bundle
  `data/card_bundles/EX8-047.md`). A rule-granted trait is a trait like any other (the 2026 Language
  Standardization / trait announcements), and `card_overrides.json` folds it in for the engine.
- **Playback signature (2026-11-11, Medusamon vs Rocks `sim_med_rocks`):** the sim chose EX8-047 among three
  [Mineral]/[Rock] sources; DCGO's panel never shows it, and End Selection with two picks does nothing. The game
  cannot be played back past that pick (step 58 of 66). The same family covers the other rule-trait cards CLAUDE.md
  names (Ice-Snow line, Iliad/TS, `[Free]`-attribute Royal Knights) wherever DCGO lacks the grant.

## Recording quarantine (`qa/rules/dcgo_recording_quarantine.json`)

For a recording that is invalid as replay input but whose invalidity no signature can prove from the recording alone, a human-reviewed entry lists its game id, an id and the diagnosis. `dcgo-replay` reports a quarantined game that STILL FAILS under `triaged`, never `pass`; if the engine later replays it cleanly it simply passes. The one current entry, `R-QUARANTINE-SCRIPTED-RAW-ACTION-IDS` (`c2bd1ab9`), is a scripted exam whose raw simulator action ids selected the wrong DCGO compact slot, so its final scripted attack targets an unsuspended Digimon with no `<Raid>`; DCGO's scripted path does not validate attack legality. Entries are for recordings whose own authoring is wrong, never for games that are merely hard to replay.

