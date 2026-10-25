//! BT18-077 KaiserLeomon. Official observation a1e821b4f6f6b8f8746a938fa59b19fba2badc0ffb49f797f8d4039bfa79dc67.
//! Image includes purple/yellow Lv.3 AND Tamer cost-4 circles omitted by the metadata projection.
//! CR 4.3 16-13-1..3: Retaliation triggers only when just its carrier loses battle.
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};

fn digi(id: &str, name: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.dp = Some(dp);
    c.colors = vec![color];
    c
}
fn tamer(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}
fn runner() -> DebugRunner {
    let mut alias = tamer("TEST-KOICHI-ALIAS", "Combined Tamer", CardColor::Blue);
    alias.also_treated_as = vec!["Koichi Kimura".to_string()];
    DebugRunner::builder()
        .dsl_card("BT18-077")
        .expect("BT18-077 production YAML")
        .add_card(digi("TEST-LOW", "Low", 3, 3000, CardColor::Purple))
        .add_card(digi("TEST-LOW4", "Low Four", 4, 6000, CardColor::Yellow))
        .add_card(digi("TEST-HIGH", "High", 5, 9000, CardColor::Blue))
        .add_card(digi("TEST-EQUAL", "Equal", 5, 7000, CardColor::Blue))
        .add_card(digi(
            "TEST-LOWEEMON",
            "Loweemon",
            4,
            5000,
            CardColor::Purple,
        ))
        .add_card(digi(
            "TEST-YELLOW",
            "Yellow Low",
            3,
            3000,
            CardColor::Yellow,
        ))
        .add_card(digi("TEST-OFF", "Off Color", 3, 3000, CardColor::Blue))
        .add_card(tamer("TEST-TAMER", "Other Tamer", CardColor::Yellow))
        .add_card(tamer("TEST-KOICHI", "Koichi Kimura", CardColor::Purple))
        .add_card(alias)
        .add_card(tamer("TEST-BLUE-TAMER", "Blue Tamer", CardColor::Blue))
        .hand(0, &["BT18-077"])
        .deck(0, &["TEST-LOW"; 20])
        .deck(1, &["TEST-LOW"; 20])
        .memory(10)
        .start()
}
fn finish_target(r: &mut DebugRunner, target: u16) {
    let v = r
        .pending_selection_view()
        .expect("mandatory deletion selection");
    assert!(!v.valid_action_ids.contains(&PASS));
    let action = encode_attack(0, target);
    assert!(v.valid_action_ids.contains(&action));
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.execute_action(0, action).expect("choose deletion");
    r.auto_resolve().expect("deletion resolves");
}
#[test]
fn bt18_077_on_play_mandatory_mask_excludes_high_levels_and_tamers() {
    let mut r = runner();
    for id in ["TEST-LOW", "TEST-LOW4", "TEST-HIGH", "TEST-TAMER"] {
        r.place_on_field(1, id, Some(0));
    }
    r.play(0, 0).expect("play KaiserLeomon");
    let v = r.pending_selection_view().expect("deletion choices");
    assert_eq!(
        v.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)]
    );
    finish_target(&mut r, 1);
    assert_eq!(r.game.players[1].battle_area.len(), 3);
    assert!(r.game.players[1]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEST-LOW4"));
}
#[test]
fn bt18_077_no_eligible_opponent_does_not_offer_impossible_selection() {
    let mut r = runner();
    r.place_on_field(1, "TEST-HIGH", Some(0));
    r.place_on_field(1, "TEST-TAMER", Some(0));
    r.play(0, 0).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].battle_area.len(), 2);
}
#[test]
fn bt18_077_when_digivolving_deletes_selected_low_level_digimon() {
    let mut r = runner();
    r.place_on_field(0, "TEST-LOW", Some(0));
    r.place_on_field(1, "TEST-LOW4", Some(0));
    let a = encode_digivolve(0, 0);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.game.decode_action(a, 0);
    finish_target(&mut r, 0);
    assert!(r.game.players[1].battle_area.is_empty());
}
fn evolve_cost(base: &str, expected: Option<i16>) {
    let mut r = runner();
    r.place_on_field(0, base, Some(0));
    r.game.current_phase = GamePhase::Main;
    let a = encode_digivolve(0, 0);
    let legal = build_action_mask(&r.game, 0)[a as usize] > 0.5;
    assert_eq!(legal, expected.is_some(), "base {base}");
    if let Some(cost) = expected {
        let before = r.memory();
        r.game.decode_action(a, 0);
        if let Some(view) = r.pending_selection_view() {
            if let Some(choices) = view.effect_choices {
                let chosen = choices
                    .iter()
                    .find(|entry| entry.label == format!("Digivolve for cost {cost}"))
                    .expect("printed route cost offered");
                assert!(!view.valid_action_ids.contains(&PASS));
                r.execute_action(0, chosen.action_id).unwrap();
            }
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), before - cost, "base {base}");
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "BT18-077"
        );
    }
}
#[test]
fn bt18_077_printed_level_three_both_color_routes_cost_four() {
    evolve_cost("TEST-LOW", Some(4));
    evolve_cost("TEST-YELLOW", Some(4));
}
#[test]
fn bt18_077_printed_tamer_route_and_named_routes_preserve_costs() {
    evolve_cost("TEST-TAMER", Some(4));
    evolve_cost("TEST-KOICHI", Some(3));
    evolve_cost("TEST-KOICHI-ALIAS", Some(3));
    evolve_cost("TEST-LOWEEMON", Some(1));
}
#[test]
fn bt18_077_unrelated_off_color_digimon_and_tamer_cannot_evolve() {
    evolve_cost("TEST-OFF", None);
    evolve_cost("TEST-BLUE-TAMER", None);
}
fn retaliation_battle(inherited: bool, equal: bool) {
    let mut r = runner();
    let h = if inherited {
        r.place_stack(0, &["BT18-077", "TEST-EQUAL"])
    } else {
        r.place_on_field(0, "BT18-077", Some(0))
    };
    assert!(r.game.has_keyword(h, Keyword::Retaliation));
    let d = r.place_on_field(1, if equal { "TEST-EQUAL" } else { "TEST-HIGH" }, Some(0));
    r.place_on_field(1, "TEST-LOW", Some(0));
    r.game.players[1].battle_area[d.index as usize].is_suspended = true;
    let a = encode_attack(h.index as u16, d.index as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.5);
    r.game.decode_action(a, 0);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0].battle_area.is_empty());
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-LOW",
        "unbattled neighbor remains"
    );
    let opp_trash = r.game.players[1].trash.len();
    assert_eq!(opp_trash, 1, "exactly one opposing battle card is trashed");
}
#[test]
fn bt18_077_retaliation_top_deletes_stronger_battle_opponent() {
    retaliation_battle(false, false);
}
#[test]
fn bt18_077_retaliation_inherited_deletes_stronger_battle_opponent() {
    retaliation_battle(true, false);
}
#[test]
fn bt18_077_retaliation_does_not_redelete_on_mutual_battle_loss() {
    retaliation_battle(false, true);
}

#[test]
fn bt18_077_effect_deletion_does_not_retaliate_against_an_opponent() {
    for inherited in [false, true] {
        let mut r = runner();
        let h = if inherited {
            r.place_stack(0, &["BT18-077", "TEST-EQUAL"])
        } else {
            r.place_on_field(0, "BT18-077", Some(0))
        };
        r.place_on_field(1, "TEST-HIGH", Some(0));
        r.game.delete_permanent_with_cause(
            h,
            digimon_engine::replacement::ReplacementCause::OpponentEffect,
        );
        r.auto_resolve().unwrap();
        assert!(r.game.players[0].battle_area.is_empty());
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert!(r.game.players[1].trash.is_empty());
    }
}
