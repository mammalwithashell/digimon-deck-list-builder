//! BT22-040 Cendrillmon — Digimon, Lv.6, Yellow.
//! Traits: Puppet, LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of
//! your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player
//! without suspending.)
//! [On Play] [When Digivolving] You may play 1 [Familiar] Token.
//! (Digimon/Yellow/3000 DP/[On Deletion] 1 of your opponent's Digimon gets
//! -3000 DP for the turn.)
//! [All Turns] [Once Per Turn] When any of your other Digimon are deleted,
//! you may activate 1 of this Digimon's [When Digivolving] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_040.cs
//!
//! # Patterns this test covers
//! - H11 Overclock keyword (Puppet/Token cost filter)
//! - E1/E2 optional [On Play] / [When Digivolving] branch choice (Play/Decline)
//! - Event observer: on_any_deletion with event-target owner/kind filters
//! - refire_effect: re-activating this Digimon's own [When Digivolving] effect
//! - [All Turns][OPT] once-per-turn lockout + clear on end of turn
//!
//! # Event-context contract
//!
//! `OnAnyDeletion` must evaluate event-target predicates against the deleted
//! object's pre-removal snapshot — scoping to "your OTHER Digimon" excludes
//! the carrier itself and the opponent's Digimon.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn runner_with_bt22_040() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-040")
        .expect("BT22-040 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_test_card("ALLY-1", "Ally 1"))
        .add_card(make_test_card("ALLY-2", "Ally 2"))
        .add_card(make_test_card("OPPONENT", "Opponent"))
        .add_card(make_test_card("FILLER", "Filler"))
        // Decks and security keep `begin_turn` from tripping a deck-out and
        // ending the game before a turn cycle completes (see the closed gap
        // G-OPT-RESET-VIA-ATTACK-CYCLE — the OPT-reset substrate is correct,
        // but the carrier's `new_turn` must actually run).
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["FILLER"; 5])
        .security(1, &["FILLER"; 5])
        .hand(0, &["BT22-040"])
        .memory(20)
        .start()
}

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

/// Re-resolve a permanent handle by card id. Deleting a permanent shifts the
/// indices of every later permanent in the same battle area, so handles
/// captured before a deletion go stale.
fn handle_by_card_id(
    runner: &DebugRunner,
    player: u8,
    card_id: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be on player {player}'s field"));
    digimon_engine::permanent::PermanentHandle {
        player,
        index: index as u8,
    }
}

#[test]
fn bt22_040_refire_clause_is_all_turns_opt_on_any_deletion() {
    let runner = runner_with_bt22_040();
    let compiled = runner
        .compiled_card("BT22-040")
        .expect("BT22-040 compiled card present");

    let refire = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnAnyDeletion) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-040 must ship an OnAnyDeletion refire observer");

    assert_eq!(
        refire.when,
        vec![CompiledTiming::OnAnyDeletion],
        "the refire clause triggers when any Digimon is deleted"
    );
    assert_eq!(
        refire.scope,
        CompiledScope::FaceUp,
        "the [All Turns] refire is a face-up effect, not inherited"
    );
    assert!(
        refire.once_per_turn,
        "printed [Once Per Turn] makes the refire once-per-turn"
    );
    assert!(
        refire
            .active_when
            .as_ref()
            .is_some_and(|active| active.all_turns == Some(true)),
        "printed [All Turns] means the refire is active on both players' turns"
    );
    assert!(
        refire.condition.is_some(),
        "refire must gate on the deleted object being your OTHER Digimon"
    );
    // The printed "you may activate" optionality is asserted behaviorally —
    // see `bt22_040_refire_can_be_declined` (visible PASS / decline window).
}

#[test]
fn bt22_040_has_puppet_overclock() {
    let mut runner = runner_with_bt22_040();
    let cendrill = runner.place_on_field(0, "BT22-040", Some(0));

    assert!(
        runner.game.has_keyword(cendrill, Keyword::Overclock),
        "BT22-040 grants Overclock while face up"
    );
}

#[test]
fn bt22_040_on_play_may_play_one_familiar_token() {
    let mut runner = runner_with_bt22_040();

    runner.play(0, 0).expect("BT22-040 plays from hand");
    let choice = runner
        .pending_selection_view()
        .expect("On Play token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[0])
        .expect("choose to play Familiar Token");
    runner
        .auto_resolve()
        .expect("accept optional On Play token");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
fn bt22_040_on_play_can_decline_token() {
    let mut runner = runner_with_bt22_040();

    runner.play(0, 0).expect("BT22-040 plays from hand");
    let choice = runner
        .pending_selection_view()
        .expect("On Play token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[1])
        .expect("decline optional On Play token");
    runner.auto_resolve().expect("finish declined token effect");

    assert_eq!(familiar_count(&runner, 0), 0, "declining plays no token");
}

#[test]
fn bt22_040_when_digivolving_may_play_one_familiar_token() {
    let mut runner = runner_with_bt22_040();
    let cendrill = runner.place_stack(0, &["BASE", "BT22-040"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let choice = runner
        .pending_selection_view()
        .expect("When Digivolving token effect says 'you may'");
    assert_eq!(choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, choice.valid_action_ids[0])
        .expect("choose to play Familiar Token");
    runner
        .auto_resolve()
        .expect("accept optional When Digivolving token");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
fn bt22_040_refires_when_digivolving_after_other_own_digimon_deleted() {
    let mut runner = runner_with_bt22_040();
    runner.place_on_field(0, "BT22-040", Some(0));
    let ally = runner.place_on_field(0, "ALLY-1", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    let refire = runner
        .pending_selection_view()
        .expect("other own Digimon deletion should offer optional refire");
    assert_eq!(refire.kind, SelectionKind::EffectChoice);
    assert!(
        runner.pending_is_optional(),
        "refiring BT22-040's When Digivolving effect is optional"
    );
    runner
        .execute_action(0, refire.valid_action_ids[0])
        .expect("accept refire");

    let token_choice = runner
        .pending_selection_view()
        .expect("refired When Digivolving effect should ask whether to play a Familiar Token");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[0])
        .expect("choose to play Familiar Token");
    runner.auto_resolve().expect("finish refired effect");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "accepted refire runs BT22-040's When Digivolving effect"
    );
}

#[test]
fn bt22_040_refire_can_be_declined() {
    let mut runner = runner_with_bt22_040();
    runner.place_on_field(0, "BT22-040", Some(0));
    let ally = runner.place_on_field(0, "ALLY-1", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_is_optional(),
        "other-deletion refire must be a visible may-choice"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline optional refire");
    runner.auto_resolve().expect("finish declined refire");

    assert_eq!(
        familiar_count(&runner, 0),
        0,
        "declining the refire does not run the token effect"
    );
}

#[test]
fn bt22_040_refire_only_triggers_for_other_own_digimon() {
    let mut positive = runner_with_bt22_040();
    positive.place_on_field(0, "BT22-040", Some(0));
    let own_digimon = positive.place_on_field(0, "BASE", Some(0));
    positive
        .game
        .delete_permanent_with_cause(own_digimon, ReplacementCause::OpponentEffect);
    assert!(
        positive.pending_selection_view().is_some(),
        "baseline own other Digimon deletion should trigger"
    );

    let mut opponent_case = runner_with_bt22_040();
    opponent_case.place_on_field(0, "BT22-040", Some(0));
    let opponent = opponent_case.place_on_field(1, "OPPONENT", Some(0));
    opponent_case
        .game
        .delete_permanent_with_cause(opponent, ReplacementCause::OpponentEffect);
    assert!(
        opponent_case.pending_selection_view().is_none(),
        "opponent Digimon deletion must not trigger your BT22-040"
    );

    let mut self_case = runner_with_bt22_040();
    let cendrill = self_case.place_on_field(0, "BT22-040", Some(0));
    self_case
        .game
        .delete_permanent_with_cause(cendrill, ReplacementCause::OpponentEffect);
    assert!(
        self_case.pending_selection_view().is_none(),
        "printed 'other Digimon' must not trigger from BT22-040 itself leaving"
    );
}

#[test]
fn bt22_040_other_deletion_refire_is_once_per_turn() {
    let mut runner = runner_with_bt22_040();
    runner.place_on_field(0, "BT22-040", Some(0));
    let ally1 = runner.place_on_field(0, "ALLY-1", Some(0));
    runner.place_on_field(0, "ALLY-2", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally1, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection_view().is_some(),
        "first other own Digimon deletion should trigger"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline first optional refire");
    runner.auto_resolve().expect("finish first refire");

    // ALLY-1's deletion shifted the battle area — re-resolve ALLY-2.
    let ally2 = handle_by_card_id(&runner, 0, "ALLY-2");
    runner
        .game
        .delete_permanent_with_cause(ally2, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection_view().is_none(),
        "same-turn OPT should suppress the second eligible deletion"
    );
}

#[test]
fn bt22_040_other_deletion_refire_opt_clears_after_turn_cycle() {
    let mut runner = runner_with_bt22_040();
    runner.place_on_field(0, "BT22-040", Some(0));
    let ally1 = runner.place_on_field(0, "ALLY-1", Some(0));
    runner.place_on_field(0, "ALLY-2", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ally1, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection_view().is_some(),
        "first other own Digimon deletion should trigger"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline first optional refire");
    runner.auto_resolve().expect("finish first refire");

    // OPT slot lives in BT22-040's carrier permanent and resets when its
    // controller's `new_turn` runs — i.e. after a full turn cycle returns to
    // player 0. Once the slot clears, the next eligible deletion re-offers
    // the refire.
    runner.end_turn();
    runner.end_turn();

    // ALLY-1's deletion shifted the battle area — re-resolve ALLY-2.
    let ally2 = handle_by_card_id(&runner, 0, "ALLY-2");
    runner
        .game
        .delete_permanent_with_cause(ally2, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection_view().is_some(),
        "Once Per Turn must reset after a turn cycle, so a fresh-turn deletion re-offers the refire"
    );
}
