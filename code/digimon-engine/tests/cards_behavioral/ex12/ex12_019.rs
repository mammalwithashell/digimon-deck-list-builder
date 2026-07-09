use digimon_engine::enums::{CardColor, EffectSourceKind, Keyword, ModifierType};
use digimon_engine::selection::AttackTarget;
use digimon_engine::trigger_context::AttackTargetChangeReason;

use super::support::{fire_attack_target_changed, plain_digimon, DebugRunner};

const CARD_ID: &str = "EX12-019";

#[test]
fn ex12_019_has_printed_keywords() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-019 YAML loads")
        .start();
    let nezhamon = runner.place_on_field(0, CARD_ID, Some(0));

    for keyword in [
        Keyword::Rush,
        Keyword::Collision,
        Keyword::Piercing,
        Keyword::Blocker,
        Keyword::Engage,
    ] {
        assert!(runner.game.has_keyword(nezhamon, keyword), "{keyword:?}");
    }
}

#[test]
fn ex12_019_attack_target_change_grants_dp_and_opponent_digimon_immunity() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-019 YAML loads")
        .add_card(plain_digimon("OLD", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("NEW", CardColor::Red, 4, 5000))
        .start();
    let nezhamon = runner.place_on_field(0, CARD_ID, Some(0));
    let old_target = runner.place_on_field(1, "OLD", Some(0));
    let new_target = runner.place_on_field(1, "NEW", Some(0));

    fire_attack_target_changed(
        &mut runner,
        nezhamon,
        AttackTarget::Digimon(old_target),
        AttackTarget::Digimon(new_target),
        AttackTargetChangeReason::Collision,
    );
    runner.auto_resolve().expect("resolve target-change buff");

    assert_eq!(
        runner.game.modifiers.sum(nezhamon, ModifierType::ChangeDp),
        4000
    );
    assert!(runner
        .game
        .permanent_is_unaffected_by_effect(nezhamon, 1, EffectSourceKind::Digimon));
}
