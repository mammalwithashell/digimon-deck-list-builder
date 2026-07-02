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
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
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
        also_treated_as: Vec::new(),
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

/// Inherited `WhenAttacking -> +1 memory` source effect.
struct InheritedAttackingMemoryGain;
impl CardEffect for InheritedAttackingMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .inherited()
            .name("+1 inherited when attacking")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

// NOTE (2026-07-01): `[When Attacking]` is carrier-scoped ("when THIS Digimon
// attacks" — see `fire_on_attack`), so these two tests put the memory-gain
// effect AND the `DisableEffect` modifier on the ATTACKER itself. The original
// shape (effect on a bystander "observer" permanent) stopped firing when the
// timing was correctly narrowed from battle-area-wide to the attacker's stack,
// which made the positive test vacuous and the negative control fail.
#[test]
fn disable_effect_modifier_suppresses_only_the_named_timing() {
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("ATK", Arc::new(AttackingMemoryGain));

    r.play(0, 0);
    let attacker_handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        attacker_handle,
        ModifierEntry::disable_effect(EffectTiming::WhenAttacking, Expiry::Permanent, 1),
    );

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
        "DisableEffect{{WhenAttacking}} must suppress the attacker's WhenAttacking gain"
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
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("ATK", Arc::new(AttackingMemoryGain));

    r.play(0, 0);
    let attacker_handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        attacker_handle,
        ModifierEntry::disable_effect(EffectTiming::OnPlay, Expiry::Permanent, 1),
    );

    let before = r.memory();
    let result = r.attack_player(attacker_handle, 1, /* vortex */ true);
    assert_ne!(result, AttackResult::Invalid);
    assert!(
        r.memory() > before,
        "DisableEffect for an unrelated timing must NOT block WhenAttacking"
    );
}

#[test]
fn cannot_activate_when_attacking_suppresses_inherited_timing_effects() {
    let mut top = plain_digimon("TOP", "Top", 5);
    top.level = Some(5);
    top.dp = Some(8000);
    let inherited = plain_digimon("INH", "Inherited", 2);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(inherited)
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("INH", Arc::new(InheritedAttackingMemoryGain));

    let attacker = r.place_stack(0, &["INH", "TOP"]);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::CannotActivateWhenAttackingEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let before = r.memory();
    let result = r.attack_player(attacker, 1, /* vortex */ true);
    assert_ne!(
        result,
        AttackResult::Invalid,
        "test setup must perform a legal attack before checking inherited suppression"
    );
    assert_eq!(
        r.memory(),
        before,
        "CannotActivateWhenAttackingEffects must suppress inherited effects on the gated stack"
    );
}

#[test]
fn cannot_activate_when_attacking_suppresses_granted_timing_effects() {
    let mut attacker_data = plain_digimon("ATK-GRANTED", "AttackerGranted", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);
    let grantor = plain_digimon("GRANTOR", "Grantor", 3);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(grantor)
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();

    let grantor_handle = r.place_on_field(0, "GRANTOR", Some(0));
    let attacker = r.place_on_field(0, "ATK-GRANTED", Some(0));
    let grantor_top = r.game.players[0].battle_area[grantor_handle.index as usize]
        .top_card()
        .handle();

    {
        let mut ctx = EffectContext::new(&mut r.game, grantor_top, Some(grantor_handle), 0);
        ctx.grant_triggered_effect(
            attacker,
            EffectTiming::WhenAttacking,
            Expiry::Permanent,
            |inner| inner.gain_memory(1),
        );
    }

    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::CannotActivateWhenAttackingEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let before = r.memory();
    let result = r.attack_player(attacker, 1, /* vortex */ true);
    assert_ne!(
        result,
        AttackResult::Invalid,
        "test setup must perform a legal attack before checking granted suppression"
    );
    assert_eq!(
        r.memory(),
        before,
        "CannotActivateWhenAttackingEffects must suppress granted effects on the gated Digimon"
    );
}
