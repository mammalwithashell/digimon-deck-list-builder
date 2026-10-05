//! BT12-024 Lanamon — Digimon, Lv.4, Blue, DP 4000, Cost 5.
//! Traits: Fairy. Attribute: Variable. Form: Hybrid.
//!
//! # Printed text (official Bandai DB — data/card_bundles/BT12-024.md)
//!
//! ```text
//! Digivolve: Blue Lv.3 / cost 2
//! [Digivolve] 0 from [Calmaramon]
//!
//! You may digivolve this card from your hand onto one of your blue Tamers as
//! if the Tamer is a level 3 blue Digimon. [When Digivolving] By placing a
//! blue level 3 Digimon card from your hand as the bottom digivolution card of
//! 1 of your blue Digimon, this Digimon gains <Jamming> for the turn. (This
//! Digimon can't be deleted in battles against Security Digimon.)
//! ```
//!
//! Official Q&A: the Tamer is treated as a Digimon that digivolves — "when a
//! Digimon digivolves" effects trigger, "can't digivolve" effects block it.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_024.cs — two
//! `AddSelfDigivolutionRequirementStaticEffect`s ([Calmaramon] cost 0; blue
//! Tamer cost 2 while in hand) + optional [When Digivolving]: SelectHand (own
//! blue Lv.3 Digimon card) → SelectPermanent (own non-token blue Digimon) →
//! AddDigivolutionCardsBottom → GainJamming(this, UntilEachTurnEnd) only when
//! the card was placed.
//!
//! # Patterns
//! - Hybrid Tamer digivolve (`source_treated_as: level_3_blue_digimon`,
//!   BT7-071 idiom)
//! - named alt digivolve (`name_is: Calmaramon`, cost 0)
//! - [When Digivolving] optional "By placing X from hand as bottom source of
//!   1 of your Digimon" cost (`select_hand { optional, cost }` →
//!   `select_own_permanent` → `place_as_bottom_source`)
//! - timed keyword grant (`grant_keyword` Jamming, `end_of_turn`)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor,
    CompiledCost, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT12-024";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.colors = vec![color];
    c
}

fn digimon(id: &str, name: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c.colors = vec![color];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-024 YAML parses, compiles and is in the embedded pack")
        .add_card(tamer("TAMER-BL", CardColor::Blue))
        .add_card(tamer("TAMER-RD", CardColor::Red))
        .add_card(digimon("ROOKIE-BL", "Blue Rookie", CardColor::Blue, 3))
        .add_card(digimon("ROOKIE-BL2", "Blue Rookie Two", CardColor::Blue, 3))
        .add_card(digimon("ROOKIE-RD", "Red Rookie", CardColor::Red, 3))
        .add_card(digimon("CHAMP-BL", "Blue Champion", CardColor::Blue, 4))
        .add_card(digimon("CHAMP-RD", "Red Champion", CardColor::Red, 4))
        .add_card(digimon("CALMARAMON", "Calmaramon", CardColor::Yellow, 4))
        .add_card(digimon("FILL", "Filler", CardColor::Green, 3))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(5)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn hand_index(runner: &DebugRunner, card_id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in hand"))
}

fn hand_contains(runner: &DebugRunner, card_id: &str) -> bool {
    runner.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn hand_action(runner: &DebugRunner, card_id: &str) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let slot = hand_index(runner, card_id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn own_field_action(runner: &DebugRunner, h: PermanentHandle) -> Option<u16> {
    let view = runner.pending_selection_view()?;
    let action = 100 + h.index as u16;
    view.valid_action_ids.contains(&action).then_some(action)
}

fn source_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn digivolve_action(runner: &DebugRunner, base: PermanentHandle) -> u16 {
    encode_digivolve(hand_index(runner, CARD_ID) as u16, base.index as u16)
}

fn can_digivolve_onto(runner: &DebugRunner, base: PermanentHandle) -> bool {
    build_action_mask(&runner.game, 0)[digivolve_action(runner, base) as usize] == 1.0
}

/// Digivolve Lanamon onto `base` through the real action pipeline.
fn digivolve_onto(runner: &mut DebugRunner, base: PermanentHandle) {
    assert!(can_digivolve_onto(runner, base), "digivolve must be legal");
    let action = digivolve_action(runner, base);
    runner.game.decode_action(action, 0);
    assert_eq!(top_id(runner, base), CARD_ID);
}

fn main_phase(runner: &mut DebugRunner) {
    runner.game.current_phase = GamePhase::Main;
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt12_024_metadata_matches_printed_card() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Lanamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(card.color, vec![CompiledColor::Blue]);
    assert!(card.traits.iter().any(|t| t == "Fairy"));
}

#[test]
fn bt12_024_alt_paths_circle_calmaramon_and_blue_tamer_route() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 3);
    assert!(card
        .alt_paths
        .iter()
        .all(|p| p.kind == CompiledAltPathKind::Digivolve && !p.ignore_requirements));

    let circle = &card.alt_paths[0];
    assert_eq!(circle.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(circle.from.as_ref().and_then(|f| f.level_eq), Some(3));
    assert!(circle.source_treated_as.is_none());

    let calm = &card.alt_paths[1];
    assert_eq!(calm.cost, Some(CompiledCost::Literal(0)));

    let hybrid = &card.alt_paths[2];
    assert_eq!(hybrid.direction, CompiledAltPathDirection::From);
    assert_eq!(hybrid.cost, Some(CompiledCost::Literal(2)));
    assert_eq!(
        hybrid.source_treated_as.as_deref(),
        Some("level_3_blue_digimon")
    );
    assert_eq!(
        hybrid.from.as_ref().and_then(|f| f.kind),
        Some(CompiledCardKind::Tamer)
    );
}

#[test]
fn bt12_024_single_when_digivolving_clause() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(card.effects.len(), 1);
    assert_eq!(triggered.len(), 1);
    assert!(triggered[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!triggered[0].once_per_turn);
}

// ─── Section 2 — Digivolution routes ─────────────────────────────────────────

#[test]
fn bt12_024_digivolves_onto_blue_tamer_for_2() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let tamer = runner.place_on_field(0, "TAMER-BL", Some(0));
    main_phase(&mut runner);
    let before = runner.memory();

    digivolve_onto(&mut runner, tamer);
    assert_eq!(before - runner.memory(), 2, "blue Lv.3 circle cost 2");
    let stack = source_ids(&runner, tamer);
    assert_eq!(stack.first().map(String::as_str), Some("TAMER-BL"));
    assert_eq!(
        runner.game.card_data
            [runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].data_index]
            .card_kind,
        CardKind::Tamer,
        "the Tamer keeps its printed kind as a source"
    );
}

#[test]
fn bt12_024_cannot_digivolve_onto_red_tamer() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let tamer = runner.place_on_field(0, "TAMER-RD", Some(0));
    main_phase(&mut runner);
    assert!(!can_digivolve_onto(&runner, tamer), "only BLUE Tamers");
}

#[test]
fn bt12_024_cannot_digivolve_onto_opponents_blue_tamer() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let opp = runner.place_on_field(1, "TAMER-BL", Some(0));
    main_phase(&mut runner);
    assert!(!runner.game.digivolve_from_hand(
        0,
        hand_index(&runner, CARD_ID),
        opp.index as usize,
        digimon_engine::enums::PlaySource::ByHand
    ));
    assert!(hand_contains(&runner, CARD_ID));
}

#[test]
fn bt12_024_digivolves_from_blue_lv3_for_2() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let rookie = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    let before = runner.memory();
    digivolve_onto(&mut runner, rookie);
    assert_eq!(before - runner.memory(), 2);
}

#[test]
fn bt12_024_cannot_digivolve_from_red_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let rookie = runner.place_on_field(0, "ROOKIE-RD", Some(0));
    main_phase(&mut runner);
    assert!(!can_digivolve_onto(&runner, rookie));
}

#[test]
fn bt12_024_digivolves_from_calmaramon_for_0() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let calm = runner.place_on_field(0, "CALMARAMON", Some(0));
    main_phase(&mut runner);
    let before = runner.memory();
    digivolve_onto(&mut runner, calm);
    assert_eq!(
        before - runner.memory(),
        0,
        "[Digivolve] 0 from [Calmaramon]"
    );
}

#[test]
fn bt12_024_cannot_digivolve_from_non_calmaramon_lv4() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let champ = runner.place_on_field(0, "CHAMP-RD", Some(0));
    main_phase(&mut runner);
    assert!(!can_digivolve_onto(&runner, champ));
}

// ─── Section 3 — [When Digivolving] ──────────────────────────────────────────

#[test]
fn bt12_024_wd_places_under_other_blue_digimon_and_gains_jamming() {
    let mut runner = builder()
        .hand(
            0,
            &[CARD_ID, "ROOKIE-BL2", "ROOKIE-RD", "CHAMP-BL", "TAMER-BL"],
        )
        .start();
    let other = runner.place_on_field(0, "CHAMP-BL", Some(0));
    let red = runner.place_on_field(0, "CHAMP-RD", Some(0));
    let base = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, base);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        runner.pending_is_optional(),
        "the placement cost is optional"
    );
    assert!(hand_action(&runner, "ROOKIE-RD").is_none(), "red excluded");
    assert!(hand_action(&runner, "CHAMP-BL").is_none(), "Lv.4 excluded");
    assert!(
        hand_action(&runner, "TAMER-BL").is_none(),
        "non-Digimon excluded"
    );
    let pick = hand_action(&runner, "ROOKIE-BL2").expect("blue Lv.3 Digimon selectable");
    runner.execute_action(0, pick).unwrap();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    assert!(
        own_field_action(&runner, red).is_none(),
        "red Digimon excluded"
    );
    assert!(
        own_field_action(&runner, base).is_some(),
        "this Digimon is one of your blue Digimon"
    );
    let host = own_field_action(&runner, other).expect("other blue Digimon selectable");
    runner.execute_action(0, host).unwrap();
    runner.auto_resolve().unwrap();

    assert_eq!(
        source_ids(&runner, other).first().map(String::as_str),
        Some("ROOKIE-BL2"),
        "placed as the BOTTOM digivolution card of the chosen Digimon"
    );
    assert!(!hand_contains(&runner, "ROOKIE-BL2"));
    assert!(
        runner.game.has_keyword(base, Keyword::Jamming),
        "THIS Digimon gains <Jamming>"
    );
    assert!(
        !runner.game.has_keyword(other, Keyword::Jamming),
        "the host does not gain <Jamming>"
    );
}

#[test]
fn bt12_024_wd_can_place_under_itself() {
    let mut runner = builder().hand(0, &[CARD_ID, "ROOKIE-BL2"]).start();
    let base = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, base);

    let pick = hand_action(&runner, "ROOKIE-BL2").unwrap();
    runner.execute_action(0, pick).unwrap();
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let host = own_field_action(&runner, base).expect("self selectable");
    runner.execute_action(0, host).unwrap();
    runner.auto_resolve().unwrap();

    assert_eq!(source_ids(&runner, base)[0], "ROOKIE-BL2");
    assert!(runner.game.has_keyword(base, Keyword::Jamming));
}

#[test]
fn bt12_024_wd_decline_places_nothing_and_no_jamming() {
    let mut runner = builder().hand(0, &[CARD_ID, "ROOKIE-BL2"]).start();
    let base = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, base);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.execute_action(0, PASS).unwrap();
    runner.auto_resolve().unwrap();

    assert!(hand_contains(&runner, "ROOKIE-BL2"));
    assert!(!source_ids(&runner, base).contains(&"ROOKIE-BL2".to_string()));
    assert!(!runner.game.has_keyword(base, Keyword::Jamming));
}

#[test]
fn bt12_024_wd_no_eligible_card_in_hand_no_prompt_no_jamming() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "ROOKIE-RD", "CHAMP-BL"])
        .start();
    let base = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, base);

    assert_ne!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.auto_resolve().unwrap();
    assert!(!runner.game.has_keyword(base, Keyword::Jamming));
    assert!(hand_contains(&runner, "ROOKIE-RD"));
    assert!(hand_contains(&runner, "CHAMP-BL"));
}

#[test]
fn bt12_024_wd_jamming_expires_at_end_of_turn() {
    let mut runner = builder().hand(0, &[CARD_ID, "ROOKIE-BL2"]).start();
    let base = runner.place_on_field(0, "ROOKIE-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, base);
    let pick = hand_action(&runner, "ROOKIE-BL2").unwrap();
    runner.execute_action(0, pick).unwrap();
    let host = own_field_action(&runner, base).unwrap();
    runner.execute_action(0, host).unwrap();
    runner.auto_resolve().unwrap();
    assert!(runner.game.has_keyword(base, Keyword::Jamming));

    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.has_keyword(base, Keyword::Jamming),
        "<Jamming> lasts only for the turn"
    );
}

/// Integrated: the Hybrid Tamer route is a real digivolution, so Lanamon's own
/// [When Digivolving] fires (Q&A — the Tamer is treated as a Digimon that
/// digivolves), and the Tamer-based Lanamon is a blue Digimon host candidate.
#[test]
fn bt12_024_tamer_route_fires_when_digivolving() {
    let mut runner = builder().hand(0, &[CARD_ID, "ROOKIE-BL2"]).start();
    let tamer = runner.place_on_field(0, "TAMER-BL", Some(0));
    main_phase(&mut runner);
    digivolve_onto(&mut runner, tamer);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "[When Digivolving] triggers on the Tamer route"
    );
    let pick = hand_action(&runner, "ROOKIE-BL2").unwrap();
    runner.execute_action(0, pick).unwrap();
    let host = own_field_action(&runner, tamer).expect("Lanamon (on the Tamer) selectable");
    runner.execute_action(0, host).unwrap();
    runner.auto_resolve().unwrap();

    let stack = source_ids(&runner, tamer);
    assert_eq!(stack.first().map(String::as_str), Some("ROOKIE-BL2"));
    assert!(runner.game.has_keyword(tamer, Keyword::Jamming));
}
