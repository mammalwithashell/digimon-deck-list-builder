//! Source: `data/card_bundles/BT27-042.md` (printed text and official Q&A).
//!
//! [On Play] [When Digivolving] You may play 1 play cost 7 or lower Digimon
//! card with [Chessmon] in its name from your hand without paying the cost.
//! [On Deletion] <Recovery +1>. Then, by suspending 1 of your [QueenChessmon],
//! you may play this card from your trash to the breeding area without paying
//! the cost.
//! Assembly -5: Lv.5 x Lv.4 x Lv.3, all w/[Chessmon] in name.

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-042";

fn colored(mut c: CardData, color: CardColor) -> CardData {
    c.colors = vec![color];
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(colored(mon("YEL5", "Yellow", 5, 7, &[]), CardColor::Yellow))
        .add_card(colored(mon("BLK5", "Black", 5, 7, &[]), CardColor::Black))
        .add_card(colored(mon("GRN5", "Green", 5, 7, &[]), CardColor::Green))
        .add_card(colored(mon("KN5", "KnightChessmon", 5, 7, &[]), CardColor::Red))
        .add_card(colored(mon("KN4", "PawnChessmon", 4, 5, &[]), CardColor::Red))
        .add_card(colored(mon("KN3", "LittleChessmon", 3, 3, &[]), CardColor::Red))
        .add_card(colored(mon("BIGCH", "RookChessmon", 6, 8, &[]), CardColor::Red))
        .add_card(colored(mon("SQ5", "Squad", 5, 7, &["DATA SQUAD"]), CardColor::Red))
        .add_card(mon("QC", "QueenChessmon", 6, 12, &[]))
        .add_card(mon("O4", "Op4", 4, 5, &[]))
        .add_card(mon("OPP", "Opp", 6, 12, &[]))
        .add_card(mon("F", "Fill", 3, 4, &[]))
        .add_card(mon("D", "DeckTop", 3, 4, &[]))
        .deck(0, &["D", "F", "F", "F", "F", "F"])
        .deck(1, &["F"; 10])
        .security(0, &["F"; 2])
        .hand(0, &[CARD, "KN5", "BIGCH", "O4"])
        .memory(10)
        .start()
}

fn opponent_deletes(r: &mut DebugRunner, target: PermanentHandle) {
    use digimon_engine::effect_context::EffectContext;
    let src = r.place_on_field(1, "OPP", Some(0));
    let card = r.top_card(src);
    r.game.set_effect_source_player_for_test(Some(1));
    {
        let mut ctx = EffectContext::new(&mut r.game, card, Some(src), 1);
        ctx.delete_permanent(target);
    }
    r.game.set_effect_source_player_for_test(None);
    r.game.drain_effect_queue();
}

fn breeding_id(r: &DebugRunner) -> Option<String> {
    r.game.players[0]
        .breeding_area
        .as_ref()
        .map(|p| ids_of(r, [p.top_card()])[0].clone())
}

fn assert_optional_pick(r: &DebugRunner, mut want: Vec<&str>) {
    let v = r.pending_selection_view().expect("an optional pick");
    assert_eq!(v.selecting_player, 0);
    assert!(v.is_optional);
    let mut c = candidates(r);
    c.sort();
    want.sort();
    assert_eq!(c, want);
}

#[test]
fn digivolves_for_3_from_yellow_black_chessmon_or_data_squad() {
    for base in ["YEL5", "BLK5", "KN5", "SQ5"] {
        let mut r = runner();
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), 3, "{base}");
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
        if !v.prompt.contains("Assembly") {
            break;
        }
        assert!(!candidates(&r).contains(&"O4".to_string()), "only [Chessmon] materials");
        let a = *v.valid_action_ids.iter().find(|a| **a != PASS).unwrap();
        r.game.decode_action(a, 0);
    }
    let h = PermanentHandle { player: 0, index: 0 };
    assert_eq!(top_id(&r, h), CARD);
    let mut s = source_ids(&r, h);
    s.sort();
    assert_eq!(s, vec!["KN3", "KN4", "KN5"]);
    assert_eq!(mem - r.memory(), 8, "13 - 5");
}

#[test]
fn on_play_may_play_a_cost_7_or_lower_chessmon_from_hand_for_free() {
    let mut r = runner();
    play_hand(&mut r, CARD);
    assert_optional_pick(&r, vec!["KN5"]);
    r.choose_prompt_card("KN5", None).unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD, "KN5"]);
    assert!(!hand_ids(&r, 0).contains(&"KN5".to_string()));
}

#[test]
fn on_play_may_decline() {
    let mut r = runner();
    play_hand(&mut r, CARD);
    r.decline_prompt().unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD]);
    assert!(hand_ids(&r, 0).contains(&"KN5".to_string()));
}

#[test]
fn when_digivolving_also_offers_the_free_chessmon_play() {
    let mut r = runner();
    let b = r.place_on_field(0, "YEL5", Some(0));
    assert!(digivolve_hand(&mut r, CARD, b));
    assert_optional_pick(&r, vec!["KN5"]);
    let mem = r.memory();
    r.choose_prompt_card("KN5", None).unwrap();
    assert_eq!(field_ids(&r, 0), vec![CARD, "KN5"]);
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

#[test]
fn on_deletion_recovers_then_suspending_queenchessmon_plays_it_to_breeding() {
    let mut r = runner();
    let me = r.place_on_field(0, CARD, Some(0));
    let queen = r.place_on_field(0, "QC", Some(0));
    let tired = r.place_on_field(0, "QC", Some(0));
    r.game.players[0].battle_area[tired.index as usize].is_suspended = true;
    opponent_deletes(&mut r, me);
    assert_eq!(r.security_count(0), 3, "<Recovery +1>");
    let v = r.pending_selection_view().expect("Queen pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.iter().filter(|a| **a != PASS).count(), 1, "only an unsuspended QueenChessmon");
    r.choose_prompt_card("QC", None).unwrap();
    while r.pending_selection().is_some() {
        r.choose_prompt_card(CARD, None).unwrap();
    }
    let queen = PermanentHandle { player: 0, index: field_ids(&r, 0).iter().position(|x| x == "QC").unwrap() as u8 };
    let _ = queen;
    assert!(r.game.players[0].battle_area.iter().all(|p| p.is_suspended));
    assert_eq!(breeding_id(&r).as_deref(), Some(CARD));
    assert!(!trash_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn on_deletion_decline_leaves_it_in_trash_and_queen_unsuspended() {
    let mut r = runner();
    let me = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "QC", Some(0));
    opponent_deletes(&mut r, me);
    assert_eq!(r.security_count(0), 3);
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert!(breeding_id(&r).is_none());
    assert!(trash_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn on_deletion_queen_that_cannot_suspend_pays_nothing_and_plays_nothing() {
    let mut r = runner();
    let me = r.place_on_field(0, CARD, Some(0));
    let queen = r.place_on_field(0, "QC", Some(0));
    r.game.modifiers.add(
        queen,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotSuspend,
            0,
            digimon_engine::enums::Expiry::Permanent,
            1,
        ),
    );
    opponent_deletes(&mut r, me);
    assert_eq!(r.security_count(0), 3);
    assert!(r.pending_is_optional());
    r.choose_prompt_card("QC", None).unwrap();
    assert!(r.pending_selection().is_none(), "the suspension cost was not paid");
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    assert!(breeding_id(&r).is_none());
    assert!(trash_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn on_deletion_without_queenchessmon_only_recovers() {
    let mut r = runner();
    let me = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "BIGCH", Some(0));
    opponent_deletes(&mut r, me);
    assert_eq!(r.security_count(0), 3);
    assert!(r.pending_selection().is_none(), "no [QueenChessmon], no prompt");
    assert!(trash_ids(&r, 0).contains(&CARD.to_string()));
}

#[test]
fn on_deletion_with_an_occupied_breeding_area_cannot_play_it() {
    let mut r = runner();
    r.place_in_breeding(0, "F");
    let me = r.place_on_field(0, CARD, Some(0));
    r.place_on_field(0, "QC", Some(0));
    opponent_deletes(&mut r, me);
    if r.pending_selection().is_some() {
        r.choose_prompt_card("QC", None).unwrap();
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(breeding_id(&r).as_deref(), Some("F"));
    assert!(trash_ids(&r, 0).contains(&CARD.to_string()));
}
