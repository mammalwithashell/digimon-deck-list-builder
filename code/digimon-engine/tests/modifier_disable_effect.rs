//! Track C taxonomy (2026-05-06) — `ModifierType::DisableEffect` end-to-end
//! integration tests.
//!
//! A `DisableEffect` modifier carries a `disable_effect_timing` parameter on
//! the entry. The dispatch hook in `effect_queue.rs::permanent_activation_blocked_for_timing`
//! suppresses dispatch of THAT specific timing on the carrier permanent;
//! other timings on the same permanent fire normally.
//!
//! Tests live in their own integration test binary (rather than in
//! `timing_dispatch.rs`) because Windows UAC heuristically flags executable
//! file names containing "dispatch" as needing elevation, blocking
//! invocation under non-admin shells.

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

/// A Lv.3 Red Digimon with configurable play_cost and no inherent effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

/// `WhenAttacking → +1 memory` observer.
struct AttackingMemoryGain;
impl CardEffect for AttackingMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .name("+1 when attacking")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn disable_effect_modifier_suppresses_only_the_named_timing() {
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(AttackingMemoryGain));

    r.play(0, 0);
    r.play(0, 0);
    let observer_handle = PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.modifiers.add(
        observer_handle,
        ModifierEntry::disable_effect(EffectTiming::WhenAttacking, Expiry::Permanent, 1),
    );

    let attacker_handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    let result = r.attack_player(attacker_handle, 1, /* vortex */ true);
    assert_ne!(
        result,
        AttackResult::Invalid,
        "test setup must perform a legal attack before checking suppression"
    );
    assert_eq!(
        r.memory(),
        before,
        "DisableEffect{{WhenAttacking}} must suppress the observer's WhenAttacking gain"
    );
}

#[test]
fn disable_effect_modifier_for_a_different_timing_does_not_suppress_when_attacking() {
    // Negative-control: a `DisableEffect` whose timing param does NOT match
    // the firing timing must leave dispatch alone.
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(AttackingMemoryGain));

    r.play(0, 0);
    r.play(0, 0);
    let observer_handle = PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.modifiers.add(
        observer_handle,
        ModifierEntry::disable_effect(EffectTiming::OnPlay, Expiry::Permanent, 1),
    );

    let attacker_handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    let result = r.attack_player(attacker_handle, 1, /* vortex */ true);
    assert_ne!(result, AttackResult::Invalid);
    assert!(
        r.memory() > before,
        "DisableEffect for an unrelated timing must NOT block WhenAttacking"
    );
}
