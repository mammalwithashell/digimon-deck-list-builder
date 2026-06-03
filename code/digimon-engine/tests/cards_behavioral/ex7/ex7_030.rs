//! EX7-030 Cendrillmon - Lv.6 Yellow Digimon.
//!
//! # Card text (cards.json)
//!
//! ```text
//! <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of
//! your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player
//! without suspending.)
//! [Start of Your Main Phase] [When Digivolving] You may play 1 [Familiar] Token.
//! [When Attacking] 1 of your opponent's Digimon gets -6000 DP for the turn.
//! ```
//!
//! # Patterns
//! - H11 Overclock with Puppet/token cost filter
//! - B1 start-of-main token creation
//! - Token creation via `play_token`
//! - WhenAttacking opponent DP modifier

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn runner_with_ex7_030() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX7-030")
        .expect("EX7-030 YAML loads")
        .memory(10)
        .start()
}

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

fn make_trait_digimon(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn make_digimon_dp(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn make_token(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.level = None;
    card
}

fn encode_field_effect(field_index: u16, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + field_index * EFFECTS_PER_PERMANENT + effect_slot
}

#[test]
fn ex7_030_structural_metadata_matches_printed_card() {
    let runner = runner_with_ex7_030();
    let card = runner
        .compiled_card("EX7-030")
        .expect("EX7-030 must be in the embedded DSL pack");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|trait_name| trait_name == "Puppet"));
    assert!(card
        .traits
        .iter()
        .any(|trait_name| trait_name == "LIBERATOR"));

    let evo = card
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::Digivolve)
        .expect("yellow Lv5 digivolution path must be present");
    assert_eq!(format!("{:?}", evo.cost), "Some(Literal(4))");
    let from = evo.from.as_ref().expect("digivolution predicate");
    assert_eq!(from.level_eq, Some(5));
    assert_eq!(from.color_is, Some(CompiledColor::Yellow));
}

#[test]
fn ex7_030_has_overclock_with_puppet_or_token_cost_filter() {
    let runner = runner_with_ex7_030();
    let card = runner
        .compiled_card("EX7-030")
        .expect("EX7-030 must be compiled");

    let filter = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                overclock_cost_filter,
                ..
            }) if keyword.eq_ignore_ascii_case("Overclock") => overclock_cost_filter.as_ref(),
            _ => None,
        })
        .expect("Overclock must carry a Puppet/token sacrifice filter");

    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.kind == Some(CompiledCardKind::Token)),
        "Overclock allows deleting one of your Tokens"
    );
    assert!(
        filter.any_of.iter().any(|branch| {
            branch
                .all_of
                .iter()
                .any(|leaf| leaf.trait_has.as_deref() == Some("Puppet"))
        }),
        "Overclock allows deleting another Puppet Digimon"
    );
}

#[test]
fn ex7_030_runtime_overclock_masks_only_token_or_other_puppet_costs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-030")
        .expect("EX7-030 YAML loads")
        .add_card(make_token("COST-TOKEN"))
        .add_card(make_trait_digimon("NON-PUPPET", &["Dragonkin"]))
        .add_card(make_trait_digimon("PUPPET-ALLY", &["Puppet"]))
        .start();

    let cendrill = runner.place_on_field(0, "EX7-030", Some(0));
    runner.place_on_field(0, "COST-TOKEN", Some(0));
    runner.place_on_field(0, "NON-PUPPET", Some(0));
    runner.place_on_field(0, "PUPPET-ALLY", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    assert!(runner.game.has_keyword(cendrill, Keyword::Overclock));

    let effect_bit = encode_field_effect(0, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[effect_bit as usize], 1.0, "Overclock action is legal");

    runner.game.decode_action(effect_bit, 0);

    let cost_mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        cost_mask[encode_attack(0, 1) as usize],
        1.0,
        "token is a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 2) as usize],
        0.0,
        "non-token non-Puppet Digimon is not a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 3) as usize],
        1.0,
        "other Puppet Digimon is a legal Overclock cost"
    );
    assert_eq!(cost_mask[PASS as usize], 1.0, "Overclock is optional");
}

#[test]
fn ex7_030_start_of_main_may_play_one_familiar_token() {
    let mut runner = runner_with_ex7_030();
    let cendrill = runner.place_on_field(0, "EX7-030", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("accept optional Familiar token play");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played at start of main"
    );
}

#[test]
fn ex7_030_when_digivolving_may_play_one_familiar_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-030")
        .expect("EX7-030 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .memory(10)
        .start();
    let cendrill = runner.place_stack(0, &["BASE", "EX7-030"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("accept optional Familiar token play");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played when digivolving"
    );
}

#[test]
fn ex7_030_token_play_clause_exposes_decline_branch() {
    let mut runner = runner_with_ex7_030();
    let cendrill = runner.place_on_field(0, "EX7-030", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection_view()
        .expect("token-play choice prompt must be exposed");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    let labels: Vec<&str> = pending
        .effect_choices
        .as_ref()
        .expect("effect-choice labels")
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Play Familiar Token", "Decline"]);
    runner
        .execute_branch(1)
        .expect("decline Familiar token play");
    runner.auto_resolve().expect("finish declined clause");

    assert_eq!(
        familiar_count(&runner, 0),
        0,
        "declining the optional token play leaves no Familiar Token"
    );
}

#[test]
fn ex7_030_when_attacking_gives_one_opponent_digimon_minus_6000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-030")
        .expect("EX7-030 YAML loads")
        .add_card(make_digimon_dp("OPP", 3000))
        .memory(0)
        .start();
    let cendrill = runner.place_on_field(0, "EX7-030", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("opponent Digimon target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "mandatory When Attacking target selection must not expose PASS"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose opponent Digimon");
    runner.auto_resolve().expect("finish DP modifier");

    // -6000 DP drops the 3000 DP target below 0. Per rule 17-1-3-1 ("a Digimon
    // in the battle area whose DP is 0 is deleted"), the target is deleted by the
    // rule check that follows the effect — it does not linger at negative DP.
    assert_eq!(
        runner.effective_dp(opp),
        None,
        "deleted target has no effective DP"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent Digimon reduced to <= 0 DP by -6000 is deleted (rule 17-1-3-1)"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the deleted opponent Digimon is sent to its owner's trash"
    );
}

#[test]
fn ex7_030_when_attacking_does_not_prompt_without_opponent_digimon() {
    let mut runner = runner_with_ex7_030();
    let cendrill = runner.place_on_field(0, "EX7-030", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(cendrill),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon means no target prompt"
    );
}
