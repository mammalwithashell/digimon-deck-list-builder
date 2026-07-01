//! Integration tests for the `SelectPlayOrder` phase used by best-of-three
//! match training. Between games of a BO3 match the Python `MatchEnv` wrapper
//! calls `Game::request_play_order_selection(loser_id)`, the loser picks
//! `PLAY_FIRST` (action 94) or `PLAY_SECOND` (action 95), and the engine
//! records the choice in `last_play_order_choice` for the wrapper to read.
//!
//! Contract:
//! - `request_play_order_selection` installs a `SelectionKind::PlayOrder`
//!   prompt with `selecting_player = loser_id`, `valid_action_ids = [94, 95]`,
//!   `is_optional = false`, and switches `current_phase` to `SelectPlayOrder`.
//! - The action mask reports 94 and 95 legal for the chooser, and NO other
//!   actions legal except those (no PASS, no concede — CONCEDE_GAME (93) is
//!   disabled in the action mask in all phases/formats as of 2026-06-19; the
//!   `Game::concede` primitive itself is still tested in `concede_primitive.rs`
//!   and backs human PvP surrender).
//! - Resolution via the action interface (94 → First, 95 → Second) or via
//!   `Game::resolve_play_order_selection(picked)` both write to
//!   `last_play_order_choice`.
//! - Outside the phase, actions 94 and 95 are NOT in the action mask.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{CONCEDE_GAME, PLAY_FIRST, PLAY_SECOND};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{PlayOrder, SelectionKind};

#[test]
fn request_installs_play_order_selection_with_chooser_set() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;

    runner.game.request_play_order_selection(1);

    assert_eq!(runner.game.current_phase, GamePhase::SelectPlayOrder);
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending_selection must be installed");
    assert!(matches!(sel.kind, SelectionKind::PlayOrder));
    assert_eq!(sel.selecting_player, 1, "loser is the chooser");
    assert!(!sel.is_optional, "play-order pick is mandatory");
    assert_eq!(
        sel.valid_action_ids,
        vec![PLAY_FIRST, PLAY_SECOND],
        "exactly two legal actions in fixed order"
    );
}

#[test]
fn mask_reports_play_first_and_play_second_legal_for_chooser_only() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(0);

    // Chooser sees the two actions.
    let chooser_mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        chooser_mask[PLAY_FIRST as usize], 1.0,
        "PLAY_FIRST legal for chooser"
    );
    assert_eq!(
        chooser_mask[PLAY_SECOND as usize], 1.0,
        "PLAY_SECOND legal for chooser"
    );

    // Non-chooser sees nothing (selection is gated on selecting_player).
    let other_mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        other_mask[PLAY_FIRST as usize], 0.0,
        "PLAY_FIRST not legal for non-chooser"
    );
    assert_eq!(
        other_mask[PLAY_SECOND as usize], 0.0,
        "PLAY_SECOND not legal for non-chooser"
    );
}

#[test]
fn mask_does_not_expose_play_order_actions_outside_phase() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_FIRST as usize], 0.0,
        "PLAY_FIRST illegal in Main phase"
    );
    assert_eq!(
        mask[PLAY_SECOND as usize], 0.0,
        "PLAY_SECOND illegal in Main phase"
    );
}

#[test]
fn mask_does_not_expose_concede_during_select_play_order() {
    // CONCEDE_GAME is intentionally legal at every agent decision point
    // EXCEPT Mulligan and SelectPlayOrder. Conceding mid-match between
    // games of a BO3 should be a strategic decision the agent learns by
    // playing PLAY_FIRST / PLAY_SECOND — not a phase where a random-init
    // policy can degenerately pick CONCEDE_GAME and forfeit the whole
    // match. (Prior behavior degraded BO3 evals to 0% win-rate with
    // random init; see add-gameplay-reward-config smoke verification.)
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(0);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[CONCEDE_GAME as usize], 0.0,
        "CONCEDE_GAME MUST NOT be legal during SelectPlayOrder \
         (forfeit-via-random-init was making BO3 evals degenerate)"
    );
    // Sanity: the real choices stay legal.
    assert_eq!(mask[PLAY_FIRST as usize], 1.0);
    assert_eq!(mask[PLAY_SECOND as usize], 1.0);
}

#[test]
fn mask_does_not_expose_concede_during_mulligan() {
    // Mulligan is the pre-game keep/redraw decision — conceding before
    // the game has started is semantically degenerate and was another
    // random-init degeneracy path. Gate CONCEDE_GAME out during Mulligan.
    //
    // `concede_primitive.rs` shows the established pattern: build a
    // started runner, then forcibly set `current_phase = Mulligan`.
    // The mask code only consults `current_phase` + `mulligan_current_player`
    // for the Mulligan branch, so this is sufficient.
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::Mulligan;
    runner.game.mulligan_pending = vec![0, 1];

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[CONCEDE_GAME as usize], 0.0,
        "CONCEDE_GAME MUST NOT be legal during Mulligan \
         (pre-game forfeit is degenerate)"
    );
    // Sanity: at least one keep/mulligan action is legal so the mask
    // isn't all-zeros (which would mask CONCEDE out trivially).
    assert!(
        mask[0] > 0.0 || mask[1] > 0.0,
        "Mulligan decider should have keep (0) or mulligan (1) legal",
    );
}

#[test]
fn mask_never_exposes_concede_in_main() {
    // CONCEDE_GAME (93) is disabled in the action mask in ALL phases/formats
    // (2026-06-19) — including Main, the agent's primary decision point with
    // real actions legal. Conceding is a strictly-dominated "give up" with no
    // strategic value (single forfeits the episode; BO3 games can always be
    // played out), and RL policies abused it as premature surrender. Human PvP
    // surrender uses `Game::concede` directly, bypassing this mask.
    use digimon_engine::debug_runner::make_test_card_with_level;
    let atk = make_test_card_with_level("ATK-LV3", "Attacker", 3);
    let mut runner = DebugRunner::builder().add_card(atk).start();
    runner.place_on_field(0, "ATK-LV3", None);
    runner.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[CONCEDE_GAME as usize], 0.0,
        "CONCEDE_GAME (93) must never be legal in Main — concede is disabled \
         in all formats; the mulligan test above proves concede stays 0 even \
         when other actions are legal.",
    );
}

#[test]
fn resolve_via_action_first_records_first() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);

    runner
        .game
        .resolve_selection(1, PLAY_FIRST)
        .expect("resolve PLAY_FIRST");

    assert_eq!(
        runner.game.last_play_order_choice,
        Some(PlayOrder::First),
        "First action records First"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "selection cleared after resolution"
    );
}

#[test]
fn resolve_via_action_second_records_second() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(0);

    runner
        .game
        .resolve_selection(0, PLAY_SECOND)
        .expect("resolve PLAY_SECOND");

    assert_eq!(
        runner.game.last_play_order_choice,
        Some(PlayOrder::Second),
        "Second action records Second"
    );
}

#[test]
fn resolve_via_convenience_method_records_choice() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);

    runner
        .game
        .resolve_play_order_selection(PlayOrder::Second)
        .expect("convenience resolve");

    assert_eq!(runner.game.last_play_order_choice, Some(PlayOrder::Second));
    assert!(runner.game.pending_selection.is_none());
}

#[test]
fn play_order_selection_clones_faithfully() {
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);

    assert!(
        runner.game.pending_selection_resume.is_some(),
        "SelectPlayOrder must park a data resume frame so cloned games do not hit the callback panic-stub"
    );

    let mut clone = runner.game.clone();
    clone
        .resolve_selection(1, PLAY_FIRST)
        .expect("clone resolves PLAY_FIRST through the data frame");

    assert_eq!(clone.last_play_order_choice, Some(PlayOrder::First));
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must not clear the original prompt"
    );
    assert_eq!(
        runner.game.last_play_order_choice, None,
        "resolving the clone must not write the original choice slot"
    );

    runner
        .game
        .resolve_selection(1, PLAY_FIRST)
        .expect("original resolves the same choice");
    assert_eq!(
        runner.game.last_play_order_choice, clone.last_play_order_choice,
        "original and clone replay identically from the same play-order choice"
    );
    assert_eq!(runner.game.current_phase, clone.current_phase);
}

#[test]
fn resolve_convenience_errors_without_pending_play_order() {
    let mut runner = DebugRunner::builder().start();
    // No pending selection installed.
    assert!(runner
        .game
        .resolve_play_order_selection(PlayOrder::First)
        .is_err());
}

#[test]
fn last_play_order_choice_starts_none() {
    let runner = DebugRunner::builder().start();
    assert_eq!(runner.game.last_play_order_choice, None);
}

#[test]
fn resolve_does_not_implicitly_reset_choice_slot() {
    // The wrapper takes the value; the engine does NOT auto-reset between
    // selections. This pins that contract.
    let mut runner = DebugRunner::builder().start();
    runner.game.current_phase = GamePhase::GameOver;
    runner.game.request_play_order_selection(1);
    runner
        .game
        .resolve_selection(1, PLAY_FIRST)
        .expect("resolve");

    assert_eq!(runner.game.last_play_order_choice, Some(PlayOrder::First));

    // Wrapper "takes" the value by setting None.
    runner.game.last_play_order_choice = None;
    assert_eq!(runner.game.last_play_order_choice, None);
}
