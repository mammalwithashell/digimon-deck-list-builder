use std::sync::Arc;

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CostDelta, EffectTiming, PlaySource};
use digimon_engine::logger::VerboseLogger;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::rules::Rules;
use digimon_engine::selection::OptionPlayResult;

fn card(card_id: &str, kind: CardKind, play_cost: u16, traits: &[&str]) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(7),
        dp: matches!(kind, CardKind::Digimon).then_some(15000),
        play_cost,
        colors: vec![CardColor::White],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn digimon(card_id: &str, play_cost: u16, traits: &[&str]) -> CardData {
    card(card_id, CardKind::Digimon, play_cost, traits)
}

fn tamer(card_id: &str) -> CardData {
    card(card_id, CardKind::Tamer, 4, &[])
}

fn egg(card_id: &str, traits: &[&str]) -> CardData {
    card(card_id, CardKind::DigiEgg, 0, traits)
}

fn setup_runner() -> DebugRunner {
    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(digimon(
            "AD1-025",
            15,
            &["Holy Warrior", "Royal Knight", "ADVENTURE", "Hero"],
        ))
        .add_card(egg("BT13-007", &[]))
        .add_card(tamer("ST21-13"))
        .add_card(egg("SRC-1", &[]))
        .add_card(egg("SRC-2", &[]))
        .hand(0, &["AD1-025"])
        .memory(20)
        .start();

    r.register_effect("BT13-007", Arc::new(DrasilReducer));
    r.register_effect("ST21-13", Arc::new(MattTkReducer));
    r.register_effect("AD1-025", Arc::new(NoEffect));
    r.register_effect("SRC-1", Arc::new(NoEffect));
    r.register_effect("SRC-2", Arc::new(NoEffect));
    r.place_in_breeding(0, "BT13-007");
    add_breeding_source(&mut r, 0, "SRC-1");
    add_breeding_source(&mut r, 0, "SRC-2");
    r.place_on_field(0, "ST21-13", Some(0));
    r.game_mut().enter_main_phase();
    r
}

fn setup_mandatory_paid_reducer_runner() -> DebugRunner {
    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(digimon("AD1-025", 15, &["Royal Knight"]))
        .add_card(tamer("PAID-REDUCER"))
        .hand(0, &["AD1-025"])
        .memory(20)
        .start();

    r.register_effect("AD1-025", Arc::new(NoEffect));
    r.register_effect("PAID-REDUCER", Arc::new(MandatoryPaidReducer));
    r.place_on_field(0, "PAID-REDUCER", Some(0));
    r.game_mut().enter_main_phase();
    r
}

fn setup_transit_reducer_runner() -> DebugRunner {
    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(digimon("AD1-025", 15, &["Royal Knight"]))
        .add_card(tamer("OPTIONAL-REDUCER"))
        .hand(0, &["AD1-025"])
        .security(0, &["AD1-025"])
        .memory(20)
        .start();

    r.register_effect("AD1-025", Arc::new(NoEffect));
    r.register_effect("OPTIONAL-REDUCER", Arc::new(OptionalRoyalKnightReducer));
    r.place_on_field(0, "OPTIONAL-REDUCER", Some(0));
    r.game_mut().enter_main_phase();
    r
}

fn setup_legacy_option_scan_runner() -> DebugRunner {
    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(card("OPT-001", CardKind::Option, 5, &[]))
        .add_card(tamer("OPTIONAL-REDUCER"))
        .hand(0, &["OPT-001"])
        .memory(20)
        .start();

    r.register_effect("OPT-001", Arc::new(NoEffect));
    r.register_effect("OPTIONAL-REDUCER", Arc::new(AlwaysOptionalReducer));
    r.place_on_field(0, "OPTIONAL-REDUCER", Some(0));
    r.game.set_logger(Box::new(VerboseLogger::new()));
    r.game_mut().enter_main_phase();
    r
}

fn setup_yaml_drasil_runner() -> DebugRunner {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(include_str!("../../cards/_examples/BT13-007.yaml"))
            .expect("BT13-007 example YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("BT13-007 example compiles");
    let mut raw = EngineRawRustRegistry::new();
    raw.register_formula("bt13_007_royal_knight_cost_reduction", |ctx, target| {
        assert_eq!(
            ctx.cost_reduction_source(),
            Some(target),
            "formula target should be the reducer source"
        );
        assert_eq!(ctx.cost_target_card_id(), Some("AD1-025"));
        assert!(ctx.cost_target_has_trait("Royal Knight"));
        assert!(ctx.cost_target_from_hand());
        4 + ctx.source_stack_source_count() as i32
    });

    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(digimon("AD1-025", 15, &["Royal Knight"]))
        .add_card(egg("BT13-007", &[]))
        .add_card(egg("SRC-1", &[]))
        .add_card(egg("SRC-2", &[]))
        .hand(0, &["AD1-025"])
        .memory(20)
        .start();

    r.register_effect(
        "BT13-007",
        Arc::new(DslCardEffect::new_with_raw(
            Arc::new(compiled),
            Arc::new(raw),
        )),
    );
    r.register_effect("AD1-025", Arc::new(NoEffect));
    r.register_effect("SRC-1", Arc::new(NoEffect));
    r.register_effect("SRC-2", Arc::new(NoEffect));
    r.place_in_breeding(0, "BT13-007");
    add_breeding_source(&mut r, 0, "SRC-1");
    add_breeding_source(&mut r, 0, "SRC-2");
    r.game_mut().enter_main_phase();
    r
}

fn add_breeding_source(r: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    let source = CardSource::new(data_idx, player, card_index);
    let breeding = r.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    breeding.card_sources.insert(0, source);
}

fn breeding_material_count(ctx: &digimon_engine::effect_context::EffectReadContext<'_>) -> i32 {
    ctx.source_stack_source_count() as i32
}

struct DrasilReducer;
impl CardEffect for DrasilReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("King Drasil Royal Knight play reduction")
            .optional()
            .once_per_turn()
            .condition(|ctx| {
                ctx.cost_target_from_hand() && ctx.cost_target_has_trait("Royal Knight")
            })
            .cost_reduction_fn(|ctx| 4 + breeding_material_count(ctx))
            .build()]
    }
}

struct NoEffect;
impl CardEffect for NoEffect {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        Vec::new()
    }
}

struct MattTkReducer;
impl CardEffect for MattTkReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("Matt Ishida & T.K. Takaishi ADVENTURE play reduction")
            .optional()
            .condition(|ctx| {
                ctx.cost_target_has_trait("ADVENTURE")
                    && ctx
                        .source_permanent()
                        .is_some_and(|perm| !perm.is_suspended)
            })
            .cost_reduction(1)
            .pay_cost_fn(|ctx| {
                let Some(source) = ctx.cost_reduction_source() else {
                    return false;
                };
                ctx.suspend(source);
                true
            })
            .build()]
    }
}

struct MandatoryPaidReducer;
impl CardEffect for MandatoryPaidReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("Mandatory paid reducer")
            .condition(|ctx| ctx.cost_target_has_trait("Royal Knight"))
            .cost_reduction(2)
            .pay_cost_fn(|ctx| {
                let Some(source) = ctx.cost_reduction_source() else {
                    return false;
                };
                ctx.suspend(source);
                true
            })
            .build()]
    }
}

struct OptionalRoyalKnightReducer;
impl CardEffect for OptionalRoyalKnightReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("Optional Royal Knight reducer")
            .optional()
            .condition(|ctx| ctx.cost_target_has_trait("Royal Knight"))
            .cost_reduction(6)
            .build()]
    }
}

struct AlwaysOptionalReducer;
impl CardEffect for AlwaysOptionalReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("Always optional reducer")
            .optional()
            .cost_reduction(4)
            .build()]
    }
}

fn assert_ad1_played(r: &DebugRunner) {
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&r.game.card_data) == "AD1-025"),
        "AD1-025 should be in the battle area"
    );
}

fn matt_tk_handle() -> PermanentHandle {
    PermanentHandle {
        player: 0,
        index: 0,
    }
}

#[test]
fn security_transit_play_keeps_card_parked_while_reducer_choice_is_pending() {
    let mut r = setup_transit_reducer_runner();
    r.game.players[0].hand.clear();

    let source = r.game.players[0].battle_area[0].top_card().handle();
    let result = {
        let mut ctx = EffectContext::new(
            &mut r.game,
            source,
            Some(PermanentHandle {
                player: 0,
                index: 0,
            }),
            0,
        );
        ctx.play_from_security(0)
    };

    assert!(
        result.is_none(),
        "security play should park behind the optional reducer choice"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.game.players[0].security.len(), 0);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "AD1-025"
    );

    r.execute_branch(0).expect("accept security-play reducer");

    assert_eq!(r.memory(), 20, "free security play remains free");
    assert_eq!(r.game.players[0].security.len(), 0);
    assert_eq!(r.game.players[0].hand.len(), 0);
    assert_ad1_played(&r);
}

#[test]
fn trash_play_surfaces_optional_play_cost_reducer_choice() {
    let mut r = setup_transit_reducer_runner();
    let card = r.game.players[0].hand.remove(0);
    r.game.players[0].trash.push(card);

    let played = r
        .game
        .play_from_trash_with_cost(0, 0, CostDelta::Reduce(0), PlaySource::ByEffect);

    assert!(
        played.is_none(),
        "trash play should park behind the optional reducer choice"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.memory(), 20, "memory is not paid before acceptance");
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(
        r.game.players[0].hand.len(),
        1,
        "card remains staged for the pending play callback"
    );

    r.execute_branch(0).expect("accept trash-play reducer");

    assert_eq!(r.memory(), 11, "15 - 6 = 9 paid from memory 20");
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].hand.len(), 0);
    assert_ad1_played(&r);
}

#[test]
fn legacy_non_play_cost_scan_logs_optional_reducer_guard() {
    let mut r = setup_legacy_option_scan_runner();

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, OptionPlayResult::Trashed);
    assert_eq!(
        r.memory(),
        15,
        "Option cost path documents the limitation and pays full cost"
    );
    assert!(r.pending_selection().is_none());
    assert!(
        r.game.logger.get_logs().iter().any(|line| line.contains(
            "optional/paid BeforePayCost reducer requires explicit pending play-cost context"
        )),
        "legacy non-play cost paths should not silently drop optional reducers"
    );
}

#[test]
fn mandatory_paid_reducer_requires_explicit_acceptance_before_pay_cost() {
    let mut r = setup_mandatory_paid_reducer_runner();

    assert!(
        r.play(0, 0).is_none(),
        "paid reducer should park behind an explicit cost-payment choice"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.memory(), 20, "memory must not be paid before acceptance");
    assert!(
        !r.game.players[0].battle_area[0].is_suspended,
        "pay_cost_fn must not run during collection or auto-apply"
    );

    r.execute_branch(0).expect("accept paid reducer");

    assert_eq!(r.memory(), 7, "15 - 2 = 13 paid from memory 20");
    assert_ad1_played(&r);
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "pay_cost_fn runs after explicit acceptance"
    );
}

#[test]
fn ad1_025_can_accept_drasil_then_matt_tk_reductions() {
    let mut r = setup_runner();

    let played = r.play(0, 0);
    assert!(
        played.is_none(),
        "play should park before memory payment while optional reducers are pending"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.memory(), 20, "memory is not paid before reducer choices");

    r.execute_branch(0).expect("accept Drasil");
    assert!(
        r.pending_selection().is_some(),
        "accepting Drasil should continue to the Matt/T.K. reducer"
    );
    r.execute_branch(0).expect("accept Matt/T.K.");

    assert_eq!(r.memory(), 12, "15 - 6 - 1 = 8 paid from memory 20");
    assert_ad1_played(&r);
    assert!(
        r.game.players[0].battle_area[matt_tk_handle().index as usize].is_suspended,
        "ST21-13 should be suspended after accepting its reducer"
    );
}

#[test]
fn declining_matt_tk_keeps_only_drasil_reduction() {
    let mut r = setup_runner();

    assert!(r.play(0, 0).is_none());
    r.execute_branch(0).expect("accept Drasil");
    r.execute_action(0, PASS).expect("decline Matt/T.K.");

    assert_eq!(r.memory(), 11, "15 - 6 = 9 paid from memory 20");
    assert_ad1_played(&r);
    assert!(
        !r.game.players[0].battle_area[matt_tk_handle().index as usize].is_suspended,
        "ST21-13 should remain ready after declining its reducer"
    );
}

#[test]
fn drasil_observer_cost_reduction_lowers_from_yaml() {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(include_str!("../../cards/_examples/BT13-007.yaml"))
            .expect("BT13-007 example YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("BT13-007 example compiles");
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));

    let reducer = effects
        .iter()
        .find(|effect| effect.timing == EffectTiming::BeforePayCost)
        .expect("BT13-007 should emit a BeforePayCost reducer");
    assert!(reducer.optional, "Drasil reducer is optional");
    assert_eq!(reducer.max_per_turn, 1, "Drasil reducer is once per turn");
    assert!(
        !reducer.when_playing_this,
        "Drasil observes allies being played, not itself being played"
    );
    assert!(
        reducer.condition.is_some(),
        "Drasil reducer has a condition"
    );
    assert!(
        reducer.cost_reduction_fn.is_some(),
        "Drasil reducer has a reduction function"
    );

    let mut r = setup_yaml_drasil_runner();
    assert!(
        r.play(0, 0).is_none(),
        "YAML-lowered Drasil should park as an optional reducer"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.memory(), 20);

    r.execute_branch(0).expect("accept YAML-lowered Drasil");

    assert_eq!(r.memory(), 11, "15 - (4 + 2 sources) = 9");
    assert_ad1_played(&r);
}

#[test]
fn drasil_production_structured_formula_matches_raw_rust() {
    // The PRODUCTION BT13-007 (cards/bt13/) reduces Royal Knight play cost via a
    // structured `amount_fn: { base: 4, per: material_count, delta: 1 }` instead
    // of the raw_rust escape hatch the _examples copy uses. It MUST reduce
    // identically (4 + digivolution cards under King Drasil) — proving the
    // migration off the test-only-registered raw_rust fn preserved behavior
    // (fix-dsl-substrate-rot-and-bugs: BT13-007 was a silent no-op in production).
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(include_str!("../../cards/bt13/BT13-007.yaml"))
            .expect("production BT13-007 YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("production BT13-007 compiles");

    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));
    let reducer = effects
        .iter()
        .find(|effect| effect.timing == EffectTiming::BeforePayCost)
        .expect("structured BT13-007 emits a BeforePayCost reducer");
    assert!(
        reducer.cost_reduction_fn.is_some(),
        "structured Drasil uses a formula reduction (not a flat literal or raw_rust)"
    );

    let mut rules = Rules::standard();
    rules.memory_range = (-20, 20);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .add_card(digimon("AD1-025", 15, &["Royal Knight"]))
        .add_card(egg("BT13-007", &[]))
        .add_card(egg("SRC-1", &[]))
        .add_card(egg("SRC-2", &[]))
        .hand(0, &["AD1-025"])
        .memory(20)
        .start();

    r.register_effect("BT13-007", Arc::new(dsl));
    r.register_effect("AD1-025", Arc::new(NoEffect));
    r.register_effect("SRC-1", Arc::new(NoEffect));
    r.register_effect("SRC-2", Arc::new(NoEffect));
    r.place_in_breeding(0, "BT13-007");
    add_breeding_source(&mut r, 0, "SRC-1");
    add_breeding_source(&mut r, 0, "SRC-2");
    r.game_mut().enter_main_phase();

    assert!(
        r.play(0, 0).is_none(),
        "structured Drasil parks as an optional reducer"
    );
    assert!(r.pending_selection().is_some());
    assert_eq!(r.memory(), 20);

    r.execute_branch(0).expect("accept structured Drasil");

    assert_eq!(
        r.memory(),
        11,
        "15 - (4 + 2 sources) = 9 — structured formula matches the former raw_rust"
    );
    assert_ad1_played(&r);
}

#[test]
fn observer_cost_reduction_preserves_compiled_optional_flag() {
    let observer = digimon_dsl::compiled::CompiledPredicate {
        trait_has: Some("Royal Knight".to_string()),
        ..Default::default()
    };
    let effect = digimon_engine::dsl_cards::lower_cost_reduction::lower_with_formula(
        CardHandle(0),
        digimon_dsl::compiled::CompiledScope::FaceUp,
        None,
        None,
        false,
        Some(digimon_dsl::compiled::CompiledFormula::Literal(3)),
        vec![],
        Arc::new(EngineRawRustRegistry::new()),
        false,
        false,
        Some(observer),
        None,
        None,
    );

    assert!(
        !effect.optional,
        "observer-shaped reducers must preserve the compiled optional flag"
    );
}
