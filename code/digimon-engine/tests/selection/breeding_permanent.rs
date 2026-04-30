use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;

#[test]
fn breeding_permanent_selection_targets_breeding_without_fake_battle_handle() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil"))
        .start();
    let p0 = 0;
    let source = r.place_on_field(p0, "SRC", Some(0));
    let source_card = r.top_card(source);
    let breeding = r.place_in_breeding(p0, "KING-DRASIL");

    let picked = Arc::new(Mutex::new(None));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_own_breeding_permanent(
            "choose breeding",
            |_, _| true,
            move |_, target| {
                *picked_slot.lock().unwrap() = Some(target);
            },
        );
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectBreedingPermanent);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::BreedingPermanent
    );
    assert!(r
        .game
        .player(p0)
        .battle_area
        .iter()
        .all(|perm| perm.top_card().handle() != breeding.card));

    r.game
        .resolve_selection(p0, encode_breeding_select(p0).expect("p0 breeding action"))
        .expect("pick breeding");

    let selected = picked.lock().unwrap().expect("selection callback fired");
    assert_eq!(selected.player, p0);
    assert_eq!(selected.card, breeding.card);
}
