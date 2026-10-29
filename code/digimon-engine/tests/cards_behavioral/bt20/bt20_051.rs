//! BT20-051: behavioral tests being authored in the Chronicle batch.
//! BT20-051 Raptordramon: inspected card image, official bundle, and BT20_051.cs.
//! CR 4.3 8-1-3: choose an evolution requirement, pay its cost, then draw.
//! CR 4.3 15-8-2-2/3: inherited persistent DP tracks the host owner's turn.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, BREEDING_TARGET, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, Expiry, ModifierType, PlaySource};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-051";

fn digimon(id: &str, name: &str, color: CardColor, level: u8, chronicle: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(8000);
    if chronicle {
        c.traits.push("Chronicle".into());
    }
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut next = digimon("NEXT", "Next Digimon", CardColor::Black, 5, false);
    next.evo_costs.push(EvoCost {
        card_color: CardColor::Black as u8,
        level: 4,
        memory_cost: 3,
    });
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-051 must be implemented")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(tamer("KOTA-A", "Kota Domoto"))
        .add_card(tamer("KOTA-B", "Kota Domoto"))
        .add_card(tamer("NEAR-NAME", "Kota Domoto & Yuji Musya"))
        .add_card(tamer("OTHER-TAMER", "Other Tamer"))
        .add_card(digimon(
            "NOT-TAMER",
            "Kota Domoto",
            CardColor::Black,
            3,
            false,
        ))
        .add_card(digimon(
            "BLACK3",
            "Black Rookie",
            CardColor::Black,
            3,
            false,
        ))
        .add_card(digimon(
            "YELLOW3",
            "Yellow Rookie",
            CardColor::Yellow,
            3,
            false,
        ))
        .add_card(digimon("BLUE3", "Blue Rookie", CardColor::Blue, 3, false))
        .add_card(digimon(
            "CHRON3",
            "Chronicle Rookie",
            CardColor::Blue,
            3,
            true,
        ))
        .add_card(digimon(
            "CHRON4",
            "Chronicle Champion",
            CardColor::Blue,
            4,
            true,
        ))
        .add_card(digimon("DORU", "Dorumon", CardColor::Black, 3, false))
        .add_card(digimon("DORU-BLUE", "Dorumon", CardColor::Blue, 3, false))
        .add_card(digimon(
            "DORU-VARIANT",
            "Dorumon Variant",
            CardColor::Blue,
            3,
            false,
        ))
        .add_card(next)
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .memory(8)
}

fn choose(r: &mut DebugRunner, action: u16) {
    let v = r.pending_selection_view().expect("player-visible choice");
    assert_eq!(
        build_action_mask(&r.game, v.selecting_player)[action as usize],
        1.0
    );
    r.execute_action(v.selecting_player, action).unwrap();
}

fn evolve(r: &mut DebugRunner, hand_index: u16, h: PermanentHandle) {
    let action = encode_digivolve(hand_index, h.index as u16);
    assert_eq!(build_action_mask(&r.game, h.player)[action as usize], 1.0);
    r.game.decode_action(action, h.player);
}

fn top(r: &DebugRunner, h: PermanentHandle) -> &str {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

#[test]
fn when_digivolving_offers_both_exact_name_tamers_and_plays_selected_one_free() {
    for own_tamers in [0, 1] {
        for pick in [0, 1] {
            let mut r = builder()
                .hand(0, &[ID, "KOTA-A", "KOTA-B", "NEAR-NAME", "NOT-TAMER"])
                .start();
            r.skip_mulligan();
            for _ in 0..own_tamers {
                r.place_on_field(0, "OTHER-TAMER", Some(0));
            }
            // The printed threshold counts the controller's Tamers only.
            r.place_on_field(1, "OTHER-TAMER", Some(0));
            r.place_on_field(1, "OTHER-TAMER", Some(0));
            let h = r.place_on_field(0, "BLACK3", Some(0));
            evolve(&mut r, 0, h);
            assert_eq!(top(&r, h), ID);
            assert_eq!(r.memory(), 5, "evolution pays 3 before the optional play");
            assert_eq!(r.deck_size(0), 19);
            let v = r.pending_selection_view().expect("optional Kota choice");
            assert_eq!(v.kind, SelectionKind::Hand);
            assert_eq!(v.selecting_player, 0);
            assert!(v.is_optional);
            let mut expected = vec![PLAY_HAND_START, PLAY_HAND_START + 1];
            let mut actual = v.valid_action_ids;
            expected.sort_unstable();
            actual.sort_unstable();
            assert_eq!(actual, expected);
            let mask = build_action_mask(&r.game, 0);
            for index in 0..5 {
                assert_eq!(
                    mask[PLAY_HAND_START as usize + index],
                    if index < 2 { 1.0 } else { 0.0 }
                );
            }
            assert_eq!(mask[PASS as usize], 1.0);
            choose(&mut r, PLAY_HAND_START + pick);
            assert!(r.pending_selection().is_none());
            assert_eq!(r.memory(), 5, "Kota's play cost is not paid");
            assert_eq!(r.hand_size(0), 4);
            assert_eq!(r.battle_area_size(0), own_tamers + 2);
            let played = r.game.players[0].battle_area.last().unwrap();
            assert_eq!(
                played.top_card().card_id(&r.game.card_data),
                if pick == 0 { "KOTA-A" } else { "KOTA-B" }
            );
            assert!(!played.is_suspended);
        }
    }
}

#[test]
fn declining_keeps_cards_and_the_next_evolution_can_offer_again() {
    let mut r = builder().hand(0, &[ID, ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    let first = r.place_on_field(0, "BLACK3", Some(0));
    let second = r.place_on_field(0, "BLACK3", Some(0));
    evolve(&mut r, 0, first);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    choose(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.hand_size(0),
        4,
        "one evolution card out and one bonus draw in"
    );
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.memory(), 5);
    evolve(&mut r, 0, second);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    choose(&mut r, PLAY_HAND_START + 1);
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(
        r.game.players[0].battle_area[2]
            .top_card()
            .card_id(&r.game.card_data),
        "KOTA-B"
    );
    assert_eq!(r.memory(), 2);
}

#[test]
fn two_own_tamers_block_the_free_play() {
    let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    r.place_on_field(0, "OTHER-TAMER", Some(0));
    r.place_on_field(0, "OTHER-TAMER", Some(0));
    let h = r.place_on_field(0, "BLACK3", Some(0));
    evolve(&mut r, 0, h);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(r.hand_size(0), 3);
    assert_eq!(r.memory(), 5);
}

#[test]
fn no_matching_hand_tamer_has_no_prompt_even_with_kota_in_trash() {
    for hand in [&[ID][..], &[ID, "NEAR-NAME", "NOT-TAMER"][..]] {
        let mut r = builder().hand(0, hand).start();
        r.skip_mulligan();
        r.inject_trash(0, "KOTA-A");
        let h = r.place_on_field(0, "BLACK3", Some(0));
        evolve(&mut r, 0, h);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.trash_size(0), 1);
        assert_eq!(r.memory(), 5);
    }
}

#[test]
fn tamer_effect_play_prohibition_hides_illegal_kota_choices_only_for_its_player() {
    for prohibited_player in [0, 1] {
        let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, "BLACK3", Some(0));
        r.game.modifiers.add_player_modifier(
            prohibited_player,
            PlayerModifierEntry::simple(
                ModifierType::CannotPlayTamerByEffect,
                0,
                Expiry::Permanent,
                None,
                1,
            ),
        );
        evolve(&mut r, 0, h);
        if prohibited_player == 0 {
            assert!(
                r.pending_selection().is_none(),
                "unplayable Kota cards must not be offered by the effect"
            );
            // Normal paid play remains legal; only the effect's choice is barred.
            let mask = build_action_mask(&r.game, 0);
            assert_eq!(mask[PLAY_HAND_START as usize], 1.0);
            assert_eq!(mask[PLAY_HAND_START as usize + 1], 1.0);
        } else {
            let v = r
                .pending_selection_view()
                .expect("opponent's prohibition does not restrict this owner");
            assert_eq!(v.kind, SelectionKind::Hand);
            assert_eq!(v.selecting_player, 0);
            let mut actual = v.valid_action_ids;
            let mut expected = vec![PLAY_HAND_START, PLAY_HAND_START + 1];
            actual.sort_unstable();
            expected.sort_unstable();
            assert_eq!(actual, expected);
            choose(&mut r, PASS);
        }
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.hand_size(0), 3);
        assert_eq!(r.memory(), 5);
    }
}

#[test]
fn full_battle_area_hides_kota_choices_without_spending_or_moving_them() {
    let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, "BLACK3", Some(0));
    while r.battle_area_size(0) < r.game.rules.field_slots as usize {
        r.place_on_field(0, "BLACK3", Some(0));
    }
    let count = r.battle_area_size(0);
    evolve(&mut r, 0, h);
    assert!(
        r.pending_selection().is_none(),
        "a free play cannot be chosen without a destination field slot"
    );
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
    assert_eq!(mask[PLAY_HAND_START as usize + 1], 0.0);
    assert_eq!(r.battle_area_size(0), count);
    assert_eq!(r.hand_size(0), 3);
    assert_eq!(r.memory(), 5);
}

#[test]
fn on_play_other_digivolution_and_inherited_position_do_not_grant_the_face_effect() {
    let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.pending_selection().is_none(), "no On Play text");
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.hand_size(0), 2);

    for inherited in [false, true] {
        let mut r = builder().hand(0, &["NEXT", "KOTA-A", "KOTA-B"]).start();
        r.skip_mulligan();
        let h = if inherited {
            r.place_on_field(0, ID, Some(0))
        } else {
            r.place_on_field(0, ID, Some(0));
            // Use a second Raptordramon as the evolution base: after evolution,
            // neither its inherited copy nor the other face may trigger.
            r.place_on_field(0, ID, Some(0))
        };
        evolve(&mut r, 0, h);
        assert_eq!(top(&r, h), "NEXT");
        assert!(r.pending_selection().is_none());
        assert_eq!(r.hand_size(0), 3);
    }
}

#[test]
fn evolution_in_breeding_does_not_offer_the_battle_area_effect() {
    let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    r.place_in_breeding(0, "BLACK3");
    let action = encode_digivolve(0, BREEDING_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.memory(), 5);
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
}

#[test]
fn when_digivolving_during_opponents_turn_still_offers_owner_the_choice() {
    let mut r = builder().hand(0, &[ID, "KOTA-A", "KOTA-B"]).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, "BLACK3", Some(0));
    r.pass_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    let before_evolution = r.memory();
    assert!(r.game.effect_initiated_digivolve(
        0,
        0,
        h,
        CostDelta::Reduce(0),
        false,
        PlaySource::ByEffect
    ));
    let v = r
        .pending_selection_view()
        .expect("WD has no Your Turn restriction");
    assert_eq!(v.kind, SelectionKind::Hand);
    assert_eq!(v.selecting_player, 0);
    assert_eq!(
        r.memory(),
        before_evolution + 3,
        "the inactive owner pays the printed evolution cost"
    );
    let before = r.memory();
    choose(&mut r, PLAY_HAND_START + 1);
    assert_eq!(r.memory(), before);
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "KOTA-B"
    );
}

#[test]
fn inherited_dp_is_self_only_and_applies_during_each_owners_opponents_turn() {
    let mut r = builder().start();
    r.skip_mulligan();
    let face = r.place_on_field(0, ID, Some(0));
    let own = r.place_stack(0, &[ID, "NEXT"]);
    let other = r.place_on_field(0, "NEXT", Some(0));
    let opponent = r.place_stack(1, &[ID, "NEXT"]);
    assert_eq!(r.effective_dp(face), Some(6000));
    assert_eq!(r.effective_dp(own), Some(8000));
    assert_eq!(r.effective_dp(other), Some(8000));
    assert_eq!(r.effective_dp(opponent), Some(10000));
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.effective_dp(face), Some(6000));
    assert_eq!(r.effective_dp(own), Some(10000));
    assert_eq!(r.effective_dp(other), Some(8000));
    assert_eq!(r.effective_dp(opponent), Some(8000));
}

#[test]
fn alternate_and_printed_evolution_routes_pay_the_selected_cost() {
    for (base, cost) in [
        ("BLACK3", 3),
        ("YELLOW3", 3),
        ("CHRON3", 2),
        ("DORU-BLUE", 2),
        ("DORU", 2),
        ("DORU", 3),
    ] {
        let mut r = builder().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, base, Some(0));
        evolve(&mut r, 0, h);
        if base == "DORU" {
            let v = r
                .pending_selection_view()
                .expect("both legal costs are selectable");
            assert_eq!(v.kind, SelectionKind::EffectChoice);
            assert!(!v.is_optional);
            assert_eq!(v.valid_action_ids.len(), 2);
            let mask = build_action_mask(&r.game, 0);
            assert_eq!(mask[PASS as usize], 0.0);
            for &action in &v.valid_action_ids {
                assert_eq!(mask[action as usize], 1.0);
            }
            let choices = v.effect_choices.unwrap();
            let action = choices
                .iter()
                .find(|c| c.label == format!("Digivolve for cost {cost}"))
                .unwrap()
                .action_id;
            choose(&mut r, action);
        }
        assert_eq!(top(&r, h), ID, "{base}");
        assert_eq!(r.memory(), 8 - cost, "{base}");
        assert_eq!(r.deck_size(0), 19);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn wrong_color_level_and_partial_name_have_no_evolution_route() {
    for base in ["BLUE3", "CHRON4", "DORU-VARIANT"] {
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
