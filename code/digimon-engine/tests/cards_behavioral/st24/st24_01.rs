//! ST24-01 Koromon — Digi-Egg, Lv.2, Yellow, Cost 0.
//! Traits: Lesser, DATA SQUAD. Form: In-Training.
//!
//! # Card text (data/cards.json — verbatim)
//! The face has NO effect. The INHERITED effect is the whole card:
//!   Inherited [When Attacking] [Once Per Turn] By trashing the bottom
//!   face-down card from under any of your Tamers, this Digimon may digivolve
//!   into a [DATA SQUAD] trait Digimon card in the hand with the cost reduced
//!   by 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_01.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3) — sister to ST23-01 (trait-swapped)
//! - G4 inherited [When Attacking] [Once Per Turn] on a Digi-Egg (single clause)
//! - activation cost: trash_bottom_face_down_source_under_tamer (DATA SQUAD stash)
//! - effect-initiated digivolve into a HAND card at cost: reduce 2 (target: source)
//! - "may" → optional inner select_hand; cost paid only on a real pick

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST24-01";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.3 Yellow DATA-SQUAD carrier — the Digi-Egg Pyocomon sits beneath.
fn carrier_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// The digivolve-INTO target: a Lv.4 Yellow [DATA SQUAD] Digimon with a printed
/// digivolution cost of 4 from a Lv.3 yellow. Reduced by 2 → 2 paid.
fn ds_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "DS Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["DATA SQUAD".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 2, // Yellow
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Lv.4 NON-DATA-SQUAD Digimon (same route) — illegal pick for the filter.
fn non_ds_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non DS Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["Mammal".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A vanilla Tamer (under which we stash a face-down source for the cost).
fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-01 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(ds_target("DS-TARGET"))
        .add_card(non_ds_target("NON-DS"))
        .add_card(tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Place Koromon as the bottom digivolution source under a Lv.3 yellow carrier.
fn carrier_over_egg(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

/// Place a Tamer with one face-down source beneath it (the cost source).
fn tamer_with_face_down(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["FILLER", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

fn process_contains_trash_fd_under_tamer(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::TrashBottomFaceDownSourceUnderTamer { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_trash_fd_under_tamer(then)
                || process_contains_trash_fd_under_tamer(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_trash_fd_under_tamer(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_01_is_yellow_digi_egg_with_data_squad_trait() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn st24_01_only_effect_is_inherited_when_attacking_opt() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "single triggered clause");
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited, "inherited clause");
    assert!(
        t.when.contains(&CompiledTiming::WhenAttacking),
        "fires [When Attacking]"
    );
    assert!(t.once_per_turn, "[Once Per Turn]");

    let has_optional_hand_pick = t
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectHand { optional: true, .. }));
    assert!(
        has_optional_hand_pick,
        "'may digivolve' → optional select_hand carries the decline"
    );
    assert!(
        process_contains_trash_fd_under_tamer(&t.process),
        "activation cost trashes a bottom face-down source under a Tamer"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with a Tamer holding a face-down source AND a legal Lv.4 [DATA
/// SQUAD] in hand, the carrier digivolves into it; the Tamer's bottom face-down
/// source is trashed (cost) and the digivolution cost is reduced by 2 (4 → 2).
#[test]
fn st24_01_may_digivolve_into_data_squad_with_cost_reduced_2() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let tamer = tamer_with_face_down(&mut r);
    // Hand: the legal DATA SQUAD target + a non-DATA-SQUAD Lv.4 (illegal).
    push_to_hand(&mut r, 0, "DS-TARGET");
    push_to_hand(&mut r, 0, "NON-DS");
    let mem_before = r.memory();
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("the 'you may digivolve' hand pick must surface");
    assert!(view.is_optional, "'may digivolve' ⇒ declinable");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Lv.4 [DATA SQUAD] hand card is a legal pick; got {:?}",
        view.valid_action_ids
    );
    let pick = view.valid_action_ids[0];
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    let perm = &r.game.player(0).battle_area[carrier.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "DS-TARGET",
        "the carrier digivolves into the [DATA SQUAD] hand card"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before - 1,
        "the Tamer's bottom face-down source is trashed as the cost"
    );
    assert_eq!(
        r.memory(),
        mem_before - 2,
        "the digivolution cost (4) is reduced by 2 → pays 2"
    );
}

/// The digivolve is DECLINABLE ("may"): PASS leaves the board, the Tamer stash,
/// and memory unchanged (the trash cost is paid only if the digivolve is taken).
#[test]
fn st24_01_digivolve_is_declinable() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");
    let mem_before = r.memory();
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("optional hand pick surfaces");
    assert!(view.is_optional);
    r.execute_action(0, PASS).expect("declining is legal");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "declined ⇒ still the original carrier"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "declined ⇒ no face-down source trashed"
    );
    assert_eq!(r.memory(), mem_before, "declined ⇒ no cost paid");
}

/// NEGATIVE (filter): no legal [DATA SQUAD] Digimon in hand ⇒ clean no-op.
#[test]
fn st24_01_no_data_squad_in_hand_clean_noop() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "NON-DS");
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no legal [DATA SQUAD] pick ⇒ no stuck selection"
    );
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "nothing digivolved"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before,
        "no digivolve ⇒ no source trashed"
    );
}

/// NEGATIVE (cost gate): no Tamer with a face-down source ⇒ the activation cost
/// is unpayable, so the effect cannot activate — clean no-op.
#[test]
fn st24_01_no_activation_without_face_down_source() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    // A Tamer is on the field but with NO face-down source beneath it.
    r.place_on_field(0, "TAMER", Some(0));
    push_to_hand(&mut r, 0, "DS-TARGET");
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no face-down source to pay the cost ⇒ no prompt"
    );
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "no digivolve when the cost is unpayable"
    );
    assert_eq!(r.memory(), mem_before, "no cost paid");
}

/// [Once Per Turn]: a second [When Attacking] in the same turn cannot re-activate.
#[test]
fn st24_01_inherited_digivolve_is_once_per_turn() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("first [When Attacking] offers the digivolve");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "DS-TARGET",
        "first activation digivolved"
    );

    push_to_hand(&mut r, 0, "DS-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] ⇒ the second [When Attacking] must not re-offer the digivolve"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Hand-pick legality: only cards THIS Digimon can digivolve into,
// at the reduced cost, are offered.
// DCGO ST24_01.cs:100-111 calls `DigivolveIntoHandOrTrashCard(payCost: true,
// reduceCost 2, isHand: true)`, whose own `CanSelectCardCondition`
// (CardEffectCommons.cs:860-883) offers a hand card when `IsDigimon`, the
// card filter, `!CanNotEvolve(target)` and `CanPlayCardTargetFrame(frame,
// PayCost: true, …)` hold. That frame check covers the route and the
// digivolve lock but NOT memory: for an occupied frame it skips the cost test
// (CardSource.cs:1151-1162) and goes to `CanEvolve`/`EvoCosts`. So DCGO
// OFFERS an unpayable card and only fails the digivolve afterwards in
// `PlayCardClass` (CardController.cs:1008-1014, `playFailed`).
// We exclude it from the pick per CR 4.3 1-3-11-1 (a player can't declare a
// digivolve whose cost they can't pay), 8-1-2-6 (the refused digivolve) and
// 1-3-2 (an impossible action is not carried out). Known candidate-set
// difference from DCGO: a recording that picks an unpayable card replays here
// as an illegal action, and that is expected.
// ════════════════════════════════════════════════════════════════════════════

/// A Lv.4 Yellow [DATA SQUAD] Digimon whose Lv.3 digivolution cost is 13: with
/// the reduction it costs 11, more than the 10 a player at 0 memory can pay.
fn ds_pricey(id: &str) -> CardData {
    let mut c = ds_target(id);
    c.card_name = "DS Pricey".to_string();
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 3,
        memory_cost: 13,
    }];
    c
}

/// A Lv.6 Yellow [DATA SQUAD] Digimon with only a Lv.5 route: no route from
/// the Lv.3 carrier.
fn ds_lv6(id: &str) -> CardData {
    let mut c = ds_target(id);
    c.card_name = "DS Lv6".to_string();
    c.level = Some(6);
    c.evo_costs = vec![EvoCost {
        card_color: 2,
        level: 5,
        memory_cost: 3,
    }];
    c
}

fn legality_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-01 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(ds_target("DS-TARGET"))
        .add_card(ds_pricey("DS-PRICEY"))
        .add_card(ds_lv6("DS-LV6"))
        .add_card(tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start()
}

/// Fire the inherited [When Attacking] and return the hand-pick candidates'
/// card ids (empty when no hand pick installs).
fn offered_digivolve_picks(r: &mut DebugRunner, carrier: PermanentHandle) -> Vec<String> {
    use digimon_engine::action::space::PLAY_HAND_START;
    fire(r, EffectTiming::WhenAttacking, carrier);
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    let hand: Vec<String> = r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
        .collect();
    out.sort();
    out
}

/// At 0 memory the cost-11 digivolve can't be paid (memory can't pass 10 on the
/// opponent's side), so only the cost-2 [DATA SQUAD] card is offered.
#[test]
fn st24_01_hand_pick_excludes_unaffordable_data_squad_digimon() {
    let mut r = legality_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");
    push_to_hand(&mut r, 0, "DS-PRICEY");
    r.game.memory = 0;
    let offered = offered_digivolve_picks(&mut r, carrier);
    assert_eq!(
        offered,
        vec!["DS-TARGET".to_string()],
        "the unpayable cost-11 digivolve is not offered"
    );
}

/// Only the unaffordable card in hand: no pick installs and the Tamer's
/// face-down source is not trashed for nothing.
/// Known difference from DCGO (pre-existing, not from the pick filter): DCGO
/// trashes the Tamer's face-down card before it looks at the hand
/// (ST24_01.cs:93-100), so it would pay that cost here and then fail the
/// digivolve. This test locks in our cost-after-pick ordering.
#[test]
fn st24_01_only_unaffordable_card_offers_no_pick_and_pays_nothing() {
    let mut r = legality_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-PRICEY");
    r.game.memory = 0;
    let before = r.game.players[0].battle_area[tamer.index as usize].card_sources.len();
    let offered = offered_digivolve_picks(&mut r, carrier);
    assert!(offered.is_empty(), "no candidate offered: {offered:?}");
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize].card_sources.len(),
        before,
        "no digivolve ⇒ no face-down source trashed"
    );
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER"
    );
}

/// Regression guard: with enough memory the same card IS offered (the filter
/// prices the reduced cost, not the printed one).
#[test]
fn st24_01_hand_pick_offers_pricey_card_when_affordable() {
    let mut r = legality_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");
    push_to_hand(&mut r, 0, "DS-PRICEY");
    r.game.memory = 1; // 1 - 11 = -10: payable
    let offered = offered_digivolve_picks(&mut r, carrier);
    assert_eq!(offered, vec!["DS-PRICEY".to_string(), "DS-TARGET".to_string()]);
}

/// Regression guard: a [DATA SQUAD] Digimon with no route from the Lv.3
/// carrier is not offered.
#[test]
fn st24_01_hand_pick_excludes_card_without_a_route() {
    let mut r = legality_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");
    push_to_hand(&mut r, 0, "DS-LV6");
    let offered = offered_digivolve_picks(&mut r, carrier);
    assert_eq!(offered, vec!["DS-TARGET".to_string()]);
}

/// "Can't digivolve by effects" (DCGO `CanNotEvolve`): nothing is offered.
#[test]
fn st24_01_hand_pick_empty_when_effect_digivolve_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let mut r = legality_runner();
    r.set_first_player(0);
    let carrier = carrier_over_egg(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "DS-TARGET");
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotDigivolveDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    let offered = offered_digivolve_picks(&mut r, carrier);
    assert!(offered.is_empty(), "no candidate under the lock: {offered:?}");
}
