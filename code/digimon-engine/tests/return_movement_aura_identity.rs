//! Regression for compacted battle-area identity during return movement.
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn fixture() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-075").unwrap()
        .dsl_card("BT25-001").unwrap()
        .dsl_card("BT25-008").unwrap()
        .dsl_card("BT25-014").unwrap()
        .memory(4).start();
    r.place_on_field(1, "BT25-075", None);
    r.place_stack(1, &["BT25-001", "BT25-008", "BT25-014"]);
    r.place_on_field(1, "BT25-014", None);
    r.game.tick_declarative_effects();
    r
}

fn check_return(to_deck: bool, with_survivor_buff: bool) {
    let mut r = fixture();
    let departing = PermanentHandle { player: 1, index: 1 };
    let old_survivor = PermanentHandle { player: 1, index: 2 };
    let survivor_card = r.game.players[1].battle_area[2].top_card().handle();
    let departing_card = r.game.players[1].battle_area[1].top_card().handle();
    if with_survivor_buff {
        r.game.modifiers.add(old_survivor,
            ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::Permanent, 1));
    }
    let expected_dp = r.game.effective_dp(old_survivor);
    let before = r.game.clone();
    for mut game in [r.game, before] {
        let moved = if to_deck {
            game.return_to_deck_from_effect(departing, 0)
        } else {
            game.return_to_hand_from_effect(departing, 0)
        };
        assert!(moved);
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[1].battle_area.len(), 2);
        assert_eq!(game.players[1].battle_area[1].top_card().handle(), survivor_card);
        assert_eq!(game.players[1].trash.len(), 2);
        let destination = if to_deck { &game.players[1].deck } else { &game.players[1].hand };
        assert!(destination.iter().any(|c| c.handle() == departing_card));
        // Reproduce the next replacement window's declarative tick after the
        // source-trash callbacks and departing permanent cleanup have run.
        game.tick_declarative_effects();
        let survivor = PermanentHandle { player: 1, index: 1 };
        assert!(game.has_keyword(survivor, Keyword::Rush));
        assert!(game.modifiers.has(survivor, ModifierType::ChangeLinkMax));
        assert_eq!(game.effective_dp(survivor), expected_dp,
            "nonmaterialized modifiers must follow the surviving physical card");
    }
}

#[test]
fn return_to_hand_preserves_compacted_survivor_aura_after_source_trash() {
    check_return(false, false);
}
#[test]
fn return_to_deck_preserves_compacted_survivor_aura_after_source_trash() {
    check_return(true, false);
}
#[test]
fn return_to_hand_rekeys_nonmaterialized_survivor_modifiers() {
    check_return(false, true);
}
#[test]
fn return_to_deck_rekeys_nonmaterialized_survivor_modifiers() {
    check_return(true, true);
}

