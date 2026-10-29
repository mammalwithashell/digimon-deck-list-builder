//! BT26-082 Ravemon — Lv.6 Purple, Cyborg/DATA SQUAD/(Rule) Birdkin.
//!
//! {Security} [End of Opponent's Turn] Play this card without paying the cost.
//! [When Digivolving] [End of Attack] By deleting this Digimon or trashing 2
//! bottom face-down cards from under any of your Tamers, delete 1 of your
//! opponent's highest DP Digimon.
//! [On Deletion] Your opponent trashes 1 card in their hand. Then, if their
//! hand has 7 or fewer cards, you may place this card face up as the bottom
//! security card.
//!
//! DCGO: BT26/Purple/BT26_082.cs. Official Q&A: {Security} works only while the
//! card is FACE UP in the security stack (rule 15-14-5).

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-082";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-082")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("BIG", "Big", CardColor::Red, 6, 12, &[]))
        .add_card(digimon("SMALL", "Small", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("JUNK", "Junk", CardColor::Red, 3, 3, &[]))
}

fn setup() -> DebugRunner {
    let mut r = builder()
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_082_has_birdkin_rule_trait() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    assert!(format!("{:?}", card.traits).contains("Birdkin"));
}

#[test]
fn bt26_082_wd_delete_self_deletes_highest_dp() {
    let mut r = setup();
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    // No Tamer face-down cards ⇒ the 2-way modal {Delete this | Don't pay}.
    r.execute_branch(0).expect("delete this Digimon");
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Ravemon deleted as the cost"
    );
    assert_eq!(
        field_ids(&r, 1),
        vec!["SMALL".to_string()],
        "the highest-DP Digimon was deleted"
    );
}

#[test]
fn bt26_082_wd_trash_two_face_down_keeps_ravemon() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    r.place_on_field(1, "SMALL", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    r.execute_branch(1).expect("trash 2 face-down cards");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "2 face-down cards trashed");
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(field_ids(&r, 1), vec!["SMALL".to_string()]);
}

#[test]
fn bt26_082_wd_dont_pay() {
    let mut r = setup();
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "BIG", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, rave);
    r.execute_branch(1).expect("don't pay (2-way modal)");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(field_ids(&r, 1), vec!["BIG".to_string()]);
}

#[test]
fn bt26_082_on_deletion_discard_then_face_up_bottom_security() {
    let mut r = setup();
    push_hand(&mut r, 1, "JUNK");
    let rave = r.place_on_field(0, CARD_ID, Some(0));
    let sec_before = r.security_count(0);
    // Delete it for real so the card is in the trash when [On Deletion] runs.
    r.game.delete_permanents_batch(
        vec![rave],
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    pick_hand(&mut r, 1, "JUNK");
    r.execute_branch(0)
        .expect("place face up as bottom security");

    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec_before + 1);
    let bottom = &r.game.players[0].security[0];
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        CARD_ID,
        "placed at the bottom"
    );
    assert!(
        r.game.players[0]
            .face_up_security
            .contains(&bottom.card_index),
        "face up"
    );
}

#[test]
fn bt26_082_face_up_security_plays_itself_at_end_of_opponents_turn() {
    let mut r = builder()
        .security(0, &[CARD_ID, "FILLER"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    r.end_turn();
    assert!(
        r.game.players[0].battle_area.is_empty(),
        "waits for the OPPONENT's turn end"
    );
    r.end_turn();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.security_count(0), 1);
}

/// A red Lv.4 5000 DP Digimon with <Engage> (the rules-conformance fixture).
const ENGAGE_YAML: &str = r#"
card: ENGAGE
name: Engagemon
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
effects:
  - kind: grant_keyword
    keyword: Engage
    summary: "<Engage> (At the end of your turn, this Digimon may attack.)"
"#;

/// Official Q&A Q7122 (2026-08-18): "An opponent's Digimon attacks the player
/// with an effect at the end of their turn. What happens if this card's
/// {Security} [End of Opponent's Turn] effect then plays this card, my
/// security stack is reduced to 0 cards, and the attack against the player is
/// successful?" -- "You lose the game."
///
/// [End of Your Turn] and [End of Opponent's Turn] both trigger when the turn
/// ends (CR 4.3 §15-16-12-1/-2, §15-8-3-6): one simultaneous pool, the turn
/// player's effects first, then the non-turn player's (§15-4-3-5). P0's
/// <Engage> (§16-44-2) declares its attack inside that pool, and the counter
/// timing waits until every triggered effect has resolved (§11-1-4), so P1's
/// face-up Ravemon plays itself before the attack reaches its security check.
/// With 0 security cards the successful attack on the player wins the game.
#[test]
fn bt26_082_q7122_engage_attack_after_security_play_wins_the_game() {
    let mut r = builder()
        .from_dsl_yaml(ENGAGE_YAML)
        .expect("ENGAGE fixture compiles")
        .security(1, &[CARD_ID])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    r.set_first_player(0);
    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    let idx = r.game.players[1].security[0].card_index;
    r.game.players[1].face_up_security.insert(idx);

    r.end_turn();
    let attack = digimon_engine::action::space::encode_attack(
        engage.index as u16,
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert!(
        r.pending_selection()
            .is_some_and(|sel| sel.selecting_player == 0 && sel.valid_action_ids.contains(&attack)),
        "<Engage> offers its attack inside the end-of-turn pool: {:?}",
        r.pending_selection()
    );
    // §15-4-3-5: the turn player's effects activate first, so the non-turn
    // player's Ravemon is still pending (in security) at the declaration.
    assert!(
        !field_ids(&r, 1).contains(&CARD_ID.to_string()),
        "Ravemon has not played itself before the turn player's <Engage> activates"
    );
    assert_eq!(r.security_count(1), 1);
    r.game.decode_action(attack, 0);
    for _ in 0..12 {
        let Some(sel) = r.pending_selection() else { break };
        if sel.is_optional {
            r.decline_optional_trigger().expect("decline an optional prompt");
        } else {
            let (player, act) = (sel.selecting_player, sel.valid_action_ids[0]);
            r.execute_action(player, act).expect("take the first pick");
        }
    }

    assert!(
        field_ids(&r, 1).contains(&CARD_ID.to_string()),
        "Ravemon's {{Security}} [End of Opponent's Turn] play resolved before the counter timing (§11-1-4)"
    );
    assert_eq!(r.security_count(1), 0);
    assert!(r.game.game_over, "Q7122: the successful attack on a player with 0 security ends the game");
    assert_eq!(r.game.winner, Some(0), "Q7122: the player whose Ravemon was played loses");
    // Game over is terminal: the end of turn the attack interrupted does not
    // resume, so no turn change follows the win.
    assert_eq!(
        r.game.current_phase,
        digimon_engine::enums::GamePhase::GameOver,
        "the end of turn does not continue after the game ended"
    );
    assert_eq!(r.game.turn_player(), 0, "the turn never passes to P1");
}


#[test]
fn bt26_082_face_down_security_does_nothing() {
    let mut r = builder()
        .security(0, &[CARD_ID, "FILLER"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    r.end_turn();
    r.end_turn();
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "{{Security}} needs face up (15-14-5)"
    );
    assert_eq!(r.security_count(0), 2);
}

// qa:Q7121: shuffling security turns every card face down.
#[test]
fn shuffle_security_turns_all_cards_face_down_only_for_shuffled_player() {
    for player in 0..2 {
        let mut r = builder()
            .security(0, &[CARD_ID, "FILLER", CARD_ID])
            .security(1, &[CARD_ID, "FILLER", CARD_ID])
            .start();
        for p in &mut r.game.players {
            for card in &p.security {
                if card.card_id(&r.game.card_data) == CARD_ID {
                    p.face_up_security.insert(card.card_index);
                }
            }
        }
        let other = 1 - player;
        let other_faces = r.game.players[other].face_up_security.clone();
        let before: Vec<_> = r
            .game
            .players
            .iter()
            .map(|p| p.security.iter().map(|c| c.card_index).collect::<Vec<_>>())
            .collect();

        r.game.shuffle_security(player as u8);

        assert!(r.game.players[player].face_up_security.is_empty());
        assert_eq!(r.game.players[other].face_up_security, other_faces);
        let after: Vec<_> = r
            .game
            .players
            .iter()
            .map(|p| p.security.iter().map(|c| c.card_index).collect::<Vec<_>>())
            .collect();
        assert_eq!(after[other], before[other], "other stack is unchanged");
        let mut expected = before[player].clone();
        let mut actual = after[player].clone();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(actual, expected, "shuffle preserves every card instance");
    }
}

#[test]
fn bt26_082_q7121_tk_shuffle_prevents_security_revival() {

    let mut r = builder()
        .dsl_card("BT1-087")
        .expect("T.K. Takaishi")
        .security(
            0,
            &[CARD_ID, "JUNK", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .start();
    r.set_first_player(0);
    // State after Ravemon's On Deletion placement: five security plus Ravemon.
    let ravemon = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(ravemon);
    let tk = r.place_on_field(0, "BT1-087", Some(0));
    let deck_before = r.game.players[0].deck.len();
    fire(&mut r, EffectTiming::OnPlay, tk);
    // Select a non-yellow card using the real DSL/VM security selection.
    r.execute_action(0, physical_menu::security_action(&r.game, 0, 1))
        .expect("pick non-yellow JUNK");
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "JUNK"));
    assert_eq!(r.game.players[0].deck.len(), deck_before, "no recovery");
    assert_eq!(r.security_count(0), 5);

    r.end_turn();
    r.end_turn();
    assert_eq!(
        r.security_count(0),
        5,
        "Q7121: shuffled Ravemon cannot play itself"
    );
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert!(r.game.players[0].face_up_security.is_empty());
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_menu;
