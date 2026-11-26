//! Source: `data/card_bundles/BT27-069.md` (printed text and official Q&A).
//!
//! <Rush> <Overclock ([Chessmon] in name)> <Blocker>
//! [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
//! [All Turns] [Once Per Turn] When any of your other Digimon with [Chessmon]
//! in their names are deleted, delete 1 of your opponent's lowest play cost
//! Digimon. If this effect didn't delete, trash their top security card.
//! Assembly -5: Lv.5 x Lv.4 x Lv.3, all w/[Chessmon] in name.
//! Q&A: with any opposing Digimon you must delete one of the lowest play cost.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-069";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("BLK5", "Black", 5, 7, &[]), CardColor::Black))
        .add_card(colored(mon("YEL5", "Yellow", 5, 7, &[]), CardColor::Yellow))
        .add_card(colored(mon("GRN5", "Green", 5, 7, &[]), CardColor::Green))
        .add_card(colored(mon("KN5", "KnightChessmon", 5, 7, &[]), CardColor::Red))
        .add_card(colored(mon("KN4", "PawnChessmon", 4, 5, &[]), CardColor::Red))
        .add_card(colored(mon("KN3", "LittleChessmon", 3, 3, &[]), CardColor::Red))
        .add_card(colored(mon("SQ5", "Squad", 5, 7, &["DATA SQUAD"]), CardColor::Red))
        .add_card(mon("O3", "Op3", 3, 3, &[]))
        .add_card(mon("O4", "Op4", 4, 5, &[]))
        .add_card(mon("O5", "Op5", 5, 7, &[]))
        .add_card(mon("O6", "Op6", 6, 12, &[]))
        .add_card(mon("F", "Fill", 3, 4, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["F"; 3])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

fn opponent_deletes(r: &mut DebugRunner, target: PermanentHandle) {
    use digimon_engine::effect_context::EffectContext;
    let src = r.place_on_field(1, "O6", Some(0));
    let card = r.top_card(src);
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(src), 1);
        ctx.delete_permanent(target);
    }
    r.game.set_effect_source_player_for_test(None);
    r.game.drain_effect_queue();
}

fn assert_mandatory_pick(r: &DebugRunner, mut want: Vec<&str>) {
    let v = r.pending_selection_view().expect("a target prompt");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional);
    let mut c = candidates(r);
    c.sort();
    want.sort();
    assert_eq!(c, want);
}

#[test]
fn printed_keywords() {
    let mut r = runner();
    let h = r.place_stack(0, &["BLK5", CARD]);
    for k in [Keyword::Rush, Keyword::Overclock, Keyword::Blocker] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

#[test]
fn digivolves_for_4_from_black_or_yellow_and_for_3_from_chessmon_or_data_squad() {
    for (base, cost) in [("BLK5", 4), ("YEL5", 4), ("KN5", 3), ("SQ5", 3)] {
        let mut r = runner();
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), cost, "{base}");
    }
    let mut r = runner();
    let b = r.place_on_field(0, "GRN5", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn assembly_reduces_the_play_cost_by_5_with_lv5_lv4_lv3_chessmon_from_trash() {
    let mut r = runner();
    for id in ["KN5", "KN4", "KN3", "O4"] {
        r.inject_trash(0, id);
    }
    let mem = r.memory();
    r.game.decode_action(PLAY_HAND_START + hand_index(&r, 0, CARD) as u16, 0);
    while let Some(v) = r.pending_selection_view() {
        assert!(v.prompt.contains("Assembly"), "{}", v.prompt);
        assert!(!candidates(&r).contains(&"O4".to_string()), "only [Chessmon] materials");
        let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
        r.game.decode_action(a, 0);
    }
    let h = PermanentHandle { player: 0, index: 0 };
    assert_eq!(top_id(&r, h), CARD);
    let mut s = source_ids(&r, h);
    s.sort();
    assert_eq!(s, vec!["KN3", "KN4", "KN5"]);
    assert_eq!(mem - r.memory(), 7, "12 - 5");
}

#[test]
fn on_play_and_when_digivolving_de_digivolve_3() {
    let mut r = runner();
    let tall = r.place_stack(1, &["O3", "O4", "O5", "O6"]);
    play_hand(&mut r, CARD);
    assert_mandatory_pick(&r, vec!["O6"]);
    r.choose_prompt_card("O6", None).unwrap();
    assert!(r.pending_is_count_declaration(), "the player declares up to 3");
    assert_eq!(r.pending_selection_view().unwrap().selecting_player, 0);
    assert!(r.declare_max_count().unwrap());
    assert_eq!(top_id(&r, tall), "O3");
    assert!(source_ids(&r, tall).is_empty());

    let mut r = runner();
    let tall = r.place_stack(1, &["O4", "O5", "O6"]);
    let b = r.place_on_field(0, "BLK5", Some(0));
    assert!(digivolve_hand(&mut r, CARD, b));
    r.choose_prompt_card("O6", None).unwrap();
    r.declare_max_count().unwrap();
    assert_eq!(top_id(&r, tall), "O4", "a lone card is never trashed");
}

#[test]
fn other_chessmon_deletion_must_delete_a_lowest_cost_opponent_digimon() {
    let mut r = runner();
    r.place_on_field(0, CARD, Some(0));
    let pawn = r.place_on_field(0, "KN4", Some(0));
    r.place_on_field(1, "O3", Some(0));
    r.place_on_field(1, "O3", Some(0));
    r.place_on_field(1, "O5", Some(0));
    opponent_deletes(&mut r, pawn);
    assert_mandatory_pick(&r, vec!["O3", "O3"]);
    let a = r.pending_selection_view().unwrap().valid_action_ids[0];
    r.game.decode_action(a, 0);
    let mut f = field_ids(&r, 1);
    f.sort();
    assert_eq!(f, vec!["O3", "O5", "O6"]);
    assert_eq!(r.security_count(1), 3, "it deleted, so no security trash");
}

#[test]
fn if_it_did_not_delete_the_top_security_card_is_trashed_and_it_is_once_per_turn() {
    let mut r = runner();
    r.place_on_field(0, CARD, Some(0));
    let p1 = r.place_on_field(0, "KN4", Some(0));
    let p2 = r.place_on_field(0, "KN3", Some(0));
    use digimon_engine::effect_context::EffectContext;
    let card = r.top_card(p1);
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, card, None, 1);
        ctx.delete_permanent(p1);
    }
    r.game.set_effect_source_player_for_test(None);
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none(), "no opposing Digimon to pick");
    assert_eq!(r.security_count(1), 2);
    assert_eq!(trash_ids(&r, 1), vec!["F"]);
    let p2 = PermanentHandle { player: 0, index: field_ids(&r, 0).iter().position(|x| x == "KN3").unwrap() as u8 };
    let _ = p2;
    opponent_deletes(&mut r, p2);
    assert!(r.pending_selection().is_none(), "once per turn");
    assert_eq!(r.security_count(1), 2);
    assert_eq!(field_ids(&r, 1), vec!["O6"]);
}

#[test]
fn non_chessmon_or_its_own_deletion_does_not_trigger() {
    let mut r = runner();
    r.place_on_field(0, CARD, Some(0));
    let other = r.place_on_field(0, "SQ5", Some(0));
    r.place_on_field(1, "O3", Some(0));
    opponent_deletes(&mut r, other);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn overclock_costs_only_other_chessmon() {
    let mut r = runner();
    r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "KN4", Some(0));
    r.place_on_field(0, "SQ5", Some(0));
    r.game.end_turn();
    let m = build_action_mask(&r.game, 0);
    assert_eq!(m[encode_attack(0, 1) as usize], 1.0, "[Chessmon] ally is a legal cost");
    assert_eq!(m[encode_attack(0, 2) as usize], 0.0, "non-[Chessmon] is not");
    assert_eq!(m[PASS as usize], 1.0, "Overclock is optional");
}
