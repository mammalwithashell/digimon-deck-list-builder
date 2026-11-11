//! AD1-020 Tommy, Takuya, & Zoe — Tamer, Blue/Red/Green, Cost 5.
//!
//! # Card text (card image + official Bandai DB; data/card_bundles/AD1-020.md)
//!
//! ```text
//! [Security] Play this card without paying the cost.
//! [Start of Your Main Phase] [On Play] You may place up to 2 [Hybrid] trait
//!   cards with different colors from your hand or trash under this Tamer. If
//!   this effect placed, <Draw 1>. Then, if there are 4 or more [Hybrid] trait
//!   cards under this Tamer, gain 2 memory.
//! (Rule) Name: Also treated as [Tommy Himi]/[Takuya Kanbara]/[Zoe Orimoto].
//! Inherited: [End of Your Turn] [Once Per Turn] By attacking with this Digimon
//!   with the [Hybrid] or [Ten Warriors] trait, it gains <Security A. +1> for
//!   the attack.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_020.cs
//!   - NameIsAlso(Tommy Himi, Takuya Kanbara, Zoe Orimoto).
//!   - Shared OP / SOMP: up to 2 picks (hand / trash / "do not place"), the
//!     second pick must differ in colour; AddDigivolutionCardsBottom; if placed
//!     DrawClass(1); then independently >= 4 Hybrid cards under → AddMemory(2).
//!   - Inherited EoYT (OPT, optional): carrier TopCard Hybrid / Ten Warriors and
//!     CanAttack; SelectAttackEffect.SetCanNotSelectNotAttack() (the attack is
//!     mandatory once accepted); ChangeDigimonSAttack(+1, UntilEndAttack).
//!   - Security: PlaySelfTamerSecurityEffect.
//!
//! # Official Q&A
//!   - Q6099: "with different colors" may use different combinations of a
//!     multicolour card's colours (injective card → colour assignment).
//!   - Q6100: the "Then" memory half runs even when nothing was placed.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - B1  start-of-main / on-play tamer (place, draw, threshold memory)
//! - G1  also_treated_as rule name
//! - E2  OPT + optional decline (inherited end-of-turn attack)
//! - Cost: the attack is the mandatory cost of the inherited effect
//!   (`cost_upgrade` → SecurityAttackChange that expires with the attack)
//! - B3-adjacent: [Security] play-self

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS, SECURITY_TARGET, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType},
    permanent::PermanentHandle,
    TriggerSource,
};

use crate::dsl_card_data::compiled;

const ID: &str = "AD1-020";

/// Hybrid / Ten Warriors / plain fixtures for the placement clause and the
/// inherited-attack carriers. "SECT" is a Tamer security card (no security
/// effect) so each security check simply removes one card.
fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("AD1-020 YAML");
    for (id, colors, tr) in [
        ("R", vec![CardColor::Red], "Hybrid"),
        ("B", vec![CardColor::Blue], "Hybrid"),
        ("G", vec![CardColor::Green], "Hybrid"),
        ("RB", vec![CardColor::Red, CardColor::Blue], "Hybrid"),
        ("W", vec![CardColor::Yellow], "Ten Warriors"),
        ("N", vec![CardColor::Red], "Other"),
    ] {
        let mut c = make_test_card(id, id);
        c.colors = colors;
        c.traits = vec![tr.into()];
        c.dp = Some(9000);
        b = b.add_card(c);
    }
    let mut sect = make_test_card("SECT", "SecTamer");
    sect.card_kind = CardKind::Tamer;
    sect.level = None;
    sect.dp = None;
    b = b.add_card(sect);
    b.hand(0, hand)
        .deck(0, &["N"; 20])
        .deck(1, &["N"; 20])
        .security(0, &["R", "B"])
        .memory(5)
        .start()
}

/// Same fixtures, with 5 Tamer security cards for the opponent (seat 1) so the
/// inherited attack's security-check count is observable.
fn attack_setup() -> DebugRunner {
    let mut r = setup(&[]);
    for _ in 0..5 {
        r.add_to_hand(1, "SECT");
        let c = r.game.players[1].hand.pop().unwrap();
        r.game.players[1].security.push(c);
    }
    r.game.turn_count = 3;
    r
}

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
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

/// Pick the n-th non-PASS legal action of the pending selection.
fn pick_nth(r: &mut DebugRunner, n: usize) {
    let a = r
        .pending_selection()
        .expect("a pending choice")
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .nth(n)
        .expect("legal action");
    pick(r, a);
}

fn sources(r: &DebugRunner, slot: usize) -> usize {
    r.game.players[0].battle_area[slot].card_sources.len()
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

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structure and printed identity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_020_identity_matches_print() {
    let card = compiled(ID);
    assert_eq!(card.name, "Tommy, Takuya, & Zoe");
    assert_eq!(card.cost, Some(5), "play cost 5");
    assert_eq!(card.level, None, "Tamers have no level");
    assert_eq!(card.dp, None, "Tamers have no DP");
    let colors: Vec<CompiledColor> = card.color.iter().copied().collect();
    assert_eq!(colors.len(), 3, "blue / red / green");
    for c in [
        CompiledColor::Blue,
        CompiledColor::Red,
        CompiledColor::Green,
    ] {
        assert!(colors.contains(&c), "{c:?} must be a printed colour");
    }
    assert_eq!(
        card.also_treated_as,
        ["Tommy Himi", "Takuya Kanbara", "Zoe Orimoto"]
    );
}

#[test]
fn ad1_020_has_three_triggered_clauses_with_printed_shape() {
    let clauses = triggered(ID);
    assert_eq!(clauses.len(), 3, "placement + inherited EoYT + security");

    let place = clauses
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(place.when.contains(&CompiledTiming::StartOfYourMainPhase));
    assert!(!place.optional, "the pick itself carries the 'may'");
    assert!(!place.once_per_turn);
    assert_eq!(place.scope, CompiledScope::FaceUp);

    let inherited = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::EndOfYourTurn])
        .expect("[End of Your Turn] clause");
    assert_eq!(inherited.scope, CompiledScope::Inherited);
    assert!(inherited.optional, "DCGO isOptional TRUE");
    assert!(inherited.once_per_turn, "[Once Per Turn]");

    let security = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("[Security] clause");
    assert!(!security.optional);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Main Phase] / [On Play] placement
// ═══════════════════════════════════════════════════════════════════════════

/// Both printed timings place up to 2 different-colour Hybrids (hand or trash),
/// then <Draw 1> once.
#[test]
fn ad1_020_mixed_colors_place_two_and_draw_once_at_both_timings() {
    for t in [EffectTiming::OnPlay, EffectTiming::StartOfYourMainPhase] {
        let mut r = setup(&["R", "N"]);
        r.inject_trash(0, "R");
        r.inject_trash(0, "B");
        let h = r.place_on_field(0, ID, None);
        trigger(&mut r, h, t);
        assert_eq!(
            r.pending_selection()
                .unwrap()
                .valid_action_ids
                .iter()
                .filter(|&&a| a != PASS)
                .count(),
            3,
            "hand R + trash R + trash B are the legal first picks"
        );
        pick(&mut r, 0); // the hand R
        assert!(
            !r.pending_selection()
                .unwrap()
                .valid_action_ids
                .contains(&TRASH_EFFECT_START),
            "the trash R shares the red colour and is no longer legal"
        );
        pick(&mut r, TRASH_EFFECT_START + 1); // the trash B
        assert_eq!(sources(&r, 0), 3, "top card + 2 placed Hybrids");
        assert_eq!(r.hand_size(0), 2, "-1 placed from hand, +1 <Draw 1>");
        assert_eq!(r.memory(), 5, "only 2 Hybrids under: no memory");
    }
}

/// Only up to 2 may be placed: a third is never offered.
#[test]
fn ad1_020_places_at_most_two() {
    let mut r = setup(&["R", "B", "G"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick_nth(&mut r, 0);
    pick_nth(&mut r, 0);
    assert!(r.pending_selection().is_none(), "max 2 placements");
    assert_eq!(sources(&r, 0), 3);
    assert_eq!(r.hand_size(0), 2, "1 hand card left + <Draw 1>");
}

/// Q6099: a dual-colour card can match different colour combinations, so two
/// copies of the same red/blue card are "different colors" (injective matching).
#[test]
fn ad1_020_q6099_two_copies_of_a_dual_colour_card_are_different_colors() {
    let mut r = setup(&["RB", "RB"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    pick(&mut r, 1);
    assert_eq!(r.hand_size(0), 1, "2 placed, 1 drawn");
    assert_eq!(sources(&r, 0), 3);
}

/// Same-colour Hybrids cannot both be placed: after the first pick the second
/// copy is not a legal pick.
#[test]
fn ad1_020_same_colour_second_pick_is_not_offered() {
    let mut r = setup(&["R", "R"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    // No legal second card: the selection finishes (or only PASS remains).
    if let Some(p) = r.pending_selection() {
        assert!(
            p.valid_action_ids.iter().all(|&a| a == PASS),
            "no second same-colour Hybrid may be placed"
        );
        r.execute_action(p.selecting_player, PASS).unwrap();
    }
    assert_eq!(sources(&r, 0), 2, "only 1 placed");
    assert_eq!(r.hand_size(0), 2, "1 left in hand + <Draw 1>");
}

/// Non-Hybrid / Ten Warriors cards are never candidates; with nothing legal no
/// prompt appears, nothing is drawn, no memory is gained.
#[test]
fn ad1_020_only_hybrid_trait_cards_are_candidates() {
    let mut r = setup(&["N", "W"]);
    r.inject_trash(0, "N");
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(sources(&r, 0), 1);
    assert_eq!(r.memory(), 5);
}

/// "You may": declining the placement draws nothing.
#[test]
fn ad1_020_declining_the_placement_draws_nothing() {
    let mut r = setup(&["R"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    pick(&mut r, PASS);
    assert_eq!(r.hand_size(0), 1, "nothing placed, nothing drawn");
    assert_eq!(sources(&r, 0), 1);
}

/// Placing exactly 1 still draws 1 (the draw is "if this effect placed").
#[test]
fn ad1_020_placing_one_card_draws_one() {
    let mut r = setup(&["R"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    if r.pending_selection().is_some() {
        pick(&mut r, PASS);
    }
    assert_eq!(sources(&r, 0), 2);
    assert_eq!(r.hand_size(0), 1, "-1 placed, +1 drawn");
}

/// The 4-or-more threshold counts every Hybrid under the Tamer regardless of
/// how it got there, and the memory is gained independently of the placement
/// (Q6100) — with 3 Hybrids under nothing is gained, with 4 or more it is.
#[test]
fn ad1_020_q6100_threshold_memory_is_independent_of_placement() {
    for n in [3usize, 4, 8] {
        let mut r = setup(&["B"]);
        let mut stack = vec!["R"; n];
        stack.push(ID);
        let h = r.place_stack(0, &stack);
        trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
        pick(&mut r, PASS); // place nothing
        assert_eq!(r.hand_size(0), 1, "nothing placed, so no draw");
        assert_eq!(
            r.memory(),
            if n >= 4 { 7 } else { 5 },
            "{n} Hybrids under this Tamer"
        );
    }
}

/// Placing a card can itself carry the Tamer over the threshold: 3 under + 1
/// placed = 4 → draw 1 and gain 2 memory in the same resolution.
#[test]
fn ad1_020_placement_can_reach_the_threshold() {
    let mut r = setup(&["B"]);
    let h = r.place_stack(0, &["R", "R", "R", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    if r.pending_selection().is_some() {
        pick(&mut r, PASS);
    }
    assert_eq!(r.hand_size(0), 1, "-1 placed, +1 drawn");
    assert_eq!(r.memory(), 7, "4 Hybrids under → +2 memory");
}

/// Non-Hybrid sources under the Tamer do not count toward the threshold.
#[test]
fn ad1_020_non_hybrid_sources_do_not_count_toward_four() {
    let mut r = setup(&["N"]);
    let h = r.place_stack(0, &["R", "R", "R", "W", "N", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.memory(), 5, "only 3 Hybrid-trait cards under");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Rule name
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_020_rule_names_apply_in_hand_and_as_a_source() {
    let mut r = setup(&[ID]);
    for name in ["Tommy Himi", "Takuya Kanbara", "Zoe Orimoto"] {
        assert!(r.game.players[0].hand[0].contains_card_name(name, &r.game.card_data));
    }
    r.place_stack(0, &[ID, "R"]);
    for name in ["Tommy Himi", "Takuya Kanbara", "Zoe Orimoto"] {
        assert!(r.game.players[0].battle_area[0].card_sources[0]
            .contains_card_name(name, &r.game.card_data));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [End of Your Turn] [Once Per Turn]: by attacking,
//             Security A. +1 for the attack
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: a Hybrid carrier attacks the player and checks 2 security cards
/// (1 printed + the +1 from this effect). The attack is mandatory once the
/// effect is accepted.
#[test]
fn ad1_020_inherited_attack_checks_two_security_and_attack_is_mandatory() {
    for top in ["R", "W"] {
        let mut r = attack_setup();
        let h = r.place_stack(0, &[ID, top]);
        trigger(&mut r, h, EffectTiming::EndOfYourTurn);

        // Outer activation prompt: declinable.
        assert!(r.pending_is_optional(), "the activation is a 'may'");
        pick_nth(&mut r, 0); // accept

        // "By attacking": the attack declaration is mandatory.
        assert!(
            !r.pending_is_optional(),
            "once accepted, the attack cannot be declined (DCGO SetCanNotSelectNotAttack)"
        );
        let attack_player = encode_attack(h.index as u16, SECURITY_TARGET);
        assert!(
            r.pending_selection()
                .unwrap()
                .valid_action_ids
                .contains(&attack_player),
            "the opponent's security is a legal attack target"
        );
        pick(&mut r, attack_player);
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.players[1].security.len(),
            3,
            "<Security A. +1> for the attack: 2 cards checked ({top} carrier)"
        );
    }
}

/// The +1 is "for the attack": it is gone once the attack ends.
#[test]
fn ad1_020_security_attack_bonus_expires_with_the_attack() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "R"]);
    assert_eq!(
        r.game.effective_security_strike(h),
        1,
        "baseline: one security check"
    );
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick_nth(&mut r, 0);
    pick(&mut r, encode_attack(h.index as u16, SECURITY_TARGET));
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].security.len(), 3);
    assert_eq!(
        r.game.effective_security_strike(h),
        1,
        "the +1 expired at the end of the attack"
    );
}

/// Negative: a carrier without the Hybrid / Ten Warriors trait is not offered
/// the effect.
#[test]
fn ad1_020_non_hybrid_carrier_is_not_offered_the_effect() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "N"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none(), "no activation prompt");
    assert_eq!(r.game.players[1].security.len(), 5);
    assert!(!r.game.players[0].battle_area[h.index as usize].is_suspended);
}

/// Declining the outer prompt makes no attack, spends nothing, and keeps the
/// once-per-turn use available.
#[test]
fn ad1_020_declining_activation_preserves_the_use() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "R"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(build_action_mask(&r.game, 0)[PASS as usize] > 0.5);
    pick(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[1].security.len(), 5, "no attack was made");
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_some(),
        "the declined activation is still available this turn"
    );
}

/// [Once Per Turn]: after an accepted activation (and its attack) the effect
/// cannot activate again the same turn, even once the carrier is unsuspended.
#[test]
fn ad1_020_opt_blocks_a_second_activation() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "R"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick_nth(&mut r, 0);
    pick(&mut r, encode_attack(h.index as u16, SECURITY_TARGET));
    r.auto_resolve().unwrap();
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn] is spent by the accepted activation"
    );
}

/// "By attacking" is a processing condition: when this Digimon can't attack
/// (here: suspended) the effect can't be chosen (CR 4.3 §15-7-4; DCGO
/// CanActivateCondition `CanAttack`), and the unused once-per-turn is intact.
#[test]
fn ad1_020_not_offered_when_the_carrier_cannot_attack() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "R"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(
        r.pending_selection().is_none(),
        "a suspended carrier cannot attack, so the effect isn't offered"
    );
    assert_eq!(r.game.players[1].security.len(), 5);

    // The unspent use is still there once the carrier can attack.
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_some());
}

/// Summoning sickness also stops the attack, so the effect isn't offered.
#[test]
fn ad1_020_not_offered_to_a_summoning_sick_carrier() {
    let mut r = attack_setup();
    r.game.turn_count = 3;
    let h = r.place_stack(0, &[ID, "R"]);
    r.game.players[0].battle_area[h.index as usize].turn_played = r.game.turn_count;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none());
}

/// A Digimon with the AD1-020 effect inherited attacks Digimon too: a Digimon
/// attack target is also a legal choice (printed "attacking", not "attack a
/// player"), and the same +1 applies.
#[test]
fn ad1_020_inherited_attack_may_target_a_digimon() {
    let mut r = attack_setup();
    let h = r.place_stack(0, &[ID, "R"]);
    let opp = r.place_on_field(1, "N", Some(0));
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick_nth(&mut r, 0);
    let want = encode_attack(h.index as u16, opp.index as u16);
    assert!(
        r.pending_selection()
            .unwrap()
            .valid_action_ids
            .contains(&want),
        "a suspended opponent Digimon is a legal attack target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [Security] play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_020_security_dispatch_plays_self_free() {
    let mut r = setup(&[]);
    r.game.players[0].security.clear();
    r.add_to_hand(0, ID);
    let c = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].security.push(c);
    let attacker = r.place_on_field(1, "N", Some(0));
    r.game.turn_player_idx = 1;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    let memory = r.memory();
    let _ = r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        ID
    );
    assert_eq!(r.memory(), memory, "played without paying the cost");
}
