// ── Task 1: timing_map ───────────────────────────────────────────────────────

use digimon_dsl::compiled::CompiledTiming;
use digimon_engine::dsl_cards::timing_map::compiled_timing_to_engine;
use digimon_engine::enums::EffectTiming;

#[test]
fn compiled_timing_mapping_covers_common_triggered_timings() {
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnPlay), Some(EffectTiming::OnPlay));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::WhenDigivolving), Some(EffectTiming::WhenDigivolving));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnAttack), Some(EffectTiming::OnAttack));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::EndOfYourTurn), Some(EffectTiming::EndOfYourTurn));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::StartOfYourTurn), Some(EffectTiming::StartOfYourTurn));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnSecurity), Some(EffectTiming::SecuritySkill));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::MainFromHand), Some(EffectTiming::MainFromHand));
    assert_eq!(compiled_timing_to_engine(CompiledTiming::BeforePayCost), Some(EffectTiming::BeforePayCost));
}

#[test]
fn compiled_timing_non_targets_return_none() {
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnAllyPlayed), None);
    assert_eq!(compiled_timing_to_engine(CompiledTiming::OnOptionPlaced), None);
    assert_eq!(compiled_timing_to_engine(CompiledTiming::Delayed), None);
}

// ── Task 2: bindings ─────────────────────────────────────────────────────────

use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::bindings::{BindingValue, Bindings};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bindings_round_trip_permanent_and_card_handles() {
    let mut b = Bindings::new();
    let perm = PermanentHandle { player: 0, index: 2 };
    let card = CardHandle(42);
    b.insert("tgt", BindingValue::Permanent(perm));
    b.insert("pick", BindingValue::Card(card));

    assert_eq!(b.get_permanent("tgt"), Some(perm));
    assert_eq!(b.get_card("pick"), Some(card));
    assert_eq!(b.get_permanent("pick"), None);
    assert_eq!(b.get_card("tgt"), None);
    assert_eq!(b.get_permanent("missing"), None);
}

// ── Task 3: step dispatcher + resolve_player ─────────────────────────────────

use digimon_dsl::compiled::CompiledPlayerRef;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::step::resolve_player;
use digimon_engine::effect_context::EffectContext;

#[test]
fn resolve_player_maps_compiled_player_refs() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("F", "F"))
        .hand(0, &["F"])
        .build();
    let card = runner.game.players[0].hand[0].handle();
    let opp;
    let active;
    {
        let ctx = EffectContext::new(&mut runner.game, card, None, 0);
        opp = ctx.opponent_id();
        active = ctx.game.turn_player();
        assert_eq!(resolve_player(&ctx, CompiledPlayerRef::You), 0);
        assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Opponent), opp);
        assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Active), active);
        assert_eq!(resolve_player(&ctx, CompiledPlayerRef::Any), 0);
    }
}

// ── Task 6: triggered clause lowering ────────────────────────────────────────

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledScope, CompiledStep,
    CompiledTriggeredClause,
};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

fn fixture_on_play_gain_memory(n: i32) -> CompiledCard {
    CompiledCard {
        card: "F-T1".into(),
        name: "Fixture".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(3),
        dp: Some(2000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Triggered(CompiledTriggeredClause {
            when: vec![CompiledTiming::OnPlay],
            scope: CompiledScope::FaceUp,
            active_when: None,
            condition: None,
            optional: false,
            once_per_turn: false,
            max_per_turn: None,
            process: vec![CompiledStep::GainMemory(n)],
            summary: Some("Gain N memory".into()),
            summary_key: None,
        })],
    }
}

#[test]
fn triggered_clause_emits_one_effect_per_timing() {
    let dsl = DslCardEffect::new(Arc::new(fixture_on_play_gain_memory(1)));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].timing, digimon_engine::enums::EffectTiming::OnPlay);
    assert!(effects[0].on_play);
}

#[test]
fn triggered_clause_with_multiple_timings_emits_one_effect_each() {
    let mut c = fixture_on_play_gain_memory(1);
    if let CompiledClause::Triggered(t) = &mut c.effects[0] {
        t.when = vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving];
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 2);
    let timings: Vec<_> = effects.iter().map(|e| e.timing).collect();
    assert!(timings.contains(&digimon_engine::enums::EffectTiming::OnPlay));
    assert!(timings.contains(&digimon_engine::enums::EffectTiming::WhenDigivolving));
}

#[test]
fn triggered_clause_once_per_turn_sets_max_per_turn_to_one() {
    let mut c = fixture_on_play_gain_memory(1);
    if let CompiledClause::Triggered(t) = &mut c.effects[0] {
        t.once_per_turn = true;
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects[0].max_per_turn, 1);
}

#[test]
fn triggered_clause_skips_non_target_timings() {
    // OnAllyPlayed is a DSL-only virtual timing — timing_map returns None,
    // so the clause should not emit.
    let mut c = fixture_on_play_gain_memory(1);
    if let CompiledClause::Triggered(t) = &mut c.effects[0] {
        t.when = vec![CompiledTiming::OnAllyPlayed];
    }
    let dsl = DslCardEffect::new(Arc::new(c));
    assert!(dsl.effects(CardHandle(0)).is_empty());
}
