//! BT17-012 BurningGreymon — Lv.4 Red/Purple Hybrid, Dark Dragon.
//!
//! [Digivolve] Red Lv.3 / Purple Lv.3: Cost 3; [Takuya Kanbara]: Cost 2;
//! [Agunimon]: Cost 1. This card may digivolve from the hand onto one of your
//! red Tamers as if that Tamer is a level 3 red Digimon.
//! <Raid>
//! [When Attacking] This Digimon may digivolve into a Digimon card with the
//! [Hybrid] trait in the hand with the digivolution cost reduced by 1.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! DCGO: BT17/Red/BT17_012.cs. Sister of BT17-023 (same set, same shape).
//! Synthetic boards only (no recorded games).
use digimon_engine::action::{
    build_action_mask,
    space::{encode_digivolve, PASS},
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT17-012";

fn base(id: &str, kind: CardKind, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    c.level = if kind == CardKind::Tamer { None } else { Some(level) };
    c.dp = if kind == CardKind::Tamer { None } else { Some(10000) };
    c
}

fn named(id: &str, name: &str, kind: CardKind, level: u8, color: CardColor) -> CardData {
    let mut card = base(id, kind, level, color);
    card.card_name = name.into();
    card
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    let mut hybrid = base("TEST-HYBRID", CardKind::Digimon, 5, CardColor::Red);
    hybrid.traits = vec!["Hybrid".into()];
    hybrid.evo_costs = vec![EvoCost { card_color: 0, level: 4, memory_cost: 3 }];
    let mut wrong = hybrid.clone();
    wrong.card_id = "TEST-WRONG-ROUTE".into();
    wrong.card_name = "Wrong route".into();
    wrong.evo_costs[0].level = 5;
    let mut expensive = hybrid.clone();
    expensive.card_id = "TEST-EXPENSIVE".into();
    expensive.card_name = "Expensive".into();
    expensive.evo_costs[0].memory_cost = 25;
    let mut plain = hybrid.clone();
    plain.card_id = "TEST-NON-HYBRID".into();
    plain.card_name = "Plain".into();
    plain.traits = vec!["Ultimate".into()];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT17-012 production YAML")
        .add_card(hybrid)
        .add_card(wrong)
        .add_card(expensive)
        .add_card(plain)
        .add_card(base("TEST-CARRIER", CardKind::Digimon, 5, CardColor::Red))
        .add_card(base("TEST-FILL", CardKind::Tamer, 0, CardColor::White))
        .deck(0, &["TEST-FILL"; 30])
        .deck(1, &["TEST-FILL"; 30])
        .security(1, &["TEST-FILL"; 5])
        .memory(10)
}

fn choose(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection_view().expect("legal player choice");
    assert!(p.valid_action_ids.contains(&a) || (a == PASS && p.is_optional));
    assert!(build_action_mask(&r.game, p.selecting_player)[a as usize] > 0.5);
    r.execute_action(p.selecting_player, a).unwrap();
}

fn attack(r: &mut DebugRunner) {
    r.game.players[0].battle_area[0].is_suspended = false;
    r.attack_player(PermanentHandle { player: 0, index: 0 }, 1, false);
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

#[test]
fn bt17_012_all_printed_routes_and_negative_cases() {
    let mut cases = vec![
        // Printed circles: Red Lv.3 / Purple Lv.3, cost 3.
        (base("TEST-RED3", CardKind::Digimon, 3, CardColor::Red), Some(3)),
        (base("TEST-PURPLE3", CardKind::Digimon, 3, CardColor::Purple), Some(3)),
        (base("TEST-BLUE3", CardKind::Digimon, 3, CardColor::Blue), None),
        (base("TEST-RED4", CardKind::Digimon, 4, CardColor::Red), None),
        // "as if that card is a level 3 red Digimon": any red Tamer, cost 3.
        (base("TEST-RED-TAMER", CardKind::Tamer, 0, CardColor::Red), Some(3)),
        (base("TEST-BLUE-TAMER", CardKind::Tamer, 0, CardColor::Blue), None),
        // [Takuya Kanbara]: Cost 2 (named Tamer, any colour).
        (named("TEST-TAKUYA-WHITE", "Takuya Kanbara", CardKind::Tamer, 0, CardColor::White), Some(2)),
        (named("TEST-TOMMY", "Tommy, Takuya, & Zoe", CardKind::Tamer, 0, CardColor::White), None),
        // [Agunimon]: Cost 1 (a Digimon, any level / colour).
        (named("TEST-AGUNIMON", "Agunimon", CardKind::Digimon, 4, CardColor::Green), Some(1)),
        (named("TEST-OTHERMON", "Agunimon X", CardKind::Digimon, 4, CardColor::Green), None),
    ];
    let mut alias = base("TEST-TAKUYA-ALIAS", CardKind::Tamer, 0, CardColor::White);
    alias.also_treated_as = vec!["Takuya Kanbara".into()];
    cases.push((alias, Some(2)));
    for (card, cost) in cases {
        let id = card.card_id.clone();
        let mut r = builder().add_card(card).hand(0, &[CARD_ID]).start();
        let h = r.place_on_field(0, &id, Some(0));
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize] > 0.5, cost.is_some(), "{id}");
        if let Some(cost) = cost {
            r.game.decode_action(a, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 10 - cost, "{id}");
            assert_eq!(
                r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
                CARD_ID,
                "{id}"
            );
        }
    }
}

#[test]
fn bt17_012_red_takuya_exposes_native_and_treated_named_cost() {
    // A red [Takuya Kanbara] satisfies both the named route and the "as if a
    // level 3 red Digimon" route; both cost 2 here (the official Q&A treats the
    // Tamer as a Digimon that digivolves), so the player picks the identity.
    let mut takuya = base("TEST-TAKUYA", CardKind::Tamer, 0, CardColor::Red);
    takuya.card_name = "Takuya Kanbara".into();
    let mut r = builder().add_card(takuya).hand(0, &[CARD_ID]).start();
    r.place_on_field(0, "TEST-TAKUYA", Some(0));
    r.game.decode_action(encode_digivolve(0, 0), 0);
    let p = r.pending_selection_view().expect("route identity choices");
    let choices = p.effect_choices.expect("route choices");
    assert!(choices.iter().any(|c| c.label == "Digivolve for cost 2"));
    let treated = choices
        .iter()
        .find(|c| c.label.starts_with("Digivolve for cost 2 (treat base"))
        .expect("treated-as-Digimon named route");
    choose(&mut r, treated.action_id);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 8);
}

#[test]
fn bt17_012_red_tamer_route_is_offered_from_hand_only() {
    // The generic Tamer route needs this physical card in hand.
    let mut r = builder()
        .add_card(base("TEST-RED-TAMER", CardKind::Tamer, 0, CardColor::Red))
        .hand(0, &[CARD_ID])
        .start();
    let h = r.place_on_field(0, "TEST-RED-TAMER", Some(0));
    let incoming = r.game.players[0].hand.remove(0);
    r.game.players[0].trash.push(incoming);
    let success = r.game.effect_initiated_digivolve_from_source(
        0,
        digimon_engine::enums::CardSourceRef::Trash(0, 0),
        h,
        digimon_engine::enums::CostDelta::Reduce(0),
        false,
        digimon_engine::enums::PlaySource::ByEffect,
    );
    assert!(!success, "a trash copy can't use the hand-only Tamer route");
}

// ─── <Raid> ──────────────────────────────────────────────────────────────────

#[test]
fn bt17_012_has_raid_face_up() {
    let mut r = builder().start();
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid));
}

// ─── [When Attacking] ────────────────────────────────────────────────────────

fn accept_outer(r: &mut DebugRunner) {
    let p = r.pending_selection_view().expect("the [When Attacking] outer prompt");
    assert!(p.is_optional, "'may digivolve': the outer prompt is declinable");
    let a = *p.valid_action_ids.iter().find(|a| **a != PASS).expect("an accept action");
    choose(r, a);
}

#[test]
fn bt17_012_attack_evolution_masks_filters_and_pays_reduced_cost() {
    let mut r = builder()
        .hand(0, &["TEST-HYBRID", "TEST-NON-HYBRID", "TEST-WRONG-ROUTE", "TEST-EXPENSIVE"])
        .start();
    r.place_on_field(0, CARD_ID, Some(0));
    attack(&mut r);
    accept_outer(&mut r);
    let p = r.pending_selection_view().expect("the [When Attacking] hand pick");
    assert_eq!(p.kind, SelectionKind::Hand);
    assert!(p.is_optional, "the pick is declinable too (DCGO canNoSelect)");
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    assert!(p.valid_action_ids.contains(&0));
    for invalid in [1, 2, 3] {
        assert!(!p.valid_action_ids.contains(&invalid), "hand slot {invalid} must not be offered");
        assert_eq!(build_action_mask(&r.game, 0)[invalid as usize], 0.0);
    }
    choose(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 10 - 2, "printed cost 3 reduced by 1");
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
        "TEST-HYBRID"
    );
    // 4 cards - the Hybrid digivolved from hand + 1 drawn by the digivolution.
    assert_eq!(r.hand_size(0), 4);
}

#[test]
fn bt17_012_declining_the_outer_prompt_or_the_pick_keeps_the_card_unevolved() {
    for decline_outer in [true, false] {
        let mut r = builder().hand(0, &["TEST-HYBRID"]).start();
        r.place_on_field(0, CARD_ID, Some(0));
        attack(&mut r);
        if decline_outer {
            let p = r.pending_selection_view().expect("outer prompt");
            assert!(p.is_optional);
            choose(&mut r, PASS);
        } else {
            accept_outer(&mut r);
            let p = r.pending_selection_view().expect("hand pick");
            assert!(p.is_optional);
            choose(&mut r, PASS);
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 10);
        assert_eq!(
            r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
            CARD_ID
        );
        assert_eq!(r.hand_size(0), 1);
    }
}

#[test]
fn bt17_012_outer_prompt_is_offered_for_any_hybrid_digimon_in_hand_even_if_none_can_digivolve() {
    // DCGO's CanActivate only counts Hybrid Digimon cards in hand; the legality
    // filter belongs to the inner pick. A Hybrid whose route doesn't fit and one
    // the reduced cost can't pay are both "in hand", so the yes/no is shown.
    let mut r = builder().hand(0, &["TEST-WRONG-ROUTE", "TEST-EXPENSIVE"]).start();
    r.place_on_field(0, CARD_ID, Some(0));
    attack(&mut r);
    accept_outer(&mut r);
    assert!(
        r.pending_selection_view().map_or(true, |p| p.kind != SelectionKind::Hand),
        "no card can digivolve, so no hand pick is offered"
    );
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 10);
}

#[test]
fn bt17_012_without_a_hybrid_digimon_in_hand_nothing_is_offered() {
    let mut r = builder().hand(0, &["TEST-NON-HYBRID"]).start();
    r.place_on_field(0, CARD_ID, Some(0));
    attack(&mut r);
    assert!(r.pending_selection_view().is_none(), "no Hybrid Digimon card in hand: not activatable");
}

// ─── Inherited [Your Turn] +2000 DP ──────────────────────────────────────────

#[test]
fn bt17_012_inherited_gives_plus_2000_dp_on_your_turn_only() {
    let mut r = builder().start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    r.game.turn_player_idx = 0;
    let host = r.place_stack(0, &[CARD_ID, "TEST-CARRIER"]);
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(12000), "10000 + 2000 on your turn");
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(10000), "[Your Turn] only");
}

#[test]
fn bt17_012_face_up_has_no_dp_bonus() {
    let mut r = builder().start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.current_phase = GamePhase::Main;
    r.game.turn_player_idx = 0;
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(h), Some(6000), "the +2000 is inherited-only");
}
