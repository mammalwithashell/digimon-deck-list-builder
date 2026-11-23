pub(super) use super::common::{action, candidates, start};
use digimon_engine::action::space::{encode_attack, encode_digivolve, SECURITY_TARGET};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

pub(super) const TAMER: &str = "TEXT-KURATA-TAMER";
pub(super) const BOUNDARY: &str = "TEXT-KURATA-11000";
pub(super) const HIGH: &str = "TEXT-KURATA-12000";
pub(super) const KURATA: &str = "TEXT-KURATA-NAME";
pub(super) const EVOLUTION: &str = "TEXT-KURATA-EVOLUTION";
const FILLER: &str = "TEXT-KURATA-FILLER";

pub(super) fn builder(card: &str) -> DebugRunnerBuilder {
    let mut tamer = make_test_card(TAMER, "Test Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Purple];
    tamer.traits = vec!["Kurata's Army".into()];
    let mut kurata = tamer.clone();
    kurata.card_id = KURATA.into();
    kurata.card_name = "Akihiro Kurata".into();
    let mut boundary = make_test_card(BOUNDARY, "Boundary");
    boundary.dp = Some(11000);
    let mut high = make_test_card(HIGH, "High");
    high.dp = Some(12000);
    let mut evolution = make_test_card(EVOLUTION, "Eligible evolution");
    evolution.level = Some(4);
    evolution.evo_costs = vec![EvoCost {
        card_color: 6,
        level: 3,
        memory_cost: 1,
    }];
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap()
        .add_card(tamer)
        .add_card(kurata)
        .add_card(boundary)
        .add_card(high)
        .add_card(evolution)
        .add_card(make_test_card(FILLER, "Filler"))
        .deck(0, &[FILLER; 30])
        .deck(1, &[FILLER; 30])
        .security(0, &[FILLER; 5])
        .security(1, &[FILLER; 5])
        .memory(10)
}

pub(super) fn tamer_evolution(card: &str, cost: i16, keywords: &[Keyword]) {
    let mut r = start(builder(card).hand(0, &[card]));
    let base = r.place_on_field(0, TAMER, Some(0));
    r.place_on_field(1, BOUNDARY, Some(0));
    let before = r.memory();
    action(&mut r, 0, encode_digivolve(0, base.index as u16));
    assert_eq!(before - r.memory(), cost);
    let stack = &r.game.players[0].battle_area[base.index as usize];
    assert_eq!(stack.top_card().card_id(&r.game.card_data), card);
    assert_eq!(stack.card_sources[0].card_id(&r.game.card_data), TAMER);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.pending_selection().is_none());
    for &keyword in keywords {
        assert!(
            r.game.has_keyword(base, keyword),
            "{card} lacks {keyword:?}"
        );
    }
}

pub(super) fn security_plays_self(card: &str) {
    let mut r = start(builder(card).security(1, &[card]));
    let attacker = r.place_on_field(0, HIGH, Some(0));
    let before = r.memory();
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert_eq!(r.security_count(1), 0);
    assert_eq!(before, r.memory());
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        card
    );
    assert!(r.pending_selection().is_none());
}
