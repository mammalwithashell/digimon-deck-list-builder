//! EX10-003 Tumblemon
//!
//! Inherited Effect [Opponent's Turn] [Once Per Turn] When one of your
//! opponent's Digimon attacks, by trashing 3 [Mineral] or [Rock] trait cards
//! from this Digimon's digivolution cards, end that attack.

use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;
use digimon_engine::CardData;

fn card_with_traits(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-003")
        .expect("EX10-003 YAML parses and compiles")
        .add_card(card_with_traits("SRC-ROCK-1", "Rock Source 1", &["Rock"]))
        .add_card(card_with_traits(
            "SRC-MINERAL",
            "Mineral Source",
            &["Mineral"],
        ))
        .add_card(card_with_traits("SRC-ROCK-2", "Rock Source 2", &["Rock"]))
        .add_card(make_test_card("SRC-BAD", "Nonmatching Source"))
        .add_card(make_test_card("HOST", "Host Digimon"))
        .add_card(make_test_card("OTHER-HOST", "Other Host"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .build()
}

#[test]
fn ex10_003_trashes_three_matching_carrier_sources_to_cancel_attack() {
    let mut runner = runner();
    let carrier = runner.place_stack(
        0,
        &[
            "EX10-003",
            "SRC-ROCK-1",
            "SRC-MINERAL",
            "SRC-ROCK-2",
            "SRC-BAD",
            "HOST",
        ],
    );
    let other = runner.place_stack(0, &["SRC-ROCK-1", "OTHER-HOST"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    runner.attack_player(attacker, 0, false);

    // §15-7-1/§15-7-4: "by trashing 3 [Mineral] or [Rock] trait cards ..., end
    // that attack" is an OPTIONAL PROCESSING CONDITION, so the clause now
    // installs an outer accept/decline prompt BEFORE the cost selection.
    //
    // This test used to assert that the SourceMulti cost prompt was the first
    // pending selection after the attack — i.e. it pinned the auto-pay bug,
    // where the 3-source cost was forced on the controller with no way to
    // decline. DCGO passes isOptional=`true` at EX10_003.cs:19 and its own
    // source prompt is `canNoSelect: () => true`, so the decline must exist.
    // Corrected: accept the outer prompt first, then the cost prompt follows.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional processing condition must surface an accept/decline prompt first (rule 17); got {:?}",
        runner.pending_kind()
    );
    runner
        .accept_optional_trigger()
        .expect("accept the optional processing condition");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Tumblemon source-cost prompt should be exposed after accepting");
    let self_source = encode_source_select(carrier.index as u16, 0).expect("self source action");
    let rock = encode_source_select(carrier.index as u16, 1).expect("rock source action");
    let mineral = encode_source_select(carrier.index as u16, 2).expect("mineral source action");
    let second_rock =
        encode_source_select(carrier.index as u16, 3).expect("second rock source action");
    let bad = encode_source_select(carrier.index as u16, 4).expect("bad source action");
    let other_rock = encode_source_select(other.index as u16, 0).expect("other stack source");
    assert!(pending.valid_action_ids.contains(&self_source));
    assert!(pending.valid_action_ids.contains(&rock));
    assert!(pending.valid_action_ids.contains(&mineral));
    assert!(pending.valid_action_ids.contains(&second_rock));
    assert!(!pending.valid_action_ids.contains(&bad));
    assert!(!pending.valid_action_ids.contains(&other_rock));

    runner
        .game
        .resolve_selection(0, self_source)
        .expect("pick EX10-003");
    runner
        .game
        .resolve_selection(0, rock)
        .expect("pick Rock source");
    runner
        .game
        .resolve_selection(0, mineral)
        .expect("pick Mineral source");

    assert!(runner.game.pending_attack.is_none(), "attack was cancelled");
    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert_eq!(runner.trash_size(0), 3, "three sources were paid");
    assert_eq!(
        runner.game.players[0].battle_area[other.index as usize]
            .card_sources
            .len(),
        2,
        "sources from other stacks were not touched"
    );
}

/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions, regardless of whether or not the content of
/// the conditions can be executed." Declining "by trashing 3 [Mineral] or
/// [Rock] trait cards from this Digimon's digivolution cards" must leave BOTH
/// halves undone — the 3 sources stay on the stack and the attack is NOT ended
/// — because §15-7-2 says that if the optional condition's content isn't
/// executed, "the processing after the conditions can't be executed".
///
/// DCGO agrees: EX10_003.cs:19 passes `isOptional: true` to
/// `SetUpActivateClass`, its source prompt is `canNoSelect: () => true`, and
/// `TrashDigivolutionCards` + `EndAttack` only run `if (selectedCards.Count >= 3)`.
///
/// This is the branch the engine had no way to reach: the clause fired
/// unconditionally, so the 3-source cost was auto-paid.
#[test]
fn ex10_003_optional_cost_may_be_declined_leaving_sources_and_attack_intact() {
    let mut runner = runner();
    let carrier = runner.place_stack(
        0,
        &[
            "EX10-003",
            "SRC-ROCK-1",
            "SRC-MINERAL",
            "SRC-ROCK-2",
            "SRC-BAD",
            "HOST",
        ],
    );
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");

    let sources_before = runner.game.players[0].battle_area[carrier.index as usize]
        .card_sources
        .len();

    runner.attack_player(attacker, 0, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional processing condition must surface an accept/decline prompt (rule 17); got {:?}",
        runner.pending_kind()
    );
    runner
        .decline_optional_trigger()
        .expect("declining must be reachable from the action space");
    let _ = runner.auto_resolve();

    // §15-7-2, half 1: the cost was NOT paid.
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        sources_before,
        "declining must not trash any of this Digimon's digivolution cards"
    );

    // §15-7-2, half 2: the processing after the condition did NOT happen —
    // the attack was not ended, so it proceeded to the security check.
    assert_eq!(
        runner.security_count(0),
        0,
        "declining must NOT end the attack: security must still be checked"
    );
}
