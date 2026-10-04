//! BT26-008 Kotemon — Lv.3 Red (Reptile / Shambala / TB / TS).
//!
//! [When Moving] [On Play] 1 of your [Shambala] or [TS] trait Digimon gains
//! <Piercing> and +3000 DP for the turn.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//!
//! DCGO: BT26/Red/BT26_008.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, Keyword};

const CARD_ID: &str = "BT26-008";
const OWN: u16 = digimon_engine::action::space::ATTACK_START; // 100 + own field idx

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-008")
        .add_card(filler("FILLER"))
        .add_card(digimon(
            "SHAM",
            "Shambalamon",
            CardColor::Red,
            4,
            5,
            &["Shambala"],
        ))
        .add_card(digimon("TSMON", "TSmon", CardColor::Red, 4, 5, &["TS"]))
        .add_card(digimon("PLAIN", "Plainmon", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("HOST", "Host", CardColor::Red, 4, 5, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(0, &["FILLER"; 3])
        .security(1, &["FILLER"; 3])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_008_alt_path_shambala_or_ts() {
    let r = setup();
    let alt = format!("{:?}", r.compiled_card(CARD_ID).unwrap().alt_paths);
    assert!(
        alt.contains("Shambala") && alt.contains("TS") && alt.contains("level_eq: Some(2)"),
        "{alt}"
    );
}

#[test]
fn bt26_008_on_play_grants_piercing_and_3000_to_a_shambala_or_ts_digimon() {
    let mut r = setup();
    let plain = r.place_on_field(0, "PLAIN", Some(0)); // idx 0
    let sham = r.place_on_field(0, "SHAM", Some(0)); // idx 1
    let ts = r.place_on_field(0, "TSMON", Some(0)); // idx 2
    let k = r.place_on_field(0, CARD_ID, None); // idx 3
    fire(&mut r, EffectTiming::OnPlay, k);
    let v = r.pending_selection_view().expect("target pick");
    assert!(!v.is_optional, "mandatory");
    assert!(!v.valid_action_ids.contains(&(OWN + plain.index as u16)));
    assert!(v.valid_action_ids.contains(&(OWN + ts.index as u16)));
    assert!(
        v.valid_action_ids.contains(&(OWN + k.index as u16)),
        "Kotemon itself is [TS]"
    );
    r.execute_action(0, OWN + sham.index as u16).unwrap();
    assert!(r.game.has_keyword(sham, Keyword::Piercing));
    assert_eq!(r.effective_dp(sham), Some(4000 + 3000));
    assert!(!r.game.has_keyword(ts, Keyword::Piercing));
    // "for the turn": gone after the turn ends.
    r.end_turn();
    assert!(!r.game.has_keyword(sham, Keyword::Piercing));
    assert_eq!(r.effective_dp(sham), Some(4000));
}

#[test]
fn bt26_008_when_moving_from_breeding_fires() {
    let mut r = setup();
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, CARD_ID);
    assert!(r.move_from_breeding(0));
    let v = r
        .pending_selection_view()
        .expect("[When Moving] target pick");
    assert!(!v.is_optional);
    pick_first(&mut r, 0);
    let k = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
        .unwrap();
    let k = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: k as u8,
    };
    assert!(r.game.has_keyword(k, Keyword::Piercing));
    assert_eq!(r.effective_dp(k), Some(1000 + 3000));
}

#[test]
fn bt26_008_other_digimon_moving_does_not_fire() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.set_phase(GamePhase::Breeding);
    r.place_in_breeding(0, "SHAM");
    assert!(r.move_from_breeding(0));
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn bt26_008_inherited_plus_2000_on_your_turn_only() {
    let mut r = setup();
    let c = r.place_stack(0, &[CARD_ID, "HOST"]);
    assert_eq!(r.effective_dp(c), Some(4000 + 2000));
    r.end_turn();
    assert_eq!(r.effective_dp(c), Some(4000));
}
