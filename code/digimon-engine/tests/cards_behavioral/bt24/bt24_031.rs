//! BT24-031 Elecmon.
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_031.cs
//! Covers On Play multi-bucket reveal and inherited When Attacking security-to-hand plus Recovery.

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_MY_SECURITY_START, SEL_REVEAL_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, TriggerSource};

#[test]
fn bt24_031_yaml_has_yellow_and_ts_level_2_paths_and_scoped_optional_security_choice() {
    let spec: CardSpec = serde_yml::from_str(include_str!("../../../cards/bt24/BT24-031.yaml"))
        .expect("BT24-031 YAML parses");
    let compiled = compile(&spec).expect("BT24-031 YAML compiles");
    assert_eq!(compiled.alt_paths.len(), 2);
    assert!(compiled
        .alt_paths
        .iter()
        .all(|path| path.cost == Some(CompiledCost::Literal(0))));
    assert!(
        compiled.alt_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            from.all_of
                .iter()
                .any(|predicate| predicate.level_eq == Some(2))
                && from
                    .all_of
                    .iter()
                    .any(|predicate| predicate.color_is == Some(CompiledColor::Yellow))
        }),
        "BT24-031 must retain the normal yellow level-2 path"
    );
    assert!(
        compiled.alt_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            from.all_of
                .iter()
                .any(|predicate| predicate.level_eq == Some(2))
                && from
                    .all_of
                    .iter()
                    .any(|predicate| predicate.trait_has.as_deref() == Some("TS"))
        }),
        "BT24-031 must also digivolve from level-2 TS"
    );

    let inherited = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when == vec![CompiledTiming::WhenAttacking] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("BT24-031 inherited When Attacking effect");
    assert!(
        !inherited.optional,
        "the inherited clause itself is mandatory; only adding top security is optional"
    );
    assert!(matches!(
        inherited.process.first(),
        Some(CompiledStep::MayAddTopSecurityToHand { .. })
    ));
}

#[test]
fn bt24_031_on_play_adds_iliad_and_ts_without_double_picking_dual_match() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_trait_card("ILIAD-ONLY", &["Iliad"]))
        .add_card(make_trait_card("TS-ONLY", &["TS"]))
        .add_card(make_trait_card("DUAL", &["Iliad", "TS"]))
        .deck(0, &["DUAL", "TS-ONLY", "ILIAD-ONLY"])
        .hand(0, &["BT24-031"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Elecmon");
    assert_required_pending_pick(&runner, "Iliad bucket");
    pick_revealed_by_id(&mut runner, "DUAL", "pick dual Iliad");
    let second_view = runner.pending_selection_view().expect("pick TS");
    assert_required_pending_pick(&runner, "TS bucket");
    let dual_action = revealed_action_for_id(&runner, "DUAL");
    assert!(
        !second_view.valid_action_ids.contains(&dual_action),
        "the same dual-match card cannot satisfy both buckets"
    );
    pick_revealed_by_id(&mut runner, "TS-ONLY", "pick TS");
    runner.auto_resolve().expect("finish reveal");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(hand_ids.iter().filter(|id| *id == "DUAL").count(), 1);
    assert!(!hand_ids.contains(&"ILIAD-ONLY".to_string()));
    assert!(hand_ids.contains(&"TS-ONLY".to_string()));
}

#[test]
fn bt24_031_inherited_when_attacking_adds_top_security_then_recovers_at_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT24-031", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    assert!(runner.pending_is_optional());
    let view = runner
        .pending_selection_view()
        .expect("security add prompt");
    assert_eq!(view.valid_action_ids, vec![SEL_MY_SECURITY_START]);
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("accept security add");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"SECURITY".to_string()));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

/// CLONE-FAITHFULNESS (make-engine-cloneable): the inherited When Attacking
/// "may add top security to hand" prompt is driven by the resumable data VM, so
/// a `Game::clone` taken AT the prompt is faithful — resolving the clone's accept
/// adds the top security to hand AND runs the recovery outer-tail (the steps that
/// follow `MayAddTopSecurityToHand` in the clause), without touching the
/// original; the original then replays identically. Also exercises the
/// outer-conts threading on the resume path (`add_top_security_to_hand` drains a
/// security-removed observer before the recovery tail runs). Before the flip the
/// prompt was closure-only, so the clone would hit the panic-stub.
#[test]
fn bt24_031_security_to_hand_clones_faithfully_at_the_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT24-031", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    // At the optional "may add top security" prompt — resume-driven (clone-safe).
    assert!(runner.pending_is_optional());
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "may-add-top-security must be resume-driven (clone-safe)"
    );
    assert_eq!(
        runner.pending_selection_view().expect("prompt").valid_action_ids,
        vec![SEL_MY_SECURITY_START]
    );

    // Clone, then resolve the accept on the CLONE only.
    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, SEL_MY_SECURITY_START)
        .expect("clone accepts the security add");
    let clone_hand = zone_ids(&clone.players[0].hand, &clone.card_data);
    assert!(
        clone_hand.contains(&"SECURITY".to_string()),
        "clone: top security added to hand"
    );
    assert_eq!(
        clone.players[0].security.last().unwrap().card_id(&clone.card_data),
        "RECOVER",
        "clone: the recovery outer-tail ran (RECOVER now in security)"
    );

    // INDEPENDENCE: the original is still at the prompt, untouched.
    assert!(
        runner.game.pending_selection.is_some(),
        "original still at the prompt after cloning + resolving the clone"
    );
    assert!(
        !zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"SECURITY".to_string()),
        "original hand untouched by the clone's resolution"
    );

    // REPLAYS IDENTICALLY: resolving the same accept on the original matches.
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("original accepts");
    assert!(
        zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"SECURITY".to_string()),
        "original: top security added to hand"
    );
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER",
        "original reaches the same state as the clone"
    );
}

#[test]
fn bt24_031_inherited_when_attacking_decline_keeps_security_and_runs_tail() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT24-031", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline security add");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(!hand_ids.contains(&"SECURITY".to_string()));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "SECURITY"
    );
}

#[test]
fn bt24_031_inherited_when_attacking_recovers_when_security_already_zero() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-031")
        .expect("BT24-031 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &["BT24-031", "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: reveal bucket should be mandatory when a matching card exists"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS must not be legal before the required pick"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id);
    assert!(view.valid_action_ids.contains(&action), "{label}");
    runner.execute_action(0, action).expect(label);
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}
