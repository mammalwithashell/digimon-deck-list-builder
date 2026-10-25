//! BT17-020: official observation 9fbc939f23ca8784ce270cf6cf7c9372181b01196d07dd6efab189baa749256d.
//! Q2751/Q2752: take all applicable reveal buckets; Q2753: Security alone is not inherited.
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;

fn tamer(id: &str, inherited: bool, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.security_text = "[Security] Play this card without paying the cost.".into();
    if inherited {
        c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
    }
    c
}

fn reveal_runner(trait_name: &str, with_tamer: bool) -> DebugRunner {
    let mut hybrid = make_test_card("TEST-HYBRID", "Hybrid");
    hybrid.traits = vec![trait_name.into()];
    DebugRunner::builder()
        .dsl_card("BT17-020")
        .expect("BT17-020 YAML loads")
        .add_card(hybrid)
        .add_card(tamer("TEST-TAMER", with_tamer, 5))
        .add_card(make_test_card("TEST-BLANK", "Blank"))
        .deck(0, &["TEST-BLANK", "TEST-TAMER", "TEST-HYBRID"])
        .hand(0, &["BT17-020"])
        .memory(10)
        .start()
}

fn choose(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("player choice");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&id| id != PASS)
        .expect("non-pass action");
    assert!(digimon_engine::action::mask::build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.execute_action(0, action).expect("legal choice resolves");
}

#[test]
fn bt17_020_reveal_requires_both_matching_buckets() {
    for trait_name in ["Hybrid", "Ten Warriors"] {
        let mut r = reveal_runner(trait_name, true);
        r.play(0, 0).expect("play Strabimon");
        for _ in 0..2 {
            let p = r.pending_selection_view().expect("required reveal bucket");
            assert!(!p.valid_action_ids.contains(&PASS));
            choose(&mut r);
        }
        r.auto_resolve().expect("bottom remainder");
        let ids: Vec<_> = r.game.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&r.game.card_data))
            .collect();
        assert!(ids.contains(&"TEST-HYBRID"));
        assert!(ids.contains(&"TEST-TAMER"));
        assert_eq!(
            r.game.players[0].deck[0].card_id(&r.game.card_data),
            "TEST-BLANK"
        );
    }
}

#[test]
fn bt17_020_reveal_security_only_tamer_is_not_inherited() {
    let mut r = reveal_runner("Hybrid", false);
    r.play(0, 0).expect("play Strabimon");
    choose(&mut r);
    r.auto_resolve().expect("bottom nonmatching cards");
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
    assert_eq!(r.game.players[0].deck.len(), 2);
}

fn attack_runner(cost: u16, inherited: bool) -> DebugRunner {
    let mut carrier = make_test_card_with_level("TEST-CARRIER", "Carrier", 4);
    carrier.dp = Some(10000);
    let mut r = DebugRunner::builder()
        .dsl_card("BT17-020")
        .expect("BT17-020 YAML loads")
        .add_card(carrier)
        .add_card(tamer("TEST-TAMER", inherited, cost))
        .add_card(tamer("TEST-OTHER", inherited, cost))
        .add_card(make_test_card("TEST-FILL", "Filler"))
        .deck(0, &["TEST-FILL"; 20])
        .deck(1, &["TEST-FILL"; 20])
        .security(1, &["TEST-FILL"; 5])
        .hand(0, &["TEST-TAMER", "TEST-OTHER"])
        .memory(10)
        .start();
    r.place_stack(0, &["BT17-020", "TEST-CARRIER"]);
    r
}

fn attack(r: &mut DebugRunner) {
    r.game.players[0].battle_area[0].is_suspended = false;
    r.attack_player(
        PermanentHandle {
            player: 0,
            index: 0,
        },
        1,
        false,
    );
}

#[test]
fn bt17_020_inherited_play_pays_reduced_cost_and_is_once_per_turn() {
    let mut r = attack_runner(5, true);
    attack(&mut r);
    assert!(r.pending_is_optional());
    choose(&mut r);
    while r.pending_selection_view().is_some() {
        choose(&mut r);
    }
    r.auto_resolve().expect("complete attack");
    assert_eq!(r.memory(), 7, "printed cost 5 reduced by 2");
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(r.game.players[0].hand.len(), 1);
    attack(&mut r);
    r.auto_resolve().expect("second attack");
    assert_eq!(r.memory(), 7);
    assert_eq!(
        r.game.players[0].hand.len(),
        1,
        "OPT cannot play second Tamer"
    );
}

#[test]
fn bt17_020_inherited_play_cost_floors_at_zero() {
    let mut r = attack_runner(1, true);
    attack(&mut r);
    choose(&mut r);
    while r.pending_selection_view().is_some() {
        choose(&mut r);
    }
    r.auto_resolve().expect("complete attack");
    assert_eq!(r.memory(), 10, "cost reduction cannot grant memory");
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}

#[test]
fn bt17_020_inherited_play_can_be_declined() {
    let mut r = attack_runner(5, true);
    attack(&mut r);
    r.pending_selection_view().expect("optional play");
    assert!(r.pending_is_optional());
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[PASS as usize],
        1.0
    );
    r.execute_action(0, PASS).expect("decline");
    r.auto_resolve().expect("finish attack");
    assert_eq!(r.memory(), 10);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].hand.len(), 2);
    // Q1818: declining the whole-effect may did not activate its OPT.
    attack(&mut r);
    assert!(r.pending_selection_view().is_some());
    choose(&mut r);
    while r.pending_selection_view().is_some() { choose(&mut r); }
    r.auto_resolve().expect("later activation after decline");
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
}

#[test]
fn bt17_020_inherited_play_excludes_security_only_tamers() {
    let mut r = attack_runner(5, false);
    attack(&mut r);
    finish_without_hand_choice(&mut r);
    assert_eq!(r.memory(), 10);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[0].hand.len(), 2);
}

#[test]
fn bt17_020_inherited_play_does_not_offer_prohibited_or_full_field_targets() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    for full in [false, true] {
        let mut r = attack_runner(5, true);
        if full {
            while r.battle_area_size(0) < r.game.rules.field_slots as usize {
                r.place_on_field(0, "TEST-CARRIER", None);
            }
        } else {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotPlayTamerByEffect,
                    0,
                    Expiry::Permanent,
                    None,
                    1,
                ),
            );
        }
        attack(&mut r);
        finish_without_hand_choice(&mut r);
        assert_eq!(r.memory(), 10);
        assert_eq!(r.game.players[0].hand.len(), 2);
        assert!(r.pending_selection_view().is_none());
    }
}

#[test]
fn bt17_020_inherited_play_excludes_unpayable_reduced_cost() {
    let mut r = attack_runner(30, true);
    attack(&mut r);
    finish_without_hand_choice(&mut r);
    assert_eq!(r.memory(), 10);
    assert_eq!(r.game.players[0].hand.len(), 2);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

fn finish_without_hand_choice(r: &mut DebugRunner) {
    use digimon_engine::selection::SelectionKind;
    while let Some(p) = r.pending_selection() {
        assert_ne!(
            p.kind,
            SelectionKind::Hand,
            "illegal hand targets must not appear in the choice mask"
        );
        choose(r);
    }
}
