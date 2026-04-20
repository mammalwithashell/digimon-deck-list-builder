# Rust Engine Build-Out Roadmap

**Living tracker** for the phased expansion of `digimon-engine`'s scripting surface. Each phase adds primitives that unblock a cluster of Cluster-tagged gap entries in `docs/RUST_ENGINE_GAPS.md` and moves parity-tracker sections in `docs/RUST_PYTHON_PARITY.md` from 🔴/🟡 to ✅.

---

## Cumulative Readiness Table

| Phase | Theme | Status | Tests | Key API added | Plan |
|---|---|---|---|---|---|
| Phase 1 | Timing dispatch | ✅ Landed 2026-04-19 | 418 → 431 (+13) | Turn-phase timings (`StartOfYourTurn`, `StartOfYourMainPhase`, `EndOfOpponentsTurn`), combat timings (`WhenAttacking`, `EndOfBattle`, `EndOfAttack`, `OnAttackTargetChange`), global observers (`OnEnterFieldAnyone`, `OnAnyDeletion`, `OnSuspend`, `OnUnsuspend`, `OnHatch`, `OnDigivolve`), archetype observers (`OnOpponentSecurityRemoved`, `OnDigivolutionCardTrashed`) | `docs/superpowers/plans/2026-04-19-rust-engine-phase-1-timing-dispatch.md` |
| Phase 2 | Zone manipulation | ✅ Landed 2026-04-19 | 431 → 447 (+16) | Zone-move primitives: `play_from_hand_with_cost`, `play_from_trash_with_cost`, `effect_initiated_digivolve`, `return_to_hand`, `return_to_deck`, `add_to_hand_from_trash`, `trash_from_hand_by_index`, `reveal_top_deck`, `hatch`, `place_on_security`, `place_as_bottom_source`; `CostDelta`, `StackPosition`, `CardSourceRef` types | `docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md` |
| Phase 3 | Native keyword parsing | ✅ Landed 2026-04-19 | 447 → 463 (+16) | `CardData::keywords: Vec<Keyword>`, `parse_printed_keywords(...)`, `Game::has_keyword(handle, Keyword) -> bool`; migrated 14 pre-existing keyword check sites to unified query | `docs/superpowers/plans/2026-04-19-rust-engine-phase-3-native-keywords.md` |
| Phase 4 | Selection-kind expansion | ✅ Landed 2026-04-20 | 463 → 495 (+32) | `SelectionKind::{UnionZone, OrderedPermutation, CountCappedMultiSelect}`, `GamePhase::{SelectUnion, SelectPermutation, SelectBudgeted}`, `UnionZoneSet` bitset, `CountCappedZone` enum; helpers: `select_union_zone`, `select_ordered_permutation`, `select_count_capped_multi`, `as_selecting_player` builder (`EffectContextSelectorScope`); all reuse existing action ranges (Python-parity); opponent-as-selector is net-new (no Python analog) | `docs/superpowers/plans/2026-04-20-rust-engine-phase-4-selection-kinds.md` |
| Phase 5 | Cost-reduction builder hooks | 🔲 Planned | — | `BeforePayCost` wiring in `calculate_play_cost`; `.cost_reduction_fn(|ctx| i16)` closure-valued variant; selection-gated cost payment (suspend/trash-N as cost); `.pay_cost(...)` builder hook | — |

---

## Immediate Next Steps

1. **Phase 1 — timing dispatch** → ✅ LANDED (2026-04-19). Plan: `docs/superpowers/plans/2026-04-19-rust-engine-phase-1-timing-dispatch.md`. Wired all declared-but-unfired `EffectTiming` variants (13 turn/combat/global observers) + 2 archetype-specific observers for Medusamon (`OnOpponentSecurityRemoved`) and Rocks (`OnDigivolutionCardTrashed`). 13 new tests. All new timings use `TriggerSource::PlayerBattleArea` / `TriggerSource::Permanent` dispatch via the effect queue.

2. **Phase 2 — zone manipulation** → ✅ LANDED (2026-04-19). Plan: `docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md`. 11 new `EffectContext` primitives covering free/cost-delta play, effect-initiated digivolve, bounce, return-to-deck, reveal pool, hatch, security placement, and stack insertion. 16 new tests. Free-play and cost-delta shapes expressed via `CostDelta::Reduce(n)`; `OnPlay` fires through the standard queue in all paths.

3. **Phase 3 — native keyword parsing** → ✅ LANDED (2026-04-19). Plan: `docs/superpowers/plans/2026-04-19-rust-engine-phase-3-native-keywords.md`. `CardData::keywords` populated at load time by `parse_printed_keywords`; unified `Game::has_keyword` query merges native + modifier-granted; 14 pre-existing call sites migrated. Closes parity §2.1b (native Rush) and §2.5f (native Jamming). 16 new tests.

4. **Phase 4 — selection-kind expansion** → ✅ LANDED (2026-04-20). Plan: `docs/superpowers/plans/2026-04-20-rust-engine-phase-4-selection-kinds.md`. 3 new `SelectionKind` variants (UnionZone, OrderedPermutation, CountCappedMultiSelect) + 3 new `GamePhase` variants (SelectUnion, SelectPermutation, SelectBudgeted), 4 new helpers (select_union_zone, select_ordered_permutation, select_count_capped_multi, as_selecting_player builder), 32 new tests. All new kinds reuse existing action ranges (Python-parity pattern). Opponent-as-selector is net-new — Python does not support it. 8 commits from `67e0afa4`..`65f0b3a6`. Full suite 495 passing. Closes Cluster D: ordered permutation, union-zone selection. Partially closes: count-capped multi (sibling of aggregate-sum gap), opponent-as-selector (DNA-pair and cross-side-target remain open). See `docs/RUST_ENGINE_API.md` §Phase 4 and annotated entries in `docs/RUST_ENGINE_GAPS.md`.

5. **Phase 5 — cost-reduction builder hooks** — Suggested next phase. Closes `BeforePayCost` scanning gap (RUST_ENGINE_API §9), which unblocks BT8-097 Crimson Blaze, BT9-112 DeathXmon, and the full Dynamic cost reduction cluster in `docs/RUST_ENGINE_GAPS.md`. Key deliverables: wire `BeforePayCost` dispatch into `calculate_play_cost`; add `.cost_reduction_fn(|&EffectReadContext| i16)` closure-valued variant on `EffectBuilder`; add `.pay_cost(...)` builder hook covering suspend/trash-N/return-self shapes; expose `select_count_capped_multi` at cost-time for selection-gated payment variants.

---

## Cluster Map

Gap clusters referenced above (`Cluster A`–`D` come from the Medusamon/DNA Omnimon/Rocks/Dark Masters audits). Each cluster groups gap entries that share the same infrastructure class.

| Cluster | Infrastructure class | Key gap entries | Phase that closes it |
|---|---|---|---|
| A | Observer timing dispatch | `OnOpponentSecurityRemoved`, phase-granular timings, `WouldBeDeleted` replacement framework | Phase 1 (partial), Phase 5+ |
| B | Zone manipulation primitives | play-from-zone, return-to-zone, reveal pool, hatch, security ops | Phase 2 (core), open sub-items |
| C | Native keyword system | Rush, Jamming, Blocker, Raid, Piercing, Blitz, Security A.±N, De-Digivolve N, Draw N | Phase 3 |
| D | Selection-kind expansion | ordered permutation, union-zone, count-capped multi, opponent-as-selector, DNA-pair | Phase 4 (partial — DNA-pair and cross-side-target open) |
| E | Cost-reduction hooks | BeforePayCost scan, closure-valued reduction, selection-gated cost payment | Phase 5 (planned) |
| F | Dynamic DP / modifier conditions | `.dp_modifier_fn`, condition-gated `ModifierEntry`, new `Expiry` variants | Phase 6+ |
| G | Keywords: Progress, Armor Purge, Training, Delay, Ace Overflow, DiGiBurst | New `Keyword` + `EffectTiming` variants, leave-field replacement framework | Phase 6+ (depends on A-residual) |
