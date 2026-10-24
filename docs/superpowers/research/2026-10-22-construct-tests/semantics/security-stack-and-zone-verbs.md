# Construct semantics: security-stack-and-zone-verbs

- **Family:** security-stack-and-zone-verbs
- **Date:** 2026-10-22
- **Repo state read:** `D:/wt/p3-constructs` @ f24f26dfa (read-only; no cargo run)
- **Constructs (in order):** `verb:add_top_security_to_hand`, `verb:return_top_security_to_deck`,
  `verb:security_place_top_card`, `verb:trash_selected_security`, `verb:play_security_card`,
  `timing:on_lose_security`, `predicate:owner_can_add_security`, `verb:select_zone_cards`,
  `verb:return_all_to_deck`, `verb:return_union_bound_to_deck`, `verb:move_self_option_under_permanent`,
  `verb:delete_permanents`

## Shared conventions (read once)

- **Security order.** `player.security` is a `Vec`: index 0 = BOTTOM, `last()` = TOP
  (`Player::put_on_security_top`; `insert_in_security(0, ..)` = bottom; `player.rs:244-250`).
  `DebugRunner::builder().security(p, &["A","B","C"])` pushes in order, so **C is the top card**.
- **Deck order.** Same: index 0 = bottom, `last()` = top (drawn first; `Player::draw` pops).
  `.deck(p, &[..])`: last id = top.
- **Stacks.** `place_stack(p, &["BOTTOM", ..., "TOP"])` (first id = bottom source, last = top card).
- **Security selection ids.** `select_security` / `may_add_top_security_to_hand` use
  `SEL_MY_SECURITY_START (40) + i` for the controller's stack and `SEL_OPP_SECURITY_START (50) + i` for the
  opponent's, where **i counts from the BOTTOM** (i = 0 is the bottom card). `PASS = 62`.
- **Zone ids.** Hand card i = `PLAY_HAND_START (0) + i`; trash card i = `TRASH_EFFECT_START (1150) + i`;
  a stack card = `encode_source_select(field_index, stack_index)`.
- **Memory.** `runner.memory()` is the shared gauge from player 0's side: a P0 `gain_memory: 1` is +1, a
  P1 `gain_memory: 1` is -1 (see `tests/effect_context/security_stack_operations.rs`).
- **Driving a clause.** Pattern from `tests/dsl/effect_battle.rs`: put the fixture on the field, then
  `runner.game.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h)); runner.game.drain_effect_queue();`
  then inspect `runner.pending_selection_view()` / `runner.game.pending_selection` and answer with
  `runner.execute_action(selecting_player, id)`. Template file for this family:
  `code/digimon-engine/tests/dsl/security_stack_steps.rs` (already exercises the sibling `may_add_top_security_to_hand`).
- **Rule numbers.** The brief pointed at "§3-5 / §6"; in CR 4.3 the security-stack area is **§3-7**, the
  general area rules are **§3-1-3**, the deck is **§3-2** (§3-5 is the hand). Citations below use the 4.3 numbers
  as they appear in `docs/digimon-rules/cr-4.3.txt`.
- **Counting location.** Tests must live in `tests/dsl/` (or the other counted dirs). Several strong behavioural
  tests for these constructs already exist in NON-counted locations (top-level `tests/*.rs`, `tests/effects/`,
  raw `EffectContext` calls in `tests/effect_context/`, `cards_behavioral`) — listed per construct as near-misses.

---

### `verb:add_top_security_to_hand`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:113` (`StepSpec::AddTopSecurityToHand(PlayerArg)`; key `"add_top_security_to_hand"` at `:909`; `PlayerArg { of }` at `:1542`). Compiled `code/digimon-dsl/src/compiled.rs:1693`; compile arm `code/digimon-dsl/src/compile.rs:2891`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs:207` -> `EffectContext::add_top_security_to_hand` `code/digimon-engine/src/effect_context/action/security.rs:325` -> `add_to_hand_from_security` `security.rs:252` -> `Game::fire_security_removed_observers` `game_actions/helpers.rs:122` -> `fire_effect_security_removal` `effect_queue.rs:5652` -> `complete_effect_security_removal` (`Hand(owner)` arm, `effect_queue.rs:5423`).
  ```rust
  // zone_moves.rs:207
  CompiledStep::AddTopSecurityToHand { of } => {
      let p = resolve_player(ctx, *of);
      ctx.add_top_security_to_hand(p);   // bool discarded; nothing recorded in bindings
      true
  }
  // security.rs:325
  let Some(card) = self.game.player(player).security.last().map(|c| c.handle()) else { return false; };
  self.add_to_hand_from_security(player, card)
  // security.rs:252 add_to_hand_from_security
  self.game.ensure_security_materialized(player, idx);
  let removed = self.game.player_mut(player).security.remove(idx);
  let owner = removed.owner;
  self.game.player_mut(player).face_up_security.remove(&removed.card_index);
  self.game.fire_security_removed_observers(player, self.player, removed,
      SecurityRemovalDestination::Hand(owner));
  ```
- **YAML shape:** `- add_top_security_to_hand: { of: you }` (or `{ of: opponent }`). From BT26-033:
  ```yaml
  - when: when_digivolving
    process:
      - add_top_security_to_hand: { of: you }
  ```
- **Semantics:** Mandatory, no prompt: the TOP security card (`security.last()`) of `of` leaves the stack and
  goes to its **owner's** hand (`removed.owner`, not necessarily `of`). Empty stack = silent no-op and the clause
  continues. The card is materialised (opaque placeholders resolved) and its face-up record cleared. It is an effect
  removal: it enqueues `OnLoseSecurity` (effect_initiated = true) for the removed card's `[Security]`-scoped effects and
  the stack owner's battle area, then (after the move) `OnOwnSecurityRemoved` for the stack owner and
  `OnOpponentSecurityRemoved` for the other player, plus `OnAddToHand` via `fire_on_add_to_hand_by_effect(owner)`.
  The step does NOT record the card in the effect's result bindings (unlike `add_to_hand_from_security`, which calls
  `bindings.record_added_to_hand`). The returned `bool` is discarded, so a "By adding ..., do Y" clause relies on a
  `security_count_gte: 1` condition for §15-7-4 gating (BT24-034, BT25-030).
- **Rule basis:**
  - §3-7-2 / §3-7-3: the security stack is private and its order fixed, so "top" is positional.
  - §3-1-3-8: a card placed in an unnamed area goes to its owner's area (the card owner's hand).
  - §3-1-3-1-1: the card becomes a new card in hand.
  - §15-15-3-2: a card actually leaving the stack triggers "when a card is removed from your security stack".
  - §15-7-4 / §15-7-5: a "By adding your top security card to the hand" cost is unchoosable with 0 security, choosable even if the rest can't happen.
- **Production users:**
  - BT26-033 Jupitermon (top-10: Jupitermon decks): "[When Digivolving] Add your top security card to the hand. Then, if it's your turn, you may play or use 1 [Iliad] or [TS] trait card from your hand with the cost reduced by 5." — YAML above (followed by `if: { your_turn }` + `select_hand` + `play_or_use_from_hand`).
  - BT24-034 (top-10: TS Toolbox): "By adding your top security card to the hand, you may play 1 [TS] Tamer ..." —
    ```yaml
    optional: true
    condition: { security_count_gte: 1 }
    process:
      - add_top_security_to_hand: { of: you }
      - select_hand: { of: you, bind_as: pick, ... }
    ```
  - (Opponent form: BT11-033 `add_top_security_to_hand: { of: opponent }` inside `if: { effect_returned_any_card: false }`.)
- **Existing near-miss tests:**
  - `tests/effect_context/security_stack_operations.rs::add_top_security_to_hand_moves_top_card_and_fires_loss_observer` — calls `EffectContext::add_top_security_to_hand` directly (no DSL lowering); checks top card to hand and an `OnOpponentSecurityRemoved` observer.
  - `src/dsl_cards/step/selections.rs:618-626`, `src/dsl_cards/step/zone_moves.rs` — source, not tests (the `may_` variant's callback reuses the same EffectContext call).
  - `tests/dsl/security_stack_steps.rs` covers the sibling `may_add_top_security_to_hand`, not this verb.
- **Proposed behavioural test (for the plan):** new file `tests/dsl/security_top_moves.rs` (or extend `security_stack_steps.rs`).
  - fixture:
    ```yaml
    card: TST-ATSH
    name: Add Top Security Probe
    kind: digimon
    level: 4
    color: [yellow]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - add_top_security_to_hand: { of: you }
          - gain_memory: 1
    ```
    plus `TST-ATSH-OPP` identical but `of: opponent`.
  - setup: `.from_dsl_yaml(..)`, `.add_card(make_test_card("SEC-BOT","Bottom"))`, `.add_card(make_test_card("SEC-TOP","Top"))`, `.security(0, &["SEC-BOT","SEC-TOP"])`, `.security(1, &["SEC-BOT","SEC-TOP"])`, `.memory(0)`, `.start()`; `let h = runner.place_on_field(0, "TST-ATSH", Some(0));` enqueue `WhenDigivolving` on `h`, drain.
  - positive: `runner.game.pending_selection.is_none()` (no prompt); `security_count(0) == 1`; `players[0].security[0]` is `SEC-BOT`; P0 hand gained exactly one card with id `SEC-TOP`; memory +1 (the clause continued). Opponent fixture: P1 security 1, P1 hand +1 (`SEC-TOP`), P0 hand unchanged. Owner routing: before triggering, set `players[0].security.last_mut().owner = 1` -> the card lands in P1's hand (§3-1-3-8). Face-up: insert the top card's `card_index` into `face_up_security` first -> afterwards `face_up_security` no longer contains it.
  - negative: `.security(0, &[])` -> no panic, hand unchanged, no prompt, memory still +1 (empty stack is a no-op that does not abort the clause).
  - selection surface: none — assert `pending_selection.is_none()` immediately after the drain (mandatory verb).
  - observer cross-check (optional): an `on_lose_security` observer fixture on P0 (see `timing:on_lose_security`) fires once with the effect bit set.
- **Suspected engine/DSL defects:** The step never calls `bindings.record_added_to_hand(h)` (contrast `AddToHandFromSecurity`, `zone_moves.rs:196-205`), so an "if you added / the added card" follow-up can't see this card. Otherwise none seen.

---

### `verb:return_top_security_to_deck`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:112` (`ReturnTopSecurityToDeck(ReturnTopSecurityToDeckArgs)`; key `:908`; args `{ of, position }` `:1549`). Compiled `compiled.rs:1689`; compile `compile.rs:2887`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:955` -> `EffectContext::return_security_card_to_deck` `security.rs:291` -> `fire_security_removed_observers(.., Deck { owner, to_bottom })` -> `complete_effect_security_removal` Deck arm `effect_queue.rs:5429` -> `Player::put_in_deck_or_digi_egg_deck` `player.rs:323`.
  ```rust
  // play_digivolve.rs:955
  let owner = resolve_player(ctx, *of);
  let to_bottom = matches!(super::map_stack_position(*position), StackPosition::Bottom);
  if let Some(handle) = ctx.game.player(owner).security.last().map(|c| c.handle()) {
      if ctx.return_security_card_to_deck(owner, handle, to_bottom) {
          bindings.record_returned_to_deck(handle);
      }
  }
  // effect_queue.rs:5429 (completion)
  SecurityRemovalDestination::Deck { owner, to_bottom } => {
      let is_egg = security.card.card_kind(&self.card_data) == CardKind::DigiEgg;
      self.player_mut(owner).put_in_deck_or_digi_egg_deck(security.card, is_egg, to_bottom);
      if !is_egg { self.note_effect_deck_add_if_resolving(); }
  }
  ```
- **YAML shape:** `- return_top_security_to_deck: { of: you|opponent, position: top|bottom }`. From BT26-086:
  ```yaml
  then:
    - return_top_security_to_deck: { of: opponent, position: bottom }
  ```
- **Semantics:** Mandatory, no prompt. Takes `of`'s top security card (`security.last()`) and moves it to the
  **card owner's** deck: `bottom` -> deck index 0, `top` -> deck `last()` (next draw); a Digi-Egg goes to the Digi-Egg
  deck. Empty stack = no-op (nothing recorded). On success the handle is recorded as returned-to-deck, so
  `effect_returned_any_card` / `returned_card_matching` read true. Fires the effect-removal chain (`OnLoseSecurity`
  effect_initiated = true, then `OnOwnSecurityRemoved` / `OnOpponentSecurityRemoved`) and, while an effect is
  resolving, opens the `OnAddToDeck` batch. `position: random` maps to `StackPosition::Random`, which this step treats
  as "not bottom", i.e. top; `position: choice` panics in `map_stack_position`.
- **Rule basis:**
  - §3-7-2 / §3-7-3: private, fixed-order stack; "top security card" is positional.
  - §3-2-2 / §3-2-3: the deck is private, face-down, fixed order (top vs bottom is meaningful).
  - §3-1-3-8: the card goes to its owner's deck.
  - §3-1-3-9-1: a Digi-Egg placed in a deck goes to the Digi-Egg deck in the specified order.
  - §15-15-3-2: the stack's count decreased, so "removed from security" observers trigger.
  - §15-7-1 / §15-7-2: as a "by returning ..., it doesn't leave" cost (BT26-016) the following processing requires the return to have happened.
- **Production users:**
  - BT26-086 Dantemon (top-10: Rebootmon / Beelstarmon): "... Then, if this Digimon has 7 link cards, return your opponent's top security card to the bottom of the deck." — YAML above.
  - BT26-016 Chronomon: Holy Mode (top-10: Chronomon): "[All Turns] [Once Per Turn] When this Digimon would leave the battle area, by returning your top security card to the bottom of the deck, it doesn't leave." —
    ```yaml
    - kind: replacement
      trigger: when_would_leave_battle_area
      optional: true
      once_per_turn: true
      active_when: { security_count_gte: 1 }
      process:
        - return_top_security_to_deck: { of: you, position: bottom }
        - cancel_replacement: {}
    ```
- **Existing near-miss tests:** coverage index lists only `src/dsl_cards/step/play_digivolve.rs` (source). The underlying EffectContext call is tested raw in `tests/effect_context/security_stack_operations.rs::return_security_card_to_deck_top_places_at_deck_end_and_fires_observer` / `..._bottom_places_at_index_zero` / `..._unknown_card_is_noop` (no DSL lowering; they pass an explicit handle rather than "top").
- **Proposed behavioural test (for the plan):**
  - fixture:
    ```yaml
    card: TST-RTSD
    name: Return Top Security Probe
    kind: digimon
    level: 4
    color: [blue]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - return_top_security_to_deck: { of: opponent, position: bottom }
          - if:
              condition: { effect_returned_any_card: true }
              then:
                - gain_memory: 1
    ```
    (parameterise `position` with `format!` for a `top` variant, and `of: you` for an own-stack variant).
  - setup: cards `SEC-BOT`, `SEC-TOP`, `D1`, `D2`; `.security(1, &["SEC-BOT","SEC-TOP"])`, `.deck(1, &["D1","D2"])`, `.memory(0)`, start; place `TST-RTSD` on P0's field; enqueue `WhenDigivolving`; drain.
  - positive (bottom): no pending selection; `security_count(1) == 1`, remaining card `SEC-BOT`; `deck_size(1)` = before + 1 and `players[1].deck[0]` is `SEC-TOP`; memory +1. Top variant: `players[1].deck.last()` is `SEC-TOP`. Owner routing: set the top security card's `owner = 0` first -> it lands in P0's deck, not P1's.
  - negative: `.security(1, &[])` -> deck unchanged, memory unchanged (`effect_returned_any_card` false), no panic.
  - observer cross-check: an `on_lose_security` observer on P1 fires once (P1 gain -> memory -1 in P0 terms) with `event_is_effect_initiated` true.
  - selection surface: none (assert `pending_selection.is_none()`).
- **Suspected engine/DSL defects:** `position: random` silently behaves as `top` (only `Bottom` is tested in the `matches!`). BT26-016 discards the success bool and runs `cancel_replacement` unconditionally — safe today only because `active_when: security_count_gte: 1` and nothing can block a security->deck move. Otherwise none seen.

---

### `verb:security_place_top_card`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:400` (`SecurityPlaceTopCard(SecurityPlaceTopStackedCardArgs)`; key `"security_place_top_card"` `:1035`; args `{ carrier, of (default you, alias target_player), position, face: up|down }` `:2413`). Compiled `compiled.rs:2289` (`face_up: bool`); compile `compile.rs:3627` (`face_up: a.face.is_up()`).
  - Runtime: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs:777` -> `EffectContext::security_place_top_card` `security.rs:546` -> `Game::place_top_stacked_on_security` `game_actions/mod.rs:4161` -> `finish_top_stacked_security_move` `game_actions/mod.rs:4241`.
  ```rust
  // zone_moves.rs:777
  let target_player = resolve_player(ctx, *of);
  if let Some(ResolvedBinding::Permanent(carrier)) = resolve_binding_ref(carrier, ctx, bindings) {
      let position = super::map_stack_position(*position);
      let _ = ctx.security_place_top_card(carrier, target_player, position, *face_up);
  }
  // game_actions/mod.rs:4161 (gates, then replacement window)
  if self.modifiers.player_has(observer, CannotAddSecurityByEffect)
      || /* any of observer's battle-area permanents has */ CannotAddSecurity { return false; }
  if permanent.card_sources.len() < 2 { return false; }
  let card = permanent.top_card().handle();
  // try_replace(WhenWouldPlaceInSecurity, Card(card, BattleArea), .., Some(Security))
  // finish_top_stacked_security_move: pop() the top; Top -> put_on_security_top,
  // Bottom -> insert_in_security(0), Random -> rng index; face_up -> face_up_security.insert;
  // Digi-Egg -> insert_in_digi_egg_deck(0); then fire_on_place_security(..)
  ```
- **YAML shape:** from BT26-033:
  ```yaml
  - security_place_top_card:
      carrier: source
      of: you
      position: bottom
      face: down
  ```
- **Semantics:** Moves the carrier's VISIBLE top card (`card_sources[len-1]`) into `of`'s security at `position`
  (`top` / `bottom` / `random`), face-down unless `face: up`. The permanent stays in play topped by the card directly
  under it (orientation untouched: a suspended Digimon stays suspended). Requires >= 2 cards in the stack (a lone card
  is a silent no-op). Gated by the ACTING player's (`self.player`) `CannotAddSecurityByEffect` and by any of the acting
  player's permanents carrying `CannotAddSecurity`; opens a `WhenWouldPlaceInSecurity` replacement window
  (Cancelled/CustomHandled -> no move; Redirected to Trash/Hand/Deck honoured). On success fires `OnPlaceSecurity` (not
  `OnLoseSecurity`) and Ace Overflow for the departing card. Unset / non-permanent `carrier` binding = no-op. Never
  surfaces a selection. The bool result is discarded by the step. Contrast `security_place_top_stacked_card` (same
  engine primitive since G-TOP-STACKED-CARD-TO-SECURITY) and `security_place_stacked_card` (a chosen source).
- **Rule basis:**
  - §4-7-1 / §4-7-2: "stacked cards" are all the cards of a stack; a single card isn't "stacked cards" (the >= 2 gate).
  - §4-7-6: when the card below becomes the top card the stack's orientation carries over (not a suspend/unsuspend).
  - §3-1-3-5: placed per the area's rules, i.e. face down in security unless specified.
  - §3-7-2 / §3-7-4: security is private and face-down; a face-up placement makes the card public.
  - §3-1-3-8: no player named -> the owner's stack (production passes `of: you`, the owner for one's own top card).
  - §3-1-3-9-2: a Digi-Egg sent to a non-deck private area goes to the bottom of the Digi-Egg deck.
  - §15-7-1 / §15-7-2 / §15-7-4: as "by placing this Digimon's top stacked card ..., they don't leave" the move is an optional processing condition.
- **Production users:**
  - BT26-033 Jupitermon (top-10: Jupitermon x2, Chronomon): "[All Turns] When any of your [TS] trait Digimon or Tamers would leave the battle area, by placing this Digimon's top stacked card as the bottom security card, they don't leave." — YAML above, inside `kind: replacement`, `trigger: when_would_leave_battle_area`, `optional: true`, `active_when: { all_of: [ replacement_subject_is_mine: true, any_of: [kind: digimon, kind: tamer], trait_has: TS, self_source_count: { op: gte, value: 1 } ] }`, followed by `- cancel_replacement: {}`.
  - (Only one production user.)
- **Existing near-miss tests:** coverage index lists only `src/dsl_cards/step/zone_moves.rs`. The shared engine primitive is exercised by `cards_behavioral` card tests (BT20-084 / BT20-055 / EX13-032 / BT26-033), which never count.
- **Proposed behavioural test (for the plan):**
  - fixture:
    ```yaml
    card: TST-SPTC
    name: Top Card To Security Probe
    kind: digimon
    level: 5
    color: [yellow]
    cost: 5
    dp: 6000
    effects:
      - when: when_digivolving
        process:
          - security_place_top_card:
              carrier: source
              of: you
              position: bottom
              face: down
    ```
    (variants via `format!`: `position: top`, `face: up`).
  - setup: `make_test_card("UNDER","Under")` (level 4), `make_test_card("SEC","Sec")`; `.security(0, &["SEC","SEC"])`; `let h = runner.place_stack(0, &["UNDER","TST-SPTC"]);` set `battle_area[h].is_suspended = true`; enqueue `WhenDigivolving` on `h`; drain.
  - positive: `security_count(0) == 3`; `players[0].security[0].card_id == "TST-SPTC"` (bottom) and its `card_index` is NOT in `face_up_security`; the permanent is still on the field with `card_sources.len() == 1` and top card `UNDER`; `is_suspended` still true (§4-7-6); P0 trash unchanged. Top variant: `security.last()` is `TST-SPTC`. Face-up variant: `face_up_security` contains its `card_index`.
  - negative 1: `place_on_field(0, "TST-SPTC", ..)` (no sources) -> security unchanged, permanent unchanged.
  - negative 2: add `PlayerModifierEntry::simple(ModifierType::CannotAddSecurityByEffect, 0, Expiry::Permanent, None, 1)` to P0 via `runner.game.modifiers.add_player_modifier(0, ..)` (pattern in `tests/flood_gates/resolver_gates.rs:882`) -> no move.
  - selection surface: none (assert no pending selection).
- **Suspected engine/DSL defects:**
  - The step discards the success bool, so BT26-033's `cancel_replacement` runs even when the placement didn't happen (CannotAddSecurityByEffect, a `WhenWouldPlaceInSecurity` Cancel/redirect) — the [TS] Digimon is saved without the cost being paid (§15-7-2). BT26-033's `active_when` also doesn't check `CannotAddSecurityByEffect` (§15-7-4).
  - Only the ACTING player's `CannotAddSecurityByEffect` is consulted; with `of: opponent` (or a foreign-owned top card) the destination player's restriction is ignored (the Venusmon fix round 1 added the destination check to `place_on_security`, not here).

---

### `verb:trash_selected_security`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:98` (`TrashSelectedSecurity(HandleMoveArgs)`; key `:904`; `HandleMoveArgs { of, card }` `:2025`). Compiled `compiled.rs:1676`; compile `compile.rs:2878`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:944` -> `EffectContext::trash_security_card` `code/digimon-engine/src/effect_context/action/trash.rs:309`.
  ```rust
  // play_digivolve.rs:944
  let owner = resolve_player(ctx, *of);
  if let Some(ResolvedBinding::Card(handle)) = resolve_binding_ref(card, ctx, bindings) {
      let _ = ctx.trash_security_card(owner, handle);
  }
  // trash.rs:309
  let Some(initial_pos) = security.iter().position(|c| c.handle() == handle) else { return false; };
  self.game.ensure_security_materialized(player, initial_pos);
  if !self.game.would_replacement_proceeds(EffectTiming::WhenWouldBeTrashed,
          ReplacementSubject::Card(handle, Zone::Security), player, Some(Zone::Trash)) { return false; }
  let pos = /* re-find by handle */;
  let card = p.security.remove(pos);
  p.face_up_security.remove(&card.card_index);
  self.game.fire_security_removed_observers(player, self.player, card, SecurityRemovalDestination::Trash);
  ```
- **YAML shape:** consumes a `select_security` binding. From BT24-018:
  ```yaml
  - select_security:
      of: opponent
      bind_as: sec_to_trash
      optional: true
      filter:
        any_of:
          - kind: digimon
          - kind: tamer
          - kind: option
      prompt: "You may trash any 1 of your opponent's security cards"
  - trash_selected_security:
      of: opponent
      card: sec_to_trash
  ```
- **Semantics:** Trashes exactly the bound security card (addressed by stable `CardHandle`, so any position, not
  just the top) from `of`'s stack into its owner's trash. Unset binding (declined optional select, or no candidates so
  no prompt) or a handle no longer in that stack = silent no-op. Opens a `WhenWouldBeTrashed` replacement window first
  (subject `Card(handle, Security)`); a parked or cancelling replacement aborts the trash. Fires the effect-removal
  chain: `OnLoseSecurity` (effect_initiated = true), `OnDiscardSecurity` for the trashed card itself (destination
  Trash, card not played), then `OnOwnSecurityRemoved` / `OnOpponentSecurityRemoved`. The verb never prompts; the
  choice is the preceding `select_security` (a real `PendingSelection { kind: Security, zone_owner: Some(of) }`,
  `is_optional` = the YAML `optional`).
- **Rule basis:**
  - §4-16-1: trashing places the card in the trash (the owner's, §3-1-3-8).
  - §3-7-2: the chosen card is face-down/private; the choice is positional (production uses a kind-agnostic filter — "any 1").
  - §15-15-3-2: removal from the stack triggers "removed from security" observers.
  - card-text idiom: "trash any 1 of your opponent's security cards" (DCGO `BT24_018.cs`).
- **Production users:**
  - BT24-018 Styracomon (top-10: Medusamon): "[When Digivolving] You may trash any 1 of your opponent's security cards. Then, this Digimon may unsuspend." — YAML above.
  - (Only one production user.)
- **Existing near-miss tests:** coverage index lists only `src/dsl_cards/step/play_digivolve.rs`. Raw EffectContext coverage: `tests/effect_context/security_stack_operations.rs::trash_security_card_trashes_chosen_non_top_card_by_handle` and `::trash_security_card_no_op_for_handle_not_in_security` (no DSL lowering). `cards_behavioral/bt24/bt24_018.rs` (never counts).
- **Proposed behavioural test (for the plan):**
  - fixture:
    ```yaml
    card: TST-TSS
    name: Trash Selected Security Probe
    kind: digimon
    level: 4
    color: [green]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - select_security:
              of: opponent
              bind_as: sec
              optional: true
              filter: {}
              prompt: "You may trash any 1 of your opponent's security cards"
          - trash_selected_security: { of: opponent, card: sec }
    ```
  - setup: `make_test_card("SA","A")`, `("SB","B")`, `("SC","C")`; `.security(1, &["SA","SB","SC"])`; place fixture on P0; enqueue `WhenDigivolving`; drain.
  - positive: pending selection with `selecting_player == 0`, `kind == SelectionKind::Security`, `zone_owner == Some(1)`, `is_optional == true`, `valid_action_ids == [50, 51, 52]`. Execute `51` (the MIDDLE card `SB`) -> `players[1].security` ids == `["SA","SC"]`; `players[1].trash` contains `SB`; P0 trash unchanged.
  - negative 1: execute `PASS` -> security still 3, trash unchanged.
  - negative 2: `.security(1, &[])` -> no pending selection at all, nothing moves.
  - selection surface: the assertions above (real prompt, no auto-pick, optional per "You may").
  - observer cross-check (optional): author `SB` as inline YAML with a self clause `when: on_discard_security` (`gain_memory: 1`) -> fires once when SB is trashed (P1 gain -> memory -1), and not at all on PASS.
- **Suspected engine/DSL defects:** none in the verb. (Upstream `select_security` addresses only indices `0..min(len, 10)` counted from the bottom, so the top cards of an 11+ card stack can never be chosen — `effect_context/selections.rs:1253-1259`.)

---

### `verb:play_security_card`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:93` (`PlaySecurityCard(HandleMoveArgs)`; key `:903`). Compiled `compiled.rs:1670`; compile `compile.rs:2874`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/play_digivolve.rs:931` -> `EffectContext::play_from_security_card` `code/digimon-engine/src/effect_context/action/play.rs:655` -> `play_from_security_index` `play.rs:741`.
  ```rust
  // play_digivolve.rs:931
  let owner = resolve_player(ctx, *of);
  if let Some(ResolvedBinding::Card(handle)) = resolve_binding_ref(card, ctx, bindings) {
      if let Some(played) = ctx.play_from_security_card(owner, handle) {
          bindings.record_played(played);
      }
  }
  // play.rs:741 play_from_security_index (abridged)
  self.game.ensure_security_materialized(player, security_index);
  let card = player_state.security.remove(security_index);
  let was_face_up = self.game.player_mut(player).face_up_security.remove(&card_index);
  self.game.player_mut(player).put_in_hand(card);      // hand-transit
  match game.play_from_hand_with_cost_result_from_origin(player, hand_index, CostDelta::Free,
          PlaySource::ByEffect, false, PendingWouldPlayOrigin::SecurityTop { was_face_up }) {
      Played(i) => Some(handle), Pending => None,
      Failed => { /* pop from hand, re-insert at security_index */ None }
  }
  ```
- **YAML shape:** after a `select_security` binding, or `card: self` from a `{Security}`-scoped clause. From BT13-012:
  ```yaml
  - select_security:
      of: you
      bind_as: sec_tamer
      optional: true
      filter:
        all_of:
          - kind: tamer
          - any_of: [ { color_is: red }, { color_is: yellow } ]
      prompt: "Play 1 red or yellow Tamer from security without paying its cost"
  - if:
      condition: { binding_present: sec_tamer }
      then:
        - play_security_card: { of: you, card: sec_tamer }
        - recover: { of: you, count: 1 }
  - shuffle_security: { of: you }
  ```
- **Semantics:** Plays the bound card out of `of`'s security stack without paying its cost, through the normal
  hand-play pipeline (`PlaySource::ByEffect`): battle-area capacity, play floodgates (`CannotPlay...ByEffect`),
  would-play replacements, `[On Play]` / enter-field observers all apply. On success the new permanent is recorded
  (`record_played`, readable by `effect_played_any_*`). Unset binding / handle not in that stack = no-op. Immediate
  failure (e.g. an Option, or a gated play) restores the card to its ORIGINAL security index; a parked would-play that
  is later cancelled restores via `PendingWouldPlayOrigin::SecurityTop`, i.e. to the TOP. The verb never prompts. It
  does NOT route through `fire_security_removed_observers`: no `OnLoseSecurity`, `OnOwnSecurityRemoved` or
  `OnOpponentSecurityRemoved` fires for the card leaving the stack.
- **Rule basis:**
  - §3-7-2 / §3-7-3: private, fixed-order stack (searching it / choosing a card / reshuffling is card-text permission).
  - §3-1-3-1-1: the played card is a new card.
  - §15-15-3-2: the stack's count decreased, so "when a card is removed from your security stack" should trigger (see defects).
  - card-text idiom: "play 1 ... among it without paying its cost".
- **Production users:**
  - BT26-082 Ravemon (top-10: DATA SQUAD Ravemon): "[Security] [End of Opponent's Turn] Play this card without paying the cost." —
    ```yaml
    - when: end_of_opponents_turn
      scope: security
      process:
        - play_security_card: { of: you, card: self }
    ```
  - BT13-012 GeoGreymon: "[When Digivolving] Search your security stack, and you may play 1 red or yellow Tamer card among it without paying its cost. If you did, <Recovery +1 (Deck)>. Then, shuffle your security stack." — YAML above.
- **Existing near-miss tests:** coverage index lists only `src/dsl_cards/step/play_digivolve.rs`. Card tests `cards_behavioral/bt13/bt13_012.rs`, `bt15/bt15_092.rs` (never count).
- **Proposed behavioural test (for the plan):**
  - fixture:
    ```yaml
    card: TST-PSC
    name: Play Security Card Probe
    kind: digimon
    level: 4
    color: [red]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - select_security:
              of: you
              bind_as: pick
              optional: true
              filter: { kind: tamer }
              prompt: "You may play 1 Tamer from your security stack without paying its cost"
          - play_security_card: { of: you, card: pick }
    ```
  - setup: a Tamer card `TAM` (`card_kind = CardKind::Tamer`, `play_cost = 5`) and a Digimon `DIG` via `make_test_card`; `.security(0, &["TAM","DIG"])` (TAM bottom); `.memory(3)`; place fixture on P0; enqueue `WhenDigivolving`; drain.
  - positive: prompt `valid_action_ids == [40]` (only the Tamer passes the filter), `is_optional == true`; execute `40` -> P0 battle area +1 whose top card is `TAM`; `security_count(0) == 1` and the remaining card is `DIG`; P0 hand size unchanged (the hand-transit is invisible); memory still 3 (free play).
  - negative 1: `PASS` -> battle area and security unchanged.
  - negative 2: binding points at a card that left the stack (move the Tamer out of security before answering, or bind via a second select on a now-empty stack) -> no-op.
  - selection surface: only `select_security` prompts; assert no second prompt from the verb.
  - observer check (documents current behaviour; flip when fixed): an `on_lose_security` observer on P0 does NOT fire today.
- **Suspected engine/DSL defects:**
  - Playing from security fires no security-removed observers. DCGO `BT13_012.cs:113` / `BT15_092.cs:151` call `IReduceSecurity(...).ReduceSecurity()` before `PlayPermanentCards(root: Security)`, which stacks `EffectTiming.OnLoseSecurity` (`CardController.cs:5914`); §15-15-3-2 agrees. The `EffectTiming::OnLoseSecurity` doc (`enums.rs:243-246`) even claims it fires when a card is "played from security by an effect".
  - A parked-then-cancelled would-play restores the card to the TOP of security (`game_actions/misc.rs:1151`, `SecurityTop`), not its original index — wrong for a `select_security`-chosen non-top card.
  - Upstream `select_security` (optional) installs `ResumeDecline::None`: PASS drops the rest of the clause, so BT13-012's and BT15-092's "Then, shuffle your security stack" (and BT15-092's Kari Kamiya self-top) don't happen when the player declines (`resume.rs:405-408`, `selections.rs:8428`).

---

### `timing:on_lose_security`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/clause.rs:255` (`OnLoseSecurity`, YAML `when: on_lose_security`). Compiled `compiled.rs:1464`; compile `compile.rs:340`.
  - Mapping: `code/digimon-engine/src/dsl_cards/timing_map.rs:59` (`CompiledTiming::OnLoseSecurity => EffectTiming::OnLoseSecurity`); builder `code/digimon-engine/src/dsl_cards/lower_triggered.rs:1561` (`Effect::on_lose_security(card)`, `effect.rs:651`).
  - Fire sites: (a) security CHECK — `code/digimon-engine/src/combat/mod.rs:3427`; (b) every EFFECT removal — `Game::fire_effect_security_removal` `effect_queue.rs:5652` (reached via `fire_security_removed_observers`, `game_actions/helpers.rs:122`, and directly from `game_actions/mod.rs:2820` / `:4392` / `:4465`, `game_actions/digivolve.rs:3046`).
  - Dispatch: scan `effect_queue.rs:593-609`, trigger context `effect_queue.rs:2241`, removed-card scope filter `effect_queue.rs:3024`.
  ```rust
  // effect_queue.rs:593  TriggerSource::SecurityRevealed { defender, card }
  let trigger_context = self.trigger_context_for_source(&source, None, timing);
  self.enqueue_from_security_card(timing, defender, card, Some(trigger_context.clone())); // removed card: effect.security only (:3024)
  if timing == EffectTiming::OnLoseSecurity {
      for i in 0..self.player(defender).battle_area.len() {          // DEFENDER's battle area only
          self.enqueue_from_permanent(timing, PermanentHandle { player: defender, index: i as u8 },
                                      Some(trigger_context.clone()));
      }
  }
  // effect_queue.rs:2241 context: target_card = event_card = removed card, source_player = defender,
  //   effect_initiated: timing == OnLoseSecurity && self.pending_security_loss_effect_initiated
  // effect_queue.rs:5652 fire_effect_security_removal (every effect removal):
  self.pending_security_loss_effect_initiated = true;
  self.enqueue_triggered(EffectTiming::OnLoseSecurity, TriggerSource::SecurityRevealed { defender, card });
  self.pending_security_loss_effect_initiated = false;
  self.maybe_drain_effect_queue();   // deferred while an effect body resolves (15-8-3-2)
  ```
- **YAML shape:** from BT24-101:
  ```yaml
  - when: on_lose_security
    once_per_turn: true
    active_when:
      all_turns: true
    trigger_if:
      event_target_owner: you
    process:
      - trash_top_security: { of: opponent }
  ```
- **Semantics — and how it differs from the neighbouring timings:**
  - `on_lose_security` -> `EffectTiming::OnLoseSecurity`, `TriggerSource::SecurityRevealed { defender, card }`. Fires for **both** a security check (combat, `effect_initiated = false`) **and** an effect removal (`effect_initiated = true`, readable as `event_is_effect_initiated`). Effect removals covered: `trash_top_security` / bottom / `trash_selected_security`, `add_top_security_to_hand` / `add_to_hand_from_security`, `return_top_security_to_deck` / `return_selected_security_to_deck`, security->digivolution-source placement, digivolve-from-security, a `WhenWouldPlaceInSecurity` redirect-to-trash of a security card. **Not** covered: `play_security_card` / `play_from_security` (no observer fire — see that construct), adding to security (`recover`, `place_on_security`), revealing (§15-15-3-1). Observers scanned: the removed card itself (only clauses with `scope: security`, i.e. `effect.security`) + every permanent in the **defender's** (stack owner's) battle area. NOT the opponent's battle area, NOT the breeding area. The trigger is enqueued while the card is still in `pending_security` (before it reaches its destination); outside an effect body it drains immediately, inside one it waits for the body to finish (§15-8-3-2).
  - `on_own_security_removed` -> `EffectTiming::OnOwnSecurityRemoved`, `TriggerSource::SecurityRemoved { observer_player = defender }`: defender's battle area **and breeding area**; fires at a check (`combat/mod.rs:3434`) and, for effect removals, only AFTER the move completes (`complete_effect_security_removal`, `effect_queue.rs:5607`).
  - `on_opponent_security_removed` -> `EffectTiming::OnOpponentSecurityRemoved`: observer = the attacker's player at a check (`combat/mod.rs:3436`); for effect removals observer = `next_clockwise(defender)` (the stack owner's opponent, whoever removed it), and only when observer != defender (`effect_queue.rs:5627`); battle + breeding area; after completion.
  - `on_discard_security` -> only the removed card itself, only for an effect trash (destination Trash, not played); never at a check (`effect_queue.rs:5380`).
  - Triggered-clause flags: `optional`, `once_per_turn`, `trigger_if` (evaluated at enqueue with the context above), `condition` (at activation).
- **Rule basis:**
  - §13-1-6: a checked card is removed from the security stack (check-driven removal counts).
  - §15-15-3-1 / §15-15-3-2: revealing doesn't remove; trashing a revealed security card does (count decreased).
  - §15-5-2: several cards removed at the same time trigger such an observer once.
  - §15-8-3-2: the observer can't activate during the processing of the effect that removed the card.
  - §15-14-5-1: a `{Security}` effect is live on a face-up security card (the removed card's own `scope: security` clause).
- **Production users:**
  - BT24-101 Jupitermon (top-10: Jupitermon x2): "[All Turns] [Once Per Turn] When your security stack is removed from, trash your opponent's top security card." — YAML above.
  - BT26-089 Kyo Sawashiro: "[All Turns] When your security stack is removed from, by suspending this Tamer, place the top card of your deck face down under this Tamer. After, if removed from by effects, give 1 of your opponent's Digimon <Security A. -1> until their turn ends." —
    ```yaml
    - when: on_lose_security
      optional: true
      active_when: { all_turns: true }
      condition: { source_is_unsuspended: true }
      process:
        - activation_cost: { suspend_self: true }
        - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: true }
        - if:
            condition: { event_is_effect_initiated: true }
            then: [ ... select_opponent_permanent + add_modifier SecurityAttackChange -1 ... ]
    ```
- **Existing near-miss tests:**
  - `tests/effect_context/security_stack_operations.rs` — `trash_top_security_fires_removed_cards_on_lose_security_once`, `trash_top_security_fires_defenders_battle_area_on_lose_security_once`, etc. use hand-built `Effect::on_lose_security(card)` (no DSL timing lowering).
  - `tests/effect_context/effect_digivolve_from_zones.rs:20`, `tests/effect_context/source_stack_operations.rs:25` — same hand-built `Effect::on_lose_security`.
  - `tests/effects/queue_drainer.rs::derived_trigger_resolves_before_still_pending_sibling` — a REAL DSL `when: on_lose_security` observer (`DERIVED_OBSERVER_YAML`, ~line 571) firing after a DSL `trash_top_security`; behavioural, but `tests/effects/` is not a counted directory.
  - `tests/token_source_destination.rs:125` — DSL `on_lose_security` with `scope: security` used as a parking witness (top-level `tests/*.rs`, not counted).
  - `src/dsl_cards/lower_triggered.rs`, `src/dsl_cards/timing_map.rs` — source.
- **Proposed behavioural test (for the plan):** `tests/dsl/on_lose_security_timing.rs` (or a `tests/timing_dispatch*` file).
  - fixtures:
    ```yaml
    card: TST-OLS
    name: Lose Security Observer
    kind: tamer
    color: [yellow]
    cost: 3
    effects:
      - when: on_lose_security
        active_when: { all_turns: true }
        process:
          - gain_memory: 1
          - if:
              condition: { event_is_effect_initiated: true }
              then:
                - gain_memory: 2
    ```
    ```yaml
    card: TST-TRASHSEC
    name: Security Trasher
    kind: digimon
    level: 4
    color: [red]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - trash_top_security: { of: you }
    ```
    plus `TST-TRASHSEC-OPP` (same, `of: opponent`) and an adjacent-timing control `TST-OPPREM` (= `TST-OLS` with `when: on_opponent_security_removed`).
  - setup: `.security(0, &["S","S","S"])`, `.security(1, &["S","S","S"])`, `.memory(0)`; `place_on_field(0, "TST-OLS", Some(0))`.
  - positive A (effect removal): place `TST-TRASHSEC` on P0, enqueue `WhenDigivolving`, drain -> memory == +3 (fired once, effect bit true); `security_count(0) == 2`.
  - positive B (security check): a P1 attacker attacks P0 (`runner.attack_player(attacker, 0, true)`, as in `security_stack_operations.rs::attack_security_check_does_not_fire_on_discard_security`) -> P0's observer fires with the effect bit false -> memory +1 relative to before.
  - negative 1 (other player's stack): `TST-TRASHSEC-OPP` on P0 trashes P1's top card -> `TST-OLS` on P0 does not fire (memory unchanged). This pins the engine's defender-only scan (see the BT23-102 defect).
  - negative 2 (adding is not losing): a clause with `recover: { of: you, count: 1 }` -> no fire.
  - adjacent timing: with `TST-OPPREM` also on P0's field, positive A must leave `TST-OPPREM` silent (P0's OWN stack); `TST-TRASHSEC-OPP` must fire `TST-OPPREM` exactly once and `TST-OLS` zero times.
  - removed-card scope: a security card whose own clause is `when: on_lose_security` WITHOUT `scope: security` must not fire for its own removal; with `scope: security` it fires once (`effect_queue.rs:3024`).
  - selection surface: none (non-optional fixture); for an `optional: true` variant assert an outer accept/decline prompt with `is_optional == true`.
- **Suspected engine/DSL defects:**
  - **BT23-102 Mastemon under-fires.** Printed "When security stacks are removed from" (either player's); DCGO `BT23_102.cs:337` uses `CanTriggerWhenLoseSecurity(hashtable, null)` and `AutoProcessing.GetSkillInfos` scans BOTH players' field permanents. The engine enqueues `OnLoseSecurity` only on the defender's battle area (`effect_queue.rs:600-608`), so Mastemon never sees the opponent's security being removed (its card test leans on this: `cards_behavioral/bt23/bt23_102.rs:255`). "Your security stack" cards (BT24-101 etc.) are fine.
  - `play_security_card` / `play_from_security` removals fire nothing (see `verb:play_security_card`), contrary to DCGO and the enum doc.
  - The breeding area is not scanned for `OnLoseSecurity` though it is for `OnOwnSecurityRemoved` (no current user needs it).

---

### `predicate:owner_can_add_security`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/predicate.rs:297` (`owner_can_add_security: Option<bool>`); compiled `compiled.rs:488`; compile `compile.rs:1090`.
  - Runtime: permanent subject `code/digimon-engine/src/dsl_cards/predicate.rs:4370` (in `eval_permanent_fields`, `:4077`); breeding subject `predicate.rs:4951` (forces false); listed as a permanent-only leaf for `any_permanent` routing in `formula_eval.rs:1620` and for the replacement payability guard in `lower_replacement.rs:479`.
  ```rust
  // predicate.rs:4370
  if let Some(want) = pred.owner_can_add_security {
      let can = !rctx.game.modifiers.player_has(
          perm.top_card().owner,
          crate::enums::ModifierType::CannotAddSecurityByEffect,
      );
      if can != want { return false; }
  }
  // predicate.rs:4947 (eval_breeding_permanent_fields tail): ... && pred.owner_can_add_security.is_none() && ...
  ```
- **YAML shape:** a permanent-filter leaf. From BT24-040:
  ```yaml
  - select_any_permanent:
      bind_as: cost_body
      filter:
        all_of:
          - kind: digimon
          - materials_count_lte: 0
          - owner_can_add_security: true
          - not_in_binding: replacement_subject
      prompt: "Place another sourceless Digimon as bottom security"
  ```
- **Semantics:** True iff the OWNER of the permanent's TOP card has no player-scoped `CannotAddSecurityByEffect`
  modifier (`false` inverts). It reads the top card's owner — not the controller and not the acting player. It does not
  consult the permanent-scoped `CannotAddSecurity` modifier, a `WhenWouldPlaceInSecurity` replacement, or token /
  Digi-Egg status. On a breeding-area subject the leaf makes the whole predicate false (for both `true` and `false`).
  It is evaluated wherever a permanent predicate is: selection filters (candidate set), `any_permanent` conditions
  (BT4-105), and the replacement payability guard (an optional replacement with no payable body is not offered).
  Added 2026-10-20 (Venusmon fix round 2, `qa/dsl-vocab-gaps.md:9122`): an unplaceable cost body can't be chosen
  (§15-7-4); DCGO still offers it (`dcgo_quirk`).
- **Rule basis:**
  - §15-7-4: an optional processing condition that can't be executed can't be chosen (so its body can't be offered).
  - §3-1-3-8: the Digimon goes to its owner's security when no player is named — hence the owner, not the controller.
- **Production users:**
  - BT24-040 Venusmon (top-10: TS Toolbox, Chronomon): "[All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave the battle area other than by your effects, by placing 1 other Digimon with no digivolution cards as the bottom security card, they don't leave." — YAML above (followed by `place_on_security: { source: { permanent: cost_body }, to_owner: true, position: bottom, disposition: cancel }`).
  - BT4-105 Tactical Retreat!: "[Main] Place 1 of your Digimon on top of your security stack face down." —
    ```yaml
    - if:
        condition:
          any_permanent: { of: you, zone: [battle_area], kind: digimon, owner_can_add_security: true }
        then:
          - select_own_permanent:
              bind_as: retreating
              filter: { kind: digimon, owner_can_add_security: true }
              prompt: "Place 1 of your Digimon on top of your security stack face down"
          - place_on_security: { source: { permanent: retreating }, of: you, position: top, face_up: false }
    ```
- **Existing near-miss tests:** coverage index lists only source (`src/dsl_cards/formula_eval.rs`, `lower_replacement.rs`, `predicate.rs`). Card test `cards_behavioral/bt24/bt24_040.rs::bt24_040_cost_body_whose_owner_cannot_add_security_is_not_offered` exercises it through the production card (never counts).
- **Proposed behavioural test (for the plan):** `tests/dsl/predicate_owner_can_add_security.rs`.
  - fixture:
    ```yaml
    card: TST-OCAS
    name: Owner Can Add Security Probe
    kind: digimon
    level: 4
    color: [yellow]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - if:
              condition:
                any_permanent: { of: opponent, zone: [battle_area], kind: digimon, owner_can_add_security: true }
              then:
                - gain_memory: 1
          - select_any_permanent:
              bind_as: body
              optional: true
              filter:
                all_of:
                  - kind: digimon
                  - owner_can_add_security: true
              prompt: "Choose a Digimon whose owner can add security"
    ```
  - setup: Digimon `X0`, `X1` via `make_test_card`; `place_on_field(0,"X0")`, `place_on_field(1,"X1")`, fixture on P0; `.memory(0)`; enqueue `WhenDigivolving`; drain.
  - positive: no modifiers -> memory +1 and the select offers 3 ids (X0, the fixture itself, X1). On a fresh runner add `CannotAddSecurityByEffect` to P1 (`runner.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotAddSecurityByEffect, 0, Expiry::Permanent, None, 0))`) -> memory unchanged (`any_permanent` false) and the select offers 2 ids (X1 excluded).
  - owner-not-controller: on a fresh runner set `players[0].battle_area[x0].card_sources.last_mut().owner = 1` and put the modifier on P1 -> X0 (controlled by P0, owned by P1) is excluded along with X1; with the modifier only on P0 instead, X0 and X1 are both offered and only the fixture (owned by P0) is excluded.
  - negative: modifier on P0 only (no owner change) -> X1 is still offered (the leaf ignores the acting player).
  - `owner_can_add_security: false` variant -> only the restricted player's Digimon are offered.
  - selection surface: `is_optional == true` (fixture), no auto-pick; PASS leaves everything unchanged.
- **Suspected engine/DSL defects:** none against BT24-040 (its placement step also checks the destination player). Note the leaf and `place_on_security { of: you }` (BT4-105) consult different players (owner vs acting player) and the leaf ignores the permanent-scoped `CannotAddSecurity`; equivalent for the current users (own Digimon, owner = actor).

---

### `verb:select_zone_cards`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:469` (`SelectZoneCards(SelectZoneCardsArgs)`; key `:1095`; args `:3726`; `ZoneCardZone { Hand, Trash, BattleArea, LinkCards }` `:3695`). Compiled `compiled.rs:2638`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/zone_cards.rs:203` (install; dispatched from `step/mod.rs:796`), candidates `zone_cards.rs:126`, live options `:367`, prompt `step_or_finish` `:379`, resume `run_zone_card_pick_step` `:447`, terminal `finish` `:476`.
  ```rust
  // zone_cards.rs:236  install
  let mut candidates = collect_zone_candidates(ctx, player, zones, filter, exclude, &bindings);
  if candidates.len() < *min as usize { candidates.clear(); }   // unpayable count -> offer nothing
  // zone_cards.rs:379  step_or_finish
  let options = live_options(game, &state);                     // still locatable, not picked, cost <= budget
  if picked >= state.max || options.is_empty() {
      if picked < state.min && !(state.optional_zero && picked == 0) { state.picked.clear(); }
      finish(game, state); return;
  }
  let is_optional = picked >= state.min || (state.optional_zero && picked == 0);
  // valid ids = options (+ PASS if is_optional); kind = PlayCostBudget{remaining,picked} if budget
  //   else CountCappedMultiSelect{min,max,picked}; phase SelectBudgeted; one pick per decision
  // zone_cards.rs:487  finish/Bind: no picks -> binding stays UNBOUND; else insert_card_list(name, picked)
  ```
- **YAML shape:** from BT26-081 (budget form) and BT26-102 (all-or-nothing form):
  ```yaml
  - select_zone_cards:
      zones: [hand, trash]
      filter: { all_of: [ { trait_has: Iliad }, { any_of: [ { kind: digimon }, { kind: tamer } ] } ] }
      play_cost_budget: 8
      bind_as: iliad_plays
      prompt: "You may play up to 8 play cost's total worth of [Iliad] trait cards ..."
  - select_zone_cards:
      zones: [battle_area, link_cards, trash]
      filter: { all_of: [ { kind: digimon }, { trait_has: "Seven Code" } ] }
      min: 6
      max: 6
      optional_zero: true
      exclude: host
      bind_as: materials
      prompt: "Place 6 [Seven Code] trait Digimon cards ..."
  ```
- **Semantics:** A one-card-per-decision multi-pick over concrete CARDS of `of`'s (default you) listed zones: hand
  cards, trash cards, the TOP card of each battle-area permanent (except `exclude`; empty stacks skipped) and every link
  card. Candidates are fixed at install (filter evaluated with the card as subject) and re-located every step (a card
  that left its zone drops out). `min` (default 0), `max` (default unbounded = 255), `optional_zero` ("0 or at least
  min"), `play_cost_budget` (each pick's play cost must fit the remaining budget; the formula is evaluated once at
  install). Fewer candidates than `min` -> no prompt at all and the binding stays unbound. PASS is offered only when
  committable (`picked >= min`, or `optional_zero && picked == 0`); the prompt ends automatically at `max` or when no
  live option remains; a short all-or-nothing pick commits nothing. On finish the picks (pick order) bind as a card
  list and the captured tail runs; zero picks leave the name unbound so `binding_present` is false. `zone_owner` is set
  when `of` is not the selecting player. Clone-safe (`ResumeFrame::ZoneCardPickStep`, inert callback).
- **Rule basis:**
  - card-text idiom: "up to N play cost's total worth of ..." / "By placing 6 ... from your battle area, link cards or trash".
  - §15-7-3 / §15-7-4: an all-or-nothing cost ("By placing 6") can't be partly paid and can't be chosen if unpayable (`min` / `optional_zero` behaviour).
  - §4-7-1: a "card from your battle area" is a stack's top card.
- **Production users:**
  - BT26-081 Mervamon (top-10: TS Toolbox, Chronomon): "[On Play] [When Digivolving] You may play up to 8 play cost's total worth of [Iliad] trait cards from your hand or trash without paying the costs." — YAML above, then `play_cards_free: { cards: iliad_plays }`.
  - BT26-102 Seven Code PAD: "[Main] By placing 6 [Seven Code] trait Digimon cards from your battle area, link cards or trash as 1 of your [Seven Code] trait Digimon's bottom digivolution cards, ..." — YAML above, then `if: { binding_present: materials }` -> `place_cards_as_bottom_sources`.
- **Existing near-miss tests:**
  - `tests/top_only_field_card_placement.rs` — DSL `select_zone_cards { zones: [battle_area], min 1, max 1, optional_zero, exclude: source }` + `place_cards_as_bottom_sources`, driven through the action mask (EX2-007 Q3287 probe). Behavioural, but a top-level `tests/*.rs` file, so not counted.
  - `src/dsl_cards/step/zone_cards.rs` — source.
- **Proposed behavioural test (for the plan):** `tests/dsl/select_zone_cards.rs`.
  - fixture A (budget, hand+trash):
    ```yaml
    card: TST-SZC-BUDGET
    name: Zone Pick Budget Probe
    kind: digimon
    level: 4
    color: [purple]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - select_zone_cards:
              zones: [hand, trash]
              filter: { name_is: "Probe" }
              play_cost_budget: 5
              bind_as: picked
              prompt: "Pick up to 5 play cost's total worth of Probe cards"
          - if:
              condition: { binding_present: picked }
              then:
                - gain_memory: 1
    ```
  - fixture B (count, trash only, observable consumer):
    ```yaml
    card: TST-SZC-COUNT
    name: Zone Pick Count Probe
    kind: digimon
    level: 4
    color: [purple]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - select_zone_cards:
              zones: [trash]
              filter: { name_is: "Probe" }
              min: 2
              max: 2
              optional_zero: true
              bind_as: picked
              prompt: "Pick exactly 2 Probe cards, or none"
          - return_trash_list_to_deck_bottom: { of: you, cards: picked }
    ```
  - setup A: `P2` (name "Probe", `play_cost = 2`) in hand; `P3` (cost 3) and `P4` (cost 4) in trash via `inject_trash`; `Q1` (name "Other") in hand. Place fixture, enqueue `WhenDigivolving`, drain.
  - positive A: first prompt `kind == PlayCostBudget { remaining_play_cost: 5, picked: 0 }`, `is_optional == true`, ids == {`PLAY_HAND_START + idx(P2)`, `TRASH_EFFECT_START + idx(P3)`, `TRASH_EFFECT_START + idx(P4)`, `PASS`} (Q1 absent). Pick P3 -> second prompt remaining 2, ids == {P2, PASS} (P4 now over budget). Pick P2 -> no further prompt (budget exhausted), memory +1.
  - negative A: PASS at the first prompt -> memory unchanged (binding unbound).
  - positive B: three matching cards in trash. First prompt `CountCappedMultiSelect { min: 2, max: 2, picked: 0, .. }` with PASS present (`optional_zero`); after one pick the prompt has NO PASS (`is_optional == false`); after the second pick it finishes: trash -2, deck +2 with the first pick deepest (`deck[0]`).
  - negative B: only 1 matching card in trash -> no prompt at all, trash and deck unchanged (fewer candidates than `min`).
  - variant: `min: 1, max: 1` without `optional_zero` -> first prompt `is_optional == false` (no PASS).
  - selection surface: every decision is a real `pending_selection` (assert kind, `selecting_player == 0`, `zone_owner == None`); never auto-picked even when exactly one option remains.
- **Suspected engine/DSL defects:** Candidates whose encoded id is out of range (hand index >= 30, trash index past its range, stack index >= 12, field slot >= 14) silently disappear (`encode_zone_card` returns `None`), so a large hand/trash can hide a legal pick. Otherwise none seen.

---

### `verb:return_all_to_deck`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:181` (`ReturnAllToDeck(ReturnAllToDeckArgs)`; key `:936`; args `{ over, position, include_sources (default false), selector }` `:2047`). Compiled `compiled.rs:3137`; compile `compile.rs:3063`.
  - Runtime: expanded by `code/digimon-engine/src/dsl_cards/step/return_results.rs:6` (dispatcher `step/mod.rs:737` / `:879`) into `ReturnCapturedBatch` + `ReturnReceiptFinalize` (`return_results.rs:52`) -> `replacement::start_departure_batch` `code/digimon-engine/src/replacement_departure_batch.rs:11` -> ordering prompt in `finish_departure_window` `replacement_departure_batch.rs:93` -> `Game::commit_simultaneous_returns` `code/digimon-engine/src/game_actions/grouped_return.rs:10`.
  ```rust
  // return_results.rs:21  expand
  let mut targets = super::permanent_scan::scan(ctx, over, Some(bindings));   // both battle areas, scoped by `of`
  if let Some(selector) = selector { /* keep only the extreme value among matches */ }
  targets.sort_by_key(|h| (h.player, h.index)); targets.dedup();
  targets.retain(|h| ctx.can_affect_permanent(*h));
  // -> ReturnCapturedBatch { receipt, targets(identities), position, include_sources } + ReturnReceiptFinalize
  // replacement_departure_batch.rs:120  (after WhenWouldLeaveBattleArea + WhenWouldBeReturnedToDeck windows)
  if d.position.is_some() && remaining.len() > 1 {
      // PendingSelection { kind: AnyField, selecting_player: cause_player, is_optional: false,
      //   valid_action_ids: encode_attack(h.player, h.index) per remaining member,
      //   prompt: "Choose the bottommost remaining card" | "Choose the topmost remaining card" }
  }
  // grouped_return.rs:68  per member: token -> vanishes; include_sources -> whole stack to owners' decks,
  //   else top card -> insert_card_into_deck(owner, top, position) and sources -> owners' trash; links -> trash
  ```
- **YAML shape:** from EX13-024:
  ```yaml
  - return_all_to_deck:
      over:
        of: opponent
        zone: [battle_area]
        kind: digimon
        materials_count_matches_aggregate: { selector: fewest_materials, of: opponent }
      position: bottom
  ```
- **Semantics:** Scans both battle areas once with the `over` permanent predicate (optionally narrowed by
  `selector` to the extreme-value matches), drops effect-immune targets, captures stable identities, and moves every
  surviving match off the field as ONE simultaneous departure (one `WhenWouldLeaveBattleArea` window, then one
  `WhenWouldBeReturnedToDeck` window over the batch; prevented members stay). With 2+ survivors and a deck position the
  effect's controller orders them: a mandatory one-at-a-time `AnyField` prompt (first pick = bottommost for `bottom`,
  topmost for `top`); one survivor -> no prompt. Each top card goes to its OWNER's deck at `position` (Digi-Egg ->
  Digi-Egg deck); without `include_sources` its digivolution cards are trashed in one `OnDigivolutionCardTrashed`
  event; with `include_sources` the whole stack goes to the deck(s) per physical owner. Tokens vanish. Link cards are
  trashed. Returned top cards are recorded (`effect_returned_any_card`). No matches -> no-op, no prompt.
- **Rule basis:**
  - §3-1-3-3: cards leaving an area at the same time leave and are placed simultaneously.
  - §3-1-3-4: the player who owns the card causing the action chooses the order (its example is exactly "return all ... to the bottom of the deck in any order").
  - §3-1-3-8: each card goes to its owner's deck.
  - §4-7-8: cards under a removed card are trashed at the same time.
  - §4-21-3 / §15-1-10: tokens are not placed in the deck.
  - §15-5-2: one simultaneous return is one event for observers.
- **Production users:**
  - EX13-024 Slayerdramon (top-10: Examon): "... Then, you may return all of their Digimon with the fewest digivolution cards to the bottom of the deck." — YAML above (inside `if: { equals: [bottom_choice, 0] }` after a `select_effect_choice`).
  - BT20-045 Examon: "[When Digivolving] If DNA digivolving, return all of your opponent's Digimon with the highest DP to the bottom of the deck." —
    ```yaml
    - return_all_to_deck:
        over:
          all_of:
            - of: opponent
            - zone: [battle_area]
            - kind: digimon
            - dp_gte: { aggregate: { selector: highest_dp, scope: opponent } }
        position: bottom
    ```
- **Existing near-miss tests:**
  - `tests/grouped_return_contract.rs` — THE behavioural contract for this verb via inline DSL (`GROUP-CONTRACT` fixture): all six orderings of three members for `top`, `include_sources` for top/bottom, sources routed to physical owners, clone/digest agreement. Top-level `tests/*.rs`, so not counted. **Porting its first three tests into `tests/dsl/` is the cheapest credit.**
  - `tests/return_receipt_replay_reset.rs:14` — DSL `return_all_to_deck` producer used for receipt/replay reset (top-level, not counted).
  - `src/dsl_cards/step/return_results.rs` — source.
- **Proposed behavioural test (for the plan):** `tests/dsl/return_all_to_deck.rs`, reusing the `GROUP-CONTRACT` fixture and the `act` / `choose` / `begin` helpers from `tests/grouped_return_contract.rs`:
  ```yaml
  card: GROUP-CONTRACT
  name: Group contract fixture
  kind: digimon
  color: [blue]
  level: 3
  cost: 0
  dp: 2000
  effects:
    - when: on_play
      process:
        - return_all_to_deck:
            over: {of: opponent, zone: [battle_area], kind: digimon, level_eq: 4}
            position: bottom
            include_sources: false
  ```
  - setup: as in `grouped_return_contract.rs::fixture` (level-4 `TOP-A/B/C`, level-3 sources, `skip_mulligan`, Main phase, play from hand via `decode_action(PLAY_HAND_START, 0)`).
  - positive 1 (single member): one level-4 opponent Digimon on a 2-card stack -> no prompt; `players[1].battle_area` empty; `players[1].deck[0]` is the top card; its source is in P1's trash.
  - positive 2 (ordering): two members -> prompt with `selecting_player == 0`, `is_optional == false`, `valid_action_ids.len() == 2` (`encode_attack(1, i)`); pick member 1 first -> `deck[0]` is member 1's card, `deck[1]` member 0's. Top variant: pick order = top-down.
  - negative: a level-3 opponent Digimon only -> no prompt, nothing moves. A level-4 Digimon of P0 (wrong `of`) is untouched.
  - immunity: make the target unaffectable by the opponent's effects (whatever `can_affect_permanent` checks) -> skipped.
  - selection surface: the ordering prompt is mandatory (no PASS) and never auto-picked with 2+ members.
- **Suspected engine/DSL defects:** none seen. (The ordering prompt also appears when the returned cards go to different owners' decks, where relative order is meaningless; harmless.)

---

### `verb:return_union_bound_to_deck`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:352` (`ReturnUnionBoundToDeck(ReturnUnionBoundToDeckArgs)`; key `:1003`; args `{ binding: String, position }` `:2906`). Compiled `compiled.rs:2138`; compile `compile.rs:3472`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs:649` -> `return_hand_card_to_deck` (`effect_context/action/zones.rs:403`), `return_trash_cards_to_deck_top` / `_bottom` (`zones.rs:339` / `:308`), or `return_card_source_to_deck` (`effect_context/action/sources.rs:1032`).
  ```rust
  // zone_moves.rs:649
  let to_bottom = super::map_stack_position(*position) == StackPosition::Bottom;
  if let Some((card, origin, owner)) = bindings.get_union_card(binding) {
      let moved = match origin {
          UnionZoneOrigin::Hand => ctx.return_hand_card_to_deck(owner, card, to_bottom),
          UnionZoneOrigin::Trash => !(if to_bottom { ctx.return_trash_cards_to_deck_bottom(owner, &[card]) }
                                      else { ctx.return_trash_cards_to_deck_top(owner, &[card]) }).is_empty(),
          UnionZoneOrigin::Material { carrier, .. } => ctx.return_card_source_to_deck(carrier, card, to_bottom),
      };
      if moved { bindings.record_returned_to_deck(card); }
  }
  ```
- **YAML shape:** consumes a `select_union_zone` binding (by NAME, not a binding ref). From BT26-009:
  ```yaml
  - select_union_zone:
      of: you
      zones: [hand]
      bind_as: bottomed
      filter: {}
      prompt: "Return 1 card in your hand to the bottom of the deck"
  - return_union_bound_to_deck: { binding: bottomed, position: bottom }
  ```
- **Semantics:** Moves the ONE card bound by `select_union_zone` from its true origin zone (hand / trash / a
  permanent's digivolution cards) to the top or bottom of its owner's deck (Digi-Egg -> Digi-Egg deck via
  `move_card_to_deck`). A return, not a trash: no `OnDiscardHand` / `OnDigivolutionCardTrashed`; a bottom return from a
  stack fires `OnDigivolutionCardReturnedToDeckBottom`; a stack return honours `ImmuneFromStackReturnToLibrary`
  (`stack_return_blocked`). Binding absent / not a union card / card no longer in its origin = no-op. On success the
  card is recorded (`effect_returned_any_card`). One card per call: an "N cards" cost is N sequential
  `select_union_zone` + return pairs, the pick order being the deck order (the last card returned to `top` ends on
  top). `position: random` behaves as top; `choice` panics. Never prompts itself.
- **Rule basis:**
  - §3-1-3-8: returned to its owner's deck.
  - §3-2-2 / §3-2-3: deck order is fixed once placed (pick order = final order).
  - §3-1-3-4: the causing player chooses the order of several cards returned together (modelled as sequential picks).
  - §15-7-1 / §15-7-3: "By returning 3 ..." is an optional processing condition that can't be partly paid (EX7-043).
- **Production users:**
  - BT26-009 Hyokomon (top-10: Chronomon): inherited "[When Attacking] [Once Per Turn] <Draw 1>. Then, if your hand has 6 or more cards, return 1 card in your hand to the bottom of the deck." — YAML above.
  - EX7-043 Tankmon: "[On Play] [When Digivolving] By returning 3 cards with the [Three Musketeers] trait from your hand or trash to the top of the deck, <De-Digivolve 1> 1 of your opponent's Digimon." —
    ```yaml
    - select_union_zone: { of: you, zones: [hand, trash], bind_as: return_1, optional: true, cost: true,
                           filter: { trait_has: "Three Musketeers" }, prompt: "... (1/3)" }
    - return_union_bound_to_deck: { binding: return_1, position: top }
    # ... return_2, return_3 likewise
    ```
- **Existing near-miss tests:** coverage index lists only `src/dsl_cards/step/zone_moves.rs`. Card tests `cards_behavioral/ex7/ex7_043.rs`, `bt26/bt26_009.rs`, `ex13/ex13_053.rs` (never count).
- **Proposed behavioural test (for the plan):** `tests/dsl/return_union_bound_to_deck.rs`.
  - fixture:
    ```yaml
    card: TST-RUBD
    name: Union Return Probe
    kind: digimon
    level: 4
    color: [blue]
    cost: 3
    dp: 4000
    effects:
      - when: when_digivolving
        process:
          - select_union_zone:
              of: you
              zones: [hand, trash]
              bind_as: picked
              optional: true
              filter: { name_is: "Probe" }
              prompt: "Return 1 Probe card from your hand or trash to the deck"
          - return_union_bound_to_deck: { binding: picked, position: top }
          - if:
              condition: { effect_returned_any_card: true }
              then:
                - gain_memory: 1
    ```
    (`format!` variant `position: bottom`).
  - setup: `PH` (name "Probe") in hand, `PT` (name "Probe") via `inject_trash`, `D1` in deck; `.memory(0)`.
  - positive (hand, top): the prompt offers `PLAY_HAND_START + idx(PH)` and `TRASH_EFFECT_START + idx(PT)`; pick PH -> `players[0].deck.last()` is PH, hand -1, trash unchanged, memory +1.
  - positive (trash, bottom): pick PT -> `players[0].deck[0]` is PT, trash -1, memory +1.
  - sequence: two consecutive select+return pairs to `top` -> the second-picked card is on top (`deck.last()`), the first just under it.
  - negative: PASS on the optional select -> deck unchanged, memory unchanged. (Check whether PASS also drops the rest of the clause — same `ResumeDecline` question as `select_security`; assert only "nothing moved".)
  - observer check: an `on_discard_hand` observer must NOT fire (a return is not a discard).
  - selection surface: the only prompt is `select_union_zone` (`is_optional` per YAML); the verb itself never prompts.
- **Suspected engine/DSL defects:** A printed "return N cards" (EX7-043) is N separate single-card moves, not one simultaneous move (§3-1-3-3), so a "when cards are returned/added to the deck" observer can see N events instead of one (§15-5-2). `position: random` silently means top. Otherwise none seen.

---

### `verb:move_self_option_under_permanent`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:380` (`MoveSelfOptionUnderPermanent(MoveSelfOptionUnderPermanentArgs)`; key `:1028`; args `{ target: BindingRef, face_down: bool = true }` `:1666`). Compiled `compiled.rs:1966`; compile `compile.rs:3262`.
  - Runtime: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs:369` -> `EffectContext::move_self_option_under_permanent` `code/digimon-engine/src/effect_context/action/lifecycle.rs:251` -> `Game::move_field_option_under_permanent` `code/digimon-engine/src/option_lifecycle.rs:379` -> `seat_card_source_under_permanent` `option_lifecycle.rs:457`.
  ```rust
  // zone_moves.rs:369
  if let Some(ResolvedBinding::Permanent(target_handle)) = resolve_binding_ref(target, ctx, bindings) {
      let _ = ctx.move_self_option_under_permanent(target_handle, *face_down);   // result discarded
  }
  // lifecycle.rs:251
  let Some(option_handle) = self.source_permanent else { return false; };
  if target.player != self.player { return false; }
  // option_lifecycle.rs:387-437
  if self.option_field_state(option_handle).is_none() { return false; }
  let permanent = self.player_mut(owner).battle_area.remove(option_handle.index as usize); // indices shift!
  let Some(top) = cards.pop() else { return false; };
  self.seat_card_source_under_permanent(top, target, face_down, cause)
  // option_lifecycle.rs:466  seat_card_source_under_permanent re-reads `battle_area.get(target.index)`
  //   (the PRE-removal index) to find target_top; on a miss -> card to owner's trash, return false
  ```
- **YAML shape:** from ST24-15:
  ```yaml
  - select_own_permanent:
      bind_as: ds_tamer
      optional: true
      filter: { all_of: [ { kind: tamer }, { trait_has: "DATA SQUAD" } ] }
      prompt: "Place this card face down under a [DATA SQUAD] Tamer to draw 1 and gain 1 memory"
  - if:
      condition: { binding_present: ds_tamer }
      then:
        - move_self_option_under_permanent: { target: ds_tamer, face_down: true }
        - draw: { of: you, count: 1 }
        - gain_memory: 1
  ```
- **Semantics:** Relocates THIS effect's source Option — which must be a live field Option permanent
  (`source_permanent` with an `option_field_state`: Delay / ordinary field Option / orphaned plug-in / Training) — out
  of the battle area and seats it as the BOTTOM digivolution card of the bound OWN permanent, face-down by default. A
  move, not a trash: no `OnOptionTrashed`, no `OnDigivolutionCardTrashed`; the receiving host gets an effect-driven
  `OnAddDigivolutionCards` note. No-op (false) when the binding is unset / not a permanent, the source isn't a field
  Option, or the target belongs to the other player. If the target can't be resolved after the Option is removed, or is
  a token, the Option goes to its owner's TRASH and the call returns false. The step discards the result; the YAML
  gates the follow-up on `binding_present`, not on success. The verb surfaces no selection.
- **Rule basis:**
  - §4-28-1: an "Option card in the battle area" is one placed there by an effect (the only legal source).
  - §4-7-5 / §4-7-9 / §4-7-10: stacked cards are face-up unless specified; a face-down one has no referable information (owner may look).
  - §4-21-3: cards can't be stacked with tokens (token target -> fails).
  - §15-7-1 / §15-7-2: "By placing this card ... under ..., <Draw 1> and gain 1 memory" — the draw/memory require the placement.
- **Production users:**
  - ST24-15 DNA Charge (top-10: DATA SQUAD Ravemon): "[Start of Your Main Phase] By placing this card from the battle area face down under any of your [DATA SQUAD] trait Tamers, <Draw 1> and gain 1 memory." — YAML above.
  - ST23-15 e-Pulse: same text with [BEATBREAK] — same YAML shape (`bb_tamer`).
- **Existing near-miss tests:**
  - `tests/dsl/option_lifecycle_cluster.rs:12` — mentions the verb only in a doc comment (its tests cover `place_self_under_permanent`).
  - `cards_behavioral/st23/st23_15.rs` / `st24/st24_15.rs` — card tests; `setup_field_option` places the Option LAST "so its index is known", which hides the index-shift defect below.
  - `src/dsl_cards/step/zone_moves.rs` — source.
- **Proposed behavioural test (for the plan):** `tests/dsl/move_self_option_under_permanent.rs`.
  - fixture:
    ```yaml
    card: TST-MSO
    name: Field Option Relocate Probe
    kind: option
    color: [white]
    cost: 0
    effects:
      - when: start_of_your_main_phase
        process:
          - select_own_permanent:
              bind_as: host
              optional: true
              filter: { kind: tamer }
              prompt: "Place this card face down under a Tamer"
          - if:
              condition: { binding_present: host }
              then:
                - move_self_option_under_permanent: { target: host, face_down: true }
                - gain_memory: 1
    ```
    (`format!` variant `face_down: false`).
  - setup (mirrors `st23_15.rs::setup_field_option`): a Tamer `TAM` (`card_kind = Tamer`), deck fillers, `.memory(0)`; `place_on_field(0, "TAM", Some(0))`; `let opt = place_on_field(0, "TST-MSO", Some(0)); runner.game.player_mut(0).battle_area[opt.index as usize].option_state = OptionState::OrdinaryFieldOption;` then `enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::PlayerBattleArea(0))` + drain.
  - positive: prompt `is_optional == true`, one valid id (the Tamer); execute it -> `battle_area_size(0) == 1`; the Tamer's `card_sources[0]` is `TST-MSO` with `face_down == true`; P0 trash does not contain `TST-MSO`; memory +1. Face-up variant: `face_down == false`.
  - **index-shift case (expected RED today):** place the Option FIRST (index 0) and the Tamer second (index 1), then run the same flow -> the rule-correct expectation is identical to the positive case (Option under the Tamer, not in trash). Add a third own permanent after the Tamer to show the wrong-host variant.
  - negative 1: PASS -> Option still on the field, memory unchanged.
  - negative 2: the Option is NOT a field Option (leave `option_state` at a non-field state such as `Linked`, or run with no `source_permanent`) -> no move, Option not trashed.
  - selection surface: only `select_own_permanent` prompts; no auto-pick.
- **Suspected engine/DSL defects:**
  - **Index shift:** `move_field_option_under_permanent` removes the Option from `battle_area` and then `seat_card_source_under_permanent` looks the target up by its OLD `target.index` (`option_lifecycle.rs:466-471`; the pre-removal `target_top` captured at `:407` is discarded at `:418`). When the Option's index is lower than the Tamer's (e.g. the Tamer was played after the Option), the Option is trashed (index now out of range) or seated under the NEXT permanent (possibly a Digimon).
  - The step discards the bool, and ST23-15 / ST24-15 gate the draw + memory on `binding_present`, so they still draw and gain memory when the relocate failed (§15-7-2).

---

### `verb:delete_permanents`

- **Lowering site(s):**
  - Parse: `code/digimon-dsl/src/step.rs:163` (`DeletePermanents(DeletePermanentsArgs)`; key `:929`; args `{ targets: Vec<BindingRef> }` `:1362`). Compiled `compiled.rs:1788`; compile `compile.rs:3041`.
  - Runtime (live path): `code/digimon-engine/src/dsl_cards/step/deletion_results.rs:7` `expand` (dispatcher `step/mod.rs:741` / `:883`) -> `DeleteCapturedBatch` / `DeleteReceiptFinalize` (`deletion_results.rs:33-47`) -> `Game::delete_permanents_batch_with_receipt` -> `combat/deletion.rs` (`enqueue_batch_on_any_deletion` + `collapse_simultaneous_deletion_event_batch`, `deletion.rs:921-989`). The arm at `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs:174` is the pre-expansion fallback (not reached once `expand` returns `Some`).
  ```rust
  // deletion_results.rs:19 (expand)
  CompiledStep::DeletePermanents { targets } => targets.iter().filter_map(|t| match resolve_binding_ref(t, ctx, bindings)
      { Some(ResolvedBinding::Permanent(h)) => Some(h), _ => None }).collect(),
  // ...
  targets.sort_by_key(|h| (h.player, h.index)); targets.dedup();
  targets.retain(|h| ctx.game.player(h.player).battle_area.get(h.index as usize).is_some()
                      && ctx.can_affect_permanent(*h));
  if targets.is_empty() { return Some(Vec::new()); }
  let captured = targets.into_iter().map(|h| (h.player, ctx.game.capture_digivolve_host_identity(h))).collect();
  Some(vec![DeleteCapturedBatch { receipt, targets: captured }, DeleteReceiptFinalize { receipt }])
  // DeleteCapturedBatch: one delete_permanents_batch_with_receipt(handles, infer_deletion_cause(first), receipt)
  // DeleteReceiptFinalize: bindings.record_deleted(handle, dp, is_digimon) for each ACTUAL deletion
  ```
- **YAML shape:** from EX7-071:
  ```yaml
  - select_opponent_permanent: { bind_as: lv3_target, filter: { kind: digimon, level_eq: 3 }, prompt: "..." }
  - select_opponent_permanent: { bind_as: lv4_target, filter: { kind: digimon, level_eq: 4 }, prompt: "..." }
  - select_opponent_permanent: { bind_as: lv5_target, filter: { kind: digimon, level_eq: 5 }, prompt: "..." }
  - delete_permanents: { targets: [lv3_target, lv4_target, lv5_target] }
  ```
- **Semantics:** Deletes the permanents bound by SEPARATE single-permanent bindings simultaneously, as one batch
  (one deletion replacement window over the batch, one receipt). Unset bindings (a select that self-skipped with no
  candidate) are ignored; duplicate handles collapse; permanents that already left, or that this effect can't affect,
  are skipped; nothing left -> no-op. Deletion cause is inferred from the first target. `[On Deletion]` handlers run
  post-trash (CLAUDE.md rule 25) and, inside the resolving effect, wait for it to finish. "When ... is deleted"
  observers (`on_any_deletion`) are collapsed per observer for the batch (§15-5-2), so an observer of the batch fires
  once. Actual deletions (not prevented ones) are recorded for `effect_deleted_*` predicates. The verb never prompts
  (the choices are the preceding selects).
- **Rule basis:**
  - §4-15-1 / §4-15-2: deleted cards are trashed; `[On Deletion]` becomes pending after reaching the trash.
  - §15-4-3-2: effects triggered before a single effect resolves trigger simultaneously.
  - §15-5-2: one trigger condition occurring several times at the same time triggers once.
  - §15-8-3-2: observers wait until the deleting effect finishes.
  - card-text idiom: "Delete 1 of X, 1 of Y, and 1 of Z" (DCGO `EX7_071.cs` accumulates picks then one `DestroyPermanentsClass(list)`).
- **Production users:**
  - EX7-071 Hurricane Screw Shot (top-10: Beelstarmon (3M)): "[Main] Delete 1 of your opponent's level 3, 1 of their level 4, and 1 of their level 5 Digimon. Then, place this card as the bottom digivolution card of 1 of your Digimon with the [Three Musketeers] trait." — YAML above.
  - BT24-075 SkullBaluchimon: "[On Play] [When Digivolving] By trashing 1 card in your hand, delete 1 each of your opponent's level 3 and level 4 Digimon." — `- delete_permanents: { targets: [lv3_target, lv4_target] }` after `select_hand` (cost) + two `select_opponent_permanent`.
- **Existing near-miss tests:** coverage index lists only source (`src/dsl_cards/step/deletion_results.rs`, `permanent_mutations.rs`). Card tests `cards_behavioral/ex7/ex7_071.rs`, `bt24/bt24_075.rs` (never count). The observer-collapse mechanism is exercised by the `tests/rules_conformance/fixtures/ch15_04/OBS_OPP_DEL_MEM.yaml` scenarios, but not through this verb.
- **Proposed behavioural test (for the plan):** `tests/dsl/delete_permanents_batch.rs`.
  - fixtures:
    ```yaml
    card: TST-DELS
    name: Batch Delete Probe
    kind: digimon
    level: 5
    color: [purple]
    cost: 5
    dp: 5000
    effects:
      - when: when_digivolving
        process:
          - select_opponent_permanent:
              bind_as: a
              filter: { kind: digimon, level_eq: 3 }
              prompt: "Delete 1 of your opponent's level 3 Digimon"
          - select_opponent_permanent:
              bind_as: b
              filter: { kind: digimon, level_eq: 4 }
              prompt: "Delete 1 of your opponent's level 4 Digimon"
          - delete_permanents: { targets: [a, b] }
    ```
    ```yaml
    card: TST-DEL-OBS
    name: Deletion Witness
    kind: tamer
    color: [red]
    cost: 3
    effects:
      - when: on_any_deletion
        trigger_if:
          all_of:
            - event_target_owner: you
            - event_target_kind: digimon
        process:
          - gain_memory: 1
    ```
    Control fixture `TST-DEL-SEQ` = `TST-DELS` with the last step replaced by `- delete_permanent: { target: a }` and `- delete_permanent: { target: b }`.
  - setup: `L3` (level 3) and `L4` (level 4) Digimon on P1's field, `TST-DEL-OBS` on P1's field, `.memory(0)`; fixture on P0; enqueue `WhenDigivolving`; drain.
  - positive: first prompt = level-3 pick (1 id, `is_optional == false`); after answering it `battle_area_size(1)` is still 3 (nothing deleted yet); second prompt = level-4 pick; after answering, both `L3` and `L4` are in P1's trash, `TST-DEL-OBS` still on field; the witness fired ONCE -> memory == -1.
  - contrast (same setup, `TST-DEL-SEQ`): both deleted, witness fired TWICE -> memory == -2 (two events). This distinguishes the batch from sequential deletion.
  - negative 1 (unset binding): no level-4 Digimon on P1's field -> the second select self-skips, `L3` alone is deleted, no panic.
  - negative 2 (duplicates): `targets: [a, a]` -> `L3` deleted once (P1 trash +1).
  - negative 3 (immunity): make `L4` unaffectable by the opponent's effects -> only `L3` deleted.
  - selection surface: assert the verb adds no prompt after the last select (pending selection is None after the second answer, unless an observer prompts).
- **Suspected engine/DSL defects:** none seen. (Dead-code note: the `permanent_mutations.rs:174` arm duplicates the logic without a receipt and is unreachable while `deletion_results::expand` handles the variant; keep them in sync or remove one.)
