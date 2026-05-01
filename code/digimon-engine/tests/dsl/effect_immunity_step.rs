use digimon_dsl::compiled::{CompiledBindingRef, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::scheduled_effects::fire_scheduled_for_timing;
use digimon_engine::{
    CardData, CardKind, EffectSourceKind, EffectTiming, Expiry, Keyword, TriggerSource,
};

#[test]
fn dsl_grants_opponent_digimon_effect_immunity_to_self() {
    let yaml = r#"
card: TST-IMMUNE
name: Immunity Test
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    process:
      - grant_effect_immunity:
          target: self
          source_kind: digimon
          source_controller: opponent
          expiry: end_of_opponents_turn
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DSL compiles")
        .build();
    let handle = runner.place_on_field(0, "TST-IMMUNE", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(handle, 1, EffectSourceKind::Digimon),
        "opponent Digimon effects should not affect the target"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(handle, 1, EffectSourceKind::Option),
        "opponent Option effects should still affect the target"
    );
}

fn card_with_kind(id: &str, kind: CardKind) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    card
}

fn grant_blocker_step(target: &str) -> CompiledStep {
    CompiledStep::GrantKeyword {
        target: CompiledBindingRef::Named(target.to_string()),
        keyword: "Blocker".to_string(),
        expiry: "end_of_turn".to_string(),
        value: None,
    }
}

#[test]
fn grant_keyword_respects_source_kind_effect_immunity() {
    let mut runner = DebugRunner::builder()
        .add_card(card_with_kind("TARGET", CardKind::Digimon))
        .add_card(card_with_kind("DIGI-SOURCE", CardKind::Digimon))
        .add_card(card_with_kind("OPT-SOURCE", CardKind::Option))
        .hand(1, &["OPT-SOURCE"])
        .build();
    let target = runner.place_on_field(0, "TARGET", None);
    let digimon_source = runner.place_on_field(1, "DIGI-SOURCE", None);
    runner.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let digimon_source_card = runner.game.players[1].battle_area[0].top_card().handle();
    {
        let mut ctx = EffectContext::new_with_source_kind(
            &mut runner.game,
            digimon_source_card,
            Some(digimon_source),
            EffectSourceKind::Digimon,
            1,
        );
        ctx.grant_keyword(target, Keyword::Blocker, Expiry::EndOfTurn);
    }
    assert!(
        !runner.game.has_keyword(target, Keyword::Blocker),
        "opponent Digimon-sourced keyword grants should be blocked by Digimon-effect immunity"
    );

    let option_source_card = runner.game.players[1].hand[0].handle();
    {
        let mut ctx = EffectContext::new_with_source_kind(
            &mut runner.game,
            option_source_card,
            None,
            EffectSourceKind::Option,
            1,
        );
        ctx.grant_keyword(target, Keyword::Blocker, Expiry::EndOfTurn);
    }
    assert!(
        runner.game.has_keyword(target, Keyword::Blocker),
        "opponent Option-sourced keyword grants should pass a Digimon-only immunity filter"
    );
}

#[test]
fn scheduled_effect_preserves_context_sensitive_source_kind() {
    let mut runner = DebugRunner::builder()
        .add_card(card_with_kind("TARGET", CardKind::Digimon))
        .add_card(card_with_kind("DUAL-SOURCE", CardKind::Dual))
        .build();
    let target = runner.place_on_field(0, "TARGET", None);
    let dual_source = runner.place_on_field(1, "DUAL-SOURCE", None);
    runner.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let source_card = runner.game.players[1].battle_area[0].top_card().handle();
    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);
    {
        let mut ctx = EffectContext::new_with_source_kind(
            &mut runner.game,
            source_card,
            Some(dual_source),
            EffectSourceKind::Option,
            1,
        );
        ctx.schedule_delayed(
            EffectTiming::EndOfYourTurn,
            vec![grant_blocker_step("tgt")],
            bindings,
        );
    }

    fire_scheduled_for_timing(&mut runner.game, EffectTiming::EndOfYourTurn);

    assert!(
        runner.game.has_keyword(target, Keyword::Blocker),
        "a delayed DUAL Option-face effect should replay as Option-sourced, not infer Digimon from the DUAL permanent"
    );
}
