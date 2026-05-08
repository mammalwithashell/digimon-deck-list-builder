//! BT13-106 Odin's Breath
//!
//! Track A witness for the printed security-trash timing:
//! "When an effect trashes this card from the security stack, activate this
//! card's [Main] effect." The full Main body is card-local authoring; this
//! fixture proves the engine/DSL timing fires only for effect-driven security
//! trash, not for normal attack security checks.

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn odins_breath() -> CardData {
    let mut card = make_test_card("BT13-106", "Odin's Breath");
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 7;
    card.effect_text = "When an effect trashes this card from the security stack, activate this card's [Main] effect.".to_string();
    card
}

struct OdinDiscardWitness;

impl CardEffect for OdinDiscardWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_discard_security(card)
            .name("Odin's Breath discarded-from-security witness")
            .process(|ctx| ctx.gain_memory(2))
            .build()]
    }
}

#[test]
fn bt13_106_effect_trash_from_security_activates_discard_timing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TRASHER", "Security Trasher"))
        .add_card(odins_breath())
        .security(1, &["BT13-106"])
        .memory(0)
        .start();
    runner.register_effect("BT13-106", Arc::new(OdinDiscardWitness));
    let trasher = runner.place_on_field(0, "TRASHER", Some(0));
    let source_card = runner.game.players[0].battle_area[trasher.index as usize]
        .top_card()
        .handle();

    runner.game.set_effect_source_player_for_test(Some(0));
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(trasher), 0);
        assert!(ctx.trash_top_security(1));
    }
    runner.game.set_effect_source_player_for_test(None);

    assert_eq!(
        runner.memory(),
        -2,
        "Odin's Breath should activate when an opponent effect trashes it from security"
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}

#[test]
fn bt13_106_attack_security_check_does_not_activate_discard_timing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("ATK", "Attacker"))
        .add_card(odins_breath())
        .security(1, &["BT13-106"])
        .memory(0)
        .start();
    runner.register_effect("BT13-106", Arc::new(OdinDiscardWitness));
    let attacker = runner.place_on_field(0, "ATK", Some(0));

    let _ = runner.attack_player(attacker, 1, true);

    assert_eq!(
        runner.memory(),
        0,
        "normal security checks should not activate OnDiscardSecurity"
    );
    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.game.players[1].trash.len(), 1);
}
