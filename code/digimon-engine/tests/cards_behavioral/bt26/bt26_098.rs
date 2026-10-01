//! BT26-098 Queen of Thorns — Option, Green, DATA SQUAD.
//!
//! When this card would be used, by trashing the bottom face-down card from
//! under any of your Tamers, reduce the cost by 2.
//! [Main] By placing 1 [Sunflowmon] and 1 [Lilamon] from your trash as 1 of
//! your [Lalamon]'s bottom digivolution cards, that Digimon may digivolve into
//! [Rosemon] in the hand, ignoring digivolution requirements and without paying
//! the cost.
//! [Security] You may play 1 [Lalamon] or [Yoshino Fujieda] from your hand or
//! trash without paying the cost. Then, add this card to the hand.
//!
//! DCGO: BT26/Green/BT26_098.cs; sister BT25-096 Mirage Beast Knight.

use super::support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "BT26-098";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-098")
        .add_card(filler("FILLER"))
        .add_card(tamer(
            "YOSHINO",
            "Yoshino Fujieda",
            CardColor::Green,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "LALA",
            "Lalamon",
            CardColor::Green,
            3,
            3,
            &["Vegetation", "DATA SQUAD"],
        ))
        .add_card(digimon(
            "SUNF",
            "Sunflowmon",
            CardColor::Green,
            4,
            5,
            &["Vegetation", "DATA SQUAD"],
        ))
        .add_card(digimon(
            "LILA",
            "Lilamon",
            CardColor::Green,
            5,
            6,
            &["Fairy", "DATA SQUAD"],
        ))
        .add_card(digimon(
            "ROSE",
            "Rosemon",
            CardColor::Green,
            6,
            12,
            &["Fairy", "DATA SQUAD"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

fn use_queen(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_098_main_places_both_then_free_digivolves_into_rosemon() {
    let mut r = setup();
    r.place_on_field(0, "YOSHINO", Some(0));
    let lala = r.place_on_field(0, "LALA", Some(0));
    push_trash(&mut r, 0, "SUNF");
    push_trash(&mut r, 0, "LILA");
    push_hand(&mut r, 0, "ROSE");
    use_queen(&mut r);
    let mem_after_use = r.memory();
    pick_first(&mut r, 0); // Lalamon
    pick_first(&mut r, 0); // Sunflowmon
    pick_first(&mut r, 0); // Lilamon
    pick_hand(&mut r, 0, "ROSE");
    let _ = r.auto_resolve();
    let stack: Vec<String> = r.game.players[0].battle_area[lala.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(stack.last().map(String::as_str), Some("ROSE"));
    assert!(stack.contains(&"SUNF".to_string()) && stack.contains(&"LILA".to_string()));
    assert_eq!(r.memory(), mem_after_use, "digivolve was free");
}

#[test]
fn bt26_098_main_is_all_or_nothing() {
    let mut r = setup();
    r.place_on_field(0, "YOSHINO", Some(0));
    let lala = r.place_on_field(0, "LALA", Some(0));
    push_trash(&mut r, 0, "SUNF");
    push_trash(&mut r, 0, "LILA");
    push_hand(&mut r, 0, "ROSE");
    use_queen(&mut r);
    pick_first(&mut r, 0); // Lalamon
    pick_first(&mut r, 0); // Sunflowmon
    pass(&mut r, 0); // decline Lilamon
    let _ = r.auto_resolve();
    assert_eq!(
        sources(&r, lala),
        0,
        "nothing placed when only one was chosen (Q&A)"
    );
    assert!(trash_ids(&r, 0).contains(&"SUNF".to_string()));
}

#[test]
fn bt26_098_has_cost_reduction_main_and_security() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).unwrap();
    let dbg = format!("{:?}", card.effects);
    assert!(dbg.contains("CostReduction"), "use-cost reduction clause");
    assert!(card.effects.iter().any(|c| matches!(c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand))));
    assert!(card.effects.iter().any(|c| matches!(c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity))));
}

#[test]
fn bt26_098_face_down_trash_reduces_use_cost_by_two() {
    let mut r = setup();
    let y = tamer_with_face_down(&mut r, 0, "YOSHINO", 1);
    let mem = r.memory();
    use_queen(&mut r);
    // "When this card would be used, by trashing ... reduce the cost by 2."
    let v = r.pending_selection_view().expect("cost-reduction offer");
    assert!(v.is_optional);
    r.execute_branch(0).expect("accept the reduction");
    pick_first(&mut r, 0); // the Tamer
                           // [Main] has nothing to do (no Lalamon); let it resolve.
    while r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    assert_eq!(sources(&r, y), 0, "face-down card trashed");
    assert_eq!(r.memory(), mem - 3, "use cost 5 reduced by 2");
}

#[test]
fn bt26_098_no_reduction_offer_without_face_down_card() {
    let mut r = setup();
    r.place_on_field(0, "YOSHINO", Some(0));
    let mem = r.memory();
    use_queen(&mut r);
    while r.game.pending_selection.is_some() {
        pass(&mut r, 0);
    }
    assert_eq!(r.memory(), mem - 5, "full use cost");
}
