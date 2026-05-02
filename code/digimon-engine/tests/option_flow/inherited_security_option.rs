use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;

fn option_source(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.card_kind = CardKind::Option;
    cd.level = None;
    cd.dp = None;
    cd.colors = vec![CardColor::Red];
    cd
}

fn digimon_card(card_id: &str) -> digimon_engine::CardData {
    let mut cd = make_test_card(card_id, card_id);
    cd.colors = vec![CardColor::Red];
    cd
}

#[test]
fn inherited_security_places_source_option_as_delay_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(option_source("P-035"))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_source(host, "P-035");

    r.run_inherited_security_effect(host, "P-035", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });

    assert_eq!(
        r.game.player(0).battle_area[host.index as usize]
            .card_sources
            .len(),
        1
    );
    let placed = r
        .game
        .player(0)
        .battle_area
        .last()
        .expect("placed option permanent");
    assert!(matches!(
        placed.option_state,
        OptionState::Delayed { owner: 0, .. }
    ));
    assert_eq!(placed.top_card().card_id(&r.game.card_data), "P-035");
}

#[test]
fn inherited_security_does_not_place_non_option_source() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_card("HOST"))
        .add_card(digimon_card("SOURCE-DIGIMON"))
        .memory(0)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_source(host, "SOURCE-DIGIMON");

    r.run_inherited_security_effect(host, "SOURCE-DIGIMON", |ctx| {
        ctx.place_self_as_delay_option_permanent();
    });

    assert_eq!(r.game.player(0).battle_area.len(), 1);
    let host_stack = &r.game.player(0).battle_area[host.index as usize].card_sources;
    assert_eq!(host_stack.len(), 2);
    assert_eq!(host_stack[0].card_id(&r.game.card_data), "SOURCE-DIGIMON");
    assert!(!r.game.player(0).battle_area.iter().any(|permanent| {
        matches!(permanent.option_state, OptionState::Delayed { .. })
            && permanent.top_card().card_id(&r.game.card_data) == "SOURCE-DIGIMON"
    }));
}
