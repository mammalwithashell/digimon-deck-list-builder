//! BT6-103 Blasted Disaster — Option, Green, Cost 6. Traits: (none).
//!
//! # Printed text (official Bandai DB — data/card_bundles/BT6-103.md)
//!
//! ```text
//! [Main] Suspend all of your opponent's Digimon. Then, gain 1 memory for each
//! of your opponent's suspended Digimon.
//! Security Effect: [Security] Suspend 1 of your opponent's Digimon.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT6/Green/BT6_103.cs
//!   - OptionSkill: every opponent battle-area Digimon not immune to this
//!     effect → ONE SuspendPermanentsClass(list).Tap() (a prohibited one is
//!     skipped); then AddMemory(count of the opponent's battle-area Digimon
//!     that are suspended — including ones that already were).
//!   - SecuritySkill: if an opponent Digimon exists, a mandatory
//!     (canNoSelect: false) pick of 1 opponent Digimon, Mode.Tap.
//!
//! # Patterns
//! - board-wide suspend (`for_each` + `suspend`)
//! - formula memory gain (`gain_memory_fn` over `card_count_in_zone`)
//! - [Security] mandatory single opponent pick + suspend

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const CARD_ID: &str = "BT6-103";

fn digimon(id: &str, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Red];
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-103 in the embedded DSL pack")
        .add_card(digimon("FILL", CardColor::Red, 1000))
        .add_card(digimon("GREEN", CardColor::Green, 4000))
        .add_card(digimon("RED", CardColor::Red, 4000))
        .add_card(digimon("OPP-A", CardColor::Blue, 4000))
        .add_card(digimon("OPP-B", CardColor::Blue, 5000))
        .add_card(digimon("OPP-C", CardColor::Blue, 6000))
        .add_card(digimon("ATK", CardColor::Red, 9000))
        .add_card(tamer("OPP-T"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn set_suspended(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn use_from_hand(r: &mut DebugRunner) -> OptionPlayResult {
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == CARD_ID)
        .expect("Blasted Disaster in hand");
    let res = r.game.play_option_from_hand(0, idx);
    let _ = r.auto_resolve();
    res
}

// ─── Section 1: structural ──────────────────────────────────────────────────

#[test]
fn bt6_103_structure() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::Option);
    assert_eq!(c.cost, Some(6));
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2, "[Main] + [Security]");
    let main = trig
        .iter()
        .find(|t| t.when == vec![CompiledTiming::MainFromHand])
        .expect("[Main]");
    assert!(!main.optional, "mandatory");
    let sec = trig
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("[Security]");
    assert!(!sec.optional, "mandatory");
}

// ─── Section 3: [Main] ──────────────────────────────────────────────────────

#[test]
fn bt6_103_main_suspends_all_opponent_digimon_and_gains_memory_per_suspended() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let own = r.place_on_field(0, "GREEN", Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let t = r.place_on_field(1, "OPP-T", Some(0));
    assert_ne!(use_from_hand(&mut r), OptionPlayResult::Invalid);
    assert!(suspended(&r, a) && suspended(&r, b), "all opponent Digimon");
    assert!(!suspended(&r, t), "a Tamer is not a Digimon");
    assert!(!suspended(&r, own), "only the opponent's Digimon");
    assert_eq!(r.memory(), 10 - 6 + 2, "cost 6, then +1 per suspended Digimon");
}

#[test]
fn bt6_103_main_counts_digimon_that_were_already_suspended() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.place_on_field(0, "GREEN", Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    let c = r.place_on_field(1, "OPP-C", Some(0));
    set_suspended(&mut r, c);
    use_from_hand(&mut r);
    assert!(suspended(&r, a) && suspended(&r, b) && suspended(&r, c));
    assert_eq!(
        r.memory(),
        10 - 6 + 3,
        "'each of your opponent's suspended Digimon' — 3, one already suspended"
    );
}

#[test]
fn bt6_103_main_does_not_count_a_digimon_that_could_not_be_suspended() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.place_on_field(0, "GREEN", Some(0));
    let a = r.place_on_field(1, "OPP-A", Some(0));
    let b = r.place_on_field(1, "OPP-B", Some(0));
    r.game.modifiers.add(
        b,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 1),
    );
    use_from_hand(&mut r);
    assert!(suspended(&r, a));
    assert!(!suspended(&r, b), "a Digimon that can't be suspended stays unsuspended");
    assert_eq!(r.memory(), 10 - 6 + 1, "only the suspended one counts");
}

#[test]
fn bt6_103_main_with_no_opponent_digimon_gains_nothing() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    let own = r.place_on_field(0, "GREEN", Some(0));
    r.place_on_field(1, "OPP-T", Some(0));
    assert_ne!(use_from_hand(&mut r), OptionPlayResult::Invalid);
    assert!(!suspended(&r, own));
    assert_eq!(r.memory(), 10 - 6);
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == CARD_ID),
        "the used Option is trashed"
    );
}

#[test]
fn bt6_103_needs_green_to_use() {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.place_on_field(0, "RED", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "green colour requirement"
    );
    assert_eq!(r.memory(), 10);
}

// ─── Section 3: [Security] ──────────────────────────────────────────────────

#[test]
fn bt6_103_security_suspends_the_chosen_opponent_digimon() {
    // P0 attacks P1; Blasted Disaster is P1's security card. For P1, "your
    // opponent's Digimon" are P0's.
    let mut r = base().security(1, &[CARD_ID]).memory(3).start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let other = r.place_on_field(0, "RED", Some(0));
    let keep = r.place_on_field(0, "GREEN", Some(0));
    r.attack_player(atk, 1, false);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField), "P1 chooses");
    let v = r.pending_selection_view().unwrap();
    assert_eq!(v.selecting_player, 1);
    assert!(!r.pending_is_optional(), "mandatory");
    let action = 100 + other.index as u16;
    assert!(v.valid_action_ids.contains(&action));
    r.execute_action(1, action).unwrap();
    let _ = r.auto_resolve();
    assert!(suspended(&r, other));
    assert!(!suspended(&r, keep), "only 1");
    assert_eq!(r.security_count(1), 0);
}

#[test]
fn bt6_103_security_does_not_touch_the_owners_digimon() {
    let mut r = base().security(1, &[CARD_ID]).memory(3).start();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let p1_own = r.place_on_field(1, "OPP-A", Some(0));
    r.attack_player(atk, 1, false);
    let _ = r.auto_resolve();
    assert!(!suspended(&r, p1_own), "P1's own Digimon is never a target");
    assert!(suspended(&r, atk));
}
