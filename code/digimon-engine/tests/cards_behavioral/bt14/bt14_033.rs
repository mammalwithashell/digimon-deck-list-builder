//! BT14-033 Patamon — selected-security digivolve and security-stack tail.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::SEL_MY_SECURITY_START;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::{EffectTiming, TriggerSource};

fn bt14_033_yaml() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt14/BT14-033.yaml"),
    )
    .expect("BT14-033 YAML should exist")
}

const EFFECT_DIGIVOLVE_OBSERVER_YAML: &str = r#"
card: TEST-EFFECT-DIGI-OBSERVER
name: Effect Digivolve Observer
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: on_digivolve
    condition:
      all_of:
        - event_is_effect_initiated: true
        - event_card_trait_has: Vaccine
    process:
      - gain_memory: 3
"#;

fn yellow_vaccine_lv4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Vaccine".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 2,
    }];
    card
}

fn yellow_vaccine_hand(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Vaccine".to_string()];
    card
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn security_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn bt14_033_selected_security_digivolves_preserves_order_and_runs_tail() {
    let yaml = bt14_033_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("BT14-033 YAML loads")
        .from_dsl_yaml(EFFECT_DIGIVOLVE_OBSERVER_YAML)
        .expect("effect digivolve observer YAML loads")
        .add_card(filler("BOTTOM"))
        .add_card(yellow_vaccine_lv4("EVO4"))
        .add_card(filler("TOP"))
        .add_card(yellow_vaccine_hand("HAND-VACCINE"))
        .security(0, &["BOTTOM", "EVO4", "TOP"])
        .hand(0, &["HAND-VACCINE"])
        .memory(0)
        .start();
    let patamon = runner.place_on_field(0, "BT14-033", Some(0));
    runner.place_on_field(0, "TEST-EFFECT-DIGI-OBSERVER", Some(1));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(patamon),
    );
    runner.game.drain_effect_queue();

    let security_pick = runner
        .pending_selection_view()
        .expect("BT14-033 should search security");
    assert!(
        security_pick
            .valid_action_ids
            .contains(&(SEL_MY_SECURITY_START + 1)),
        "the non-top EVO4 security card must be a legal pick"
    );
    assert_mask_allows(
        &runner,
        security_pick.selecting_player,
        SEL_MY_SECURITY_START + 1,
    );
    runner
        .execute_action(security_pick.selecting_player, SEL_MY_SECURITY_START + 1)
        .expect("choose EVO4 from security");

    assert_eq!(
        security_ids(&runner),
        vec!["BOTTOM".to_string(), "TOP".to_string()],
        "selecting a middle security card must preserve remaining relative order before shuffle"
    );
    assert_eq!(
        runner.memory(),
        3,
        "selected-security digivolve must fire OnDigivolve with effect-initiated provenance"
    );

    let hand_pick = runner
        .pending_selection_view()
        .expect("successful digivolve should expose the hand-to-security tail");
    let hand_action = *hand_pick
        .valid_action_ids
        .first()
        .expect("yellow Vaccine hand card must be a legal tail pick");
    assert_mask_allows(&runner, hand_pick.selecting_player, hand_action);
    runner
        .execute_action(hand_pick.selecting_player, hand_action)
        .expect("place hand card as bottom security");

    let stack = &runner.game.players[0].battle_area[patamon.index as usize];
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        "EVO4",
        "selected security card must become the digivolution result"
    );
    let security = security_ids(&runner);
    assert_eq!(security.len(), 3);
    assert!(
        security.contains(&"HAND-VACCINE".to_string()),
        "hand tail card must enter security after successful digivolve"
    );
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
