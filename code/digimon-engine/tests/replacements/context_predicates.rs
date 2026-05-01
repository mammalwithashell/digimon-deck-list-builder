use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{EffectTiming, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
use digimon_engine::selection::SelectionKind;

struct CancelOnlyOpponentEffects(Arc<Mutex<u32>>);

impl CardEffect for CancelOnlyOpponentEffects {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .name("cancel only opponent effects")
            .replacement_condition(|ctx, _subject| {
                ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
                    && ctx.replacement_source_controller() != Some(ctx.player())
            })
            .replacement_process(move |rctx| {
                *count.lock().unwrap() += 1;
                rctx.cancel();
            })
            .build()]
    }
}

struct OptionalCardSubjectReplacement;

impl CardEffect for OptionalCardSubjectReplacement {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_place_in_security(card)
            .name("optional card subject replacement")
            .optional()
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}

struct DeleteOpponentGateOnPlay;

impl CardEffect for DeleteOpponentGateOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("delete opponent gate")
            .process(|ctx| {
                let target_index = ctx
                    .game
                    .player(0)
                    .battle_area
                    .iter()
                    .position(|perm| perm.top_card().card_id(&ctx.game.card_data) == "GATE");
                let Some(index) = target_index else {
                    return;
                };
                ctx.game.delete_permanent_with_cause(
                    PermanentHandle {
                        player: 0,
                        index: index as u8,
                    },
                    ReplacementCause::OpponentEffect,
                );
            })
            .build()]
    }
}

#[test]
fn replacement_condition_can_gate_on_opponent_effect_cause() {
    let count = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("GATE", "Gate"))
        .start();
    r.register_effect("GATE", Arc::new(CancelOnlyOpponentEffects(count.clone())));

    r.game.set_effect_source_player_for_test(Some(0));
    let gate = r.place_on_field(0, "GATE", Some(0));
    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OwnEffect);
    assert_eq!(*count.lock().unwrap(), 0);
    assert_eq!(
        r.battle_area_size(0),
        0,
        "own-effect deletion is not cancelled"
    );

    r.game.set_effect_source_player_for_test(Some(1));
    let gate = r.place_on_field(0, "GATE", Some(0));
    r.game
        .delete_permanent_with_cause(gate, ReplacementCause::OpponentEffect);
    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "opponent-effect deletion is cancelled"
    );
    r.game.set_effect_source_player_for_test(None);
}

#[test]
fn queued_opponent_effect_populates_replacement_source_controller() {
    let count = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("GATE", "Gate"))
        .add_card(make_test_card("KILLER", "Killer"))
        .start();
    r.register_effect("GATE", Arc::new(CancelOnlyOpponentEffects(count.clone())));
    r.register_effect("KILLER", Arc::new(DeleteOpponentGateOnPlay));

    r.place_on_field(0, "GATE", Some(0));
    r.place_on_field(1, "KILLER", Some(0));

    r.fire_on_play(1, 0);

    assert_eq!(*count.lock().unwrap(), 1);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "queued opponent effect deletion is cancelled"
    );
}

#[test]
fn revealed_card_subject_controller_is_not_fallback_player_zero() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("WATCHER", "Watcher"))
        .add_card(make_test_card("REVEALED", "Revealed"))
        .deck(1, &["REVEALED"])
        .start();
    r.register_effect("WATCHER", Arc::new(OptionalCardSubjectReplacement));
    r.place_on_field(0, "WATCHER", Some(0));

    let revealed = r.game.reveal_top_deck(1, 1);
    assert_eq!(revealed.len(), 1);
    assert_eq!(
        r.game
            .owner_of_card(revealed[0])
            .expect("revealed card keeps owner"),
        1
    );

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldPlaceInSecurity,
        ReplacementSubject::Card(revealed[0], Zone::Reveal),
        ReplacementCause::OpponentEffect,
        Some(Zone::Security),
    );

    assert_eq!(outcome, ReplacementOutcome::None);
    let selection = r
        .pending_selection_view()
        .expect("optional replacement prompt is installed");
    assert_eq!(selection.kind, SelectionKind::Replacement);
    assert_eq!(
        selection.selecting_player, 1,
        "revealed card owner chooses the optional replacement"
    );
}

#[test]
fn replacement_boolean_predicates_fail_when_context_controller_is_missing() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("PRED", "Predicate"))
        .start();
    let ctx = EffectReadContext::new(&r.game, CardHandle(0), None, 0);

    let mut source_false = CompiledPredicate::default();
    source_false.replacement_source_is_opponent = Some(false);
    assert!(
        !eval_predicate(&source_false, &ctx, PredicateSubject::None),
        "missing source controller does not satisfy replacement_source_is_opponent: false"
    );

    let mut subject_false = CompiledPredicate::default();
    subject_false.replacement_subject_is_mine = Some(false);
    assert!(
        !eval_predicate(&subject_false, &ctx, PredicateSubject::None),
        "missing subject controller does not satisfy replacement_subject_is_mine: false"
    );
}

#[test]
fn dsl_active_when_can_gate_ex9_032_style_opponent_effect_cause() {
    let mut r = runner_from_dsl(EX9_032_YAML, "EX9-032");

    let subject = r.place_on_field(0, "EX9-032", Some(0));
    r.game.set_effect_source_player_for_test(Some(0));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);
    assert_eq!(r.battle_area_size(0), 0, "own-effect deletion proceeds");

    let subject = r.place_on_field(0, "EX9-032", Some(0));
    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "opponent-effect deletion is cancelled"
    );
}

#[test]
fn dsl_active_when_can_gate_ex7_027_style_opponent_source_controller() {
    let mut r = runner_from_dsl(EX7_027_YAML, "EX7-027");

    let subject = r.place_on_field(0, "EX7-027", Some(0));
    r.game.set_effect_source_player_for_test(Some(0));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert_eq!(r.battle_area_size(0), 0, "own-sourced deletion proceeds");

    let subject = r.place_on_field(0, "EX7-027", Some(0));
    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "opponent-sourced deletion is cancelled"
    );
}

#[test]
fn dsl_active_when_can_gate_bt22_036_style_subject_controller() {
    let mut r = runner_from_dsl(BT22_036_YAML, "BT22-036");
    let subject = r.place_on_field(0, "BT22-036", Some(0));
    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert_eq!(r.battle_area_size(0), 1, "my subject is cancelled");

    let mut r = runner_from_dsl(BT22_036_NOT_MINE_YAML, "BT22-036");
    let subject = r.place_on_field(0, "BT22-036", Some(0));
    r.game.set_effect_source_player_for_test(Some(1));
    r.game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.battle_area_size(0),
        0,
        "subject controller mismatch proceeds"
    );
}

fn runner_from_dsl(yaml: &str, card_id: &str) -> DebugRunner {
    let _ = card_id;
    DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline replacement DSL parses and compiles")
        .start()
}

const EX9_032_YAML: &str = r#"
card: EX9-032
name: EX9 Fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      replacement_cause: opponent_effect
    process:
      - cancel_replacement: {}
"#;

const EX7_027_YAML: &str = r#"
card: EX7-027
name: EX7 Fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      replacement_source_is_opponent: true
    process:
      - cancel_replacement: {}
"#;

const BT22_036_YAML: &str = r#"
card: BT22-036
name: BT22 Fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: true
    process:
      - cancel_replacement: {}
"#;

const BT22_036_NOT_MINE_YAML: &str = r#"
card: BT22-036
name: BT22 Fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: false
    process:
      - cancel_replacement: {}
"#;
