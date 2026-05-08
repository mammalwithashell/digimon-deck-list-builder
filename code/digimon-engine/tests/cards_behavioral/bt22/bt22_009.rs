//! BT22-009 Effecmon
//!
//! Implemented slice:
//! - [On Play] [When Digivolving] Delete 1 opponent Digimon with 4000 DP or less.
//!
//! Gap-routed slice:
//! - [Security] At end of battle, play this card free.
//! - Link Requirements text.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt22_009_has_on_play_and_when_digivolving_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("BT22-009")
        .expect("BT22-009 must be compiled");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT22-009 must have a shared OnPlay/WhenDigivolving clause");

    assert!(
        !clause.optional,
        "delete clause is mandatory when a target exists"
    );
}

#[test]
fn bt22_009_on_play_deletes_4000_dp_or_lower_target() {
    let mut low = make_test_card("BT22-009-LOW", "LowTarget");
    low.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(low)
        .hand(0, &["BT22-009"])
        .memory(5)
        .start();

    runner.place_on_field(1, "BT22-009-LOW", None);
    runner.play(0, 0);
    runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "BT22-009 should delete the eligible low-DP opponent Digimon"
    );
}

#[ignore = "pending: G-SECURITY-END-OF-BATTLE-PLAY — security effect plays this card at end of battle, not immediate OnSecurity resolution"]
#[test]
fn bt22_009_security_plays_self_at_end_of_battle() {}

#[ignore = "pending: G-LINK-REQUIREMENTS — Link text requires plug-in/link action and attachment lifecycle support"]
#[test]
fn bt22_009_link_requirements_are_available_as_link_actions() {}
