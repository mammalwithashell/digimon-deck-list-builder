use digimon_dsl::compiled::{CompiledBindingRef, CompiledClause, CompiledStep, CompiledZone};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;

fn digixros_yaml() -> &'static str {
    r#"
card: DX-TARGET
name: DigiXros Target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { trait_has: Xros Heart }
        zones: [hand, battle_area]
        cost_delta: -1
    cost: 5
"#
}

fn modifier_yaml() -> &'static str {
    r#"
card: DX-MODIFIER
name: DigiXros Modifier
kind: tamer
color: [red]
cost: 3
traits: [Xros Heart]
effects:
  - kind: cost_reduction
    when_any_ally_played: { trait_has: Xros Heart }
    optional: true
    amount: 0
    pay_cost:
      - allow_digixros_material_zone: { zone: trash, max_count: 1 }
      - add_digixros_cost_delta: { delta: -2 }
"#
}

fn ballistamon_target_yaml() -> &'static str {
    r#"
card: DX-BALLISTAMON
name: Ballistamon Requirement Target
kind: digimon
level: 4
color: [red]
cost: 5
dp: 5000
traits: [Xros Heart]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_is: Ballistamon }
        zones: [battle_area]
        cost_delta: -2
    cost: 5
"#
}

fn wildcard_yaml() -> &'static str {
    r#"
card: DX-WILDCARD
name: Wildcard Body
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
traits: [Xros Heart]
effects:
  - when: on_play
    process:
      - register_digixros_wildcard_for_turn:
          card: this
"#
}

#[test]
fn digixros_wildcard_steps_lower_to_compiled_steps() {
    let yaml = r#"
card: DX-WILDCARD-COMPILE
name: Wildcard Compile
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
traits: [Xros Heart]
effects:
  - when: on_play
    process:
      - register_digixros_wildcard_for_turn:
          card: this
      - add_digixros_wildcard_to_pending_transaction:
          card: pick
          zone: material
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        &clause.process[0],
        CompiledStep::RegisterDigixrosWildcardForTurn {
            card: CompiledBindingRef::Source,
            zone: None,
        }
    ));
    assert!(matches!(
        &clause.process[1],
        CompiledStep::AddDigixrosWildcardToPendingTransaction {
            card: CompiledBindingRef::Named(name),
            zone: Some(CompiledZone::Material),
        } if name == "pick"
    ));
}

#[test]
fn optional_cost_reduction_body_can_mutate_pending_digixros_transaction() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(digixros_yaml())
        .expect("compile DigiXros target")
        .from_dsl_yaml(modifier_yaml())
        .expect("compile DigiXros modifier")
        .hand(0, &["DX-TARGET"])
        .memory(10)
        .start();
    runner.place_on_field(0, "DX-MODIFIER", Some(0));

    assert_eq!(runner.play(0, 0), None, "optional modifier parks first");
    let action = runner
        .pending_selection()
        .expect("optional transaction modifier prompt")
        .valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("accept optional transaction modifier");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("finish without selecting DigiXros materials");

    assert_eq!(
        runner.memory(),
        7,
        "DigiXros base cost 5 plus one-shot -2 transaction delta"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 2);
}

#[test]
fn turn_scoped_wildcard_step_makes_source_legal_for_later_digixros() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(wildcard_yaml())
        .expect("compile wildcard source")
        .from_dsl_yaml(ballistamon_target_yaml())
        .expect("compile DigiXros target")
        .hand(0, &["DX-WILDCARD", "DX-BALLISTAMON"])
        .memory(10)
        .start();

    let wildcard_index = runner.play(0, 0).expect("play wildcard source") as u16;
    runner
        .auto_resolve()
        .expect("resolve on-play wildcard step");

    assert_eq!(runner.play(0, 0), None, "DigiXros target should park");
    let prompt = runner
        .pending_selection()
        .expect("DigiXros material prompt");
    assert!(
        prompt.valid_action_ids.contains(&wildcard_index),
        "turn-scoped wildcard source should be offered as the missing Ballistamon requirement"
    );
    runner
        .execute_action(0, wildcard_index)
        .expect("select wildcard material");
    runner
        .execute_action(0, PASS)
        .expect("finish DigiXros material selection");

    let target = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "DX-BALLISTAMON")
        .expect("target should enter");
    assert!(target
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "DX-WILDCARD"));
}
