## Context

The Rust engine models each battle-area `Permanent` as `{ card_sources: Vec<CardSource>, linked_cards, option_state, ... }`. The cardinal invariant is: **a `Permanent` in `Player::battle_area` has a non-empty `card_sources` Vec, with the topmost source the visible "body" of the permanent**. `Permanent::top_card()` (`permanent.rs:131-135`) is the canonical accessor and bakes this invariant into a panicking `.expect("Permanent must have at least one card")`.

Two failure modes can violate the invariant:

1. **Deletion**: a permanent dies. PR #525 routed deletion through `Game::delete_permanents_batch`, which (a) takes a pre-removal snapshot, (b) trashes the carrier (clears `battle_area` slot), then (c) fires `OnDeletion` against the snapshot. After PR #525 there is no window where a deleted permanent has empty `card_sources` AND is still in `battle_area`.

2. **Material extraction**: a permanent's only card is moved elsewhere (digivolved onto another permanent, played as a free play, returned to hand, trashed, parked under another permanent as a bottom source, placed into security). The carrier did not "die" — its body just left. Without explicit cleanup the slot remains in `battle_area` with empty `card_sources`. This is the **zombie permanent** class.

PR #533 closed mode (2) for ONE path — `effect_initiated_digivolve_from_source_inner` — by introducing `Game::soft_remove_if_emptied(handle)`. Called immediately after the source mutation, the helper:
- Removes the now-empty slot from `battle_area`.
- Routes any `linked_cards` to trash and fires per-player `OnLinkedCardTrashed` (mirroring `combat::finalize_permanent_deletion`).
- Returns `true` if the slot was removed; the caller pairs with `Game::shift_handle_after_soft_remove` to adjust any in-flight `PermanentHandle` for the index shift.

Crucially `soft_remove_if_emptied` is NOT a deletion (no `OnDeletion`, no replacement window, no carrier-body trash — the body already moved). It is the analog of DCGO's `CardObjectController.RemoveField(permanent)` (`DCGO/.../CardController.cs:1509`), not `DestroyPermanentsClass.Destroy()`.

The same 2026-05-23 PR added Layer 2 defensive guards in three queue-side functions (`Game::top_card_handle`, `Game::enqueue_from_permanent`, `Game::queued_effect_source_is_live`) so that if a zombie slips past Layer 1, the trigger queue tolerates it.

Static audit of the post-PR-#533 codebase identified that:
- `Game::soft_remove_if_emptied` is called from EXACTLY ONE production site (`game_actions.rs:6481`).
- 6 other production code paths still mutate `card_sources` in a way that can empty the carrier without calling `soft_remove_if_emptied`.
- 2 effect-queue read-side callers (`find_event_gated_delay_permanent`, `event_gated_delay_source`) iterate `battle_area` calling raw `top_card()` and panic on any zombie. They are NOT covered by PR #533's Layer 2 set.

The training run `pilot_ppo_20260523_215003` (started ~2h after PR #533 landed) captured 10 fresh panics of the same family at ~0.25 panics/min — the same order as the pre-fix rate. The remaining siblings are clearly hot enough to block long-running training.

## Goals / Non-Goals

**Goals:**
- Close every remaining mutation sibling that can produce a zombie permanent in `battle_area`, using the established `soft_remove_if_emptied` cleanup pattern.
- Extend Layer 2 defensive guards to the two remaining unguarded effect-queue iterators (`find_event_gated_delay_permanent`, `event_gated_delay_source`).
- Add behavioral regression tests — one per closed mutation sibling — that mirror the existing digivolve-from-material zombie-test pattern in `code/digimon-engine/tests/effect_context/effect_digivolve_from_zones.rs`.
- Bring the engine-gaps.md and panic-families.json tracking artifacts into sync with reality (gaps.md prematurely marked the family RESOLVED; JSON correctly still says open).
- Unblock the generalist training run by eliminating the `Permanent must have at least one card` panic class.

**Non-Goals:**
- **Architectural refactor of `Permanent::top_card()` to return `Option<&CardSource>`**. DCGO's `Permanent.cs:1352-1367` `TopCard` returns null and all callers null-check; that is the systemically-correct shape and is already noted as a follow-up in engine-gaps.md's "Family-wide note". This change deliberately stays within PR #533's chosen Layer 1 + Layer 2 pattern so the fix is small, mechanical, and reviewable. The architectural refactor is a parallel track with ~40 caller sites and a much larger blast radius; deferring it does not block this change.
- Auditing every `top_card()` caller in `combat.rs`, `dsl_cards/predicate.rs`, `dna_digivolve.rs`, etc. These read-side sites are part of the architectural-refactor track. This change only hardens the two effect-queue read-side sites whose panic surface is provably reachable from the closed mutation siblings.
- Replaying captured Python `_draw_crash.json` recordings through the Rust engine for end-to-end validation. `RustHeadlessGame::new()` accepts only decks + seed; recording-driven replay needs an initial-state-injecting constructor. That is a follow-up that helps triage future panic families but is not required to validate this fix — the per-sibling unit tests are sufficient.

## Decisions

### Decision 1 — Reuse the existing `soft_remove_if_emptied` helper rather than introduce a new abstraction

`Game::soft_remove_if_emptied(handle)` is already the correct primitive for this class of cleanup: it idempotently removes empty slots, handles `linked_cards` flush + `OnLinkedCardTrashed`, and reports back whether a remove occurred so callers can fix up their in-flight handles via `shift_handle_after_soft_remove`. PR #533 designed it deliberately to be called from any caller, not just digivolve. **Decision: call the existing helper from each sibling; do not introduce a parallel helper.**

Alternative considered: a wrapper like `take_card_source_ref_and_cleanup` that bundles the take + soft-remove. Rejected because the take and the cleanup are not always adjacent (e.g., `place_as_bottom_source_observed` takes, then conditionally restores on failure, then pushes under target — the cleanup must come AFTER push success, not adjacent to take). A wrapper would only fit half the sibling sites.

### Decision 2 — `play_from_materials_suppress_on_play` soft-remove placement (post-success only)

`play_from_materials_suppress_on_play` (effect_context/mod.rs:3299-3376) has three outcomes from `play_from_hand_with_cost_result_from_origin_suppress`:
- `Played(field_index)` — the play committed; the source card is now its own permanent. Carrier may be empty.
- `Pending` — the play parked a selection; outcome decided later. Carrier may be empty; rollback NOT applicable.
- `Failed` — the play was rejected (e.g., battle-area full). The rollback path at lines 3361-3373 reinserts the source back into `target.card_sources[source_index]`.

**Decision: call `soft_remove_if_emptied` ONLY on the `Played` branch (after `record_played`), and NOT on the `Pending` branch.** Rationale:
- `Failed`: source is reinserted; carrier is not empty; nothing to clean up. Calling soft-remove would be a no-op but is unnecessary.
- `Pending`: a parked selection MAY resume and trash the source / cancel the play. If we soft-remove the carrier slot now, a later resume that needs to restore the source would have no slot to restore to. Defer cleanup to the resume callback (which will go through the same code path post-resolution).
- `Played`: source committed; carrier safely empty; soft-remove now.

This is more conservative than the digivolve fix (which always runs `soft_remove_if_emptied` after the `digivolve()` call) because the play path has a deferred-pending failure mode the digivolve path doesn't.

Alternative considered: soft-remove unconditionally after the play attempt and add a corresponding "re-add slot" path to the Failed rollback. Rejected because (a) re-adding a slot to `battle_area` from inside a failure rollback would re-introduce the index-shift complexity for any other in-flight handles, and (b) the `Pending` case still requires deferral.

### Decision 3 — Layer 2 guards mirror the existing `enqueue_from_permanent` pattern

PR #533's `enqueue_from_permanent` guard is a 2-line early-return: `if perm.card_sources.is_empty() { return; }`. Apply the same shape to `find_event_gated_delay_permanent` (skip zombies inside the iter loop with `continue`) and `event_gated_delay_source` (return `None` when the source perm is empty). No new helper; no new abstraction; the comment cites `G-PERMANENT-EMPTY-DURING-BATCH-DELETION` so future readers find the context.

### Decision 4 — One regression test per closed sibling

PR #533 added 4 regression tests in `effect_digivolve_from_zones.rs` covering the digivolve-from-material variant. Mirror that density for each of the 6 mutation siblings:
- 6 base "emptying source leaves no zombie" tests
- Plus 1 each for the index-shift case where it materially applies (the digivolve fix needed this; not every sibling does — `trash_source_ref` mutates the source perm itself, no separate target handle to shift)

Total: ~8-10 new tests. The base shape is `place_on_field(... 1-source carrier ...); call sibling op; assert battle_area.iter().all(|p| !p.card_sources.is_empty())`.

### Decision 5 — Tracking artifact reconciliation

The gaps.md entry was prematurely marked `RESOLVED 2026-05-23 (mis-named; actually digivolve-from-material zombie)` while panic-families.json correctly kept `status: "open"`. **Decision: split the gaps.md family entry into two**:
- `G-PERMANENT-EMPTY-DIGIVOLVE-FROM-MATERIAL` — resolved by PR #533
- `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` — open until this change lands; sibling list above; cite this change's PR when closed

panic-families.json gains the new family_id and resolves the original once this change lands. This keeps the resolved fix discoverable (gaps.md prose) without misleading future panic-triage that the broader class is closed.

## Risks / Trade-offs

- **[Risk] `Pending` branch in `play_from_materials_suppress_on_play` may leave a zombie carrier until resume.** → Mitigation: that's a temporary state inside one `Game::step()` call. The Layer 2 guards on `enqueue_from_permanent` / `queued_effect_source_is_live` already tolerate zombies for the duration of a parked selection. The remaining unguarded read sites (`find_event_gated_delay_permanent`, `event_gated_delay_source`) are also guarded by this change. End of `step()` callback resumes (or fails) the play and runs the cleanup. No observer sees a zombie outside the trigger-queue read path.

- **[Risk] `Game::trash_source_ref` is called from selections the agent makes ("trash 1 of your sources"); fixing it changes observable state when the agent picks the carrier's only source.** → Mitigation: pre-change behavior is a PANIC, so any change is strictly an improvement. Post-change behavior is "carrier is silently removed from battle_area" — the same shape DCGO produces (`CardObjectController.RemoveField`). Add a regression test that explicitly picks the single-source carrier's only source and asserts clean removal.

- **[Risk] Save / Stash effects under `place_as_bottom_source_observed` (sibling #2) are common; the index shift on cleanup may affect downstream effect-script bindings.** → Mitigation: the digivolve fix already proved out `shift_handle_after_soft_remove`; the same primitive applies here. The new regression tests include the lower-indexed-source variant the digivolve fix needed.

- **[Risk] Hidden caller of `take_card_source_ref` or `card_sources.remove(pos)` is missed by the audit.** → Mitigation: grep + lint pass listed in tasks.md (an explicit task is "verify every `card_sources.remove`, `card_sources.drain`, and `take_card_source_ref` call site has either a soft-remove follow-up OR a top-protecting guard"). A grep-based check in CI is overkill for one panic family, but the manual audit is the explicit gate before claiming completeness.

- **[Risk] Layer 2 guards mask future Layer 1 regressions.** → Mitigation: the guards only kick in when the Layer 1 invariant is already violated. As long as the test suite continues to exercise the Layer 1 path (each sibling has a regression test asserting `battle_area.iter().all(|p| !p.card_sources.is_empty())`), a Layer 1 regression fails the test BEFORE the Layer 2 guard masks it. This is the same trade-off PR #533 already accepted.

- **[Trade-off] We're patching siblings rather than refactoring `top_card()` to `Option`.** → Acknowledged. The architectural refactor is the systemically correct fix and IS the right long-term direction; engine-gaps.md "Family-wide note" already tracks this. This change unblocks training in a focused, mechanical, reviewable way using the pattern PR #533 already chose. When the architectural refactor lands, the per-site `soft_remove_if_emptied` calls become defense-in-depth rather than the primary safety mechanism.
