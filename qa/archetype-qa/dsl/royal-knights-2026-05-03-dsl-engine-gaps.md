# Royal Knights Rust DSL/Engine Gap Rollup

Date: 2026-05-03

Assessment source: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `Royal Knights`, prioritizing the current competitive lists and recurring core cards. This is a Rust DSL/engine gap document, not the legacy Python-lane faithfulness report in `qa/archetype-qa/royal-knights.md`.

## Purpose

This document distills the Royal Knights audit into reusable DSL and engine gaps that should feed a future cross-archetype implementation spec. It intentionally separates:

- reusable engine or DSL primitives that unblock many archetypes;
- Royal Knights card-YAML/test work that should not become a generic gap;
- reusable gaps already closed by recent groups so future specs do not reopen them.

## Verdict

`blocked`

Royal Knights is not faithfully implementable as executable Rust YAML DSL yet. The archetype depends on King Drasil in breeding, delayed Royal Knight options, stack-source selection/play, leave-field replacement prevention, and immediate attack effects. Some reusable primitives are already closed, but several remaining blockers affect action masks or pending-selection fidelity and therefore cannot be papered over with hidden auto-selection.

## High-Frequency Card Pool

The current `Royal Knights` deck-library entry has 51 decklists. These cards recur often enough to treat as core or near-core for cross-archetype gap planning:

| Card | Role | Current Rust DSL status | Notes |
|---|---|---|---|
| `BT13-007` King Drasil_7D6 | Core breeding engine | example YAML only | Start-main breeding trigger still needs breeding-source dispatch. |
| `BT20-100` The Last Guardian | Core Delay/prevention option | production YAML partial | Search, option placement, and Security hand/trash branch implemented; Delay leave-prevention blocked by `RK-G003`. |
| `BT20-091` Cool Boy | Core tamer draw/memory and Omekamon play | production YAML partial | RK play/digivolve suspend-cost draw+memory and Security play implemented; opponent-turn Omekamon response blocked by `RK-G004`. |
| `BT20-102` Omnimon (X Antibody) | Core finisher | production YAML partial | Boardwipe uses raw Rust; end-turn attack without suspending is blocked. |
| `BT20-083` Omekamon | Core bridge/blocker | production YAML partial | Blocker and low-security free Omnimon X digivolve implemented; breeding-source and material-play behavior still blocked. |
| `BT23-054` Magnamon | Core RK body | production YAML partial | Blocker, draw, and selected return protection implemented; Armor Purge blocked by `RK-G003`. |
| `BT13-112` Omnimon | Core payoff | production YAML gap stub | Plays one each different-name RK from King Drasil sources and suppresses On Play. |
| `BT20-017` Jesmon | Core body/token pressure | production YAML gap stub | Needs token creation plus immediate may-attack/action flow. |
| `BT20-060` Alphamon: Ouryuken | Core/near-core ACE | production YAML partial | ACE metadata, routes, and DP reduction implemented; Blast DNA and DNA tail remain blocked; security-removed timing payload is wired, but this card still needs card-local authoring/tests. |
| `BT13-110` Royal Knights of the Purge | Core Delay option | production YAML partial | Draw and option placement implemented; King Drasil source placement/play blocked by `RK-G001`. |
| `BT23-035` Dynasmon | Common RK body | production YAML partial | Barrier and security-trash -6000 DP slice implemented; security-removed timing payload is wired, but the recovery branch still needs card-local authoring/tests. |
| `BT23-072` King Drasil_7D6 | Common support | production YAML gap stub | Hand main bottom-source placement; grants Rush/Raid/Reboot/Blocker. |
| `BT19-072` LordKnightmon | Common RK body | production YAML partial | Trash play of level 4 or lower Digimon implemented; attack retarget blocked. |

## 2026-05-05 Batch 1 Implementation Notes

Resolver input: `python code/tools/resolve_deck.py "Royal Knights"` wrote `qa/archetype-qa/royal-knights/deck_pool.json` with 51 decklists and 72 unique cards.

Implemented / audited in this batch:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `BT6-082` Sistermon Blanc | `IMPLEMENTED` | [All Turns] filtered Sistermon Blocker aura while own Huckmon/Royal Knight is in play; [On Play] Draw 1. | none |
| `BT13-093` Omekamon | `PARTIAL` | [On Play] Draw 1. | `RK-G001`: filtered King Drasil breeding target before placing a Royal Knight hand card as source. |
| `BT20-083` Omekamon | `PARTIAL` | Face-up Blocker; low-security optional free digivolve into hand [Omnimon (X Antibody)]. | `RK-G001` for On Deletion King Drasil target; `G-BREEDING-TRIGGER-DISPATCH` plus source/material play for inherited breeding trigger. |
| `EX11-071` Cool Boy | `PARTIAL` | [On Play] dual-bucket reveal top 3, add Omekamon/Omnimon X plus Royal Knight/LIBERATOR, bottom remainder. | `RK-G002`: return-this-Tamer activation cost feeding a reduced-cost hand play. |

Regression files:

- `code/digimon-engine/tests/cards_behavioral/bt6/bt6_082.rs`
- `code/digimon-engine/tests/cards_behavioral/bt13/bt13_093.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_083.rs`
- `code/digimon-engine/tests/cards_behavioral/ex11/ex11_071.rs`

## 2026-05-05 Batch 2-3 Implementation Notes

Implemented / audited in these batches:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `BT20-060` Alphamon: Ouryuken | `PARTIAL` | Printed metadata, ACE Overflow -5, standard Black Lv.6 route, Black + Yellow/Red Lv.6 DNA route, [On Play]/[When Digivolving] selected -15000 DP modifier. | Counter Blast DNA support, DNA-origin security trash/recovery tail, global security-removed memory observer. |
| `BT20-100` The Last Guardian | `PARTIAL` | [Main] dual-bucket reveal/search, bottom remainder, place self as Delay option; [Security] explicit hand/trash branch for optional Omekamon/Cool Boy play, then place self. | `RK-G003`: Delay leave-prevention for an Omnimon-name Digimon. |
| `BT23-054` Magnamon | `PARTIAL` | Blocker; standard and CS/Veemon digivolve routes; Draw 1 plus selected Royal Knight/CS return-to-hand/deck protection. | `RK-G003`: Armor Purge top-card trash replacement. |
| `BT23-058` Craniamon | `PARTIAL` | Reboot, Blocker, standard/CS digivolve routes, optional suspend-self replacement preventing one own Digimon/Tamer from leaving by opponent effects. | Self-scoped `on_suspend` predicate plus aggregate lowest play-cost delete-all. |
| `BT13-110` Royal Knights of the Purge | `PARTIAL` | [Main] Draw 1 then place self in battle area; [Security] place self in battle area. | `RK-G001`: hand-to-King-Drasil source placement and Delay play from breeding sources with On Play suppression/Rush. |
| `BT20-091` Cool Boy | `PARTIAL` | Your-turn Royal Knight play/digivolve observer with suspend self, Draw 1, gain 1 memory; [Security] play self. | `RK-G004`: opponent-turn would-leave Royal Knight response into optional Omekamon hand play. |
| `BT19-072` LordKnightmon | `PARTIAL` | [On Play]/[When Digivolving] optional level 4 or lower Digimon play from trash. | `G-ATTACK-RETARGET`: opponent-turn attack target switch to own Royal Knight. |

Regression files:

- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_060.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_100.rs`
- `code/digimon-engine/tests/cards_behavioral/bt23/bt23_054.rs`
- `code/digimon-engine/tests/cards_behavioral/bt23/bt23_058.rs`
- `code/digimon-engine/tests/cards_behavioral/bt13/bt13_110.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_091.rs`
- `code/digimon-engine/tests/cards_behavioral/bt19/bt19_072.rs`

## 2026-05-05 Batch 4-15 Implementation Notes

Pool coverage after the full batched pass: `qa/archetype-qa/royal-knights/deck_pool.json` resolves to 72 unique cards, and all 72 now have Rust DSL YAML entries under `code/digimon-engine/cards/`. The final 25 cards in this pass each have an active embedded-pack load test plus an ignored gap test for unsupported printed clauses.

Implemented / audited in these batches:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `AD1-004` Examon | `PARTIAL` | Raid and Piercing. | Token play from sources / printed multi-part action flow not covered in this RK pass. |
| `AD1-017` Dynasmon | `BLOCKED` | Load-only gap stub. | Top-or-bottom security-trash cost plus board-wide debuff. |
| `AD1-018` Gallantmon | `PARTIAL` | [On Play]/[When Digivolving] play-cost 3 or lower delete. | Inherited security-removed / retaliation shape. |
| `BT13-019` Gankoomon | `PARTIAL` | Blocker. | Union play from trash or breeding sources with name exclusions. |
| `BT13-030` UlforceVeedramon | `BLOCKED` | Load-only gap stub. | Counted source trash by Royal Knight/blue Tamer count; sourceless-opponent aura. |
| `BT13-040` Kentaurosmon | `PARTIAL` | Blocker and Recovery +1. | Security-search/placement and attack-prevention tail. |
| `BT13-075` Alphamon | `BLOCKED` | Load-only gap stub. | Source-placement cost tied to play-cost 10+ attack-player restriction; security-trash leave replacement. |
| `BT13-087` Dynasmon | `PARTIAL` | Reveal 4; add up to two Lucemon/Royal Knight cards; trash rest. | Another matching Digimon played observer and delete-all level 4 or lower. |
| `BT13-102` Keenan Crier | `PARTIAL` | [Security] play self. | Opponent hidden-hand choice; opponent-turn effect-play memory observer. |
| `BT13-111` Gallantmon | `PARTIAL` | Rush. | Combined-trash play-cost reduction; delete-result fallback. |
| `BT13-112` Omnimon | `BLOCKED` | Load-only gap stub. | Modal delete or different-name Royal Knight source play from breeding, King Drasil trash, Rush grant, On Play suppression. |
| `BT15-092` Revelation of Light | `BLOCKED` | Load-only gap stub. | Security-trash self-dispatch; security search/play; self-to-security top; security-Digimon debuff. |
| `BT17-077` Imperialdramon: Paladin Mode | `PARTIAL` | ACE metadata; trash all sources of all opponent Digimon. | Blast Digivolve; bulk trash-to-deck; returned-card memory binding; sourceless bottom-deck cost. |
| `BT19-093` Queen Device | `BLOCKED` | Load-only gap stub. | Option battle-area carrier lifecycle; negative color-bypass predicate; two-target security modifier. |
| `BT20-017` Jesmon | `BLOCKED` | Load-only gap stub. | Atho/Rene/Por token registration; other-Digimon-played delete/may-attack observer. |
| `BT20-021` Jesmon GX | `BLOCKED` | ACE metadata and standard route. | Union hand/trash source cost; source-DP compare; unsuspend; source-count security trash. |
| `BT20-045` Examon | `PARTIAL` | ACE metadata; Raid, Piercing, Blocker, Evade. | Blast DNA; DNA-gated highest-DP bottom-deck sweep; any-Digimon-suspend observer. |
| `BT20-056` Alphamon | `PARTIAL` | Barrier; [On Play]/[When Digivolving] Recovery +1. | During-attack breeding digivolve; inherited replacement; security-removed card-local observer test. |
| `BT22-025` UlforceVeedramon | `PARTIAL` | ACE metadata; [When Attacking][OPT] unsuspend self. | Blast Digivolve; modal lowest-level bottom-deck or blue Tamer play. |
| `BT22-041` Kentaurosmon | `PARTIAL` | Blocker, Barrier, optional yellow hand-to-top-security. | Total-security play-cost reduction; self-suspend security-trash unsuspend cost. |
| `BT22-052` Leopardmon | `PARTIAL` | ACE metadata; optional 5000 DP-or-lower hand play; own level 3+ Blocker grant. | Blast Digivolve; other-Digimon would-leave memory observer. |
| `BT23-013` Jesmon | `PARTIAL` | Rush and Alliance. | Atho/Rene/Por token or Sistermon union play with name exclusion; other-Digimon-played may-attack observer. |
| `BT23-035` Dynasmon | `PARTIAL` | Barrier; top-security cost into -6000 DP board debuff. | Security-removed Security A. +1 / recovery tail. |
| `BT23-047` Examon | `PARTIAL` | Piercing and Security A. +1. | Partition; five-target suspend; next-unsuspend lock; may attack; security-removed tail. |
| `BT23-057` Gankoomon | `BLOCKED` | Load-only gap stub. | Multi-card trash-to-deck cost reduction; Hinukamuy token; dynamic play-cost delete. |
| `BT23-072` King Drasil_7D6 | `BLOCKED` | Load-only gap stub. | Hand-main source placement; played-Digimon keyword grant; breeding source play. |
| `EX8-073` Gallantmon (X Antibody) | `BLOCKED` | Load-only gap stub. | Source-gated DP swings; delete-or-security fallback; memory aura immunity. |
| `EX10-068` Digimon Emperor | `PARTIAL` | [On Play] delete play cost 5 or lower; [Security] play self. | Opponent distinct-color count; returned-card color binding into same-color hand/trash play. |
| `EX11-053` Omekamon | `BLOCKED` | Load-only gap stub. | Royal Knight hand-to-King-Drasil source placement; Omnimon X union hand/source play and attach self. |

Regression files were added under:

- `code/digimon-engine/tests/cards_behavioral/ad1/`
- `code/digimon-engine/tests/cards_behavioral/bt13/`
- `code/digimon-engine/tests/cards_behavioral/bt15/`
- `code/digimon-engine/tests/cards_behavioral/bt17/`
- `code/digimon-engine/tests/cards_behavioral/bt19/`
- `code/digimon-engine/tests/cards_behavioral/bt20/`
- `code/digimon-engine/tests/cards_behavioral/bt22/`
- `code/digimon-engine/tests/cards_behavioral/bt23/`
- `code/digimon-engine/tests/cards_behavioral/ex8/`
- `code/digimon-engine/tests/cards_behavioral/ex10/`
- `code/digimon-engine/tests/cards_behavioral/ex11/`

## Reusable Open Gaps

### Breeding-Area Trigger Fan-Out

- **Gap:** Effects whose source permanent remains in breeding are not generally enqueued at turn/event timings.
- **Type:** `engine-gap`
- **Tracker:** `qa/archetype-qa/engine-gaps.md` (`G-BREEDING-TRIGGER-DISPATCH`)
- **Blocks:** `BT13-007` start-main source tuck; breeding-source observer shapes under King Drasil.
- **Why it matters:** King Drasil is the archetype's central source stack. If it must be moved to battle or silently skipped, the legal game state and action mask diverge from printed text.
- **Evidence:** `code/digimon-engine/cards/_examples/BT13-007.yaml` authors `when: start_of_your_main_phase` with `active_when: { in_breeding: true }`; `qa/archetype-qa/engine-gaps.md` records that start-main dispatch scans battle-area observers, not breeding sources.
- **First test:** Put `BT13-007` in player 0 breeding, a Royal Knight in player 0 battle area, and at least one card in the Digi-Egg deck. Enter main phase and assert the digitama plus the battle-area Royal Knight move under King Drasil.
- **Implementation hint:** Add a breeding trigger source or an `include_breeding` fan-out mode for timings like `StartOfYourMainPhase`, while preserving source-card/controller attribution and once-per-turn accounting.

### Global Security-Removed Observer Timing

- **Gap:** Security-stack removal must fan out to battle and relevant inherited/breeding observers with correct controller and event context.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Global OnOpponentSecurityRemoved observer timing`)
- **Blocks:** `BT20-083` inherited Omekamon, `BT20-060` Alphamon: Ouryuken, `BT23-035` Dynasmon, `BT20-056` Alphamon.
- **Why it matters:** Royal Knights repeatedly reacts to security being removed. Missing or partial observer fan-out either removes legal optional effects from the action mask or lets mandatory effects fail to trigger.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` lists global `OnOpponentSecurityRemoved` as a blocking reusable timing gap; `BT20-083` example YAML has inherited `when: on_opponent_security_removed`.
- **First test:** Place `BT20-083` as a source under King Drasil in breeding, remove the controller's security on the opponent's turn, and assert the optional Omekamon play-from-materials prompt appears.
- **Implementation hint:** Treat security removal as a first-class event with trigger context fields for affected player, source effect controller, removed count/card if available, and observer fan-out over battle and breeding/inherited sources.
- **Updated 2026-05-06:** Battle-area and battle/effect security-removal payloads are now wired for both `OnOpponentSecurityRemoved` and `OnOwnSecurityRemoved` and proved by BT24-001 plus BT4-097 fixtures. This RK gap is narrowed to breeding-resident/inherited King Drasil fan-out and card-local follow-up selections.

### Immediate May-Attack / Attack Without Suspending

- **Gap:** Scripts cannot install an immediate in-effect attack, including the "without suspending" variant.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Force-follow-up-attack / "may attack without suspending" script helpers`)
- **Blocks:** `BT20-102` end-of-turn clause; `BT20-017` Jesmon tail; `BT13-112` and `BT13-110` Rush payoff patterns after playing Royal Knights.
- **Why it matters:** These effects grant a player-visible attack decision. Granting only Rush or auto-attacking would change the action surface.
- **Evidence:** `code/digimon-engine/cards/bt20/BT20-102.yaml` explicitly leaves `force_attack_now` commented out under `G-MAY-ATTACK-NOW`; `docs/RUST_ENGINE_GAPS.md` marks the reusable primitive blocking.
- **First test:** Resolve `BT20-102` end-of-turn effect, choose a Digimon, and assert the next pending action allows that Digimon to attack without suspending, with pass/decline behavior matching printed optionality.
- **Implementation hint:** Add a script primitive such as `force_follow_up_attack` or `may_attack_now` plus an attack flag for `without_suspending`, reusing existing pending attack/action-mask machinery.

### Leave-Field Replacement and Prevention Effects

- **Gap:** Replacement effects for "would leave", Armor Purge, Barrier, and effect-specific prevention remain incomplete for Royal Knights' core prevention package.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`WhenWouldBeDeleted / leave-field replacement-effect framework`, `<Armor Purge>`, `<Barrier>`)
- **Blocks:** `BT20-100`, `BT23-054`, `BT23-035`, `BT23-058`, `BT20-056`, and related protection effects.
- **Why it matters:** The archetype survives by preventing departures from battle. These are optional or costed replacement decisions, so they must surface as choices before the zone move resolves.
- **Evidence:** `BT20-100` printed text prevents an Omnimon from leaving via Delay; `BT23-054` uses Armor Purge; `BT23-035` uses Barrier. The reusable trackers still list these replacement variants as blocking or incomplete.
- **First test:** Attempt to delete `BT23-054` with a source and assert Armor Purge offers a prevention choice that trashes the top source instead of moving the permanent. Separately, test `BT20-100` preventing a named Omnimon leave-field event by trashing the option.
- **Implementation hint:** Build on the existing replacement context predicates and cost framework, but add keyword-specific replacement emitters and Delay-as-replacement activation where applicable.

### Stack-Source Multi-Selection and Play From King Drasil

- **Gap:** The archetype needs robust selection and extraction from a breeding permanent's digivolution cards, including "one each with different names", On Play suppression, and Rush grants.
- **Type:** `engine-gap` / `dsl-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` under source/material selection and play-from-material helpers
- **Blocks:** `BT13-112`, `BT13-110`, `BT13-019`, `EX11-053`, `BT20-083`, `BT23-072`.
- **Why it matters:** Auto-playing the first matching source or ignoring name uniqueness hides a major Royal Knights decision. The selected cards must leave the source stack and become fresh permanents with correct On Play suppression when printed.
- **Evidence:** `BT20-083` example YAML uses `select_material` and `play_from_materials`; the broader tracker still calls out material extraction/play helpers and breeding follow-ups as recurring blockers.
- **First test:** Give King Drasil multiple Royal Knight sources with duplicate and distinct names, resolve `BT13-112`, and assert the player can choose at most one per name, selected cards enter battle, On Play effects are suppressed, King Drasil is trashed, and all played Digimon gain Rush.
- **Implementation hint:** Prefer a generic count-capped source selection with uniqueness predicates over card-specific raw Rust. It should work for battle-area and breeding carriers.

### Raid Target-Switch Timing

- **Gap:** Raid is still not fully represented as a mid-attack optional target-switch timing.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Raid target-switch interrupt`)
- **Blocks:** `BT20-102`, `BT23-072` granted Raid, and any other Royal Knight tech with printed Raid.
- **Why it matters:** Mask-time targeting is not equivalent to "when this Digimon attacks, you may switch the target". The player must be able to attack security first and then decide whether to Raid.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` documents Raid as mask-only and lacking an attack-state interrupt.
- **First test:** Attack security with a Digimon that has Raid while the opponent has an eligible unsuspended highest-DP Digimon; assert a pending optional switch appears before counter/block/security flow continues.
- **Implementation hint:** Add a `RaidOpen` attack state before later combat interrupts and dispatch `OnAttackTargetChange` after any switch.

### Aggregate-Sum Multi-Select

- **Gap:** Select any number of targets constrained by a running aggregate sum, such as total DP.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` / `qa/archetype-qa/engine-gaps.md` (`G-DP-BUDGET-MULTI-SELECT`)
- **Blocks:** `BT17-018` Gallantmon: Crimson Mode, a common Royal Knights tech.
- **Why it matters:** A single-pick fallback is not faithful when the player may choose any number of opposing Digimon whose total DP is at most 15000.
- **Evidence:** `code/digimon-engine/cards/bt17/BT17-018.yaml` documents the raw Rust fallback and ignored behavioral tests for the DP-budget selection.
- **First test:** Present opponent Digimon with 7000, 8000, and 9000 DP, resolve `BT17-018`, and assert the action mask allows 7000+8000 but not any combination above 15000.
- **Implementation hint:** Add a pending multi-select kind that validates a running aggregate over selected permanents and supports "finish early" once at least one legal target is chosen.

## Resolved Reusable Gaps To Avoid Reopening

The following reusable primitives surfaced in Royal Knights or adjacent audits and are now closed enough that a future cross-archetype spec should treat remaining work as card migration/test work unless new evidence appears:

| Primitive | Status | Remaining Royal Knights work |
|---|---|---|
| Delay option lifecycle and placement-turn gating | resolved 2026-05-02 | Author/migrate `BT20-100` and `BT13-110` YAML/tests. |
| `OnOptionPlaced` timing and breeding inherited fan-out for placed options | resolved 2026-05-02 | Use native timing in King Drasil inherited effect; add card-level regression. |
| Breeding permanent selection and bottom-source placement to real breeding slot | resolved 2026-04-29 / 2026-05-02 | Use explicit breeding selection or sugar in Omekamon/King Drasil scripts. |
| `self_digivolution_contains_name` predicate | resolved 2026-05-02 | Migrate `BT20-102` away from raw Rust only after other boardwipe/attack blockers are handled. |
| `not_in_binding` for excluding saved permanents | resolved 2026-05-01 | Use for BT20-102 boardwipe once source-stack predicate and flow are native. |
| `card_count_in_zone` formula filters | resolved 2026-05-02 | `BT8-097` already uses filtered count in YAML. |
| `dp_lte` / `dp_gte` permanent predicates | resolved 2026-05-02 | Update stale card comments/tests that still cite the old predicate gap. |
| Ace Overflow metadata and covered stack-leave paths | resolved 2026-05-02 | Add targeted coverage for exotic ACE movements before relying on them. |
| Dynamic cost reduction and triggered pay-cost selections | resolved Group 3 | Use for King Drasil cost reduction and similar play-cost hooks. |

## Card-Local YAML/Test Backlog

These items should not be promoted to generic gaps until a failing test proves a reusable primitive is missing:

| Card | Needed next step |
|---|---|
| `BT20-100` The Last Guardian | Add Delay leave-prevention after `RK-G003`; test King Drasil inherited memory from option placement when breeding fan-out is reliable. |
| `BT20-091` Cool Boy | Add optional Omekamon hand play when an RK would leave after `RK-G004`. |
| `BT23-054` Magnamon | Add Armor Purge after `RK-G003`; broaden behavioral runtime tests for return-to-hand/deck protection. |
| `BT13-112` Omnimon | Fill payoff stub after source multi-select/play support; test On Play suppression and Rush grant. |
| `BT20-017` Jesmon | Fill token/observer stub after token registration and may-attack flow; test token creation, other-Digimon-play observer, delete target, and may-attack tail. |
| `BT20-060` Alphamon: Ouryuken | Add Counter Blast DNA action path, DNA-gated security trash/recovery, and security-removed memory. |
| `BT23-035` Dynasmon | Add security-removed recovery branch after observer gap; broaden runtime test for security-trash cost debuff. |
| `BT23-072` King Drasil_7D6 | Fill support stub after hand-main source placement and breeding/inherited fan-out support; test granted keyword package. |
| `BT13-110` Royal Knights of the Purge | Add source selection from King Drasil, On Play suppression, and Rush grant after `RK-G001`. |
| `EX11-053` Omekamon | Fill stub after hand-to-fielded-source and union hand/source play support; test placing RK source under King Drasil and low-security Omnimon play from hand/materials. |

## Suggested Spec Grouping

For a cross-archetype gap spec, group remaining work by reusable capability rather than by Royal Knights card:

1. Breeding-source event fan-out: start-main, security-removed, and inherited observer attribution.
2. Immediate attack action flow: may-attack, force-follow-up attack, and without-suspending variants.
3. Replacement/prevention framework: would-leave, Armor Purge, Barrier, and Delay-as-replacement.
4. Source-stack selection/play: breeding carrier sources, uniqueness constraints, play-from-materials, On Play suppression.
5. Combat interrupt timing: Raid and attack-target-change events.
6. Aggregate pending selections: DP-budget multi-select.
7. Card migration pass: Royal Knights YAML/tests using only primitives closed above.

## Spec Input Checklist

A future spec should require each reusable gap to include:

- one failing Rust behavioral test under `code/digimon-engine/tests/`;
- one DSL lowering/compiler test when YAML vocabulary changes;
- action-mask or `PendingSelection` assertions for every player-visible choice;
- no `ACTION_SPACE_SIZE` or tensor contract expansion unless the action/tensor specs are updated in the same change;
- tracker updates in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and this file when a reusable gap closes or is split.
