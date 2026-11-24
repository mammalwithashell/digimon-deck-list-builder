//! Source: `data/card_bundles/BT27-066.md` (printed text).
//!
//! <Reboot> <Blocker>
//! [On Play] [On Deletion] To 1 of your opponent's Digimon, <De-Digivolve 1>
//! for each of your other Digimon with [Chessmon] in their names. Then, delete
//! 1 of your opponent's Digimon with a play cost of 5 or less.
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to this Digimon.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-066";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("BLK4", "Black", 4, 5, &[]), CardColor::Black))
        .add_card(colored(mon("GRN4", "Green", 4, 5, &[]), CardColor::Green))
        .add_card(colored(mon("PAWN", "PawnChessmon", 4, 5, &[]), CardColor::Red))
        .add_card(colored(mon("SQ4", "Squad", 4, 5, &["DATA SQUAD"]), CardColor::Red))
        .add_card(mon("O3", "Op3", 3, 3, &[]))
        .add_card(mon("O4", "Op4", 4, 5, &[]))
        .add_card(mon("O5", "Op5", 5, 7, &[]))
        .add_card(mon("O6", "Op6", 6, 12, &[]))
        .add_card(mon("BIG", "Big", 5, 9, &[]))
        .add_card(mon("ATK", "Attacker", 4, 5, &[]))
        .add_card(mon("HOST", "Host", 6, 12, &[]))
        .add_card(mon("F", "Fill", 3, 3, &[]))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(0, &["F"; 3])
        .security(1, &["F"; 3])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

fn opponent_deletes(r: &mut DebugRunner, target: PermanentHandle) {
    use digimon_engine::effect_context::EffectContext;
    let src = r.place_on_field(1, "F", Some(0));
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
    let h = r.place_stack(0, &["BLK4", CARD]);
    assert!(r.game.has_keyword(h, Keyword::Reboot));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
}

#[test]
fn digivolves_for_4_from_black_and_for_3_from_chessmon_or_data_squad() {
    for (base, cost) in [("BLK4", 4), ("PAWN", 3), ("SQ4", 3)] {
        let mut r = runner();
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), cost, "{base}");
    }
    let mut r = runner();
    let b = r.place_on_field(0, "GRN4", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn on_play_de_digivolves_once_per_other_chessmon_then_deletes_a_cost_5_or_lower() {
    let mut r = runner();
    r.place_on_field(0, "PAWN", Some(0));
    r.place_on_field(0, "PAWN", Some(0));
    let tall = r.place_stack(1, &["O3", "O4", "O5", "O6"]);
    r.place_on_field(1, "BIG", Some(0));
    play_hand(&mut r, CARD);
    assert_mandatory_pick(&r, vec!["BIG", "O6"]);
    r.choose_prompt_card("O6", None).unwrap();
    assert_eq!(top_id(&r, tall), "O4", "De-Digivolve 1 twice");
    assert_eq!(source_ids(&r, tall), vec!["O3"]);
    assert_mandatory_pick(&r, vec!["O4"]);
    r.choose_prompt_card("O4", None).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["BIG"]);
}

#[test]
fn on_play_without_other_chessmon_skips_de_digivolve_and_still_deletes() {
    let mut r = runner();
    r.place_on_field(0, "SQ4", Some(0));
    r.place_stack(1, &["O3", "O4", "O5", "O6"]);
    r.place_on_field(1, "O4", Some(0));
    play_hand(&mut r, CARD);
    assert_mandatory_pick(&r, vec!["O4"]);
    r.choose_prompt_card("O4", None).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["O6"]);
}

#[test]
fn on_play_without_a_cost_5_or_lower_target_deletes_nothing() {
    let mut r = runner();
    r.place_on_field(1, "BIG", Some(0));
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
    assert_eq!(field_ids(&r, 1), vec!["BIG"]);
}

#[test]
fn on_deletion_repeats_the_effect_counting_remaining_chessmon() {
    let mut r = runner();
    r.place_on_field(0, "PAWN", Some(0));
    let me = r.place_stack(0, &["BLK4", CARD]);
    let tall = r.place_stack(1, &["O3", "O4", "O5"]);
    opponent_deletes(&mut r, me);
    assert_mandatory_pick(&r, vec!["F", "O5"]);
    r.choose_prompt_card("O5", None).unwrap();
    assert_eq!(top_id(&r, tall), "O4", "De-Digivolve 1 once");
    assert_eq!(source_ids(&r, tall), vec!["O3"]);
    assert_mandatory_pick(&r, vec!["F", "O4"]);
    r.choose_prompt_card("O4", None).unwrap();
    assert_eq!(field_ids(&r, 1), vec!["F"]);
}

fn opponent_turn() -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut r = runner();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    r.game.memory = 10;
    let host = r.place_stack(0, &[CARD, "HOST"]);
    let a1 = r.place_on_field(1, "ATK", Some(0));
    let a2 = r.place_on_field(1, "ATK", Some(0));
    (r, host, a1, a2)
}

#[test]
fn inherited_may_redirect_an_opponent_attack_to_this_digimon() {
    let (mut r, _host, a1, _) = opponent_turn();
    r.attack_player(a1, 0, false);
    assert!(r.pending_is_optional(), "you may change");
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3, "redirected: no security check");
    assert!(field_ids(&r, 0).contains(&"HOST".to_string()));
    assert_eq!(field_ids(&r, 1), vec!["ATK"], "the attacker lost the battle");
}

#[test]
fn inherited_redirect_can_be_declined_and_is_once_per_turn() {
    let (mut r, _host, a1, _) = opponent_turn();
    r.attack_player(a1, 0, false);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);

    let (mut r, _host, a1, a2) = opponent_turn();
    r.attack_player(a1, 0, false);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    let _ = a2;
    let a2 = PermanentHandle { player: 1, index: field_ids(&r, 1).iter().position(|x| x == "ATK").unwrap() as u8 };
    r.attack_player(a2, 0, false);
    assert!(!r.pending_is_optional(), "once per turn");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 2);
}

#[test]
fn top_card_effect_is_not_the_inherited_redirect() {
    let mut r = runner();
    r.end_turn();
    r.game.enter_main_phase();
    r.game.memory = 10;
    r.place_stack(0, &["BLK4", CARD]);
    let a1 = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(a1, 0, false);
    let v = r.pending_selection_view();
    assert!(v.is_none_or(|v| !v.prompt.contains("attack target")), "only <Blocker> may intercept");
}
