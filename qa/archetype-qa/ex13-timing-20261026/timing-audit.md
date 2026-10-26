# Frozen and refreshed-pool timing audit

Inventory: **280 unique card IDs, 13 archetypes, 21 training lists and 13 evaluation lists** (the frozen input manifests, including repeated pet train/eval lists).

The YAML inventory contains **817 clauses**, including **518 triggered clauses / 665 timing declarations**, **40 replacements**, **22 Delay declarations** and **1 nested scheduled body**. Counts refer to authored YAML; generated keyword effects are a separate denominator.

## Confirmed event-scope defects

| Card | Incorrect event | Repair | Evidence |
|---|---|---|---|
| EX12-034 Erlangmon | Own Tamer play activated a Digimon-play clause | Explicit event_target_kind: digimon qualifier | Printed image, official bundle, DCGO; native RED then GREEN controls |
| EX12-036 Ryugumon | Own Tamer play activated a Digimon-play clause | Explicit event_target_kind: digimon qualifier | Printed image, official bundle, DCGO; native RED then GREEN controls |
| EX12-057 Takutoumon | Own Tamer play activated a Digimon-play clause | Explicit event_target_kind: digimon qualifier | Printed image, official bundle, DCGO; native RED then GREEN controls |
| BT25-013 Firamon | Own Tamer play activated a Digimon-play clause | Explicit event_target_kind: digimon qualifier | Printed image, official bundle, DCGO; native RED then GREEN controls |

The shared on_ally_played dispatcher deliberately observes all own permanents. It cannot gain a blanket Digimon restriction: EX13-060 deliberately observes both Chronicle Digimon and Tamers. The repairs belong in the four printed clauses.

Definitive initial RED: `timing-audit-red-final.log` = 8 passed / 4 failed. Each failed negative Tamer test has passing own-Digimon and opponent-Digimon controls. Later `timing-audit-delay-red-final.log` = 13 passed / 1 failed: all 12 event-kind tests passed after four YAML repairs; the remaining failure is the separate start-turn ordering defect.

## Confirmed shared start-turn ordering defect

LM-027 Red Scramble and BT24-088 Asuna coexist in a frozen Chronomon training list. Both print Start of Your Turn. The ordinary Asuna clause was offered alone before the due Delay was collected. Native regression `start_turn_scramble_delay_and_asuna_share_trigger_order` fails; `start_turn_scramble_delay_alone_is_due` passes. CR 4.3 15-4-3-1/2/4/5 requires the player to choose among their simultaneously triggered effects. Both DCGO cards register OnStartTurn.

Repair complete: capture due start-turn Delay entries in the same queue before draining; advance surviving carriers to their next owner window once. Decline, a later-false condition or an unpaid cost retains the card for a later window. DSL bodies retain their trash-before-body cost. Native Delay bodies retain the existing post-body cleanup contract, and an interrupted native cleanup resumes before the outer phase continuation. Clone, either-order, decline/reoffer, native cleanup and parked-selection controls pass.

An independent review caught an activation-condition hazard in the initial repair: prefiltering a currently false `active_when` at capture would lose an effect whose sibling later enables it. CR 4.3 15-6-1 and 15-8-3-9-1/2 classify printed “if” as a processing condition. Capture is now unconditional for due Delay entries; the existing queue checks processing conditions at activation. A real LM-027 with a synthetic simultaneous sibling that plays an opposing Digimon was RED, then GREEN. The inverse sibling deletes the opposing Digimon before Delay activation and preserves the unspent Option.

## Confirmed per-turn reset boundary defect

BT25-077 is **Bacchusmon**, an All Turns / Once Per Turn Digimon-play observer. BT24-088 Asuna can play a TS Digimon during Start of Your Turn. Resetting counters only at Unsuspend caused both failures: a previous-turn use suppressed the valid new-turn observer, while a use made during the start window was erased before Main and could be used again. CR 4.3 15-14-1-5-1 resets uses when the turn changes.

Two native regressions were RED (`timing-opt-red.log`: 16 pass / 2 fail), then GREEN after moving existing counter resets to `begin_turn`, before collecting start triggers, and removing the second reset at Unsuspend. These fixtures deliberately put actual frozen-pool cards together to isolate the shared substrate; they are not a replay of one frozen list. Bacchusmon belongs to the frozen TS Mervamon lists; Asuna belongs to Chronomon, TS Jupitermon and Three Musketeers. Their printed effects and actual play path are used; the counterexample does not certify those entire archetypes.

## Final focused validation

`timing-options-final-verified.log`: **22/22 timing regressions and 143/143 option_flow tests pass**, exit 0. No ignored or filtered tests in either final suite. The timing file contains 12 event-kind controls, four start-window order/clone/decline controls, two OPT boundary controls, two processing-condition controls, and two native cleanup incarnation controls. The complete Option suite includes interrupted native Delay cleanup and replacement/paid-cost controls. An initial integration failure exposed the native cleanup continuation ordering and was fixed before this final result.

Four YAML repairs are limited to the printed Digimon event qualifier. The shared phase repair changes no action IDs or tensor layout. Independent read-only review of capture conditions, nested native cleanup and reset timing found no further blocker; broader combat/phase/effect/DSL/card suites are being run separately by the mechanics agent. Those results are not assumed here.

## Coverage and limits

- All 280 card IDs and all 817 YAML clauses inventoried, with archetype membership, timing, scope, trigger_if, timing-specific conditions, active conditions and bodies in timing-inventory.json.
- Explicit timing counts: on_ally_played 9; on_enter_field_anyone 6; on_digivolve 7; on_any_deletion 1; on_deletion 18; on_security_check 0; on_check_face_up_security 0. No authored on_ally_deleted timing is present. Zero explicit clauses does not certify generated keyword behavior.
- Reviewed shared dispatcher/controller routing, top/inherited/linked scans, source-subject predicate construction, play-event bundling, turn/main-phase entry, scheduled Delay flow, and source-identity handling. No blanket pass is asserted for their interactions.
- EX1-066 is the only explicit on_any_deletion clause: owner, Digimon kind, level >=5 and source presence are all authored trigger qualifiers. This is a static guard check, not new behavioral certification.
- Existing dedicated EX3-024 tests cover own/opponent main entry, breeding exclusion, inherited/top behavior and forced-attack windows; these were located/read, not newly run by this audit yet. Existing ST24-14 tests cover host-scoped source trashing and all-turns; likewise not a new pass claim.
- P-204 Delay intentionally observes either side attacking a player: its all-owner scan is correct at the helper level. It must not be changed into a controller-only attack observer based on the on_ally_attack spelling.
- End-turn Delay inventory contains three empty persistence markers (BT21-093, ST23-15, ST24-15), no active end-turn scheduled Delay bodies. This does not establish correctness of all native/end-turn Delay APIs outside this pool.
- No explicit StartOfOpponentsTurn clause is in either union. Missing general dispatch is a separate unmeasured substrate risk; it was not changed. The derived OPT reset defect is now confirmed and repaired above.
- The final refreshed inventory below is a separate denominator; frozen counts are retained for historical reproducibility.

## Sources

- Local official comprehensive rules: docs/digimon-rules/cr-4.3.txt, sections15-8-3-1/3/4 and15-4-3-1/2/4/5.
- Card images inspected: EX12-034, EX12-036, EX12-057, BT25-013, LM-027, BT24-083, BT24-088 and BT25-077, from the local DCGO texture mirror.
- DCGO sources inspected: EX12/Blue/EX12_034.cs, EX12/Blue/EX12_036.cs, EX12/Black/EX12_057.cs, BT25/Red/BT25_013.cs, LM/Red/LM_027.cs, BT24/Purple/BT24_088.cs and BT25/Black/BT25_077.cs.

## Explicit timing inventory

| Timing | Declarations |
|---|---:|
| counter | 2 |
| end_of_attack | 8 |
| end_of_opponents_turn | 1 |
| end_of_your_turn | 15 |
| main | 1 |
| main_from_hand | 48 |
| main_on_field | 7 |
| on_add_digivolution_cards | 1 |
| on_add_to_deck | 4 |
| on_ally_attack | 8 |
| on_ally_played | 9 |
| on_ally_won_battle | 6 |
| on_any_deletion | 1 |
| on_attack | 1 |
| on_attack_target_change | 2 |
| on_deletion | 18 |
| on_digivolution_card_trashed | 9 |
| on_digivolve | 7 |
| on_enter_field_anyone | 6 |
| on_lose_security | 2 |
| on_move | 13 |
| on_opponent_attack | 8 |
| on_opponent_security_removed | 17 |
| on_own_security_removed | 11 |
| on_play | 130 |
| on_security | 75 |
| on_suspend | 12 |
| start_of_opponents_main_phase | 2 |
| start_of_your_main_phase | 33 |
| start_of_your_turn | 9 |
| when_attacking | 71 |
| when_digivolving | 126 |
| when_linked | 2 |

## Final refreshed denominator and bounded follow-up review

`refreshed-timing-inventory.json` is generated from the final `data/training_pools/ex13-20261026/{train_snapshot,heldout_snapshot,pet_regression_snapshot}.json` with SHA-256 hashes. It includes **21 train entries / 13 archetypes**, **23 held-out entries / 12 archetypes**, and **5 original pet regression entries**. These are 49 panel entries, not 49 independently sourced or unique lists. The combined monitoring panel and optional supplemental lists are excluded from this union.

The refreshed union contains **311 unique IDs**, **908 YAML clauses**, **574 triggered clauses / 739 explicit timing declarations**, **44 replacements** and **23 Delay declarations**. Relative to the frozen 280-card union, **45 IDs are added and 14 removed**. Every refreshed ID has a YAML file. This is inventory completeness, not behavioral certification.

Added observer declarations were screened for event carrier/controller predicates. BT20-023's ally-play trigger explicitly requires an own green Digimon; BT16-085's digivolve trigger explicitly requires an own blue/green Digimon. The added Shine cards BT12-034, BT21-004 and EX4-005 explicitly scope suspension to own red/yellow Tamers; both BT21-044 deletion clauses explicitly scope owner, Tamer kind and red/yellow color. BT21-042's Marcus-name play trigger has owner/name guards. This static predicate check found no additional instance of the four demonstrated Tamer-as-Digimon defects. It is not a behavioral pass for those cards.

Prioritized candidates for further native timing coverage (unmeasured by this audit):

| Candidate | Reason to target | Current evidence |
|---|---|---|
| BT21-044 normal + inherited deletion | Added simultaneous Tamer-deletion observers and host/source liveness | Owner/kind/color guards statically present; no new native test here |
| BT25-034 security-trash clause | New on_discard_security timing; must not activate for ordinary security battle removal | Explicit security scope; dispatcher path read; no new native test here |
| LM-029 | Added start-turn Scramble, same shared phase path and conditional activation | YAML inspected; shared LM-027 controls pass; individual body not newly tested here |
| EX13-006 + BT9-112 | Added inherited own-end-turn and opponent-end-turn clauses | Timings inventoried; no new independent end-turn native cases here |
| BT25-088 / BT1-087 | Added memory setters sharing the start window | Timings inventoried; phase substrate tested, exact card combinations not newly tested here |

All 23 refreshed Delay clauses were inventoried: the three end-turn declarations remain empty persistence markers (BT21-093, ST23-15, ST24-15). No active end-turn scheduled Delay body is present. Existing event-gated and Main-phase Delay behavior is covered only to the extent of the separate Option suite, not by a blanket 23-card certificate. Generated keywords, every possible pair of simultaneous effects, and DCGO live-oracle equivalence remain outside the asserted pass.

## Native cleanup incarnation review — evidence correction

The first remove/replay fixture was invalid: `place_self_as_delay_option_permanent` does not extract a top/only source card when supplied a source permanent, so it had not created a replacement permanent. Its failures (`timing-native-incarnation-red.log`, `timing-options-incarnation-green.log`, diagnostic logs) are not engine reproduction evidence. The corrected fixture uses actual `bounce_self` followed by effect-driven Option use and asserts a new placement before cleanup. Its parked version now uses an empty typed continuation for its no-op decision so a clone does not depend on an uncloneable closure. The definitive check is described below; the earlier failures are retained only as explicitly labeled construction diagnostics.

### Definitive incarnation check and final result

With the corrected fixture, both cases pass when cleanup requires the captured `battle_identity`. Temporarily removing **only the immediate and parked identity filters**, while retaining the same valid fixtures, produces **0 passed / 2 failed** in `timing-native-incarnation-red-valid.log`. Both fixtures first assert that the same physical Option really has a new placement; the failure is then the old cleanup deleting that new permanent. This is an explicitly labeled identity-filter ablation, not a claim that the invalid initial fixture demonstrated the bug.

The two filters were restored. Final `timing-options-final-verified.log` is **22/22 timing tests +143/143 Option-flow tests GREEN**, exit 0, no ignored or filtered tests. Cleanup now pins the old stack identity before native body processing and requires it both immediately and across a parked/cloned continuation. A new permanent does not inherit old cleanup even when its physical card and scheduled next turn match. The original native cost contract is retained, and normal carrier compaction remains supported.

Independent review of the final identity delta found no further blocker. Source diff checking passed. The native build target was released for the mechanics agent's broader scoped combat/DSL/card checks; those outcomes have their own logs and are not rolled into the 22+143 count here.
