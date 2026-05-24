## ADDED Requirements

### Requirement: Outer accept/decline prompt installs when an optional triggered clause's condition would pass under the queued trigger context

When `drain_effect_queue` resolves a single-trigger bundle whose `QueuedEffect` is optional (`qe.is_optional == true`), has no top-level `activation_cost_fn` (so the pre-cost branch did not handle it), and was flagged by the DSL lowering as `needs_outer_optional_prompt`, the engine SHALL install a `SelectionKind::Replacement` accept/decline `pending_selection` (the "outer optional prompt") whenever the clause's `condition` would pass *with the QueuedEffect's `trigger_context` installed*. The engine MUST NOT skip the prompt because of a `current_trigger_context` mismatch that the queued effect's body would not itself observe at run time.

#### Scenario: Optional on_digivolve clause with event_target_owner condition surfaces a prompt

- **WHEN** BT16-085 Davis & Ken is on player 0's field unsuspended, player 0 digivolves one of their own Digimon into a Lv.5 Blue/Green Digimon (DNA or normal), and `drain_effect_queue` reaches Davis & Ken's `on_digivolve` clause (`optional: true`, condition `all_of: [event_target_owner: you, event_target_kind: digimon, event_card_color_has: [blue, green]]`)
- **THEN** before `run_queued_effect` runs the clause body, `pending_selection` is `Some` with `kind == Replacement`, `selecting_player == 0`, `is_optional == true`, and `valid_action_ids` contains the accept action id

#### Scenario: Declining the prompt skips the body entirely

- **WHEN** the outer optional prompt from the previous scenario is installed and the controller submits the decline action (`pass_or_decline` / `on_decline` callback)
- **THEN** the Tamer is NOT suspended, no MemoryChange event is emitted from this clause, and the queue drain resumes with the clause's QueuedEffect dropped

#### Scenario: Accepting the prompt runs the body with the queued trigger context

- **WHEN** the outer optional prompt is installed and the controller submits the accept action
- **THEN** `run_queued_effect` installs the QueuedEffect's `trigger_context`, the clause body's condition gate re-evaluates and passes, the suspend-self step suspends Davis & Ken, and a `MemoryChange { delta: 1, player: 0 }` event is emitted

#### Scenario: The prompt is suppressed when the condition would fail under the queued trigger context

- **WHEN** an optional `on_digivolve` clause's condition would NOT pass with the QueuedEffect's trigger context installed (e.g. opponent's Digimon digivolves and `event_target_owner: you` fails), and the clause is `optional: true` with a non-declinable first body step
- **THEN** the outer prompt is NOT installed, the QueuedEffect is silently dropped, and no body steps run

### Requirement: Outer optional condition evaluation uses the queued effect's trigger context, not ambient state

The condition check inside the outer-optional-prompt decision (`Game::queued_effect_wants_outer_optional_prompt` or equivalent) SHALL evaluate predicates against the `QueuedEffect::trigger_context` value captured at enqueue time. Implementations MUST temporarily install that context (e.g. via `TriggerContextGuard`) before invoking the condition closure and MUST restore the prior `current_trigger_context` value afterwards (including on panic), so that subsequent drain iterations and concurrent observers are not contaminated. The same context MUST be visible to the `outer_optional_guard` (body-actionability guard) call in the same function.

#### Scenario: event_target_owner sees the queued trigger context

- **WHEN** an optional triggered clause whose condition is `event_target_owner: you` is checked for prompt eligibility, and the QueuedEffect's `trigger_context.event_permanent.player` equals the queued effect's controller
- **THEN** the predicate evaluator MUST observe `rctx.game.current_trigger_context == Some(qe.trigger_context)` during the call and the predicate returns true

#### Scenario: event_card_color_has sees the queued trigger context

- **WHEN** an optional triggered clause whose condition is `event_card_color_has: [blue, green]` is checked, and the QueuedEffect's `trigger_context.event_card` resolves to a card whose colors intersect `[blue, green]`
- **THEN** the predicate returns true and the prompt is installed (subject to other gates)

#### Scenario: Trigger context restoration on panic

- **WHEN** the condition closure or `outer_optional_guard` panics during the outer-optional-prompt decision
- **THEN** `Game::current_trigger_context` is restored to the value it held before the decision was entered (the guard's `Drop` impl runs), and no subsequent drain step observes a leaked context

### Requirement: Existing pre-cost-prompt and run_queued_effect paths preserve their current trigger-context behavior

The pre-cost-prompt branch (`needs_pre_cost_prompt`) and the `run_queued_effect` body-execution path MUST continue to install the QueuedEffect's `trigger_context` before evaluating conditions or running body steps. This requirement exists to prevent regressions: the outer-optional fix changes only the third path (outer-optional decision), and the sibling paths must continue to behave as today.

#### Scenario: Pre-cost prompt still evaluates condition under installed trigger context

- **WHEN** a single optional trigger with a top-level `activation_cost_fn` (e.g. `suspend_self` lifted as `ActivationCost`) is in the bundle and its condition references `event_*` predicates
- **THEN** the pre-cost prompt installs iff the condition passes with the QueuedEffect's trigger context installed, matching today's behavior at the call site that already uses `TriggerContextGuard::install`

#### Scenario: Body execution still installs trigger context

- **WHEN** any QueuedEffect is run via `run_queued_effect_inner`
- **THEN** `current_trigger_context` is assigned the QueuedEffect's `trigger_context` before the body process closure runs, and restored to the prior value after
