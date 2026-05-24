//! Source-scoped movement immunity for "by your opponent's effects" text.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SecurityPhase, SecurityResolutionState};

fn active_security_resolution(defender: u8) -> SecurityResolutionState {
    SecurityResolutionState {
        attacker: None,
        defender,
        turn_player: 1 - defender,
        revealed_card: digimon_engine::card_source::CardHandle(999),
        card_kind: CardKind::Option,
        was_face_up: false,
        phase: SecurityPhase::SecuritySkillDrain,
        phase_enqueue_done: false,
        // Renamed by `fix-security-check-recompute-mid-attack`: countdown
        // -> performed counter. `0` here means "no checks performed yet".
        checks_performed: 0,
        outcome_so_far: digimon_engine::combat::AttackResult::SecurityCheckSurvived,
    }
}

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

fn protected_handle() -> PermanentHandle {
    PermanentHandle {
        player: 0,
        index: 0,
    }
}

#[test]
fn opponent_effect_cannot_return_protected_digimon_to_hand() {
    let mut r = DebugRunner::builder().add_card(card("EX8-070")).start();
    let target = r.place_on_field(0, "EX8-070", Some(0));
    assert_eq!(target, protected_handle());
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToHand,
            Expiry::Permanent,
            0,
        ),
    );

    let moved = r.game.return_to_hand_from_effect(target, 1);

    assert!(!moved, "opponent effect should be blocked");
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert!(r.game.player(0).hand.is_empty());
}

#[test]
fn own_effect_can_return_protected_digimon_to_hand() {
    let mut r = DebugRunner::builder().add_card(card("EX8-070")).start();
    let target = r.place_on_field(0, "EX8-070", Some(0));
    assert_eq!(target, protected_handle());
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToHand,
            Expiry::Permanent,
            0,
        ),
    );

    let moved = r.game.return_to_hand_from_effect(target, 0);

    assert!(
        moved,
        "own effect should not be blocked by opponent-only immunity"
    );
    assert!(r.game.player(0).battle_area.is_empty());
    assert_eq!(r.game.player(0).hand.len(), 1);
}

#[test]
fn opponent_effect_cannot_return_to_deck_or_de_digivolve_protected_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(card("BT18-064"))
        .add_card(card("BT17-064"))
        .start();
    let target = r.place_stack(0, &["BT17-064", "BT18-064"]);
    assert_eq!(target, protected_handle());
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToDeck,
            Expiry::Permanent,
            0,
        ),
    );
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDeDigivolved,
            Expiry::Permanent,
            0,
        ),
    );

    assert!(!r.game.return_to_deck_from_effect(target, 1));
    assert!(!r.game.de_digivolve_from_effect(target, 1, 1));
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 2);
}

#[test]
fn effect_context_grants_zone_return_immunity_to_opponent_effects() {
    let mut r = DebugRunner::builder()
        .add_card(card("P-215"))
        .add_card(card("BT17-064"))
        .start();
    let target = r.place_stack(0, &["BT17-064", "P-215"]);
    assert_eq!(target, protected_handle());

    {
        let mut ctx = EffectContext::new(
            &mut r.game,
            digimon_engine::card_source::CardHandle(0),
            None,
            0,
        );
        ctx.grant_zone_return_immunity_to_opponent_effects(target, Expiry::Permanent);
    }

    assert!(r.game.modifiers.blocks_opponent_effect(
        target,
        ModifierType::CannotBeReturnedToHand,
        1
    ));
    assert!(!r.game.modifiers.blocks_opponent_effect(
        target,
        ModifierType::CannotBeReturnedToHand,
        0
    ));
    assert!(!r.game.return_to_deck_from_effect(target, 1));
    assert!(!r.game.de_digivolve_from_effect(target, 1, 1));
    assert!(r.game.return_to_hand_from_effect(target, 0));
    assert_eq!(r.game.player(0).hand.len(), 1);
}

#[test]
fn opponent_security_effect_cannot_return_or_de_digivolve_protected_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(card("P-215"))
        .add_card(card("BT17-064"))
        .start();
    let target = r.place_stack(0, &["BT17-064", "P-215"]);
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToHand,
            Expiry::Permanent,
            0,
        ),
    );
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeDeDigivolved,
            Expiry::Permanent,
            0,
        ),
    );
    r.game.security_resolution = Some(active_security_resolution(1));

    assert!(
        !r.game.return_to_hand_from_effect(target, 1),
        "opponent-controlled security effect should still be OpponentEffect"
    );
    assert!(
        !r.game.de_digivolve_from_effect(target, 1, 1),
        "opponent-controlled security effect should not become SecurityCheck"
    );
    assert_eq!(r.game.player(0).battle_area.len(), 1);
    assert_eq!(r.game.player(0).battle_area[0].card_sources.len(), 2);
    assert!(r.game.player(0).hand.is_empty());
}

#[test]
fn security_resolution_without_effect_source_does_not_trigger_opponent_only_immunity() {
    let mut r = DebugRunner::builder().add_card(card("BT18-064")).start();
    let target = r.place_on_field(0, "BT18-064", Some(0));
    r.game.modifiers.add(
        target,
        ModifierEntry::passive_replacement(
            ModifierType::CannotBeReturnedToHand,
            Expiry::Permanent,
            0,
        ),
    );
    r.game.security_resolution = Some(active_security_resolution(1));

    assert!(
        r.game.return_to_hand(target).is_some(),
        "rule-level security resolution should remain SecurityCheck, not OpponentEffect"
    );
    assert!(r.game.player(0).battle_area.is_empty());
    assert_eq!(r.game.player(0).hand.len(), 1);
}
