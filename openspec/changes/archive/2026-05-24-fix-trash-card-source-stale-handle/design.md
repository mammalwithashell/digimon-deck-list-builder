## Context

A new panic family `G-DSL-TRASH-SOURCES-STALE-HANDLE` surfaced in run `generalist_1m_v2` (game 9728, recorder action 87, TS Olympos vs Yellow Tamer-heavy, turn 10). Backtrace:

```
trash_card_source (effect_context/mod.rs:~4087)
  ← zone_moves.rs:211   ← TrashSelectedSources DSL loop
  ← run_tail_preserving_trigger_context  ← `then:` block of select_own_sources
  ← install_select_own_sources::closure$2
  ← select_own_sources::closure$0   ← final_callback in install_source_multi_selection
  ← install_source_multi_selection (recursive, picked.len()==max → final_callback)
  ← install_source_multi_selection (initial install — captured the stale candidate)
  ← resolve_generic_selection
  ← decode_action ← ReplayRunner::step
```

The replay shows EX10-033 Pyramidimon and EX10-032 Proganomon's [WD] effects firing in sequence on the same digivolve action. EX10-033 Clause B (`select_own_sources(min:0, max:3) → trash_selected_sources`) ran first and trashed both EX8-050 sources from slot 0. EX10-032 Clause 2 (`select_own_sources(min:1, max:1)`) installed its SourceMulti picker; the picker's `action_to_source` snapshot captured a now-stale candidate referencing one of the already-trashed EX8-050s. The agent submitted that candidate, the DSL tail ran `trash_card_source(slot 0, stale_card_handle)`, the `.expect("card not in this permanent's stack")` fired.

DCGO reference (`DCGO/Assets/Scripts/Script/CardController.cs:5181` `ITrashDigivolutionCards.TrashDigivolutionCards()`):

```csharp
// Guard chain — silent yield break on every "carrier or target gone" case
if (_trashTargetCards == null) yield break;
if (_permanent == null) yield break;
if (_permanent.TopCard == null) yield break;
if (_permanent.HasNoDigivolutionCards) yield break;

// LIVE re-validation against permanent.DigivolutionCards
_trashTargetCards = _trashTargetCards.Filter((cardSource) =>
    _permanent.DigivolutionCards.Contains(cardSource) &&
    !cardSource.CanNotTrashFromDigivolutionCards(_cardEffect));

if (_trashTargetCards.Count == 0) yield break;
// ... actually trash the survivors ...
```

Caller branches on the actually-trashed set via `IsTrashed(card)` / `TrashedCards` (`TrashDigivolutionCardsAndProcessAccordingToResult` at `CardEffectCommons.cs:541`). DCGO never panics on a stale source reference — the trash primitive is declarative ("trash these if possible") and the outcome is observable from the returned set.

Adjacent Rust specs that frame the territory: `permanent-deletion-semantics`, `zombie-permanent-cleanup` (already encodes the read-after-empty sibling — `Permanent::top_card` panic, fixed in PR #533 / #539 by `Game::soft_remove_if_emptied`), `dsl-card-scripting-vocabulary` (declares the DSL surface for `select_own_sources` / `trash_selected_sources`). None of those specs' requirements change — this change adds a new capability that sits beside them.

## Goals / Non-Goals

**Goals:**
- Eliminate the `G-DSL-TRASH-SOURCES-STALE-HANDLE` panic class deterministically. The recording-derived replay test MUST pass after the change without changing card YAML.
- Bring `trash_card_source` into structural parity with DCGO `ITrashDigivolutionCards.TrashDigivolutionCards`: soft-fail primitive that filters live, never panics on rules-natural fizzles.
- Bring `install_source_multi_selection`'s pick callback into structural parity with DCGO's live-list picker contract: a candidate that vanishes between display and submit MUST NOT be acted on as if it were live; the picker re-prompts with current candidates.
- Preserve every existing DSL card-script as-is. The DSL surface (`select_own_sources { then: trash_selected_sources }`) keeps the same shape.

**Non-Goals:**
- **Full DCGO-shape picker** (per-permanent two-step SelectPermanent → SelectCard with `customRootCardList: selectedPermanent.DigivolutionCards`). That is the Tier 3 of the diagnosis; it's a larger refactor and is not required to close the panic class. Track separately if desired.
- **Recorder/replay JSON schema fix** ([`code/digimon-engine/src/runners/replay.rs:134`](code/digimon-engine/src/runners/replay.rs:134) expects `initial_state` at the top of the JSON; the recorder nests it under `recording`). Separate chip — its only impact is dev-friction running `digimon-engine-cli replay <recorder_output>`. Worked around in tests by jq-extracting the inner `recording` object.
- **Generalizing soft-fail to other trash/move primitives** (`return_card_source_to_hand` already returns `bool`; `trash_top_source` returns `bool`; `armor_purge_top` panics by design as it's used inside the `<Armor Purge>` keyword install where the gate is upstream). No change to those.
- **Changing observer-fan-out behavior** — `fire_digivolution_card_trashed`'s synchronous queue drain ([game_actions.rs:3230](code/digimon-engine/src/game_actions.rs:3230)) is intentional per EX10-036's [WD] interleaving test. We accept the interleaving and make the trash primitive tolerant.

## Decisions

### D1: Return `bool` from `trash_card_source` (not `Result` / not `Option<CardHandle>`)

**Decision**: `pub fn trash_card_source(...) -> bool` where `true` = trashed, `false` = no-op (any soft-fail condition).

**Rationale**:
- DCGO's analog returns `void` and exposes the actually-trashed set via a separate `IsTrashed(card)` / `TrashedCards` accessor on the holder class. In Rust the equivalent for our DSL caller is just "did this specific call succeed", which is `bool`.
- `Result` implies an actionable error — but every "failure" path here is rules-natural (stale handle, missing carrier, empty stack are all expected outcomes during observer-interleaved resolution). Forcing callers to handle `Result` would push noise into call sites that don't need to discriminate why.
- `Option<CardHandle>` returning the trashed handle would carry strictly more information than callers currently use; the bool is the minimum sufficient signal and matches the sibling `return_card_source_to_hand`'s shape.

**Alternative considered**: keep return type `()` and have callers pre-validate via a new `EffectContext::can_trash_card_source(perm, card) -> bool` helper before calling. Rejected: pushes the validation logic out to every caller (5+ sites), creates a TOCTOU between check and call, doesn't fix the upstream `expect`s in `trash_card_source`.

### D2: Re-install (not partial-validate-and-recurse) on stale pick in `install_source_multi_selection`

**Decision**: When the submit callback detects the picked card has vanished, refuse the action by re-invoking `install_source_multi_selection` with the unchanged prior `picked` set and freshly enumerated candidates (`source_multi_candidates` is already called at the top of `install_source_multi_selection`, so the re-install pays no extra cost).

**Rationale**:
- The agent's submitted action_id is now invalid; the cleanest recovery is to refuse it and re-present a valid menu. The picker is now in "after observer cascade" state; offering the agent the current candidates is the truthful UX.
- Preserves prior valid picks (the agent doesn't lose work on a multi-pick where only one entry went stale).
- Same code path as the recursive install used after a valid pick — minimal new code, easy to reason about, idempotent re-install is the engine's existing pattern.

**Alternative considered**: silently drop the stale pick and final-callback with `picked` minus the stale ref. Rejected: violates the picker's own min-count invariant (e.g., `min:1, max:1` would final-callback with 0 picks); harder to reason about; doesn't match DCGO's re-prompt semantics.

**Alternative considered**: panic with a different message indicating "agent submitted a stale pick" — assume it's a logic error. Rejected: it's not a logic error, it's the expected outcome whenever observer drains race with player input. Mainline path.

### D3: DSL step keeps its surface; only the bool is ignored

**Decision**: `CompiledStep::TrashSelectedSources` becomes:
```rust
CompiledStep::TrashSelectedSources { source_refs } => {
    if let Some(source_refs) = bindings.get_source_refs(source_refs) {
        for source_ref in source_refs {
            let _ = ctx.trash_card_source(source_ref.permanent, source_ref.card);
        }
    }
    true
}
```

`TrashUnionBound`'s Material arm gets the same `let _ = ...` treatment.

**Rationale**:
- Zero card-YAML changes required; DSL vocab is unchanged.
- The "actually trashed" set isn't observably needed by current cards. EX10-033 Clause B's `per_selected` over `chosen_sources` (cost reduction) iterates the BOUND set, not the trashed-set; that's the printed-text semantics ("for each card trashed" — and since the cost is paid by `select_own_sources` having picked them, the reduction applies per picked even if the trash itself fizzles for one). DCGO's `EX10_033.cs` `trashedCount = cards.Count` uses the post-select count from the selector callback, which mirrors our bound-set semantics.
- Future expansion (binding `__actually_trashed` for cards that branch on outcome) is a clean follow-up without changing this change's surface.

**Alternative considered**: expose the trashed set as a fresh DSL binding (`then: trash_selected_sources { actually_trashed_as: foo }`). Rejected as overscope; no current card needs it.

### D4: Don't change `<Fragment>` install, `trash_all_sources`, `trash_top_n_digivolution_cards_of_each`, or `trash_bottom_face_down_source` call sites

**Decision**: These callers already pre-validate (re-check stack membership before calling, or operate within a single-carrier scope), so `let _ = ctx.trash_card_source(...)` is a mechanical change at each call site with no behavior shift.

**Rationale**: Existing safety nets remain in place; the new soft-fail just collapses the residual edge case where pre-validation isn't sufficient (e.g., a future card adds inter-source observer cascades).

### D5: New capability `source-trash-soft-fail` rather than modifying `permanent-deletion-semantics` or `zombie-permanent-cleanup`

**Decision**: Create a new spec rather than amending adjacent ones.

**Rationale**:
- `permanent-deletion-semantics` covers the batched-delete flow (`delete_permanents_batch`, `OnDeletion`); source-trash is a stack-mutation operation that doesn't delete the carrier. Different concern.
- `zombie-permanent-cleanup` covers `soft_remove_if_emptied` and `shift_handle_after_soft_remove` — the read-after-empty side of the staleness family. This change is the write-after-shift side, distinct in mechanism (panic in `trash_card_source` vs `top_card`) and in fix (caller-level soft-fail vs `soft_remove` plumbing). Keeping them as separate capabilities makes archival traceable.
- Cross-references in this design.md and in the family entry in `panic-families.json` thread the relationship.

## Risks / Trade-offs

- **[Risk]** Soft-fail could mask a future legitimate bug where a stale handle indicates a real engine invariant violation (e.g., a refactor that breaks how SourceSelectionRef is constructed). **Mitigation**: add a `debug_assert!` or `tracing::warn!` on the soft-fail path so dev builds surface the no-op for inspection while release builds silently continue. The PendingSelection-resolution metrics emit `panic_count_by_family` to TensorBoard; we should add a counterpart `soft_fail_count_by_site` so training runs surface unusual rates.

- **[Risk]** Re-install on stale pick (D2) could loop indefinitely if the candidate list also goes empty between re-install and player input. **Mitigation**: `install_source_multi_selection` already handles the empty-candidates case (line 2548: `if candidates.is_empty() || picked.len() == max as usize { if picked.len() >= min as usize { final_callback(game, picked); } return; }`). Re-install would hit the same branch — either final-callback with prior valid picks, or no-op if picked < min. Need a unit test for both branches.

- **[Risk]** Existing behavioral tests in `tests/effect_context/trash_card_source.rs` likely assert the panic on stale handles (the panic IS the contract today). Those assertions need to flip to expecting `false`. **Mitigation**: enumerate the affected tests in `tasks.md`; convert them as a single sub-task.

- **[Trade-off]** Tier 1 (D1) alone closes the panic but leaves stale candidates visible in the picker — agents may waste a turn picking phantoms. Tier 2 (D2) closes that UX hole. We're doing both in this change; if D2 surfaces unforeseen complexity, D1 alone is shippable.

- **[Trade-off]** Not adopting the full DCGO per-permanent picker (Tier 3 / Non-Goal) means the SourceMulti UI still shows "Select source N on slot M" rather than DCGO's "Pick a Digimon, then its sources". That's a UX gap, not a correctness gap. Acceptable for this change; track Tier 3 separately if desired.

## Migration Plan

This is an engine-only change with no on-disk or wire-format impact.

1. Land the Rust changes (signature flip + body softening + picker revalidation) behind no feature flag — the new soft-fail behavior is strictly safer than the current panic.
2. Run the full test suite (`cargo test --manifest-path code/digimon-engine/Cargo.toml`) — converted `trash_card_source` tests must pass with inverted assertions; new regression tests must pass.
3. Run the Python-side parity test (`DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v`) — no parity change expected since the Python engine isn't running these card scripts.
4. Smoke-replay the captured recording: `target/debug/digimon-engine-cli.exe replay <extracted_recording>.json --step 87 --verify` — must complete without panic.
5. Tag the panic-families entry as `resolved` once a clean training run lands without recurrence.

Rollback: revert the PR. No data migration, no state on disk that depends on the new behavior.

## Open Questions

- **Q1**: Should `soft_remove_if_emptied(perm)` still run when `trash_card_source` returns `false`? Current design says no (early-return before `soft_remove` call). Rationale: if no card moved, no cleanup needed. Confirm there's no scenario where `card_sources` was emptied by a sibling effect and the failed `trash_card_source` was supposed to be the soft-remove trigger. Probably handled elsewhere — but worth one focused audit pass.

- **Q2**: Should the picker-revalidation re-install (D2) also re-run the `filter` on prior valid picks (in case the filter result changed after observer cascades)? E.g., a picked source whose host carrier lost the trait gate. DCGO's analog re-evaluates `CanSelectCardCondition` on each picker re-display. Probably yes for parity, but the cost is N filter calls per re-install. Decide before implementing or as a follow-up.

- **Q3**: Worth adding a `tracing` span around `trash_card_source` to capture (perm, card, outcome) for diagnostic logs during training? Or rely on `panic_count_by_family` going to zero as the only signal? My instinct: emit a `tracing::debug!` on soft-fail with the four-tuple (perm, card, current stack contents) so post-hoc analysis of `runs/.../console.log` can identify which cards exercised the new path. Cheap and reversible.
