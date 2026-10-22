//! BT20-014 printed image + official bundle; suspend cost precedes optional evolution.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
fn builder() -> DebugRunnerBuilder {
    let mut base = make_test_card("BASE", "Level four");
    base.level = Some(4);
    base.colors = vec![CardColor::Red];
    let mut jes = make_test_card("JES", "Jesmon X Antibody");
    jes.level = Some(6);
    jes.traits = vec!["Royal Knight".into()];
    jes.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 4,
    }];
    let mut wrong = jes.clone();
    wrong.card_id = "WRONG".into();
    wrong.card_name = "WarGreymon".into();
    wrong.traits.clear();
    let mut illegal = jes.clone();
    illegal.card_id = "ILLEGAL".into();
    illegal.evo_costs[0].level = 6;
    let mut tamer = jes.clone();
    tamer.card_id = "TAMER".into();
    tamer.card_kind = CardKind::Tamer;
    let mut small = make_test_card("SMALL", "Small");
    small.dp = Some(5000);
    let mut big = small.clone();
    big.card_id = "BIG".into();
    big.dp = Some(5001);
    DebugRunner::builder()
        .dsl_card("BT20-014")
        .expect("BT20-014 production YAML")
        .add_card(base)
        .add_card(jes)
        .add_card(wrong)
        .add_card(illegal)
        .add_card(tamer)
        .add_card(small)
        .add_card(big)
        .deck(0, &["BASE"; 5])
        .deck(1, &["BASE"; 5])
        .memory(10)
}
fn choose(r: &mut DebugRunner, optional: bool, n: usize) {
    let p = r.pending_selection().expect("player-visible choice");
    assert_eq!(p.valid_action_ids.len(), n);
    let a = p.valid_action_ids[0];
    let mask = build_action_mask(&r.game, 0);
    for a in &p.valid_action_ids {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], if optional { 1.0 } else { 0.0 });
    r.execute_action(0, a).unwrap();
}
#[test]
fn bt20_014_on_play_and_evolve_mandatory_delete_dp_boundary() {
    for evo in [false, true] {
        let mut r = builder().hand(0, &["BT20-014"]).start();
        r.place_on_field(0, "SMALL", None);
        r.place_on_field(1, "SMALL", None);
        r.place_on_field(1, "SMALL", None);
        r.place_on_field(1, "BIG", None);
        r.place_on_field(1, "TAMER", None);
        if evo {
            r.place_on_field(0, "BASE", None);
            let a = encode_digivolve(0, 1);
            assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 7);
        } else {
            r.play(0, 0).unwrap();
        }
        choose(&mut r, false, 2);
        assert_eq!(r.game.players[1].battle_area.len(), 3);
        assert_eq!(r.trash_size(1), 1);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt20_014_no_delete_target() {
    let mut r = builder().hand(0, &["BT20-014"]).start();
    r.place_on_field(1, "BIG", None);
    r.place_on_field(1, "TAMER", None);
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area.len(), 2);
}
#[test]
fn bt20_014_end_turn_pays_other_suspend_before_free_legal_name_evolution() {
    let mut r = builder()
        .hand(0, &["JES", "JES", "WRONG", "ILLEGAL", "TAMER"])
        .start();
    r.place_on_field(0, "BT20-014", None);
    r.place_on_field(0, "SMALL", None);
    r.place_on_field(0, "SMALL", None);
    r.place_on_field(0, "TAMER", None);
    r.place_on_field(1, "SMALL", None);
    r.pass_turn();
    choose(&mut r, true, 2);
    assert!(r.game.players[0].battle_area[1].is_suspended);
    let before = r.memory();
    choose(&mut r, true, 2);
    assert_eq!(r.memory().abs(), before.abs());
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "JES"
    );
}
#[test]
fn bt20_014_decline_cost_or_payoff_and_pay_without_payoff() {
    for mode in 0..3 {
        let mut r = builder()
            .hand(0, if mode == 2 { &[][..] } else { &["JES"][..] })
            .start();
        r.place_on_field(0, "BT20-014", None);
        r.place_on_field(0, "SMALL", None);
        r.pass_turn();
        if mode == 0 {
            r.execute_action(0, PASS).unwrap();
            assert!(!r.game.players[0].battle_area[1].is_suspended);
        } else {
            choose(&mut r, true, 1);
            assert!(r.game.players[0].battle_area[1].is_suspended);
            if mode == 1 {
                r.execute_action(0, PASS).unwrap();
            }
        }
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "BT20-014"
        );
    }
}
#[test]
fn bt20_014_no_unsuspended_other_digimon_no_cost_choice() {
    let mut r = builder().hand(0, &["JES"]).start();
    r.place_on_field(0, "BT20-014", None);
    r.place_on_field(0, "SMALL", None);
    r.game.players[0].battle_area[1].is_suspended = true;
    r.place_on_field(0, "TAMER", None);
    r.pass_turn();
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_014_inherited_alliance_only_royal_knight_own_turn() {
    let mut r = builder().start();
    let knight = r.place_stack(0, &["BT20-014", "JES"]);
    let other = r.place_stack(0, &["BT20-014", "WRONG"]);
    let top = r.place_on_field(0, "BT20-014", None);
    for turn in [0, 1] {
        r.game.turn_player_idx = turn;
        r.game.tick_declarative_effects();
        assert_eq!(r.game.has_keyword(knight, Keyword::Alliance), turn == 0);
        assert!(!r.game.has_keyword(other, Keyword::Alliance));
        assert!(!r.game.has_keyword(top, Keyword::Alliance));
    }
}
#[test]
fn bt20_014_cannot_suspend_is_not_a_payable_cost() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::ModifierEntry,
    };
    let mut r = builder().hand(0, &["JES"]).start();
    r.place_on_field(0, "BT20-014", None);
    let p = r.place_on_field(0, "SMALL", None);
    r.game.modifiers.add(
        p,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    r.pass_turn();
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[1].is_suspended);
}
#[test]
fn bt20_014_opponent_end_turn_does_not_offer_evolution() {
    let mut r = builder().hand(0, &["JES"]).start();
    r.place_on_field(0, "BT20-014", None);
    r.place_on_field(0, "SMALL", None);
    r.game.turn_player_idx = 1;
    r.pass_turn();
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt20_014_failed_payment_after_selection_cannot_evolve() {
    use digimon_engine::{
        enums::{Expiry, ModifierType},
        modifiers::ModifierEntry,
    };
    let mut r = builder().hand(0, &["JES"]).start();
    r.place_on_field(0, "BT20-014", None);
    let payment = r.place_on_field(0, "SMALL", None);
    r.pass_turn();
    let action = r.pending_selection().unwrap().valid_action_ids[0];
    // Revalidate payment success even if prohibition appears after candidates were built.
    r.game.modifiers.add(
        payment,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    r.execute_action(0, action).unwrap();
    assert!(!r.game.players[0].battle_area[1].is_suspended);
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BT20-014"
    );
}
