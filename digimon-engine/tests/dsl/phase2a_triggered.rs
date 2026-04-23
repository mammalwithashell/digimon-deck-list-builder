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
