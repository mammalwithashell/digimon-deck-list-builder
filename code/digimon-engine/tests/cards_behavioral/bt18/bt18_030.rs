//! BT18-030 Candlemon: printed image / official bundle; DCGO BT18_030.cs.
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT18-030";

#[test]
fn bt18_030_overlapping_buckets_preserve_every_legal_physical_choice() {
    // Q2953 makes available category picks mandatory. Q4704 clarifies that
    // an overlapping card may fill the first category even if this leaves
    // no unchosen match for the second. Do not optimize the player's choice.
    for first in ["DUAL", "DATA"] {
        let mut r = builder()
            .deck(0, &["PLAIN", "DATA", "DUAL"])
            .hand(0, &[ID])
            .memory(10)
            .start();
        r.play(0, 0).unwrap();
        let mask = digimon_engine::action::build_action_mask(&r.game, 0);
        assert_eq!(mask[reveal_action(&r, "DUAL") as usize], 1.0);
        assert_eq!(mask[reveal_action(&r, "DATA") as usize], 1.0);
        assert_eq!(mask[PASS as usize], 0.0, "available category is mandatory");
        pick(&mut r, first);
        if first == "DATA" {
            pick(&mut r, "DUAL");
        }
        r.auto_resolve().unwrap();
        let hand: Vec<_> = r.game.players[0].hand.iter()
            .map(|c| c.card_id(&r.game.card_data)).collect();
        assert_eq!(hand.iter().filter(|id| **id == "DUAL").count(), 1);
        assert_eq!(hand.len(), if first == "DATA" { 2 } else { 1 });
    }
}

fn card(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer_carrier() -> CardData {
    let mut c = card("TAMER", CardColor::Yellow, &["Witchelny", "Data"]);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("production Candlemon")
        .add_card(card("DUAL", CardColor::Yellow, &["Data", "Witchelny"]))
        .add_card(card("DATA", CardColor::Yellow, &["Data"]))
        .add_card(card("WITCH", CardColor::Purple, &["Witchelny"]))
        .add_card(card("YELLOW-WITCH", CardColor::Yellow, &["Witchelny"]))
        .add_card(card("WRONG-COLOR", CardColor::Red, &["Data"]))
        .add_card(card("PLAIN", CardColor::Yellow, &["Vaccine"]))
        .add_card(card("S1", CardColor::Yellow, &[]))
        .add_card(card("S2", CardColor::Yellow, &[]))
        .add_card(tamer_carrier())
}

fn reveal_action(r: &DebugRunner, id: &str) -> u16 {
    SEL_REVEAL_START
        + r.game
            .revealed_cards
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .expect("revealed") as u16
}

fn pick(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("mandatory reveal bucket");
    let a = reveal_action(r, id);
    assert!(!v.valid_action_ids.contains(&PASS));
    assert!(v.valid_action_ids.contains(&a));
    let mask = digimon_engine::action::build_action_mask(&r.game, v.selecting_player);
    assert_eq!(mask[PASS as usize], 0.0);
    assert_eq!(mask[a as usize], 1.0);
    r.execute_action(v.selecting_player, a).unwrap();
}

#[test]
fn bt18_030_reveal_requires_both_buckets_and_never_reuses_a_physical_card() {
    let mut r = builder()
        .deck(0, &["DUAL", "WITCH", "DATA"])
        .hand(0, &[ID])
        .memory(10)
        .start();
    r.play(0, 0).unwrap();
    pick(&mut r, "DUAL");
    let v = r.pending_selection_view().expect("Witchelny bucket");
    assert!(!v.valid_action_ids.contains(&reveal_action(&r, "DUAL")));
    assert!(!v.valid_action_ids.contains(&reveal_action(&r, "DATA")));
    pick(&mut r, "WITCH");
    r.auto_resolve().unwrap();
    let hand: Vec<_> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert_eq!(hand.len(), 2);
    assert!(hand.contains(&"DUAL") && hand.contains(&"WITCH"));
    assert_eq!(r.game.players[0].deck.len(), 1);
    assert_eq!(r.game.players[0].deck[0].card_id(&r.game.card_data), "DATA");
}

#[test]
fn bt18_030_reveal_excludes_non_yellow_data_and_returns_all_nonmatches() {
    let mut r = builder()
        .deck(0, &["WRONG-COLOR", "PLAIN", "DATA"])
        .hand(0, &[ID])
        .memory(10)
        .start();
    r.play(0, 0).unwrap();
    let v = r.pending_selection_view().expect("Data bucket");
    assert!(!v
        .valid_action_ids
        .contains(&reveal_action(&r, "WRONG-COLOR")));
    assert!(!v.valid_action_ids.contains(&reveal_action(&r, "PLAIN")));
    pick(&mut r, "DATA");
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.game.players[0].deck.len(), 2);
}

fn inherited(carrier: &str, security: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder()
        .security(0, security)
        .deck(0, &["PLAIN"; 6])
        .deck(1, &["PLAIN"; 6])
        .start();
    r.skip_mulligan();
    let h = r.place_stack(0, &[ID, carrier]);
    (r, h)
}

fn accept(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("save offered");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.is_optional);
    let mask = digimon_engine::action::build_action_mask(&r.game, v.selecting_player);
    assert_eq!(mask[PASS as usize], 1.0);
    assert!(v.valid_action_ids.iter().any(|a| *a != PASS));
    for a in &v.valid_action_ids {
        assert_eq!(mask[*a as usize], 1.0);
    }
    r.accept_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
}

#[test]
fn bt18_030_inherited_saves_yellow_data_pays_top_security_and_is_once_per_turn() {
    let (mut r, h) = inherited("DATA", &["S1", "S2"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    accept(&mut r);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.security_count(0), 1);
    assert_eq!(
        r.game.players[0].security[0].card_id(&r.game.card_data),
        "S1"
    );
    assert_eq!(
        r.game.players[0]
            .trash
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "S2"
    );
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn bt18_030_inherited_accepts_yellow_witchelny_without_data() {
    let (mut r, h) = inherited("YELLOW-WITCH", &["S1"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    accept(&mut r);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
}

#[test]
fn bt18_030_inherited_filters_color_trait_cause_and_unpayable_cost() {
    for (carrier, cause, security) in [
        ("WITCH", ReplacementCause::OpponentEffect, vec!["S1"]),
        ("WRONG-COLOR", ReplacementCause::OpponentEffect, vec!["S1"]),
        ("PLAIN", ReplacementCause::OpponentEffect, vec!["S1"]),
        ("DATA", ReplacementCause::OwnEffect, vec!["S1"]),
        ("DATA", ReplacementCause::Battle, vec!["S1"]),
        ("DATA", ReplacementCause::OpponentEffect, vec![]),
    ] {
        let (mut r, h) = inherited(carrier, &security);
        r.game.delete_permanent_with_cause(h, cause);
        assert!(r.pending_selection_view().is_none(), "{carrier}: {cause:?}");
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.security_count(0), security.len());
    }
}

#[test]
fn bt18_030_inherited_decline_pays_nothing_and_does_not_protect_other_digimon() {
    let (mut r, h) = inherited("DATA", &["S1"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.security_count(0), 1);
    let (mut r, _) = inherited("DATA", &["S1"]);
    let other = r.place_on_field(0, "DUAL", Some(0));
    r.game
        .delete_permanent_with_cause(other, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.security_count(0), 1);
}

#[test]
fn bt18_030_inherited_does_not_protect_a_yellow_witchelny_tamer() {
    let (mut r, h) = inherited("TAMER", &["S1"]);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection_view().is_none(),
        "printed carrier must be a Digimon"
    );
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.security_count(0), 1);
}
