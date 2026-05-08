# Puppets Rust Engine / DSL Gap Inventory

Date: 2026-05-03
Archetype: Puppets / Nyabootmon
Assessment source: `data/deck_library.json` archetype `Puppets`
Rust target: `code/digimon-engine/` plus YAML DSL under `code/digimon-engine/cards/`
Verdict: blocked

This file captures reusable Rust engine and DSL gaps surfaced by the Puppets archetype so they can be folded into a cross-archetype tracking spec. It is intentionally stricter than `qa/archetype-qa/Puppets.md`, which is a legacy Python-lane faithfulness report.

## Assessment Target

`code/tools/resolve_deck.py Puppets --json` resolves `Puppets` through `data/deck_library.json` and `data/archetype_aliases.json` to 25 decklists and 61 unique card IDs. `data/archetype_aliases.json` maps `Nyabootmon` as an alias for canonical `Puppets`. The resolver also refreshed `qa/archetype-qa/puppets/deck_pool.json`.

High-frequency core cards across those lists:

| Card | Name | Frequency | Core role |
|---|---:|---:|---|
| `BT22-042` | Nyabootmon | 25/25 | top-end Overclock, play Lv4 or lower Puppet, re-fire When Digivolving |
| `EX9-024` | Hanimon | 25/25 | hand-trash cost, Puppet recursion |
| `EX9-032` | Karakurumon | 25/25 | delete Token/Puppet to effect-digivolve |
| `EX9-033` | Kaguyamon | 25/25 | Alliance/Blocker aura, lowest-level delete, trash play |
| `EX9-067` | Mirai Kinosaki | 25/25 | reveal search, digivolve observer, reduced-cost play |
| `ST19-03` | Shoemon | 25/25 | reveal search, inherited security-DP aura |
| `P-165` | ShoeShoemon | 24/25 | Security play, Familiar token |
| `EX7-024` | Shoemon | 21/25 | Puppet digivolve cost reduction |
| `ST19-14` | Arisa Kinosaki | 20/25 | memory setter, Token/Puppet play observer, Rush grant |
| `BT16-055` | Namakemon | 19/25 | keyword protection/grant package |
| `BT22-002` | Kyaromon | 18/25 | inherited draw on Token/Puppet deletion |
| `EX7-027` | Chaperomon | 18/25 | Overclock, play Lv3 Puppet, prevent leave |
| `EX9-027` | Kokeshimon | 17/25 | discard-cost DP reduction, attack-cancel inherited |
| `BT22-098` | Unique Emblem: Fable Waltz | 16/25 | event-gated Delay |
| `EX7-025` | ShoeShoemon | 15/25 | free Arisa play, inherited security-DP aura |
| `EX7-063` | Arisa Kinosaki | 15/25 | memory, security play, deletion observer |
| `LM-029` | Yellow Scramble | 15/25 | option support package |

Newer `EX11` cards are lower-frequency in this snapshot but important for the same reusable gaps: `EX11-019`, `EX11-021`, `EX11-022`, `EX11-023`, `EX11-024`, `EX11-060`, and `EX11-061`.

## Current Implementation Evidence

- Production effects are embedded from YAML under `code/digimon-engine/cards/` by `code/digimon-engine/build.rs`, then registered through `code/digimon-engine/src/cards.rs`.
- `BT22-029` and `BT22-032` now have production YAML and focused card-level behavioral tests under `code/digimon-engine/cards/bt22/` and `code/digimon-engine/tests/cards_behavioral/bt22/`.
- `code/digimon-engine/cards/ex11/` now includes Puppet `EX11-019.yaml`, `EX11-021.yaml`, `EX11-022.yaml`, `EX11-023.yaml`, `EX11-024.yaml`, `EX11-060.yaml`, and `EX11-061.yaml`; `EX11-023`, `EX11-060`, and `EX11-061` are partial for the slices routed below.
- `code/digimon-engine/cards/ex9/EX9-024.yaml`, `EX9-027.yaml`, `EX9-032.yaml`, `EX9-033.yaml`, and `EX9-067.yaml` now cover the implemented/partial slices listed below.
- `code/digimon-engine/cards/st19/ST19-03.yaml` now covers Shoemon's On Play dual-bucket reveal search. Its inherited opponent-security-Digimon DP aura remains a DSL vocabulary gap.
- `code/digimon-engine/cards/st19/` also now contains `ST19-04`, `ST19-05`, `ST19-06`, `ST19-07`, `ST19-09`, `ST19-10`, `ST19-12`, and `ST19-14`. `ST19-14` is partial because its effect-played Token/Puppet Rush observer remains blocked by `PUPPETS-G005`.
- `code/digimon-engine/cards/ex7/` now includes Puppet `EX7-025.yaml`, `EX7-027.yaml`, and `EX7-063.yaml`.
- `code/digimon-engine/cards/p/` now includes `P-134.yaml`, `P-165.yaml`, and partial `P-229.yaml`.
- Engine support exists for several reusable pieces: Overclock pending cost/mask flow, Familiar token behavior, Scapegoat/Barrier keyword auto-effects, Alliance interrupt flow, effect-initiated play/digivolve primitives, reveal/add-to-hand movement, and attack observer/cancel flow. This batch added DSL timing vocabulary for `on_ally_attack` / `on_opponent_attack` so inherited attack-cancel cards can use the existing engine flow declaratively.

## Batch 1 Implementation Note

Implemented and validated on 2026-05-03:

| Card | Status | Covered slices | Remaining faithful gap |
|---|---|---|---|
| `BT22-029` | implemented | On Play/On Deletion Puppet Blocker grant; inherited When Attacking -2000 DP | none identified in covered text |
| `BT22-032` | implemented | optional On Deletion play level 3 Puppet from hand; decline path; inherited When Attacking -2000 DP | none identified in covered text |
| `EX9-067` | partial | On Play reveal search; Security play this Tamer without paying cost | `PUPPETS-G005` for digivolve observer/reduced-cost effect-play rider |
| `ST19-03` | partial | On Play reveal top 3, add 1 Puppet and 1 LIBERATOR without double-picking the same card | `G-OPPONENT-SECURITY-DP-AURA` / `PUPPETS-G008` for inherited opponent security Digimon DP aura |

Implemented and validated in follow-up batches on 2026-05-03:

| Card | Status | Covered slices | Remaining faithful gap |
|---|---|---|---|
| `ST19-04` | implemented | On Play Puppet hand-discard Draw 2; decline; inherited Reboot | none identified in covered text |
| `ST19-05` | implemented | Blocker; On Deletion Puppet hand-discard Draw 2 | none identified in covered text |
| `ST19-06` | implemented | On Play/On Deletion opponent Digimon Security Attack -1 | none identified in covered text |
| `ST19-07` | implemented | Jamming; inherited Barrier | none identified in covered text |
| `ST19-09` | implemented | Blocker; optional On Deletion play level 3 Puppet from hand | none identified in covered text |
| `ST19-10` | implemented | Armor Purge; inherited Barrier | none identified in covered text |
| `ST19-12` | implemented | Puppet Overclock; Blocker; optional When Digivolving play 2 Familiar Tokens | none identified in covered text |
| `ST19-14` | partial | Start-of-turn memory setter; Security play this Tamer without paying cost | `PUPPETS-G005` for effect-played Token/Puppet Rush observer |

Resolver-backed corrected batches on 2026-05-03:

| Card | Status | Covered slices | Remaining faithful gap |
|---|---|---|---|
| `EX9-024` | implemented | On Play optional hand-trash cost into Puppet recursion from trash; decline path; inherited opponent-turn attack cancel by deleting this Digimon | none identified in covered text |
| `EX9-027` | implemented | When Digivolving/On Deletion hand-trash cost into opponent Digimon -4000 DP; inherited opponent-turn attack cancel by deleting this Digimon | none identified in covered text |
| `EX11-019` | implemented | explicit may-choice for On Deletion Familiar Token play; inherited Barrier | none identified in covered text |
| `P-134` | implemented | On Play opponent Digimon Security Attack -1; inherited When Attacking -2000 DP | none identified in covered text |
| `EX7-025` | partial | When Digivolving optional Arisa play from hand if you have 1 or fewer Tamers | `G-OPPONENT-SECURITY-DP-AURA` / `PUPPETS-G008` for inherited opponent security Digimon DP aura |
| `EX7-027` | partial | Puppet Overclock; optional When Digivolving play level 3 Puppet from hand | inherited leave-prevention by deleting Token/other Puppet needs reusable replacement-cost body |
| `EX7-063` | partial | Start of Your Main Phase memory gain; Security play this Tamer without paying cost | Token/Puppet deletion observer with suspend-this-Tamer cost |
| `EX11-021` | implemented | When Digivolving optional Mirai play from hand if you have 1 or fewer Tamers; inherited opponent-turn attack cancel by deleting this Digimon | none identified in covered text |
| `ST19-01` | implemented | inherited When Attacking Draw 1 if another own Digimon exists; carrier exclusion; once-per-turn lockout | none identified in covered text |
| `BT15-003` | blocked | tests document top/bottom choice, top-security branch, and no-security gate | bottom/selected security trash needs native DSL movement; `_examples/BT15-003.yaml` duplicate/raw_rust fixture blocks production YAML |
| `BT22-002` | implemented | inherited Token/other-Puppet deletion observer reads deleted-object snapshot, draws 1, excludes carrier/opponent/non-Puppet deletes, and respects once-per-turn | none identified in covered text |
| `EX11-020` | partial | inherited opponent-turn attack cancel by deleting one other Digimon | `PUPPETS-G012` for On Deletion deletion-cause predicate |
| `EX7-024` | partial | printed metadata and yellow Lv2 cost-0 digivolution path | `PUPPETS-G013` for source-scoped digivolve-into-trait cost reduction; `PUPPETS-G008` for inherited opponent security Digimon DP aura |
| `ST19-08` | partial | Puppet/Token Overclock cost filter | `PUPPETS-G014` for filtered origin-preserving union-zone play; `PUPPETS-G008` for inherited opponent security Digimon DP aura |
| `ST19-11` | partial | On Play/When Digivolving select 1 opponent Digimon and -3000 DP | `PUPPETS-G015` for the 3+ Digimon extra -3000 branch; inherited leave-prevention by deleting Token/other Puppet remains on the replacement-cost body tracked with `EX7-027` / `BT22-036` |
| `P-165` | partial | Security play-self; On Play/When Digivolving Familiar Token; inherited Barrier | `PUPPETS-G016` for "that token" provenance plus opponent-turn-end cleanup |
| `EX7-030` | implemented | Puppet/Token Overclock; optional Start of Main/When Digivolving Familiar Token; When Attacking -6000 DP | none identified in covered text |
| `EX11-024` | implemented | Alliance; Puppet/Token Overclock; optional Lv4-or-lower Puppet free-play; Familiar Tokens per opponent Digimon; count-scaled DP reduction | none identified in covered text |
| `BT22-040` | implemented | Puppet/Token Overclock; optional On Play/When Digivolving Familiar Token; All Turns OPT other-Digimon-deleted refire of its When Digivolving effect | none identified in covered text |
| `BT22-042` | partial | standard yellow Lv6 cost-4 digivolve; Puppet/Token Overclock; When Digivolving optional Lv4-or-lower Puppet free-play followed by mandatory count-scaled DP reduction; All Turns OPT other-Digimon-deleted refire of its When Digivolving effect | conditional Arisa+Chaperomon route needs `G-ALT-PATH-CONDITION` |
| `EX9-032` | partial | printed metadata; yellow Lv5 and Puppet Lv5 digivolve paths | active delete-cost self-digivolve needs `PUPPETS-G018`; inherited leave-prevention by deleting Token/other Puppet needs `PUPPETS-G019` |
| `EX9-033` | partial | yellow Lv5 and Puppet Lv5 digivolve paths; Alliance/Blocker aura for own Puppet Digimon and Tokens; End of Your Turn optional level 4 or lower Puppet play from trash | other-deletion lowest-level delete observer now needs card-local YAML/test adoption using the closed `PUPPETS-G011` payload |
| `BT22-036` | partial | yellow Lv4 cost-3 digivolve path; Puppet/Token Overclock | Hand Main Arisa-gated ShoeShoemon trash-to-Shoemon source placement and hand-card digivolve needs `PUPPETS-G020`; inherited leave-prevention by deleting Token/other Puppet needs `PUPPETS-G019` |
| `EX11-022` | partial | yellow Lv4 and Puppet Lv3 digivolve paths; Scapegoat replacement using another own Digimon | hand-or-trash Puppet DP<=4000 free-play needs `PUPPETS-G021`; effect-played cleanup needs `PUPPETS-G003`; inherited leave-prevention by deleting Token/other Puppet needs `PUPPETS-G019` |
| `EX11-023` | partial | yellow Lv5 and Puppet Lv5 digivolve paths; Alliance; Scapegoat; mandatory lowest-level opponent Digimon delete on When Digivolving and End of Opponent's Turn | other-Digimon-deleted optional level 4 or lower Puppet trash play now needs card-local YAML/test adoption using the closed `PUPPETS-G011` payload |
| `EX11-060` | implemented | start-of-turn memory setter; Token/Puppet deletion observer with visible suspend-this-Tamer Draw 1 branch; Overclock-only level 4 Puppet hand-play; Security play this Tamer without paying cost | none identified in covered text |
| `EX11-061` | partial | Start of Your Main Phase memory gain; Security play this Tamer without paying cost | Puppet digivolve observer/effect-play branch needs `PUPPETS-G005`; exact turn-end cleanup needs `PUPPETS-G003` |
| `P-229` | partial | Main and Security reveal-top-3 dual-bucket search for 1 Puppet Digimon and 1 LIBERATOR | option battle-area placement and Mirai-played event-gated Delay/reduced-cost digivolve need `PUPPETS-G004` and `PUPPETS-G009` |
| `BT22-098` | partial | supported hand-origin Main/Security Shoemon/Arisa free-play; Arisa-suspend event-gated Delay with Puppet base and Puppet+LIBERATOR hand digivolve cost -3 | exact hand-or-trash origin-preserving play needs `PUPPETS-G014`; full security/Main option placement remains under `PUPPETS-G009` |
| `LM-029` | partial | Main yellow Digimon effect-digivolve cost -3; mandatory Security add-this-option-to-hand tail | optional Security yellow DP<=2000 trash play and Scramble Delay trash-to-deck-top body need hidden-zone DP filtering, trash-to-deck-top movement, and mandatory-tail continuation under `PUPPETS-G017` |
| `P-105` | partial | Main reveal-top-2 yellow-card search; battle-area delayed Option placement; inherited Security placement; scheduled Delay yellow hand digivolve cost -2 | standard Delay main-phase activation remains under `PUPPETS-G009` |
| `LM-054` | partial | color-bypass use requirement; Main reveal-top-2 yellow/black search; inherited Security reveal/search plus battle-area placement; scheduled Delay yellow/black hand digivolve cost -2 | standard Delay main-phase activation remains under `PUPPETS-G009` |
| `BT13-101` | partial | optional On Play PawnChessmon-named hand free-play; Security play self | All Turns 2-color black/yellow play observer needs event-card exact color/count predicates and visible source-bound suspend-cost preflight under `PUPPETS-G023` |
| `BT16-055` | partial | black Lv3 and Pulsemon digivolve paths; <=3 security Blocker/Reboot grant branch | >=3 security narrow protection and inherited text-contains-Pulsemon aura need `PUPPETS-G024` and `PUPPETS-G025` |
| `BT20-084` | partial | Sistermon Ciel cost-1 digivolve; On Play/When Digivolving CannotSuspend target; trash-resident optional free digivolve from trash | top-stack-card-to-security movement needs `PUPPETS-G027` |
| `BT22-088` | partial | Security play self | Start of Main return-this-Tamer optional cost and Token/Puppet played observer need `PUPPETS-G028` and `PUPPETS-G005` |
| `BT23-077` | implemented | Blocker; On Play delete opponent Digimon with play cost 4 or less; self-suspend De-Digivolve observer gated by `event_permanent_is_source` | none identified in covered text |
| `BT5-033` | implemented | yellow Lv2 digivolve path; opponent-turn digivolution cost-reduction floodgate | none identified in covered text |
| `BT5-106` | partial | Main optional own-Digimon delete into visible own suspended purple Digimon unsuspend selection | Security trash play with played-Digimon On Play suppression needs `PUPPETS-G030` |
| `BT6-084` | implemented | Huckmon/Royal Knight own-Digimon +2000 DP aura; On Play gain 1 memory | none identified in covered text |
| `BT9-033` | implemented | yellow Lv2 digivolve path; all-player Digimon-by-effect play floodgate | none identified in covered text |
| `EX4-074` | partial | purple/yellow metadata; purple Lv6 and ShineGreymon digivolve paths | opponent-next-turn DP reduction needs next-turn expiry support; End of Attack chain needs `PUPPETS-G031` |
| `EX6-011` | partial | metadata; Ace Overflow; red Lv6 and red+black Lv6 DNA paths; Raid/Reboot; On Play/When Digivolving security trash, opponent-effect immunity, and DNA-origin De-Digivolve/delete tail | Hand Counter Blast DNA Digivolve needs `PUPPETS-G032` |
| `EX8-030` | implemented | yellow Lv2 and NSo Lv2 digivolve paths; opponent memory-gain floodgate with Tamer-source exemption | none identified in covered text |
| `P-136` | partial | metadata; optional On Play exact Shoemon hand free-play; Security play self | Puppet-digivolve observer with source-bound suspend-this-Tamer cost needs `PUPPETS-G023` |
| `P-156` | partial | Tamer-based option color bypass; Main chosen-Tamer binding into hand/trash Digimon free-play filtered by bound Tamer color and play cost <=3; mandatory Security add-this-option-to-hand tail | Security optional Tamer play before mandatory add-to-hand tail needs `PUPPETS-G017` |

The earlier ST19 follow-up batch used the broader Puppet/ST19 card group. In the current resolver output, `ST19-04`, `ST19-05`, `ST19-12`, and `ST19-14` are in the resolved `Puppets` deck pool; `ST19-06`, `ST19-07`, `ST19-09`, and `ST19-10` are valid Puppet/ST19 implementations but are outside this deck-library pool snapshot.

## Gap Summary

| Gap ID | Type | Status | Blocks | Canonical tracker |
|---|---|---|---|---|
| `PUPPETS-G001` | dsl-gap / test-gap | open | Most Puppet core cards | none; archetype-local authoring backlog |
| `PUPPETS-G002` | engine-gap / dsl-gap | closed for Puppet self-refire | refire primitive exists; `BT22-040` and `BT22-042` prove deleted-object gated self-refire fixtures, including visible optional refire and once-per-turn handling | `qa/archetype-qa/engine-gaps.md` |
| `PUPPETS-G003` | engine-gap | open | `EX11-022`, `EX11-061`, related effect-play cleanup cards | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G004` | hybrid | partially resolved | `BT22-098`, `P-229` | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G005` | engine-gap / test-gap | open | `EX9-067`, `EX11-061`, `ST19-14`, `BT22-088` | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G006` | test-gap | partially resolved | security end-of-battle play effects | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G007` | test-gap | open | `EX9-033`, `EX11-023`, Puppet aura package | none; add card-level regression coverage |
| `PUPPETS-G008` | dsl-gap | open | `ST19-03`, `EX7-024`, other inherited opponent-security DP auras | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G009` | engine-gap | open | `P-037`, `P-105`, `LM-035`, `LM-037`, `LM-054`, `BT22-098`, standard Memory Boost/Training/Scramble Delay options | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G010` | dsl-gap | open | `BT15-003`, top/bottom security-cost effects | `qa/archetype-qa/engine-gaps.md` |
| `PUPPETS-G011` | engine-gap | closed | `BT22-002`, Token/Puppet deletion observers | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G012` | dsl-gap | open | `EX11-020`, On Deletion cause-gated effects | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G013` | hybrid | open | `EX7-024`, source-scoped digivolve-into-trait cost reduction | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G014` | hybrid | open | `ST19-08`, `BT22-098`, hand-or-trash filtered free-play security effects | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G015` | engine-gap / dsl-gap | open | `ST19-11`, count-threshold modifier branches | `qa/archetype-qa/engine-gaps.md` |
| `PUPPETS-G016` | engine-gap | open | `P-165`, "that token" cleanup riders | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G017` | hybrid | partially resolved | `BT22-042` closed by outer-tail rewrap for nested selections; `LM-029`, `P-156`, and other optional sub-effect + mandatory "Then" tails still need card-shaped adoption | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G018` | hybrid | open | `EX9-032`, costed self-digivolve after deleting Token/Puppet cost body | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G019` | engine-gap / dsl-gap | open | `EX9-032`, `BT22-036`, `EX11-022`, inherited Token/Puppet leave-prevention replacements | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G020` | hybrid | open | `BT22-036`, hand-main Arisa-gated ShoeShoemon trash-to-Shoemon source placement and hand-card digivolve | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G021` | dsl-gap | open | `EX11-022`, hand-or-trash Puppet DP<=4000 free-play selection | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G022` | hybrid | closed | `EX11-060`, suspend-this-Tamer deletion observer with Overclock cause branch | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G023` | hybrid | open | `BT13-101`, `P-136`, event-card/event-target predicates plus source-bound suspend-this-Tamer triggered cost | `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G024` | engine-gap / dsl-gap | open | `BT16-055`, narrow protection from opponent DP reduction and De-Digivolve | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G025` | dsl-gap | open | `BT16-055`, inherited carrier rules-text-contains-Pulsemon DP aura | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G026` | hybrid | closed | `BT20-084`, trash-resident observer and effect digivolve from trash into a field Sistermon Ciel | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G027` | engine-gap / dsl-gap | open | `BT20-084`, move top stacked card to top security at End of All Turns | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G028` | hybrid | open | `BT22-088`, optional triggered return-this-Tamer-to-deck cost before chained free-play branches | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G029` | dsl-gap | closed | `BT23-077`, self-scoped OnSuspend observer predicate | `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G030` | engine-gap / dsl-gap | open | `BT5-106`, Security play from trash while suppressing played Digimon On Play effects | `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md` |
| `PUPPETS-G031` | engine-gap / dsl-gap | open | `EX4-074`, End of Attack self-delete/opponent-delete/Recovery/conditional-hatch chain | `docs/RUST_ENGINE_GAPS.md` |
| `PUPPETS-G032` | engine-gap | open | `EX6-011`, Counter Blast DNA Digivolve activation from hand | `docs/RUST_ENGINE_GAPS.md` |

## Detailed Gaps

### PUPPETS-G001: Production YAML Missing for Core Puppet Package

- **Type:** `dsl-gap` / `test-gap`
- **Status:** open
- **Blocks:** remaining unauthored Puppet pool cards and unauthored option/support cards. `BT22-002`, `BT22-040`, `EX9-024`, `EX9-027`, `EX11-019`, `EX11-021`, `EX11-024`, `EX7-030`, and `ST19-01` now have full covered-text YAML/tests. `EX9-032`, `EX9-033`, `EX9-067`, `EX7-024`, `EX7-025`, `EX7-027`, `EX7-063`, `EX11-020`, `EX11-022`, `EX11-023`, `EX11-060`, `EX11-061`, `ST19-03`, `ST19-08`, `ST19-11`, `ST19-14`, `BT22-036`, `BT22-042`, `P-165`, and `P-229` now have partial production YAML, with omitted slices routed to reusable gaps below. For `BT22-042`, only the conditional Arisa+Chaperomon alternate digivolution route remains omitted.
- **Why it matters:** The Rust runtime only executes production card behavior that is registered from the embedded DSL pack or explicit Rust effects. The Puppet archetype cannot be used as a serious Rust training/evaluation target while its core cards are metadata-only.
- **Evidence:** The relevant set/card YAML files are absent from `code/digimon-engine/cards/`; only unrelated or adjacent cards are present in `ex9/`, `ex11/`, and `p/`.
- **First test:** Add a card-level DSL registration test for `BT22-042` and assert the compiled card has Overclock, When Digivolving, and other-deletion reactivation clauses.
- **Implementation hint:** Start with production YAML under `code/digimon-engine/cards/bt22/BT22-042.yaml` and card-level tests under `code/digimon-engine/tests/cards_behavioral/bt22/`.

### PUPPETS-G002: Re-Activate a Card's `[When Digivolving]` Effect From Another Trigger

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** closed for Puppet self-refire as of 2026-05-08; `PUPPETS-G011` no longer blocks the deletion-observer gate.
- **Blocks:** no longer blocks `EX11-024`, `BT22-040`, or `BT22-042`. Non-Puppet foreign-card activation variants remain tracked in `qa/archetype-qa/engine-gaps.md`.
- **Effect text:** "When any of your other Digimon are deleted, you may activate 1 of this Digimon's [When Digivolving] effects."
- **Why it matters:** This is a core Puppet payoff. The engine needs to enumerate eligible `[When Digivolving]` effects, expose the player choice if more than one branch is legal, and execute the selected effect with correct source attribution.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` records `EffectContext::refire_effect_from_permanent(...)` and DSL `refire_effect` as implemented for constrained permanent-sourced `when_digivolving` re-firing. `BT22-040.yaml` combines `on_any_deletion`, deleted-object `event_target_*` predicates, `event_permanent_is_source: false`, and `refire_effect` to prove "your other Digimon" self-refire without broad over-triggering. `BT22-042.yaml` proves the same refire route against a real When Digivolving body with an optional play branch and mandatory DP tail. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_040 --nocapture` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_042 --nocapture`.
- **First test:** Closed by `bt22_042_other_own_digimon_deletion_may_refire_when_digivolving_effect`.
- **Implementation hint:** Add a reusable effect-selection helper that can run a source card's `[When Digivolving]` effects from a non-digivolve trigger while preserving `source_permanent`, `source_card`, once-per-turn accounting, and pending-selection ordering.

### PUPPETS-G003: Effect-Played Permanent Provenance and Scheduled Turn-End Cleanup

- **Type:** `engine-gap`
- **Status:** open
- **Blocks:** `EX11-022` Karakurumon, `EX11-061` Mirai Kinosaki, and any Puppet effect that says to delete "the Digimon this effect played" at turn end.
- **Effect text examples:** "At turn end, delete the Digimon this effect played."
- **Why it matters:** The engine must remember exactly which permanent was played by a specific effect, then delete that permanent later if it is still present. Index-based battle-area references are not enough because other deletes/plays can shift slots.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` includes the reusable provenance/scheduled cleanup gap for "delete the Digimon this effect played".
- **First test:** Resolve `EX11-022` to play a 4000 DP or less Puppet from hand or trash, shift battle-area indices before turn end, and assert only the effect-played permanent is deleted.
- **Implementation hint:** Store a stable provenance key on effect-played permanents or schedule a cleanup keyed by stable `CardSource` identity rather than battle-area index.

### PUPPETS-G004: Event-Gated Delay Windows for Puppet Options

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** partially resolved
- **Blocks:** `BT22-098` Unique Emblem: Fable Waltz, `P-229` Unique Emblem: Narrative Ronde.
- **Effect text:** `BT22-098` delays when Arisa suspends; `P-229` delays when Mirai Kinosaki is played.
- **Why it matters:** These options are important Puppet consistency tools and must place into battle, watch later events, then expose the delayed digivolve choice through pending selections.
- **Evidence:** `qa/dsl-vocab-gaps.md` records this as partially resolved for `BT22-098`'s `on_suspend` slice. Batch 7 production YAML/tests now cover the Arisa-suspend event-gated Delay body for `BT22-098`; `P-229` remains blocked while `on_ally_played` is virtual/skipped and its option placement/full Delay route remain open.
- **First test:** Play `P-229`, play `Mirai Kinosaki` on a later turn, and assert the delayed option can be trashed to offer a level 6 or lower `LIBERATOR` digivolution from hand with cost reduced by 3.
- **Implementation hint:** Lower `on_ally_played` to a real entered-field event predicate or a more explicit `on_enter_field_anyone` condition, then reuse the event-gated Delay lifecycle.

### PUPPETS-G005: Effect-Initiated Play/Digivolve Event Context and "By This Effect" Filters

- **Type:** `engine-gap` / `test-gap`
- **Status:** open
- **Blocks:** `EX9-067`, `EX11-061`, `ST19-14`, `BT22-088`, and related Tamer observer loops.
- **Effect text examples:** "When any of your Digimon digivolve into a [Puppet] trait Digimon..." and "When effects play one of your Tokens or a Digimon with the [Puppet] trait..."
- **Why it matters:** Puppet Tamers care whether a Digimon was played or digivolved by effects, and they often immediately play or grant keywords to the entered/digivolved permanent. Normal hand play/digivolve event context has coverage, but effect-initiated paths need to prove the same context and provenance.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` notes follow-up paths remain open for effect-created permanents, token play, play-from-trash context, and effect-initiated digivolve context unless separately tested.
- **First test:** Trigger `EX11-061` by effect-digivolving into a Puppet and assert Mirai can suspend to play a level 3 Puppet from hand, then schedule the correct turn-end cleanup for that played card.
- **Implementation hint:** Thread `PlaySource::ByEffect` / effect source identity into `OnEnterFieldAnyone` and `OnDigivolve` trigger context for every effect-created play/digivolve path.

### PUPPETS-G006: Security End-of-Battle Play for Puppet Digimon

- **Type:** `engine-gap` / `test-gap`
- **Status:** partially resolved for `P-165` card-level coverage; keep open only if a later card proves a stricter end-of-battle security subwindow is missing.
- **Blocks:** future cards whose security text requires a distinct end-of-battle subwindow beyond the existing `on_security` + `play_from_security` flow.
- **Effect text:** "[Security] At the end of the battle, play this card without paying the cost."
- **Why it matters:** The timing is not just normal security reveal. The card must survive the security battle window long enough to play itself at the correct sub-timing, fire On Play, and avoid being trashed by the normal security disposition.
- **Evidence:** Batch 3 added `P-165` production YAML and a focused security-play test that reveals `P-165` from security during a player attack and asserts it enters the owner's battle area. The remaining `P-165` blocker is no longer security play; it is `PUPPETS-G016` token cleanup.
- **First test:** For any future stricter case, reveal the card from security in battle, finish the battle, assert it enters battle area without paying cost, then assert any required On Play follow-up resolves at the right sub-timing.
- **Implementation hint:** Reuse the existing security pending-card disposition machinery where possible, but ensure the action/mask path does not collapse optional choices or skip On Play.

### PUPPETS-G007: Puppet Aura Package Needs Card-Level Regression Coverage

- **Type:** `test-gap`
- **Status:** open
- **Blocks:** confidence for `EX9-033`, `EX11-023`, `ST19-14`, and related aura/keyword packages.
- **Effect text examples:** "All of your Tokens and [Puppet] trait Digimon gain <Alliance> and <Blocker>" and "When effects play one of your Tokens or Puppet Digimon... gains <Rush>."
- **Why it matters:** The engine has general Alliance, Blocker, Rush, keyword grants, and lowest-level predicates, but the exact Puppet combination is broad and action-mask-sensitive.
- **Evidence:** Combat tests cover Alliance and Overclock generically; production Puppet cards do not yet have end-to-end YAML behavioral tests.
- **First test:** Author `EX9-033` YAML, place Puppet and Token allies, assert Alliance and Blocker masks appear only for legal targets, then delete another Digimon and assert the once-per-turn lowest-level delete plus end-turn trash-play choices behave correctly.
- **Implementation hint:** Keep this as a card-level regression suite rather than a new engine primitive unless the test uncovers missing mask or aura scope behavior.

### PUPPETS-G008: Inherited Opponent Security Digimon DP Aura DSL Bridge

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `ST19-03`, `EX7-024`, and any inherited effect that modifies all opponent security Digimon DP during your turn.
- **Effect text:** "[Your Turn] All of your opponent's security Digimon get -3000 DP."
- **Why it matters:** The Rust engine has an `EffectBuilder::applies_to_opponent_security_dp` targeting hook, but production YAML has no declarative vocabulary that lowers an inherited aura into that builder path.
- **Evidence:** `ST19-03` can faithfully ship its On Play reveal search, but its inherited text is omitted rather than approximated. The reusable vocabulary gap is tracked in `qa/dsl-vocab-gaps.md` as `G-OPPONENT-SECURITY-DP-AURA`.
- **First test:** Stack `ST19-03` under an attacking Digimon, reveal an opponent security Digimon during your turn, and assert the security Digimon's battle DP is reduced by 3000 without affecting battle-area Digimon.
- **Implementation hint:** Add an inherited aura DSL shape that lowers to `EffectBuilder::applies_to_opponent_security_dp()` plus a turn-scoped DP modifier.

### PUPPETS-G009: Standard Delay Activation as a Visible Main-Phase Action

- **Type:** `engine-gap`
- **Status:** open
- **Blocks:** full no-approximations readiness for `P-037`, `P-105`, `LM-035`, `LM-037`, `LM-054`, and other standard Memory Boost/Training/Scramble-style Options. `BT22-098` also uses the same visible delayed-option placement/activation surface for its full Main/Security mirror.
- **Effect text examples:** `[Main] <Delay> (By trashing this card after the placing turn, activate the effect below.) Gain 2 memory.`
- **Why it matters:** The current Rust Delay primitive parks Options on the battle area and fires `DelayEffect` through a scheduled turn/start/event scan. That supports placement and a deterministic delayed body, but standard `<Delay>` is a player-visible `[Main]` activation after the placing turn: the player chooses whether and when to trash the Option. Auto-firing the body hides that choice from the action mask.
- **Evidence:** Batch 1 added `P-037`, `LM-035`, and `LM-037` against the current `kind: delay` lifecycle and marked them `PARTIAL` in `qa/qa-reports/validated_cards_dsl.json`. Batch 7 added `P-105` and `LM-054` with the same scheduled Delay workaround and explicit ignored tests for later Main-phase activation. The batches also tightened mandatory reveal picks so PASS is not legal when eligible cards exist.
- **First test:** Place a standard Memory Boost as `OptionState::Delayed`, advance past the placing turn, enter its controller's Main phase, and assert the action mask exposes a field-effect activation for the delayed Option while PASS remains legal. Choosing the activation must trash the Option as a cost, then run the Delay body; declining must leave the Option on field.
- **Implementation hint:** Reuse the existing field-effect action range for battle-area Options. Standard `kind: delay` likely needs an activation-mode distinct from event-gated/start/end scheduled Delay triggers so `classify_option_subtype` can still park the Option while the body is gated by a post-placement Main action.

### PUPPETS-G010: Trash Selected or Bottom Security From a Visible Choice

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `BT15-003` and any inherited/triggered cost that lets the player choose top or bottom security to trash.
- **Effect text:** "By trashing the top or bottom card of your security stack, gain 1 memory."
- **Why it matters:** The visible top-vs-bottom choice can be represented as an effect-choice prompt, and `trash_top_security` covers the top branch. The bottom branch currently needs either a native `trash_bottom_security`/`trash_selected_security` DSL step or a selected-security binding consumer; using raw Rust from `_examples/BT15-003.yaml` is not acceptable for production batch implementation.
- **Evidence:** Batch 2 added `bt15_003` behavioral tests. The bottom-branch test is ignored pending this gap; the top branch, visible choice labels, and no-security gate pass against the current embedded example fixture.
- **First test:** Put two cards in own security, trigger `BT15-003`, choose "Trash bottom security", and assert the first security entry moves to trash and memory increases by 1 while the top card remains.
- **Implementation hint:** Reuse `select_security`'s existing security-index binding if practical, then add a DSL step that consumes the bound index and routes the card through the same trash/move hooks as top-security trash.

### PUPPETS-G011: `OnAnyDeletion` Deleted Permanent/Card Event Context

- **Type:** `engine-gap`
- **Status:** closed 2026-05-08
- **Blocks:** `BT22-002`, `EX11-023`, `EX11-060`, and related observers that care exactly which Token, Digimon, or trait object was deleted.
- **Effect text:** "[Your Turn] [Once Per Turn] When any of your Tokens or other [Puppet] trait Digimon are deleted, Draw 1."
- **Why it matters:** The existing `OnAnyDeletion` observer can fan out, but the worker test showed `event_target_*` predicates do not see the deleted permanent/card. Without deleted-object context, YAML cannot faithfully distinguish own Tokens, own other Puppet Digimon, own non-Puppet Digimon, opponent Puppet Digimon, or the carrier itself.
- **Resolution:** `BT22-002.yaml` now authors the inherited `on_any_deletion` observer with `event_target_owner`, `event_target_kind`, `event_target_trait_has`, and `event_permanent_is_source: false` for the "other Puppet" branch. Runtime event predicates read `DeletedObjectSnapshot.card_kind` / `traits` first, so deleted Tokens remain matchable after leaving the battle area.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_002` covers own Token, own other Puppet, own non-Puppet, opponent Puppet, carrier deletion, and once-per-turn behavior.

### PUPPETS-G012: On Deletion Predicate for Deletion Cause

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `EX11-020` and any On Deletion text gated on "deleted other than in battle" or similar cause restrictions.
- **Effect text:** "[On Deletion] If deleted other than in battle, you may play 1 [Shoemon] trait from your hand without paying the cost."
- **Why it matters:** The engine exposes `EffectContext::deletion_cause()` during OnDeletion observer resolution, but the YAML predicate vocabulary has no leaf for it. `replacement_cause` is replacement-context only and does not inspect deletion observer cause, so using it would over-fire on battle deletion.
- **Evidence:** Batch 2 attempted the predicate path, then reverted it after `ex11_020_on_deletion_does_not_fire_when_deleted_in_battle` failed. The On Deletion tests remain ignored and the production YAML ships only the inherited attack-cancel slice.
- **First test:** Add a DSL predicate such as `deletion_cause_not: battle`, delete `EX11-020` with `ReplacementCause::OpponentEffect`, `OwnEffect`, and `Battle`, and assert only non-battle causes offer the Shoemon-trait free-play prompt.
- **Implementation hint:** Add `deletion_cause` / `deletion_cause_not` predicate leaves that evaluate `EffectReadContext::deletion_cause()` and are valid for triggered OnDeletion/OnAnyDeletion effects, separate from replacement-context predicates.

### PUPPETS-G013: Source-Scoped Digivolve-Into-Trait Cost Reduction

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** closed 2026-05-06
- **Blocks:** `EX7-024` and sibling "when this Digimon would digivolve into [trait]" inherited/top-card cost reducers.
- **Effect text:** "[Your Turn] When this Digimon would digivolve into a Digimon card with the [Puppet] trait, reduce the digivolution cost by 1."
- **Why it matters:** This is not an alternate printed digivolution route for `EX7-024` itself and not a play-cost reduction. It must apply only while this card is the live source/top card that is about to be used as the digivolution base, only on your turn, and only when the target evolution card has the required trait.
- **Evidence:** `qa/dsl-vocab-gaps.md` already tracks the sibling `BT23-005` shape as missing `when_this_digivolves_into` plus target-trait threading through `BeforePayCost`.
- **First test:** Put `EX7-024` in battle, offer a Puppet evolution and a non-Puppet evolution from hand, and assert only the Puppet target's cost is reduced by 1 during its controller's turn.
- **Implementation hint:** Extend cost-reduction clauses with a source-scoped digivolve trigger plus target-card predicate payload, then consult that hook from the normal digivolve cost path.

### PUPPETS-G014: Filtered Hand-or-Trash Security Free-Play

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `ST19-08`, `BT22-098`, and other security effects that play a filtered card from hand or trash.
- **Effect text:** "[Security] You may play 1 [LIBERATOR] card with play cost 4 or less from your hand or trash without paying the cost."
- **Why it matters:** A faithful implementation must expose one visible choice over both zones, enforce the trait and play-cost filter, preserve which zone the chosen card came from, and move/play the chosen card from that origin. Binding only a `CardHandle` without origin loses enough information to misroute the selected card.
- **Evidence:** The generic union-zone selection family is documented in `docs/RUST_ENGINE_GAPS.md`, but `ST19-08` needs the card-authoring proof that filters and origin are enforced for security free-play. `BT22-098` Batch 7 therefore covers only the hand-origin Shoemon/Arisa slice for Main/Security and keeps exact hand-or-trash play ignored.
- **First test:** Put one eligible LIBERATOR in hand, one in trash, plus ineligible cards in both zones, reveal `ST19-08` from security, and assert only eligible union-zone actions are legal and the chosen origin is consumed.
- **Implementation hint:** Preserve origin in the union-zone binding and route the consumer step to `play_from_hand_free` or `play_from_trash_free` based on that origin.

### PUPPETS-G015: Count-Threshold Branches for Modifier Amounts

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `ST19-11` and any effect whose amount increases when a board-count threshold is met.
- **Effect text:** "1 of your opponent's Digimon gets -3000 DP for the turn. If there are 3 or more Digimon, increase the DP reduction by -3000."
- **Why it matters:** The first -3000 is target-specific and implemented. The second -3000 must be conditional on a total Digimon count across battle areas. Current aggregate predicates have known evaluation gaps in triggered conditions, so encoding the branch would over-fire if the count predicate evaluates true by default.
- **Evidence:** `qa/archetype-qa/engine-gaps.md` documents unevaluated `count_gte` / `count_lte` aggregate predicates. `ST19-11` keeps the extra branch ignored until this is proven by a card-level regression.
- **First test:** With two total battle-area Digimon, resolve `ST19-11` and assert -3000. With three total Digimon, resolve it and assert -6000.
- **Implementation hint:** Either fix subjectless count aggregate evaluation for battle-area permanents or add a small formula/conditional amount shape that can safely add the extra modifier only at the threshold.

### PUPPETS-G016: Token Handle Provenance for "That Token" Cleanup

- **Type:** `engine-gap`
- **Status:** open
- **Blocks:** `P-165` and any card that creates a token and later refers to "that token."
- **Effect text:** "At the end of your opponent's turn, delete that token."
- **Why it matters:** `play_token` can create the Familiar Token, but the YAML process cannot bind the resulting permanent handle or schedule an end-of-opponent-turn cleanup keyed to that exact token. A broad cleanup of any Familiar Token would delete the wrong permanent if multiple effects create tokens.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` tracks effect-played permanent provenance and scheduled cleanup for non-token permanents; `P-165` surfaces the token-specific sibling where the newly created token handle must be bound.
- **First test:** Resolve `P-165`, create another Familiar Token from a separate effect, advance to the opponent's turn end, and assert only the token created by the `P-165` effect is deleted.
- **Implementation hint:** Let `play_token` bind its returned `PermanentHandle`, and add a scheduled cleanup step that consumes that binding at the printed turn-end timing.

### PUPPETS-G017: Optional Sub-Effect Followed by Mandatory Tail

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** partially resolved 2026-05-08 for nested-selection outer-tail continuation.
- **Blocks:** no longer blocks `BT22-042`; still blocks `LM-029`, `P-156`, and other effects shaped as "You may do X. Then, do mandatory Y." that need card-shaped YAML/test adoption.
- **Effect text:** "[When Digivolving] You may play 1 level 4 or lower [Puppet] trait Digimon card from your hand without paying the cost. Then, to 1 of your opponent's Digimon, give -3000 DP until their turn ends for each of your Digimon."
- **Why it matters:** Declining or being unable to perform the optional free-play must not skip the mandatory DP reduction tail. A simple optional `select_hand` can terminate the process on PASS, while a hand-authored branch can still fail to install the hand selection in this When Digivolving shape. The tail also needs to preserve ordering, target selection, count only your Digimon, and expire at opponent turn end.
- **Evidence:** `BT22-042` now uses an explicit `select_effect_choice` branch for the optional play, and `drain_dsl_outer_tail` re-wraps any pending inner selection instead of draining the saved tail immediately. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt22_042 --nocapture`.
- **First test:** Closed for `BT22-042` by `bt22_042_when_digivolving_plays_puppet_then_debuffs_by_own_digimon_count` and `bt22_042_declining_free_play_still_applies_scaled_dp_reduction`. Keep the same fixture shape for `LM-029` / `P-156` security tails.
- **Implementation hint:** Prefer explicit visible branch choices when the optional sub-effect has follow-up selections. The engine-side outer-tail rewrap keeps mandatory sibling tails parked behind any nested pending selection.

### PUPPETS-G018: Costed Self-Digivolve Stable Source Binding

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `EX9-032` and other effects that pay a field cost, then digivolve the resolving source card.
- **Effect text:** `EX9-032` Karakurumon: "[On Play] [When Digivolving] By deleting 1 of your Tokens or other [Puppet] trait Digimon, this Digimon may digivolve into a [Puppet] trait Digimon card in your hand without paying the cost."
- **Why it matters:** The legal cost body can be below the resolving permanent in the battle area. Deleting that body shifts battle-area indices before a later `target: self` digivolve step runs, so an index-bound source can digivolve the wrong permanent or lose the original source. The cost preflight also needs to distinguish "legal cost body exists" from "the source itself is the only Puppet".
- **Evidence:** Batch 5 removed the attempted `EX9-032` active YAML after the focused tests failed for costed self-digivolve and no-cost-body masking. The ignored tests carry `G-COSTED-SELF-DIGIVOLVE-STABLE-SOURCE` and `G-COSTED-SELF-DIGIVOLVE-PREFLIGHT`.
- **First test:** Put `EX9-032` above another own Puppet at a lower battle-area index, trigger the effect, choose the lower Puppet as the cost, and assert the original `EX9-032` stack, not the shifted permanent at the old index, digivolves into the selected hand Puppet.
- **Implementation hint:** Bind the resolving source by a stable permanent handle before paying costs, let cost preflight evaluate legal Token/Puppet bodies that exclude the source, and consume that stable source binding in `effect_initiated_digivolve`.

### PUPPETS-G019: Inherited Token/Puppet Leave-Prevention Replacement Dispatch

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** inherited leave-prevention on `EX9-032`, `BT22-036`, `EX11-022`, `EX7-027`, and `ST19-11` style Puppet bodies.
- **Effect text:** "[All Turns] [Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, prevent it from leaving."
- **Why it matters:** The reusable replacement framework handles face-up replacement sources, but these Puppet cards need the inherited effect to dispatch from a buried source under the threatened stack, expose a player-visible cost selection, exclude the protected carrier/source from the cost, and cancel only the original leave event.
- **Evidence:** Batch 5 tried the inherited replacement on `BT22-036` and `EX11-022`, then removed it after the positive tests failed to produce a pending selection from the inherited source. The ignored tests carry `G-INHERITED-REPLACEMENT-DISPATCH`.
- **First test:** Build a Puppet stack with one of these cards as a source and another own Familiar Token in battle. Attempt to remove the stack by an opponent effect, assert a pending replacement-cost selection appears, delete the Token, and assert the stack remains with once-per-turn accounting consumed.
- **Implementation hint:** Extend leave-field replacement dispatch to scan inherited source effects under the threatened permanent, build `EffectContext` from the inherited source while keeping the replacement subject as the carrier, and keep Puppet/Token cost-body predicates binding-aware.

### PUPPETS-G020: Hand-Main Trash-to-Source Hand-Card Digivolve Chain

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT22-036` Chaperomon.
- **Effect text:** `BT22-036` hand text: "[Hand] [Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon] from your trash as any of your [Shoemon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."
- **Why it matters:** This is an activated hand effect with an Arisa board condition, an exact trash-card cost, a player-visible choice among your Shoemon stacks, bottom-source placement, and digivolving that chosen Shoemon into the resolving hand card. No part can be hidden or auto-picked because the trash cost and Shoemon target are player-visible choices.
- **Evidence:** Batch 5 left the `BT22-036` hand-main tests ignored with `G-HAND-MAIN-TRASH-PREFLIGHT` and `G-HAND-MAIN-SELF-DIGIVOLVE`.
- **First test:** Put `BT22-036` in hand, control `Arisa Kinosaki`, put `ShoeShoemon` in trash, and control a `Shoemon`. Activate the hand effect, assert the ShoeShoemon trash card is selected and moved as that Shoemon's bottom source, then assert the selected Shoemon digivolves into `BT22-036` from hand for cost 3 ignoring requirements.
- **Implementation hint:** Add hand-main effect activation/mask preflight for card-specific trash costs, bind exact trash `CardSource`s for bottom-source placement, and allow the chosen field Shoemon to digivolve into the resolving hand card.

### PUPPETS-G021: Hidden-Zone DP Predicate for Hand/Trash Free-Play

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `EX11-022` Karakurumon and any hand-or-trash selection filtered by printed DP.
- **Effect text:** `EX11-022`: "[On Play] [When Digivolving] You may play 1 [Puppet] trait Digimon card with 4000 DP or less from your hand or trash without paying the cost. At turn end, delete the Digimon this effect played."
- **Why it matters:** The selection must include eligible hand and trash cards while excluding Puppet cards above 4000 DP. Broad trait-only selection would expose illegal choices; hand-only or trash-only selection would hide legal choices.
- **Evidence:** Batch 5 left `EX11-022` free-play tests ignored with `G-HAND-TRASH-CARD-DP-FILTER`; the cleanup rider is separately covered by `PUPPETS-G003`.
- **First test:** Put one 4000 DP Puppet and one higher-DP Puppet across hand/trash, trigger `EX11-022`, and assert only the legal DP<=4000 cards are offered in the pending union-zone selection before the selected card is played for free.
- **Implementation hint:** Extend union-zone card predicates/lowering to inspect hidden-zone `CardData.dp` from `CardSource`/card ID metadata, then reuse the existing origin-preserving hand/trash play consumer.

### PUPPETS-G022: Deletion Observer Suspend-This-Tamer Cost With Overclock Cause Branch

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** closed 2026-05-06
- **Blocks:** `EX11-060` Arisa Kinosaki and any Tamer that reacts to a deleted Token/Puppet, suspends itself as the visible cost, draws, then branches when the deletion was caused by Overclock.
- **Effect text:** `EX11-060`: "[All Turns] When any of your Tokens or [Puppet] trait Digimon are deleted, by suspending this Tamer, <Draw 1>. If this effect was activated by <Overclock>, you may play 1 level 4 or lower [Puppet] trait Digimon card from your hand without paying the cost."
- **Why it matters:** The trigger must read the deleted object, not the observing Tamer, and the "by suspending this Tamer" cost must be a player-visible legal-cost gate. The Overclock rider must know the deletion cause/source so normal deletes draw without exposing the free-play branch, while Overclock deletes expose the optional level 4 or lower Puppet hand play.
- **Resolution:** `EX11-060.yaml` now uses the deleted-object event payload and `event_cause: overclock` to distinguish ordinary Token/Puppet deletions from Overclock cost deletions. The Arisa activation is surfaced as an explicit `select_effect_choice`; accepting suspends the source Tamer, draws 1, and only the Overclock branch exposes the optional level 4 or lower Puppet hand-play prompt.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex11_060`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_context`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch`.

### PUPPETS-G023: Event Predicates Plus Source-Bound Suspend Cost

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT13-101` Miki Kurosaki & Megumi Shirakawa, `P-136` Arisa Kinosaki.
- **Effect text:** `BT13-101`: "[All Turns] When you play a 2-color black/yellow Digimon, by suspending this Tamer, <Draw 1> and gain 1 memory." `P-136`: "[Your Turn] [Once Per Turn] When one of your Digimon digivolves into a Digimon with the [Puppet] trait, by suspending this Tamer, gain 1 memory."
- **Why it matters:** The observer must inspect the triggering card/permanent rather than the observing Tamer, and it must expose the "by suspending this Tamer" activation cost as a legal preflight. An already-suspended source must not install a prompt, and accepting the trigger must pay the cost before the draw/memory body.
- **Evidence:** Batch 8 shipped `BT13-101` with On Play and Security slices only. Batch 11 shipped `P-136` with On Play and Security slices only. Their observer tests are ignored under this gap because event-card/event-target predicates alone are insufficient without the source-bound suspend-cost surface.
- **First test:** Control unsuspended `BT13-101`, play an exact black/yellow two-color Digimon, assert the mask exposes an optional activation, accept it, and verify this exact Tamer suspends, the controller draws 1, and memory increases by 1. Repeat with `P-136`, a Puppet digivolve by its controller during their turn, and assert this exact Tamer suspends and gains 1 memory.
- **Implementation hint:** Add event-card predicates such as `event_card_color_only: [black, yellow]` / `event_card_color_count: 2`, event-target owner/trait predicates for digivolve observers, and pair them with the generic triggered activation-cost hook documented in `docs/RUST_ENGINE_GAPS.md`.

### PUPPETS-G024: Narrow Protection From Opponent DP Reduction and De-Digivolve

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT16-055` Namakemon.
- **Effect text:** "While you have 3 or more security cards, this Digimon isn't affected by your opponent's DP reduction effects and can't be de-digivolved by their effects."
- **Why it matters:** This is not broad immunity. It protects only against two effect categories, only from the opponent, and only while the controller has 3 or more security. Modeling it as `CannotBeAffected` would over-block legal effects; omitting the source/opponent/category gates would produce illegal protection.
- **Evidence:** Batch 8 covers the low-security Blocker/Reboot branch, then leaves this high-security branch omitted with ignored tests.
- **First test:** With 3 security, target `BT16-055` with opponent DP reduction and De-Digivolve effects and assert both are ignored; then repeat with 2 security and assert both effects apply.
- **Implementation hint:** Add category-scoped protection modifiers for DP reduction and De-Digivolve with opponent-source and live security-count predicates.

### PUPPETS-G025: Rules-Text Contains Predicate for Inherited Carrier Aura

- **Type:** `dsl-gap`
- **Status:** open
- **Blocks:** `BT16-055` Namakemon inherited effect.
- **Effect text:** "[Your Turn] While this Digimon has [Pulsemon] in its text, it gets +1000 DP."
- **Why it matters:** The predicate is over the carrier stack's printed rules text, not card name, traits, or colors. A name-based approximation would miss non-Pulsemon cards with Pulsemon in text and could include cards that only have Pulsemon in their name.
- **Evidence:** Batch 8 ships the card without the inherited aura and records the text-predicate blocker in validated DSL notes.
- **First test:** Stack `BT16-055` under a Digimon whose printed text includes `[Pulsemon]` and assert +1000 DP on your turn, then stack it under a Digimon without that text and assert no aura.
- **Implementation hint:** Add a predicate that can inspect the carrier's top-card and/or stack printed text during inherited effect evaluation, preserving inherited-source context.

### PUPPETS-G026: Trash-Resident Observer and Effect Digivolve From Trash

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT20-084` Sistermon Ciel (Awakened).
- **Effect text:** "[Trash] [All Turns] When any of your Digimon are played, 1 of your [Sistermon Ciel]s may digivolve into this card without paying the cost."
- **Resolution:** `EffectTiming::OnAllyPlayed` scans the playing player's battle-area observers and top-level trash observers. DSL `when: on_ally_played` lowers to that timing, and `BT20-084.yaml` uses `effect_initiated_digivolve` with `source: self` to consume the resolving trash card after the optional Sistermon Ciel target choice.
- **Evidence:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- trash_resident_on_ally_played_observer_sees_played_subject_once`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_084`.

### PUPPETS-G027: Move Top Stacked Card to Top Security

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT20-084` Sistermon Ciel (Awakened).
- **Effect text:** "[End of All Turns] Place this Digimon's top stacked card as the top security card."
- **Why it matters:** The effect extracts the top card of the Digimon's stack and places it on security. If that top card is also the active top card, the engine must keep the remaining permanent state legal or remove an empty battle-area object without firing unrelated deletion hooks.
- **Evidence:** This is a sibling/extension of the reusable `pop_top_source`/stack-to-security gap in `docs/RUST_ENGINE_GAPS.md`, but `BT20-084` needs the active top stacked card rather than a selected digivolution source.
- **First test:** Resolve End of All Turns with `BT20-084` on a stack and assert the top stacked card moves to top security while the battle-area stack remains legal.
- **Implementation hint:** Add a curated stack-extraction API for top stacked cards, then route the returned card through the security-top placement primitive with correct ownership and cleanup semantics.

### PUPPETS-G028: Optional Triggered Return-Self Cost Before Chained Free-Play Branches

- **Type:** hybrid `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT22-088` Arisa Kinosaki.
- **Effect text:** "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Arisa Kinosaki] with a different card number in your hand without paying the cost, or play 1 [Shoemon] from your hand or trash without paying the cost."
- **Why it matters:** The source Tamer is the activation cost, and after paying it the player chooses between distinct free-play branches with their own legal target sets. Auto-returning the Tamer or auto-picking a branch would hide player-visible decisions.
- **Evidence:** Batch 8 ships only the Security play-self slice for `BT22-088`; the Start of Main tests remain ignored pending this cost and branch surface.
- **First test:** Control `BT22-088`, enter Start of Main with both legal branch targets, assert the activation choice is visible, accept, verify this exact Tamer returns to bottom deck, then choose the Arisa or Shoemon branch through the mask.
- **Implementation hint:** Generalize triggered activation costs to zone-moving the source permanent, then chain into an in-effect branch selector with hand/trash origin-preserving play consumers.

### PUPPETS-G029: Self-Scoped OnSuspend Event Predicate

- **Type:** `dsl-gap`
- **Status:** closed 2026-05-08
- **Blocks:** `BT23-077` Sistermon Ciel.
- **Effect text:** "[All Turns] When this Digimon suspends, <De-Digivolve 1> 1 of your opponent's Digimon."
- **Why it matters:** The observer must fire only when the source permanent itself suspends. Current `OnSuspend` fan-out can observe that something suspended, but production YAML needs a reusable predicate that proves the event permanent is the same permanent that owns the effect.
- **Evidence:** `event_permanent_is_source: true` now compares `TriggerContext.event_permanent` with the observer's source permanent. `BT23-077` is authored with the printed self-suspend De-Digivolve clause, and its behavioral test proves another own permanent suspending does not trigger the prompt while self-suspending does.
- **First test:** Control `BT23-077` and another own Digimon. Suspend the other Digimon and assert no prompt; suspend `BT23-077` and assert the opponent De-Digivolve target prompt appears.
- **Verification:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- event_permanent_is_source` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_077`.

### PUPPETS-G030: Effect Play With Played-Digimon On Play Suppression

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `BT5-106` Demonic Disaster and source/security play effects that say the played Digimon's `[On Play]` effects do not activate.
- **Effect text:** "[Security] You may play 1 level 3 purple Digimon card from your trash without paying its memory cost. Any [On Play] effects on Digimon played with this effect don't activate."
- **Why it matters:** Ordinary play-from-trash support is not enough: the selected Digimon enters play, but its own On Play clauses must be suppressed only for this effect-played permanent. Broadly disabling all On Play effects or omitting the play source provenance would misfire elsewhere.
- **Evidence:** Batch 9 ships `BT5-106`'s Main effect and leaves Security tests ignored under this gap. Existing reusable zone-play work fires normal On Play effects, so this needs a distinct suppression flag.
- **First test:** Reveal `BT5-106` in security with a level 3 purple Digimon in trash whose On Play would mutate memory or board state. Select and play it, assert it enters play, and assert its On Play effect did not fire.
- **Implementation hint:** Add an effect-play provenance flag such as `suppress_on_play: true` to play helpers and DSL consumers, then make On Play enqueue skip only the played permanent's On Play effects for that play event.

### PUPPETS-G031: End-of-Attack Self-Delete, Opponent Delete, Recovery, and Hatch Chain

- **Type:** `engine-gap` / `dsl-gap`
- **Status:** open
- **Blocks:** `EX4-074` ShineGreymon: Ruin Mode.
- **Effect text:** "[End of Attack] Delete this Digimon and 1 of your opponent's Digimon, and <Recovery +1 (Deck)>. Then, if you have a Tamer in play, hatch 1 Digi-Egg card to an empty space in your breeding area."
- **Why it matters:** The chain mixes source self-deletion, a player-visible opponent target, mandatory Recovery, and a conditional hatch tail. The engine must not lose the resolving source's queued continuation when it deletes itself, and the hatch branch must not appear as a no-op when the Tamer/breeding conditions are unmet.
- **Evidence:** Batch 10 ships only `EX4-074` metadata/digivolve paths. The End of Attack test remains ignored under this gap, and `docs/RUST_ENGINE_GAPS.md` records the reusable engine shape.
- **First test:** Attack with `EX4-074`, resolve the End of Attack trigger with a Tamer and an opponent Digimon in play, and assert self-delete, selected opponent delete, Recovery +1, and conditional hatch all resolve in order.
- **Implementation hint:** Add source-stable self-delete continuation support, explicit no-target continuation semantics for mandatory target selections, and a guarded hatch step keyed on "have a Tamer in play" plus empty breeding availability.

### PUPPETS-G032: Counter Blast DNA Digivolve From Hand

- **Type:** `engine-gap`
- **Status:** open
- **Blocks:** `EX6-011` RagnaLoardmon.
- **Effect text:** "[Hand] [Counter] <Blast DNA Digivolve ([Durandamon] + [BryweLudramon])>."
- **Why it matters:** This is a defender-side Counter-window activation that selects a specific field Digimon plus a specific hand material, then DNA digivolves into the hand card without paying cost. It must be exposed through the Counter action mask and pending-selection flow, not approximated as normal main-phase DNA.
- **Evidence:** Batch 10 implements the normal red+black Lv6 DNA route and DNA-origin resolution tail, but leaves the Counter activation ignored under this gap. `docs/RUST_ENGINE_GAPS.md` now anchors the reusable `G-COUNTER-BLAST-DNA-ACTIVATION` tracker.
- **First test:** During the opponent's attack, hold `EX6-011` with eligible `Durandamon`/`BryweLudramon` materials, assert the Counter action mask exposes Blast DNA, choose materials, and assert the card DNA digivolves with DNA-origin context.
- **Implementation hint:** Implement the Counter window and `prompt_blast_dna_digivolve` action surface from the existing `Counter window + <Blast Digivolve>` gap.

## Cross-Archetype Spec Tags

Use these tags when normalizing gaps across archetypes:

- `missing-production-yaml`
- `effect-refire`
- `effect-play-provenance`
- `scheduled-cleanup`
- `event-gated-delay`
- `on-ally-played`
- `effect-initiated-event-context`
- `security-end-of-battle`
- `aura-keyword-regression`
- `opponent-security-dp-aura`
- `standard-delay-main-activation`
- `selected-security-trash`
- `on-any-deletion-event-context`
- `deletion-cause-predicate`
- `source-scoped-cost-reduction`
- `filtered-union-zone-origin`
- `count-threshold-modifier`
- `token-provenance-cleanup`
- `optional-substep-mandatory-tail`
- `costed-self-digivolve-stable-source`
- `inherited-replacement-dispatch`
- `hand-main-self-digivolve`
- `hidden-zone-dp-filter`
- `overclock-cause-context`
- `suspend-this-tamer-cost`
- `event-card-color-predicate`
- `narrow-effect-category-protection`
- `rules-text-contains-predicate`
- `trash-resident-observer`
- `effect-digivolve-from-trash`
- `stacked-card-to-security`
- `return-self-cost`
- `event-permanent-is-source`
- `on-play-suppression`
- `end-of-attack-chain`
- `counter-blast-dna`

## Suggested Build Order

1. `EX9-032` costed self-digivolve stable-source regression.
2. Inherited Token/Puppet leave-prevention replacement dispatch regression using `BT22-036` or `EX11-022`.
3. `EX11-022` hidden-zone DP-filtered union-zone play plus effect-played cleanup once `PUPPETS-G003` is ready.
4. `BT22-036` hand-main trash-to-source hand-card digivolve chain.
5. `P-229` event-gated Delay test for Mirai-played trigger.
6. `EX11-060` deletion observer test for suspend-this-Tamer cost, Draw 1, and Overclock-only hand-play branch.
