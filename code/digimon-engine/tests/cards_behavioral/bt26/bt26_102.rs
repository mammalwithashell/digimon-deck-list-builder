//! BT26-102 Seven Code PAD — Option (White), use cost 7.
//!
//! <Use Req. ([Seven Code] trait)>
//! [Main] By placing 6 [Seven Code] trait Digimon cards from your battle area,
//! link cards or trash as 1 of your [Seven Code] trait Digimon's bottom
//! digivolution cards, that Digimon may digivolve into [Dantemon] in the hand,
//! ignoring digivolution requirements and without paying the cost.
//! [Security] You may play 1 play cost 5 or lower [Appmon] trait card from your
//! hand or trash without paying the cost. Then, add this card to the hand.
//!
//! DCGO: BT26/*/BT26_102.cs. G-DSL-PLACE-MATERIALS-MULTI-ZONE.

use super::support::*;
use digimon_engine::action::space::{
    encode_attack, encode_source_select, PASS, TRASH_EFFECT_START,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT26-102";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-102")
        .dsl_card("BT26-086")
        .expect("BT26-086 Dantemon")
        .add_card(filler("FILLER"))
        // Not [Appmon]: Dantemon's own [When Digivolving] would link it away.
        .add_card(digimon(
            "HOST",
            "Host",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SCB",
            "Seven B",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SCL",
            "Seven Link",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SC1",
            "Seven 1",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SC2",
            "Seven 2",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SC3",
            "Seven 3",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "SC4",
            "Seven 4",
            CardColor::White,
            5,
            5,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "APP4",
            "App Four",
            CardColor::White,
            3,
            4,
            &["Appmon"],
        ))
        .add_card(digimon(
            "APP6",
            "App Six",
            CardColor::White,
            4,
            6,
            &["Appmon"],
        ))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
}

fn setup() -> DebugRunner {
    let mut r = builder().start();
    r.set_first_player(0);
    r.game.players[0].hand.clear();
    r
}

fn link_card(r: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let next = r.game.next_card_index();
    let card = CardSource::new(data_idx, host.player, next);
    r.game.player_mut(host.player).battle_area[host.index as usize]
        .linked_cards
        .push(card);
}

fn use_pad(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    r.game.enter_main_phase();
    let _ = r.game.play_option_from_hand(0, idx);
}

fn trash_action(r: &DebugRunner, id: &str) -> u16 {
    TRASH_EFFECT_START + trash_ids(r, 0).iter().position(|t| t == id).unwrap() as u16
}

/// Host + one other [Seven Code] Digimon + one link card on the host + four
/// trash cards = exactly 6 materials besides the host.
fn six_materials(r: &mut DebugRunner) -> (PermanentHandle, PermanentHandle) {
    let host = r.place_on_field(0, "HOST", Some(0));
    let other = r.place_on_field(0, "SCB", Some(0));
    link_card(r, host, "SCL");
    for id in ["SC1", "SC2", "SC3", "SC4"] {
        push_trash(r, 0, id);
    }
    (host, other)
}

#[test]
fn bt26_102_places_six_from_mixed_zones_then_digivolves_into_dantemon() {
    let mut r = setup();
    let (host, other) = six_materials(&mut r);
    push_hand(&mut r, 0, "BT26-086");
    use_pad(&mut r);
    // Host pick (optional — "by placing …" is a may-cost).
    let v = r.pending_selection_view().expect("host pick");
    assert!(v.is_optional);
    r.execute_action(0, encode_attack(0, host.index as u16))
        .unwrap();

    // ONE material prompt over battle area + link cards + trash.
    let sel = r.game.pending_selection.as_ref().expect("material pick");
    assert!(matches!(
        sel.kind,
        SelectionKind::CountCappedMultiSelect { min: 6, max: 6, .. }
    ));
    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
    assert!(sel.is_optional, "0 (decline the cost) or exactly 6");
    let other_top = encode_source_select(other.index as u16, 0).unwrap();
    // The host's link card follows its 1-card stack.
    let link = encode_source_select(host.index as u16, 1).unwrap();
    let host_top = encode_source_select(host.index as u16, 0).unwrap();
    assert!(
        sel.valid_action_ids.contains(&other_top),
        "another [Seven Code] Digimon"
    );
    assert!(
        sel.valid_action_ids.contains(&link),
        "the host's own link card"
    );
    assert!(
        !sel.valid_action_ids.contains(&host_top),
        "not the host itself"
    );

    r.execute_action(0, other_top).unwrap();
    assert!(
        !r.pending_selection_view().unwrap().is_optional,
        "can't stop at 1"
    );
    r.execute_action(0, link).unwrap();
    for id in ["SC1", "SC2", "SC3", "SC4"] {
        let a = trash_action(&r, id);
        r.execute_action(0, a).unwrap();
    }
    // Optional Dantemon digivolve from hand.
    let v = r.pending_selection_view().expect("Dantemon pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "BT26-086");
    let _ = r.auto_resolve();

    let field = field_ids(&r, 0);
    assert!(
        !field.contains(&"SCB".to_string()),
        "the moved Digimon left the field"
    );
    assert!(
        !trash_ids(&r, 0).contains(&"SCB".to_string()),
        "moved, not deleted"
    );
    let host = r
        .game
        .permanent_with_top_card(
            r.game.players[0]
                .battle_area
                .iter()
                .find(|p| p.top_card().card_id(&r.game.card_data) == "BT26-086")
                .expect("Dantemon on the field")
                .top_card()
                .handle(),
        )
        .unwrap();
    let ids = source_ids(&r, host);
    for id in ["SCB", "SCL", "SC1", "SC2", "SC3", "SC4", "HOST"] {
        assert!(ids.contains(&id.to_string()), "{id} in the stack: {ids:?}");
    }
    assert!(trash_ids(&r, 0).iter().all(|t| !t.starts_with("SC")));
}

#[test]
fn bt26_102_declining_the_materials_places_nothing() {
    let mut r = setup();
    let (host, _) = six_materials(&mut r);
    use_pad(&mut r);
    r.execute_action(0, encode_attack(0, host.index as u16))
        .unwrap();
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        trash_ids(&r, 0)
            .iter()
            .filter(|t| t.starts_with("SC"))
            .count(),
        4
    );
    assert!(field_ids(&r, 0).contains(&"SCB".to_string()));
    assert_eq!(top_id(&r, host), "HOST");
}

#[test]
fn bt26_102_fewer_than_six_materials_offers_nothing() {
    let mut r = setup();
    let host = r.place_on_field(0, "HOST", Some(0));
    for id in ["SC1", "SC2", "SC3", "SC4"] {
        push_trash(&mut r, 0, id);
    }
    use_pad(&mut r);
    r.execute_action(0, encode_attack(0, host.index as u16))
        .unwrap();
    assert!(
        !matches!(
            r.game.pending_selection.as_ref().map(|s| &s.kind),
            Some(SelectionKind::CountCappedMultiSelect { .. })
        ),
        "an unpayable 6-card cost is not offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(
        trash_ids(&r, 0).len(),
        4 + 1,
        "4 materials + the used Option"
    );
}

#[test]
fn bt26_102_security_plays_cheap_appmon_from_trash_then_goes_to_hand() {
    let mut r = builder().security(1, &[CARD_ID]).start();
    r.set_first_player(0);
    push_trash(&mut r, 1, "APP4");
    push_trash(&mut r, 1, "APP6");
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    let v = r.pending_selection_view().expect("security play pick");
    assert!(v.is_optional);
    assert_eq!(non_pass(&r).len(), 1, "only the cost-4 [Appmon]");
    pick_first(&mut r, 1);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"APP4".to_string()));
    assert!(
        hand_ids(&r, 1).contains(&CARD_ID.to_string()),
        "then to the hand"
    );
}
