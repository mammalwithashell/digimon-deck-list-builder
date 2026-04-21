//! Phase 7 Task 5 — passive restriction modifiers auto-install as
//! cancel-replacements under the Task 2 dispatcher.
//!
//! Covers:
//!   - `CannotBeReturnedToDeck` / `…Hand` (Phase 6 movement protection)
//!   - `CannotBeDeDigivolved` (Phase 6 de-digi protection)
//!   - `CannotBeDestroyed*` family (Phase 0 destruction protection, now
//!     routed through the replacement framework).
//!   - Cause-filter gating (opponent-only vs cause-agnostic vs battle-only).
//!   - Player-scoped passive modifier path.
//!   - `replacement_condition` closure gating.

use std::sync::{Arc, Mutex};

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, ModifierType, StackPosition};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

/// Trigger card — its `OnPlay` process invokes a user-supplied closure
/// against the game mutably. Used in tests to stage an opponent-effect
/// action: place this card on P0, push a closure into the slot, then call
/// `r.fire_on_play(0, ...)` — the closure runs while `effect_source_player`
/// is P0, so operations targeting P1 are inferred as `OpponentEffect`.
///
/// The slot is re-armable: each `fire_on_play` call consumes the current
/// `Some(closure)` via `.take()`, leaving the slot empty until the next
/// `*slot.lock() = Some(...)`.
struct OnPlayClosure(
    Arc<Mutex<Option<Box<dyn FnMut(&mut digimon_engine::game::Game) + Send + Sync>>>>,
);
impl CardEffect for OnPlayClosure {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnPlayClosure trigger")
            .process(move |ctx| {
                if let Some(mut f) = slot.lock().unwrap().take() {
                    f(&mut *ctx.game);
                }
            })
            .build()]
    }
}

fn new_end_of_turn_slot()
-> Arc<Mutex<Option<Box<dyn FnMut(&mut digimon_engine::game::Game) + Send + Sync>>>> {
    Arc::new(Mutex::new(None))
}

// ─── Tests ─────────────────────────────────────────────────────────────

/// Test 1: `CannotBeReturnedToDeck` with default cause_filter = OpponentEffect
/// cancels a return triggered by the opponent's effect.
#[test]
fn cannot_be_returned_to_deck_cancels_opponent_return() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("VICTIM"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    // VICTIM lives on P1, with CannotBeReturnedToDeck installed (opp-only).
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToDeck,
            Expiry::Permanent,
            1,
        ),
    );

    // TRIGGER lives on P0; its OnPlay process (running as P0's effect)
    // calls return_to_deck(victim) — cause inferred as OpponentEffect.
    let trigger = r.place_on_field(0, "TRIGGER", Some(0));
    let deck_before = r.game.player(1).deck.len();
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        game.return_to_deck(victim, StackPosition::Bottom);
    }));

    r.game.fire_on_play(trigger.player, trigger.index as usize);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "victim should still be on P1's field (opponent return cancelled)"
    );
    assert_eq!(
        r.game.player(1).deck.len(),
        deck_before,
        "deck size unchanged (return cancelled)"
    );
}

/// Test 2: `CannotBeReturnedToDeck` with default cause_filter = OpponentEffect
/// allows an own-effect return to proceed.
#[test]
fn cannot_be_returned_to_deck_allows_own_return() {
    let mut r = DebugRunner::builder().add_card(card("VICTIM")).start();

    // VICTIM on P0 with CannotBeReturnedToDeck (defaults to OpponentEffect).
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToDeck,
            Expiry::Permanent,
            0,
        ),
    );

    // Direct test-level call → cause inferred as OwnEffect (no
    // effect_source_player, no security_resolution). OwnEffect ≠
    // OpponentEffect so the modifier does NOT fire.
    let deck_before = r.game.player(0).deck.len();
    let ok = r.game.return_to_deck(victim, StackPosition::Bottom);

    assert!(ok, "own-effect return should succeed (modifier doesn't gate it)");
    assert_eq!(r.battle_area_size(0), 0, "victim left the field");
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "deck grew by 1"
    );
}

/// Test 3: `CannotBeDestroyedByEffect` (cause_filter = None, cause-agnostic)
/// cancels an opponent-effect deletion.
///
/// Note: swapped in for the original "CannotBeTrashedByEffect from hand"
/// test — per the plan's simplification, that subject model (Card(_, Hand))
/// is not supported by passive modifiers in v1 (no per-hand-card modifier
/// store). The field-permanent case is canonical and covers the same scope.
#[test]
fn cannot_be_destroyed_by_effect_cancels_opponent_deletion() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("VICTIM"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    let victim = r.place_on_field(1, "VICTIM", Some(0));
    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDestroyedByEffect,
            Expiry::Permanent,
            1,
        ),
    );

    let trigger = r.place_on_field(0, "TRIGGER", Some(0));
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        game.delete_permanent_with_effects(victim);
    }));

    r.game.fire_on_play(trigger.player, trigger.index as usize);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "victim survives — CannotBeDestroyedByEffect cancels effect deletion"
    );
}

/// Test 4: `CannotBeDeDigivolved` cancels a de-digivolve on the target
/// permanent via opponent's effect.
#[test]
fn cannot_be_de_digivolved_cancels_de_digi() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("VICTIM"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    // Build a victim with a digivolution stack (level 3 on top, level 2
    // underneath) so de_digivolve has something to pop if it runs.
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    // Push a second CardSource under the top to simulate a digi-stack.
    {
        let next_idx = r.game.next_card_index();
        let mut filler = digimon_engine::card_source::CardSource::new(
            0, /*data_index placeholder*/ 1, next_idx,
        );
        filler.card_index = next_idx;
        let perm = &mut r.game.players[1].battle_area[victim.index as usize];
        // Insert BELOW the current top — the top should be at the end.
        perm.card_sources.insert(0, filler);
    }
    let stack_before = r.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(stack_before, 2);

    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDeDigivolved,
            Expiry::Permanent,
            1,
        ),
    );

    let trigger = r.place_on_field(0, "TRIGGER", Some(0));
    *slot.lock().unwrap() = Some(Box::new(move |ctx_game| {
        // Invoke de_digivolve via an ephemeral EffectContext. We're inside
        // the OnPlay process, so `effect_source_player` is already set to
        // P0 via the effect queue — an EffectContext built on top of the
        // same Game will inherit that cause-inference state.
        use digimon_engine::effect_context::EffectContext;
        let mut ec = EffectContext::new(ctx_game, CardHandle(0), None, 0);
        let popped = ec.de_digivolve(victim, Some(3), Some(1));
        assert_eq!(popped, 0, "de-digi cancelled by CannotBeDeDigivolved");
    }));

    r.game.fire_on_play(trigger.player, trigger.index as usize);

    let stack_after = r.game.players[1].battle_area[victim.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after, stack_before,
        "digi-stack unchanged — de-digi was cancelled"
    );
}

/// Test 5: `CannotBeDestroyedByBattle` (cause_filter = Battle) cancels a
/// deletion that comes from `resolve_battle`.
#[test]
fn cannot_be_destroyed_by_battle_cancels_battle_deletion() {
    let mut attacker_card = card("BIG_ATTACKER");
    attacker_card.dp = Some(10000);
    let mut defender_card = card("WEAK_DEFENDER");
    defender_card.dp = Some(1000);

    let mut r = DebugRunner::builder()
        .add_card(attacker_card)
        .add_card(defender_card)
        .start();

    let attacker = r.place_on_field(0, "BIG_ATTACKER", Some(0));
    let defender = r.place_on_field(1, "WEAK_DEFENDER", Some(0));

    // Install CannotBeDestroyedByBattle on defender.
    r.game.modifiers.add(
        defender,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDestroyedByBattle,
            Expiry::Permanent,
            1,
        ),
    );

    let _ = r.attack_digimon(attacker, defender, false);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "defender survives battle — CannotBeDestroyedByBattle cancelled the deletion"
    );
}

/// Test 6: `CannotBeDestroyedByBattle` does NOT cancel effect deletions.
#[test]
fn cannot_be_destroyed_by_battle_does_not_cancel_effect_deletion() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("VICTIM"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    let victim = r.place_on_field(1, "VICTIM", Some(0));
    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDestroyedByBattle,
            Expiry::Permanent,
            1,
        ),
    );
    // Hardcode defense: cause_filter should be Battle.
    let trigger = r.place_on_field(0, "TRIGGER", Some(0));
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        game.delete_permanent_with_effects(victim);
    }));

    r.game.fire_on_play(trigger.player, trigger.index as usize);

    assert_eq!(
        r.battle_area_size(1),
        0,
        "victim deleted — CannotBeDestroyedByBattle doesn't gate effect deletions"
    );
}

/// Test 7: player-scoped passive modifier also auto-installs as a
/// replacement. Installing `CannotBeReturnedToHand` on P1 blocks P0's
/// effect from returning P1's permanents — but leaves P0's own permanents
/// alone.
#[test]
fn player_scoped_passive_also_auto_installs_as_replacement() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("P1_VICTIM"))
        .add_card(card("P0_OWN"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    let p1_victim = r.place_on_field(1, "P1_VICTIM", Some(0));
    let p0_own = r.place_on_field(0, "P0_OWN", Some(0));

    // Player-scoped CannotBeReturnedToHand on P1 (opp-only by default).
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToHand,
            Expiry::Permanent,
            None,
            1,
        ),
    );

    let trigger = r.place_on_field(0, "TRIGGER", Some(0));

    // First: P0-effect returns P1's victim → cancelled by player modifier.
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        let _ = game.return_to_hand(p1_victim);
    }));
    let p1_hand_before = r.game.player(1).hand.len();
    r.game.fire_on_play(trigger.player, trigger.index as usize);
    assert_eq!(
        r.battle_area_size(1),
        1,
        "P1's victim still on field — opponent return cancelled"
    );
    assert_eq!(r.game.player(1).hand.len(), p1_hand_before);

    // Second: P0-effect returns P0's own permanent → NOT cancelled
    // (player modifier is on P1, cause_filter would be OpponentEffect but
    // target_player = 0, so the player_modifiers_iter(0) scan turns up
    // nothing). Expect success.
    let p0_hand_before = r.game.player(0).hand.len();
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        let _ = game.return_to_hand(p0_own);
    }));
    r.game.fire_on_play(trigger.player, trigger.index as usize);
    assert_eq!(
        r.game.player(0).hand.len(),
        p0_hand_before + 1,
        "P0's own permanent returned to hand (only the trigger remains on P0's field)"
    );
}

/// Test 8: `replacement_condition` closure gates a passive modifier.
#[test]
fn replacement_condition_gates_passive() {
    let slot = new_end_of_turn_slot();

    let mut r = DebugRunner::builder()
        .add_card(card("VICTIM"))
        .add_card(card("SPARE"))
        .add_card(card("TRIGGER"))
        .start();
    r.register_effect("TRIGGER", Arc::new(OnPlayClosure(slot.clone())));

    let victim = r.place_on_field(1, "VICTIM", Some(0));
    // Start with just ONE permanent on P1 — condition will return true.
    let cond: digimon_engine::replacement::ReplacementConditionFn =
        Box::new(|read_ctx, subject| {
            use digimon_engine::replacement::ReplacementSubject;
            if let ReplacementSubject::Permanent(h) = subject {
                read_ctx.battle_area(h.player).len() == 1
            } else {
                false
            }
        });
    r.game.modifiers.add(
        victim,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToDeck,
            Expiry::Permanent,
            1,
        )
        .with_condition(cond),
    );

    let trigger = r.place_on_field(0, "TRIGGER", Some(0));

    // Case A: battle_area.len() == 1 → condition true → modifier fires →
    // return cancelled.
    assert_eq!(r.game.player(1).battle_area.len(), 1);
    let deck_before = r.game.player(1).deck.len();
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        game.return_to_deck(victim, StackPosition::Bottom);
    }));
    r.game.fire_on_play(trigger.player, trigger.index as usize);
    assert_eq!(r.battle_area_size(1), 1, "victim stayed (condition true)");
    assert_eq!(r.game.player(1).deck.len(), deck_before);

    // Case B: add a second permanent → battle_area.len() == 2 → condition
    // false → modifier does NOT fire → return succeeds.
    let _spare = r.place_on_field(1, "SPARE", Some(0));
    assert_eq!(r.game.player(1).battle_area.len(), 2);
    let deck_before = r.game.player(1).deck.len();
    *slot.lock().unwrap() = Some(Box::new(move |game| {
        game.return_to_deck(victim, StackPosition::Bottom);
    }));
    r.game.fire_on_play(trigger.player, trigger.index as usize);
    assert_eq!(
        r.game.player(1).deck.len(),
        deck_before + 1,
        "victim returned to deck (condition false)"
    );
}
