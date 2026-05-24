## Why

Optional triggered clauses whose `condition` references `event_*` predicates (e.g. `event_target_owner`, `event_target_kind`, `event_card_color_has`) silently auto-fire instead of surfacing the outer accept/decline prompt the printed text demands. The bug was observed on BT16-085 Davis & Ken's `[Your Turn] When one of your Digimon digivolves into a blue or green Digimon, by suspending this Tamer, gain 1 memory` clause during a Paildramon DNA digivolve — the Tamer was suspended and +1 memory granted without the player ever being asked. This breaks the no-approximations policy in [CLAUDE.md](CLAUDE.md): every "you may" choice must reach the RL action space so agents can learn to decline. The root cause is in `Game::queued_effect_wants_outer_optional_prompt` in [effect_queue.rs:2833](code/digimon-engine/src/effect_queue.rs:2833): the function evaluates `effect.condition` without first installing the queued effect's `trigger_context`, so every `event_*` predicate short-returns `None`, the condition appears to fail, and the prompt is skipped — even though the body's condition gate (which runs *after* `run_queued_effect` installs the context at [effect_queue.rs:2024](code/digimon-engine/src/effect_queue.rs:2024)) passes and the effect fires.

## What Changes

- Install `qe.trigger_context` via `TriggerContextGuard` around the condition evaluation in `queued_effect_wants_outer_optional_prompt`, mirroring the pre-cost-prompt branch at [effect_queue.rs:802-814](code/digimon-engine/src/effect_queue.rs:802). The `outer_optional_guard` evaluation at [effect_queue.rs:2883](code/digimon-engine/src/effect_queue.rs:2883) is in the same function and must see the same context.
- Change the function's signature from `&self` → `&mut self` to allow installing/restoring the trigger context guard.
- Add a regression behavioral test on BT16-085: after a same-controller blue-or-green digivolve, `runner.pending_selection()` MUST be `Some` AND `pending_selection().is_optional == true` BEFORE any drain step accepts the trigger. Today's test at [bt16_085.rs:552-612](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs:552) deliberately tolerates either auto-fire or prompt; the new test pins the prompt as the only correct behavior.
- Add a parallel regression test on a sister Tamer with the same pattern (BT17-081 or AD1-010) so the fix is documented as cross-card.

## Capabilities

### New Capabilities
- `optional-trigger-outer-prompt`: When a triggered clause has `optional: true` and its body's first step is non-declinable (so the outer accept/decline prompt is the only decline path), the engine MUST surface that prompt to the controller whenever the clause's condition would pass *with the queued trigger context installed*. Condition gates that reference event-context predicates (`event_target_owner`, `event_target_kind`, `event_card_color_has`, etc.) MUST evaluate against `QueuedEffect::trigger_context`, not against the engine's ambient (likely stale or empty) `current_trigger_context`.

### Modified Capabilities
<!-- No existing spec owns this contract — `dsl-card-scripting-vocabulary` covers declinable activation costs (the `activation_cost_fn` pre-cost-prompt path), which is a sibling concern but a distinct surface. -->

## Impact

- **Code**: [code/digimon-engine/src/effect_queue.rs](code/digimon-engine/src/effect_queue.rs) — `queued_effect_wants_outer_optional_prompt` (signature + body) and its single call site at line 857.
- **Tests**: new behavioral assertions in [code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs](code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs) plus a sibling test on BT17-081 or AD1-010.
- **Card coverage**: cards confirmed affected by direct read of YAML — BT16-085 (this report), BT17-081 (sister Davis & Ken Tamer with the same `on_digivolve` + `optional: true` + suspend-self pattern), AD1-010 (other sister Tamer). Any other `optional: true` triggered clause whose `condition` references `event_*` predicates AND whose first body step is mandatory is also affected — the audit can be scoped by grepping YAML for `optional: true` clauses with `event_*` condition leaves.
- **No DSL surface change**: card scripts are correct as authored; only runtime evaluation order is wrong.
- **No tensor / action-space change**: the additional pending selection is already a known `SelectionKind::Replacement` shape (see `install_outer_optional_trigger_selection`) — the action space already encodes accept/decline. This change makes it actually appear in cases that should always have shown it.
- **RL impact**: existing trained policies have never observed this prompt for the affected cards. Re-training or fine-tuning may be needed to teach agents to evaluate the cost/reward tradeoff (e.g. should Davis & Ken stay unsuspended to enable a future free Veemon free-play rather than burn the suspend now for +1 memory).
- **Cross-engine parity**: this is Rust-engine-only. The Python sunset engine (`code/engine_py_legacy/`) is not in scope.
