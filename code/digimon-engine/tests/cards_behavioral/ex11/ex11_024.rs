//! EX11-024 Cendrillmon.
//! Printed text covered here:
//! - <Alliance>.
//! - <Overclock ([Puppet] Trait)>.
//! - [On Play] [When Digivolving] optional free-play of 1 level 4 or lower
//!   Puppet Digimon from hand, then optional Familiar Token generation equal
//!   to the opponent's Digimon count.
//! - [When Digivolving] [When Attacking] -3000 DP to 1 opponent Digimon for
//!   each of your Digimon.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

fn make_digimon(
    id: &str,
    level: u8,
    dp: i32,
    traits: &[&str],
) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn ex11_024_has_alliance_and_puppet_overclock_while_face_up() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .start();
    let cendrill = runner.place_on_field(0, "EX11-024", Some(0));

    assert!(runner.game.has_keyword(cendrill, Keyword::Alliance));
    assert!(runner.game.has_keyword(cendrill, Keyword::Overclock));
}

#[test]
fn ex11_024_on_play_may_play_level4_or_lower_puppet_then_tokens_per_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .dsl_card("EX11-019")
        .expect("EX11-019 YAML loads")
        .dsl_card("EX7-027")
        .expect("EX7-027 YAML loads")
        .add_card(make_digimon("NON-PUPPET-L4", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .add_card(make_digimon("OPP-B", 4, 5000, &[]))
        .hand(0, &["EX11-024", "EX11-019", "EX7-027", "NON-PUPPET-L4"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));

    runner.play(0, 0).expect("Cendrillmon plays from hand");

    let puppet_choice = runner
        .pending_selection_view()
        .expect("optional Puppet play choice");
    assert_eq!(puppet_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, puppet_choice.valid_action_ids[0])
        .expect("choose to play a Puppet");

    let hand_pick = runner
        .pending_selection_view()
        .expect("level 4 or lower Puppet hand prompt");
    assert_eq!(hand_pick.kind, SelectionKind::Hand);
    assert_eq!(
        hand_pick.valid_action_ids,
        vec![0],
        "only the level 4 Puppet is eligible"
    );
    runner
        .execute_action(0, hand_pick.valid_action_ids[0])
        .expect("select level 4 Puppet");

    let token_choice = runner
        .pending_selection_view()
        .expect("optional Familiar Token choice");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[0])
        .expect("choose token generation");
    runner.auto_resolve().expect("finish On Play effect");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-019"),
        "selected Puppet enters the battle area"
    );
    assert_eq!(
        familiar_count(&runner, 0),
        2,
        "one Familiar Token is played for each opponent Digimon"
    );
    assert_eq!(
        runner.hand_size(0),
        2,
        "ineligible level 5 Puppet and non-Puppet level 4 remain in hand"
    );
}

#[test]
fn ex11_024_declining_hand_play_still_offers_token_generation() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("PUPPET-L4", 4, 4000, &["Puppet"]))
        .add_card(make_digimon("OPP-A", 3, 2000, &[]))
        .hand(0, &["EX11-024", "PUPPET-L4"])
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-A", Some(0));

    runner.play(0, 0).expect("Cendrillmon plays from hand");

    let puppet_choice = runner
        .pending_selection_view()
        .expect("optional Puppet play choice");
    assert_eq!(puppet_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, puppet_choice.valid_action_ids[1])
        .expect("decline hand play");

    let token_choice = runner
        .pending_selection_view()
        .expect("token choice still appears after declining hand play");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[0])
        .expect("choose token generation");
    runner.auto_resolve().expect("finish token generation");

    assert_eq!(runner.hand_size(0), 1, "Puppet card remains in hand");
    assert_eq!(familiar_count(&runner, 0), 1, "one token is played");
}

#[test]
fn ex11_024_when_digivolving_reduces_one_opponent_digimon_by_3000_per_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("BASE", 5, 7000, &[]))
        .add_card(make_digimon("ALLY-A", 3, 2000, &[]))
        .add_card(make_digimon("ALLY-B", 4, 4000, &[]))
        .add_card(make_digimon("OPP-A", 5, 10000, &[]))
        .start();
    let cendrill = runner.place_stack(0, &["BASE", "EX11-024"]);
    runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let trigger_order = runner
        .pending_selection_view()
        .expect("choose order for Cendrillmon's When Digivolving triggers");
    assert_eq!(trigger_order.kind, SelectionKind::TriggerOrder);
    runner
        .execute_action(0, trigger_order.valid_action_ids[0])
        .expect("resolve hand/token trigger first");

    let puppet_choice = runner
        .pending_selection_view()
        .expect("optional Puppet play choice");
    assert_eq!(puppet_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, puppet_choice.valid_action_ids[1])
        .expect("decline hand play");

    let token_choice = runner
        .pending_selection_view()
        .expect("token choice resolves before DP clause");
    assert_eq!(token_choice.kind, SelectionKind::EffectChoice);
    runner
        .execute_action(0, token_choice.valid_action_ids[1])
        .expect("decline token generation");

    let dp_target = runner
        .pending_selection_view()
        .expect("opponent Digimon DP target prompt");
    assert_eq!(dp_target.kind, SelectionKind::OppField);
    assert_eq!(dp_target.valid_action_ids.len(), 1);
    runner
        .execute_action(0, dp_target.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP clause");

    assert_eq!(
        runner.effective_dp(target),
        Some(1000),
        "three own Digimon produce -9000 DP for the turn"
    );
}

#[test]
fn ex11_024_when_attacking_reduces_one_opponent_digimon_by_3000_per_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-024")
        .expect("EX11-024 YAML loads")
        .add_card(make_digimon("ALLY", 3, 2000, &[]))
        .add_card(make_digimon("OPP-A", 5, 10000, &[]))
        .start();
    let cendrill = runner.place_on_field(0, "EX11-024", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let dp_target = runner
        .pending_selection_view()
        .expect("opponent Digimon DP target prompt");
    assert_eq!(dp_target.kind, SelectionKind::OppField);
    runner
        .execute_action(0, dp_target.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP clause");

    assert_eq!(
        runner.effective_dp(target),
        Some(4000),
        "two own Digimon produce -6000 DP for the turn"
    );
}
