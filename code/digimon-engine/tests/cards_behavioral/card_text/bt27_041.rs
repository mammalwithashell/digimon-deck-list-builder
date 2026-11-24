//! Source: `data/card_bundles/BT27-041.md` (printed text).
//!
//! <Barrier> <Ascension>
//! [On Play] [When Digivolving] Until your opponent's turn ends, give 1 of
//! their Digimon <Security A. -1>, and for each of your Digimon with [Chessmon]
//! in their names, -3000 DP.
//! Inherited: [All Turns] [Once Per Turn] When any of your other Digimon with
//! [Chessmon] in their names are deleted, 1 of your opponent's Digimon gets
//! -5000 DP for the turn.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-041";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn big() -> CardData {
    let mut c = mon("OPP", "Opp", 6, 12, &[]);
    c.dp = Some(20000);
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("YEL4", "Yellow", 4, 5, &[]), CardColor::Yellow))
        .add_card(colored(mon("BLK4", "Black", 4, 5, &[]), CardColor::Black))
        .add_card(colored(mon("GRN4", "Green", 4, 5, &[]), CardColor::Green))
        .add_card(colored(mon("PAWN", "PawnChessmon", 4, 5, &[]), CardColor::Red))
        .add_card(colored(mon("SQ4", "Squad", 4, 5, &["DATA SQUAD"]), CardColor::Red))
        .add_card(big())
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

fn pick_opp(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("target prompt");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional);
    assert!(candidates(r).iter().any(|c| c == "OPP"));
    r.choose_prompt_card("OPP", None).unwrap();
    assert!(r.game.players[1].battle_area.get(h.index as usize).is_some());
}

#[test]
fn printed_keywords() {
    let mut r = runner();
    let h = r.place_stack(0, &["YEL4", CARD]);
    assert!(r.game.has_keyword(h, Keyword::Barrier));
    assert!(r.game.has_keyword(h, Keyword::Ascension));
}

#[test]
fn digivolves_for_3_from_yellow_black_chessmon_or_data_squad_level_4() {
    for base in ["YEL4", "BLK4", "PAWN", "SQ4"] {
        let mut r = runner();
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), 3, "{base}");
    }
    let mut r = runner();
    let b = r.place_on_field(0, "GRN4", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b));
}

#[test]
fn on_play_counts_itself_and_other_chessmon_for_the_dp_reduction() {
    for (pawns, minus) in [(0, 3000), (2, 9000)] {
        let mut r = runner();
        for _ in 0..pawns {
            r.place_on_field(0, "PAWN", Some(0));
        }
        r.place_on_field(0, "SQ4", Some(0));
        let o = r.place_on_field(1, "OPP", Some(0));
        let dp = r.effective_dp(o).unwrap();
        play_hand(&mut r, CARD);
        pick_opp(&mut r, o);
        assert_eq!(dp - r.effective_dp(o).unwrap(), minus, "{pawns} other Chessmon");
    }
}

#[test]
fn when_digivolving_gives_security_a_minus_1_and_dp_until_opponent_turn_ends() {
    let mut r = runner();
    let b = r.place_on_field(0, "YEL4", Some(0));
    let o = r.place_on_field(1, "OPP", Some(0));
    let dp = r.effective_dp(o).unwrap();
    assert!(digivolve_hand(&mut r, CARD, b));
    pick_opp(&mut r, o);
    assert_eq!(dp - r.effective_dp(o).unwrap(), 3000);
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    r.game.memory = 10;
    let o = PermanentHandle { player: 1, index: field_ids(&r, 1).iter().position(|x| x == "OPP").unwrap() as u8 };
    assert_eq!(dp - r.effective_dp(o).unwrap(), 3000, "lasts through their turn");
    r.attack_player(o, 0, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), 3, "<Security A. -1>: no security check");
    r.end_turn();
    assert_eq!(r.effective_dp(o).unwrap(), dp, "expired after their turn");
}

#[test]
fn no_opponent_digimon_means_no_prompt() {
    let mut r = runner();
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ascension_may_place_it_as_the_top_security_card_when_deleted() {
    for accept in [true, false] {
        let mut r = runner();
        let me = r.place_stack(0, &["YEL4", CARD]);
        opponent_deletes(&mut r, me);
        let v = r.pending_selection_view().expect("you may place");
        assert_eq!(v.selecting_player, 0);
        assert_eq!(v.effect_choices.unwrap().len(), 2, "Yes / No");
        choose_branch(&mut r, usize::from(!accept));
        let _ = r.auto_resolve();
        let top = r.game.players[0].security.last().unwrap().card_id(&r.game.card_data).to_string();
        assert_eq!(r.security_count(0), if accept { 4 } else { 3 });
        assert_eq!(top == CARD, accept);
    }
}

#[test]
fn inherited_other_chessmon_deletion_gives_minus_5000_once_per_turn() {
    let mut r = runner();
    r.place_stack(0, &[CARD, "SQ4"]);
    let p1 = r.place_on_field(0, "PAWN", Some(0));
    let o = r.place_on_field(1, "OPP", Some(0));
    let dp = r.effective_dp(o).unwrap();
    opponent_deletes(&mut r, p1);
    pick_opp(&mut r, o);
    assert_eq!(dp - r.effective_dp(o).unwrap(), 5000);
    let p2 = r.place_on_field(0, "PAWN", Some(0));
    opponent_deletes(&mut r, p2);
    assert!(r.pending_selection().is_none(), "once per turn");
    assert_eq!(dp - r.effective_dp(o).unwrap(), 5000);
    r.end_turn();
    assert_eq!(r.effective_dp(o).unwrap(), dp, "for the turn");
}

#[test]
fn inherited_ignores_non_chessmon_deletions() {
    let mut r = runner();
    r.place_stack(0, &[CARD, "SQ4"]);
    let other = r.place_on_field(0, "YEL4", Some(0));
    r.place_on_field(1, "OPP", Some(0));
    opponent_deletes(&mut r, other);
    assert!(r.pending_selection().is_none());
}
