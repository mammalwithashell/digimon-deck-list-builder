use std::collections::HashMap;

use digimon_engine::card_data::EvoCost;
use digimon_engine::card_store::shared_card_store;
use digimon_engine::debug_runner::make_test_card;

fn changed_route_is_visible(id: &str, changed: EvoCost) {
    let original = EvoCost { card_color: 1, level: 3, memory_cost: 2 };
    let mut card = make_test_card(id, "Evolution cache regression");
    card.evo_costs = vec![original.clone()];
    let mut db = HashMap::from([(id.to_owned(), card)]);
    let first = shared_card_store(&db);
    db.get_mut(id).unwrap().evo_costs[0] = changed.clone();
    let second = shared_card_store(&db);
    assert_eq!(second.data[second.index[id]].evo_costs, vec![changed]);
    assert_eq!(first.data[first.index[id]].evo_costs, vec![original], "existing games retain their original rules data");
}

#[test]
fn equal_length_cost_update_changes_the_next_games_printed_cost() {
    changed_route_is_visible("CACHE-EVO-AMOUNT", EvoCost { card_color: 1, level: 3, memory_cost: 7 });
}

#[test]
fn equal_length_level_update_changes_the_next_games_requirement() {
    changed_route_is_visible("CACHE-EVO-LEVEL", EvoCost { card_color: 1, level: 4, memory_cost: 2 });
}

#[test]
fn equal_length_color_update_changes_the_next_games_requirement() {
    changed_route_is_visible("CACHE-EVO-COLOR", EvoCost { card_color: 2, level: 3, memory_cost: 2 });
}
