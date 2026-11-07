//! EX4-074 prints [ShineGreymon], without "in its name": an exact-name route.
use digimon_engine::{card_data::CardData, debug_runner::DebugRunner,
    enums::{CardSourceRef, CostDelta, PlaySource}};

#[test]
fn ruin_mode_cannot_use_its_own_name_as_the_exact_shinegreymon_requirement() {
    for player in 0..2 {
        let data = CardData::load_from_file(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json"
        ))).unwrap();
        let mut r = DebugRunner::builder().dsl_card("EX4-074").unwrap()
            .with_card_data(data).hand(player, &["EX4-074"])
            .deck(player, &["ST1-03"; 4]).memory(5).start();
        let host = r.place_on_field(player, "EX4-074", Some(0));
        let original = r.top_card(host);
        let result = r.game.player(player).hand[0].handle();
        let memory = r.memory();
        let success = r.game.effect_initiated_digivolve_from_source(
            player, CardSourceRef::Hand(player, 0), host,
            CostDelta::Fixed(0), false, PlaySource::ByEffect,
        );
        assert!(!success, "Ruin Mode is not exactly [ShineGreymon] and is not a level6 base");
        assert_eq!(r.top_card(host), original);
        assert_eq!(r.game.player(player).hand[0].handle(), result);
        assert_eq!(r.memory(), memory);
        assert_eq!(r.game.player(player).deck.len(), 4);
    }
}
