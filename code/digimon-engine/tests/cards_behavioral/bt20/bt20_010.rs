//! BT20-010 Ryudamon: actual image, official bundle, and DCGO BT20_010.cs.
//! CR 4.3 8-1-2-1: choose among legal evolution requirements; 3-4-7: breeding.
use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET, PASS};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{
    make_test_card, make_test_egg, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;

const ID: &str = "BT20-010";

fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-010 must be in the production pack")
        .add_card(make_test_card("PLAIN", "Plain Rookie"));
    for (id, name, traits) in [
        ("GIN", "Ginryumon", vec![]),
        ("CHRONICLE", "Chronicle Champion", vec!["Chronicle"]),
        ("NEAR-NAME", "Ginryumon X", vec![]),
        (
            "NEAR-TRAIT",
            "Unrelated Champion",
            vec!["Chronicle Variant"],
        ),
    ] {
        let mut c = make_test_card(id, name);
        c.level = Some(4);
        c.dp = Some(5000);
        c.traits = traits.into_iter().map(str::to_owned).collect();
        c.evo_costs = vec![EvoCost {
            level: 3,
            card_color: 0,
            memory_cost: 3,
        }];
        b = b.add_card(c);
    }
    for (id, color, x_antibody) in [
        ("RED-EGG", CardColor::Red, false),
        ("BLACK-EGG", CardColor::Black, false),
        ("BLACK-X", CardColor::Black, true),
        ("RED-X", CardColor::Red, true),
        ("BLUE-X", CardColor::Blue, true),
    ] {
        let mut c = make_test_egg(id, id);
        c.colors = vec![color];
        if x_antibody {
            c.traits = vec!["X Antibody".into()];
        }
        b = b.add_card(c);
    }
    b.deck(0, &["PLAIN"; 12]).deck(1, &["PLAIN"; 12]).memory(10)
}

fn top(r: &DebugRunner, h: PermanentHandle) -> &str {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
}

fn evolve(r: &mut DebugRunner, hand: u16, h: PermanentHandle) {
    let action = encode_digivolve(hand, h.index as u16);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, h.player)[action as usize],
        1.0
    );
    r.game.decode_action(action, h.player);
}

#[test]
fn bt20_010_reduces_both_matching_targets_but_not_near_matches() {
    for (index, expected_cost, target) in [
        (0, 2, "GIN"),
        (1, 2, "CHRONICLE"),
        (2, 3, "NEAR-NAME"),
        (3, 3, "NEAR-TRAIT"),
    ] {
        let mut r = base()
            .hand(0, &["GIN", "CHRONICLE", "NEAR-NAME", "NEAR-TRAIT"])
            .start();
        r.skip_mulligan();
        let h = r.place_on_field(0, ID, Some(0));
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        for candidate in 0..4 {
            assert_eq!(
                mask[encode_digivolve(candidate, h.index as u16) as usize],
                1.0
            );
        }
        evolve(&mut r, index, h);
        assert_eq!(top(&r, h), target);
        assert_eq!(r.memory(), 10 - expected_cost);
        assert_eq!(r.deck_size(0), 11, "digivolution draws one");
        assert!(
            r.pending_selection().is_none(),
            "mandatory reduction adds no optional prompt"
        );
    }
}

#[test]
fn bt20_010_reduction_only_applies_to_this_top_card() {
    for inherited in [false, true] {
        let mut r = base().hand(0, &["GIN"]).start();
        r.skip_mulligan();
        let h = if inherited {
            r.place_stack(0, &[ID, "PLAIN"])
        } else {
            r.place_on_field(0, ID, Some(0));
            r.place_on_field(0, "PLAIN", Some(0))
        };
        evolve(&mut r, 0, h);
        assert_eq!(top(&r, h), "GIN");
        assert_eq!(r.memory(), 7);
    }
}

#[test]
fn bt20_010_reduction_is_inactive_during_opponents_turn() {
    let mut r = base().hand(0, &["GIN"]).start();
    r.skip_mulligan();
    let h = r.place_on_field(0, ID, Some(0));
    r.pass_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    let before = r.memory();
    assert!(r.game.effect_initiated_digivolve(
        0,
        0,
        h,
        CostDelta::Reduce(0),
        false,
        PlaySource::ByEffect
    ));
    assert_eq!(top(&r, h), "GIN");
    assert_eq!(
        r.memory(),
        before + 3,
        "the inactive owner pays the full cost"
    );
}

#[test]
fn bt20_010_reduction_is_inactive_in_breeding() {
    let mut r = base().hand(0, &["GIN"]).start();
    r.skip_mulligan();
    r.place_in_breeding(0, ID);
    let action = encode_digivolve(0, BREEDING_TARGET);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
        1.0
    );
    r.game.decode_action(action, 0);
    assert_eq!(r.memory(), 7);
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .card_id(&r.game.card_data),
        "GIN"
    );
}

#[test]
fn bt20_010_inherited_dp_is_self_only_and_turn_gated() {
    let mut r = base().start();
    r.skip_mulligan();
    let face = r.place_on_field(0, ID, Some(0));
    let inherited = r.place_stack(0, &[ID, "GIN"]);
    let other = r.place_on_field(0, "GIN", Some(0));
    assert_eq!(r.effective_dp(face), Some(1000));
    assert_eq!(r.effective_dp(inherited), Some(7000));
    assert_eq!(r.effective_dp(other), Some(5000));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(inherited), Some(5000));
}

#[test]
fn bt20_010_normal_routes_cost_one_and_special_route_filters_are_exact() {
    for (egg, legal) in [
        ("RED-EGG", true),
        ("BLACK-EGG", true),
        ("RED-X", true),
        ("BLUE-X", false),
        ("PLAIN", false),
    ] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, egg, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            digimon_engine::action::build_action_mask(&r.game, 0)[action as usize] > 0.0,
            legal,
            "{egg}"
        );
        if legal {
            r.game.decode_action(action, 0);
            assert_eq!(top(&r, h), ID);
            assert_eq!(r.memory(), 9, "{egg}");
        }
    }
}

#[test]
fn bt20_010_black_x_antibody_egg_exposes_both_cost_routes() {
    for cost in [0, 1] {
        let mut r = base().hand(0, &[ID]).start();
        r.skip_mulligan();
        let h = r.place_on_field(0, "BLACK-X", Some(0));
        evolve(&mut r, 0, h);
        let v = r
            .pending_selection_view()
            .expect("choose between printed cost 1 and special cost 0");
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(), 2);
        let choices = v.effect_choices.as_ref().expect("cost labels");
        let action = choices
            .iter()
            .find(|c| c.label == format!("Digivolve for cost {cost}"))
            .unwrap()
            .action_id;
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        for candidate in &v.valid_action_ids {
            assert_eq!(mask[*candidate as usize], 1.0);
        }
        assert_eq!(mask[PASS as usize], 0.0);
        assert_eq!(r.memory(), 10);
        r.execute_action(0, action).unwrap();
        assert_eq!(top(&r, h), ID);
        assert_eq!(r.memory(), 10 - cost);
        assert_eq!(r.deck_size(0), 11);
        assert!(r.pending_selection().is_none());
    }
}
