# Judge-quiz card resolution (authoritative)

Source: "ZXavier's Digi Rulings" (TCG-Judges' quiz), 30 questions, full board states and card
images read from the PDF at `C:\Users\james\Downloads\judge_quiz.pdf` (rendered Google Form,
repaired to `judge_quiz_repaired.pdf` for extraction). **Card IDs below are read off the printed
card images in the PDF — they supersede the earlier name-based inventory, which guessed wrong
printings for the majority of cards.** This table is the frozen output of tasks.md §1.1.

Cluster legend: A immunity scope · B deferred rules-check · C declare-then-pay cost · D trigger
activation site · E `<Partition>`/DigiXros/de-digivolve · F token & memory arithmetic · G zone/keyword scoping.

| Q | Cluster | Cards (authoritative IDs) | Judge answer |
|---|---------|---------------------------|--------------|
| 1 | A | Belphemon: Sleep Mode **BT13-088**, Medusamon **BT24-017** | YES — Belphemon [Opp Turn] ends the attack (`<Progress>` guards Medusamon, not the battle) |
| 2 | A | Medusamon **BT24-017**, Ice Wall! **EX1-068** | NO — Medusamon `<Progress>` immune while attacking; no memory loss |
| 3 | G | Puppetmon **EX10-020**, Quartzmon **BT12-057** | YES — Puppetmon [All Turns] doesn't function in breeding area |
| 4 | G | Aldamon **AD1-002**, Atomic Inferno **BT4-098** (your Digimon Sec.A.+1), Holy Flame **ST3-15** (opp Digimon Sec.A.−1) | NO — +1 and −1 net to 1 check; one done ⇒ stop |
| 5 | C | Omnimon **AD1-025** (`[Assembly]`); WarGreymon + MetalGarurumon in trash | YES — may declare a play if its cost can be made payable |
| 6 | B | Pillomon **BT9-033**, Flame Hellscythe **BT8-109** | NO — Pillomon at 0 DP not deleted until effect resolves |
| 7 | B | Pillomon **BT9-033**, Eye of the Gorgon **BT9-108** | YES — first sub-effect deletes, second plays a Lv3 |
| 8 | B | Stack: ShineGreymon: Burst Mode **BT13-020**, ShineGreymon **AD1-016**, RizeGreymon **BT21-044**, GeoGreymon **BT21-042**, Agumon **EX4-005**, Koromon **BT21-004**; security Comet Hammer **BT23-096** | Agumon trashed → Koromon trashed (Burst-Digivolve EoT trash; DP-less can't remain) |
| 9 | D | Mastemon **BT23-102**, Gatomon **BT15-037** | After both trashed; NO memory (Gatomon not in BA during removal) |
| 10 | F | Akihiro Kurata **BT13-103**, MirageGaogamon **BT11-033**, Gravity Crush **BT1-090**, Mental Training **P-104** | 0 |
| 11 | F | (same as Q10) | 4 (Gravity Crush not `[Once Per Turn]` fires again) |
| 12 | F | Venusmon **BT24-040**, Sharkmon **BT24-059**, Petrification token | YES, will unsuspend (token placeable as digivolution card) |
| 13 | B | Nyabootmon **BT22-042**, ShoeShoemon **P-165**, Rapidmon (X Antibody) **BT16-101**, Rapidmon **ST17-07** | −6000 DP |
| 14 | B | Nyabootmon **BT22-042**, ShoeShoemon **P-165**, ShineGreymon: Ruin Mode **EX4-074**, Rapidmon (X Antibody) **BT16-101** | −6000 DP |
| 15 | E | LordKnightmon (X Antibody) **BT19-073**, LordKnightmon **BT19-072**; Player B stack: Omnimon (X Antibody) **BT20-102**, Gallantmon (X Antibody) **EX8-073**, Gallantmon **BT17-016**, WarGrowlmon **BT12-016**, Growlmon **EX3-057**, Guilmon **EX4-006** | Gallantmon (X Antibody) is topmost (its `[All Turns]` immunity blocks remaining `<De-Digivolve 1>`) |
| 16 | E | Paildramon **BT16-025**, ExVeemon **BT12-022**, Stingmon **BT12-050**, Lilithmon **EX6-057** | NO — granted-effect-self-delete counts as leaving by own effect; `<Partition>` won't trigger |
| 17 | A | Magnamon (X Antibody) **BT16-102**, Magnamon **BT21-036**, Lilithmon **EX6-057** | NO — `[When Digivolving]` immunity removes the Lilithmon-granted `[EoT] Delete` |
| 18 | A | Quantumon **LM-020**, Imperialdramon: Paladin Mode ACE **BT17-077** | NO — Quantumon immunity is to ALL Digimon effects incl. its own; `<Blast Digivolve>` is an effect |
| 19 | D | Eyesmon: Scatter Mode **BT7-069**, Gabumon **BT2-069**, DemiMeramon **BT3-006**, Calling From the Darkness **BT7-107** | 0 — returned to hand ⇒ no `[On Deletion]` |
| 20 | D | (Q19 board) + Pumpkinmon **BT2-076** | 8 — Eyesmon stayed in trash ⇒ all `[On Deletion]` fire |
| 21 | D | (Q20 board) + Back for Revenge! **BT3-109** | 0 — Eyesmon played from trash ⇒ remaining `[On Deletion]` can't fire |
| 22 | F | Proganomon **EX8-051**, Tumblemon **EX8-005** ×3, Medusamon **BT24-017**, Petrification token | YES — Digi-Eggs to egg deck still satisfy "send 2 to bottom" ⇒ 2 tokens |
| 23 | D | (Q22 board) | 1 — only one Tumblemon's inherited `[On Deletion]` remains in trash |
| 24 | B | Hudiemon **BT23-101**, Tentomon **BT23-037**, Kokomon **EX6-004**, Rapidmon (X Antibody) **BT16-101**, Rapidmon **ST17-07** | 3000 DP (Tentomon suspended → −4000 → deleted by rules check before Kokomon's trigger) |
| 25 | E | WarGreymon **AD1-004**, Miraculous Mega Knight **BT17-095**, Dorbickmon **EX3-014**, MetalGarurumon **AD1-014**, Omnimon **AD1-025** | YES — DigiXros departure ≠ leaving by battle |
| 26 | C | (Q25 board) | Returns to hand — cost unpayable after Miraculous Mega Knight DNA-evo |
| 27 | C | (Q25 board) | Pays 0 memory |
| 28 | A | Gankoomon (X Antibody) **BT20-059**, Gankoomon **BT23-057**, Dragomon **EX5-060**, Sistermon Ciel **BT6-084** | YES — plays AND activates (Gankoomon X protection beats Dragomon "[On Play] don't activate") |
| 29 | E | Yuu Amano **BT10-093**, ChuuChuumon **EX10-039**, Damemon **EX10-044**, DarknessBagramon **EX10-059**, Bagramon **EX10-056**, DarkKnightmon **EX10-031** | 3 legal stacks (Yuu Amano top placement either order; DigiXros targets bottom in spec order) |
| 30 | C+E | Chaosmon: Valdur Arm **BT20-037**, BanchoLeomon **BT20-036**, MedievalGallantmon **EX8-074**, Imperialdramon: Dragon Mode **EX3-063**, Dinobeemon **BT16-077**, Flamedramon **EX3-008** | Suspend Imperialdramon: Dragon Mode + Chaosmon: Valdur Arm w/ cost reduction (`<Partition>` interruptive; BanchoLeomon not yet in play) |

## Implementation status (filesystem scan, 2026-05-28 — tasks §1.3 DONE)

**79 distinct cards** (+ Petrification token). Scanned `code/digimon-engine/cards/**` (DSL YAML) and
`tests/cards_behavioral/**` (behavioral tests). `raw_rust/` is empty (only `mod.rs`), so DSL YAML is
the sole implementation path — no hidden hand-written effects.

- **Implemented (DSL YAML present) — 27** (26 also have a behavioral test; BT7-107 lacks one):
  AD1-004, AD1-014, AD1-025, BT1-090, BT6-084, BT7-107, BT9-033, BT12-022, BT12-050, BT16-025,
  BT17-077, BT17-095, BT19-072, BT20-102, BT22-042, BT23-057, BT23-096, BT24-017, BT24-040,
  EX1-068, EX4-006, EX4-074, EX8-005, EX8-051, EX8-073, EX8-074, P-165. Plus the **Petrification token**.
- **Need authoring — 52:**
  AD1-002, AD1-016 · BT2-069, BT2-076 · BT3-006, BT3-109 · BT4-098 · BT7-069 · BT8-109 · BT9-108 ·
  BT10-093 · BT11-033 · BT12-016, BT12-057 · BT13-020, BT13-088, BT13-103 · BT15-037 · BT16-077,
  BT16-101, BT16-102 · BT17-016 · BT19-073 · BT20-036, BT20-037, BT20-059 · BT21-004, BT21-036,
  BT21-042, BT21-044 · BT23-037, BT23-101, BT23-102 · BT24-059 · EX3-008, EX3-014, EX3-057,
  EX3-063 · EX4-005 · EX5-060 · EX6-004, EX6-057 · EX10-020, EX10-031, EX10-039, EX10-044,
  EX10-056, EX10-059 · LM-020 · P-104 · ST3-15 · ST17-07.

**Caveat:** "YAML present" ≠ verified-faithful/passing. §1.3's authoring re-audit (AUDIT mode) must
still confirm each implemented card's YAML is complete against printed text and its test passes;
BT7-107 needs a test authored regardless.

### Discovery-wave readiness (all referenced cards already implemented)

- **Q22 — DISCOVERED ENGINE BUG (2026-05-29).** Audited and a focused test PROVES a real gap:
  `return_trash_cards_to_deck_bottom` (effect_context/mod.rs:5554) inserts Digi-Eggs into the MAIN
  deck — no digitama routing — violating the rule Q22 tests. Test
  `f_token_and_memory::q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` confirmed
  failing (`digitama_deck.len()` 0, expected 1), now `#[ignore]`-d citing
  **G-RETURN-TRASH-DIGI-EGG-ROUTING** (qa/archetype-qa/engine-gaps.md). NOTE: the surface judge
  answer ("2 tokens?") would pass regardless of routing — only checking the *destination* catches it.
  Fix is small (branch on `CardKind::DigiEgg` → `digitama_deck`).
- **Q23 — candidate, same area.** Inherited gain-memory (Tumblemon EX8-005) IS implemented; the
  "must remain in trash to resolve" gating (only 1 of 3 gains memory after 2 are returned) routes
  through the same return verb — audit pending.
- **Q5 — PASS (resolved 2026-05-29, change `fix-ad1-025-assembly-data`).** Originally BLOCKED-DATA
  (discovery wave): AD1-025's `[Assembly]` keyword was ABSENT from `data/cards.json` — but the apply
  spike found a SECOND, deeper layer: there was no engine Assembly executor at all (the `assembly`
  alt-path KIND compiled but was matched in no play path; BT18-102 only proved the DSL compiles it).
  Fixed as a hybrid engine+data+YAML change: implemented the Assembly play executor
  (G-ASSEMBLY-PLAY-EXECUTION — eligibility from trash, surfaced per-element selection, bottom
  placement before `[On Play]`, reduced cost, declare-then-pay mask), restored `[Assembly]` to
  `card_overrides.json`, and authored the `assembly` alt_path in `cards/ad1/AD1-025.yaml` (materials
  WarGreymon × MetalGarurumon, zones [trash], reduce cost 6 — DCGO AD1_025.cs:214-255). Test
  `c_declare_then_pay::q5_...` is now a live mask assertion and PASSES.

**Discovery-wave gaps (4 distinct, all proven/code-confirmed):**
- Q2 + Q16 + Q17 → RESOLVED (`add-grant-triggered-effect-dsl`): the grant-triggered-effect substrate
  existed (EX10-034); all three cause-attribution directions landed — Q2 (Progress suppresses the
  granted opponent effect; EX1-068), Q16 (granted body runs as the carrier's own effect so a granted
  self-delete is OwnEffect → `<Partition>` skips it; EX6-057), Q17 (a carrier immune to the grantor's
  effects suppresses the granted clause via `permanent_is_unaffected_by_effect`; BT16-102 Magnamon X).
  All three PASS. BT21-036 was NOT needed (Armor-Form source staged synthetically).
- Q5 → RESOLVED: was missing SOURCE DATA *and* the engine Assembly executor; both fixed by
  `fix-ad1-025-assembly-data` (G-ASSEMBLY-PLAY-EXECUTION). Q5 now PASSES.
- Q22 → RESOLVED (`fix-judge-quiz-engine-gaps` Gap 2): Digi-Egg now routes to the digitama deck
  (G-RETURN-TRASH-DIGI-EGG-ROUTING). Q22 PASSES.
- Cluster B (Q6/Q8/Q13/Q14/Q24) → ENGINE GAP, the systemic one: **no general state-based ≤0-DP
  rules-check** (G-NO-GENERAL-ZERO-DP-RULES-CHECK) — only Arts digivolve triggers it. Proven by a
  synthetic probe independent of card authoring.

Rule confirmed PRESENT (not a gap): the `<Partition>` cause-filter (skips Battle | OwnEffect,
keyword_effects.rs:839) — the rule behind Q16/Q25/Q30, exercisable via implemented AD1-025.

Central lesson: "DSL YAML present" is a weak proxy for faithful — gaps live at four layers (engine
rules-machinery, engine primitive, card-YAML clause, source-data keyword), and only audit-before-
asserting distinguishes them. Probing the engine RULE each cluster needs (not just the cards) finds
systemic gaps like the ≤0-DP rules-check before any card is authored.
- **Q2 — BLOCKED (discovery-wave finding, 2026-05-28).** Both cards' YAML exists, but EX1-068 Ice
  Wall!'s `[Main]` grant is OMITTED (`G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT`, qa/dsl-vocab-gaps.md),
  so the granted "[When Attacking] lose 2 memory" can't be staged on Medusamon. A test playing the
  no-op Ice Wall would false-pass. `<Progress>` immunity itself IS implemented
  (`Game::progress_excludes`, combat.rs:2667). Test `q2_...` is `#[ignore]`-blocked citing the gap.
  This vindicates the "YAML present ≠ verified-faithful" caveat above — the AUDIT pass is real work.
- **One card away: Q1** (BT13-088), **Q6** (BT8-109), **Q7** (BT9-108), **Q12** (BT24-059),
  **Q14** (BT16-101), **Q16** (EX6-057), **Q18** (LM-020), **Q25/Q26/Q27** (EX3-014 Dorbickmon).

### Per-cluster authoring load (distinct unimplemented cards, gap-finding order)

| Cluster | Questions | Cards to author |
|---------|-----------|-----------------|
| A immunity scope | Q1,2,17,18,28 | 7 — BT13-088, BT16-102, BT21-036, EX6-057, LM-020, BT20-059, EX5-060 |
| B deferred rules-check | Q6,7,8,13,14,24 | 13 — BT8-109, BT9-108, BT13-020, AD1-016, BT21-044, BT21-042, EX4-005, BT21-004, BT16-101, ST17-07, BT23-101, BT23-037, EX6-004 |
| C declare-then-pay | Q5,26,27,30 | 6 — EX3-014, BT20-037, BT20-036, EX3-063, BT16-077, EX3-008 (Q5 fully covered) |
| D activation site | Q9,19,20,21,23 | 7 — BT23-102, BT15-037, BT7-069, BT2-069, BT3-006, BT2-076, BT3-109 (Q23 covered) |
| F token & memory | Q10,11,12,22 | 4 — BT13-103, BT11-033, P-104, BT24-059 (Q22 covered) |
| E partition/digixros | Q15,16,25,29,30 | shares C's BT20-037/036, EX3-063, BT16-077, EX3-008; own: BT19-073, BT17-016, BT12-016, EX3-057, BT10-093, EX10-031/039/044/056/059 |
| G zone/keyword | Q3,4 | 5 — EX10-020, BT12-057, AD1-002, BT4-098, ST3-15 |

A is the smallest authoring load AND highest gap-likelihood ⇒ natural first authoring wave after the
discovery pass. B is heaviest but is the richest rules cluster (deferred rules-check).
