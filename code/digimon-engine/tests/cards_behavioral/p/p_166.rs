//! P-166 Galemon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Bird Dragon / LIBERATOR. Form: Champion. Attribute: Data.
//! Digivolve: green Lv.3, cost 2.
//!
//! # Card text (official Bandai DB — data/card_bundles/P-166.md; card image agrees)
//!
//! ```text
//! [On Play] [When Digivolving] You may suspend 1 Digimon. Then, if it's your
//! turn, this Digimon may digivolve into a Digimon card with [Avian] or [Bird]
//! in any of its traits in the hand. For every other suspended Digimon, reduce
//! this effect's digivolution cost by 1.
//! Inherited: [Your Turn] This Digimon gets +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Green/P_166.cs
//!   - On Play / When Digivolving (not optional): SelectPermanent over ANY
//!     battle-area Digimon (canNoSelect, Mode.Tap); then, if IsOwnerTurn,
//!     suspendedCount = suspended Digimon on both fields other than this
//!     permanent → DigivolveIntoHandOrTrashCard(this, [Avian]/[Bird]
//!     ContainsTraits Digimon, payCost, reduceCost: suspendedCount, isHand).
//!   - Inherited: ChangeSelfDPStaticEffect +2000, IsOwnerTurn.
//!
//! # Rulings
//! - Q4276: with BT3-103 [Hidden Potential Discovered!] suspending one more
//!   Digimon at the "would digivolve" timing, this card's reduction counts it
//!   (2, for 7 in total). See `p_166_q4276_hidden_potential_suspension_counts`
//!   (ignored — G-ENGINE-EFFECT-DIGIVOLVE-COST-LATE-COUNT).
//!
//! # Patterns this test covers
//! - F-row `[on_play, when_digivolving]`, optional any-Digimon suspend, then
//!   `your_turn`-gated optional digivolve into a hand card
//!   (`effect_initiated_digivolve`, BT12-030 shape) with a formula reduction
//!   `suspended_count { of: any, exclude_source }` (EX8-074 shape)
//! - D-row inherited `[Your Turn]` +2000 DP

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledScope, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-166";

fn digimon(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Green Lv.5 with `traits`, digivolving from green Lv.4 for 3.
fn lv5(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, CardColor::Green, 5, traits);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 4,
        memory_cost: 3,
    }];
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-166 in pack")
        .dsl_card("BT3-103")
        .expect("BT3-103 in pack")
        .add_card(lv5("AVIAN5", &["Avian"]))
        .add_card(lv5("BIRDDRAGON5", &["Bird Dragon"]))
        .add_card(lv5("BEAST5", &["Beast"]))
        .add_card({
            let mut c = lv5("AVIAN5-9", &["Avian"]);
            c.evo_costs[0].memory_cost = 9;
            c
        })
        .add_card(digimon("G-LV3", CardColor::Green, 3, &["Beast"]))
        .add_card(digimon("ALLY", CardColor::Blue, 3, &["Beast"]))
        .add_card(digimon("OPP", CardColor::Red, 4, &["Dragon"]))
        .add_card(digimon("OPP-2", CardColor::Red, 4, &["Dragon"]))
        .add_card(make_test_card("F1", "F1"))
        .deck(0, &["F1"; 10])
        .deck(1, &["F1"; 10])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(10);
    r
}

fn hand_idx(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand")) as u16
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn find(r: &DebugRunner, id: &str) -> Option<PermanentHandle> {
    r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
}

/// Play Galemon from the hand (cost 4).
fn play_galemon(r: &mut DebugRunner) {
    r.add_to_hand(0, CARD_ID);
    let h = hand_idx(r, CARD_ID) as usize;
    let _ = r.play(0, h);
}

/// The suspend pick: `Some(action)` picks, `None` declines.
fn answer_suspend(r: &mut DebugRunner, action: Option<u16>) {
    let v = r.pending_selection_view().expect("'You may suspend 1 Digimon'");
    assert!(v.is_optional, "{v:?}");
    r.execute_action(0, action.unwrap_or(PASS)).unwrap();
}

/// `select_any_permanent` (AnyField) encodes `encode_attack(player, index)`.
fn any(h: PermanentHandle) -> u16 {
    encode_attack(h.player as u16, h.index as u16)
}

/// An OwnField pick (BT3-103's suspend cost).
fn own_field(h: PermanentHandle) -> u16 {
    ATTACK_START + h.index as u16
}

/// Pick `id` in the "may digivolve" hand prompt; returns memory paid.
fn digivolve_into(r: &mut DebugRunner, id: &str) -> i16 {
    let v = r.pending_selection_view().expect("'may digivolve' hand pick");
    assert!(v.is_optional, "'may digivolve'");
    let a = PLAY_HAND_START + hand_idx(r, id);
    assert!(v.valid_action_ids.contains(&a), "{id}: {:?}", v.valid_action_ids);
    let before = r.memory();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    before - r.memory()
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn p_166_metadata_and_green_lv3_cost2_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "Galemon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    assert_eq!(
        card.traits,
        vec!["Bird Dragon".to_string(), "LIBERATOR".to_string()]
    );
    assert_eq!(card.attribute, ["Data"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(2)));
}

#[test]
fn p_166_on_play_and_when_digivolving_share_one_mandatory_clause() {
    let card = dsl_card_data::compiled(CARD_ID);
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .expect("face-up clause");
    assert_eq!(
        main.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert!(!main.optional, "the 'may' parts are the picks");
}

// ─── SECTION 2 — [On Play] ─────────────────────────────────────────────────

/// Suspend an opponent's Digimon; digivolve into an [Avian] Lv.5 for
/// 3 - 1 (one other suspended Digimon) = 2.
#[test]
fn p_166_on_play_suspend_then_digivolve_reduced_by_other_suspended() {
    let mut r = runner();
    let o = r.place_on_field(1, "OPP", Some(0));
    r.add_to_hand(0, "AVIAN5");
    play_galemon(&mut r);
    answer_suspend(&mut r, Some(any(o)));
    assert!(suspended(&r, o));
    assert_eq!(digivolve_into(&mut r, "AVIAN5"), 2, "3 - 1");
    let g = find(&r, "AVIAN5").expect("digivolved");
    assert_eq!(r.game.players[0].battle_area[g.index as usize].card_sources.len(), 2);
}

/// Both players' suspended Digimon count ("every other suspended Digimon");
/// the reduction floors the cost at 0.
#[test]
fn p_166_counts_both_sides_and_floors_at_zero() {
    let mut r = runner();
    let o = r.place_on_field(1, "OPP", Some(0));
    let o2 = r.place_on_field(1, "OPP-2", Some(0));
    suspend(&mut r, o2);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    suspend(&mut r, ally);
    r.add_to_hand(0, "AVIAN5");
    play_galemon(&mut r);
    answer_suspend(&mut r, Some(any(o)));
    assert_eq!(digivolve_into(&mut r, "AVIAN5"), 0, "3 - 3");
}

/// "other": suspending Galemon itself does not reduce the cost.
#[test]
fn p_166_galemon_itself_is_not_counted() {
    let mut r = runner();
    r.add_to_hand(0, "AVIAN5");
    play_galemon(&mut r);
    let g = find(&r, CARD_ID).expect("Galemon on field");
    answer_suspend(&mut r, Some(any(g)));
    assert!(suspended(&r, g));
    assert_eq!(digivolve_into(&mut r, "AVIAN5"), 3, "no OTHER suspended Digimon");
}

/// "You may suspend": declining still offers the digivolution; existing
/// suspended Digimon still reduce it.
#[test]
fn p_166_declining_the_suspend_still_offers_the_digivolve() {
    let mut r = runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    suspend(&mut r, ally);
    r.add_to_hand(0, "AVIAN5");
    play_galemon(&mut r);
    answer_suspend(&mut r, None);
    assert_eq!(digivolve_into(&mut r, "AVIAN5"), 2, "3 - 1");
}

/// Only Digimon cards with [Avian] or [Bird] in a trait ("Bird Dragon"
/// contains "Bird") are offered.
#[test]
fn p_166_offers_only_avian_or_bird_trait_digimon() {
    let mut r = runner();
    r.add_to_hand(0, "AVIAN5");
    r.add_to_hand(0, "BIRDDRAGON5");
    r.add_to_hand(0, "BEAST5");
    play_galemon(&mut r);
    answer_suspend(&mut r, None);
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "AVIAN5"))));
    assert!(v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "BIRDDRAGON5"))));
    assert!(!v.valid_action_ids.contains(&(PLAY_HAND_START + hand_idx(&r, "BEAST5"))));
}

/// "may digivolve": declining leaves Galemon as is.
#[test]
fn p_166_declining_the_digivolve_keeps_galemon() {
    let mut r = runner();
    r.add_to_hand(0, "AVIAN5");
    play_galemon(&mut r);
    answer_suspend(&mut r, None);
    let before = r.memory();
    r.execute_action(0, PASS).unwrap();
    let _ = r.auto_resolve();
    assert!(find(&r, CARD_ID).is_some());
    assert_eq!(r.memory(), before);
}

/// "if it's your turn": on the opponent's turn only the suspension happens.
#[test]
fn p_166_no_digivolve_on_the_opponents_turn() {
    let mut r = runner();
    let o = r.place_on_field(1, "OPP", Some(0));
    r.add_to_hand(0, "AVIAN5");
    r.game.turn_player_idx = 1;
    // Played by an effect during the opponent's turn.
    let h = r.add_to_hand(0, CARD_ID);
    let _ = r
        .game
        .play_from_hand_with_cost(0, h, CostDelta::Free, PlaySource::ByEffect);
    answer_suspend(&mut r, Some(any(o)));
    let _ = r.auto_resolve();
    assert!(suspended(&r, o));
    assert!(find(&r, "AVIAN5").is_none(), "[if it's your turn]");
    assert!(find(&r, CARD_ID).is_some());
}

// ─── SECTION 3 — [When Digivolving] ────────────────────────────────────────

#[test]
fn p_166_when_digivolving_same_effect() {
    let mut r = runner();
    let base = r.place_on_field(0, "G-LV3", Some(0));
    let o = r.place_on_field(1, "OPP", Some(0));
    r.add_to_hand(0, "AVIAN5");
    let h = r.add_to_hand(0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(h, base.index as u16), 0);
    assert_eq!(r.memory(), 8, "digivolve cost 2 paid");
    answer_suspend(&mut r, Some(any(o)));
    assert_eq!(digivolve_into(&mut r, "AVIAN5"), 2, "3 - 1");
    assert!(find(&r, "AVIAN5").is_some());
}

// ─── SECTION 4 — Inherited [Your Turn] +2000 DP ────────────────────────────

#[test]
fn p_166_inherited_plus_2000_on_your_turn() {
    let mut r = runner();
    let carrier = r.place_stack(0, &[CARD_ID, "AVIAN5"]);
    assert_eq!(r.effective_dp(carrier), Some(9000));
}

#[test]
fn p_166_inherited_not_on_the_opponents_turn() {
    let mut r = runner();
    let carrier = r.place_stack(0, &[CARD_ID, "AVIAN5"]);
    r.game.turn_player_idx = 1;
    assert_eq!(r.effective_dp(carrier), Some(7000));
}

#[test]
fn p_166_inherited_inactive_while_galemon_is_the_top_card() {
    let mut r = runner();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(r.effective_dp(me), Some(4000));
}

// ─── SECTION 5 — Q4276 (known gap) ─────────────────────────────────────────

/// Q4276: Galemon's [On Play] suspends 1 other Digimon; at the digivolution
/// BT3-103's armed reducer suspends another — Galemon's reduction counts BOTH
/// (2), and with BT3-103's 5 the total is 7. A cost-9 target makes the split
/// visible: 9 - 7 = 2 paid (the engine pays 3: it counts the suspended
/// Digimon when the effect starts the digivolution, before the would-digivolve
/// reducer window — DCGO does the same). Every step up to the final cost
/// assertion passes: the reducer is armed, offered and accepted, ALLY is
/// suspended, and the digivolution resolves; only the amount paid differs.
#[test]
#[ignore = "G-ENGINE-EFFECT-DIGIVOLVE-COST-LATE-COUNT (Q4276): the reduction is counted before the would-digivolve reducer window"]
fn p_166_q4276_hidden_potential_suspension_counts() {
    let mut r = runner();
    let o = r.place_on_field(1, "OPP", Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    // An Option needs a Digimon/Tamer of its colour in play to be used.
    r.place_on_field(0, "G-LV3", Some(0));
    // Use BT3-103 from the hand as a main-phase action (the
    // `tests/effect_player_digivolve_reducer.rs` staging) and prove its
    // player-scoped reducer is armed before Galemon is played.
    let h = r.add_to_hand(0, "BT3-103") as u16;
    assert_eq!(build_action_mask(&r.game, 0)[(PLAY_HAND_START + h) as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START + h, 0);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1, "BT3-103 armed");
    r.game.set_memory(10);
    r.add_to_hand(0, "AVIAN5-9");
    play_galemon(&mut r);
    answer_suspend(&mut r, Some(any(o)));
    let before = r.memory();
    let a = PLAY_HAND_START + hand_idx(&r, "AVIAN5-9");
    r.execute_action(0, a).unwrap();
    // BT3-103's reducer: an optional EffectChoice offered before the cost is
    // paid; accept, then suspend ALLY (OwnField).
    let v = r
        .pending_selection_view()
        .expect("BT3-103's armed reducer is offered on Galemon's effect digivolution");
    assert_eq!(v.kind, SelectionKind::EffectChoice);
    assert!(v.is_optional);
    assert_eq!(r.memory(), before, "nothing paid before the reducer choice");
    let yes = *v.valid_action_ids.iter().find(|&&x| x != PASS).unwrap();
    r.execute_action(0, yes).unwrap();
    let v = r.pending_selection_view().expect("choose 1 own Digimon to suspend");
    assert_eq!(v.kind, SelectionKind::OwnField);
    r.execute_action(0, own_field(ally)).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, ally));
    assert!(find(&r, "AVIAN5-9").is_some());
    assert!(r.game.player_digivolve_cost_reducers.is_empty());
    assert_eq!(before - r.memory(), 2, "9 - 5 - 2 (Q4276)");
}
