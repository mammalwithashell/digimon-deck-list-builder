//! Gap 2 — `cost: { return_own_sources_to_deck: { filter, count, position } }`
//! leave-field replacement cost (BT21-062 Galacticmon).
//!
//! "[All Turns] When this Digimon would leave the battle area, by returning
//! N [Vemmon] from its digivolution cards to the bottom of the deck, it doesn't
//! leave." Proven via an inline DSL card (NOT authoring the blocked BT21-062):
//!  - pay-in-full: exactly `count` sources are picked (min == max == count),
//!  - not-offered-when-unpayable: with < `count` matching sources the accept
//!    prompt is never surfaced and the removal proceeds,
//!  - picks exposed to the action space (a `SourceMulti` selection),
//!  - `outcome: prevent` cancels the leave,
//!  - the returned sources hit the BOTTOM of the deck (not trash) and the
//!    payment fires the Gap-1 `OnDigivolutionCardReturnedToDeckBottom` observer.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

/// Inline card whose would-leave replacement returns EXACTLY 2 [Vemmon]
/// digivolution sources to deck bottom to prevent the leave.
fn galacticmon_like_yaml() -> &'static str {
    r#"
card: DX-LEAVE-RETURN
name: Leave Return
kind: digimon
level: 6
color: [black]
cost: 12
dp: 12000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - none_of:
            - replacement_cause: battle
    outcome: prevent
    cost:
      return_own_sources_to_deck:
        count: 2
        position: bottom
        filter:
          name_contains: "Vemmon"
    summary: "[All Turns] by returning 2 [Vemmon] digivolution sources to deck bottom, it doesn't leave"
"#
}

fn make_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn permanent_exists(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

#[test]
fn returning_two_vemmon_sources_to_deck_bottom_prevents_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(galacticmon_like_yaml())
        .expect("inline leave-return YAML compiles")
        .add_card(make_digimon("VEMMON-A", "Vemmon"))
        .add_card(make_digimon("VEMMON-B", "Vemmon"))
        .start();

    // [VEMMON-A, VEMMON-B, DX-LEAVE-RETURN] bottom→top: two Vemmon sources.
    let host = runner.place_stack(0, &["VEMMON-A", "VEMMON-B", "DX-LEAVE-RETURN"]);
    let deck_before = runner.game.players[0].deck.len();

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    // Accept the optional replacement.
    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the leave-prevention");

    // The cost is a source multi-pick requiring exactly 2 sources.
    let cost_view = runner
        .pending_selection_view()
        .expect("source-return cost selection installs");
    assert!(
        matches!(
            cost_view.kind,
            SelectionKind::SourceMulti { min: 2, max: 2, .. }
        ),
        "the cost is an exact-2 digivolution-source pick, got {:?}",
        cost_view.kind
    );
    // Two Vemmon sources are the only legal picks; PASS is NOT offered before
    // the minimum is met (pay-in-full).
    assert_eq!(
        cost_view.valid_action_ids.len(),
        2,
        "both Vemmon sources are legal picks and PASS is not yet offered"
    );
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("return the first Vemmon source");
    runner
        .auto_resolve()
        .expect("finish the second pick + return");

    assert!(
        permanent_exists(&runner, 0, "DX-LEAVE-RETURN"),
        "paying the cost prevents the removal — the Digimon stays"
    );
    // Both sources left the stack and hit the BOTTOM of the deck (not trash).
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "deck grows by the two returned sources"
    );
    assert_eq!(runner.trash_size(0), 0, "returns must not route to trash");
    let host_perm = &runner.game.players[0].battle_area[host.index as usize];
    assert!(
        host_perm
            .card_sources
            .iter()
            .all(|s| !s.card_id(&runner.game.card_data).starts_with("VEMMON")),
        "both Vemmon sources are gone from the digivolution stack"
    );
}

#[test]
fn not_offered_when_fewer_than_count_matching_sources() {
    // Only ONE Vemmon source under the host: the exact-2 cost is unpayable, so
    // the replacement's accept prompt is never surfaced and the removal goes
    // through.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(galacticmon_like_yaml())
        .expect("inline leave-return YAML compiles")
        .add_card(make_digimon("VEMMON-A", "Vemmon"))
        .add_card(make_digimon("AGUMON", "Agumon"))
        .start();

    // [VEMMON-A, AGUMON, DX-LEAVE-RETURN]: only 1 Vemmon source.
    let host = runner.place_stack(0, &["VEMMON-A", "AGUMON", "DX-LEAVE-RETURN"]);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "with < 2 [Vemmon] sources the cost is unpayable — no accept prompt"
    );
    assert!(
        !permanent_exists(&runner, 0, "DX-LEAVE-RETURN"),
        "unpayable cost lets the Digimon leave the battle area"
    );
}

#[test]
fn declining_lets_it_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(galacticmon_like_yaml())
        .expect("inline leave-return YAML compiles")
        .add_card(make_digimon("VEMMON-A", "Vemmon"))
        .add_card(make_digimon("VEMMON-B", "Vemmon"))
        .start();

    let host = runner.place_stack(0, &["VEMMON-A", "VEMMON-B", "DX-LEAVE-RETURN"]);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the leave-prevention");
    runner.auto_resolve().expect("finish declined removal");

    assert!(
        !permanent_exists(&runner, 0, "DX-LEAVE-RETURN"),
        "declining the optional replacement lets it leave"
    );
}

#[test]
fn payment_fires_return_to_deck_bottom_observer() {
    // A Snatchmon-like inherited observer sits UNDER the leaving Digimon; paying
    // the return cost (2 Vemmon to deck bottom) must fire the Gap-1
    // `OnDigivolutionCardReturnedToDeckBottom` observer, which deletes an
    // opponent Digimon (BT21-058's shape).
    let observer_card = r#"
card: DX-UNDER-OBSERVER
name: Under Observer
kind: digimon
level: 4
color: [black]
cost: 4
dp: 4000
effects:
  - scope: inherited
    when: on_source_returned_to_deck_bottom
    once_per_turn: true
    condition:
      all_of:
        - event_host_permanent_is_source: true
        - event_card_name_contains: "Vemmon"
        - any_permanent: { of: opponent, zone: [battle_area], kind: digimon }
    summary: "when a [Vemmon] returns to deck bottom from this Digimon, delete 1 opp Digimon"
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
          prompt: "Delete 1 opponent Digimon"
      - delete_permanent: { target: target }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(galacticmon_like_yaml())
        .expect("leave-return YAML compiles")
        .from_dsl_yaml(observer_card)
        .expect("observer YAML compiles")
        .add_card(make_digimon("VEMMON-A", "Vemmon"))
        .add_card(make_digimon("VEMMON-B", "Vemmon"))
        .add_card(make_digimon("OPP-DIGI", "Opp Digimon"))
        .start();

    // Stack: observer (bottom) / VEMMON-A / VEMMON-B / DX-LEAVE-RETURN (top).
    let host = runner.place_stack(
        0,
        &[
            "DX-UNDER-OBSERVER",
            "VEMMON-A",
            "VEMMON-B",
            "DX-LEAVE-RETURN",
        ],
    );
    runner.place_on_field(1, "OPP-DIGI", None);

    runner
        .game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    // Accept the leave-prevention, then pay the 2-Vemmon return cost.
    let view = runner
        .pending_selection_view()
        .expect("accept prompt installs");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept");

    let cost_view = runner
        .pending_selection_view()
        .expect("source cost installs");
    assert!(matches!(
        cost_view.kind,
        SelectionKind::SourceMulti { min: 2, max: 2, .. }
    ));
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("return first Vemmon");
    // After the second Vemmon returns, the observer fires → OppField delete.
    // auto_resolve walks any intervening picks; the delete must land.
    runner.auto_resolve().expect("resolve remaining selections");

    assert!(
        permanent_exists(&runner, 0, "DX-LEAVE-RETURN"),
        "the leave was prevented"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        0,
        "the under-observer fired on the deck-bottom return and deleted the opponent Digimon"
    );
}
