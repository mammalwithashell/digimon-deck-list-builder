use digimon_dsl::{compile, CardSpec};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::OptionPlayResult;
use std::sync::Arc;

fn compile_yaml(yaml: &str) -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    Arc::new(compile::compile(&spec).expect("compile card yaml"))
}

fn option_card(card_id: &str, cost: u16, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.play_cost = cost;
    cd.colors = vec![color];
    cd
}

fn digimon_card(card_id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![color];
    cd
}

fn register_dsl_yaml(r: &mut DebugRunner, yaml: &str) {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    let card_id = spec.card.clone();
    let compiled = compile::compile(&spec).expect("compile card yaml");
    r.register_effect(&card_id, Arc::new(DslCardEffect::new(Arc::new(compiled))));
}

#[test]
fn link_requirement_clause_lowers_to_link_option_effect() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 2
    filter: { level_gte: 3 }
"#;
    let card = compile_yaml(yaml);
    let dsl = DslCardEffect::new(card);
    let effects = dsl.effects(CardHandle(0));
    assert!(effects.iter().any(|e| {
        e.timing == EffectTiming::OptionMain && e.link_cost.is_some() && e.link_filter.is_some()
    }));
}

#[test]
fn linked_scope_lowers_to_linked_effect_flag() {
    let yaml = r#"
card: ST22-08
name: Offensive Plug-In V
kind: option
effects:
  - scope: linked
    when: end_of_your_turn
    optional: true
    process:
      - gain_memory: 1
"#;
    let card = compile_yaml(yaml);
    let dsl = DslCardEffect::new(card);
    let effects = dsl.effects(CardHandle(0));
    assert!(effects
        .iter()
        .any(|e| e.linked && e.timing == EffectTiming::EndOfYourTurn));
}

#[test]
fn linked_scope_only_fires_from_linked_card_scan() {
    let yaml = r#"
card: LINKED-EFFECT
name: Linked Draw
kind: option
effects:
  - kind: link_requirement
    scope: inherited
    cost: 0
    filter: { kind: digimon }
  - scope: linked
    when: end_of_your_turn
    process:
      - draw: { of: you, count: 1 }
"#;

    let mut face_up = DebugRunner::builder()
        .add_card(option_card("LINKED-EFFECT", 0, CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .deck(0, &["FILLER"; 5])
        .memory(0)
        .start();
    register_dsl_yaml(&mut face_up, yaml);
    face_up.place_on_field(0, "LINKED-EFFECT", Some(0));

    face_up.end_turn();

    assert_eq!(
        face_up.hand_size(0),
        0,
        "linked-scope effect must not fire from normal face-up permanents"
    );

    let mut linked = DebugRunner::builder()
        .add_card(option_card("LINKED-EFFECT", 0, CardColor::Red))
        .add_card(digimon_card("HOST", CardColor::Red))
        .add_card(digimon_card("FILLER", CardColor::Red))
        .hand(0, &["LINKED-EFFECT"])
        .deck(0, &["FILLER"; 5])
        .memory(0)
        .start();
    register_dsl_yaml(&mut linked, yaml);
    let host = linked.place_on_field(0, "HOST", Some(0));
    linked.game.enter_main_phase();

    assert_eq!(
        linked.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let action = linked
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    let _ = linked.game.resolve_selection(0, action);
    assert_eq!(
        linked.game.player(0).battle_area[host.index as usize]
            .linked_cards
            .len(),
        1
    );

    linked.end_turn();

    assert_eq!(
        linked.hand_size(0),
        1,
        "linked-scope effect must fire through the host linked-card scan"
    );
}
