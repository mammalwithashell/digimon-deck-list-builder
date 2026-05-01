use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, Expiry};
use digimon_engine::modifiers::ModifierEntry;

fn test_card(card_id: &str, kind: CardKind) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = kind;
    card.colors = vec![CardColor::Red];
    match kind {
        CardKind::Digimon => {
            card.level = Some(3);
            card.dp = Some(3000);
        }
        CardKind::Option | CardKind::Tamer => {
            card.level = None;
            card.dp = None;
        }
        _ => {}
    }
    card
}

#[test]
fn opponent_digimon_source_cannot_delete_digimon_with_digimon_effect_immunity() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("TARGET", CardKind::Digimon))
        .add_card(test_card("OPP-SOURCE", CardKind::Digimon))
        .start();
    let target = r.place_on_field(0, "TARGET", Some(0));
    let source = r.place_on_field(1, "OPP-SOURCE", Some(0));
    let source_card = r.game.player(1).battle_area[source.index as usize]
        .top_card()
        .handle();

    r.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let mut ctx = EffectContext::new_with_source_kind(
        &mut r.game,
        source_card,
        Some(source),
        EffectSourceKind::Digimon,
        1,
    );
    ctx.delete_permanent(target);

    assert_eq!(ctx.game.player(0).battle_area.len(), 1);
}

#[test]
fn opponent_option_source_can_delete_digimon_with_digimon_effect_immunity() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("TARGET", CardKind::Digimon))
        .add_card(test_card("OPT-SOURCE", CardKind::Option))
        .hand(1, &["OPT-SOURCE"])
        .start();
    let target = r.place_on_field(0, "TARGET", Some(0));
    let source_card = r.game.player(1).hand[0].handle();

    r.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let mut ctx = EffectContext::new_with_source_kind(
        &mut r.game,
        source_card,
        None,
        EffectSourceKind::Option,
        1,
    );
    ctx.delete_permanent(target);

    assert_eq!(ctx.game.player(0).battle_area.len(), 0);
}

#[test]
fn own_digimon_source_can_affect_digimon_with_opponent_only_immunity() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("TARGET", CardKind::Digimon))
        .add_card(test_card("OWN-SOURCE", CardKind::Digimon))
        .start();
    let target = r.place_on_field(0, "TARGET", Some(0));
    let source = r.place_on_field(0, "OWN-SOURCE", Some(0));
    let source_card = r.game.player(0).battle_area[source.index as usize]
        .top_card()
        .handle();

    r.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let mut ctx = EffectContext::new_with_source_kind(
        &mut r.game,
        source_card,
        Some(source),
        EffectSourceKind::Digimon,
        0,
    );
    ctx.delete_permanent(target);

    assert_eq!(ctx.game.player(0).battle_area.len(), 1);
    assert_eq!(
        ctx.game.player(0).battle_area[0]
            .top_card()
            .card_id(&ctx.game.card_data),
        "OWN-SOURCE"
    );
}

#[test]
fn central_immunity_query_matches_source_kind_and_controller() {
    let mut r = DebugRunner::builder()
        .add_card(test_card("TARGET", CardKind::Digimon))
        .start();
    let target = r.place_on_field(0, "TARGET", Some(0));

    r.game.modifiers.add(
        target,
        ModifierEntry::cannot_be_affected_by_opponents_source_kind(
            EffectSourceKind::Digimon,
            Expiry::EndOfTurn,
            0,
        ),
    );

    assert!(r
        .game
        .permanent_is_unaffected_by_effect(target, 1, EffectSourceKind::Digimon));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(target, 1, EffectSourceKind::Option));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(target, 0, EffectSourceKind::Digimon));
}
