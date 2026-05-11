# Alter-S Ladder Rust Engine / DSL Assessment

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


Date: 2026-05-03

## Verdict

**Blocked, but substantially advanced by the 2026-05-10 implementation pass.** The refreshed `Alter-S Ladder` deck pool has only one exact `Alter-S Ladder` decklist, from DigiLab. The deck contains 17 unique card IDs. As of the 2026-05-10 pass, 10 have Rust YAML/tests in the active lane: `BT16-082`, `BT17-077`, `BT5-112`, `EX10-002`, `EX10-010`, `EX4-060`, `EX9-013`, `EX9-021`, `P-101`, and `P-128`.

Do not claim this archetype is playable in Rust yet. The remaining blockers are reusable engine/DSL gaps around face-down sources, source-to-hand costs, granted future attacks / source-name predicates, reveal-to-play, replacement-triggered DNA, and Paladin-style mass cleanup.

## 2026-05-10 Implementation Update

Implemented or audited forward:

- `P-101`: full YAML plus 5 behavioral tests for active draw/discard and inherited Lv3 deletion.
- `BT5-112`: full YAML plus 7 behavioral tests for security play, Tamer deletion, and Digimon deletion.
- `EX10-002`: inherited attack-target-change draw YAML plus 5 passing tests; shared triggered OPT enforcement remains ignored under `G-OPT-TRIGGERED`.
- `P-128`: full YAML plus 10 behavioral tests for start-main memory, modal On Play, free play/digivolve, decline paths, and security play.
- `EX10-010`: target filter now enforces `play_cost_lte: 7`; source-scoped continuous immunity remains a separate blocker.
- `EX9-021`: DNA-origin immunity now implemented with opponent-source effect immunity for the turn, alongside the existing highest-level delete and end-of-attack source play/security placement.
- `EX9-013`: metadata/route drift fixed to red/black Virus, standard Lv5 red cost 4, plus explicit Blast Digivolve alt-path marker.
- `BT17-077`: metadata drift fixed to white/blue; main cleanup clauses remain blocked.

Blocked with tests or assessment evidence:

- `BT15-096`: resolved 2026-05-10. The 6 `G-PLAY-COST-LTE-BINDING` tests are active and pass; Delay uses formula-valued `play_cost_lte` relative to the selected source Digimon's play cost.
- `EX10-008`: full card blocked on inherited `source_name_contains` target-change predicate evaluation and granted future start-main attack behavior.
- `EX5-048`: full card blocked on play-from-reveal and granted future start-main attack behavior.
- `EX9-011` / `EX9-068`: blocked on face-down source placement/state and related predicates/formulas.
- `EX9-020`: blocked on replacement-triggered effect DNA using the in-flight leaving Lv6.
- `BT5-087`: full card blocked on returning a Lv6 source to hand as an attack cost.
- `BT17-077`: full main clause blocked on all-source mass trash, player choice over whose trash to return, and returned-card result predicates.

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
| `BT16-082` Ukkomon | On move from breeding: reveal 3, add Digimon/Tamer, bottom rest, may hatch | implemented | `code/digimon-engine/cards/bt16/BT16-082.yaml`; `code/digimon-engine/tests/cards_behavioral/bt16/bt16_082.rs` | Card behavior implemented; BT16-specific OPT lockout/reset coverage remains ignored as shared triggered OPT work. |
| `EX10-010` BlackWarGreymon | Blast/Raid/Reboot/Blocker; delete cost <=7 Digimon/Tamer; conditional +3000 DP and opponent-Digimon-effect immunity | partial | `code/digimon-engine/cards/ex10/EX10-010.yaml`; `code/digimon-engine/tests/cards_behavioral/ex10/ex10_010.rs` | `play_cost_lte: 7` fixed 2026-05-10; continuous source-scoped immunity remains blocked. |
| `EX9-013` BlitzGreymon | Blast/Alliance/Blocker; De-Digivolve 3; end-turn optional DNA into Omnimon Alter-S, then optional attack | implemented | `code/digimon-engine/cards/ex9/EX9-013.yaml`; `code/digimon-engine/tests/cards_behavioral/ex9/ex9_013.rs` | Metadata/route drift fixed 2026-05-10. |
| `EX10-008` MetalGreymon | Grant Collision and forced start-main attack to opponent Digimon; inherited target-change security trash | engine-gap | no YAML under `code/digimon-engine/cards/`; printed text in `data/cards.json` | Need granted Collision/forced attack effect coverage plus inherited attack-target-change observer. |
| `EX9-011` MetalGreymon | Cost reduction by trashing hand card; tuck trash card face down; delete DP budget scaling with face-down sources | dsl-gap | no YAML | Author card after validating face-down source representation and DP-budget formula/count support. |
| `EX9-020` CresGarurumon | Bottom-deck Lv5 or lower; when any Lv6 would leave by opponent effect, play a Lv6 source instead | engine-gap | no YAML | Needs leave-field replacement scoped to any own Lv6 and source-stack play. |
| `EX4-060` Omnimon Alter-S | Delete small Digimon and bottom-deck Lv6+; when leaving, play BlitzGreymon + CresGarurumon sources and place self bottom security | implemented | `code/digimon-engine/cards/ex4/EX4-060.yaml`; `code/digimon-engine/tests/cards_behavioral/ex4/ex4_060.rs` | Source-stack play and bottom-security replacement are covered for this card. |
| `EX9-021` Omnimon Alter-S | DNA immunity, delete all highest-level opponent Digimon, end-of-attack play two named/trait sources and place self top security | implemented | `code/digimon-engine/cards/ex9/EX9-021.yaml`; `code/digimon-engine/tests/cards_behavioral/ex9/ex9_021.rs` | DNA-origin immunity adopted 2026-05-10. |
| `BT5-087` Omnimon Zwart | Mill 3, may play up to two cost <=8 black/purple Digimon from trash; source-to-hand cost then delete unsuspended cost <=12 | dsl-gap | no YAML | Author up-to-two trash play and source-return cost tests. |
| `BT15-096` Supreme Connection! | Reveal 3, add one Machine/Cyborg and trash one, return rest on top; Delay play hand Digimon with cost <= selected source play cost, reduced by 3 | implemented | `code/digimon-engine/cards/bt15/BT15-096.yaml`; `code/digimon-engine/tests/cards_behavioral/bt15/bt15_096.rs` | Resolved 2026-05-10 via formula-valued `play_cost_lte` + `binding_play_cost`; 6 active tests pass with `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture`. |
| `EX10-002` Koromon | Inherited attack-target-change draw | implemented | `code/digimon-engine/cards/ex10/EX10-002.yaml`; `code/digimon-engine/tests/cards_behavioral/ex10/ex10_002.rs` | Shared triggered OPT enforcement remains ignored under `G-OPT-TRIGGERED`. |
| `P-101` Raremon | Draw/discard and inherited Lv3 deletion | implemented | `code/digimon-engine/cards/p/P-101.yaml`; `code/digimon-engine/tests/cards_behavioral/p/p_101.rs` | Full card covered. |
| `P-128` Cody Hida | Free-trait memory, modal On Play, security play | implemented | `code/digimon-engine/cards/p/P-128.yaml`; `code/digimon-engine/tests/cards_behavioral/p/p_128.rs` | Full card covered. |
| `BT5-112` Omnimon Zwart Defeat | Security play, when-digivolving Tamer deletion, on-deletion Digimon deletion | implemented | `code/digimon-engine/cards/bt5/BT5-112.yaml`; `code/digimon-engine/tests/cards_behavioral/bt5/bt5_112.rs` | Full card covered. |
| `EX9-068`, `EX5-048` | Played-Digimon observer/source tuck; forced attack plus reveal play | engine-gap / dsl-gap | no YAML for these IDs | `EX9-068` waits on face-down source support; `EX5-048` waits on play-from-reveal and granted future attack support. |

## First Tests To Add

1. `EX4-060`: when it would leave by opponent effect, present source selections for one `BlitzGreymon` and one `CresGarurumon`, play both, then place `EX4-060` at bottom security.
2. `EX9-021`: DNA digivolving deletes all opponent Digimon tied for highest level and grants opponent-effect immunity only for that turn.
3. `EX9-013`: after the end-turn DNA process resolves, the player receives an optional pending selection to attack with one of their Digimon. Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex9_013_eot_clause_contains_post_dna_may_attack_now ex9_013_eot_after_dna_one_digimon_may_attack`.
4. `EX10-010`: opponent cost-8 permanent is excluded from the delete target mask; opponent cost-7 permanent is legal.
5. `BT16-082`: moving an own Digimon out of breeding installs reveal selection, adds exactly one Digimon/Tamer, bottoms the rest, then gates an optional hatch action.

## Verification

- 2026-05-10 targeted pass:
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex10_010_on_play_filter_excludes_cost_above_7_target` → 1 passed.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex9_021` → 15 passed.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral p_101` → 5 passed.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt5_112` → 7 passed.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral ex10_002` → 5 passed, 1 ignored.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral p_128` → 10 passed.
  - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral bt15_096 -- --nocapture` → 6 passed, 0 ignored after formula-valued `play_cost_lte` landing.
