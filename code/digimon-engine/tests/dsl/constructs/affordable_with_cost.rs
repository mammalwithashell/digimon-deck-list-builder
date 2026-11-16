//! `affordable_with_cost: <CostDelta>` (G-DSL-AFFORDABLE-WITH-COST) and the
//! DUAL pricing of `affordable_with_cost_reduce`.
//!
//! A paid "play / use 1 ... from your hand with the cost reduced by <X>" pick
//! offers only cards the verb can pay for, written with the same `CostDelta`
//! the verb carries: a formula reduction (AD1-019 "for every 2 of your
//! Tamers' colours", BT26-090 "for each memory your opponent has", EX13-043
//! "4 + suspended Digimon"), a literal fixed cost, `free` or `printed`. DCGO
//! filters the pick the same way: AD1_019.cs:56 `CanPlayAsNewPermanent(card,
//! true, activateClass, fixedCost: Math.Max(0, GetCostItself - reduction))`,
//! BT26_090.cs:61 `Owner.MaxMemoryCost >= GetCostItself - reduceCost`. An
//! unpayable pick would be an action that can't be carried out (CR 4.3
//! 1-3-2). A DUAL card is used, never played (CR 4.3 4-6-2), so it is priced
//! at its Option use cost.
use digimon_engine::{
    action::space::{PASS, PLAY_HAND_START},
    card_data::{CardData, DualCardData, DualDigimonFace, DualOptionFace},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, GamePhase},
    permanent::PermanentHandle,
};

/// `{reduce: <controller's memory>}`: 3 with the fixture's 3 memory.
const FORMULA: &str = "{reduce: {base: 0, per: {player_memory: {of: you}}, delta: 1}}";

fn yaml(filter_cost: &str) -> String {
    format!(
        r#"
card: AFFORD-SOURCE
name: Afford source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter: {{{filter_cost}}}
          prompt: Play 1 card from your hand with the cost reduced
      - if:
          condition: {{binding_present: chosen}}
          then:
            - play_from_hand: {{of: you, hand_index: chosen, cost_delta: {FORMULA}}}
"#
    )
}

fn plain(id: &str, kind: CardKind, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.play_cost = cost;
    c.colors = vec![CardColor::White];
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    c
}

/// A DUAL whose stored play cost (20) differs from its Option use cost (5),
/// so the test tells the two prices apart.
fn dual() -> CardData {
    let mut c = make_test_card("AF-DUAL", "AF-DUAL");
    c.card_kind = CardKind::Dual;
    c.play_cost = 20;
    c.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 6,
            dp: 9000,
            colors: vec![CardColor::White],
            traits: vec![],
            evo_costs: vec![],
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: vec![],
        },
        option: DualOptionFace {
            use_cost: 5,
            colors: vec![CardColor::White],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: vec![],
        },
    });
    c
}

/// Hand: 0 Digimon cost 16, 1 Digimon cost 17, 2 Option cost 16, 3 Option
/// cost 17, 4 DUAL (use cost 5), 5 Digimon cost 5. Memory 3, so the payable
/// maximum is 13 (floor -10).
fn fixture(yaml: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(plain("AF-D16", CardKind::Digimon, 16))
        .add_card(plain("AF-D17", CardKind::Digimon, 17))
        .add_card(plain("AF-O16", CardKind::Option, 16))
        .add_card(plain("AF-O17", CardKind::Option, 17))
        .add_card(dual())
        .add_card(plain("AF-D05", CardKind::Digimon, 5))
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["AF-D16"; 20])
        .deck(1, &["AF-D16"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    for id in ["AF-D16", "AF-D17", "AF-O16", "AF-O17", "AF-DUAL", "AF-D05"] {
        r.add_to_hand(0, id);
    }
    let source = r.place_on_field(0, "AFFORD-SOURCE", None);
    (r, source)
}

fn offered(filter: &str) -> Vec<u16> {
    let (mut r, source) = fixture(&yaml(filter));
    r.fire_on_play(source.player, source.index as usize);
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| id - PLAY_HAND_START)
        .collect()
}

#[test]
fn formula_reduction_offers_exactly_the_payable_cards() {
    assert_eq!(
        offered(&format!("affordable_with_cost: {FORMULA}")),
        vec![0, 2, 4, 5],
        "16 - 3 = 13 is payable from 3 memory, 17 - 3 = 14 is not; the DUAL costs 5 - 3"
    );
}

#[test]
fn literal_reduction_matches_the_formula_of_the_same_value() {
    assert_eq!(offered("affordable_with_cost: {reduce: 3}"), vec![0, 2, 4, 5]);
}

#[test]
fn reduce_leaf_prices_a_dual_at_its_use_cost() {
    // Before the fix the DUAL was priced at its stored play cost (20).
    assert_eq!(offered("affordable_with_cost_reduce: 3"), vec![0, 2, 4, 5]);
}

#[test]
fn fixed_free_and_printed_costs_follow_the_verb_semantics() {
    assert_eq!(offered("affordable_with_cost: 13"), vec![0, 1, 2, 3, 4, 5], "fixed 13 for all");
    assert!(offered("affordable_with_cost: 14").is_empty(), "fixed 14 exceeds the floor");
    assert_eq!(offered("affordable_with_cost: free"), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(offered("affordable_with_cost: printed"), vec![4, 5], "5 is payable, 16/17 are not");
}

#[test]
fn false_polarity_selects_the_unpayable_cards() {
    assert_eq!(offered(&format!("not: {{affordable_with_cost: {FORMULA}}}")), vec![1, 3]);
}

#[test]
fn commit_pays_the_same_reduced_cost_the_filter_priced() {
    let (mut r, source) = fixture(&yaml(&format!("affordable_with_cost: {FORMULA}")));
    r.fire_on_play(source.player, source.index as usize);
    r.execute_action(0, PLAY_HAND_START + 5).unwrap();
    let _ = r.auto_resolve();
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "AF-D05"),
        "the cost-5 Digimon was played"
    );
    assert_eq!(r.memory(), 1, "3 - (5 - 3)");
}
