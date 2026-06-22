# make-engine-cloneable — Resumable-VM Migration Playbook

> Generated 2026-06-18 by the `cloneable-installer-migration-playbook` Workflow
> (16 read-only analysis agents — one per remaining selection installer/trampoline —
> + a synthesis pass). `Game: Clone` already landed (tasks 4.1/4.2); this is the
> ordered plan to extend *faithful* clone from the resume path to **every** decision
> point by porting the remaining ~16 installers onto the resumable VM.

## Current state (updated 2026-06-22 — Batch 0 substrate + Batch 1 flips COMPLETE)
- **Batch 0a/0b DONE:** `ResumeSelectKind` has all 9 RunTail kinds — `Hand`, `Trash`, `FieldPermanent` (own+opp), `Security`, `BreedingPermanent`, `AnyPermanent`, `Reveal`, `Material`, `UnionZone` — each with a source-verified `run_resume` decode arm.
- **Decline model DONE:** `ResumeDecline {None, RunTail{tail, aborts_clause}}` — the 3-way optional-decline semantics (no-decline / run-a-tail / cost-abort), incl. dual-tail (breeding/union_zone).
- **Batch 0c DONE (count_capped keystone):** `ResumeFrame::MultiPickStep(MultiPickState)` + `run_multipick_step` + `install_multipick_step` (re-park) + the data terminal (binds the accumulated list, runs the tail — the former `Arc<Mutex<Box<dyn FnOnce>>>` final-callback as data). distinct_by ported. The "post-stack final-callback channel" blocker is resolved by making the terminal plain data.
- **Batch 1 DONE (commit 6aaccd265):** the 3 mechanical installers (`install_select_trash` / `install_select_security` / `install_select_own_breeding_permanent`) now park a `RunTail` frame alongside the legacy closure. Gated GREEN on the full pool: `cards_behavioral` **5825/5825** (0 failed), `dsl` 781/781, `lib`+resume units 250/250 (15 resume units, incl. a nested-composition guard).
- **NESTED-RESUME OUTER-TAIL COMPOSITION DONE (the big Batch-1 learning):** flipping ANY installer needs this shared substrate — the playbook mis-rated Batch 1 as "mechanical." EX11-044 proved it: clause_a trashes its own sources → fires clause_b's `on_digivolution_card_trashed` interrupt (a flipped `select_trash`) → clause_a's pending tail got `wrap_pending_selection_with_tail`-wrapped onto the **bypassed** closure and silently dropped (a loud tripwire surfaced it). **Fix:** `wrap_pending_selection_with_tail` is now resume-aware — it composes the outer tail as **data** (`resume::OuterContinuation` pushed onto the frame's `outer_conts`) instead of onto the closure; `run_resume` runs the conts after the inner/decline tail via the **same** `drain_or_rewrap_pending_tail` path, so a deeper nested select re-composes recursively. `dsl_clause_aborted` scoping + the `dsl_resolved_tail_bindings` freshness channel are preserved (byte-for-byte parity with the closure wrapper). A `MultiPickStep` frame reaching a wrap still panics loudly (Batch 4 extends it).

**Remaining:** Batches 2-3 (flip the remaining RunTail installers to emit frames — **now ride on the nested-composition substrate, so they really are mechanical**; parity-gated by `cards_behavioral`); Batch 4 (port the other 6 trampolines — source-multi/dp-budget/play-cost-budget/reveal-bucket/permutation/partition — reusing the MultiPickStep executor pattern with their own terminal binds + candidate types, **plus** extend `outer_conts` composition to `MultiPickStep` frames); then delete the legacy closures + the whole-Game clone-replay capstone (task 4.2).

---

## Batch 0 — Shared core (do FIRST, single PR; nothing flips until this lands)

### 0a. Add all RunTail `ResumeSelectKind` variants (one diff to `resume.rs`)
Add 8 variants beside `Hand`:
- `Trash { of_player }`
- `OwnPermanent { selector: Option<CompiledFieldSelector>, selected_field: SelectedField }`
- `OpponentPermanent { of_player }`
- `AnyPermanent { candidates: Vec<(u16, PermanentHandle)> }`
- `Reveal { of: CompiledPlayerRef }`
- `Material { perm, filter_bindings, source_card, source_permanent, source_kind, player }`
- `Security { of_player }`
- `BreedingPermanent { of_player }`
- `UnionZone { of_player, zones: UnionZoneSet }`

**Clone-discipline gate:** every new field must be `Clone` (the whole point). `CompiledFieldSelector`, `SelectedField`, `UnionZoneSet`, `CompiledPlayerRef`, `Bindings` are already `Clone`. If `Material`'s frame ends up serialized for MCTS, watch the bincode `skip_serializing_if` gotcha (memory `reference_dsl_substrate_authoring_gotchas`) — use `#[serde(default)]`, not `skip`.

### 0b. Implement the 9 RunTail decode arms in `run_resume`
Each mirrors the `Hand` arm at `selections.rs:269` (push_effect_target → build `EffectContext` → `insert_*` → `run_tail_preserving_trigger_context`). Decode tables:

| Kind | action_id → value | bind call |
|---|---|---|
| Trash | `- TRASH_EFFECT_START (1150)` | `insert_trash_index(name, of_player, idx)` |
| OwnPermanent / OpponentPermanent | `(action_id - ATTACK_START) % TARGETS_PER_ATTACKER` → field idx; player fixed | `insert_permanent(name, handle)` |
| AnyPermanent | **linear search** `candidates` for matching action_id | `insert_permanent` |
| Reveal | `- SEL_REVEAL_START (30)`; **upfront ownership check** via `revealed_owner_matches` | `insert_card(name, handle)` |
| Material | `material_zone_geometry(game, perm)` → `range_start`; `- range_start` | `insert_card` |
| Security | base = `SEL_MY_SECURITY_START (40)` vs `SEL_OPP_SECURITY_START (50)` by ownership | `insert_card` |
| BreedingPermanent | `== BREEDING_SELECTION_TARGET (99)`, no arithmetic | `insert_breeding_permanent_ref` |
| UnionZone | **3-way branch** on action range (Hand/Trash/Material) → resolve CardHandle | `insert_card` (zone-tagged) |

### 0c. Write the `MultiPickStep` executor (the keystone — replaces the `unimplemented!`)
This is the single hardest piece of work and gates all 7 trampolines. It must:
1. Decode the resolving `action_id` against the frame's `candidates`/zone (zone-aware: `range_start`).
2. Append to `accum`; recompute remaining candidates.
3. Decide: **re-park** another `MultiPickStep` (more picks needed) or **drain** to `then` (count satisfied / exhausted / PASS).
4. On PASS: honor `min`/`is_optional_zero` semantics, then run `then`.
5. Run the terminal continuation. **Blocker to resolve here:** the trampolines' `final_callback` is `Box<dyn FnOnce(&mut Game, Vec<CardHandle>)>` — *not* data. The data model only carries `then: Box<ResumeFrame>`. The executor needs a **post-stack callback channel** (parallel to `dsl_resolved_tail_bindings` / `dsl_outer_tail`) so the final callback runs *after* the frame stack drains, not inline. **Decide and build this channel in Batch 0** — every MultiPickStep installer depends on it. (permutation spec calls this out explicitly; count_capped/source-multi/partition all reframe `final_callback` as a terminal `RunTail` or a published callback.)

### 0d. Extend the spike test
Add an `OwnPermanent` (or `Trash`) RunTail unit test and one `MultiPickStep` unit test (two-pick accumulator → terminal) mirroring `runtail_hand_runs_inner_tail_as_data` + `game_clone_is_independent_and_replays_identically`. These guard the core before any installer flips.

**Batch 0 parity gate:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test '*' resume` plus the new unit tests. Nothing in `cards_behavioral` changes yet (no installer flipped) — this batch is pure additive substrate.

---

## Batch 1 — Mechanical RunTail flips (DONE 2026-06-22, commit 6aaccd265)

> **Re-rating:** these 3 were rated "mechanical" but flipping the first one exposed a shared blocker the playbook missed — **nested-resume outer-tail composition** (an interrupt's flipped select being wrapped by an outer clause's tail). That substrate was built here (see "Current state") and now all RunTail flips ride on it, so Batches 2-3 are genuinely mechanical. Closure body still maps 1:1 to a decode arm, no selector/candidate state:

1. **`install_select_trash`** (`1794-1888`) — Trash kind. Carries `decline_aborts_clause: cost` (the only one here with a real cost-abort path). Parity: `BT13-075, BT13-088, BT16-040, BT17-007, BT17-019, BT17-095, BT17-097, BT19-072, BT21-007, EX10-036, EX10-069, EX11-023` + all `CompiledStep::SelectTrash`.
2. **`install_select_security`** (~`2960`) — Security kind. `decline_aborts_clause: false`. Parity: cards with `CompiledStep::SelectSecurity`.
3. **`install_select_own_breeding_permanent`** (`3387-3463`) — BreedingPermanent. `decline_aborts_clause: false`; success_tail = `then ++ tail`, decline_tail = `tail`. **Best-tested target** — parity suite is concrete: `tests/dsl/phase2g_breeding_selection.rs` (11 tests) + `cards_behavioral/bt13/bt13_093`, `bt20/bt20_083`, `bt24/bt24_089`, `p/p_103`.

**Flip pattern (all):** after `ctx.select_*`, if `pending_selection.is_some()` set `pending_selection_resume = Some(ResumeStack{frames: vec![RunTail{...}]})` carrying the *same* Arc/bindings/runtime/trigger_context the closure captured. Legacy closure stays as panic-guard during coexistence.

---

## Batch 2 — Moderate RunTail flips (selector/candidate state)

4. **`install_select_own_permanent`** (`~1893-1975`) — carries `selector` + pre-computed `selected_field`; pre-filters empty candidates (early return). `decline_aborts_clause: false`. Note `continue_on_decline` is **not** ported here (decline is future MultiPickStep work). Parity: grep `select_own_permanent|SelectOwnPermanent`.
5. **`install_select_opponent_permanent`** (`2179`) — OpponentPermanent. **Add a capture**: `let opponent_player = opponent;` before the `select_*` call (spec notes it's computed but not currently captured). Parity: cards with `SelectOpponentPermanent` (destroy/bounce/status on opp field).
6. **`install_select_any_permanent`** (`2272-2363`) — AnyPermanent; frame must carry the **full `candidates: Vec<(u16, PermanentHandle)>`** (heterogeneous both-player domain; decode = linear search). Parity: `EX8-028` + selector-filtered `SelectAnyPermanent`.
7. **`install_select_reveal`** (`2806`) — Reveal; **upfront ownership check** in the resume arm (`revealed_owner_matches`), and stale-index → silent skip-bind. Parity: grep `SelectReveal` in fixtures; optional reveal PASS must route through `is_pass` gate.
8. **`install_select_material`** (`2910`) — Material; decode via `material_zone_geometry` (breeding-vs-battle routing). Parity: "choose 1 of {perm}'s digivolution sources" cards. Watch `reference_debugrunner_empty_evo_costs` if tests synthesize carriers.

---

## Batch 3 — Hard RunTail (dual-tail)

9. **`install_select_union_zone`** (`3386-3542`) — rated **hard** despite RunTail shape: routes success vs decline to **different tails** (`success_tail` vs `decline_tail`) and decodes across **three action ranges** (Hand/Trash/Material, incl. breeding). Frame carries both tails; decline path runs via the `is_pass` branch with `decline_aborts_clause = cost`. **Do this only after Batches 1-2 prove the RunTail decode arms.** Parity: `BT24-029`/`BT24-031` (King Drasil union-bond), `Beelzemon_X02` (BT24); verify all 5 cases (hand-bind, trash-bind, material decode, cost-decline-aborts, non-cost-decline-runs-decline_tail).

---

## Batch 4 — MultiPickStep trampolines (hardest; serialize, do NOT parallelize first)

All 7 depend on the **Batch-0c executor + final-callback channel**. Convert `Arc<Mutex<Option<Box<dyn FnOnce>>>>` recursion into `accum`/`candidates` data + a `then` frame. Recommended order (simplest accumulator → most state):

| Order | Installer (loc) | Distinguishing state / risk |
|---|---|---|
| 1 | **`install_permutation_step`** (`2463-2529`) | Simplest: ordered accumulate, no min/max gating, no per-pick filter. **Best first MultiPick** to validate the executor + callback channel. `OrderedPermutation{items, accum}`. |
| 2 | **`install_count_capped_step`** (`3341-3586`) | Canonical; zone-aware decode (Hand/Trash/Material), `is_optional_zero`, `distinct_by`. This is the reference the executor was designed against — `CountCappedZone{of_player, zone, range_start}`. |
| 3 | **`install_dp_budget_selection`** (`~2991-3101`) | Budget accumulator (`remaining_dp`); candidates carry `(u16, handle, i32 cost)`; recursion reduces budget. `DpBudgetPick{opponent}`. |
| 4 | **`install_play_cost_budget_selection`** | Mirror of #3 over play-cost. `PlayCostBudget{remaining_play_cost, picked}`. |
| 5 | **`install_reveal_bucket_step`** (~2180 lines) | Multi-*bucket* state machine: `bucket_index`, per-bucket min/max, cross-bucket `no_duplicate_cards` dedup, empty-bucket auto-advance. `then` is the **next bucket**, not the final callback. Largest single function. |
| 6 | **`install_source_multi_selection`** (`2819-2953`) | Recursive source trampoline; decode `decode_source_select`/`decode_breeding_source_select`; binding happens in user `final_callback` (no `insert_*` in frame). |
| 7 | **`install_partition_source_selection`** (`2675-2767`) | **Hardest:** heterogeneous `requirements: Vec<PartitionRequirement>` gate each pick differently; candidates **recomputed per recursion** (`partition_next_candidates`), not captured. Depends on #6's source-decode being proven. |

**Batch 4 parity gates (per installer):** count_capped → all `G-SELECT-MULTI-*` (up-to-N, distinct_by, optional-zero, material-from-breeding). dp/play-cost → `select_opponent_permanents_by_dp_budget` / `..._by_play_cost_budget` cards (accept / early-decline / multi-pick / min-threshold-PASS). source-multi → `G-DSL-OWN/OPP/TRASH-SOURCES`, tamer-sources. partition → `select_partition_sources` cards (likely needs a **new minimal 2-requirement test card** authored — spec notes none may exist). permutation → grep `select_ordered_permutation`. reveal-bucket → grep `reveal_bucket` (verify bucket advance, dedup, empty short-circuit, min/max PASS gating).

---

## Hardest items & blockers (call-outs)

- **B1 — MultiPickStep executor + final-callback channel (Batch 0c).** The single keystone. The data model carries `then: Box<ResumeFrame>` but the 7 trampolines' terminal is a `Box<dyn FnOnce(&mut Game, Vec<CardHandle>)>`. You must add a **post-stack callback channel on `Game`** (parallel to `dsl_resolved_tail_bindings`/`dsl_outer_tail`) and run it after the stack drains. Get this wrong and every MultiPick card silently runs its continuation at the wrong time. Validate with `install_permutation_step` first (simplest accumulator).
- **B2 — `install_partition_source_selection`** is the hardest installer: per-requirement heterogeneous gating + candidates recomputed each recursion. Sequence it last and only after `install_source_multi_selection` lands.
- **B3 — `install_reveal_bucket_step`** (~2180 lines): its `then` advances to the *next bucket*, a nested state machine — not a flat accumulator. Budget extra time; its cross-bucket dedup is the subtle correctness trap.
- **B4 — `install_select_union_zone`** is RunTail-shaped but tri-range + dual-tail; gate it behind the simpler RunTail decode arms.
- **Coexistence invariant (all batches):** `resolve_generic_selection` runs `run_resume` when `pending_selection_resume.is_some()`, else the legacy closure. Keep the legacy closure installed (as a panic-guard where tests assert the data path) until the whole migration is green, then a final cleanup PR can delete the closure paths.
- **B0 — nested-resume outer-tail composition (SOLVED in Batch 1).** When a flipped select is installed as an interrupt nested inside another clause, that outer clause's remaining tail is composed via `wrap_pending_selection_with_tail`, which historically wrapped the **closure** — bypassed by `run_resume`, so the tail was silently dropped. This is shared substrate every RunTail flip needs (not per-installer). Now handled: wrap composes the tail as data (`resume::OuterContinuation` on the frame's `outer_conts`), run after the inner/decline tail via `drain_or_rewrap_pending_tail`. **Batch 4 must extend `outer_conts` to `MultiPickStep` frames** (they currently panic loudly on a wrap).
- **Parity-test infra risk:** the `cards_behavioral` binary (~5800 tests) has two known env failure modes — non-deterministic stack-overflow abort (set `RUST_MIN_STACK=268435456`) and `LNK1104` from a hung prior run (kill the holding PID). Per memory `reference_cards_behavioral_flaky_crash`; neither is a regression, don't chase them as parity failures.
- **Build isolation:** verify in an isolated `CARGO_TARGET_DIR='D:\cargo-target-wt\elated-hopper-9f42c2'` to avoid phantom cross-worktree compile errors (memory `reference_cargo_target_per_worktree`).

## Recommended PR sequence
1. **PR-0:** Batch 0 (variants + 9 RunTail arms + MultiPickStep executor + callback channel + unit tests). No installer flips.
2. **PR-1:** Batch 1 (3 mechanical flips) — fastest parity win, proves the RunTail flip pattern end-to-end on `phase2g_breeding_selection.rs`.
3. **PR-2:** Batch 2 (4 moderate RunTail flips).
4. **PR-3:** Batch 3 (`union_zone`).
5. **PR-4..N:** Batch 4, one installer per PR in the order above (permutation → count_capped → dp → play-cost → reveal-bucket → source-multi → partition), each gated on its own `cards_behavioral` targets.
6. **PR-final:** delete legacy closure callbacks; assert `Game: Clone` + the clone-independence/replay property test (`game_clone_is_independent_and_replays_identically`) as the capstone gate.

Key files: `code/digimon-engine/src/resume.rs` (variants/executor), `code/digimon-engine/src/dsl_cards/step/selections.rs` (`run_resume` ~221, RunTail installers), `code/digimon-engine/src/effect_context/selections.rs` (the 7 MultiPickStep installers), `code/digimon-engine/src/effect_queue.rs` (`resolve_generic_selection` dispatch).