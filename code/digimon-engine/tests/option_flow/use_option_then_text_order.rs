//! Pre-existing engine gap G-ENGINE-USED-OPTION-FINISHES-AFTER-THEN-TEXT
//! (logged in `docs/RUST_ENGINE_GAPS.md`; not fixed here).
//!
//! An effect reads "You may use 1 Option card from your hand without paying
//! the cost. Then, X". When the used Option's [Main] parks a selection, the
//! using effect's "Then, X" runs before the used Option is finished (trashed
//! or offered <Arts Digivolve>). DCGO finishes the used Option inside
//! `CardController.UseOption` before control returns to the using effect
//! (Script/CardController.cs:2026-2129), and CR 4.3 9-1-5 places the Option
//! in the trash once its effect has resolved, so X should already see the
//! used Option in the trash.
//!
//! The fixture: a Digimon whose [On Play] uses 1 Option card from the hand
//! and then may return 1 Option card from the trash to the hand. The used
//! Option's [Main] makes the player choose 1 opposing Digimon to delete (a
//! parked selection). In DCGO the return pick lists the used Option.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

const USER_YAML: &str = r#"
card: P-USER
name: Option User
kind: digimon
level: 3
color: [green]
cost: 0
dp: 2000
traits: []
effects:
  - text: "[On Play] You may use 1 Option card from your hand without paying the cost. Then, you may return 1 Option card from your trash to the hand."
    when: on_play
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
          prompt: "Use 1 Option card from your hand"
      - select_trash:
          of: you
          bind_as: back
          optional: true
          filter: { kind: option }
          prompt: "Return 1 Option card from your trash to the hand"
      - add_to_hand_from_trash: { of: you, card: back }
"#;

const OPTION_YAML: &str = r#"
card: P-OPT
name: Pick Option
kind: option
color: [green]
cost: 1
effects:
  - text: "[Main] Delete 1 of your opponent's Digimon."
    when: main
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: false
          filter: { kind: digimon }
          prompt: "Delete 1 of your opponent's Digimon"
      - delete_permanent: { target: victim }
"#;

fn in_trash(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

/// Drives the line. `Some(in_trash)` when "Then, X" asked its question
/// (whether the used Option was in the trash at that moment); `None` when X
/// had no Option card in the trash to offer, so it asked nothing.
fn then_text_view_of_the_used_option(opponent_has_a_digimon: bool) -> Option<bool> {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(USER_YAML)
        .expect("user")
        .from_dsl_yaml(OPTION_YAML)
        .expect("option")
        .add_card(make_test_card("OPP", "Opp"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(0, &["P-USER", "P-OPT"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    if opponent_has_a_digimon {
        r.place_on_field(1, "OPP", None);
    }
    r.game.enter_main_phase();
    let _ = r.play(0, 0);
    // Accept the use, choose P-OPT, then answer its [Main] pick.
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else {
            break;
        };
        if view.prompt.contains("Return 1 Option card") {
            return Some(in_trash(&r, "P-OPT"));
        }
        let a = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != digimon_engine::action::space::PASS)
            .expect("a non-pass answer");
        r.execute_action(view.selecting_player, a).expect("answer");
    }
    assert!(
        r.pending_selection().is_none(),
        "no other decision is left; kind {:?}",
        r.pending_kind()
    );
    // The return pick was never offered: the trash had no Option card when
    // "Then, X" ran.
    assert_eq!(r.battle_area_size(1), 0, "the Option's [Main] deleted the target");
    assert!(opponent_has_a_digimon, "without a parked pick X asks its question");
    assert!(in_trash(&r, "P-OPT"), "the used Option ends in the trash");
    None
}

/// What the engine does today (the gap): "Then, X" runs before the used
/// Option is trashed, so X finds no Option card in the trash and asks
/// nothing; the Option reaches the trash afterwards.
#[test]
fn today_then_text_runs_before_the_used_option_is_trashed() {
    assert_eq!(then_text_view_of_the_used_option(true), None);
}

/// Control: when the used Option's [Main] parks nothing (no opposing Digimon
/// to choose), the Option is finished before "Then, X", as in DCGO.
#[test]
fn without_a_parked_pick_then_text_sees_the_used_option_in_the_trash() {
    assert_eq!(then_text_view_of_the_used_option(false), Some(true));
}

/// DCGO's order (CardController.cs:2026-2129; CR 4.3 9-1-5). Red until the
/// gap is fixed.
#[test]
#[ignore = "G-ENGINE-USED-OPTION-FINISHES-AFTER-THEN-TEXT (open)"]
fn dcgo_then_text_sees_the_used_option_in_the_trash() {
    assert_eq!(then_text_view_of_the_used_option(true), Some(true));
}
