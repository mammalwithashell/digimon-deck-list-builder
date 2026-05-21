use std::sync::Arc;

use digimon_dsl::compiled::CompiledAltPathKind;
use digimon_engine::action::{build_action_mask, DNA_DIGIVOLVE_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};

fn compile_yaml(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse card yaml");
    digimon_dsl::compile::compile(&spec).expect("compile card yaml")
}

fn colored_digimon_level(id: &str, color: CardColor, level: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        dual: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
        ace_overflow: None,
    }
}

fn registration_yaml(cost: i32) -> String {
    format!(
        r#"
card: T-G7-ALT
name: Alt Registration
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
effects:
  - kind: alt_path_registration
    scope: inherited
    trigger: end_of_your_turn
    applies_to: {{ kind: digimon, color_is: blue, level_eq: 5 }}
    registers:
      kind: dna_digivolve
      materials:
        - filter: {{ color_is: blue, level_eq: 4 }}
        - filter: {{ color_is: green, level_eq: 4 }}
      cost: {cost}
"#
    )
}

fn blast_dna_yaml() -> &'static str {
    r#"
card: T-G7-BLAST-DNA
name: Blast DNA
kind: digimon
level: 7
color: [red, black]
cost: 9
dp: 15000
alt_paths:
  - kind: blast_dna_digivolve
    materials:
      - { name_is: Durandamon }
      - { name_is: BryweLudramon }
    cost: 0
"#
}

#[test]
fn inherited_end_of_turn_alt_path_registration_compiles() {
    let compiled = compile_yaml(&registration_yaml(0));
    let digimon_dsl::compiled::CompiledClause::Declarative(
        digimon_dsl::compiled::CompiledDeclarativeClause::AltPathRegistration {
            trigger,
            registers,
            ..
        },
    ) = &compiled.effects[0]
    else {
        panic!("expected alt_path_registration clause");
    };
    assert_eq!(
        *trigger,
        digimon_dsl::compiled::CompiledTiming::EndOfYourTurn
    );
    assert_eq!(registers.materials.len(), 2);
}

#[test]
fn inherited_alt_path_registration_lowers_to_runtime_effect() {
    let compiled = compile_yaml(&registration_yaml(0));
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));
    assert!(
        effects.iter().any(|effect| {
            effect.timing == EffectTiming::EndOfYourTurn
                && effect.alt_path_registration.is_some()
                && effect.inherited
        }),
        "inherited alt_path_registration must lower into an end-of-turn runtime effect"
    );
}

#[test]
fn blast_dna_alt_path_lowers_to_counter_blast_marker_effect() {
    let compiled = compile_yaml(blast_dna_yaml());
    assert!(compiled.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::BlastDnaDigivolve
            && path.materials.len() == 2
            && path.materials[0].filter.name_is.as_deref() == Some("Durandamon")
            && path.materials[1].filter.name_is.as_deref() == Some("BryweLudramon")
    }));

    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));
    assert!(
        effects
            .iter()
            .any(|effect| effect.timing == EffectTiming::Declarative && effect.blast_digivolve),
        "blast_dna_digivolve must lower into a declarative CounterTiming blast marker effect"
    );
}

#[test]
fn inherited_alt_path_registration_makes_dna_action_legal_at_end_of_turn() {
    let yaml = registration_yaml(0);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("registration DSL compiles")
        .add_card(colored_digimon_level("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_digimon_level("GREEN-LV4", CardColor::Green, 4))
        .add_card(colored_digimon_level("BLUE-LV5", CardColor::Blue, 5))
        .hand(0, &["BLUE-LV5"])
        .memory(3)
        .start();

    runner.place_stack(0, &["T-G7-ALT", "BLUE-LV4"]);
    runner.place_on_field(0, "GREEN-LV4", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);
}

#[test]
fn registered_dna_action_uses_registered_materials_and_cost() {
    let yaml = registration_yaml(2);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("registration DSL compiles")
        .add_card(colored_digimon_level("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_digimon_level("GREEN-LV4", CardColor::Green, 4))
        .add_card(colored_digimon_level("BLUE-LV5", CardColor::Blue, 5))
        .hand(0, &["BLUE-LV5"])
        .memory(5)
        .start();

    runner.place_stack(0, &["T-G7-ALT", "BLUE-LV4"]);
    runner.place_on_field(0, "GREEN-LV4", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    assert!(runner.game.initiate_dna_digivolve(0, 0));
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    runner.game.resolve_selection(0, 0).expect("stage 1");
    runner.game.resolve_selection(0, 1).expect("stage 2");

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(
        runner.game.players[0].battle_area[0]
            .top_card()
            .card_id(&runner.game.card_data),
        "BLUE-LV5"
    );
    assert_eq!(runner.game.memory, 3);
    assert_eq!(runner.game.current_phase, GamePhase::EndOfTurnAction);
}

// ── Phase 2 Track F (G-ALT-PATH-DIRECTION-INTO) ─────────────────────────────

fn into_direction_warp_yaml(hand_target_name: &str, cost: i32) -> String {
    // Source card carries an alt-path with direction:into pointing at a
    // specific hand card. ST20-10 Agumon's [Your Turn] warp into WarGreymon
    // is the canonical fixture, but here we use a synthetic carrier to
    // isolate the substrate from the unrelated G-DSL-DISTINCT-TAMER-COLORS
    // gate still blocking ST20-10's full clause.
    format!(
        r#"
card: T-G7-INTO
name: Into Direction Carrier
kind: digimon
level: 3
color: [black]
cost: 3
dp: 1000
alt_paths:
  - kind: digivolve
    direction: into
    from:
      zone: [hand]
      of: you
      name_is: "{hand_target_name}"
    cost: {cost}
    ignore_requirements: true
"#
    )
}

#[test]
fn alt_path_direction_into_compiles_and_preserves_field() {
    let compiled = compile_yaml(&into_direction_warp_yaml("WarGreymon", 4));
    let path = &compiled.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.direction,
        digimon_dsl::compiled::CompiledAltPathDirection::Into
    );
    assert!(path.ignore_requirements);
}

#[test]
fn alt_path_direction_from_defaults_when_omitted() {
    // Back-compat: existing YAML without `direction:` lowers to
    // CompiledAltPathDirection::From and behaves exactly as before.
    let yaml = r#"
card: T-G7-DEFAULT
name: Default Direction
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 1000
alt_paths:
  - kind: digivolve
    from: { level_eq: 2, color_is: blue }
    cost: 0
"#;
    let compiled = compile_yaml(yaml);
    assert_eq!(
        compiled.alt_paths[0].direction,
        digimon_dsl::compiled::CompiledAltPathDirection::From
    );
}

#[test]
fn alt_path_direction_into_routes_hand_card_digivolve_onto_carrier() {
    // End-to-end: an Into-direction alt-path on the field permanent (the
    // CARRIER, ST20-10 in printed form) allows the named hand card
    // (WarGreymon analog) to digivolve onto it for the literal alt-path
    // cost. The standard rules-based route would reject this (the level/
    // color mismatch), so a successful digivolve proves the alt-path
    // substrate took over.
    let yaml = into_direction_warp_yaml("WARP-TARGET", 4);
    let mut warp_target = colored_digimon_level("WARP-TARGET", CardColor::Red, 6);
    warp_target.dp = Some(12000);
    warp_target.play_cost = 12;
    // Empty evo_costs so the rules-based route cannot succeed — only the
    // Into-direction alt-path can satisfy this digivolve.
    warp_target.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("warp-into YAML compiles")
        .add_card(warp_target)
        .hand(0, &["WARP-TARGET"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "T-G7-INTO", None);
    let memory_before = runner.game.memory;

    // End-to-end: digivolve succeeds; carrier's top is now the hand card;
    // memory reduced by the alt-path cost. The standard rules route is
    // unavailable (empty evo_costs on WARP-TARGET), so a successful
    // digivolve proves the Into-direction substrate took over.
    assert!(
        runner.game.digivolve_from_hand(
            0,
            0,
            carrier.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "digivolve_from_hand must succeed for Into-direction warp"
    );
    let top = runner.game.players[0].battle_area[carrier.index as usize].top_card();
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "WARP-TARGET",
        "carrier top is now the warp target"
    );
    assert_eq!(
        memory_before - runner.game.memory,
        4,
        "memory cost matches alt-path cost"
    );
}

#[test]
fn alt_path_direction_into_filter_rejects_non_matching_hand_card() {
    // Negative case: the Into-direction alt-path filter restricts to a
    // single named card. A different hand card must NOT receive a route
    // via this path.
    let yaml = into_direction_warp_yaml("WARP-TARGET", 4);
    let mut other = colored_digimon_level("OTHER", CardColor::Red, 6);
    other.dp = Some(12000);
    other.play_cost = 12;
    other.evo_costs = Vec::new();

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("warp-into YAML compiles")
        .add_card(other)
        .hand(0, &["OTHER"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "T-G7-INTO", None);

    assert!(
        !runner.game.digivolve_from_hand(
            0,
            0,
            carrier.index as usize,
            digimon_engine::enums::PlaySource::ByHand,
        ),
        "Into-direction filter must reject hand cards that don't match `from:`"
    );
    // Top card unchanged.
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "T-G7-INTO"
    );
}

#[test]
fn end_turn_parks_for_registered_dna_and_decoder_routes_existing_action_id() {
    let yaml = registration_yaml(0);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("registration DSL compiles")
        .add_card(colored_digimon_level("BLUE-LV4", CardColor::Blue, 4))
        .add_card(colored_digimon_level("GREEN-LV4", CardColor::Green, 4))
        .add_card(colored_digimon_level("BLUE-LV5", CardColor::Blue, 5))
        .hand(0, &["BLUE-LV5"])
        .memory(3)
        .start();

    runner.place_stack(0, &["T-G7-ALT", "BLUE-LV4"]);
    runner.place_on_field(0, "GREEN-LV4", Some(0));

    runner.game.end_turn();
    assert_eq!(runner.game.current_phase, GamePhase::EndOfTurnAction);

    runner.game.decode_action(DNA_DIGIVOLVE_START, 0);
    assert_eq!(runner.game.current_phase, GamePhase::SelectMaterial);
    assert!(runner.game.pending_selection.is_some());
}
