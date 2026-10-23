//! Minimal offline observer input. Never supplied to a policy or search.
use digimon_engine::game::Game;
use serde_json::{json, Value};

pub fn snapshot(game: &Game) -> Value {
    let mut result = json!({"pendingSelection": game.pending_selection.is_some()});
    for pid in 0..2 {
        let player = game.player(pid);
        let top = |p: &digimon_engine::permanent::Permanent| json!({"topCardId": game.card_data[p.top_card().data_index].card_id});
        result[format!("player{}", pid + 1)] = json!({
            "handIds": player.hand.iter().map(|c| game.card_data[c.data_index].card_id.clone()).collect::<Vec<_>>(),
            "battleArea": player.battle_area.iter().map(top).collect::<Vec<_>>(),
            "breedingArea": player.breeding_area.as_ref().map(top),
        });
    }
    result
}
