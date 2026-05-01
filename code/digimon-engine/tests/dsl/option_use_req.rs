use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::OptionPlayResult;
use digimon_engine::{CardColor, CardData, CardKind, EffectReadContext};

fn beatbreak_use_req_yaml() -> &'static str {
    r#"
card: TST-PRED-SOURCE
name: Predicate Source
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: { of: you, count: 1 }
"#
}

fn beatbreak_dual_use_req_yaml() -> &'static str {
    r#"
card: TST-DUAL-USE-REQ
name: Dual Use Req Test
kind: dual
level: 6
color: [red]
dp: 12000
traits: [Test]
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [red]
    traits: [Test]
    effect_text: ""
    inherited_text: ""
  option:
    use_cost: 5
    colors: [green]
    use_requirement:
      any_field_permanent:
        of: you
        any_of:
          - kind: digimon
          - kind: tamer
        trait_has: BEATBREAK
    effect_text: ""
    security_text: ""
    keywords: []
effects:
  - when: main_from_hand
    process:
      - draw: { of: you, count: 1 }
"#
}

fn beatbreak_dual_top_level_use_req_yaml() -> &'static str {
    r#"
card: TST-DUAL-TOP-USE-REQ
name: Dual Top-Level Use Req Test
kind: dual
level: 6
color: [red]
dp: 12000
traits: [Test]
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [red]
    traits: [Test]
    effect_text: ""
    inherited_text: ""
  option:
    use_cost: 5
    colors: [green]
    effect_text: ""
    security_text: ""
    keywords: []
effects:
  - when: main_from_hand
    process:
      - draw: { of: you, count: 1 }
"#
}

fn beatbreak_use_req() -> digimon_dsl::compiled::CompiledPredicate {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(beatbreak_use_req_yaml()).expect("parse predicate carrier card");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile predicate carrier card");
    compiled
        .use_requirement
        .expect("compiled option use requirement")
}

fn beatbreak_egg(id: &str) -> CardData {
    let mut card = make_test_card(id, "Beatbreak Egg");
    card.card_kind = CardKind::DigiEgg;
    card.level = Some(2);
    card.traits = vec!["BEATBREAK".to_string()];
    card
}

fn off_trait_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, "Plain Rookie");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.traits = vec!["Plain".to_string()];
    card
}

fn beatbreak_lv3(id: &str) -> CardData {
    let mut card = make_test_card(id, "Beatbreak Rookie");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.traits = vec!["BEATBREAK".to_string()];
    card
}

fn beatbreak_option_permanent(id: &str) -> CardData {
    let mut card = make_test_card(id, "Beatbreak Option Permanent");
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 0;
    card.colors = vec![CardColor::Green];
    card.traits = vec!["BEATBREAK".to_string()];
    card
}

#[test]
fn parses_and_compiles_any_field_permanent() {
    let yaml = r#"
card: TST-USE-REQ
name: Use Req Test
kind: option
color: [green]
cost: 5
use_requirement:
  any_field_permanent:
    of: you
    any_of:
      - kind: digimon
      - kind: tamer
    trait_has: BEATBREAK
effects:
  - when: main_from_hand
    process:
      - draw: { of: you, count: 1 }
"#;

    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("parse use requirement yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile use requirement yaml");
    let use_req = compiled
        .use_requirement
        .as_ref()
        .expect("compiled option use requirement");
    let ex = use_req
        .any_field_permanent
        .as_ref()
        .expect("compiled any_field_permanent");

    assert!(ex
        .predicate
        .any_of
        .iter()
        .any(|p| p.kind == Some(CompiledCardKind::Digimon)));
    assert!(ex
        .predicate
        .any_of
        .iter()
        .any(|p| p.kind == Some(CompiledCardKind::Tamer)));
    assert_eq!(ex.predicate.trait_has.as_deref(), Some("BEATBREAK"));
}

#[test]
fn card_subject_kind_digimon_does_not_match_digi_egg() {
    let runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .hand(0, &["EGG-BEAT"])
        .start();
    let egg = runner.game.players[0].hand[0].handle();
    assert_eq!(
        runner
            .game
            .card_data_for_handle(egg)
            .expect("egg card data")
            .card_kind,
        CardKind::DigiEgg
    );
    let ctx = EffectReadContext::new(&runner.game, egg, None, 0);
    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..Default::default()
    };

    assert!(
        !digimon_engine::dsl_cards::predicate::eval_predicate(
            &pred,
            &ctx,
            digimon_engine::dsl_cards::predicate::PredicateSubject::Card(egg),
        ),
        "ordinary card predicates should not treat Digi-Egg as Digimon"
    );
}

#[test]
fn any_field_permanent_sees_traited_egg_in_breeding() {
    let use_req = beatbreak_use_req();

    let mut runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(off_trait_lv3("PLAIN-LV3"))
        .digitama(0, &["EGG-BEAT"])
        .start();
    let pre_hatch_source = runner.game.players[0].digitama_deck[0].handle();
    let pre_hatch_ctx = EffectReadContext::new(&runner.game, pre_hatch_source, None, 0);
    assert!(
        !digimon_engine::dsl_cards::predicate::eval_predicate(
            &use_req,
            &pre_hatch_ctx,
            digimon_engine::dsl_cards::predicate::PredicateSubject::None,
        ),
        "field scan should fail before the egg is in breeding"
    );

    assert!(runner.game.hatch(0), "egg should move into breeding");
    let source = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding permanent")
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);

    assert!(digimon_engine::dsl_cards::predicate::eval_predicate(
        &use_req,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::None,
    ));
}

#[test]
fn any_field_permanent_sees_traited_breeding_stack() {
    let use_req = beatbreak_use_req();

    let mut evo = beatbreak_lv3("LV3-BEAT");
    evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0,
        level: 2,
        memory_cost: 0,
    }];
    let mut runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(evo)
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["LV3-BEAT"])
        .deck(0, &["LV3-BEAT"])
        .memory(10)
        .start();
    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();
    assert!(
        runner.game.digivolve_from_hand_onto_breeding(
            0,
            0,
            digimon_engine::PlaySource::ByDigivolve
        ),
        "LV3 should digivolve onto the egg in breeding"
    );
    let source = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding stack")
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);

    assert!(digimon_engine::dsl_cards::predicate::eval_predicate(
        &use_req,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::None,
    ));

    let mut off_trait_evo = off_trait_lv3("LV3-PLAIN");
    off_trait_evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: 0,
        level: 2,
        memory_cost: 0,
    }];
    let mut runner = DebugRunner::builder()
        .add_card(beatbreak_egg("EGG-BEAT"))
        .add_card(off_trait_evo)
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["LV3-PLAIN"])
        .deck(0, &["LV3-PLAIN"])
        .memory(10)
        .start();
    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();
    assert!(
        runner.game.digivolve_from_hand_onto_breeding(
            0,
            0,
            digimon_engine::PlaySource::ByDigivolve
        ),
        "off-trait LV3 should digivolve onto the egg in breeding"
    );
    let source = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("off-trait breeding stack")
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);

    assert!(
        !digimon_engine::dsl_cards::predicate::eval_predicate(
            &use_req,
            &ctx,
            digimon_engine::dsl_cards::predicate::PredicateSubject::None,
        ),
        "field scan should evaluate only the breeding stack top card, not buried egg sources"
    );
}

#[test]
fn use_req_allows_option_without_matching_color_when_field_trait_exists() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(beatbreak_use_req_yaml())
        .expect("inline use req DSL compiles")
        .add_card(beatbreak_egg("EGG-BEAT"))
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["TST-PRED-SOURCE"])
        .memory(10)
        .start();

    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "use requirement should satisfy option use even without a matching green Digimon/Tamer"
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Trashed,
        "execution legality should match the action mask"
    );
}

#[test]
fn use_req_allows_dual_option_face_without_matching_color_when_field_trait_exists() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(beatbreak_dual_use_req_yaml())
        .expect("inline dual use req DSL compiles")
        .add_card(beatbreak_egg("EGG-BEAT"))
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["TST-DUAL-USE-REQ"])
        .memory(10)
        .start();

    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "dual option-face use requirement should satisfy option use without matching green color"
    );
    assert_ne!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "execution legality should accept the dual option-face use requirement"
    );
}

#[test]
fn top_level_use_req_does_not_bypass_dual_option_face_color_requirement() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(beatbreak_dual_top_level_use_req_yaml())
        .expect("inline dual top-level use req DSL compiles")
        .add_card(beatbreak_egg("EGG-BEAT"))
        .digitama(0, &["EGG-BEAT"])
        .hand(0, &["TST-DUAL-TOP-USE-REQ"])
        .memory(10)
        .start();

    assert!(runner.game.hatch(0), "egg should move into breeding");
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "top-level use_requirement on a DUAL card should not satisfy option-face color legality"
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "execution legality should ignore top-level DUAL use_requirement"
    );
}

#[test]
fn use_req_does_not_count_option_permanents_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(beatbreak_use_req_yaml())
        .expect("inline use req DSL compiles")
        .add_card(beatbreak_option_permanent("OPT-BEAT"))
        .hand(0, &["TST-PRED-SOURCE"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OPT-BEAT", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "BEATBREAK Option permanents should not satisfy a Digimon-or-Tamer use requirement"
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "execution legality should reject the same false use requirement"
    );
}
