//! AD1-008 Gallantmon — Digimon, Lv.6, Red, DP 12000, Cost 12.
//! Holy Warrior / Royal Knight / Hero. Form: Mega. Attribute: Virus.
//!
//! # Card text (card image + official Bandai DB; data/card_bundles/AD1-008.md)
//!
//! ```text
//! [Digivolve] Lv.5 w/[WarGrowlmon] in name or w/[Hero] trait: Cost 3
//! <Rush> <Raid> <Piercing>
//! [When Digivolving] Delete up to 10000 DP total worth of your opponent's
//!   Digimon. Then, this Digimon may attack.
//! [When Digivolving] [When Attacking] [Once Per Turn] Delete 1 of your
//!   opponent's lowest DP Digimon.
//! [Your Turn] Your opponent's effects don't affect this Digimon with [Takato
//!   Matsuki] in its digivolution cards and it gets +5000 DP.
//! ```
//! Standard circle: Red Lv.5 / Cost 4.
//!
//! cards.json is wrong in two places (the image / official DB / DCGO win): the
//! [Digivolve] box is Lv.5 (not Lv.4), and "[Takato Matsuki]" is a NAME
//! reference (not a trait).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_008.cs
//!
//! # Official Q&A
//!   - Q6066: the third digivolution requirement (name / trait box) digivolves
//!     for cost 3, ignoring nothing else.
//!   - Q6067-Q6072: "effects don't affect" semantics (the card can still be
//!     chosen; effects on it simply don't apply).
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - H1/H9/H3  Rush / Raid / Piercing keyword grants
//! - Selection: running-DP-sum budget multi-select (min 1, PASS after one)
//! - Selection: selector lowest_dp (ties stay a player choice)
//! - E3  OPT shared across [When Digivolving] / [When Attacking]
//! - D1/F6  [Your Turn] conditional +5000 DP and opponent-effect immunity
//! - "Then, this Digimon may attack" after (and independent of) the delete

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, encode_digivolve, PASS, SECURITY_TARGET},
    },
    card_data::CardData,
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    effect_context::EffectContext,
    enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, GamePhase, Keyword, PlaySource},
    permanent::PermanentHandle,
    selection::SelectionKind,
    TriggerSource,
};

use crate::dsl_card_data::compiled;

const ID: &str = "AD1-008";

/// Effect slots in the YAML order: 3 keywords (0..=2), then the
/// [When Digivolving] delete-budget clause (3), then the shared
/// [When Digivolving]/[When Attacking] lowest-DP clause (4 = WD, 5 = WA).
/// A structural test below pins this order so a YAML reshuffle fails loudly.
const SLOT_WD_BUDGET: usize = 3;
const SLOT_LOWEST_WD: usize = 4;
const SLOT_LOWEST_WA: usize = 5;

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("AD1-008 YAML");
    // Opponent Digimon of several DPs: id "D<dp>", plus tie twins "A3K"/"B3K".
    for dp in [3000, 4000, 6000, 7000, 10000, 11000] {
        b = b.add_card(digimon(&format!("D{dp}"), &format!("Digimon {dp}"), 4, dp));
    }
    b = b
        .add_card(digimon("A3K", "Twin A", 4, 3000))
        .add_card(digimon("B3K", "Twin B", 4, 3000));

    let mut wgrow5 = digimon("WGROW5", "WarGrowlmon", 5, 7000);
    wgrow5.colors = vec![CardColor::Blue];
    let mut wgrow4 = digimon("WGROW4", "WarGrowlmon", 4, 5000);
    wgrow4.colors = vec![CardColor::Blue];
    let mut hero5 = digimon("HERO5", "Some Hero", 5, 7000);
    hero5.colors = vec![CardColor::Blue];
    hero5.traits = vec!["Hero".to_string()];
    let mut plain5 = digimon("PLAIN5", "Plain Five", 5, 7000);
    plain5.colors = vec![CardColor::Blue];
    let red5 = digimon("RED5", "Red Five", 5, 7000);
    // Takato Matsuki by NAME, and a card that only carries the words as a trait.
    let mut trait_only = digimon("TAKATO-TRAIT", "Not Takato", 3, 1000);
    trait_only.traits = vec!["Takato Matsuki".to_string()];

    let mut sect = tamer("SECT", "SecTamer");
    sect.colors = vec![CardColor::Red];

    b.add_card(wgrow5)
        .add_card(wgrow4)
        .add_card(hero5)
        .add_card(plain5)
        .add_card(red5)
        .add_card(tamer("TAKATO", "Takato Matsuki"))
        .add_card(trait_only)
        .add_card(tamer("OPP-TAMER", "Opp Tamer"))
        .add_card(sect)
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(10)
}

/// Gallantmon on the field (not summoning sick) facing opponent Digimon of the
/// given card ids, with 5 Tamer security cards for the opponent.
fn board(opp: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().start();
    r.game.turn_count = 3;
    let h = r.place_on_field(0, ID, Some(0));
    for id in opp {
        r.place_on_field(1, id, Some(0));
    }
    for _ in 0..5 {
        r.add_to_hand(1, "SECT");
        let c = r.game.players[1].hand.pop().unwrap();
        r.game.players[1].security.push(c);
    }
    (r, h)
}

fn pick(r: &mut DebugRunner, a: u16) {
    let p = r
        .pending_selection()
        .expect("a pending choice")
        .selecting_player;
    assert!(
        build_action_mask(&r.game, p)[a as usize] > 0.5,
        "action {a} must be legal in the mask"
    );
    r.execute_action(p, a).unwrap();
}

fn non_pass(r: &DebugRunner) -> Vec<u16> {
    r.pending_selection()
        .expect("a pending choice")
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect()
}

fn pass_legal(r: &DebugRunner) -> bool {
    build_action_mask(&r.game, 0)[PASS as usize] > 0.5
}

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

/// If the simultaneous triggers ask for an order, resolve the effect from
/// effect slot `first` first.
fn order_first(r: &mut DebugRunner, first: usize) {
    let Some(view) = r.pending_selection_view() else {
        return;
    };
    if view.kind != SelectionKind::TriggerOrder {
        return;
    }
    let want = format!("{ID} slot {first} ");
    let action = view
        .effect_choices
        .as_ref()
        .and_then(|cs| cs.iter().find(|c| c.label.starts_with(&want)))
        .map(|c| c.action_id)
        .unwrap_or_else(|| {
            panic!(
                "no trigger-order entry for slot {first}: {:?}",
                view.effect_choices
            )
        });
    pick(r, action);
}

fn kind_of(r: &DebugRunner) -> Option<SelectionKind> {
    r.pending_kind()
}

fn opp_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn triggered(id: &str) -> Vec<CompiledTriggeredClause> {
    compiled(id)
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// The opponent's slot-`i` Digimon as an attack-style selection action.
fn opp_slot(i: u16) -> u16 {
    encode_attack(0, i)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structure and printed identity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_008_identity_matches_print() {
    let card = compiled(ID);
    assert_eq!(card.name, "Gallantmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    let traits: Vec<String> = card.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for t in ["holy warrior", "royal knight", "hero", "mega", "virus"] {
        assert!(traits.contains(&t.to_string()), "missing {t}: {traits:?}");
    }
}

#[test]
fn ad1_008_alt_paths_are_the_printed_circle_and_the_box() {
    let card = compiled(ID);
    let costs: Vec<Option<CompiledCost>> = card.alt_paths.iter().map(|p| p.cost.clone()).collect();
    assert_eq!(
        costs,
        vec![
            Some(CompiledCost::Literal(4)),
            Some(CompiledCost::Literal(3))
        ]
    );
    for path in &card.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert!(!path.ignore_requirements);
    }
}

/// Pins the effect order the slot constants above rely on, and the printed
/// optionality / once-per-turn flags.
#[test]
fn ad1_008_clause_order_and_flags() {
    let card = compiled(ID);
    for i in 0..3 {
        assert!(
            matches!(
                card.effects[i],
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            ),
            "effects[{i}] is a keyword"
        );
    }
    let CompiledClause::Triggered(budget) = &card.effects[SLOT_WD_BUDGET] else {
        panic!("effects[{SLOT_WD_BUDGET}] is the delete-budget clause");
    };
    assert_eq!(budget.when, vec![CompiledTiming::WhenDigivolving]);
    assert!(!budget.once_per_turn && !budget.optional);

    let CompiledClause::Triggered(lowest) = &card.effects[SLOT_LOWEST_WD] else {
        panic!("effects[{SLOT_LOWEST_WD}] is the lowest-DP clause");
    };
    assert!(lowest.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(lowest.when.contains(&CompiledTiming::WhenAttacking));
    assert!(lowest.once_per_turn, "[Once Per Turn]");
    assert!(!lowest.optional, "'Delete 1' is not a 'may'");

    let auras = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    assert_eq!(auras, 2, "immunity + +5000 DP");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Keywords
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_008_face_up_keywords_install_on_the_carrier() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    for k in [Keyword::Rush, Keyword::Raid, Keyword::Piercing] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Digivolve routes
// ═══════════════════════════════════════════════════════════════════════════

fn digivolve_over(base: &str) -> (DebugRunner, usize) {
    let mut r = builder().hand(0, &[ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, base, Some(0)).index as usize;
    (r, slot)
}

fn offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.5
}

/// [Digivolve] Lv.5 w/[WarGrowlmon] in name: cost 3, whatever the colour.
#[test]
fn ad1_008_box_accepts_a_wargrowlmon_named_lv5_for_3() {
    let (mut r, slot) = digivolve_over("WGROW5");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// [Digivolve] Lv.5 w/[Hero] trait: cost 3, whatever the colour (Q6066).
#[test]
fn ad1_008_box_accepts_a_hero_trait_lv5_for_3() {
    let (mut r, slot) = digivolve_over("HERO5");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// Standard circle: Red Lv.5 / Cost 4.
#[test]
fn ad1_008_standard_circle_red_lv5_costs_4() {
    let (mut r, slot) = digivolve_over("RED5");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 4);
}

#[test]
fn ad1_008_rejects_an_off_colour_lv5_without_the_name_or_trait() {
    let (mut r, slot) = digivolve_over("PLAIN5");
    assert!(!offered(&r, slot));
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

/// The box is Lv.5 on the printed card (cards.json's "Lv.4" is wrong): a Lv.4
/// [WarGrowlmon] is not a legal base.
#[test]
fn ad1_008_box_is_level_5_not_level_4() {
    let (mut r, slot) = digivolve_over("WGROW4");
    assert!(!offered(&r, slot));
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] delete up to 10000 DP total; then may attack
// ═══════════════════════════════════════════════════════════════════════════

/// Choose several targets within the running 10000 DP budget: 6000 + 4000
/// leaves nothing more affordable. The deletion happens, and then the optional
/// attack is offered.
#[test]
fn ad1_008_wd_deletes_chosen_digimon_within_the_running_budget() {
    let (mut r, h) = board(&["D3000", "D4000", "D6000", "D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);

    assert!(matches!(
        kind_of(&r),
        Some(SelectionKind::DpBudget {
            remaining_dp: 10000,
            picked: 0
        })
    ));
    assert_eq!(
        non_pass(&r),
        vec![opp_slot(0), opp_slot(1), opp_slot(2)],
        "the 11000 DP Digimon exceeds the whole budget"
    );
    assert!(!pass_legal(&r), "at least 1 pick is required (mandatory)");

    pick(&mut r, opp_slot(2)); // 6000
    assert!(
        matches!(
            kind_of(&r),
            Some(SelectionKind::DpBudget {
                remaining_dp: 4000,
                picked: 1
            })
        ),
        "{:?}",
        kind_of(&r)
    );
    pick(&mut r, opp_slot(1)); // 4000 → budget exhausted

    // "Then, this Digimon may attack."
    assert_eq!(kind_of(&r), Some(SelectionKind::Target));
    assert!(r.pending_is_optional(), "'may attack' is declinable");
    assert_eq!(
        opp_ids(&r),
        ["D3000", "D11000"],
        "6000 + 4000 were deleted; 3000 fits no remaining budget"
    );
}

/// A pick that would exceed the budget is not offered: after the 7000 pick only
/// 3000 DP remains, so the 4000 DP Digimon is excluded but the 3000 is not.
#[test]
fn ad1_008_wd_a_pick_that_exceeds_the_budget_is_not_offered() {
    let (mut r, h) = board(&["D7000", "D4000", "D3000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    pick(&mut r, opp_slot(0)); // 7000
    assert_eq!(
        non_pass(&r),
        vec![opp_slot(2)],
        "only the 3000 DP Digimon fits"
    );
    assert!(pass_legal(&r), "after one pick the player may stop");
}

/// Boundary: exactly 10000 DP is within the budget; 11000 is not.
#[test]
fn ad1_008_wd_ten_thousand_dp_is_the_boundary() {
    let (mut r, h) = board(&["D10000", "D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    assert_eq!(non_pass(&r), vec![opp_slot(0)]);
    pick(&mut r, opp_slot(0));
    assert_eq!(opp_ids(&r), ["D11000"]);
}

/// "Up to": after the first pick the player may stop early.
#[test]
fn ad1_008_wd_may_stop_after_the_first_pick() {
    let (mut r, h) = board(&["D3000", "D4000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    pick(&mut r, opp_slot(1)); // 4000
    assert!(pass_legal(&r));
    pick(&mut r, PASS);
    assert_eq!(
        opp_ids(&r),
        ["D3000"],
        "only the picked Digimon was deleted"
    );
    assert_eq!(
        kind_of(&r),
        Some(SelectionKind::Target),
        "then the attack prompt"
    );
}

/// With no affordable target the delete is skipped, but "Then, this Digimon
/// may attack" is still offered (the halves are independent).
#[test]
fn ad1_008_wd_attack_is_still_offered_when_nothing_can_be_deleted() {
    let (mut r, h) = board(&["D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    assert_eq!(
        kind_of(&r),
        Some(SelectionKind::Target),
        "straight to the attack"
    );
    assert!(r.pending_is_optional());
    assert!(non_pass(&r).contains(&encode_attack(h.index as u16, SECURITY_TARGET)));
    assert_eq!(opp_ids(&r), ["D11000"]);
}

/// The attack is optional: PASS makes no attack.
#[test]
fn ad1_008_wd_the_attack_may_be_declined() {
    let (mut r, h) = board(&["D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    pick(&mut r, PASS);
    assert_eq!(r.game.players[1].security.len(), 5, "no attack was made");
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

/// Accepting the attack runs it: the opponent's security is checked.
#[test]
fn ad1_008_wd_the_attack_checks_security() {
    let (mut r, h) = board(&["D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_WD_BUDGET);
    pick(&mut r, encode_attack(h.index as u16, SECURITY_TARGET));
    r.auto_resolve().unwrap();
    assert!(r.game.players[1].security.len() < 5);
}

/// Integration through a real digivolve: the [When Digivolving] clause runs when
/// Gallantmon digivolves in from the hand.
#[test]
fn ad1_008_when_digivolving_runs_through_a_real_digivolve() {
    let mut r = builder().hand(0, &[ID]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "RED5", Some(0)).index as usize;
    r.place_on_field(1, "D3000", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    r.game.drain_effect_queue();
    order_first(&mut r, SLOT_WD_BUDGET);
    assert!(matches!(kind_of(&r), Some(SelectionKind::DpBudget { .. })));
    pick(&mut r, opp_slot(0));
    assert!(opp_ids(&r).is_empty(), "the 3000 DP Digimon was deleted");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [When Digivolving] [When Attacking] [Once Per Turn]: delete 1 of
//             the opponent's lowest DP Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// Only the minimum-DP Digimon are offered; ties stay a player choice; the
/// delete is mandatory.
#[test]
fn ad1_008_lowest_dp_pick_is_restricted_to_the_minimum_and_ties_are_a_choice() {
    let (mut r, h) = board(&["A3K", "B3K", "D6000"]);
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    order_first(&mut r, SLOT_LOWEST_WA);
    assert_eq!(kind_of(&r), Some(SelectionKind::OppField));
    assert!(!r.pending_is_optional(), "'Delete 1' is mandatory");
    assert_eq!(
        non_pass(&r),
        vec![opp_slot(0), opp_slot(1)],
        "the two 3000 DP twins; the 6000 DP Digimon is not the lowest"
    );
    pick(&mut r, opp_slot(1));
    assert_eq!(opp_ids(&r), ["A3K", "D6000"], "the chosen twin was deleted");
}

/// The "lowest" is measured over the opponent's Digimon only (a Tamer has no
/// DP); with one Digimon it is the only legal pick.
#[test]
fn ad1_008_lowest_dp_ignores_tamers() {
    let (mut r, h) = board(&["OPP-TAMER", "D6000"]);
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    order_first(&mut r, SLOT_LOWEST_WA);
    assert_eq!(non_pass(&r), vec![opp_slot(1)]);
}

/// With no opponent Digimon there is no prompt.
#[test]
fn ad1_008_lowest_dp_without_opponent_digimon_is_a_no_op() {
    let (mut r, h) = board(&["OPP-TAMER"]);
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    order_first(&mut r, SLOT_LOWEST_WA);
    assert!(r.pending_selection().is_none());
    assert_eq!(opp_ids(&r), ["OPP-TAMER"]);
}

/// The [When Digivolving] instance deletes the lowest as well.
#[test]
fn ad1_008_lowest_dp_also_fires_when_digivolving() {
    let (mut r, h) = board(&["D6000", "D11000"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    order_first(&mut r, SLOT_LOWEST_WD);
    assert_eq!(kind_of(&r), Some(SelectionKind::OppField));
    pick(&mut r, opp_slot(0));
    assert_eq!(opp_ids(&r), ["D11000"]);
}

/// [Once Per Turn] is shared between [When Digivolving] and [When Attacking]:
/// after the [When Attacking] use, a [When Digivolving] trigger only runs the
/// budget clause (no lowest-DP prompt), and a second [When Attacking] does not
/// activate it again.
#[test]
fn ad1_008_opt_is_shared_between_the_two_timings() {
    let (mut r, h) = board(&["D3000", "D4000", "D6000"]);
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    order_first(&mut r, SLOT_LOWEST_WA);
    pick(&mut r, opp_slot(0)); // delete the lowest (3000)
    assert!(r.pending_selection().is_none());
    assert_eq!(opp_ids(&r), ["D4000", "D6000"]);

    // Only the budget clause remains for [When Digivolving].
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(
        matches!(kind_of(&r), Some(SelectionKind::DpBudget { .. })),
        "the lowest-DP use was spent by [When Attacking]: {:?}",
        kind_of(&r)
    );

    // And a second [When Attacking] is locked out.
    let (mut r2, h2) = board(&["D3000", "D4000"]);
    trigger(&mut r2, h2, EffectTiming::WhenAttacking);
    order_first(&mut r2, SLOT_LOWEST_WA);
    pick(&mut r2, opp_slot(0));
    trigger(&mut r2, h2, EffectTiming::WhenAttacking);
    order_first(&mut r2, SLOT_LOWEST_WA);
    assert!(
        r2.pending_selection().is_none(),
        "no second lowest-DP delete this turn"
    );
    assert_eq!(opp_ids(&r2), ["D4000"]);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — [Your Turn] opponent's effects don't affect it (Takato Matsuki
//             under) and it gets +5000 DP
// ═══════════════════════════════════════════════════════════════════════════

fn dp(r: &mut DebugRunner, h: PermanentHandle) -> i32 {
    r.game.tick_declarative_effects();
    r.effective_dp(h).expect("dp")
}

fn immune(r: &mut DebugRunner, h: PermanentHandle) -> bool {
    r.game.tick_declarative_effects();
    r.game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon)
}

/// With [Takato Matsuki] under it on your turn: +5000 DP and immune to the
/// opponent's effects (but not to its own controller's).
#[test]
fn ad1_008_takato_under_on_your_turn_gives_plus_5000_and_immunity() {
    let mut r = builder().start();
    let h = r.place_stack(0, &["TAKATO", ID]);
    assert_eq!(dp(&mut r, h), 17000, "12000 + 5000");
    assert!(immune(&mut r, h));
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(h, 0, EffectSourceKind::Digimon),
        "the immunity is opponent-scoped only"
    );
}

/// End to end: the opponent's deletion effect fizzles against it.
#[test]
fn ad1_008_an_opponent_delete_effect_fizzles_while_immune() {
    let mut r = builder().start();
    let h = r.place_stack(0, &["TAKATO", ID]);
    let opp = r.place_on_field(1, "D3000", Some(0));
    r.game.tick_declarative_effects();
    let opp_card = r.top_card(opp);
    {
        let mut ctx = EffectContext::new(&mut r.game, opp_card, Some(opp), 1);
        ctx.delete_permanent(h);
    }
    r.game.drain_effect_queue();
    assert_eq!(r.battle_area_size(0), 1, "the deletion did not affect it");
}

/// Without [Takato Matsuki] under it: no bonus, no immunity.
#[test]
fn ad1_008_no_takato_no_bonus_and_no_immunity() {
    let mut r = builder().start();
    let h = r.place_stack(0, &["FILL", ID]);
    assert_eq!(dp(&mut r, h), 12000);
    assert!(!immune(&mut r, h));
}

/// The reference is the NAME: a card that merely has the words as a trait does
/// not count (cards.json's "trait" wording is wrong).
#[test]
fn ad1_008_a_takato_matsuki_trait_alone_does_not_count() {
    let mut r = builder().start();
    let h = r.place_stack(0, &["TAKATO-TRAIT", ID]);
    assert_eq!(dp(&mut r, h), 12000);
    assert!(!immune(&mut r, h));
}

/// [Your Turn]: on the opponent's turn neither the bonus nor the immunity
/// applies.
#[test]
fn ad1_008_nothing_applies_on_the_opponents_turn() {
    let mut r = builder().start();
    let h = r.place_stack(0, &["TAKATO", ID]);
    r.game.turn_player_idx = 1;
    assert_eq!(dp(&mut r, h), 12000);
    assert!(!immune(&mut r, h));
}

/// Takato must be among the digivolution cards (under), not the top card: a
/// Takato in hand does nothing.
#[test]
fn ad1_008_takato_in_hand_does_nothing() {
    let mut r = builder().hand(0, &["TAKATO"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    assert_eq!(dp(&mut r, h), 12000);
    assert!(!immune(&mut r, h));
}
