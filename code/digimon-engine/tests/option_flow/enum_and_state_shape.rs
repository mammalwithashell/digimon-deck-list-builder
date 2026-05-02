use digimon_engine::enums::{DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionResolutionPhase, OptionUseSource, PendingOption};

#[test]
fn option_timings_exist() {
    let _ = EffectTiming::OnUseOption;
    let _ = EffectTiming::OptionMain; // already existed — smoke
    let _ = EffectTiming::DelayEffect;
    let _ = EffectTiming::OnLink;
    let _ = EffectTiming::OnLinkedCardTrashed;
    let _ = EffectTiming::OnUnlink;
    let _ = EffectTiming::OnTrainingTrash;
}

#[test]
fn delay_trigger_variants_exist() {
    let _ = DelayTrigger::EndOfYourNextTurn;
    let _ = DelayTrigger::EndOfThisTurn;
    let _ = DelayTrigger::StartOfYourNextTurn;
}

#[test]
fn option_state_default_is_standard() {
    assert_eq!(OptionState::default(), OptionState::Standard);
}

#[test]
fn option_state_variants_exist() {
    let h = PermanentHandle {
        player: 0,
        index: 0,
    };
    let _ = OptionState::Standard;
    let _ = OptionState::Delayed {
        owner: 0,
        trash_on_turn: 5,
        trigger: DelayTrigger::EndOfYourNextTurn,
    };
    let _ = OptionState::Linked { host: h };
    let _ = OptionState::Training { owner: 0 };
}

#[test]
fn option_resolution_phase_variants_exist() {
    let _ = OptionResolutionPhase::MainEffectDrain;
    let _ = OptionResolutionPhase::Disposing;
    let _ = OptionResolutionPhase::LinkSelectHost;
    let _ = OptionResolutionPhase::Done;
}

#[test]
fn pending_option_struct_exists() {
    use digimon_engine::card_source::CardSource;
    let cs = CardSource::new(0, 0, 0);
    let po = PendingOption {
        owner: 0,
        card: cs,
        source_kind: OptionUseSource::Hand,
        resolution_phase: OptionResolutionPhase::MainEffectDrain,
    };
    assert_eq!(po.owner, 0);
    assert_eq!(po.resolution_phase, OptionResolutionPhase::MainEffectDrain);
}

#[test]
fn permanent_default_option_state_is_standard() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::permanent::Permanent;

    let cs = CardSource::new(0, 0, 0);
    let perm = Permanent::new(cs, 1);
    assert_eq!(perm.option_state, OptionState::Standard);
    assert!(perm.linked_cards.is_empty());
}

#[test]
fn effect_builder_option_main_sets_flag() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::EffectBuilder;

    let card = CardHandle(0);
    let eff = EffectBuilder::new(card, EffectTiming::None)
        .option_main()
        .build();
    assert_eq!(eff.timing, EffectTiming::OptionMain);
    assert!(eff.option_main);
}

#[test]
fn effect_builder_delay_sets_trigger() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::EffectBuilder;

    let card = CardHandle(0);
    let eff = EffectBuilder::new(card, EffectTiming::None)
        .delay(DelayTrigger::EndOfYourNextTurn)
        .build();
    assert_eq!(eff.timing, EffectTiming::DelayEffect);
    assert_eq!(eff.delay_trigger, Some(DelayTrigger::EndOfYourNextTurn));
}

#[test]
fn effect_builder_link_stores_cost_and_filter() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::EffectBuilder;

    let card = CardHandle(0);
    let eff = EffectBuilder::new(card, EffectTiming::None)
        .link(2, |_ctx, _h| true)
        .build();
    assert_eq!(eff.link_cost, Some(2));
    assert!(eff.link_filter.is_some());
}

#[test]
fn effect_builder_training_sets_flag() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::effect::EffectBuilder;

    let card = CardHandle(0);
    let eff = EffectBuilder::new(card, EffectTiming::None)
        .training()
        .build();
    assert!(eff.training);
}

#[test]
fn game_pending_option_default_is_none() {
    use digimon_engine::debug_runner::DebugRunner;

    let r = DebugRunner::builder().start();
    assert!(r.game.pending_option.is_none());
}
