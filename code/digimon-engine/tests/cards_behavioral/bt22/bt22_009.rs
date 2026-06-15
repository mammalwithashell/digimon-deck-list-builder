//! BT22-009 Effecmon
//!
//! Implemented slice:
//! - [On Play] [When Digivolving] Delete 1 opponent Digimon with 4000 DP or less.
//!
//! Gap-routed slice:
//! - [Security] At end of battle, play this card free.
//! - Link Requirements text.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load from embedded DSL pack")
        .memory(5)
        .start()
}

/// Build a `CardSource` for `card_id` owned by `player`.
fn make_source(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardSource {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("make_source: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    CardSource::new(data_idx, player, card_index)
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

/// Place a host Digimon stack with BT22-009 as its bottom digivolution source,
/// returning the host permanent handle. `host_id` is a plain test Digimon with no
/// effects of its own, so any observed deletion comes from BT22-009's inherited box.
fn host_with_effecmon_source(runner: &mut DebugRunner, host_id: &str) -> PermanentHandle {
    // Base = BT22-009 (becomes the inherited source), then digivolve the host on top.
    let base = runner.place_on_field(0, "BT22-009", Some(0));
    let host = make_source(runner, 0, host_id);
    assert!(
        runner
            .game
            .digivolve_onto(0, base.index as usize, host),
        "host should digivolve onto the BT22-009 base"
    );
    base
}

/// [Inherited] Delete 1 of your opponent's Digimon with 4000 DP or less.
///
/// Fires as a [When Digivolving] effect on the host that carries BT22-009 as a
/// digivolution source. Positive case: a 4000-DP opponent Digimon is deleted.
#[test]
fn bt22_009_inherited_deletes_4000_dp_or_lower_when_host_digivolves() {
    let mut host_base = make_test_card("HOST-EVO", "HostEvo");
    host_base.level = Some(5);
    let mut low = make_test_card("OPP-4000", "Opp4000");
    low.dp = Some(4000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(host_base)
        .add_card(low)
        .memory(20)
        .start();

    let target = runner.place_on_field(1, "OPP-4000", Some(0));
    let host = host_with_effecmon_source(&mut runner, "HOST-EVO");

    // Fire the host's When Digivolving — BT22-009's inherited box must trigger.
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert!(
        runner
            .game
            .players[1]
            .battle_area
            .get(target.index as usize)
            .is_none(),
        "the inherited delete must remove the 4000-DP opponent Digimon"
    );
}

/// [Inherited] negative case: an opponent Digimon above 4000 DP is NOT a legal
/// target, so nothing is deleted (the inherited clause has no eligible target).
#[test]
fn bt22_009_inherited_spares_above_4000_dp_target() {
    let mut host_base = make_test_card("HOST-EVO", "HostEvo");
    host_base.level = Some(5);
    let mut high = make_test_card("OPP-5000", "Opp5000");
    high.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-009")
        .expect("BT22-009 must load")
        .add_card(host_base)
        .add_card(high)
        .memory(20)
        .start();

    let target = runner.place_on_field(1, "OPP-5000", Some(0));
    let host = host_with_effecmon_source(&mut runner, "HOST-EVO");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(host),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert!(
        runner.pending_selection().is_none(),
        "no selection should surface when no opponent Digimon is 4000 DP or less"
    );
    assert!(
        runner
            .game
            .players[1]
            .battle_area
            .get(target.index as usize)
            .is_some(),
        "a >4000-DP opponent Digimon must NOT be deleted by the inherited box"
    );
}

#[ignore = "pending: G-SECURITY-END-OF-BATTLE-PLAY — security effect plays this card at end of battle, not immediate OnSecurity resolution"]
#[test]
fn bt22_009_security_plays_self_at_end_of_battle() {}

#[ignore = "pending: G-LINK-REQUIREMENTS — Link text requires plug-in/link action and attachment lifecycle support"]
#[test]
fn bt22_009_link_requirements_are_available_as_link_actions() {}
