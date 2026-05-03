# Alter-S Ladder Rust Engine / DSL Assessment

Date: 2026-05-03

## Verdict

**Blocked.** The refreshed `Alter-S Ladder` deck pool has only one exact `Alter-S Ladder` decklist, from DigiLab. The deck contains 17 unique card IDs, but only 3 are currently reported by `digimon_engine.load_implemented_card_ids()`: `BT16-082`, `EX10-010`, and `EX9-013`. Those three are partial: targeted `cards_behavioral` tests pass for implemented slices, but 15 tests remain ignored for card-body placeholders or omitted printed clauses.

Do not claim this archetype is playable in Rust yet. The blockers are mostly authored-card coverage plus a few reusable engine/DSL gaps around source-stack play, effect-initiated attacks, security placement, and effect immunity.

## Data Refresh Notes

- Ran `python code\tools\meta_loader.py --scrape-digilab --fetch-meta --scrape-egman "https://egmanevents.com/digimon-bt24-tournaments/carta-magica-april-online-2026"`.
- `data/deck_library.json` now has `generated_at: 2026-05-03T02:59:53.604677+00:00` and `total_entries: 1900`.
- Exact `Alter-S Ladder` remains 1 decklist: DigiLab, placement 21, event date 2025-12-02, display card `EX4-060`.
- DigiLab aggregate stats for `Alter-S Ladder`: 2 played, 50.00% conversion/top4, 50.00% win rate.
- The Egman April regional source added/merged separate `Garuru Alter-S` lists; it did not add a second exact `Alter-S Ladder` decklist.

## Deck Pool Assessed

| Count | Card | Role |
|---:|---|---|
| 4 | `EX10-002` Koromon | egg; attack-target-change draw inherited |
| 4 | `BT16-082` Ukkomon | rookie utility |
| 4 | `EX10-008` MetalGreymon | Collision / forced attack enabler |
| 4 | `EX9-011` MetalGreymon | face-down source + DP-budget deletion |
| 4 | `EX10-010` BlackWarGreymon | ACE Lv6; Raid/Reboot/Blocker, delete, immunity |
| 4 | `EX9-013` BlitzGreymon | ACE Lv6; De-Digivolve + end-turn DNA |
| 4 | `EX9-020` CresGarurumon | ACE Lv6; bottom-deck + leave-field replacement |
| 4 | `EX4-060` Omnimon Alter-S | Lv7 payoff; delete/bottom-deck + leave replacement |
| 3 | `P-101` Raremon | draw/discard and inherited removal |
| 3 | `BT5-087` Omnimon Zwart | mill + play from trash; source bounce cost |
| 3 | `EX9-068` Analogman | memory setter; played-Digimon observer + source tuck |
| 3 | `P-128` Cody Hida | tech tamer |
| 3 | `BT15-096` Supreme Connection! | reveal/add/trash + Delay play |
| 2 | `EX5-048` Etemon | forced attack enabler |
| 2 | `BT5-112` Omnimon Zwart Defeat | security play / delete |
| 2 | `EX9-021` Omnimon Alter-S | DNA Lv7 payoff |
| 1 | `BT17-077` Imperialdramon: Paladin Mode | ACE Lv7 tech |

## Capability Table

| Card | Required behavior | Status | Evidence | Gap / next step |
|---|---|---|---|---|
| `BT16-082` Ukkomon | On move from breeding: reveal 3, add Digimon/Tamer, bottom rest, may hatch | dsl-gap | `code/digimon-engine/cards/bt16/BT16-082.yaml`; `code/digimon-engine/tests/cards_behavioral/bt16/bt16_082.rs` | Replace raw-rust no-op body with real reveal/select/hatch process; unignore behavioral tests. |
| `EX10-010` BlackWarGreymon | Blast/Raid/Reboot/Blocker; delete cost <=7 Digimon/Tamer; conditional +3000 DP and opponent-Digimon-effect immunity | dsl-gap / engine-gap | `code/digimon-engine/cards/ex10/EX10-010.yaml`; `code/digimon-engine/tests/cards_behavioral/ex10/ex10_010.rs` | Add `play_cost_lte: 7` to YAML now that predicate support exists; implement/wire source-kind effect immunity for the conditional aura. |
| `EX9-013` BlitzGreymon | Blast/Alliance/Blocker; De-Digivolve 3; end-turn optional DNA into Omnimon Alter-S, then optional attack | dsl-gap | `code/digimon-engine/cards/ex9/EX9-013.yaml`; `code/digimon-engine/tests/cards_behavioral/ex9/ex9_013.rs` | Implement `G-MAY-ATTACK-NOW` as a pending-selection-backed effect attack step. |
| `EX10-008` MetalGreymon | Grant Collision and forced start-main attack to opponent Digimon; inherited target-change security trash | engine-gap | no YAML under `code/digimon-engine/cards/`; printed text in `data/cards.json` | Need granted Collision/forced attack effect coverage plus inherited attack-target-change observer. |
| `EX9-011` MetalGreymon | Cost reduction by trashing hand card; tuck trash card face down; delete DP budget scaling with face-down sources | dsl-gap | no YAML | Author card after validating face-down source representation and DP-budget formula/count support. |
| `EX9-020` CresGarurumon | Bottom-deck Lv5 or lower; when any Lv6 would leave by opponent effect, play a Lv6 source instead | engine-gap | no YAML | Needs leave-field replacement scoped to any own Lv6 and source-stack play. |
| `EX4-060` Omnimon Alter-S | Delete small Digimon and bottom-deck Lv6+; when leaving, play BlitzGreymon + CresGarurumon sources and place self bottom security | engine-gap | no YAML; `docs/RUST_ENGINE_GAPS.md` tracks source-stack play and security-stack placement | Implement source-stack pair selection/play and bottom-security placement without auto-picks. |
| `EX9-021` Omnimon Alter-S | DNA immunity, delete all highest-level opponent Digimon, end-of-attack play two named/trait sources and place self top security | engine-gap | no YAML | Needs DNA-origin conditional immunity, highest-level group deletion, source-stack play, and top-security placement. |
| `BT5-087` Omnimon Zwart | Mill 3, may play up to two cost <=8 black/purple Digimon from trash; source-to-hand cost then delete unsuspended cost <=12 | dsl-gap | no YAML | Author up-to-two trash play and source-return cost tests. |
| `BT15-096` Supreme Connection! | Reveal 5, add one Machine/Cyborg and trash one, return rest on top; Delay play Lv5+ Machine/Cyborg with cost -3 | dsl-gap | no YAML; Delay primitives recently resolved in `docs/RUST_ENGINE_GAPS.md` | Author reveal multi-choice/order and delayed play-cost reduction tests. |
| Other techs (`EX10-002`, `P-101`, `EX9-068`, `P-128`, `EX5-048`, `BT5-112`, `BT17-077`) | Draw/discard costs, played-Digimon observers, forced attacks, security play, mass trash/deck returns | dsl-gap / engine-gap | no YAML for these IDs | Batch author after core Lv5/Lv6/Lv7 shell gaps are closed. |

## First Tests To Add

1. `EX4-060`: when it would leave by opponent effect, present source selections for one `BlitzGreymon` and one `CresGarurumon`, play both, then place `EX4-060` at bottom security.
2. `EX9-021`: DNA digivolving deletes all opponent Digimon tied for highest level and grants opponent-effect immunity only for that turn.
3. `EX9-013`: after the end-turn DNA process resolves, the player receives an optional pending selection to attack with one of their Digimon.
4. `EX10-010`: opponent cost-8 permanent is excluded from the delete target mask; opponent cost-7 permanent is legal.
5. `BT16-082`: moving an own Digimon out of breeding installs reveal selection, adds exactly one Digimon/Tamer, bottoms the rest, then gates an optional hatch action.

## Verification

- `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- ex9_013 ex10_010 bt16_082`
  - Result: 40 passed, 15 ignored, 0 failed.
