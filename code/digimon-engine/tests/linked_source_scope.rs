use digimon_engine::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect::Effect,
    enums::Keyword,
    game::Game,
};
#[test]
fn stacked_card_scope_excludes_link_only_even_when_adopted() {
    let mut link = Effect::on_play(CardHandle(0)).build();
    link.linked = true;
    for (index, size, adopted) in [(0, 1, false), (0, 1, true), (0, 2, false), (0, 2, true)] {
        assert!(!Game::source_effect_is_active(index, size, adopted, &link));
    }
}
#[test]
fn native_inherited_and_succession_scopes_are_preserved() {
    let face = Effect::on_play(CardHandle(0)).build();
    assert!(Game::source_effect_is_active(0, 1, false, &face));
    assert!(!Game::source_effect_is_active(0, 2, false, &face));
    assert!(Game::source_effect_is_active(0, 2, true, &face));
    let mut inherited = Effect::on_play(CardHandle(0)).build();
    inherited.inherited = true;
    assert!(!Game::source_effect_is_active(0, 1, false, &inherited));
    assert!(Game::source_effect_is_active(0, 2, false, &inherited));
}
fn fixture() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT26-010")
        .unwrap()
        .add_card(make_test_card("HOST", "Host"))
        .deck(0, &["HOST"; 12])
        .deck(1, &["HOST"; 12])
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
#[test]
fn real_roleplaymon_top_and_digivolution_source_do_not_grant_link_keywords() {
    let mut r = fixture();
    let own = r.place_on_field(0, "BT26-010", None);
    r.game.tick_declarative_effects();
    for kw in [Keyword::Progress, Keyword::Piercing] {
        assert!(!r.game.has_keyword(own, kw));
    }
    assert_eq!(
        r.game.source_dp_contribution(own, 0),
        0,
        "link DP cannot leak into top-card contribution"
    );
    let host = r.place_on_field(0, "HOST", None);
    r.push_source(host, "BT26-010");
    r.game.tick_declarative_effects();
    for kw in [Keyword::Progress, Keyword::Piercing] {
        assert!(!r.game.has_keyword(host, kw));
    }
}
#[test]
fn real_roleplaymon_linked_host_keeps_both_keywords_dp_and_clone() {
    let mut r = fixture();
    let host = r.place_on_field(0, "HOST", None);
    let before = r.effective_dp(host).unwrap();
    r.push_linked_owned(host, "BT26-010", 0);
    r.game.tick_declarative_effects();
    for kw in [Keyword::Progress, Keyword::Piercing] {
        assert!(r.game.has_keyword(host, kw));
        assert!(r.game.clone().has_keyword(host, kw));
    }
    assert_eq!(r.effective_dp(host), Some(before + 3000));
}
