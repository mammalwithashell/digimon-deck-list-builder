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
