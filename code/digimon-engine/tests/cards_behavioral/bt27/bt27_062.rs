//! Official Bandai BT27-062 image / Q7763-64, checked 2026-11-22.
//! Full-cost route under Solarmon is required; these are behavior fixtures only.
use digimon_engine::{
    action::{
        mask::build_action_mask,
        space::{encode_digivolve, PASS},
    },
    card_data::CardData,
    debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind, Expiry, ModifierType},
    modifiers::ModifierEntry,
    permanent::PermanentHandle,
    replacement::ReplacementCause,
};
const ID: &str = "BT27-062";
fn card(id: &str, kind: CardKind, cost: u16, knight: bool) -> CardData {
    let mut c = make_test_card_with_level(id, id, 3);
    c.card_kind = kind;
    c.play_cost = cost;
    c.colors = vec![CardColor::Black];
    c.dp = Some(1000);
    if knight {
        c.effect_text = "Return 1 [Knightmon] to the hand.".into();
    }
    c
}
fn base() -> DebugRunnerBuilder {
    let mut top = card("TOP", CardKind::Digimon, 8, false);
    top.level = Some(6);
    top.dp = Some(12000);
    let mut red = card("RED", CardKind::Digimon, 3, false);
    red.colors = vec![CardColor::Red];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-062 production YAML")
        .dsl_card("ST12-03")
        .unwrap()
        .add_card(card("F", CardKind::Digimon, 3, false))
        .add_card(top)
        .add_card(red)
        .add_card(card("K", CardKind::Digimon, 5, true))
        .add_card(card("EXP", CardKind::Digimon, 11, true))
        .add_card(card("T", CardKind::Tamer, 5, true))
        .add_card(card("O", CardKind::Option, 5, true))
        .add_card(card("EGG", CardKind::DigiEgg, 0, true))
        .deck(0, &["F"; 30])
        .deck(1, &["F"; 30])
        .security(1, &["F"; 5])
        .memory(10)
}
fn start(hand: &[&str]) -> DebugRunner {
    let mut r = base().hand(0, hand).start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}
fn picks(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection_view()
        .unwrap()
        .valid_action_ids
        .into_iter()
        .filter(|a| *a != PASS)
        .collect()
}
fn act(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, p)[a as usize], 1.0);
    r.execute_action(p, a).unwrap();
}
fn choose(r: &mut DebugRunner, n: usize) {
    let a = picks(r)[n];
    act(r, a);
}
fn drive(r: &mut DebugRunner) {
    for _ in 0..12 {
        if r.pending_selection().is_none() {
            return;
        }
        choose(r, 0);
    }
    panic!("unresolved choice");
}
fn count(r: &DebugRunner, p: usize, id: &str) -> usize {
    r.game.players[p]
        .battle_area
        .iter()
        .filter(|x| x.top_card().card_id(&r.game.card_data) == id)
        .count()
}
fn attack(r: &mut DebugRunner) -> PermanentHandle {
    let h = r.place_on_field(0, ID, Some(0));
    r.attack_player(h, 1, false);
    h
}
#[test]
fn bt27_062_printed_metadata_and_actual_evolution() {
    for (base_id, legal) in [("F", true), ("RED", false), ("TOP", false)] {
        let mut r = start(&[ID]);
        let h = r.place_on_field(0, base_id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[a as usize],
            if legal { 1.0 } else { 0.0 }
        );
        if legal {
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), 8);
        }
    }
    let r = start(&[]);
    let c = r.compiled_card(ID).unwrap();
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.dp, Some(4000));
    assert_eq!(c.level, Some(4));
}
#[test]
fn bt27_062_attack_plays_or_uses_reduced_cost() {
    for id in ["K", "T", "O"] {
        let mut r = start(&[id]);
        attack(&mut r);
        assert!(r.pending_is_optional());
        assert_eq!(picks(&r).len(), 1);
        drive(&mut r);
        assert_eq!(r.memory(), 7, "{id}");
        if id == "O" {
            assert_eq!(
                r.game.players[0]
                    .trash
                    .last()
                    .unwrap()
                    .card_id(&r.game.card_data),
                id
            );
        } else {
            assert_eq!(count(&r, 0, id), 1);
        }
    }
}
#[test]
fn bt27_062_q7764_solarmon_allows_full_cost_digimon_and_tamer() {
    for id in ["K", "T"] {
        let mut r = start(&[id]);
        r.place_on_field(1, "ST12-03", Some(0));
        attack(&mut r);
        assert_eq!(picks(&r).len(), 1, "full cost is payable");
        drive(&mut r);
        assert_eq!(count(&r, 0, id), 1);
        assert_eq!(r.memory(), 5, "Solarmon forbids reduction, not the play");
    }
}
#[test]
fn bt27_062_q7764_insufficient_full_cost_not_offered() {
    let mut r = start(&["EXP"]);
    r.game.memory = 0;
    r.place_on_field(1, "ST12-03", Some(0));
    attack(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "11 cost cannot be paid from zero memory even though reduced9 could"
    );
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(count(&r, 0, "EXP"), 0);
}
#[test]
fn bt27_062_without_solarmon_reduced_only_affordability_is_legal() {
    let mut r = start(&["EXP"]);
    r.game.memory = 0;
    attack(&mut r);
    assert_eq!(picks(&r).len(), 1);
    drive(&mut r);
    assert_eq!(count(&r, 0, "EXP"), 1);
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.memory(), 9);
}
#[test]
fn bt27_062_solarmon_does_not_forbid_option_use_reduction() {
    let mut r = start(&["O"]);
    r.place_on_field(1, "ST12-03", Some(0));
    attack(&mut r);
    drive(&mut r);
    assert_eq!(r.memory(), 7);
}
#[test]
fn bt27_062_decline_and_wrong_text_or_kind_are_negative() {
    let mut r = start(&["K", "F", "EGG"]);
    attack(&mut r);
    assert_eq!(picks(&r).len(), 1);
    act(&mut r, PASS);
    assert_eq!(r.hand_size(0), 3);
    assert_eq!(r.memory(), 10);
    let mut r = start(&["F", "EGG"]);
    attack(&mut r);
    assert!(r.pending_selection().is_none());
}
#[test]
fn bt27_062_full_printed_text_domains_are_matched() {
    for field in 0..5 {
        let mut c = card("MATCH", CardKind::Digimon, 5, false);
        match field {
            0 => c.card_name = "DarkKnightmon".into(),
            1 => c.traits = vec!["Knightmon".into()],
            2 => c.inherited_text = "[Knightmon]".into(),
            3 => c.security_text = "[Knightmon]".into(),
            _ => c.requirement_text = "[Digivolve] [Knightmon]".into(),
        };
        let mut r = base().add_card(c).hand(0, &["MATCH"]).start();
        r.skip_mulligan();
        r.set_first_player(0);
        attack(&mut r);
        assert_eq!(picks(&r).len(), 1, "field {field}");
        drive(&mut r);
        assert_eq!(count(&r, 0, "MATCH"), 1);
    }
}
#[test]
fn bt27_062_attack_once_per_turn_is_per_card() {
    let mut r = start(&["K", "K"]);
    let h = attack(&mut r);
    drive(&mut r);
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    r.attack_player(h, 1, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    let h2 = r.place_on_field(0, ID, Some(0));
    r.attack_player(h2, 1, false);
    drive(&mut r);
    assert_eq!(r.hand_size(0), 0);
}
#[test]
fn bt27_062_inherited_other_own_digimon_payment_then_prevent_once() {
    let mut r = start(&[]);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.place_on_field(0, "K", Some(0));
    r.place_on_field(0, "K", Some(0));
    r.place_on_field(0, "T", Some(0));
    r.place_on_field(1, "K", Some(0));
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert_eq!(picks(&r).len(), 2);
    choose(&mut r, 1);
    drive(&mut r);
    assert_eq!(count(&r, 0, "TOP"), 1);
    assert_eq!(count(&r, 0, "K"), 1);
    assert_eq!(count(&r, 1, "K"), 1);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    drive(&mut r);
    assert_eq!(count(&r, 0, "TOP"), 0);
}
#[test]
fn bt27_062_inherited_decline_no_cost_and_top_face_do_not_prevent() {
    for case in 0..3 {
        let mut r = start(&[]);
        let h = if case == 2 {
            r.place_on_field(0, ID, Some(0))
        } else {
            r.place_stack(0, &[ID, "TOP"])
        };
        if case != 1 {
            r.place_on_field(0, "K", Some(0));
        }
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
        if r.pending_selection().is_some() {
            act(&mut r, PASS);
        }
        assert_eq!(count(&r, 0, if case == 2 { ID } else { "TOP" }), 0);
    }
}
#[test]
fn bt27_062_inherited_works_for_own_effect_and_battle() {
    for cause in [ReplacementCause::OwnEffect, ReplacementCause::Battle] {
        let mut r = start(&[]);
        let h = r.place_stack(0, &[ID, "TOP"]);
        r.place_on_field(0, "K", Some(0));
        r.game.delete_permanent_with_cause(h, cause);
        drive(&mut r);
        assert_eq!(count(&r, 0, "TOP"), 1);
        assert_eq!(count(&r, 0, "K"), 0);
    }
}
#[test]
fn bt27_062_inherited_cannot_pay_with_indestructible_digimon() {
    let mut r = start(&[]);
    let h = r.place_stack(0, &[ID, "TOP"]);
    let k = r.place_on_field(0, "K", Some(0));
    r.game.modifiers.add(
        k,
        ModifierEntry::simple(ModifierType::CannotBeDestroyed, 1, Expiry::Permanent, 0),
    );
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    drive(&mut r);
    assert_eq!(count(&r, 0, "TOP"), 0);
    assert_eq!(count(&r, 0, "K"), 1);
}
#[test]
fn bt27_062_inherited_does_not_protect_another_subject() {
    let mut r = start(&[]);
    r.place_stack(0, &[ID, "TOP"]);
    let h = r.place_on_field(0, "F", Some(0));
    r.place_on_field(0, "K", Some(0));
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert_eq!(count(&r, 0, "F"), 0);
}
#[test]
fn bt27_062_declined_attack_can_activate_later_in_same_turn() {
    let mut r = start(&["K"]);
    let h = attack(&mut r);
    act(&mut r, PASS);
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    r.attack_player(h, 1, false);
    assert_eq!(picks(&r).len(), 1);
    drive(&mut r);
    assert_eq!(count(&r, 0, "K"), 1);
}
#[test]
fn bt27_062_saved_payer_does_not_pay_inherited_leave_prevention() {
    let mut r = base().dsl_card("BT27-023").unwrap().start();
    r.skip_mulligan();
    r.set_first_player(0);
    let h = r.place_stack(0, &[ID, "TOP"]);
    r.place_stack(0, &["BT27-023", "K"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    drive(&mut r);
    assert_eq!(
        count(&r, 0, "TOP"),
        0,
        "Evade saved the payment Digimon, so the original leave proceeds"
    );
    assert_eq!(count(&r, 0, "K"), 1);
}
#[test]
fn bt27_062_inherited_prevents_return_and_is_controller_relative() {
    for p in [0u8, 1u8] {
        let mut r = start(&[]);
        let h = r.place_stack(p, &[ID, "TOP"]);
        r.place_on_field(p, "K", Some(0));
        r.place_on_field(1 - p, "K", Some(0));
        r.game.return_to_hand(h);
        assert_eq!(r.pending_selection_view().unwrap().selecting_player, p);
        drive(&mut r);
        assert_eq!(count(&r, p as usize, "TOP"), 1);
        assert_eq!(count(&r, p as usize, "K"), 0);
        assert_eq!(count(&r, (1 - p) as usize, "K"), 1);
    }
}
