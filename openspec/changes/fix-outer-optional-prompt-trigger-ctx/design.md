## Context

`drain_effect_queue` in [code/digimon-engine/src/effect_queue.rs](code/digimon-engine/src/effect_queue.rs) is the single dispatch point for triggered effects. For a single-trigger bundle (`bundle.len() == 1`) the loop walks three decision branches before running the effect:

1. **Pre-cost-prompt** ([effect_queue.rs:756-832](code/digimon-engine/src/effect_queue.rs:756)) — for optional triggers carrying a top-level `activation_cost_fn` (e.g. `suspend_self` lifted as a `CompiledStep::ActivationCost`). Installs `TriggerContextGuard::install(self, trigger_context)` around the condition closure call, then routes through `install_trigger_order_selection(chooser, &bundle, true)` to expose accept/decline before the cost fires.
2. **Outer-optional-prompt** ([effect_queue.rs:857-860](code/digimon-engine/src/effect_queue.rs:857)) — for optional triggers whose decline path is "skip the entire body" (the DSL lowering flagged `needs_outer_optional_prompt` because the body's first step is not in the declinable-first-step allow-list at [lower_triggered.rs:295-330](code/digimon-engine/src/dsl_cards/lower_triggered.rs:295)). Calls `queued_effect_wants_outer_optional_prompt(&qe)` which evaluates the clause's condition and (when implemented) the `outer_optional_guard`, then installs a `Replacement`-kind accept/decline `pending_selection`.
3. **Run inline** ([effect_queue.rs:861](code/digimon-engine/src/effect_queue.rs:861)) — when neither of the above gates fires, `run_queued_effect` is called directly. `run_queued_effect_inner` at [effect_queue.rs:2024](code/digimon-engine/src/effect_queue.rs:2024) sets `self.current_trigger_context = qe.trigger_context.clone()` before running the body and restores it after.

The bug: branch 2's condition evaluation at [effect_queue.rs:2864-2878](code/digimon-engine/src/effect_queue.rs:2864) does NOT install the queued effect's trigger context. `EffectReadContext::new_with_source_kind(self, ...)` borrows `self.current_trigger_context` as-is — usually `None` after the previous drain restored it, or stale from an unrelated trigger. Every `event_*` predicate ([predicate.rs:1614-1734](code/digimon-engine/src/dsl_cards/predicate.rs:1614)) starts with `let trigger = rctx.game.current_trigger_context.as_ref()?` and returns `None` when missing, making the parent predicate at [predicate.rs:1289-1295](code/digimon-engine/src/dsl_cards/predicate.rs:1289) return `false`. The condition fails, the function returns `false`, and the prompt is skipped — but `run_queued_effect_inner` then installs the correct context and the same condition gate now passes, so the body fires silently. Net effect: optional cost paid, optional reward granted, player never asked.

Reproduction: [scripts/mcp_paildramon_dna_confirm_choice.py](scripts/mcp_paildramon_dna_confirm_choice.py) drives the digimon-engine-mcp through a DNA digivolve into BT16-025 Paildramon with BT16-022 ExVeemon, BT12-050 Stingmon, and BT16-085 Davis & Ken on field. Three `MemoryChange { delta: 1 }` events fire on the second-material `resolve_selection` call; no Davis & Ken prompt ever appears; Davis & Ken's permanent ends `is_suspended: true`.

The existing behavioral coverage at [bt16_085.rs:552-612](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs:552) does not catch this — its `while let Some(view) = runner.pending_selection_view()` loop is tolerant of no-prompt and still passes when the body auto-fires. The comment at line 587-588 explicitly hedges *"if an outer accept/decline prompt installs"*.

## Goals / Non-Goals

**Goals:**

- The outer-optional decision in `queued_effect_wants_outer_optional_prompt` evaluates `effect.condition` and `effect.outer_optional_guard` against the **same** trigger context that `run_queued_effect_inner` would install for the body — so a clause that fires inline today MUST install the outer prompt tomorrow.
- Trigger-context installation is panic-safe (RAII guard) and does not leak across the drain loop's iterations.
- A new behavioral test pins the prompt as the only correct behavior for BT16-085, and a sibling test covers at least one other affected card (BT17-081 or AD1-010) to lock in the cross-card contract.
- Pre-cost-prompt path and body-execution path retain their current trigger-context handling (no regressions on the sibling code paths).

**Non-Goals:**

- Surveying every YAML for affected clauses. The audit is out of scope; the test suite, once expanded to cover one or two representative cards per pattern, will catch regressions on the rest implicitly via `cargo test`.
- Re-training existing RL policies. They've never seen this prompt and may pick suboptimally; that's a separate training concern documented in proposal Impact.
- Touching the Python sunset engine. The bug is Rust-only.
- Changing the DSL surface or the YAML for any affected card. Card scripts are correct as authored.
- Reworking the bundle-size-> 1 dispatch branch (multi-trigger TriggerOrder selection at [effect_queue.rs:893-908](code/digimon-engine/src/effect_queue.rs:893)). That path is correct for the BT16-085 scenario because the OnDigivolve drain queues only Davis & Ken's clause (BT12-022 and BT12-050 fire on the earlier `before_pay_cost_observe` dispatch, not as queued OnDigivolve observers).

## Decisions

### Decision 1: Change `queued_effect_wants_outer_optional_prompt` from `&self` to `&mut self`

The fix needs to mutate `current_trigger_context` (via `TriggerContextGuard::install`, which assigns into `self.current_trigger_context`). The function is called from one site only — `drain_effect_queue` at [effect_queue.rs:857](code/digimon-engine/src/effect_queue.rs:857) — and that site already holds `&mut self`, so the signature change is mechanical.

**Alternatives considered:**

- *Inline the check at the call site*, mirroring the verbose `needs_pre_cost_prompt` pattern at lines 756-828. Rejected because the function exists precisely to keep that branch readable — inlining 70+ lines back into the drain loop would undo a previous refactor and make the call site even harder to follow than today.
- *Make the function a free function taking `&mut Game`*. Same outcome as Decision 1 but worse for grep-ability and discoverability; chose the method form for consistency with `install_outer_optional_trigger_selection` and `install_trigger_order_selection`, which are also `&mut self` methods on `Game` and live in the same impl block.

### Decision 2: Install the trigger context via `TriggerContextGuard::install` (the existing RAII guard)

The pre-cost branch already uses this guard at [effect_queue.rs:802-803](code/digimon-engine/src/effect_queue.rs:802). Reusing it gives panic-safety for free (the `Drop` impl at [effect_queue.rs:69-72](code/digimon-engine/src/effect_queue.rs:69) restores the previous value) and keeps the two branches stylistically aligned. The guard is constructed once at function entry; both the `effect.condition` closure (line 2874) and the `effect.outer_optional_guard` closure (line 2883) MUST evaluate while the guard is in scope so they see the same context.

**Alternative considered:** *Manual save-and-restore inline (`let prev = self.current_trigger_context.take(); ...; self.current_trigger_context = prev;`)*. Rejected — not panic-safe, and the guard already exists for exactly this use case.

### Decision 3: Clone `qe.trigger_context` rather than borrowing

`TriggerContextGuard::install` takes ownership of an `Option<TriggerContext>`. The pre-cost branch clones at line 803 (`trigger_context` is a moved-out local; the call site clones into the local at line 776). Cloning is cheap (`TriggerContext` is a small struct of `Option<...>` fields) and the QueuedEffect outlives the guard scope because we move the QE out of the queue at line 833 *before* the outer-optional decision — so `qe.trigger_context.clone()` is safe and aligned with the sibling branch.

**Alternative considered:** *Pass a reference and adjust `TriggerContextGuard` to accept `&TriggerContext`*. Rejected — would require changing the guard's API for one extra caller, and the clone cost is negligible compared to the predicate evaluation work that follows.

### Decision 4: Test the prompt via the MCP-style drive (pending_selection assertion), not via raw runner action ids

The new regression test should assert `runner.pending_selection().is_some()` AND `pending_selection().is_optional == true` AND `pending_selection().kind == SelectionKind::Replacement` **immediately after** the digivolve action and `drain_effect_queue`, before any user action accepts/declines. This is what `mcp_paildramon_dna_confirm_choice.py` validates externally, lifted into a Rust unit test.

**Alternative considered:** *Expand the existing tolerant test at line 552-612 to assert the prompt*. Rejected — better to leave the tolerant test intact (covers the post-accept assertions, which remain valid) and add a sharper test that focuses solely on the prompt-installation contract. Names: `bt16_085_optional_outer_prompt_installs_on_dna_digivolve` and `bt16_085_optional_outer_prompt_installs_on_normal_digivolve`. Add the BT17-081 analog as `bt17_081_optional_outer_prompt_installs_on_own_digivolve`.

### Decision 5: Scope to the single-trigger bundle branch only

The multi-trigger TriggerOrder branch at [effect_queue.rs:893-908](code/digimon-engine/src/effect_queue.rs:893) doesn't call `queued_effect_wants_outer_optional_prompt` — it installs the TriggerOrder prompt directly and resolves each pick by calling `run_queued_effect` from the selection's callback. Per the comment at lines 837-845, the two branches are mutually exclusive in practice. This change does not touch the multi-trigger path. If a future scenario surfaces a similar context-leak bug there (e.g. an `any_mandatory` bundle where one optional trigger's condition uses `event_*` predicates and matters for whether it should be a pick option at all), that's a separate change.

## Risks / Trade-offs

- **[Risk]** The fix could cause prompts to appear in additional cases beyond BT16-085 — any optional triggered clause whose condition mixes `event_*` predicates with non-declinable first body steps is currently auto-firing and will start prompting. → **Mitigation:** This is the desired behavior per the no-approximations policy; the audit done in proposal Impact identifies the known affected cards (BT16-085, BT17-081, AD1-010). Run the full Rust engine test suite after the fix to surface any tests that asserted "no prompt" behavior; update those tests to assert "prompt then accept" using `runner.execute_action(view.selecting_player, view.valid_action_ids[0])`. Existing tolerant tests (like [bt16_085.rs:552-612](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs:552)) will keep passing because they already accept whatever pending appears.

- **[Risk]** RL agents may behave worse on affected cards because their training distribution has never included this prompt. → **Mitigation:** Document in the change archive; tracking re-training is out of scope. The action space is unchanged (the prompt uses an already-encoded `Replacement` shape), so no model architecture change is needed.

- **[Risk]** Smoke tests that hit DNA-digivolve scenarios may slow down slightly because of the additional pending-selection round-trip. → **Mitigation:** The extra work is one predicate evaluation + one allocation per affected trigger fire; negligible. Smoke tests already handle pending selections via auto-resolve or random-policy callbacks.

- **[Risk]** Subtle ordering bug if a future caller mutates the queue inside the condition closure while the guard is in scope. → **Mitigation:** Condition closures are pure reads (`Fn(&EffectReadContext) -> bool`) — they can't mutate the queue by design. This risk is structural-only, not introduced by this change.

- **[Risk]** Forgetting to restore the trigger context if the guard's `Drop` doesn't run (e.g. `std::mem::forget` on the guard). → **Mitigation:** The pre-cost branch is the canonical pattern and we mirror it exactly. No `forget` in either path.
