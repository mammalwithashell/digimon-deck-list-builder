//! `CannotUnsuspendInUnsuspendPhase` — printed "… can't unsuspend in their
//! next unsuspend phase" (EX13-041 Groundramon). Unlike `CannotUnsuspend`, it
//! gates ONLY the controller's turn-start bulk unsuspend; effect unsuspends
//! stay legal. DCGO `GainCanNotUnsuspendPlayerEffect(isOnlyActivePhase: true)`.

use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn runner() -> DebugRunner {
    let mut c = make_test_card_with_level("UPL-MON", "Lockmon", 4);
    c.colors = vec![CardColor::Red];
    c.dp = Some(4000);
    let mut r = DebugRunner::builder()
        .add_card(c)
        .add_card(make_test_card_with_level("UPL-PAD", "Pad", 3))
        .deck(0, &["UPL-PAD"; 10])
        .deck(1, &["UPL-PAD"; 10])
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn lock(r: &mut DebugRunner, h: PermanentHandle, modifier: ModifierType) {
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(modifier, 1, Expiry::EndOfOpponentsTurn, 0),
    );
}

#[test]
fn modifier_name_round_trips_through_the_dsl_table() {
    assert_eq!(
        digimon_engine::dsl_cards::modifier_map::lookup_modifier_type(
            "CannotUnsuspendInUnsuspendPhase"
        ),
        Some(ModifierType::CannotUnsuspendInUnsuspendPhase)
    );
}

#[test]
fn blocks_the_next_unsuspend_phase() {
    let mut r = runner();
    assert_eq!(r.turn_player(), 0);
    let mon = r.place_on_field(1, "UPL-MON", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    lock(&mut r, mon, ModifierType::CannotUnsuspendInUnsuspendPhase);
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(suspended(&r, mon), "stays suspended through its unsuspend phase");
}

#[test]
fn effect_unsuspend_is_still_legal() {
    let mut r = runner();
    let mon = r.place_on_field(1, "UPL-MON", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    lock(&mut r, mon, ModifierType::CannotUnsuspendInUnsuspendPhase);
    r.game.unsuspend_with_cause(mon, true);
    assert!(!suspended(&r, mon), "effects may still unsuspend it");
}

#[test]
fn plain_cannot_unsuspend_also_blocks_effects_contrast() {
    let mut r = runner();
    let mon = r.place_on_field(1, "UPL-MON", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    lock(&mut r, mon, ModifierType::CannotUnsuspend);
    r.game.unsuspend_with_cause(mon, true);
    assert!(suspended(&r, mon), "CannotUnsuspend is the broader lock");
}

#[test]
fn lock_only_touches_the_locked_permanent() {
    let mut r = runner();
    let mon = r.place_on_field(1, "UPL-MON", Some(0));
    let other = r.place_on_field(1, "UPL-MON", Some(0));
    for i in 0..2 {
        r.game.players[1].battle_area[i].is_suspended = true;
    }
    lock(&mut r, mon, ModifierType::CannotUnsuspendInUnsuspendPhase);
    r.end_turn();
    assert!(suspended(&r, mon));
    assert!(!suspended(&r, other));
}
