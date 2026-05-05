//! BT22-088 Arisa Kinosaki.
//! Printed text covered here: Security Effect [Security] Play this card
//! without paying the cost.
//!
//! Partial: the start-of-main return-this-Tamer cost and all-turns
//! Token/Puppet play observer are omitted until the engine can expose the
//! required optional cost and event-context choices without auto-resolution.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt22_088_is_yellow_tamer_cost_3_liberator() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.traits, vec!["LIBERATOR".to_string()]);
}

#[test]
fn bt22_088_only_exposes_supported_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT22-088").expect("compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "only the currently faithful [Security] slice should be live"
    );
    let security = triggered[0];
    assert!(security.when.contains(&CompiledTiming::OnSecurity));
    assert!(!security.optional, "[Security] play this card is mandatory");
    assert_eq!(security.scope, CompiledScope::FaceUp);
}

#[test]
fn bt22_088_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-BT22-088", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-088")
        .expect("BT22-088 YAML loads")
        .add_card(attacker)
        .security(1, &["BT22-088"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-BT22-088", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT22-088"),
        "BT22-088 is played from the defender's security"
    );
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - single optional triggered effects auto-fire, so returning this Tamer cannot be exposed as a visible optional cost"]
fn bt22_088_start_of_main_can_decline_before_returning_tamer_to_deck_bottom() {
    todo!("place BT22-088 in battle area, enter start of main, assert PASS is legal before the Tamer leaves, then decline and assert it remains in battle area");
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - start-of-main return-self cost must be chosen before exact named free-play branches resolve"]
fn bt22_088_start_of_main_accepts_cost_then_plays_exact_arisa_from_hand() {
    todo!("after accepting the return-to-bottom cost, assert only exact-name Arisa Kinosaki, not longer names, is legal from hand and is played free");
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G028 - no-Digimon Shoemon trash branch is chained after the blocked return-self cost"]
fn bt22_088_start_of_main_no_digimon_branch_plays_exact_shoemon_from_trash() {
    todo!("after accepting the return-to-bottom cost with no Digimon, assert only exact Shoemon, not ShoeShoemon, is legal from trash and is played free");
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G005 - OnEnterFieldAnyone lacks faithful Token/Puppet play event context plus source-bound suspend-this-Tamer cost preflight"]
fn bt22_088_all_turns_token_or_puppet_played_suspends_this_tamer_to_draw() {
    todo!("play an own Token or Puppet while BT22-088 is unsuspended, assert a visible suspend-this-Tamer cost then Draw 1; non-Puppet and opponent plays must not trigger");
}
