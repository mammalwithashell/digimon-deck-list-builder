//! BT25-005 Pagumon — DIGI-EGG, Lv.2, Black, Cost 0.
//! Traits: Lesser / Iliad / TS. Form: In-Training.
//!
//! # Card text (data/card_bundles/BT25-005.md — official Bandai DB, verbatim;
//! cross-checked against the card image BT25-005.webp)
//!
//! ```text
//! Inherited Effect [Your Turn] [Once Per Turn] When [Three Musketeers]
//! trait cards are placed in this Digimon's digivolution cards, it may
//! digivolve into a Digimon card with [Three Musketeers] in its text or the
//! [TS] trait in the hand with the cost reduced by 2.
//! ```
//!
//! Official Q&A ("in its text"): name, traits, effects, inherited effects,
//! (Rule), digivolution / DNA / DigiXros / burst / App Fusion / Link /
//! Assembly requirements all count.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_005.cs —
//! `EffectTiming.OnAddDigivolutionCards`, `SetIsInheritedEffect(true)`,
//! `SetUpActivateClass(..., 1 /* OPT */, TRUE /* optional yes/no */)`.
//! CanUse = IsExistOnBattleAreaDigimon && IsOwnerTurn &&
//! `CanTriggerOnAddDigivolutionCard(permanent == self, cardEffectCondition:
//! NULL (any effect — the opponent's too), cardCondition:
//! HasThreeMusketeersTraits)`. Body: if `HasMatchConditionOwnersHand(HasText
//! ("Three Musketeers") || HasTSTraits)` →
//! `DigivolveIntoHandOrTrashCard(targetPermanent: self's permanent,
//! cardCondition, payCost: true, reduceCostTuple: (2, null), isHand: true)`
//! — the hand prompt offers only Digimon cards that can legally digivolve
//! from the host (`CanPlayCardTargetFrame`) and is declinable.
//!
//! # Rules
//! - `general_rule.pdf` 15-14-1-5 (p.29): choosing to activate an [X Per
//!   Turn] effect counts as a use even if its processing can't be executed;
//!   declining the activation itself (DCGO's initial yes/no) does NOT count →
//!   `optional: true` + `outer_prompt: true` (G-OPT-REFUND-ON-DECLINE).
//! - 15-5-2: several cards placed at once by one effect → ONE trigger.
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt25/BT25-005.yaml
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G4: inherited triggered clause, [Your Turn] gate
//! - E2: optional (outer yes/no) + OPT lockout / refund-on-decline
//! - effect-initiated digivolve from hand with a cost reduction
//!   (`effect_initiated_digivolve { cost: { reduce: 2 } }`, BT12-016 idiom)
//! - `can_digivolve_from_source` routed through `all_digivolve_routes_for_card`
//!   (alt-path special circles are offered)
//! - `on_add_digivolution_cards` + `event_host_permanent_is_source` +
//!   `event_added_card_any` (G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-005";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c
}

/// A black Lv.N Digimon with a printed "Black Lv.(N-1) / cost `cost`" circle.
fn black_evo(id: &str, level: u8, cost: u16) -> CardData {
    let mut c = digimon(id, level, CardColor::Black);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Black as u8,
        level: level - 1,
        memory_cost: cost,
    }];
    c
}

/// [TS]-trait Lv.4, digivolves from a black Lv.3 for 3.
fn ts_lv4(id: &str) -> CardData {
    let mut c = black_evo(id, 4, 3);
    c.traits = vec!["Beast".to_string(), "TS".to_string()];
    c
}

/// Lv.4 whose ONLY [Three Musketeers] reference is in its effect text.
fn tm_text_lv4(id: &str) -> CardData {
    let mut c = black_evo(id, 4, 3);
    c.effect_text = "[When Digivolving] Play 1 [Three Musketeers] trait card.".to_string();
    c
}

/// [TS]-trait Lv.4 with a cost-1 circle — the reduction clamps at 0.
fn ts_lv4_cheap(id: &str) -> CardData {
    let mut c = black_evo(id, 4, 1);
    c.traits = vec!["TS".to_string()];
    c
}

/// [TS]-trait Lv.5 — digivolves from a Lv.4, so NOT from the Lv.3 host.
fn ts_lv5(id: &str) -> CardData {
    let mut c = black_evo(id, 5, 4);
    c.traits = vec!["TS".to_string()];
    c
}

fn option(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Black];
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Green];
    c
}

/// The CAUSE: an [On Play] effect that places the first `count` cards of its
/// controller's hand under the permanent at `target`.
struct PlaceHandUnder {
    count: usize,
    target: PermanentHandle,
}
impl CardEffect for PlaceHandUnder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.count;
        let target = self.target;
        vec![Effect::on_play(card)
            .name("test placer: tuck hand cards under a permanent")
            .process(move |ctx| {
                for _ in 0..count {
                    let ok =
                        ctx.place_as_bottom_source(CardSourceRef::Hand(ctx.player, 0), target, false);
                    assert!(ok, "placement must succeed");
                }
            })
            .build()]
    }
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-005 YAML loads from the embedded pack")
        .add_card(digimon("HOST", 3, CardColor::Black))
        .add_card(digimon("SIB", 3, CardColor::Black))
        .add_card(digimon("PLACER", 3, CardColor::Black))
        .add_card(digimon("OPP-PLACER", 3, CardColor::Black))
        .add_card(option("TM-CARD", &["Three Musketeers"]))
        .add_card(option("OTHER-CARD", &["Toho"]))
        .add_card(ts_lv4("TS-LV4"))
        .add_card(tm_text_lv4("TMTEXT-LV4"))
        .add_card(ts_lv4_cheap("TS-CHEAP"))
        .add_card(ts_lv5("TS-LV5"))
        .add_card(black_evo("OTHER-LV4", 4, 3))
        .add_card(filler("F"))
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
}

/// Player 0's turn; `HOST` (black Lv.3) on player 0's slot 0 with Pagumon as
/// its sole digivolution source.
fn host_with_pagumon(r: &mut DebugRunner) -> PermanentHandle {
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    r.place_stack(0, &[CARD_ID, "HOST"])
}

/// Play `placer_id` for `player` and let its [On Play] tuck `player`'s
/// hand[0..count] under `target`. Does NOT auto-resolve — the caller drives
/// Pagumon's prompt.
fn tuck(r: &mut DebugRunner, player: PlayerId, placer_id: &str, count: usize, target: PermanentHandle) {
    r.register_effect(placer_id, Arc::new(PlaceHandUnder { count, target }));
    let placer = r.place_on_field(player, placer_id, Some(0));
    r.game.fire_on_play(player, placer.index as usize);
}

fn hand_action(r: &DebugRunner, player: PlayerId, card_id: &str) -> u16 {
    let idx = r.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    PLAY_HAND_START + idx as u16
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn stack_len(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
}

/// Accept DCGO's outer yes/no (a `Replacement`-kind optional prompt).
fn accept_outer(r: &mut DebugRunner) {
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement), "outer yes/no");
    assert!(r.pending_is_optional(), "the outer yes/no is declinable");
    r.accept_optional_trigger().expect("accept the 'may digivolve'");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt25_005_is_a_black_lv2_lesser_iliad_ts_digi_egg() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.kind, CompiledCardKind::DigiEgg);
    assert_eq!(c.level, Some(2));
    assert_eq!(c.cost, Some(0));
    for t in ["Lesser", "Iliad", "TS"] {
        assert!(c.traits.iter().any(|x| x == t), "missing trait {t}; traits={:?}", c.traits);
    }
}

#[test]
fn bt25_005_single_inherited_optional_opt_your_turn_on_add_digivolution_cards_clause() {
    let r = base().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.effects.len(), 1, "exactly one clause");
    let CompiledClause::Triggered(t) = &c.effects[0] else {
        panic!("triggered clause expected");
    };
    assert_eq!(t.scope, CompiledScope::Inherited, "Inherited Effect");
    assert_eq!(t.when, vec![CompiledTiming::OnAddDigivolutionCards]);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.optional, "\"it may digivolve\" (DCGO isOptional: true)");
    assert!(
        t.outer_prompt,
        "DCGO's initial yes/no — declining it refunds the OPT (rule 15-14-1-5)"
    );
    assert_eq!(t.active_when.as_ref().and_then(|g| g.your_turn), Some(true), "[Your Turn]");
    let leaves = &t.condition.as_ref().expect("condition").all_of;
    assert!(
        leaves.iter().any(|p| p.event_host_permanent_is_source == Some(true)),
        "\"this Digimon's digivolution cards\" → host == self"
    );
    let added = leaves
        .iter()
        .find_map(|p| p.event_added_card_any.as_deref())
        .expect("event_added_card_any gate");
    assert_eq!(added.trait_has.as_deref(), Some("Three Musketeers"));
    assert!(added.kind.is_none(), "any card KIND with the trait qualifies");
    assert!(
        !leaves.iter().any(|p| p.event_caused_by_own_effect.is_some()),
        "not owner-gated (DCGO cardEffectCondition: null)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2/3 — Behavioral
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE: a [Three Musketeers]-trait card is placed under the host on your
/// turn → yes/no → hand pick limited to legal [TS]/[Three Musketeers]-text
/// Digimon → digivolve for the cost reduced by 2.
#[test]
fn bt25_005_may_digivolve_into_ts_card_in_hand_with_cost_reduced_by_2() {
    let mut r = base()
        .hand(0, &["TM-CARD", "TS-LV4", "TMTEXT-LV4", "OTHER-LV4", "TS-LV5"])
        .start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);

    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert!(r.pending_is_optional(), "the hand pick is declinable (DCGO canNoSelect)");
    let view = r.pending_selection_view().unwrap();
    let mut expected = vec![
        hand_action(&r, 0, "TS-LV4"),
        hand_action(&r, 0, "TMTEXT-LV4"),
    ];
    expected.sort();
    let mut got = view.valid_action_ids.clone();
    got.sort();
    assert_eq!(
        got, expected,
        "only [TS]-trait / [Three Musketeers]-text Digimon that can digivolve from the Lv.3 host \
         (OTHER-LV4 has neither; TS-LV5 needs a Lv.4 base)"
    );

    let before = r.memory();
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");

    assert_eq!(top_id(&r, host), "TS-LV4", "the host digivolved into the hand card");
    assert_eq!(stack_len(&r, host), 4, "Pagumon + TM-CARD + HOST + TS-LV4");
    assert_eq!(r.memory(), before - 1, "digivolution cost 3, reduced by 2 → pays 1");
}

#[test]
fn bt25_005_three_musketeers_in_text_card_is_a_legal_target() {
    let mut r = base().hand(0, &["TM-CARD", "TMTEXT-LV4"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    let before = r.memory();
    r.execute_action(0, hand_action(&r, 0, "TMTEXT-LV4")).expect("pick TMTEXT-LV4");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TMTEXT-LV4");
    assert_eq!(r.memory(), before - 1);
}

/// The reduction clamps at 0 — a cost-1 circle is paid as 0, never a gain.
#[test]
fn bt25_005_cost_reduction_clamps_at_zero() {
    let mut r = base().hand(0, &["TM-CARD", "TS-CHEAP"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    let before = r.memory();
    r.execute_action(0, hand_action(&r, 0, "TS-CHEAP")).expect("pick TS-CHEAP");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-CHEAP");
    assert_eq!(r.memory(), before, "cost 1 − 2 clamps to 0");
}

/// Declining the outer yes/no: nothing happens AND the [Once Per Turn] use is
/// refunded (rule 15-14-1-5 — the player did not choose to activate), so a
/// second placement in the same turn triggers again.
#[test]
fn bt25_005_declining_the_outer_prompt_keeps_the_opt_use() {
    let mut r = base().hand(0, &["TM-CARD", "TM-CARD", "TS-LV4"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    let before = r.memory();
    r.execute_action(0, PASS).expect("decline");
    r.auto_resolve().expect("nothing else pending");
    assert_eq!(top_id(&r, host), "HOST", "declined → no digivolve");
    assert_eq!(r.memory(), before, "declined → no cost");
    assert_eq!(r.hand_size(0), 2, "TM-CARD #2 + TS-LV4 still in hand");

    // Second placement, same turn → the effect triggers again.
    tuck(&mut r, 0, "SIB", 1, host);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "declining the activation did not consume the [Once Per Turn] use"
    );
    accept_outer(&mut r);
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-LV4");
}

/// Accepting the activation but declining the hand pick: no digivolve, no
/// cost — and the activation COUNTS (15-14-1-5), so no re-trigger this turn.
#[test]
fn bt25_005_accepting_then_declining_the_hand_pick_consumes_the_opt() {
    let mut r = base().hand(0, &["TM-CARD", "TM-CARD", "TS-LV4"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let before = r.memory();
    r.execute_action(0, PASS).expect("decline the hand pick");
    r.auto_resolve().expect("nothing else pending");
    assert_eq!(top_id(&r, host), "HOST");
    assert_eq!(r.memory(), before);

    tuck(&mut r, 0, "SIB", 1, host);
    assert!(
        r.game.pending_selection.is_none(),
        "the activation was chosen → [Once Per Turn] is used up"
    );
}

/// NEGATIVE (trait gate): the placed card lacks the [Three Musketeers] trait.
#[test]
fn bt25_005_no_prompt_when_placed_card_lacks_the_trait() {
    let mut r = base().hand(0, &["OTHER-CARD", "TS-LV4"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    assert!(r.game.pending_selection.is_none(), "no trigger");
    assert_eq!(top_id(&r, host), "HOST");
}

/// NEGATIVE (no legal target): only non-qualifying cards in hand → no dead
/// prompt (DCGO `HasMatchConditionOwnersHand` guard), no stuck selection.
#[test]
fn bt25_005_no_prompt_when_hand_has_no_legal_digivolve_target() {
    let mut r = base().hand(0, &["TM-CARD", "OTHER-LV4", "TS-LV5"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    assert!(
        r.game.pending_selection.is_none(),
        "OTHER-LV4 has neither [TS] nor the text; TS-LV5 can't digivolve from a Lv.3"
    );
    assert_eq!(top_id(&r, host), "HOST");
    assert_eq!(stack_len(&r, host), 3, "the TM card was still placed");
}

/// POSITIVE (any effect): the OPPONENT's effect placing a [Three Musketeers]
/// card under your Digimon on YOUR turn triggers (DCGO cardEffectCondition
/// is null).
#[test]
fn bt25_005_triggers_when_opponent_effect_places_tm_card_on_your_turn() {
    let mut r = base().hand(0, &["TS-LV4"]).hand(1, &["TM-CARD"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 1, "OPP-PLACER", 1, host);
    accept_outer(&mut r);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-LV4");
}

/// NEGATIVE ([Your Turn]): the same placement on the OPPONENT's turn does not
/// trigger.
#[test]
fn bt25_005_no_prompt_on_opponents_turn() {
    let mut r = base().hand(0, &["TS-LV4"]).hand(1, &["TM-CARD"]).start();
    let host = host_with_pagumon(&mut r);
    r.game.turn_player_idx = 1;
    tuck(&mut r, 1, "OPP-PLACER", 1, host);
    assert!(r.game.pending_selection.is_none(), "[Your Turn] gate");
}

/// NEGATIVE (host gate): the card is placed under a SIBLING.
#[test]
fn bt25_005_no_prompt_when_tm_card_is_placed_under_a_sibling() {
    let mut r = base().hand(0, &["TM-CARD", "TS-LV4"]).start();
    let _host = host_with_pagumon(&mut r);
    let sib = r.place_on_field(0, "SIB", Some(0));
    tuck(&mut r, 0, "PLACER", 1, sib);
    assert!(r.game.pending_selection.is_none(), "\"this Digimon's digivolution cards\" only");
}

/// NEGATIVE (inherited only): a face-up Pagumon receiving a [Three
/// Musketeers] card gains no effect.
#[test]
fn bt25_005_face_up_pagumon_does_not_trigger() {
    let mut r = base().hand(0, &["TM-CARD", "TS-LV4"]).start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let egg = r.place_on_field(0, CARD_ID, Some(0));
    tuck(&mut r, 0, "PLACER", 1, egg);
    assert!(r.game.pending_selection.is_none(), "inherited effects only work from the digivolution cards");
}

/// Rule 15-5-2 batch: two [Three Musketeers] cards tucked by ONE effect →
/// ONE prompt.
#[test]
fn bt25_005_multi_card_placement_in_one_effect_prompts_once() {
    let mut r = base().hand(0, &["TM-CARD", "TM-CARD", "TS-LV4"]).start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 2, host);
    accept_outer(&mut r);
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-LV4");
    assert!(r.game.pending_selection.is_none(), "one trigger condition → one activation");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — the hand filter honours DSL alt-path special circles
// ─────────────────────────────────────────────────────────────────────────────

const TS_ALT_ONLY: &str = r#"
card: TEST-TS-ALT
name: Test TS AltPath Lv4
kind: digimon
level: 4
color: [black]
cost: 5
dp: 5000
traits: [TS]
alt_paths:
  # The ONLY way onto a Lv.3: "Lv.3 w/[TS] trait: Cost 3" (no printed circle).
  - kind: digivolve
    from:
      all_of:
        - level_eq: 3
        - trait_has: TS
    cost: 3
"#;

/// A [TS] Lv.4 whose only digivolution circle is a DSL alt-path (trait-gated,
/// colour-free) is offered and digivolves for 3 − 2 = 1 from a GREEN [TS]
/// Lv.3 host — the filter routes through `all_digivolve_routes_for_card`,
/// exactly like the commit path (DCGO `CanPlayCardTargetFrame`).
#[test]
fn bt25_005_alt_path_only_ts_card_is_offered_and_digivolves() {
    let mut green_ts_host = digimon("HOST-TS", 3, CardColor::Green);
    green_ts_host.traits = vec!["TS".to_string()];
    let mut r = base()
        .from_dsl_yaml(TS_ALT_ONLY)
        .expect("inline YAML compiles")
        .add_card(green_ts_host)
        .hand(0, &["TM-CARD", "TEST-TS-ALT", "TS-LV4"])
        .start();
    r.game.turn_count = 1;
    r.game.turn_player_idx = 0;
    let host = r.place_stack(0, &[CARD_ID, "HOST-TS"]);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids,
        vec![hand_action(&r, 0, "TEST-TS-ALT")],
        "the alt-path-only card is legal from the green [TS] host; TS-LV4's black circle is not"
    );
    let before = r.memory();
    r.execute_action(0, hand_action(&r, 0, "TEST-TS-ALT")).expect("pick");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TEST-TS-ALT");
    assert_eq!(r.memory(), before - 1, "alt-path cost 3, reduced by 2");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — [Once Per Turn]
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bt25_005_second_placement_in_the_same_turn_is_locked_out_after_a_digivolve() {
    let mut r = base()
        .hand(0, &["TM-CARD", "TM-CARD", "TS-LV4", "TS-CHEAP"])
        .start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-LV4");

    // Pagumon is still a source of the (now Lv.4) host; a second placement
    // this turn must not prompt again.
    tuck(&mut r, 0, "SIB", 1, host);
    assert!(r.game.pending_selection.is_none(), "[Once Per Turn]");
    assert_eq!(top_id(&r, host), "TS-LV4");
}

#[test]
fn bt25_005_lockout_clears_on_your_next_turn() {
    let mut r = base()
        .hand(0, &["TM-CARD", "TM-CARD", "TS-LV4", "TS-LV5"])
        .start();
    let host = host_with_pagumon(&mut r);
    tuck(&mut r, 0, "PLACER", 1, host);
    accept_outer(&mut r);
    r.execute_action(0, hand_action(&r, 0, "TS-LV4")).expect("pick TS-LV4");
    r.auto_resolve().expect("finish");

    r.end_turn();
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    r.game.set_memory(5);
    // The host is now a Lv.4 [TS] → TS-LV5 (Lv.4 black circle) is legal.
    tuck(&mut r, 0, "SIB", 1, host);
    accept_outer(&mut r);
    let before = r.memory();
    r.execute_action(0, hand_action(&r, 0, "TS-LV5")).expect("pick TS-LV5");
    r.auto_resolve().expect("finish");
    assert_eq!(top_id(&r, host), "TS-LV5", "OPT resets on your next turn");
    assert_eq!(r.memory(), before - 2, "cost 4 − 2");
}
