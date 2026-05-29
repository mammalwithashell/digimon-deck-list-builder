## Context

The DSL's `bind_as` mechanism is used by both intra-resolution selection verbs (e.g. `select_own_permanent: { bind_as: tgt }`) and play verbs (`play_from_hand_free: { bind_as: played }`). Both currently store a positional `PermanentHandle { player: u8, index: u8 }` into the `Bindings` map ([`bindings.rs:11-12`](../../../code/digimon-engine/src/dsl_cards/bindings.rs)). For selections this is correct — selections live and die inside a single effect resolution, and the battle-area shape cannot mutate between bind and consume.

For play-then-schedule patterns this is wrong. `schedule_delayed` ([`scheduled_effects.rs:32-51`](../../../code/digimon-engine/src/scheduled_effects.rs)) captures the current `Bindings` map verbatim and replays them into a fresh `EffectContext` at a future drain boundary. Between the schedule point and the drain, any number of stack-changing events can mutate the battle area:

- **Regular digivolve** ([`game.rs`](../../../code/digimon-engine/src/game.rs)): preserves the permanent at its slot; new top card pushed onto `card_sources`. The slot's identity from the played card's POV has changed — the played card is no longer the top.
- **DNA digivolve** (`dna_digivolve_inner`): `target_a` keeps its handle (possibly with a `-1` index shift if `target_b` had a lower index); `target_b` is removed; both stacks' `card_sources` are merged under the new top.
- **Deletion + reslot**: a permanent at a lower index gets deleted; permanents at higher indices shift down by one.

Today's resolver treats `played` as a live positional handle, finds whatever permanent sits at `(player, index)` now, and applies the verb (e.g. `return_to_hand`) to it. The user-visible failure mode for **BT16-085 Davis Motomiya & Ken Ichijoji** + **BT16-025 Paildramon** is documented in the proposal: the merged Paildramon stack gets bounced (top card to hand, three digivolution cards to trash) instead of the bounce silently fizzling.

The engine already has the right primitive elsewhere. `ProvenanceToken` ([`trigger_context.rs:27-34`](../../../code/digimon-engine/src/trigger_context.rs#L27)) is a trivial `From<CardHandle>` wrapper (`token = card.0 as u64`). Every `CardHandle` has a deterministic token; no separate minting step exists or is needed. `Game::resolve_provenance_token` returns `Some(EventSubject::Permanent(handle))` when the card identified by the token appears anywhere in a battle-area permanent's `card_sources` — top OR digivolution card. Two cards (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki) already use this via `ScheduleDeletePlayedAtTurnEnd` for end-of-turn self-deletion of the played Digimon.

## Goals / Non-Goals

**Goals:**

- Make `bind_as` on play verbs (`play_from_hand_free`, `play_from_revealed_free`, `play_from_materials`, `play_union_bound_free`, `play_token`) survive stack-changing events between schedule and drain by binding a stable identity, not a positional handle.
- Preserve current behavior for intra-resolution selection bindings (`select_*: { bind_as: ... }`). They keep producing positional handles — they do not outlive their resolution and don't need provenance.
- Match DCGO's [`BT16_085.cs`](../../../DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_085.cs) identity model: the bounce silently fizzles when the played Permanent is no longer a battle-area top (consumed in DNA, buried by digivolve, deleted by another effect).
- Preserve `ScheduleDeletePlayedAtTurnEnd`'s permissive carrier-deletion semantics (EX11-022 Karakurumon, EX11-061 Mirai Kinosaki, P-165 ShoeShoemon — the carrier is the correct deletion target even after a digivolve buries the played card).

**Non-Goals:**

- Re-architecting `PermanentHandle` to be a non-positional identity. Positional handles are correct and efficient for the vast majority of binding lifetimes (one effect resolution). Only the cross-resolution case needs provenance.
- Changing `dna_digivolve_inner`'s merge logic. The merge is correct; the bug is in how the played-permanent binding is captured, not in how the stack is built.
- Tracking digivolution cards' card-side modifiers across the merge. That is a broader concern (`modifiers.rs` keyed by `PermanentHandle`) and is independent of this change.
- Backporting to the Python engine. Python is sunsetted (CLAUDE.md rule 22); this is a Rust-only fix.

## Decisions

### Decision 1: Add a new `BindingValue` variant rather than changing `Permanent`'s payload

**Choice**: introduce `BindingValue::PlayedPermanent { token: ProvenanceToken, fallback: PermanentHandle }` as a sibling of `BindingValue::Permanent(PermanentHandle)`.

The `fallback` carries the schedule-time handle, used only for diagnostics (e.g. `get_played_permanent` test getter, debug logging).

**Why this over alternatives:**

- *Alternative A*: change `BindingValue::Permanent` to `Permanent { handle, token }`. **Rejected** — every selection-produced binding (the vast majority) would have to mint a synthetic token, the binding-ref resolver would have to choose between handle-fast-path and token-resolve-path on every consume, and the type churn touches every step that consumes a permanent binding. The fix is concentrated on play-verb sites; isolating the variant keeps the blast radius small.
- *Alternative B*: keep `Permanent(handle)` and add a parallel "identity" map on `Bindings`. **Rejected** — splits a single conceptual binding across two storage slots, making it easy for a step author to consume the handle without ever consulting the identity check.

### Decision 2: Two resolvers serve two different semantics (strict + permissive)

**Choice**: add `Game::resolve_token_as_battle_area_top(token) -> Option<PermanentHandle>` as a strict helper. Yields `Some(handle)` only when the card identified by `token` is currently the **top card** of a battle-area permanent. Keep the existing permissive `Game::resolve_provenance_token` for `ScheduleDeletePlayedAtTurnEnd`'s carrier-aware semantics.

`resolve_binding_ref` intercepts `BindingValue::PlayedPermanent` and routes it through the strict helper. A sibling resolver `resolve_played_permanent_permissive(name, ctx, bindings)` in `binding_ref.rs` calls the permissive lookup for `ScheduleDeletePlayedAtTurnEnd`.

**Why two resolvers**: the printed-text semantic differs by card. Davis & Ken's "return *it*" implies "the Digimon I played" — once "it" is no longer a Digimon (it's a digivolution card under another top), the effect has no valid referent (DCGO behavior). Karakurumon's "delete the Digimon this effect played" implies "the carrier wrapping the played card" — even after a digivolve, the carrier is the right deletion target (existing engine behavior).

### Decision 3: Bind on success; do nothing on play-failure

**Choice**: play verbs mint and insert the token binding ONLY if the play succeeds and produces a permanent handle. On failure (replaced/redirected/blocked/play primitive returns None), no binding is inserted. Downstream consumers see no binding and skip silently.

**Why**: matches the existing `record_played` log behavior. The new variant doesn't introduce a "failed play" sentinel; it inherits the existing "no binding = skip" convention.

### Decision 4: Token lookup at bind time, not at primitive return

**Choice**: the play primitives (`ctx.play_from_hand_free`, `ctx.play_from_revealed_free`, etc.) keep returning `PermanentHandle`. The new helper `bind_played_with_provenance` reads `ctx.game.player(played.player).battle_area.get(played.index).top_card().handle()` at bind time to derive the `ProvenanceToken`.

**Why**: keeps the play primitives' signatures unchanged. The token derivation is a single lookup at the bind site (one indirection through battle_area), trivial cost. If the primitive returned a handle but the slot is somehow empty by bind time, the bind is skipped silently (same convention).

## Risks / Trade-offs

- **[Risk] Existing cards that depended on the buggy positional-handle behavior break silently.** → Mitigation: audit `code/digimon-engine/cards/**/*.yaml` for cards combining a play verb's `bind_as` with `schedule_delayed`. Only BT16-085 matches in the current codebase. If a future card actually *wants* the positional-handle behavior, it should be expressed with a different verb (e.g. binding a `PermanentHandle` via `select_own_permanent` first), not via play-verb `bind_as`.

- **[Risk] Two existing dsl-suite tests call `bindings.get_permanent(...)` directly on play-verb bindings.** → Mitigation: switch them to the new diagnostic getter `bindings.get_played_permanent(...)`. Both are unit tests asserting the binding was inserted; the semantics are unchanged.

- **[Risk] Provenance token lookup is O(battle-area-size); a queue of many scheduled effects could compound the cost.** → Mitigation: battle areas are ≤ field-slots (typically 5-7 permanents) and scheduled-effect queues are small. The total cost is dominated by the existing scheduled-drain plumbing. No optimization needed.

- **[Trade-off] Bindings become slightly noisier to debug.** A `BindingValue::PlayedPermanent { token: ProvenanceToken(N), fallback: PermanentHandle { player: …, index: … } }` print is longer than `BindingValue::Permanent(handle)`. → Acceptable; the derived `Debug` impl is at least as informative.

## Migration Plan

This is a backward-incompatible behavioral fix in the engine, not in user-facing YAML. No card YAML needs to change. Migration is:

1. Land the engine change behind no feature flag — the fix is a bug fix, not a new feature.
2. Run the full Rust test suite and confirm zero NEW failures (baseline has 4 pre-existing failures unrelated to this change).
3. Add the new behavioral tests under `code/digimon-engine/tests/cards_behavioral/bt16/bt16_085.rs` to lock in the fixed behavior.
4. No production rollout step — the desktop and hosted-API builds pick up the engine update on their next rebuild.

Rollback: revert the change. No data migration; this is pure runtime logic.

## Open Questions — Resolved

- **Provenance token minting**: `ProvenanceToken` is a trivial `From<CardHandle>` cast. Every `CardHandle` has a deterministic token; no separate minting step exists or is needed.
- **Existing resolver semantics are too permissive** for the strict use case. `Game::resolve_provenance_token` returns `Permanent(handle)` if the card appears **anywhere** in a battle-area permanent's `card_sources`. Resolution: add a separate strict helper that additionally verifies the card is the top.
- **Cards combining play-verb `bind_as` with `schedule_delayed`**: audit revealed only BT16-085 in the current codebase. BT22-089's `play_from_hand_free` has no `bind_as`. AD1-010 binds selection verbs only. BT13-110 uses materials-play (different path). BT1-090's `schedule_delayed` does `gain_memory`, not anything binding-targeted.
