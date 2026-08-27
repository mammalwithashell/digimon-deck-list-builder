//! BT23-064 Bakemon
//!
//! Implemented slice:
//! - [On Play] [When Digivolving] By deleting 1 of your Digimon, delete
//!   1 opponent level 4 or lower Digimon.
//! - Inherited [On Deletion] Gain 1 memory.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt23_064_has_play_digivolve_and_inherited_deletion_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-064")
        .expect("BT23-064 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "BT23-064 must have a shared OnPlay/WhenDigivolving delete-cost clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnDeletion)
        )),
        "BT23-064 must have inherited OnDeletion gain-memory clause"
    );
}

/// Clause shape: "By deleting 1 of your Digimon, delete 1 of your opponent's
/// level 4 or lower Digimon" is an OPTIONAL PROCESSING CONDITION.
///
/// §15-7-1: "Optional processing conditions include text such as 'by X, Y.'"
/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions, regardless of whether or not the content
/// of the conditions can be executed."
///
/// The body's first step is a mandatory `select_own_permanent` pick, so the
/// decline can only reach the RL action space through the forced outer
/// accept/decline confirm (`outer_prompt: true`). Rule 17 (no auto-selections)
/// requires that branch to exist.
#[test]
fn bt23_064_shared_clause_is_an_optional_processing_condition() {
    let runner = runner();
    let card = runner
        .compiled_card("BT23-064")
        .expect("BT23-064 must be compiled");

    let shared = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT23-064 must have the shared OnPlay/WhenDigivolving clause");

    assert!(
        shared.optional,
        "\"By deleting 1 of your Digimon\" is an optional processing condition \
         (§15-7-1/§15-7-4): the player may decline to pay it"
    );
    assert!(
        shared.outer_prompt,
        "the body leads with a mandatory select_own_permanent, so the decline \
         needs the forced outer accept/decline confirm to be reachable"
    );

    // The inherited [On Deletion] "Gain 1 memory" prints no "by ..." cost and
    // must stay mandatory — do not let the optional flag leak onto it.
    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnDeletion) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("BT23-064 must have the inherited OnDeletion clause");
    assert!(
        !inherited.optional,
        "inherited [On Deletion] Gain 1 memory carries no optional processing \
         condition and must remain mandatory"
    );
}

/// ACCEPT path for the optional processing condition. `auto_resolve` takes
/// the first legal action at every prompt, which on the outer accept/decline
/// confirm is ACCEPT — so paying the cost still deletes an opponent Digimon.
#[test]
fn bt23_064_on_play_deletes_own_digimon_then_level4_opponent() {
    let mut ally = make_test_card("BT23-064-ALLY", "Ally");
    ally.level = Some(3);
    let mut opp = make_test_card("BT23-064-OPP", "Opponent");
    opp.level = Some(4);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load")
        .add_card(ally)
        .add_card(opp)
        .hand(0, &["BT23-064"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT23-064-ALLY", None);
    runner.place_on_field(1, "BT23-064-OPP", None);
    runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "BT23-064 should delete the selected level 4 or lower opponent Digimon"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "BT23-064 should pay by deleting one of your Digimon"
    );
}

/// DECLINE path — §15-7-4: "A player can choose whether or not to execute the
/// content of optional processing conditions." Refusing "By deleting 1 of your
/// Digimon" must leave BOTH halves undone: no own Digimon is deleted (the cost
/// is not paid) and no opponent Digimon is deleted, because §15-7-2 says that
/// when the condition's content isn't executed, "the processing after the
/// conditions can't be executed".
///
/// Before the fix the clause fired unconditionally, so the self-delete was
/// auto-paid and this branch was unreachable from the action space (rule 17).
#[test]
fn bt23_064_by_deleting_cost_may_be_declined_leaving_both_boards_intact() {
    let mut ally = make_test_card("BT23-064-ALLY", "Ally");
    ally.level = Some(3);
    let mut opp = make_test_card("BT23-064-OPP", "Opponent");
    opp.level = Some(4);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load")
        .add_card(ally)
        .add_card(opp)
        .hand(0, &["BT23-064"])
        .memory(5)
        .start();

    runner.place_on_field(0, "BT23-064-ALLY", None);
    runner.place_on_field(1, "BT23-064-OPP", None);
    runner.play(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // The cost was NOT paid: nothing of the controller's went to trash and the
    // ally is still on the field (alongside BT23-064 itself).
    assert_eq!(
        runner.trash_size(0),
        0,
        "declining the optional cost must NOT delete one of your Digimon"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT23-064-ALLY"),
        "the ally that would have paid the cost must survive the decline"
    );

    // §15-7-2: with the condition declined, the processing after it can't run.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "with the optional condition declined, the opponent's level 4 Digimon \
         must NOT be deleted"
    );
}

#[test]
fn bt23_064_inherited_on_deletion_gains_memory() {
    let carrier = make_test_card("BT23-064-CARRIER", "Carrier");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-064")
        .expect("BT23-064 must load")
        .add_card(carrier)
        .memory(0)
        .start();

    let handle = runner.place_on_field(0, "BT23-064-CARRIER", None);
    runner.push_source(handle, "BT23-064");
    runner.game.delete_permanent_with_effects(handle);
    runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "inherited OnDeletion should gain 1 memory"
    );
}
