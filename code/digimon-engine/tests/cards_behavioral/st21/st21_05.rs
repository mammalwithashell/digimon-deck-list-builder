//! ST21-05 Angemon — Digimon, Lv.4, Yellow, DP 4000, Cost 4.
//! Traits: Angel / ADVENTURE. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/ST21-05.md — official Bandai DB, verbatim)
//!
//! ```text
//! Digivolve: Yellow Lv.3 / cost 2
//! [Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer card
//! with the [ADVENTURE] trait from your hand without paying the cost.
//! Inherited Effect
//! [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
//! for the turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST21/Yellow/ST21_05.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - "[Digivolve] Lv.3 w/[ADVENTURE] trait: Cost 2" alt-path (ST20-10 exemplar)
//! - [When Digivolving] gated on "1 or fewer Tamers" (BT22-022 idiom): optional
//!   hand pick filtered to `kind: tamer` + [ADVENTURE], free play
//! - Inherited [When Attacking] [Once Per Turn] -2000 DP to a chosen opponent
//!   Digimon until end of turn (BT24-036 select + add_dp_modifier idiom)
//! - Event-log cost test (digivolve cost 2 → MemoryChange -2), OPT lockout and
//!   re-arm

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-05";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn dig(id: &str, color: CardColor, level: u8, dp: i32, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn tamer(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-05 in embedded DSL pack")
        .add_card(dig("YELLOW3", CardColor::Yellow, 3, 3000, false))
        .add_card(dig("RED3-ADV", CardColor::Red, 3, 3000, true))
        .add_card(dig("RED3-PLAIN", CardColor::Red, 3, 3000, false))
        .add_card(dig("YELLOW2-ADV", CardColor::Yellow, 2, 1000, true))
        .add_card(dig("ADV-DIGI", CardColor::Yellow, 3, 3000, true))
        .add_card(dig("OPP", CardColor::Red, 4, 5000, false))
        .add_card(dig("OPP2", CardColor::Red, 4, 5000, false))
        .add_card(dig("BIG-HOST", CardColor::Yellow, 4, 9000, false))
        .add_card(dig("SEC", CardColor::Red, 3, 1000, false))
        .add_card(tamer("ADV-T1", true))
        .add_card(tamer("ADV-T2", true))
        .add_card(tamer("PLAIN-T", false))
        .add_card(tamer("EXTRA-T1", false))
        .add_card(tamer("EXTRA-T2", false))
        .add_card(make_test_card("FILLER", "Filler"))
}

fn runner() -> DebugRunner {
    let mut r = base_builder()
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC"; 8])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.game.turn_count = 3;
    r
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn field_pick(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in hand: {:?}", hand_ids(r, p)))
}

/// Card ids offered by the pending hand prompt (PASS excluded), sorted.
fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let hand = hand_ids(r, view.selecting_player);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let p = view.selecting_player;
    let slot = hand_index(r, p, id) as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    r.execute_action(p, a).expect("pick hand card");
}

fn own_tamer_count(r: &DebugRunner) -> usize {
    r.game.players[0]
        .battle_area
        .iter()
        .filter(|perm| {
            let top = perm.top_card();
            r.game.card_data[top.data_index].card_kind == CardKind::Tamer
        })
        .count()
}

/// Put the given cards in P0's hand, place `base` on P0's field, add Angemon
/// to the hand last, and digivolve it onto `base`. Returns (ok, base handle).
fn digivolve_with_hand(r: &mut DebugRunner, base: &str, hand: &[&str]) -> (bool, PermanentHandle) {
    for id in hand {
        r.add_to_hand(0, id);
    }
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, h)
}

fn digivolve_from(base: &str) -> (bool, i16) {
    let mut r = runner();
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, before - r.memory())
}

fn triggered<'a>(
    card: &'a digimon_dsl::compiled::CompiledCard,
    f: impl Fn(&CompiledTriggeredClause) -> bool,
) -> Option<&'a CompiledTriggeredClause> {
    card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if f(t) => Some(t),
        _ => None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_05_identity_matches_printed_card() {
    let c = compiled(CARD_ID);
    assert_eq!(c.name, "Angemon");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.level, Some(4));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.dp, Some(4000));
    assert_eq!(c.color, vec![CompiledColor::Yellow]);
    assert!(c.traits.iter().any(|t| t == "Angel"), "{:?}", c.traits);
    assert!(c.traits.iter().any(|t| t == "ADVENTURE"), "{:?}", c.traits);
}

#[test]
fn st21_05_has_two_cost_two_requirement_respecting_digivolve_paths() {
    let c = compiled(CARD_ID);
    let paths: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(paths.len(), 2, "printed yellow Lv.3 circle + ADVENTURE box");
    for p in &paths {
        assert_eq!(p.cost, Some(CompiledCost::Literal(2)));
        assert!(!p.ignore_requirements);
    }
    let std_from = paths[0].from.as_ref().expect("yellow Lv.3 `from:`");
    assert_eq!(std_from.level_eq, Some(3));
    assert_eq!(std_from.color_is, Some(CompiledColor::Yellow));
    let adv_from = paths[1].from.as_ref().expect("ADVENTURE Lv.3 `from:`");
    assert_eq!(adv_from.level_eq, Some(3));
    assert!(format!("{adv_from:?}").contains("ADVENTURE"));
}

#[test]
fn st21_05_clause_shapes() {
    let c = compiled(CARD_ID);

    let wd = triggered(&c, |t| {
        t.scope == CompiledScope::FaceUp && t.when.contains(&CompiledTiming::WhenDigivolving)
    })
    .expect("[When Digivolving] clause");
    assert_eq!(wd.when, vec![CompiledTiming::WhenDigivolving]);
    assert!(
        wd.condition.is_some(),
        "'If you have 1 or fewer Tamers' is the clause condition"
    );
    assert!(!wd.once_per_turn);

    let inh = triggered(&c, |t| t.scope == CompiledScope::Inherited)
        .expect("inherited [When Attacking] clause");
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh.once_per_turn, "[Once Per Turn]");
    assert!(!inh.optional, "the -2000 DP is not a 'you may'");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve conditions + cost event
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_05_digivolves_from_yellow_lv3_for_two() {
    let (ok, paid) = digivolve_from("YELLOW3");
    assert!(ok);
    assert_eq!(paid, 2);
}

#[test]
fn st21_05_digivolves_from_non_yellow_lv3_with_adventure_for_two() {
    let (ok, paid) = digivolve_from("RED3-ADV");
    assert!(ok, "[Digivolve] Lv.3 w/[ADVENTURE] lifts the colour gate");
    assert_eq!(paid, 2);
}

#[test]
fn st21_05_cannot_digivolve_from_non_yellow_lv3_without_adventure() {
    let (ok, _) = digivolve_from("RED3-PLAIN");
    assert!(!ok);
}

#[test]
fn st21_05_cannot_digivolve_from_lv2_adventure() {
    let (ok, _) = digivolve_from("YELLOW2-ADV");
    assert!(!ok, "both circles need Lv.3");
}

#[test]
fn st21_05_digivolve_cost_event_is_two_memory() {
    let mut r = runner();
    let h = r.place_on_field(0, "RED3-ADV", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let cp = r.event_checkpoint();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand));
    assert!(
        r.events_since(cp)
            .iter()
            .any(|e| matches!(e, GameEvent::MemoryChange { delta: -2, .. })),
        "the digivolution cost of 2 must hit the event log"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving]: <= 1 Tamers ⇒ may play an [ADVENTURE] Tamer
// ═══════════════════════════════════════════════════════════════════════════

const WD_HAND: [&str; 4] = ["ADV-T1", "ADV-T2", "PLAIN-T", "ADV-DIGI"];

#[test]
fn st21_05_with_no_tamers_offers_only_adventure_tamer_cards() {
    let mut r = runner();
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);

    let v = r.pending_selection_view().expect("Tamer play prompt");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.kind, SelectionKind::Hand);
    assert!(v.is_optional, "'you may' — PASS must be legal");
    assert_eq!(
        offered_hand_ids(&r),
        vec!["ADV-T1".to_string(), "ADV-T2".to_string()],
        "only Tamer cards with [ADVENTURE]: not the plain Tamer, not the [ADVENTURE] Digimon"
    );
}

#[test]
fn st21_05_with_exactly_one_tamer_still_triggers() {
    let mut r = runner();
    r.place_on_field(0, "EXTRA-T1", Some(0));
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);
    assert!(
        r.pending_selection().is_some(),
        "1 Tamer is within '1 or fewer'"
    );
}

#[test]
fn st21_05_with_two_tamers_does_not_trigger() {
    let mut r = runner();
    r.place_on_field(0, "EXTRA-T1", Some(0));
    r.place_on_field(0, "EXTRA-T2", Some(0));
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);
    assert!(
        r.pending_selection().is_none(),
        "2 Tamers exceeds '1 or fewer' ⇒ no prompt"
    );
    assert_eq!(own_tamer_count(&r), 2);
}

#[test]
fn st21_05_opponents_tamers_do_not_count_toward_your_tamer_total() {
    let mut r = runner();
    r.place_on_field(1, "EXTRA-T1", Some(0));
    r.place_on_field(1, "EXTRA-T2", Some(0));
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);
    assert!(
        r.pending_selection().is_some(),
        "'If you have 1 or fewer Tamers' counts only YOUR Tamers"
    );
}

#[test]
fn st21_05_accepting_plays_the_chosen_tamer_free() {
    let mut r = runner();
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);
    let memory_after_digivolve = r.memory();
    let tamers_before = own_tamer_count(&r);

    pick_hand(&mut r, "ADV-T2");
    r.auto_resolve().unwrap();

    assert_eq!(
        own_tamer_count(&r),
        tamers_before + 1,
        "a Tamer entered play"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "ADV-T2"),
        "the chosen ADV-T2 is on the battle area"
    );
    assert!(
        !hand_ids(&r, 0).contains(&"ADV-T2".to_string()),
        "ADV-T2 left the hand"
    );
    assert!(
        hand_ids(&r, 0).contains(&"ADV-T1".to_string()),
        "the unchosen Tamer stays in hand"
    );
    assert_eq!(
        r.memory(),
        memory_after_digivolve,
        "'without paying the cost' — no memory is spent on the Tamer"
    );
}

#[test]
fn st21_05_declining_plays_nothing() {
    let mut r = runner();
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &WD_HAND);
    assert!(ok);
    let tamers_before = own_tamer_count(&r);
    let hand_before = r.hand_size(0);

    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();

    assert_eq!(own_tamer_count(&r), tamers_before);
    assert_eq!(r.hand_size(0), hand_before);
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_05_no_adventure_tamer_in_hand_means_no_prompt() {
    let mut r = runner();
    let (ok, _) = digivolve_with_hand(&mut r, "YELLOW3", &["PLAIN-T", "ADV-DIGI"]);
    assert!(ok);
    assert!(
        r.pending_selection().is_none(),
        "no legal [ADVENTURE] Tamer in hand ⇒ nothing to offer"
    );
}

#[test]
fn st21_05_when_digivolving_does_not_fire_when_played_directly() {
    let mut r = runner();
    for id in WD_HAND {
        r.add_to_hand(0, id);
    }
    let hi = r.add_to_hand(0, CARD_ID);
    r.play(0, hi).expect("Angemon plays from hand");
    assert!(
        r.pending_selection().is_none(),
        "[When Digivolving] is not [On Play]"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [When Attacking] [Once Per Turn]: -2000 DP
// ═══════════════════════════════════════════════════════════════════════════

fn attacker_over_angemon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "BIG-HOST"])
}

#[test]
fn st21_05_inherited_attack_trigger_lowers_a_chosen_opponent_digimon_by_2000() {
    let mut r = runner();
    let opp = r.place_on_field(1, "OPP", Some(0));
    let host = attacker_over_angemon(&mut r);
    let dp_before = r.effective_dp(opp).unwrap();

    let _ = r.attack_player(host, 1, false);

    let v = r.pending_selection_view().expect("-2000 DP target pick");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional, "the -2000 DP pick is mandatory");
    assert!(v.valid_action_ids.contains(&field_pick(opp)));
    r.execute_action(0, field_pick(opp)).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(
        r.effective_dp(opp),
        Some(dp_before - 2000),
        "-2000 DP applied to the chosen Digimon"
    );
}

#[test]
fn st21_05_only_the_chosen_digimon_is_affected() {
    let mut r = runner();
    let a = r.place_on_field(1, "OPP", Some(0));
    let b = r.place_on_field(1, "OPP2", Some(0));
    let host = attacker_over_angemon(&mut r);
    let a_before = r.effective_dp(a).unwrap();
    let b_before = r.effective_dp(b).unwrap();

    let _ = r.attack_player(host, 1, false);
    let v = r.pending_selection_view().expect("pick");
    assert!(v.valid_action_ids.contains(&field_pick(a)));
    assert!(v.valid_action_ids.contains(&field_pick(b)));
    r.execute_action(0, field_pick(b)).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(r.effective_dp(a), Some(a_before), "A untouched");
    assert_eq!(r.effective_dp(b), Some(b_before - 2000), "B reduced");
}

#[test]
fn st21_05_dp_reduction_lasts_the_turn_then_wears_off() {
    let mut r = runner();
    let opp = r.place_on_field(1, "OPP", Some(0));
    let host = attacker_over_angemon(&mut r);
    let dp_before = r.effective_dp(opp).unwrap();

    let _ = r.attack_player(host, 1, false);
    r.execute_action(0, field_pick(opp)).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(opp), Some(dp_before - 2000));

    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(
        r.effective_dp(opp),
        Some(dp_before),
        "'for the turn' — the -2000 is gone after the turn ends"
    );
}

#[test]
fn st21_05_inherited_trigger_is_once_per_turn_and_rearms() {
    let mut r = runner();
    let opp = r.place_on_field(1, "OPP", Some(0));
    let host = attacker_over_angemon(&mut r);
    let dp_before = r.effective_dp(opp).unwrap();

    let _ = r.attack_player(host, 1, false);
    r.execute_action(0, field_pick(opp)).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(opp), Some(dp_before - 2000));

    // Second attack the same turn: no new prompt, no stacking.
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    let _ = r.attack_player(host, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: the second attack in the same turn does not trigger it"
    );
    let _ = r.auto_resolve();
    assert_eq!(
        r.effective_dp(opp),
        Some(dp_before - 2000),
        "no second -2000"
    );

    // Two end_turns later it is my turn again and the once-per-turn use is back.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    r.game.players[0].battle_area[host.index as usize].is_suspended = false;
    let _ = r.attack_player(host, 1, false);
    let v = r
        .pending_selection_view()
        .expect("the once-per-turn use re-arms on my next turn");
    assert!(v.valid_action_ids.contains(&field_pick(opp)));
}

#[test]
fn st21_05_attack_with_no_opposing_digimon_has_nothing_to_target() {
    let mut r = runner();
    let host = attacker_over_angemon(&mut r);
    let sec_before = r.security_count(1);
    let _ = r.attack_player(host, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "no opposing Digimon ⇒ no -2000 DP pick"
    );
    let _ = r.auto_resolve();
    assert!(
        r.security_count(1) < sec_before,
        "the attack still resolved"
    );
}

#[test]
fn st21_05_inherited_effect_requires_angemon_in_the_digivolution_cards() {
    let mut r = runner();
    let opp = r.place_on_field(1, "OPP", Some(0));
    let host = r.place_on_field(0, "BIG-HOST", Some(0));
    let _ = r.attack_player(host, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "a Digimon without Angemon under it gets no -2000 DP trigger"
    );
}
