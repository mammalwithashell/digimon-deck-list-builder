use digimon_engine::action::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};

const CS_OBSERVER_YAML: &str = r#"
card: TST-CS-OBSERVER
name: CS Observer
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 1000
traits: [CS]
effects:
  - scope: inherited
    when: on_digivolution_card_placed
    condition:
      all_of:
        - event_is_effect_initiated: true
        - event_host_permanent_is_source: true
        - event_card_trait_has: CS
    process:
      - gain_memory: 1
"#;

const CS_OBSERVER_SELECTION_YAML: &str = r#"
card: TST-CS-SELECT-OBSERVER
name: CS Selection Observer
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 1000
traits: [CS]
effects:
  - scope: inherited
    when: on_digivolution_card_placed
    condition:
      all_of:
        - event_is_effect_initiated: true
        - event_host_permanent_is_source: true
        - event_card_trait_has: CS
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter: { kind: tamer }
          prompt: "Choose a Tamer"
"#;

#[test]
fn inherited_source_placement_observer_fires_for_effect_placed_cs_source() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CS_OBSERVER_YAML)
        .expect("observer YAML loads")
        .add_card(digimon("CARRIER", "Carrier", &[]))
        .add_card(digimon("TST-PLACED-CS", "Placed CS", &["CS"]))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .deck(0, &["TST-PLACED-CS"])
        .memory(0)
        .start();

    let host = runner.place_stack(0, &["TST-CS-OBSERVER", "CARRIER"]);
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        assert!(
            ctx.place_as_bottom_source(CardSourceRef::DeckTop(0), host, false),
            "effect should place the deck-top card as the bottom source"
        );
    }

    assert_eq!(
        runner.memory(),
        1,
        "inherited observer should fire from source stack context after effect-created source placement"
    );
}

#[test]
fn stack_setup_does_not_fire_effect_created_source_placement_observer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CS_OBSERVER_YAML)
        .expect("observer YAML loads")
        .add_card(digimon("CARRIER", "Carrier", &[]))
        .memory(0)
        .start();

    runner.place_stack(0, &["TST-CS-OBSERVER", "CARRIER"]);

    assert_eq!(
        runner.memory(),
        0,
        "ordinary setup must not look like an effect-created source placement event"
    );
}

#[test]
fn source_placement_observer_can_install_followup_pending_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(CS_OBSERVER_SELECTION_YAML)
        .expect("observer YAML loads")
        .add_card(digimon("CARRIER", "Carrier", &[]))
        .add_card(digimon("TST-PLACED-CS", "Placed CS", &["CS"]))
        .add_card(make_test_card("EFFECT", "Effect Source"))
        .add_card(tamer("TAMER-PICK", "Tamer Pick"))
        .deck(0, &["TST-PLACED-CS"])
        .hand(0, &["TAMER-PICK"])
        .memory(0)
        .start();

    let host = runner.place_stack(0, &["TST-CS-SELECT-OBSERVER", "CARRIER"]);
    let effect_perm = runner.place_on_field(0, "EFFECT", None);
    let effect_card = runner.top_card(effect_perm);

    {
        let mut ctx = EffectContext::new(&mut runner.game, effect_card, Some(effect_perm), 0);
        assert!(
            ctx.place_as_bottom_source(CardSourceRef::DeckTop(0), host, false),
            "effect should place the deck-top card as the bottom source"
        );
    }

    let pending = runner
        .pending_selection_view()
        .expect("source-placement observer should park on select_hand");
    assert_eq!(pending.valid_action_ids.len(), 1);
    let action = pending.valid_action_ids[0];
    assert_eq!(
        build_action_mask(&runner.game, 0)[action as usize],
        1.0,
        "follow-up pending selection action must be exposed through the action mask"
    );
}

fn digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(1000);
    card.colors = vec![CardColor::Purple];
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Purple];
    card
}
