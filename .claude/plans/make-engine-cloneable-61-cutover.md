# make-engine-cloneable — 6.1 cutover plan (delete the legacy closure executor)

Produced 2026-07-01, after the full caller map + hooks defunctionalization session.
Supersedes the "what remains" sections of `make-engine-cloneable-nondsl-surface.md`
(Waves A–C are all DONE; the hooks channel is now data).

## Where we are

- **Production is fully resume-backed.** A transitive caller map of all 16
  `callback_only` primitives (including the `selections.rs` wrapper layer) found
  ZERO live-closure production call paths. Every production install parks a
  `ResumeStack`; `resolve_generic_selection` prefers resume; the closure halves
  are dead weight in production.
- **The hooks channel is data.** `ResumeContinuationHooks` holds typed
  `AfterSelectionHook` variants (play-cost / digivolve-reducer / option-reducer /
  DigiXros-leave / partition-second-play). `run_after_selections_drain` is
  deleted. Armed continuations survive `Game::clone`
  (guard: `q30_partition_second_play_hook_clones_faithfully_mid_chain`).
- **The only executors of `sel.callback` / `on_decline` left** are:
  1. ~45 legacy test files that hand-roll `CardEffect`s calling `ctx.select_*`
     WITHOUT parking resume (list below).
  2. The TEST-010/011/012 worked-example cards (`src/cards/test/`).
  3. `tests/replacements/partition.rs` via the test-only
     `select_partition_sources` primitive (raw `Box<dyn Fn>` requirements).

## The staged deletion (each step full-suite-gated)

1. **Migrate the legacy test files** (the grind — see list + strategies below).
2. **Rewrite TEST-010/011/012** as clone-safe worked examples: park a
   `ResumeFrame` directly (the rule-28 `RevealBucketStep` pattern) or re-author
   as DSL YAML. They are pilots/teaching cards — they should teach the
   VM-driven pattern, not the retired closure pattern.
3. **Decide `partition`**: the production `<Partition>` keyword uses the
   resume-backed count-capped path; `select_partition_sources` +
   `install_partition_source_selection` + the `partition_*` matcher helpers are
   test-only. Either port `PartitionRequirement.matches` to `CompiledPredicate`
   and flip the installer, or delete the API + rewrite
   `tests/replacements/partition.rs` against the production keyword
   (`Keyword::Partition` on a real carrier, e.g. the Q30 flow) — the latter is
   preferred (deletes ~400 lines of dead machinery).
4. **Delete the dead installers**: `install_partition_source_selection` (after
   step 3), plus any wrapper whose callers all migrated
   (`select_reveal_buckets`, `select_own_tamer_sources`,
   `select_sources_under_own_tamer`, `select_opponent_no_source_digimon` are
   already test-only).
5. **Neuter, then remove, the callback params**: change each `select_*` /
   `install_*` primitive to stop installing the caller's closure (install a
   loud-panic stub), run the FULL suite — anything that trips reveals a missed
   caller. Then drop the parameters + the `PendingSelection.callback` /
   `on_decline` fields entirely, `derive(Clone)` on `PendingSelection`, delete
   the panic-stub manual `impl Clone`, and delete `resolve_generic_selection`'s
   `else { sel.callback }` + closure `on_decline` branches.
6. **Flip the audit**: `audit_pending_selection_callbacks.py --strict-zero` in
   CI (`dsl-guards.yml` / `engine-clone-safety.yml`); the tool then guards
   against reintroduction.

## Legacy closure-path test files (45, by migration strategy)

Strategy A — **assert-on-state tests of a selection KIND's mask/candidates**
(mask_and_tensor/*, most of selection/*): re-express the installing effect as a
DSL-compiled card (the kind's real production path) or park a RunTail frame
with a compiled tail; assertions unchanged.

Strategy B — **callback-value tests** (the closure asserts which indices/handles
arrived): re-express the continuation as compiled steps operating on the bound
pick (bindings are visible in end-state assertions), or convert to a
`*_clones_faithfully`-style end-state assertion.

Strategy C — **subsystem-flow tests** (option_flow/*, replacements/*, combat/*,
cost_hooks/*): these mostly test the SUBSYSTEM (delay, decoy, redirect,
reducers) whose production installers are already resume-backed — swap the
hand-rolled closure effect for the production keyword/DSL card and the flow
assertions stand.

combat/progress_partial.rs, combat/redirect_and_cancel.rs,
cost_hooks/interactive_digivolve_reducer.rs, cost_hooks/interactive_option_use_reducer.rs,
cost_hooks/pay_cost_selection.rs,
effect_context/material_zone_select.rs, effect_context/no_source_targets.rs,
effect_context/opponent_stack_trashing.rs, effect_context/override_persistence.rs,
effect_context/security_stack_operations.rs, effect_context/source_move_under_tamer.rs,
effect_context/source_snapshot_rescue.rs, effect_context/under_tamer_hand_placement.rs,
effect_context/under_tamer_play.rs, effect_context/under_tamer_selectors.rs,
effect_context/under_tamer_trash_placement.rs, effect_context/under_tamer_union_placement.rs,
effect_source_kind/classification.rs, event_emission/effect_target.rs,
mask_and_tensor/breeding_selection_mask.rs, mask_and_tensor/dp_budget_selection_mask.rs,
mask_and_tensor/source_selection_mask.rs,
option_flow/delay_flow.rs, option_flow/event_gated_delay.rs,
option_flow/option_placed_observers.rs, option_flow/standard_flow.rs,
option_flow/start_delay_flow.rs, phase_flow/pending_selection_turn_end.rs,
replacements/attack_cancel.rs, replacements/nested_select_decoy.rs,
replacements/nested_select_fragment.rs, replacements/nested_select_save.rs,
replacements/nested_select_substrate.rs, replacements/partition.rs,
selection/breeding_permanent.rs, selection/count_capped.rs, selection/dp_budget.rs,
selection/material.rs, selection/opponent_selector.rs, selection/ordered_permutation.rs,
selection/reveal_buckets.rs, selection/source_multi.rs, selection/union_zone.rs
(+ src/cards/test/test_010.rs, test_011.rs, test_012.rs)

## Non-goals / already settled

- `zone_moves.rs`'s `CompiledStep::PlaceRemainderOnDeck` arm stays: reachable
  only when the reveal pool is empty (a no-op that installs nothing) — the
  selection-installing cases are intercepted in `selections::try_install`.
- No new closure APIs: the audit ratchet fails on any new `callback_only` site.
