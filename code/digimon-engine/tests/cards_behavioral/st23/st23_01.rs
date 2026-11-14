//! ST23-01 Kekkomon — Digi-Egg, Lv.2, Green, Cost 0.
//! Traits: Lesser, Glowing Dawn, BEATBREAK. Form: In-Training.
//!
//! # Card text (data/cards.json — verbatim)
//! The face has NO effect. The INHERITED effect is the whole card:
//!   Inherited [When Attacking] [Once Per Turn] By trashing the bottom
//!   face-down card from under any of your Tamers, this Digimon may digivolve
//!   into a [Glowing Dawn] trait Digimon card in the hand with the cost reduced
//!   by 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_01.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - G4 inherited [When Attacking] [Once Per Turn] on a Digi-Egg (single clause)
//! - activation cost: trash_bottom_face_down_source_under_tamer (BEATBREAK stash)
//! - effect-initiated digivolve into a HAND card at cost: reduce 2 (target: source)
//! - "By trashing ..." = optional processing condition: clause-level optional
//!   (DCGO isOptional activation, ST23_01.cs:20); the cost is payable whether
//!   or not the digivolve can follow (CR 4.3 15-7-5), unpayable without a
//!   face-down source (15-7-4)
//! - "may digivolve" → optional select_hand after the cost is paid

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "ST23-01";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.3 Green Glowing-Dawn carrier — the Digimon Kekkomon sits beneath.
fn carrier_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// The digivolve-INTO target: a Lv.4 Green [Glowing Dawn] Digimon with a
/// printed digivolution cost of 4 from a Lv.3 green. Reduced by 2 → 2 paid.
fn gd_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "GD Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 3, // Green
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Lv.4 NON-Glowing-Dawn Digimon (same route) — illegal pick for the filter.
fn non_gd_target(id: &str) -> CardData {
    let mut c = make_test_card(id, "Non GD Target");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 6;
    c.traits = vec!["Mammal".to_string()];
    c.evo_costs = vec![EvoCost {
        card_color: 3,
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
    c.colors = vec![CardColor::Green];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
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
        .expect("ST23-01 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(gd_target("GD-TARGET"))
        .add_card(non_gd_target("NON-GD"))
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

/// Place Kekkomon as the bottom digivolution source under a Lv.3 green carrier.
fn carrier_over_kekkomon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "CARRIER"])
}

/// Place a Tamer with one face-down source beneath it (the cost source).
fn tamer_with_face_down(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["FILLER", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

/// Accept the optional activation ("By trashing ...") and pay the cost: answer
/// the yes/no prompt, then pick the (only) Tamer with a face-down source.
fn activate_and_pay(r: &mut DebugRunner) {
    let view = r
        .pending_selection_view()
        .expect("the optional 'By trashing ...' activation prompt surfaces");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional, "paying the cost is the player's choice");
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    let view = r
        .pending_selection_view()
        .expect("the payment Tamer pick surfaces");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "the cost, once chosen, must be paid");
    assert_eq!(view.valid_action_ids.len(), 1, "one Tamer holds a face-down source");
    let tamer = view.valid_action_ids[0];
    r.execute_action(0, tamer).unwrap();
}

fn face_down_count(r: &DebugRunner) -> usize {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.card_sources.iter().filter(|s| s.face_down).count())
        .sum()
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
fn st23_01_is_green_digi_egg_with_glowing_dawn_traits() {
    let runner = base_runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Kekkomon");
    assert_eq!(card.kind, CompiledCardKind::DigiEgg);
    assert_eq!(card.level, Some(2));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
}

#[test]
fn st23_01_only_effect_is_inherited_when_attacking_opt() {
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
    assert!(
        t.optional,
        "'By trashing ...' is an optional processing condition (DCGO isOptional)"
    );

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

/// POSITIVE: with a Tamer holding a face-down source AND a legal Lv.4 [Glowing
/// Dawn] in hand, the carrier digivolves into it; the Tamer's bottom face-down
/// source is trashed (cost) and the digivolution cost is reduced by 2 (4 → 2).
#[test]
fn st23_01_may_digivolve_into_glowing_dawn_with_cost_reduced_2() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    let tamer = tamer_with_face_down(&mut r);
    // Hand: the legal Glowing-Dawn target + a non-Glowing-Dawn Lv.4 (illegal).
    push_to_hand(&mut r, 0, "GD-TARGET");
    push_to_hand(&mut r, 0, "NON-GD");
    let mem_before = r.memory();
    let tamer_sources_before = r.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    activate_and_pay(&mut r);
    let view = r
        .pending_selection_view()
        .expect("the 'you may digivolve' hand pick must surface");
    assert!(view.is_optional, "'may digivolve' ⇒ declinable");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Lv.4 [Glowing Dawn] hand card is a legal pick; got {:?}",
        view.valid_action_ids
    );
    let pick = view.valid_action_ids[0];
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    let perm = &r.game.player(0).battle_area[carrier.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "GD-TARGET",
        "the carrier digivolves into the [Glowing Dawn] hand card"
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

/// Declining the activation ("By trashing ..." is optional): PASS at the yes/no
/// prompt pays nothing, changes nothing, and keeps the [Once Per Turn] use — a
/// second [When Attacking] re-offers it (DCGO: a declined optional activation
/// is not counted against maxPerTurn).
#[test]
fn st23_01_activation_is_declinable_and_keeps_opt() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();
    let fd_before = face_down_count(&r);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let view = r
        .pending_selection_view()
        .expect("the optional activation prompt surfaces");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(view.is_optional);
    r.execute_action(0, PASS).expect("declining is legal");
    assert!(r.game.pending_selection.is_none());

    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "declined ⇒ still the original carrier"
    );
    assert_eq!(face_down_count(&r), fd_before, "declined ⇒ no source trashed");
    assert_eq!(r.memory(), mem_before, "declined ⇒ no cost paid");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.game.pending_selection.is_some(),
        "a declined activation keeps the [Once Per Turn] use"
    );
}

/// The digivolve is DECLINABLE ("may") after the cost is paid: PASS at the hand
/// pick keeps the carrier and memory, but the face-down source stays trashed
/// (ST23_01.cs:90-96 trashes before the digivolve) and the OPT is spent.
#[test]
fn st23_01_digivolve_is_declinable_after_paying() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");
    let mem_before = r.memory();
    let fd_before = face_down_count(&r);

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    activate_and_pay(&mut r);
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
    assert_eq!(face_down_count(&r), fd_before - 1, "the paid cost stays paid");
    assert_eq!(r.memory(), mem_before, "no digivolution cost paid");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    assert!(
        r.game.pending_selection.is_none(),
        "a paid activation spends the [Once Per Turn] use"
    );
}

/// CR 4.3 15-7-5: no [Glowing Dawn] Digimon in hand does not stop the player
/// from paying the cost (DCGO CanActivate has no hand gate, ST23_01.cs:33-37).
/// Paying trashes the source; no hand pick installs; the carrier stays.
#[test]
fn st23_01_no_glowing_dawn_in_hand_cost_still_payable() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "NON-GD");
    let fd_before = face_down_count(&r);
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    activate_and_pay(&mut r);

    assert!(
        r.game.pending_selection.is_none(),
        "no legal [Glowing Dawn] pick ⇒ no hand pick installs"
    );
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER",
        "nothing digivolved"
    );
    assert_eq!(face_down_count(&r), fd_before - 1, "the cost was paid");
    assert_eq!(r.memory(), mem_before);
}

/// NEGATIVE (cost gate): no Tamer with a face-down source ⇒ the activation cost
/// is unpayable, so the effect cannot activate — clean no-op.
#[test]
fn st23_01_no_activation_without_face_down_source() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    // A Tamer is on the field but with NO face-down source beneath it.
    r.place_on_field(0, "TAMER", Some(0));
    push_to_hand(&mut r, 0, "GD-TARGET");
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
fn st23_01_inherited_digivolve_is_once_per_turn() {
    let mut r = base_runner();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    let _tamer = tamer_with_face_down(&mut r);
    push_to_hand(&mut r, 0, "GD-TARGET");

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    activate_and_pay(&mut r);
    let view = r
        .pending_selection_view()
        .expect("first [When Attacking] offers the digivolve");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "GD-TARGET",
        "first activation digivolved"
    );

    push_to_hand(&mut r, 0, "GD-TARGET");
    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn] ⇒ the second [When Attacking] must not re-offer the digivolve"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — hand-pick legality: only cards this Digimon can digivolve into
// ════════════════════════════════════════════════════════════════════════════
//
// DCGO ST23_01.cs:39-42 `CanSelectCardCondition = EqualsTraits("Glowing
// Dawn")` is handed to `CardEffectCommons.DigivolveIntoHandOrTrashCard`
// (:101-109, payCost: true, reduceCost 2), whose own candidate filter
// (CardEffectCommons.cs:858-881) adds `IsDigimon && !CanNotEvolve(target) &&
// CanPlayCardTargetFrame(frame, PayCost: true, …)`: the card must meet a
// digivolution requirement of this Digimon and the reduced cost must be
// payable (CardSource.cs CanPlayCardTargetFrame: `Owner.MaxMemoryCost < cost`
// refuses). CR 4.3 1-3-11-1: a card can be declared for digivolution only if it
// can be digivolved, never if its cost can't be paid; 8-1-2-6.

/// A Lv.5 green [Glowing Dawn] Digimon that digivolves only from a Lv.4: the
/// Lv.3 carrier does not meet its requirement.
fn gd_lv5(id: &str) -> CardData {
    let mut c = gd_target(id);
    c.card_name = id.to_string();
    c.level = Some(5);
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 4,
        memory_cost: 3,
    }];
    c
}

/// A Lv.4 green [Glowing Dawn] Digimon whose digivolution cost from a green
/// Lv.3 is 13 (11 after the reduction).
fn gd_expensive(id: &str) -> CardData {
    let mut c = gd_target(id);
    c.card_name = id.to_string();
    c.evo_costs = vec![EvoCost {
        card_color: 3,
        level: 3,
        memory_cost: 13,
    }];
    c
}

fn legality_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-01 in embedded DSL pack")
        .add_card(carrier_lv3("CARRIER"))
        .add_card(gd_target("GD-TARGET"))
        .add_card(gd_lv5("GD-LV5"))
        .add_card(gd_expensive("GD-EXP"))
        .add_card(tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();
    r.set_first_player(0);
    let carrier = carrier_over_kekkomon(&mut r);
    tamer_with_face_down(&mut r);
    for h in hand {
        push_to_hand(&mut r, 0, h);
    }
    (r, carrier)
}

/// Fire [When Attacking], accept and pay the cost, and return the sorted card
/// ids offered at the hand pick (empty when no hand pick installs).
fn offered(r: &mut DebugRunner, carrier: PermanentHandle) -> Vec<String> {
    use digimon_engine::action::space::{HAND_EFFECT_START, PLAY_HAND_START};
    fire(r, EffectTiming::WhenAttacking, carrier);
    activate_and_pay(r);
    let Some(v) = r.pending_selection_view() else {
        return Vec::new();
    };
    let mut out: Vec<String> = r.game.players[0]
        .hand
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let i = *i as u16;
            v.valid_action_ids.contains(&(PLAY_HAND_START + i))
                || v.valid_action_ids.contains(&(HAND_EFFECT_START + i))
        })
        .map(|(_, c)| c.card_id(&r.game.card_data).to_string())
        .collect();
    out.sort();
    out
}

/// Regression guard: a [Glowing Dawn] Lv.5 that this Lv.3 can't digivolve into
/// is not offered (the compile pass adds `can_digivolve_onto: source`).
#[test]
fn st23_01_hand_pick_excludes_card_whose_requirement_is_unmet() {
    let (mut r, carrier) = legality_runner(&["GD-LV5", "GD-TARGET"]);
    assert_eq!(offered(&mut r, carrier), vec!["GD-TARGET"]);
}

/// Memory 0: GD-EXP would cost 11 after the reduction, past the 10-memory limit
/// on the opponent's side, so it can't be digivolved into and is not offered;
/// GD-TARGET (pay 2) still is.
#[test]
fn st23_01_hand_pick_excludes_unaffordable_digivolution() {
    let (mut r, carrier) = legality_runner(&["GD-EXP", "GD-TARGET"]);
    r.game.memory = 0;
    assert_eq!(offered(&mut r, carrier), vec!["GD-TARGET"]);
}

/// Memory 0 with only the unaffordable card in hand: the cost is still offered
/// and can be paid (CR 4.3 15-7-5; DCGO ST23_01.cs:90-96 trashes before it
/// looks at the hand), after which no hand pick installs and the carrier stays.
#[test]
fn st23_01_unaffordable_only_card_cost_payable_but_no_pick() {
    let (mut r, carrier) = legality_runner(&["GD-EXP"]);
    r.game.memory = 0;
    let fd_before = face_down_count(&r);
    assert!(offered(&mut r, carrier).is_empty());
    assert!(r.game.pending_selection.is_none(), "no hand pick installs");
    assert_eq!(
        r.game.player(0).battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "CARRIER"
    );
    assert_eq!(face_down_count(&r), fd_before - 1, "the cost was paid");
    assert_eq!(r.game.memory, 0, "no digivolution cost paid");
}
