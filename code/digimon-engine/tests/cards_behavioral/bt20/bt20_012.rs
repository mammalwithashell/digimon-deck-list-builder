//! BT20-012 Ginryumon: card image and official bundle; DCGO BT20_012.cs.
//! CR 4.3 8-1-3: choose a legal evolution requirement, pay its cost, draw one.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START, SECURITY_TARGET},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-012";

fn card(id: &str, name: &str, level: u8, color: CardColor, chronicle: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(8000);
    if chronicle {
        c.traits.push("Chronicle".into());
    }
    c
}

fn evolution(id: &str, name: &str, chronicle: bool, from: CardColor, level: u8) -> CardData {
    let mut c = card(id, name, 5, CardColor::Black, chronicle);
    c.evo_costs.push(EvoCost {
        card_color: from as u8,
        level,
        memory_cost: 3,
    });
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut carrier = card("CARRIER", "Dual-color Champion", 4, CardColor::Black, false);
    carrier.colors.push(CardColor::Red);
    let mut option = evolution("OPTION", "Chronicle Option", true, CardColor::Black, 4);
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-012 must be implemented")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(card("RED3", "Red Rookie", 3, CardColor::Red, false))
        .add_card(card("BLACK3", "Black Rookie", 3, CardColor::Black, false))
        .add_card(card("BLUE3", "Blue Rookie", 3, CardColor::Blue, false))
        .add_card(card("CHRON3", "Chronicle Rookie", 3, CardColor::Blue, true))
        .add_card(card(
            "CHRON4",
            "Chronicle Champion",
            4,
            CardColor::Blue,
            true,
        ))
        .add_card(card("RYUDA", "Ryudamon", 3, CardColor::Red, false))
        .add_card(card("RYUDA-BLUE", "Ryudamon", 3, CardColor::Blue, false))
        .add_card(card(
            "RYUDA-VARIANT",
            "Ryudamon Variant",
            3,
            CardColor::Blue,
            false,
        ))
        .add_card(evolution(
            "HISYA",
            "Hisyaryumon",
            false,
            CardColor::Black,
            4,
        ))
        .add_card(evolution(
            "CHRON",
            "Chronicle Ultimate",
            true,
            CardColor::Red,
            4,
        ))
        .add_card(evolution(
            "WRONG-NAME",
            "Hisyaryumon Variant",
            false,
            CardColor::Black,
            4,
        ))
        .add_card(evolution(
            "WRONG-COLOR",
            "Chronicle Blue",
            true,
            CardColor::Blue,
            4,
        ))
        .add_card(evolution(
            "WRONG-LEVEL",
            "Chronicle Mega",
            true,
            CardColor::Black,
            5,
        ))
        .add_card(option)
        .add_card(carrier)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(1, &["FILL"; 5])
        .memory(8)
}

fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    (r, h)
}

fn attack(r: &mut DebugRunner, h: PermanentHandle) {
    let action = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
}

fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .into()
}

fn choose(r: &mut DebugRunner, action: u16) {
    let view = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(
        build_action_mask(&r.game, view.selecting_player)[action as usize],
        1.0
    );
    r.execute_action(view.selecting_player, action).unwrap();
}

fn finish_combat(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        assert!(
            view.is_optional,
            "only optional counter/block choices remain: {view:?}"
        );
        choose(r, PASS);
    }
    panic!("combat did not finish");
}

#[test]
fn when_attacking_offers_both_named_and_trait_cards_with_exact_filters() {
    for pick in [0, 1] {
        let (mut r, h) = setup(&[
            "HISYA",
            "CHRON",
            "WRONG-NAME",
            "WRONG-COLOR",
            "WRONG-LEVEL",
            "OPTION",
        ]);
        r.inject_trash(0, "CHRON");
        attack(&mut r, h);
        let view = r
            .pending_selection_view()
            .expect("optional evolution from hand");
        assert_eq!(view.kind, SelectionKind::Hand);
        assert_eq!(view.selecting_player, 0);
        assert!(view.is_optional);
        let mask = build_action_mask(&r.game, 0);
        for i in 0..6 {
            assert_eq!(
                mask[PLAY_HAND_START as usize + i],
                if i < 2 { 1.0 } else { 0.0 }
            );
        }
        assert_eq!(mask[PASS as usize], 1.0);
        assert_eq!(
            view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
            2
        );
        let deck_before = r.game.players[0].deck.len();
        choose(&mut r, PLAY_HAND_START + pick);
        assert_eq!(top(&r, h), if pick == 0 { "HISYA" } else { "CHRON" });
        assert_eq!(r.memory(), 5, "printed cost 3 is paid");
        assert_eq!(
            r.game.players[0].deck.len(),
            deck_before - 1,
            "evolution bonus draw"
        );
        assert_eq!(
            r.effective_dp(h),
            Some(10000),
            "Ginryumon's inherited DP applies immediately"
        );
        assert_eq!(
            r.trash_size(0),
            1,
            "the hand-only effect leaves trash alone"
        );
    }
}

#[test]
fn decline_preserves_hand_memory_and_can_be_offered_on_another_attack() {
    let (mut r, h) = setup(&["HISYA", "CHRON"]);
    for _ in 0..2 {
        attack(&mut r, h);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
        choose(&mut r, PASS);
        finish_combat(&mut r);
        assert_eq!(top(&r, h), ID);
        assert_eq!(r.memory(), 8);
        assert_eq!(r.hand_size(0), 2);
        r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    }
}

#[test]
fn no_legal_hand_card_does_not_offer_evolution_even_with_a_matching_trash_card() {
    for hand in [
        &[][..],
        &["WRONG-NAME", "WRONG-COLOR", "WRONG-LEVEL", "OPTION"][..],
    ] {
        let (mut r, h) = setup(hand);
        r.inject_trash(0, "HISYA");
        attack(&mut r, h);
        assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
        finish_combat(&mut r);
        assert_eq!(top(&r, h), ID);
        assert_eq!(r.memory(), 8);
    }
}

#[test]
fn another_digimons_attack_does_not_trigger_ginryumon() {
    let (mut r, h) = setup(&["HISYA", "CHRON"]);
    let other = r.place_on_field(0, "FILL", Some(0));
    attack(&mut r, other);
    assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
    finish_combat(&mut r);
    assert_eq!(top(&r, h), ID);
    assert_eq!(r.memory(), 8);
}

#[test]
fn inherited_dp_tracks_each_owners_turn_and_does_not_affect_other_digimon() {
    let (mut r, face) = setup(&[]);
    let own = r.place_stack(0, &[ID, "HISYA"]);
    let other = r.place_on_field(0, "HISYA", Some(0));
    let opposing = r.place_stack(1, &[ID, "HISYA"]);
    assert_eq!(r.effective_dp(face), Some(6000));
    assert_eq!(r.effective_dp(own), Some(10000));
    assert_eq!(r.effective_dp(other), Some(8000));
    assert_eq!(r.effective_dp(opposing), Some(8000));
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    assert_eq!(r.effective_dp(own), Some(8000));
    assert_eq!(r.effective_dp(opposing), Some(10000));
}

#[test]
fn inherited_card_does_not_grant_its_face_when_attacking_effect() {
    let mut r = builder().hand(0, &["HISYA", "CHRON"]).start();
    r.skip_mulligan();
    // Lv4 carrier makes both result cards legal, so failure to trigger is scope-specific.
    let h = r.place_stack(0, &[ID, "CARRIER"]);
    attack(&mut r, h);
    assert_ne!(r.pending_kind(), Some(SelectionKind::Hand));
    finish_combat(&mut r);
    assert_eq!(top(&r, h), "CARRIER");
}

#[test]
fn alternate_and_printed_evolution_routes_pay_the_selected_cost() {
    for (base, cost) in [
        ("RED3", 3),
        ("BLACK3", 3),
        ("CHRON3", 2),
        ("RYUDA-BLUE", 2),
        ("RYUDA", 2),
        ("RYUDA", 3),
    ] {
        let mut r = builder().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, base, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            1.0,
            "{base}"
        );
        r.game.decode_action(action, 0);
        if base == "RYUDA" {
            let view = r
                .pending_selection_view()
                .expect("both cost-2 and cost-3 routes are selectable");
            assert_eq!(view.kind, SelectionKind::EffectChoice);
            let choices = view.effect_choices.unwrap();
            assert!(choices.len() >= 2);
            for choice in &choices {
                assert_eq!(
                    build_action_mask(&r.game, 0)[choice.action_id as usize],
                    1.0
                );
            }
            let selected = choices
                .iter()
                .find(|c| c.label.contains(&format!("{cost}")))
                .unwrap();
            choose(&mut r, selected.action_id);
        }
        assert_eq!(top(&r, h), ID, "{base}");
        assert_eq!(r.memory(), 8 - cost, "{base}");
        assert_eq!(r.game.players[0].deck.len(), 19);
    }
}

#[test]
fn wrong_color_level_or_name_has_no_evolution_route() {
    for base in ["BLUE3", "CHRON4", "RYUDA-VARIANT"] {
        let mut r = builder().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, base, Some(0));
        assert_eq!(
            build_action_mask(&r.game, 0)[encode_digivolve(0, h.index as u16) as usize],
            0.0,
            "{base}"
        );
        assert_eq!(top(&r, h), base);
    }
}
