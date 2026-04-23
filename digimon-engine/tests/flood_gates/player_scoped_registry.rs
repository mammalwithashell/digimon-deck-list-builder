use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn player_has_returns_false_when_no_modifier_installed() {
    let r = DebugRunner::builder().start();
    assert!(!r.game.modifiers.player_has(0, ModifierType::CannotGainMemoryByEffect));
    assert!(!r.game.modifiers.player_has(1, ModifierType::CannotGainMemoryByEffect));
}

#[test]
fn player_has_returns_true_after_install() {
    let mut r = DebugRunner::builder().start();
    r.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotGainMemoryByEffect, 0, Expiry::Permanent, None, 0));
    assert!(r.game.modifiers.player_has(1, ModifierType::CannotGainMemoryByEffect));
    assert!(!r.game.modifiers.player_has(0, ModifierType::CannotGainMemoryByEffect));
}

#[test]
fn end_of_turn_expiry_removes_player_modifier() {
    let mut r = DebugRunner::builder().start();
    let tp = r.game.turn_player();
    r.game.modifiers.add_player_modifier(1 - tp, PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::EndOfTurn, None, tp));
    assert!(r.game.modifiers.player_has(1 - tp, ModifierType::CannotPlayDigimonByEffect));
    r.game.modifiers.expire_player_end_of_turn(tp);
    assert!(!r.game.modifiers.player_has(1 - tp, ModifierType::CannotPlayDigimonByEffect));
}

#[test]
fn until_leave_field_expiry_removes_modifier_when_source_permanent_leaves() {
    let mut r = DebugRunner::builder().start();
    // Use a synthetic PermanentHandle — the expiry mechanism only needs the handle value.
    let src = PermanentHandle { player: 0, index: 0 };
    r.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotActivateMainEffects, 0, Expiry::UntilLeaveField, Some(src), 0));
    assert!(r.game.modifiers.player_has(1, ModifierType::CannotActivateMainEffects));
    r.game.modifiers.expire_player_on_permanent_leave(src);
    assert!(!r.game.modifiers.player_has(1, ModifierType::CannotActivateMainEffects));
}

#[test]
fn end_of_opponents_turn_expiry_discriminates_by_source_player() {
    let mut r = DebugRunner::builder().start();
    // Player 0 installs modifier on player 1 with EndOfOpponentsTurn.
    // "Opponent" from player 0's perspective is player 1.
    r.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotReducePlayCost, 0, Expiry::EndOfOpponentsTurn, None, 0));
    // Ending player 0's own turn should NOT expire it:
    r.game.modifiers.expire_player_end_of_turn(0);
    assert!(r.game.modifiers.player_has(1, ModifierType::CannotReducePlayCost),
        "modifier should persist when the installer's own turn ends");
    // Ending player 1's turn (the opponent of the installer) SHOULD expire it:
    r.game.modifiers.expire_player_end_of_turn(1);
    assert!(!r.game.modifiers.player_has(1, ModifierType::CannotReducePlayCost),
        "modifier should expire at end of opponent's turn");
}

#[test]
fn multiple_player_modifiers_coexist() {
    let mut r = DebugRunner::builder().start();
    r.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 0));
    r.game.modifiers.add_player_modifier(1, PlayerModifierEntry::simple(ModifierType::CannotGainMemoryByEffect, 0, Expiry::Permanent, None, 0));
    assert!(r.game.modifiers.player_has(1, ModifierType::CannotPlayDigimonByEffect));
    assert!(r.game.modifiers.player_has(1, ModifierType::CannotGainMemoryByEffect));
    // Both modifiers coexist:
    let count = r.game.modifiers.player_modifiers_iter(1).count();
    assert_eq!(count, 2);
}
