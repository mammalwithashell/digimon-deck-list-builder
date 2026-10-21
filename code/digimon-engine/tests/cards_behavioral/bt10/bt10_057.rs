//! BT10-057 BloomLordmon — Digimon, Lv.6, Green, DP 12000, Cost 12.
//! Traits: Fairy. Form: Mega. Attribute: Vaccine.
//! Digivolve: green Lv.5, cost 4.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT10-057.md; card image agrees)
//!
//! ```text
//! [When Digivolving] You may suspend 1 of your Digimon. Then, gain 1 memory
//! for each of your suspended Digimon with [Vegetation], [Plant] or [Fairy] in
//! any of their traits. If this effect gains 2 or more memory, this Digimon
//! unsuspends and gains ＜Piercing＞ for the turn.
//! [Your Turn] For every 2 of your suspended Digimon, this Digimon gains
//! ＜Security A. +1＞ and +2000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT10/Green/BT10_057.cs
//!   - [When Digivolving] (not optional): SelectPermanent over the owner's
//!     Digimon (canNoSelect, Mode.Tap) → plusMemory = owner's suspended
//!     Digimon with HasPlantTraits || HasFairyTraits (this one included) →
//!     AddMemory(plusMemory) → if plusMemory >= 2: unsuspend this Digimon +
//!     Pierce UntilEachTurnEnd.
//!   - [Your Turn] (two static effects): count = owner's suspended Digimon / 2
//!     (no trait filter, this one included) → +2000×count DP, +count S.Attack.
//!
//! # Rulings
//! - Q1980: it may suspend ITSELF; with another suspended [Vegetation]/
//!   [Plant]/[Fairy] Digimon it gains 2 memory, unsuspends and gains
//!   <Piercing>.
//! - Q1981: 4 suspended Digimon → +4000 DP and <Security A. +1> twice.
//! - Q1982: this Digimon counts when it is suspended.
//!
//! # Patterns this test covers
//! - F-row `when_digivolving`, optional own suspend, formula `gain_memory`,
//!   conditional unsuspend + `<Piercing>` until end of turn
//! - D-row `[Your Turn]` count-scaled DP + base-inclusive `security_attack_fn`

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledCost, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT10-057";

fn digimon(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn runner() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-057 in pack")
        .add_card(digimon("BASE", CardColor::Green, 5, &["Dragon"]))
        .add_card(digimon("VEG", CardColor::Green, 4, &["Vegetation"]))
        .add_card(digimon("PLANT", CardColor::Green, 4, &["Plant"]))
        .add_card(digimon("FAIRY-X", CardColor::Green, 4, &["Fairy Spirit"]))
        .add_card(digimon("BEAST", CardColor::Red, 4, &["Beast"]))
        .add_card(digimon("OPP-VEG", CardColor::Red, 4, &["Vegetation"]))
        .add_card(make_test_card("F1", "F1"))
        .deck(0, &["F1"; 10])
        .deck(1, &["F1"; 10])
        .security(1, &["F1"; 5])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.enter_main_phase();
    r.game.set_memory(5);
    r
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Digivolve `base` into BloomLordmon (cost 4) from the hand. Returns the
/// memory after paying, before the [When Digivolving] resolves.
fn digivolve_into_bloomlordmon(r: &mut DebugRunner, base: PermanentHandle) {
    let h = r.add_to_hand(0, CARD_ID) as u16;
    r.game.decode_action(encode_digivolve(h, base.index as u16), 0);
}

fn own_pick(h: PermanentHandle) -> u16 {
    ATTACK_START + h.index as u16
}

/// Answer the [When Digivolving] "You may suspend 1 of your Digimon" with
/// `pick` (None = decline the suspension), then let the rest resolve.
fn resolve_wd(r: &mut DebugRunner, pick: Option<PermanentHandle>) {
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.is_optional, "'You may suspend'");
    match pick {
        Some(h) => {
            assert!(v.valid_action_ids.contains(&own_pick(h)), "{v:?}");
            r.execute_action(0, own_pick(h)).unwrap();
        }
        None => r.execute_action(0, PASS).unwrap(),
    }
    let _ = r.auto_resolve();
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt10_057_metadata_and_green_lv5_cost4_circle() {
    let card = dsl_card_data::compiled(CARD_ID);
    assert_eq!(card.name, "BloomLordmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.traits, vec!["Fairy".to_string()]);
    assert_eq!(card.attribute, ["Vaccine"]);
    assert_eq!(card.alt_paths.len(), 1);
    assert_eq!(card.alt_paths[0].cost, Some(CompiledCost::Literal(4)));
}

#[test]
fn bt10_057_when_digivolving_clause_is_mandatory_with_an_optional_pick() {
    let card = dsl_card_data::compiled(CARD_ID);
    let wd: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::WhenDigivolving] => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(wd.len(), 1);
    assert!(!wd[0].optional, "the clause resolves; only the suspension is optional");
    assert_eq!(wd[0].scope, CompiledScope::FaceUp);
}

// ─── SECTION 2 — [When Digivolving] ────────────────────────────────────────

/// Suspending a [Vegetation] ally with a [Plant] ally already suspended:
/// 2 memory; BloomLordmon (suspended base) unsuspends and gains <Piercing>.
#[test]
fn bt10_057_two_plant_suspended_gains_2_unsuspends_and_pierces() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    suspend(&mut r, base);
    let veg = r.place_on_field(0, "VEG", Some(0));
    let plant = r.place_on_field(0, "PLANT", Some(0));
    suspend(&mut r, plant);
    digivolve_into_bloomlordmon(&mut r, base);
    assert_eq!(r.memory(), 1, "5 - 4 paid");
    resolve_wd(&mut r, Some(veg));
    assert!(suspended(&r, veg));
    // BloomLordmon itself is a suspended [Fairy] Digimon too: 3 counted.
    assert_eq!(r.memory(), 4, "VEG + PLANT + BloomLordmon = 3");
    assert!(!suspended(&r, base), "unsuspended");
    assert!(r.game.has_keyword(base, Keyword::Piercing));
}

/// Exactly 1 qualifying suspended Digimon: 1 memory, no unsuspend, no
/// <Piercing>.
#[test]
fn bt10_057_one_qualifying_gains_1_and_nothing_else() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    digivolve_into_bloomlordmon(&mut r, base);
    resolve_wd(&mut r, Some(veg));
    assert_eq!(r.memory(), 2, "1 + 1");
    assert!(!r.game.has_keyword(base, Keyword::Piercing));
}

/// Q1980: BloomLordmon may suspend itself; with another suspended
/// [Vegetation] Digimon it gains 2, then unsuspends and gains <Piercing>.
#[test]
fn bt10_057_q1980_suspend_itself_then_unsuspend() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    suspend(&mut r, veg);
    digivolve_into_bloomlordmon(&mut r, base);
    resolve_wd(&mut r, Some(base));
    assert_eq!(r.memory(), 3, "1 + 2");
    assert!(!suspended(&r, base));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
}

/// "You may" suspend — declining still runs "Then, gain ...".
#[test]
fn bt10_057_declining_the_suspend_still_gains() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let plant = r.place_on_field(0, "PLANT", Some(0));
    suspend(&mut r, veg);
    suspend(&mut r, plant);
    digivolve_into_bloomlordmon(&mut r, base);
    resolve_wd(&mut r, None);
    assert_eq!(r.memory(), 3, "1 + 2");
    assert!(r.game.has_keyword(base, Keyword::Piercing));
}

/// Only YOUR suspended Digimon with the traits count: a suspended [Beast]
/// and the opponent's suspended [Vegetation] Digimon don't.
#[test]
fn bt10_057_counts_only_own_plant_fairy_suspended_digimon() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let beast = r.place_on_field(0, "BEAST", Some(0));
    suspend(&mut r, beast);
    let opp = r.place_on_field(1, "OPP-VEG", Some(0));
    suspend(&mut r, opp);
    let fairy = r.place_on_field(0, "FAIRY-X", Some(0));
    digivolve_into_bloomlordmon(&mut r, base);
    resolve_wd(&mut r, Some(fairy));
    assert_eq!(r.memory(), 2, "only the 'Fairy Spirit' ally: 1");
    assert!(!r.game.has_keyword(base, Keyword::Piercing));
}

/// The suspension pick is over YOUR Digimon only, and unsuspended ones.
#[test]
fn bt10_057_suspend_pick_is_own_unsuspended_digimon() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    let plant = r.place_on_field(0, "PLANT", Some(0));
    suspend(&mut r, plant);
    r.place_on_field(1, "OPP-VEG", Some(0));
    digivolve_into_bloomlordmon(&mut r, base);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.valid_action_ids.contains(&own_pick(base)));
    assert!(v.valid_action_ids.contains(&own_pick(veg)));
    assert!(!v.valid_action_ids.contains(&own_pick(plant)), "already suspended");
}

/// <Piercing> lasts for the turn only.
#[test]
fn bt10_057_piercing_ends_with_the_turn() {
    let mut r = runner();
    let base = r.place_on_field(0, "BASE", Some(0));
    let veg = r.place_on_field(0, "VEG", Some(0));
    suspend(&mut r, veg);
    digivolve_into_bloomlordmon(&mut r, base);
    resolve_wd(&mut r, Some(base));
    assert!(r.game.has_keyword(base, Keyword::Piercing));
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(!r.game.has_keyword(base, Keyword::Piercing));
}

// ─── SECTION 3 — [Your Turn] per 2 suspended: +2000 DP and <S.A. +1> ──────

fn field_bloomlordmon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_on_field(0, CARD_ID, Some(0))
}

fn place_suspended(r: &mut DebugRunner, player: u8, id: &str, n: usize) {
    for _ in 0..n {
        let h = r.place_on_field(player, id, Some(0));
        suspend(r, h);
    }
}

#[test]
fn bt10_057_one_suspended_gives_nothing() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    place_suspended(&mut r, 0, "BEAST", 1);
    assert_eq!(r.effective_dp(me), Some(12000));
    assert_eq!(r.game.effective_security_strike(me), 1);
}

#[test]
fn bt10_057_two_suspended_any_traits_give_2000_and_one_check() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    place_suspended(&mut r, 0, "BEAST", 2);
    assert_eq!(r.effective_dp(me), Some(14000));
    assert_eq!(r.game.effective_security_strike(me), 2);
}

#[test]
fn bt10_057_three_suspended_floor_to_one_bonus() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    place_suspended(&mut r, 0, "BEAST", 3);
    assert_eq!(r.effective_dp(me), Some(14000));
    assert_eq!(r.game.effective_security_strike(me), 2);
}

/// Q1981 + Q1982: BloomLordmon itself + 3 others = 4 → +4000, S.A. +2.
#[test]
fn bt10_057_q1981_q1982_four_including_itself_double_bonus() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    suspend(&mut r, me);
    place_suspended(&mut r, 0, "BEAST", 3);
    assert_eq!(r.effective_dp(me), Some(16000));
    assert_eq!(r.game.effective_security_strike(me), 3);
}

#[test]
fn bt10_057_opponents_suspended_digimon_do_not_count() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    place_suspended(&mut r, 1, "OPP-VEG", 2);
    assert_eq!(r.effective_dp(me), Some(12000));
    assert_eq!(r.game.effective_security_strike(me), 1);
}

#[test]
fn bt10_057_not_on_the_opponents_turn() {
    let mut r = runner();
    let me = field_bloomlordmon(&mut r);
    place_suspended(&mut r, 0, "BEAST", 2);
    r.game.turn_player_idx = 1;
    assert_eq!(r.effective_dp(me), Some(12000));
    assert_eq!(r.game.effective_security_strike(me), 1);
}
