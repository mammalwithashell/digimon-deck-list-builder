//! BT15-037 Gatomon.

use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

const YAML: &str = include_str!("../../../cards/bt15/BT15-037.yaml");

#[test]
fn bt15_037_has_barrier_and_inherited_barrier() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-037 YAML loads")
        .add_card(digimon("CARRIER", "Carrier"))
        .start();
    let gatomon = runner.place_on_field(0, "BT15-037", Some(0));
    let carrier = runner.place_stack(0, &["BT15-037", "CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(gatomon, Keyword::Barrier));
    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));
}

#[test]
fn bt15_037_effect_trashed_from_security_may_play_itself() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-037 YAML loads")
        .add_card(make_test_card("TRASHER", "Security Trasher"))
        .add_card(make_test_card("SEC", "Security Filler"))
        .security(1, &["SEC", "BT15-037"])
        .memory(0)
        .start();
    let trasher = runner.place_on_field(0, "TRASHER", Some(0));
    let source_card = runner.game.players[0].battle_area[trasher.index as usize]
        .top_card()
        .handle();

    runner.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut runner.game,
            source_card,
            Some(trasher),
            0,
        );
        assert!(ctx.trash_top_security(1));
    }
    runner.game.set_effect_source_player_for_test(None);

    assert!(
        runner.pending_selection().is_some(),
        "optional play-self trigger must surface through a pending selection"
    );
    let accept = runner
        .pending_selection_view()
        .expect("optional play-self trigger")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 1, accept);
    runner
        .execute_action(1, accept)
        .expect("accept play-self from security-trash trigger");
    runner.auto_resolve().expect("play resolves");

    assert!(battle_area_contains(&runner, 1, "BT15-037"));
    assert_eq!(runner.security_count(1), 1);
}

#[test]
fn bt15_037_own_security_removed_gains_memory_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT15-037 YAML loads")
        .add_card(make_test_card("SEC1", "Security 1"))
        .add_card(make_test_card("SEC2", "Security 2"))
        .security(0, &["SEC1", "SEC2"])
        .memory(0)
        .start();
    runner.place_on_field(0, "BT15-037", Some(0));

    fire_security_removed(&mut runner);
    fire_security_removed(&mut runner);

    assert_eq!(runner.memory(), 1, "OPT memory gain fires only once");
}

fn fire_security_removed(runner: &mut DebugRunner) {
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: digimon_engine::card_source::CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

fn digimon(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card
}

fn battle_area_contains(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == id)
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
