//! BT18-094: official observation 3cc3ca248260cf295a06338506c4aae3a05793ea6da2b7db26aa43e9b7d9229d.
//! Image confirms Security is top-scope Security, not an inherited effect.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET, TRASH_EFFECT_START},
};
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase};
use digimon_engine::selection::TriggerSource;

fn base() -> DebugRunnerBuilder {
    let mut hybrid = make_test_card("TEST-HYBRID", "Hybrid");
    hybrid.traits = vec!["Hybrid".into()];
    let mut warrior = make_test_card("TEST-WARRIOR", "Warrior");
    warrior.traits = vec!["Ten Warriors".into()];
    let mut carrier = make_test_card_with_level("TEST-CARRIER", "Carrier", 4);
    carrier.dp = Some(10000);
    let mut security = make_test_card("TEST-SECURITY", "Security");
    security.card_kind = CardKind::Tamer;
    security.level = None;
    security.dp = None;
    DebugRunner::builder()
        .dsl_card("BT18-094")
        .expect("BT18-094 production YAML")
        .add_card(hybrid)
        .add_card(warrior)
        .add_card(carrier)
        .add_card(security)
        .add_card(make_test_card("TEST-PLAIN", "Plain"))
        .deck(0, &["TEST-PLAIN"; 20])
        .deck(1, &["TEST-PLAIN"; 20])
        .security(1, &["TEST-SECURITY"; 5])
        .memory(3)
}
fn main_runner(hand: &[&str]) -> DebugRunner {
    let mut r = base().hand(0, hand).start();
    let h = r.place_on_field(0, "BT18-094", Some(0));
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    r
}
fn attack(r: &mut DebugRunner) {
    let h = r.perm_handle(0, 0);
    r.game.players[0].battle_area[0].is_suspended = false;
    r.game.current_phase = GamePhase::Main;
    assert!(build_action_mask(&r.game, 0)[encode_attack(0, SECURITY_TARGET) as usize] > 0.5);
    r.attack_player(h, 1, false);
}
fn inherited_runner() -> DebugRunner {
    let mut r = base().start();
    for id in ["TEST-HYBRID", "TEST-WARRIOR", "TEST-PLAIN"] {
        r.inject_trash(0, id);
    }
    r.place_stack(0, &["BT18-094", "TEST-CARRIER"]);
    r
}
#[test]
fn bt18_094_main_cost_filters_both_traits_and_gains_only_after_trash() {
    for chosen in [0u16, 1] {
        let mut r = main_runner(&["TEST-HYBRID", "TEST-WARRIOR", "TEST-PLAIN"]);
        let mask = build_action_mask(&r.game, 0);
        assert!(mask[PLAY_HAND_START as usize] > 0.5);
        assert!(mask[(PLAY_HAND_START + 1) as usize] > 0.5);
        assert_eq!(mask[(PLAY_HAND_START + 2) as usize], 0.0);
        assert!(mask[PASS as usize] > 0.5);
        assert_eq!(r.memory(), 3);
        assert_eq!(r.game.players[0].trash.len(), 0);
        r.execute_action(0, PLAY_HAND_START + chosen).unwrap();
        assert_eq!(r.memory(), 4);
        assert_eq!(r.game.players[0].hand.len(), 2);
        assert_eq!(r.game.players[0].trash.len(), 1);
    }
}
#[test]
fn bt18_094_main_decline_pays_nothing_and_gains_nothing() {
    let mut r = main_runner(&["TEST-HYBRID"]);
    r.execute_action(0, PASS).unwrap();
    assert_eq!(r.memory(), 3);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert!(r.game.players[0].trash.is_empty());
}
#[test]
fn bt18_094_main_without_eligible_card_does_not_offer_or_gain() {
    let r = main_runner(&["TEST-PLAIN"]);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.memory(), 3);
}
#[test]
fn bt18_094_inherited_recovery_filters_traits_and_is_once_per_turn() {
    let mut r = inherited_runner();
    attack(&mut r);
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[TRASH_EFFECT_START as usize] > 0.5);
    assert!(mask[(TRASH_EFFECT_START + 1) as usize] > 0.5);
    assert_eq!(mask[(TRASH_EFFECT_START + 2) as usize], 0.0);
    assert!(mask[PASS as usize] > 0.5);
    r.execute_action(0, TRASH_EFFECT_START + 1).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "TEST-WARRIOR"
    );
    attack(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand.len(), 1);
}
#[test]
fn bt18_094_inherited_decline_preserves_card_and_opt_for_later_attack() {
    let mut r = inherited_runner();
    attack(&mut r);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].hand.is_empty());
    assert_eq!(r.game.players[0].trash.len(), 3);
    attack(&mut r);
    assert!(r.pending_selection_view().is_some());
    r.execute_action(0, TRASH_EFFECT_START).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand.len(), 1);
}
#[test]
fn bt18_094_inherited_without_matching_trash_does_not_offer() {
    let mut r = base().start();
    r.inject_trash(0, "TEST-PLAIN");
    r.place_stack(0, &["BT18-094", "TEST-CARRIER"]);
    attack(&mut r);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].hand.is_empty());
}
#[test]
fn bt18_094_security_plays_tamer_without_paying_cost() {
    let mut r = base().security(1, &["BT18-094"]).start();
    let h = r.place_on_field(0, "TEST-CARRIER", Some(0));
    let before = r.memory();
    r.attack_player(h, 1, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
    assert!(r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "BT18-094"));
    assert!(r.game.players[1]
        .trash
        .iter()
        .all(|c| c.card_id(&r.game.card_data) != "BT18-094"));
}
