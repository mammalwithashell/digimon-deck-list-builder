//! BT6-022: official observation c2e5311758c452b8dfa7cd44d7d2492d241c63b231d5ca9e990b5629e50e5440.
//! Image cross-check: printed Hybrid / Ten Warriors condition applies to carrier.
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};

fn runner(traits: &[&str], inherited: bool) -> DebugRunner {
    let mut carrier = make_test_card_with_level("TEST-CARRIER", "Carrier", 4);
    carrier.dp = Some(6000);
    carrier.traits = traits.iter().map(|s| s.to_string()).collect();
    let mut r = DebugRunner::builder()
        .dsl_card("BT6-022")
        .expect("BT6-022 production YAML")
        .add_card(carrier)
        .add_card(make_test_card("TEST-FILL", "Filler"))
        .deck(0, &["TEST-FILL"; 20])
        .deck(1, &["TEST-FILL"; 20])
        .security(1, &["TEST-FILL"; 5])
        .memory(3)
        .start();
    if inherited {
        r.place_stack(0, &["BT6-022", "TEST-CARRIER"]);
    } else {
        r.place_on_field(0, "BT6-022", Some(0));
    }
    r
}

fn attack(r: &mut DebugRunner) {
    let handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game_mut().players[0].battle_area[0].is_suspended = false;
    r.game_mut().current_phase = digimon_engine::enums::GamePhase::Main;
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    let action = digimon_engine::action::space::encode_attack(
        0,
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert!(mask[action as usize] > 0.5, "security attack must be legal");
    assert_ne!(
        r.attack_player(handle, 1, false),
        digimon_engine::combat::AttackResult::Invalid
    );
    r.auto_resolve().expect("attack resolves");
}

#[test]
fn bt6_022_hybrid_carrier_gains_memory_only_once_per_turn() {
    let mut r = runner(&["Hybrid"], true);
    let before = r.memory();
    attack(&mut r);
    assert_eq!(r.memory(), before + 1);
    attack(&mut r);
    assert_eq!(r.memory(), before + 1, "second attack cannot reuse OPT");
}

#[test]
fn bt6_022_ten_warriors_carrier_gains_memory() {
    let mut r = runner(&["Ten Warriors"], true);
    let before = r.memory();
    attack(&mut r);
    assert_eq!(r.memory(), before + 1);
}

#[test]
fn bt6_022_plain_carrier_does_not_gain_memory() {
    let mut r = runner(&["Beastkin"], true);
    let before = r.memory();
    attack(&mut r);
    assert_eq!(
        r.memory(),
        before,
        "source Hybrid does not satisfy carrier filter"
    );
}

#[test]
fn bt6_022_top_card_does_not_apply_inherited_effect() {
    let mut r = runner(&["Hybrid"], false);
    let before = r.memory();
    attack(&mut r);
    assert_eq!(r.memory(), before);
}
