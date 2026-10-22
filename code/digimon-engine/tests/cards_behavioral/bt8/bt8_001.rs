//! Gurimon: image + official BT8-001 bundle, inherited mandatory OPT Draw 1 at >=6000 DP.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, GamePhase};

fn runner(dp: i32) -> DebugRunner {
    let mut carrier = make_test_card("TEST-CARRIER", "Carrier");
    carrier.dp = Some(dp);
    let mut security = make_test_card("TEST-SEC", "Security");
    security.card_kind = CardKind::Tamer;
    security.dp = None;
    DebugRunner::builder()
        .dsl_card("BT8-001")
        .expect("BT8-001 must be embedded")
        .add_card(carrier)
        .add_card(security)
        .deck(0, &["TEST-CARRIER"; 10])
        .security(1, &["TEST-SEC"; 5])
        .memory(5)
        .start()
}

#[test]
fn bt8_001_draw_threshold_and_once_per_turn() {
    for (dp, expected) in [(5999, 0), (6000, 1), (7000, 1)] {
        let mut r = runner(dp);
        let h = r.place_stack(0, &["BT8-001", "TEST-CARRIER"]);
        let before = r.hand_size(0);
        attack(&mut r, h);
        assert_eq!(r.hand_size(0), before + expected, "DP {dp}");
        assert!(
            r.game.pending_selection.is_none(),
            "mandatory draw has no decline"
        );
        r.game_mut().players[0].battle_area[h.index as usize].is_suspended = false;
        attack(&mut r, h);
        assert_eq!(r.hand_size(0), before + expected, "OPT at DP {dp}");
    }
}

#[test]
fn bt8_001_other_attack_does_not_draw() {
    let mut r = runner(6000);
    r.place_stack(0, &["BT8-001", "TEST-CARRIER"]);
    let other = r.place_on_field(0, "TEST-CARRIER", Some(0));
    let before = r.hand_size(0);
    attack(&mut r, other);
    assert_eq!(r.hand_size(0), before);
}

#[test]
fn bt8_001_once_per_turn_is_independent_per_carrier() {
    let mut r = runner(6000);
    let first = r.place_stack(0, &["BT8-001", "TEST-CARRIER"]);
    let second = r.place_stack(0, &["BT8-001", "TEST-CARRIER"]);
    let before = r.hand_size(0);
    attack(&mut r, first);
    assert_eq!(r.hand_size(0), before + 1);
    attack(&mut r, second);
    assert_eq!(r.hand_size(0), before + 2);
}

#[test]
fn bt8_001_printed_metadata_and_scope() {
    use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
    let r = runner(6000);
    let card = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "BT8-001")
        .unwrap();
    assert_eq!(card.card_kind, CardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert_eq!(card.card_name, "Gurimon");
    let compiled = r.compiled_card("BT8-001").unwrap();
    assert_eq!(compiled.effects.len(), 1);
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("triggered clause")
    };
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert!(clause.when.contains(&CompiledTiming::WhenAttacking));
    assert!(clause.once_per_turn);
    assert!(!clause.optional);
}

fn attack(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    use digimon_engine::action::{build_action_mask, encode_attack, SECURITY_TARGET};
    r.set_phase(GamePhase::Main);
    let action = encode_attack(h.index as u16, SECURITY_TARGET);
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.game.decode_action(action, 0);
}
