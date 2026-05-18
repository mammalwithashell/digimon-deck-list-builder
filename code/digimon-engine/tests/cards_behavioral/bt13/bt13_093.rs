//! BT13-093 Omekamon - Digimon, Lv.4, White.
//!
//! Implemented slices (RK-G001 substrate landed Phase 2 Track J PR 1):
//! - [On Play] <Draw 1>.
//! - [On Deletion] Place 1 Digimon card with the [Royal Knight] trait from
//!   your hand under a [King Drasil_7D6] in the breeding area as its bottom
//!   digivolution card.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;

fn filler(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::White];
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(filler("FILLER"))
        .memory(10)
        .start()
}

#[test]
fn bt13_093_has_printed_metadata() {
    let runner = runner();
    let card = runner
        .compiled_card("BT13-093")
        .expect("BT13-093 compiled card present");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::White]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.traits.iter().any(|name| name == "X Antibody"));
    assert_eq!(card.attribute.as_deref(), Some("Data"));
}

#[test]
fn bt13_093_authors_on_play_draw_and_on_deletion_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT13-093")
        .expect("BT13-093 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "On Play draw + On Deletion King Drasil source placement"
    );
    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Play clause exists");
    assert!(
        on_play
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. })),
        "On Play must be Draw 1"
    );
    let on_deletion = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnDeletion) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("On Deletion clause exists");
    assert!(
        on_deletion
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::SelectOwnBreedingPermanent { .. })),
        "On Deletion opens a filtered breeding-permanent selection"
    );
}

#[test]
fn bt13_093_on_play_draws_one_after_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(filler("FILLER"))
        .hand(0, &["BT13-093"])
        .deck(0, &["FILLER"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("Omekamon plays from hand");

    assert_eq!(runner.memory(), 6, "pay 4 memory, then draw 1");
    assert_eq!(
        runner.hand_size(0),
        1,
        "draw replaces the played card in hand"
    );
    assert!(runner.pending_selection().is_none());
}

/// The On Deletion clause opens its breeding-permanent prompt only when a
/// [King Drasil_7D6] is in the breeding area. Other breeding hosts are
/// ignored by the RK-G001 filter; the rest of the effect short-circuits.
#[test]
fn bt13_093_on_deletion_short_circuits_when_no_king_drasil_in_breeding() {
    use digimon_engine::permanent::PermanentHandle;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card({
            let mut c = make_test_card("RK-IN-HAND", "Royal Knight Filler");
            c.colors = vec![CardColor::White];
            c.traits = vec!["Royal Knight".to_string()];
            c
        })
        .add_card(make_test_card("OTHER-EGG", "Other Egg"))
        .hand(0, &["BT13-093", "RK-IN-HAND"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "OTHER-EGG");
    runner.play(0, 0).expect("Omekamon plays");

    let omekamon_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&runner.game.card_data) == "Omekamon")
        .expect("Omekamon on field");
    let handle = PermanentHandle {
        player: 0,
        index: omekamon_idx as u8,
    };

    let breeding_stack_before = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .unwrap()
        .stack_size();
    let hand_before = runner.hand_size(0);

    runner.game.delete_permanent_with_effects(handle);

    assert!(
        runner.game.pending_selection.is_none(),
        "No King Drasil in breeding → no selection opens"
    );
    let breeding_stack_after = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .unwrap()
        .stack_size();
    assert_eq!(
        breeding_stack_after, breeding_stack_before,
        "Non-King-Drasil breeding host is untouched"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "No Royal Knight is pulled from hand when the breeding filter fails"
    );
}

/// When [King Drasil_7D6] is in breeding AND a [Royal Knight] Digimon sits
/// in hand, the On Deletion clause prompts the breeding permanent, then
/// prompts the hand selection, then tucks the chosen card under King Drasil.
#[test]
fn bt13_093_on_deletion_places_royal_knight_hand_card_under_king_drasil() {
    use digimon_engine::action::space::encode_breeding_select;
    use digimon_engine::action::space::PLAY_HAND_START;
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .add_card({
            let mut c = make_test_card("MAGNAMON", "Magnamon");
            c.colors = vec![CardColor::White];
            c.traits = vec!["Royal Knight".to_string()];
            c
        })
        .hand(0, &["BT13-093", "MAGNAMON"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "KING-DRASIL");
    runner.play(0, 0).expect("Omekamon plays");

    let magnamon_card_handle = {
        let idx = runner
            .game
            .player(0)
            .hand
            .iter()
            .position(|c| c.card_name(&runner.game.card_data) == "Magnamon")
            .expect("Magnamon still in hand after pay-cost");
        runner.game.player(0).hand[idx].handle()
    };

    let omekamon_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&runner.game.card_data) == "Omekamon")
        .expect("Omekamon on field");
    runner.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: omekamon_idx as u8,
    });

    // First prompt: breeding-permanent selection.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("breeding-permanent selection opens");
    assert_eq!(pending.kind, SelectionKind::BreedingPermanent);

    runner
        .game
        .resolve_selection(0, encode_breeding_select(0).unwrap())
        .expect("pick King Drasil_7D6");

    // Second prompt: hand selection for the [Royal Knight] card. Find the
    // hand slot of Magnamon and resolve.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("hand selection opens for the Royal Knight card");
    assert_eq!(pending.kind, SelectionKind::Hand);
    let hand_slot = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_name(&runner.game.card_data) == "Magnamon")
        .unwrap();
    runner
        .game
        .resolve_selection(0, PLAY_HAND_START + hand_slot as u16)
        .expect("pick Magnamon");

    // Magnamon now sits at the bottom of King Drasil's breeding stack.
    let breeding = runner.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(breeding.stack_size(), 2);
    assert_eq!(
        breeding.card_sources[0].handle(),
        magnamon_card_handle,
        "Magnamon tucked under King Drasil"
    );
    assert!(
        !runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.handle() == magnamon_card_handle),
        "Magnamon left the hand"
    );
}

/// If a [King Drasil_7D6] is in breeding but the hand has no Royal Knight
/// Digimon, the inner hand selection short-circuits and nothing moves.
#[test]
fn bt13_093_on_deletion_short_circuits_when_no_royal_knight_in_hand() {
    use digimon_engine::action::space::encode_breeding_select;
    use digimon_engine::permanent::PermanentHandle;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-093")
        .expect("BT13-093 YAML loads")
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .add_card(filler("NON-RK"))
        .hand(0, &["BT13-093", "NON-RK"])
        .memory(10)
        .start();
    runner.place_in_breeding(0, "KING-DRASIL");
    runner.play(0, 0).expect("Omekamon plays");

    let omekamon_idx = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&runner.game.card_data) == "Omekamon")
        .expect("Omekamon on field");
    runner.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: omekamon_idx as u8,
    });

    // Breeding selection opens because King Drasil matches the filter.
    runner
        .game
        .resolve_selection(0, encode_breeding_select(0).unwrap())
        .expect("pick King Drasil_7D6");

    // No Royal Knight in hand → inner select_hand short-circuits.
    assert!(runner.game.pending_selection.is_none());
    let breeding = runner.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(
        breeding.stack_size(),
        1,
        "King Drasil stack unchanged when no Royal Knight is in hand"
    );
}
