# Track E — Zone Movement and Source/Material Operations: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the missing zone-movement primitives in `code/digimon-engine/`. Centralise zone movement so every move routes through observer-safe APIs, returns stable handles, and surfaces every player-visible choice through `pending_selection`.

**Architecture:** Extend the existing curated `EffectContext` API. Most of the foundational pieces — `PermanentHandle`, `CardHandle` (globally unique stable card-instance handle), `CardSource.face_down`, `CardSource.reveal_overlay`, `SourceSelectionRef { permanent, field_index, source_index, card }` — already exist. Helpers route through `Game` mutation chokepoints (`play_from_hand_with_cost_result`, `place_on_security_observed`, `effect_initiated_digivolve_from_source`, etc.) that fire the right observer timings. The work is largely (a) adding the genuinely missing helpers, (b) introducing an `EffectMoveToken` provenance scaffold for cleanup riders, (c) auditing call sites that take raw `usize` indices and converting to handles where the caller has a `CardHandle`, and (d) adding DSL schema + lowering for the new step verbs.

**Tech Stack:** Rust, PyO3 (via `code/digimon-engine-py/`), YAML DSL (parsed in `code/digimon-engine/src/dsl_cards/`).

---

## Audit summary — what's already there

The engine already implements much of the prompt's helper list. Below is the audit; tasks below address only the genuine gaps.

### Already implemented in `EffectContext` (`code/digimon-engine/src/effect_context/mod.rs`)

| Prompt requirement | Existing helper |
|---|---|
| `play_card_by_effect(Hand, …)` (with cost override) | `play_from_hand_with_cost`, `play_from_hand_free` |
| `play_card_by_effect(Trash, …)` | `play_from_trash_with_cost`, `play_from_trash_free_unsuspended` |
| `play_card_by_effect(SecurityFaceUp/FaceDown, …)` | `play_from_security` (via hand-transit through `play_from_hand_with_cost_result`) |
| `play_card_by_effect(OwnDigivolutionSource, …)` | `play_from_materials` |
| `play_from_hand_to_breeding` | `play_to_breeding_from_hand` |
| `play_token` | `play_token` |
| `digivolve_by_effect(Hand, …)` | `effect_initiated_digivolve` |
| `digivolve_by_effect(any source zone, …)` | `effect_initiated_digivolve_from_source` (consumes `CardSourceRef`) |
| Effect-initiated DNA digivolve | `effect_initiated_dna_digivolve` |
| `select_own_sources(filter, count)` | `select_own_sources` (in `effect_context/selections.rs`) |
| `trash_selected_sources(handles)` | `trash_card_source(perm, card)` (single) + `trash_all_sources(target)` (all-below-top) |
| `play_from_source(source_handle, …)` | `play_from_materials(target, source_index, cost_delta)` + `play_selected_sources_without_cost(Vec<SourceSelectionRef>)` |
| `pop_top_source(carrier_handle)` (trash variant) | `trash_top_source(target)` |
| `trash_all_sources(carrier_handle)` | `trash_all_sources` |
| `place_source_under(card, carrier, position)` | `place_card_under_permanent_bottom(card, target)`, `place_as_bottom_source(source, target)`, `attach_tamer_to_digimon` (Tamer → bottom of digimon stack) |
| Face-down source state | `CardSource.face_down: bool` field; `training_place_deck_top_under_self_face_down` sets it |
| `return_to_hand(target, options)` | `return_to_hand`, `add_to_hand_from_security`, `add_to_hand_from_deck`, `add_to_hand_from_trash`, `add_to_hand_from_reveal`, `add_top_security_to_hand`, `add_pending_security_to_hand` |
| `return_to_deck(target, position, options)` | `return_to_deck`, `return_stack_to_deck`, `return_to_deck_from_reveal` |
| `trash_from_hand(card, options)` | `trash_from_hand_by_index`, `trash_from_reveal` |
| Security helpers — `security_trash_top` | `trash_top_security`, `trash_top_security_and_cancel_current_replacement` |
| Security helpers — `security_top_to_hand` | `add_top_security_to_hand` |
| Security helpers — `security_place(card, player, position, face_up)` | `place_on_security` (consumes `CardSourceRef`) |
| Security helpers — `security_recovery` | `recover_from_deck` |
| Security helpers — `security_shuffle` | `shuffle_security` |
| Reveal — `reveal_top_n_and_select` | `reveal_top_deck` + `select_*` from selections + `place_remainder_on_deck` |
| Reveal-zone overlay | `CardSource.reveal_overlay: Option<RevealOverlay>` field; consulted at predicate sites |
| `move_from_breeding` | `move_from_breeding_by_effect`, also `play_to_breeding_from_hand`, `hatch` |

### Already-stable handles

- `PermanentHandle { player: PlayerId, index: u8 }` — `Copy`, used everywhere.
- `CardHandle(u16)` — globally unique, stable across zone moves. `SourceSelectionRef.card` carries it for stack-position resilience.
- `BreedingPermanentSelectionRef { player, card }` — for breeding-area selections.
- `SourceSelectionRef { permanent, field_index, source_index, card }` — selection result that carries `card: CardHandle` for stable identity even if `source_index` shifts after intervening battle-area changes.

### Existing reveal-overlay infrastructure

`RevealOverlay { name: Option<String>, kind: Option<CardKind> }` already exists on `CardSource`. `CardSource::clear_reveal_overlay` clears it on resolve. **Gap to verify:** every predicate that reads card type/level/color/trait must consult the overlay if present. Audit list of predicate sites is below in Task 9.

---

## Genuine gaps — work targeted by this plan

### Tier 1 — Foundational
1. **`EffectMoveToken` provenance scaffold** — opaque token returned by `play_card_by_effect` and effect-digivolve helpers, carrying `{ permanent_handle, source_effect, originating_zone, optional_cleanup_hook }`. Stored on `Permanent` so end-of-turn deletion riders, scheduled-return scripts, and scoped On-Play suppression key off the token, not an index. **Currently no such token exists.**
2. **Owner-vs-controller split audit** — confirm `Permanent` carries owner separately from controller (it does — `Permanent::owner()` returns `top_card().owner`). Audit return-to-deck/return-to-hand paths to confirm they consult owner, not controller, for routing. **Likely already correct; needs a targeted fixture test.**

### Tier 2 — Helpers
3. **`place_self_at_security`** — bundle a multi-source Digimon permanent (top + sources) into the controller's security stack at top/bottom, face-up/face-down. Generalises the existing `place_sourceless_permanent_on_security_bottom` (single-source, bottom-only, face-down-only). Unblocks **EX9-021** (top, face-up) + **EX4-060** (bottom, face-down). The Option-card flavor (ST20-15) is a separate, simpler helper because there is no source stack to bundle.
4. **`place_self_option_at_security`** — Option flavor: Option card mid-resolution from hand → security at top/bottom, face-up/face-down. Mirrors the hand-transit pattern in `play_from_security`. Unblocks **ST20-15** [Main] tail.
5. **`bounce_self`** — sugar over `return_to_hand(self.source_permanent.unwrap())`. Three-line addition.
6. **`security_place_stacked_card(source_handle, player, position, face_up)`** — extract a chosen digivolution source then route through `place_on_security`. Unblocks Puppets G027 ("move top stacked card to top security").
7. **`return_all_trash_to_deck_bottom(player_choice_callback)`** — bulk operation; player chooses whose trash; returned cards are owner-routed; binds the moved set for downstream predicates. Unblocks BT17-077 Imperialdramon: Paladin Mode.
8. **`trash_top_n_digivolution_cards_of_each(filter, n)`** — bulk; trims top N sources from every matching opponent permanent; fires per-source trash trigger per source. Unblocks BT12-028.
9. **`trash_opponent_hand_to_count(target_count)`** — forced opponent hand reduction; opponent picks which cards (no auto-selection). Unblocks Millenniummon-style cards.
10. **`search_own_security_stack(filter)`** — reveal full stack + select by filter. Mostly composable from existing primitives; gap is the pre-selection state reset on resolve.
11. **`scheduled_delayed_return(card_or_permanent, return_at)`** — registers a deferred move that fires at a specified timing. Builds on existing `ScheduledEffect`. Unblocks BG Imperial G-BG-02.
12. **`cast_time_assembly` step inside play helpers** — selection over battle-area + trash with a printed filter; chosen cards become source stack of the played permanent before `OnPlay` dispatch; cost reduction applies based on selection size.

### Tier 3 — DSL schema + lowering
13. **DSL verbs for the new helpers** — `place_self_at_security`, `place_self_option_at_security`, `bounce_self`, `return_all_trash_to_deck_bottom`, `trash_top_n_digivolution_cards`, `trash_opponent_hand_to_count`, `search_own_security_stack`, `scheduled_delayed_return`, `cast_time_assembly` block.

### Tier 4 — Card fixtures (unblocked once helpers + DSL land)
- BT13-112 Royal Knights — already implementable with existing `select_own_sources` + `play_selected_sources_without_cost`.
- BT5-106 Demonic Disaster — needs `suppress_on_play` flag wired through `play_from_trash_with_cost`.
- BT17-077 Imperialdramon: Paladin Mode — needs Tasks 7 + 8.
- EX10-032 Proganomon — needs cross-permanent source selection (existing `select_own_sources`) + per-source observer routing (existing).
- BT12-028 — needs Task 8.
- EX11-022 / Puppets fixture — needs Task 1 (provenance) + scheduled cleanup.
- G-RH-02 — needs effect-initiated digivolve from trash (existing `effect_initiated_digivolve_from_source` with `CardSourceRef::Trash`).
- G-RH-06 — needs `place_source_under_tamer` (composable from existing `place_card_under_permanent_bottom` if Tamer permanent target accepted).
- EX4-060 / BT22-015 — needs Task 3 (place self at security bottom face-down).
- EX9-021 — needs Task 3 (place self at security top face-up).
- BT24-031 / BT24-101 (TS Olympos) — needs Task 10 (search-own-security) + multi-bucket reveal.
- G-ASL-03 — needs `place_source_under(face_up: false)` parameter.
- Rocks P-130 — needs `move_from_breeding_by_effect` with suspend-memory wiring (helper exists; Rocks-specific glue).
- BG Imperial G-BG-02 — needs Task 11.
- Cast-time stack-construction card — needs Task 12.
- Owner-routing fixture — verifies Tier 1 Task 2.

---

## File structure

| File | Change | Responsibility |
|---|---|---|
| `code/digimon-engine/src/effect.rs` | Add | `EffectMoveToken` struct; provenance carrier returned by play/digivolve helpers. |
| `code/digimon-engine/src/permanent.rs` | Modify | Add `Permanent.move_token: Option<EffectMoveToken>` to track effect-played provenance. |
| `code/digimon-engine/src/effect_context/mod.rs` | Modify | Add helpers from Tier 2 (Tasks 3-12). Existing helpers extended to return / accept `EffectMoveToken` where applicable. |
| `code/digimon-engine/src/game_actions.rs` | Modify | Add `Game`-level mutation primitives that the new `EffectContext` helpers wrap (`place_self_at_security_observed`, `return_all_trash_to_deck_bottom_inner`, `trash_top_n_digivolution_cards_inner`). |
| `code/digimon-engine/src/scheduled_effects.rs` | Modify | Extend `ScheduledEffect` with a "deferred-zone-move" variant for `scheduled_delayed_return`. |
| `code/digimon-engine/src/dsl_cards/step/*.rs` | Modify | Add lowering for new DSL step verbs. |
| `code/digimon-dsl/src/spec/step.rs` (in `code/digimon-dsl/`) | Modify | Add schema for new step verbs; export through `embedded_registry`. |
| `code/digimon-engine/cards/<set>/<card>.yaml` | Modify | Uncomment the BLOCKED stubs for cards listed above as helpers/DSL verbs land. |
| `code/digimon-engine/tests/zone_movement.rs` | Create | Framework unit tests for new helpers. |
| `code/digimon-engine/tests/cards_behavioral/<set>/<card>.rs` | Modify | Replace `#[ignore]` BLOCKED stubs with real assertions. |
| `docs/RUST_ENGINE_API.md` | Modify | Document new helper signatures, handle invariants, provenance-token shape, cast-time-assembly contract. |
| `docs/RUST_ENGINE_GAPS.md` | Modify | Mark closed entries; refine open entries with current status. |
| `qa/dsl-vocab-gaps.md` | Modify | Add new DSL verb entries; mark closed when shipped. |

---

## Tasks

### Task 1: `EffectMoveToken` provenance scaffold

**Files:**
- Create: `code/digimon-engine/src/effect_move.rs` (new module)
- Modify: `code/digimon-engine/src/lib.rs:1` (re-export `EffectMoveToken`)
- Modify: `code/digimon-engine/src/permanent.rs:54-83` (add `move_token: Option<EffectMoveToken>` field)
- Modify: `code/digimon-engine/src/effect_context/mod.rs:1913` (have `play_from_hand_with_cost` write the token to `Permanent.move_token`)
- Modify: `code/digimon-engine/src/effect_context/mod.rs:1990` (`play_from_security`)
- Modify: `code/digimon-engine/src/effect_context/mod.rs:2077` (`play_from_materials`)
- Modify: `code/digimon-engine/src/effect_context/mod.rs:2141` (`play_from_trash_with_cost`)
- Test: `code/digimon-engine/tests/zone_movement.rs::effect_move_token_survives_battle_area_shifts`

- [ ] **Step 1: Write the failing test**

```rust
// code/digimon-engine/tests/zone_movement.rs
use digimon_engine::effect_move::EffectMoveToken;

#[test]
fn effect_move_token_survives_battle_area_shifts() {
    let mut runner = DebugRunner::new(...);
    // Effect plays card X from hand; capture the returned token.
    let token = runner.with_ctx(|ctx| ctx.play_from_hand_with_cost(0, 0, CostDelta::Free))
        .expect("expected play to succeed")
        .move_token
        .expect("expected token to be set");
    // Trigger three intervening plays + one deletion to shift battle-area indices.
    runner.play_card_at(...);
    runner.play_card_at(...);
    runner.delete_first_permanent();
    // Token's permanent_handle should still resolve to the same logical card.
    let perm = runner.game()
        .find_permanent_by_move_token(token)
        .expect("expected token to resolve");
    assert_eq!(perm.top_card().handle(), token.played_card());
}
```

- [ ] **Step 2: Verify it fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_movement -- effect_move_token_survives_battle_area_shifts`
Expected: FAIL — `EffectMoveToken` undefined.

- [ ] **Step 3: Define the token struct**

```rust
// code/digimon-engine/src/effect_move.rs
use crate::card_source::CardHandle;
use crate::enums::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EffectMoveToken {
    /// Globally unique token id; assigned at move time from
    /// `Game.next_move_token_id`.
    pub id: u32,
    /// Card that was played / digivolved by the effect.
    played_card: CardHandle,
    /// Effect's source card (the script that initiated the move).
    pub source_card: CardHandle,
    /// Zone the moved card came from.
    pub origin_zone: Zone,
}

impl EffectMoveToken {
    pub fn new(id: u32, played_card: CardHandle, source_card: CardHandle, origin_zone: Zone) -> Self {
        Self { id, played_card, source_card, origin_zone }
    }
    pub fn played_card(&self) -> CardHandle { self.played_card }
}
```

- [ ] **Step 4: Add `move_token` to Permanent**

Add `pub move_token: Option<EffectMoveToken>` to `Permanent` in `permanent.rs`. Default `None` in `Permanent::new`. Preserve through `digivolve` (token stays attached to the permanent regardless of stack changes — provenance is permanent-level, not source-level).

- [ ] **Step 5: Wire token assignment in play helpers**

In `effect_context/mod.rs`, after each `play_from_hand_with_cost_result::Played(field_index)` arm, set `Permanent.move_token = Some(EffectMoveToken::new(...))`. Issue ID via `Game::next_move_token_id` (add a `u32` counter to `Game` + a `next_move_token_id(&mut self) -> u32` accessor).

- [ ] **Step 6: Add `Game::find_permanent_by_move_token`**

```rust
pub fn find_permanent_by_move_token(&self, token: EffectMoveToken) -> Option<PermanentHandle> {
    for (pid, p) in self.players.iter().enumerate() {
        for (idx, perm) in p.battle_area.iter().enumerate() {
            if perm.move_token.map(|t| t.id) == Some(token.id) {
                return Some(PermanentHandle { player: pid as PlayerId, index: idx as u8 });
            }
        }
    }
    None
}
```

- [ ] **Step 7: Run test, expect PASS**

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/effect_move.rs \
        code/digimon-engine/src/lib.rs \
        code/digimon-engine/src/permanent.rs \
        code/digimon-engine/src/game.rs \
        code/digimon-engine/src/effect_context/mod.rs \
        code/digimon-engine/tests/zone_movement.rs
git commit -m "feat(engine): EffectMoveToken provenance scaffold"
```

### Task 2: Owner-vs-controller routing fixture

**Files:**
- Test: `code/digimon-engine/tests/zone_movement.rs::return_to_deck_routes_to_owner_not_controller`

- [ ] **Step 1: Write a fixture**

```rust
#[test]
fn return_to_deck_routes_to_owner_not_controller() {
    // Player A owns card X. An effect transfers control of X to player B.
    // (Use existing control-transfer mechanic if any; otherwise use a token
    // workaround via `attach_tamer_to_digimon` cross-controller — though
    // that's same-controller. May need to write a temporary control-transfer
    // helper for the test; if no control-transfer exists in the engine,
    // mark this test #[ignore] and file a sub-gap.)
    // Effect returns X to deck bottom.
    // Assertion: X is at the bottom of A's deck, not B's.
}
```

- [ ] **Step 2: Verify routing**

Confirm `Game::return_to_deck` reads `permanent.owner()` (i.e. `top_card().owner`), not `permanent_handle.player`. If it currently uses `permanent_handle.player`, fix to use `owner()`. The existing `Permanent::owner()` accessor exists at `permanent.rs:130`.

- [ ] **Step 3: Commit**

### Task 3: `place_self_at_security` for multi-source Digimon permanents

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:3729` (`place_sourceless_permanent_on_security_bottom` → generalize to `place_permanent_on_security`).
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (add `place_self_at_security` and `place_self_at_security_and_cancel_current_replacement`).
- Test: `code/digimon-engine/tests/zone_movement.rs::place_self_at_security_top_face_up_bundles_sources`
- Test: `code/digimon-engine/tests/zone_movement.rs::place_self_at_security_bottom_face_down_bundles_sources`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn place_self_at_security_top_face_up_bundles_sources() {
    // Set up a 3-source permanent (base, evo1, evo2).
    // Call ctx.place_self_at_security(SecurityPlacement::Top, SecurityFace::Up).
    // Assert:
    //   - Permanent removed from battle area
    //   - Top of security is the previous top of the stack
    //   - Sources are placed under it (still part of the bundle, in same order)
    //   - face_up_security has the top entry
    //   - linked_cards (if any) routed to trash
    //   - WhenWouldLeaveBattleArea + WhenWouldPlaceInSecurity replacements consulted
}

#[test]
fn place_self_at_security_bottom_face_down_bundles_sources() {
    // Same as above but Bottom + face-down. face_up_security stays unset.
}
```

- [ ] **Step 2: Generalise `place_sourceless_permanent_on_security_bottom`**

Rename to `place_permanent_on_security` and accept `position: StackPosition` + `face_up: bool` parameters. Replace the `if permanent.card_sources.len() != 1` early-return — accept multi-source bundles. After popping the top card, place the remaining sources somewhere — DCGO's `IPutSecurityPermanent` bundles them WITH the top card into security. Engine must mirror that — the bundle appears in security as a stack-of-cards that share a security slot. **Engine question:** does `Player.security: Vec<CardSource>` support multi-card slots? If not, this is a sub-gap; bundle preservation may need a new `SecuritySlot` variant. Likely answer: today security is one card per slot. The bundle case is unrepresentable — sources need to go to trash. Document this divergence in `RUST_PYTHON_PARITY.md` as "single-card security only; bundle move places top card and sends sources to owner trash".

- [ ] **Step 3: Wire the EffectContext helper**

```rust
pub fn place_self_at_security(
    &mut self,
    position: StackPosition,
    face_up: bool,
) -> bool {
    let Some(handle) = self.source_permanent else {
        return false;
    };
    let owner = handle.player;
    self.game.place_permanent_on_security(owner, handle, position, face_up, self.player)
}

pub fn place_self_at_security_and_cancel_current_replacement(
    &mut self,
    position: StackPosition,
    face_up: bool,
) -> bool {
    if self.place_self_at_security(position, face_up) {
        if self.game.parked_replacement.is_some() {
            self.cancel_current_replacement();
        }
        true
    } else {
        false
    }
}
```

- [ ] **Step 4: Update the existing replacement-outcome lowering**

`code/digimon-engine/src/dsl_cards/step/replacement_outcome.rs:35` calls `place_sourceless_permanent_bottom_security_and_cancel_current_replacement` — keep the old name as a thin alias delegating to `place_self_at_security_and_cancel_current_replacement(StackPosition::Bottom, false)`.

- [ ] **Step 5: Run tests**

- [ ] **Step 6: Commit**

### Task 4: `place_self_option_at_security` for Option-card subjects

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::option_card_places_self_at_security_top_face_up`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn option_card_places_self_at_security_top_face_up() {
    // Set up an Option card resolving from MainEffectDrain (or simulate the
    // hand-transit path: card sits in hand at trigger fire, ctx invokes
    // place_self_option_at_security).
    // Assert: card moves to top of security, face-up; OnPlaceSecurity fires.
}
```

- [ ] **Step 2: Implement**

```rust
pub fn place_self_option_at_security(&mut self, position: StackPosition, face_up: bool) -> bool {
    // Locate the resolving Option card. For Options resolving from hand,
    // self.source_card is the Option; find it in any player's hand.
    let card_handle = self.source_card;
    let mut found: Option<(PlayerId, usize)> = None;
    for (pid, p) in self.game.players.iter().enumerate() {
        if let Some(idx) = p.hand.iter().position(|c| c.handle() == card_handle) {
            found = Some((pid as PlayerId, idx));
            break;
        }
    }
    let Some((player, hand_index)) = found else { return false; };
    self.game.place_on_security_observed(
        player,
        CardSourceRef::Hand(player, hand_index),
        position,
        face_up,
        self.player,
    )
}
```

- [ ] **Step 3: Run tests + commit**

### Task 5: `bounce_self` sugar

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::bounce_self_returns_source_to_owner_hand`

- [ ] **Step 1: Implement and test**

```rust
pub fn bounce_self(&mut self) -> Option<CardHandle> {
    let handle = self.source_permanent?;
    self.return_to_hand(handle)
}
```

- [ ] **Step 2: Commit**

### Task 6: `security_place_stacked_card`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::security_place_stacked_card_extracts_source_then_places`

Reuses the existing `select_own_sources` selection prompt + the chosen `SourceSelectionRef.card`. Extract via `trash_card_source`'s pattern (without trashing) and route through `place_on_security`.

### Task 7: `return_all_trash_to_deck_bottom`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/game_actions.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::return_all_trash_to_deck_bottom_owner_routed`

- [ ] **Step 1: Surface a player-choice selection** — install a `SelectionKind::EffectChoice` over the two players (or current player + opponent), then for the chosen player iterate trash → owner's deck bottom (each card's `owner` field tells us which deck), firing per-card `OnReturn` triggers and binding the moved set on `Bindings` so downstream `any_returned_card` predicates see it.

### Task 8: `trash_top_n_digivolution_cards_of_each`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::trash_top_n_digivolution_cards_of_each_iterates_all_matching`

Iterates `game.player(opponent).battle_area` matching the filter, for each match peels up to N top sources via repeated `trash_top_source`. Caps at stack-size; fires per-source `OnDigivolutionCardTrashed` per source.

### Task 9: `trash_opponent_hand_to_count`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::trash_opponent_hand_to_count_opponent_chooses`

Surfaces a multi-pick selection from the opponent's hand with `selecting_player = opponent` (existing `override_selecting_player` mechanism in `EffectContext`). The opponent picks which cards to trash. No auto-selection per the no-approximations policy.

### Task 10: `search_own_security_stack`

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::search_own_security_stack_resets_state_on_resolve`

Reveals the full security stack to the controller, surfaces a count-capped multi-select with the printed filter, and on resolve clears any pre-selection state. Existing `face_up_security` bookkeeping should NOT be persistently mutated by this search.

### Task 11: `scheduled_delayed_return`

**Files:**
- Modify: `code/digimon-engine/src/scheduled_effects.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/zone_movement.rs::scheduled_delayed_return_fires_at_target_timing`

Extend `ScheduledEffect` with a `DeferredZoneMove` variant carrying `{ subject: Either<CardHandle, EffectMoveToken>, destination: Zone, position: Option<StackPosition>, fire_at: DelayTrigger }`. Drained by the existing scheduled-effect pump.

### Task 12: Cast-time `assembly` step

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:305` (`play_from_hand_with_cost`) — splice an optional pre-OnPlay assembly hook between cost calculation and `OnPlay` dispatch.
- Modify: `code/digimon-engine/src/effect_context/mod.rs` — add `play_from_hand_with_cast_time_assembly(filter, count_range, cost_per_assembled)`.
- Test: `code/digimon-engine/tests/zone_movement.rs::cast_time_assembly_installs_sources_before_on_play`

The hook runs after cost calculation, before `OnPlay`. It opens a count-capped pending selection over battle-area + trash; on resolve, pulls the chosen cards into the played permanent's `card_sources` (top-down per chosen order), applies cost reduction proportional to the chosen count, then drains `OnPlay` against the permanent with full sources installed.

### Task 13: DSL schema + lowering

For each new helper, add a step variant to `code/digimon-dsl/src/spec/step.rs` and lowering in `code/digimon-engine/src/dsl_cards/step/`. Verbs:

- `place_self_at_security: { position: top|bottom, face: up|down }`
- `place_self_option_at_security: { position: top|bottom, face: up|down }`
- `bounce_self: {}`
- `return_all_trash_to_deck_bottom: { whose: you|opponent|player_choice }`
- `trash_top_n_digivolution_cards: { of: opponent|you|all, n: <int>, filter: <PredicateSpec> }`
- `trash_opponent_hand_to_count: { count: <int> }`
- `search_own_security_stack: { filter: <PredicateSpec>, prompt: <str> }`
- `scheduled_delayed_return: { subject: <BindingRef>, destination: <Zone>, position: <StackPosition>, fire_at: <DelayTrigger> }`
- `cast_time_assembly:` block within `play:` step (filter + count range + cost-reduction-per).
- `place_source_under_tamer: { source: <BindingRef>, tamer: <BindingRef>, face: up|down }`

### Tasks 14-29: Card fixtures

For each fixture in Tier 4 above, replace the `#[ignore]` BLOCKED stub in the corresponding `tests/cards_behavioral/<set>/<card>.rs` with a real assertion. Uncomment the YAML stub and adjust the DSL to use the new step verbs.

### Task 30: Tracker discipline

Mark the following entries closed (with the verifying test command):

- `docs/RUST_ENGINE_GAPS.md`:
  - "Zone-manipulation: play-from-hand / trash without paying cost (+ cost override)" — already largely closed; mark resolved.
  - "Zone-manipulation: effect-initiated digivolve" — already closed.
  - "Zone-manipulation: return-to-hand / return-to-deck (top/bottom) / bounce self / trash-from-hand" — close after Task 5.
  - "Zone-manipulation: security stack operations" — close after Task 6.
  - "`ctx.move_from_breeding()` EffectContext helper" — already closed.
  - "Effect-played permanent cleanup provenance" — close after Task 1.
  - "Forced opponent hand reduction primitive" — close after Task 9.
  - "Trash all digivolution cards of a permanent" — already closed.
  - "Search-own-security-stack primitive" — close after Task 10.
  - "Cast-time stack-construction for cost reduction" — close after Task 12.

- `qa/dsl-vocab-gaps.md`:
  - `G-PLACE-SELF-AT-SECURITY-TOP` — close after Task 3.
  - `G-PLACE-SELF-AT-SECURITY-BOTTOM` — close after Task 3.
  - `G-PLACE-SELF-AT-SECURITY-TOP-FACE-UP-OPTION` — close after Task 4.
  - All bulk-op verbs from Task 13.

- `docs/RUST_ENGINE_API.md`:
  - Add a "Provenance" section documenting `EffectMoveToken`.
  - Add "Cast-time assembly" subsection with the contract.
  - Add a "Stable handles" subsection enumerating `PermanentHandle`, `CardHandle`, `SourceSelectionRef`.

---

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test zone_movement
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Watch for parity tests that read full event logs; routing through centralised helpers will reorder some observer firings, and you may need to update parity expectations rather than restore the old order.
