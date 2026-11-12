//! AD1-007 Siriusmon — Digimon, Lv.6, Red, DP 12000, Cost 12.
//! Light Dragon. Form: Mega. Attribute: Vaccine.
//!
//! # Card text (card image + official Bandai DB; data/card_bundles/AD1-007.md)
//!
//! ```text
//! [Digivolve] Lv.5 w/[Gammamon] in text: Cost 3
//! <Raid> <Security A. +1> <Blocker>
//! [When Digivolving] [When Attacking] [Once Per Turn] By placing 3 Digimon
//!   cards with [Gammamon] in their texts from your hand or trash as this
//!   Digimon's top or bottom digivolution cards, delete 1 of your opponent's
//!   Digimon with as much or less DP as this Digimon.
//! [End of Your Turn] [Once Per Turn] This Digimon with 5 or more digivolution
//!   cards may attack without suspending.
//! ```
//! Standard circle: Red Lv.5 / Cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_007.cs
//!   - Shared WD / WA (SetHashString "AD1_007_WD_WA", maxCount 1, isOptional
//!     TRUE): needs 3 Gammamon-text Digimon cards across hand + trash; picks the
//!     cards, then top-or-bottom, then Destroy select over opponent Digimon with
//!     DP <= this Digimon's DP. (DCGO's delete is canNoSelect TRUE; the printed
//!     text has no "may" — modelled mandatory, see the YAML header.)
//!   - [End of Your Turn] (isOptional TRUE, "AD1_007_EoYT"): >= 5 digivolution
//!     cards → attack without suspending.
//!
//! # Official Q&A
//!   - Q6064: "[X] in its text" = name / traits / effects / inherited / (Rule) /
//!     digivolution requirement lines of the card.
//!   - Q6065: the "by placing" condition can't be met by placing just 1 (or any
//!     fewer than the 3 required).
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - H5 / H9 / H4  Blocker, Raid, Security A. +1
//! - E2  OPT shared across [When Digivolving] / [When Attacking]
//! - Cost: all-or-nothing "by placing 3" (Q6065)
//! - Selection: hand ∪ trash picks, top/bottom choice, mandatory dp<=self delete
//! - End-of-turn optional attack without suspending

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
    card_data::{CardData, EvoCost},
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlaySource},
    permanent::PermanentHandle,
    TriggerSource,
};

use crate::dsl_card_data::compiled;

const ID: &str = "AD1-007";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn builder() -> DebugRunnerBuilder {
    // Three different ways a card has [Gammamon] "in its text": name, effect
    // text, trait line.
    let gamma_name = digimon("GAMMA", "Gammamon", 3, 3000);
    let mut gamma_text = digimon("GAMTEXT", "Hopeful Rookie", 4, 4000);
    gamma_text.effect_text = "[Main] Search for a [Gammamon].".to_string();
    let mut gamma_trait = digimon("GAMTRAIT", "Odd Rookie", 4, 4000);
    gamma_trait.traits = vec!["Gammamon".to_string()];
    // A non-Digimon card with Gammamon in its name: not a Digimon card.
    let mut gamma_tamer = make_test_card("GAMTAMER", "Gammamon Fan");
    gamma_tamer.card_kind = CardKind::Tamer;
    gamma_tamer.level = None;
    gamma_tamer.dp = None;

    let mut lv5_gamma = digimon("LV5-GAM", "Gamma Base", 5, 8000);
    lv5_gamma.colors = vec![CardColor::Blue];
    lv5_gamma.effect_text = "Works with [Gammamon].".to_string();
    let mut lv5_plain = digimon("LV5-PLAIN", "Plain Base", 5, 8000);
    lv5_plain.colors = vec![CardColor::Blue];
    let lv5_red = digimon("LV5-RED", "Red Base", 5, 8000);
    let mut lv4_gamma = digimon("LV4-GAM", "Gammamon", 4, 4000);
    lv4_gamma.colors = vec![CardColor::Blue];

    let mut sect = make_test_card("SECT", "SecTamer");
    sect.card_kind = CardKind::Tamer;
    sect.level = None;
    sect.dp = None;

    DebugRunner::builder()
        .dsl_card(ID)
        .expect("AD1-007 YAML")
        .add_card(gamma_name)
        .add_card(gamma_text)
        .add_card(gamma_trait)
        .add_card(gamma_tamer)
        .add_card(lv5_gamma)
        .add_card(lv5_plain)
        .add_card(lv5_red)
        .add_card(lv4_gamma)
        .add_card(sect)
        .add_card(digimon("FILL", "Filler", 3, 3000))
        .add_card(digimon("OPP-A", "Opp A", 4, 5000))
        .add_card(digimon("OPP-B", "Opp B", 4, 12000))
        .add_card(digimon("OPP-C", "Opp C", 4, 13000))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(10)
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

fn pick_nth(r: &mut DebugRunner, n: usize) {
    let a = non_pass(r)[n];
    pick(r, a);
}

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn on_field(r: &DebugRunner, player: usize, id: &str) -> bool {
    field_ids(r, player).iter().any(|c| c == id)
}

/// The ids of the card stack of `slot`, bottom → top.
fn stack_ids(r: &DebugRunner, slot: usize) -> Vec<String> {
    r.game.players[0].battle_area[slot]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
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

/// Siriusmon on the field (not summoning sick) with `hand` / `trash` cards.
fn sirius(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().hand(0, hand).start();
    for t in trash {
        r.inject_trash(0, t);
    }
    r.game.turn_count = 3;
    let h = r.place_stack(0, &["LV5-GAM", ID]);
    (r, h)
}

/// Resolve the "by placing 3" cost with the first three legal picks and the
/// given top (0) / bottom (1) choice; leaves the delete prompt pending.
fn pay_cost(r: &mut DebugRunner, position: usize) {
    pick_nth(r, 0); // accept the outer prompt
    for _ in 0..3 {
        assert!(
            build_action_mask(&r.game, 0)[PASS as usize] < 0.5,
            "the cost is exactly 3 cards — no PASS mid-payment (Q6065)"
        );
        pick_nth(r, 0);
    }
    r.execute_branch(position).expect("top / bottom choice");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structure and printed identity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_007_identity_matches_print() {
    let card = compiled(ID);
    assert_eq!(card.name, "Siriusmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));
    assert_eq!(card.cost, Some(12));
    // Trait line = form + attribute + type (general_rule.pdf 2-3-2-1).
    let traits: Vec<String> = card.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for t in ["light dragon", "mega", "vaccine"] {
        assert!(traits.contains(&t.to_string()), "missing {t}: {traits:?}");
    }
}

#[test]
fn ad1_007_alt_paths_are_the_printed_circle_and_the_gammamon_box() {
    let card = compiled(ID);
    assert_eq!(card.alt_paths.len(), 2);
    let costs: Vec<Option<CompiledCost>> = card.alt_paths.iter().map(|p| p.cost.clone()).collect();
    assert_eq!(
        costs,
        vec![
            Some(CompiledCost::Literal(4)),
            Some(CompiledCost::Literal(3))
        ],
        "standard circle 4, [Gammamon]-in-text box 3"
    );
    for path in &card.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert!(!path.ignore_requirements);
    }
}

#[test]
fn ad1_007_clause_shape() {
    let clauses = triggered(ID);
    assert_eq!(clauses.len(), 2);

    let wd = clauses
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("[When Digivolving][When Attacking] clause");
    assert!(wd.when.contains(&CompiledTiming::WhenAttacking));
    assert!(wd.once_per_turn, "[Once Per Turn]");
    assert!(wd.optional, "DCGO isOptional TRUE ('by placing')");

    let eot = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::EndOfYourTurn])
        .expect("[End of Your Turn] clause");
    assert!(eot.once_per_turn, "[Once Per Turn]");
    assert!(eot.optional, "'may attack'");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — <Raid> <Security A. +1> <Blocker>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_007_face_up_keywords_install_on_the_carrier() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid));
    assert!(r.game.has_keyword(h, Keyword::Blocker));
    assert_eq!(
        r.game.effective_security_strike(h),
        2,
        "1 printed security check + <Security A. +1>"
    );
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

/// [Digivolve] Lv.5 w/[Gammamon] in text: Cost 3 — whatever the colour.
#[test]
fn ad1_007_box_accepts_a_lv5_with_gammamon_in_its_text_for_3() {
    let (mut r, slot) = digivolve_over("LV5-GAM");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// Standard circle: Red Lv.5 / Cost 4.
#[test]
fn ad1_007_standard_circle_red_lv5_costs_4() {
    let (mut r, slot) = digivolve_over("LV5-RED");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 4);
}

/// Negative: an off-colour Lv.5 with no [Gammamon] in its text matches no circle.
#[test]
fn ad1_007_rejects_an_off_colour_lv5_without_gammamon() {
    let (mut r, slot) = digivolve_over("LV5-PLAIN");
    assert!(!offered(&r, slot));
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

/// Negative: the box is level 5 — a Lv.4 [Gammamon] is not a legal base.
#[test]
fn ad1_007_box_is_level_5_only() {
    let (mut r, slot) = digivolve_over("LV4-GAM");
    assert!(!offered(&r, slot));
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] / [When Attacking] [Once Per Turn]
// ═══════════════════════════════════════════════════════════════════════════

/// By placing 3 Gammamon-text Digimon cards as BOTTOM digivolution cards,
/// delete a Digimon with DP <= this Digimon's (12000). Both timings.
#[test]
fn ad1_007_places_three_on_the_bottom_then_deletes() {
    for t in [EffectTiming::WhenDigivolving, EffectTiming::WhenAttacking] {
        let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT"], &[]);
        r.place_on_field(1, "OPP-A", Some(0));
        r.place_on_field(1, "OPP-C", Some(0));
        trigger(&mut r, h, t);

        assert!(r.pending_is_optional(), "outer 'by placing' prompt");
        pay_cost(&mut r, 1);

        // The delete is mandatory once the cost is paid.
        assert!(!r.pending_is_optional(), "{t:?}: the delete is not a 'may'");
        assert_eq!(non_pass(&r).len(), 1, "only the 5000 DP Digimon (<= 12000)");
        pick_nth(&mut r, 0);
        r.auto_resolve().unwrap();

        assert_eq!(r.hand_size(0), 0, "all 3 cards left the hand");
        let stack = stack_ids(&r, h.index as usize);
        assert_eq!(stack.len(), 5, "3 placed + base + top");
        assert_eq!(
            &stack[3..],
            ["LV5-GAM", ID],
            "bottom placement keeps the old stack above"
        );
        let mut placed: Vec<&str> = stack[..3].iter().map(|s| s.as_str()).collect();
        placed.sort();
        assert_eq!(placed, ["GAMMA", "GAMTEXT", "GAMTRAIT"]);
        assert!(!on_field(&r, 1, "OPP-A"), "{t:?}: deleted");
        assert!(on_field(&r, 1, "OPP-C"), "13000 DP is above 12000");
    }
}

/// TOP placement: the three cards go directly under the top card, above the
/// old digivolution stack.
#[test]
fn ad1_007_places_three_on_top_directly_under_the_top_card() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT"], &[]);
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pay_cost(&mut r, 0);
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    let stack = stack_ids(&r, h.index as usize);
    assert_eq!(stack.len(), 5);
    assert_eq!(stack[0], "LV5-GAM", "the old stack stays at the bottom");
    assert_eq!(stack[4], ID, "Siriusmon stays the top card");
    let mut placed: Vec<&str> = stack[1..4].iter().map(|s| s.as_str()).collect();
    placed.sort();
    assert_eq!(placed, ["GAMMA", "GAMTEXT", "GAMTRAIT"]);
    assert!(!on_field(&r, 1, "OPP-A"));
}

/// The 3 cards may come from the hand AND the trash, mixed.
#[test]
fn ad1_007_cards_may_come_from_hand_and_trash_mixed() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT"], &["GAMTRAIT"]);
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pick_nth(&mut r, 0); // accept
    assert_eq!(non_pass(&r).len(), 3, "2 hand + 1 trash candidates");
    for _ in 0..3 {
        pick_nth(&mut r, 0);
    }
    r.execute_branch(1).unwrap();
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(stack_ids(&r, h.index as usize).len(), 5);
}

/// Q6064: "[Gammamon] in its text" is satisfied by the NAME, the EFFECT TEXT or
/// the TRAIT line; a non-Digimon card (Tamer) with Gammamon in its name and a
/// Digimon without Gammamon anywhere are not candidates.
#[test]
fn ad1_007_q6064_only_digimon_cards_with_gammamon_in_their_text_count() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT", "GAMTAMER", "FILL"], &[]);
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pick_nth(&mut r, 0); // accept
    assert_eq!(
        non_pass(&r).len(),
        3,
        "name / effect text / trait line match; the Tamer and the filler do not"
    );
}

/// Q6065: fewer than 3 qualifying cards — the "by placing 3" condition can't be
/// met, so the effect isn't offered at all.
#[test]
fn ad1_007_q6065_two_qualifying_cards_are_not_enough() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "FILL"], &[]);
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(
        r.pending_selection().is_none(),
        "no prompt: only 2 candidates"
    );
    assert_eq!(r.hand_size(0), 3);
    assert!(on_field(&r, 1, "OPP-A"));
}

/// Q6065 across zones: 2 in hand + 1 in trash IS enough, 1 + 1 is not.
#[test]
fn ad1_007_the_count_spans_hand_and_trash_together() {
    let (mut r, h) = sirius(&["GAMMA"], &["GAMTEXT"]);
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_selection().is_none(), "1 + 1 = 2 candidates");
}

/// Declining the outer prompt places nothing, deletes nothing and keeps the
/// once-per-turn use.
#[test]
fn ad1_007_declining_preserves_the_use() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT"], &[]);
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pick(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 3);
    assert!(on_field(&r, 1, "OPP-A"));
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    assert!(
        r.pending_selection().is_some(),
        "the declined activation is still available this turn"
    );
}

/// [Once Per Turn] is shared by [When Digivolving] and [When Attacking].
#[test]
fn ad1_007_opt_is_shared_between_the_two_timings() {
    let (mut r, h) = sirius(
        &[
            "GAMMA", "GAMTEXT", "GAMTRAIT", "GAMMA", "GAMTEXT", "GAMTRAIT",
        ],
        &[],
    );
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pay_cost(&mut r, 1);
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 3, "3 cards still in hand");
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    assert!(
        r.pending_selection().is_none(),
        "the [When Attacking] use was spent by [When Digivolving]"
    );
}

/// The delete reads THIS Digimon's DP: 12000 is deletable (as much or less),
/// 13000 is not; with only a 13000 DP Digimon the cost is paid and the delete
/// simply has no target.
#[test]
fn ad1_007_delete_boundary_is_this_digimons_dp() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT"], &[]);
    r.place_on_field(1, "OPP-B", Some(0));
    r.place_on_field(1, "OPP-C", Some(0));
    trigger(&mut r, h, EffectTiming::WhenAttacking);
    pay_cost(&mut r, 1);
    assert_eq!(non_pass(&r).len(), 1, "12000 DP qualifies, 13000 does not");
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(!on_field(&r, 1, "OPP-B"));
    assert!(on_field(&r, 1, "OPP-C"));
}

#[test]
fn ad1_007_no_legal_delete_target_still_pays_the_cost() {
    let (mut r, h) = sirius(&["GAMMA", "GAMTEXT", "GAMTRAIT"], &[]);
    r.place_on_field(1, "OPP-C", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    pay_cost(&mut r, 1);
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 1, "OPP-C"), "13000 DP is above this Digimon");
    assert_eq!(r.hand_size(0), 0, "the 3 cards were placed regardless");
    assert_eq!(stack_ids(&r, h.index as usize).len(), 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [End of Your Turn] [Once Per Turn]
// ═══════════════════════════════════════════════════════════════════════════

fn with_sources(n: usize) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().start();
    for _ in 0..5 {
        r.add_to_hand(1, "SECT");
        let c = r.game.players[1].hand.pop().unwrap();
        r.game.players[1].security.push(c);
    }
    r.game.turn_count = 3;
    let mut stack = vec!["FILL"; n];
    stack.push(ID);
    let h = r.place_stack(0, &stack);
    (r, h)
}

/// 5 digivolution cards: the optional attack without suspending is offered,
/// and the Digimon stays unsuspended after attacking.
#[test]
fn ad1_007_five_digivolution_cards_may_attack_without_suspending() {
    let (mut r, h) = with_sources(5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_is_optional(), "outer 'may' prompt");
    pick_nth(&mut r, 0); // accept
    assert!(r.pending_is_optional(), "the attack itself may be declined");
    let attack_player = encode_attack(h.index as u16, SECURITY_TARGET);
    assert!(non_pass(&r).contains(&attack_player));
    pick(&mut r, attack_player);
    r.auto_resolve().unwrap();
    assert!(
        r.game.players[1].security.len() < 5,
        "the attack checked security"
    );
    assert!(
        !r.game.players[0].battle_area[h.index as usize].is_suspended,
        "attacked without suspending"
    );
}

/// 4 digivolution cards are not enough.
#[test]
fn ad1_007_four_digivolution_cards_is_not_enough() {
    let (mut r, h) = with_sources(4);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].security.len(), 5);
}

/// More than 5 also qualifies ("5 or more").
#[test]
fn ad1_007_six_digivolution_cards_also_qualifies() {
    let (mut r, h) = with_sources(6);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_some());
}

/// Declining the outer prompt keeps the use; accepting and then declining the
/// attack spends it ([Once Per Turn] counts the activation).
#[test]
fn ad1_007_opt_decline_semantics() {
    let (mut r, h) = with_sources(5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick(&mut r, PASS); // decline activation
    assert_eq!(r.game.players[1].security.len(), 5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_some(),
        "declined activation preserved the use"
    );
    pick_nth(&mut r, 0); // accept
    pick(&mut r, PASS); // decline the attack
    assert_eq!(r.game.players[1].security.len(), 5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "accepting the activation spent the once-per-turn use"
    );
}

/// [Once Per Turn]: a completed attack locks out a second activation.
#[test]
fn ad1_007_opt_blocks_a_second_end_of_turn_attack() {
    let (mut r, h) = with_sources(5);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick_nth(&mut r, 0);
    pick(&mut r, encode_attack(h.index as u16, SECURITY_TARGET));
    r.auto_resolve().unwrap();
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none());
}
