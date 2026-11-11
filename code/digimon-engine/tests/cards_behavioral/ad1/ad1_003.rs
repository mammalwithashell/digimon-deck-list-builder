//! AD1-003 WarGrowlmon — Digimon, Lv.5, Red, DP 7000, Cost 7.
//! Cyborg / Hero. Form: Ultimate. Attribute: Virus.
//!
//! # Card text (card image + official Bandai DB; data/card_bundles/AD1-003.md)
//!
//! ```text
//! [Digivolve] Lv.4 w/[Growlmon] in name or w/[Hero] trait: Cost 3
//! <Raid>
//! [On Play] [When Digivolving] You may play 1 [Takato Matsuki] from your hand
//!   or trash without paying the cost. Then, you may delete 1 of your
//!   opponent's Digimon with 6000 DP or less.
//! Inherited: [All Turns] When this Digimon with [Gallantmon] in its name would
//!   leave the battle area other than by your effects, you may play 1 each of
//!   [Takato Matsuki] and [Guilmon] from its digivolution cards without paying
//!   the cost.
//! ```
//! Standard circle: Red Lv.4 / Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_003.cs
//!
//! # Official Q&A (data/card_qa.json)
//!   - Q6053: with Takato Matsuki AND Guilmon in the digivolution cards, the
//!     inherited effect must play both ("whenever possible") — never just one.
//!   - Q6054: with only Takato Matsuki, just Takato is played.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - H9  Raid keyword grant
//! - A5/C3-adjacent: union-zone free play of a named Tamer (hand OR trash)
//! - E1  two independent optional halves in one clause
//! - F3  non-cancelling would-leave replacement (inherited), own-effect exclusion
//! - G2-adjacent: alt-path digivolve boxes (name / trait / level gates)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_digivolve, PASS, TRASH_EFFECT_START},
    },
    card_data::{CardData, EvoCost},
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlaySource},
    permanent::PermanentHandle,
    replacement::ReplacementCause,
    TriggerSource,
};

use crate::dsl_card_data::compiled;

const ID: &str = "AD1-003";

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
    let mut hero4 = digimon("HERO4", "Some Hero", 4, 4000);
    hero4.colors = vec![CardColor::Blue];
    hero4.traits = vec!["Hero".to_string()];
    let mut plain4 = digimon("PLAIN4", "Plain Four", 4, 4000);
    plain4.colors = vec![CardColor::Blue];
    let mut growlmon4 = digimon("GROWL4", "Growlmon", 4, 4000);
    growlmon4.colors = vec![CardColor::Blue];
    let mut growl3 = digimon("GROWL3", "Growlmon", 3, 3000);
    growl3.colors = vec![CardColor::Blue];
    let mut red4 = digimon("RED4", "Red Four", 4, 4000);
    red4.colors = vec![CardColor::Red];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("AD1-003 YAML")
        .add_card(hero4)
        .add_card(plain4)
        .add_card(growlmon4)
        .add_card(growl3)
        .add_card(red4)
        .add_card(tamer("TAKATO", "Takato Matsuki"))
        .add_card(tamer("TAKATO2", "Takato Matsuki"))
        .add_card(tamer("TAKA", "Takato"))
        .add_card(tamer("OPP-TAMER", "Opp Tamer"))
        .add_card(digimon("GUILMON", "Guilmon", 3, 3000))
        .add_card(digimon("GALLANTMON", "Gallantmon", 6, 12000))
        .add_card(digimon("TOP", "Top Digimon", 6, 12000))
        .add_card(digimon("OPP-6K", "Opp Six", 4, 6000))
        .add_card(digimon("OPP-7K", "Opp Seven", 4, 7000))
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

/// Pick the n-th non-PASS legal action.
fn pick_nth(r: &mut DebugRunner, n: usize) {
    let a = non_pass(r)[n];
    pick(r, a);
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

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
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
fn ad1_003_identity_matches_print() {
    let card = compiled(ID);
    assert_eq!(card.name, "WarGrowlmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    // Trait line = form + attribute + type (general_rule.pdf 2-3-2-1).
    let traits: Vec<String> = card.all_traits().iter().map(|t| t.to_lowercase()).collect();
    for t in ["cyborg", "hero", "ultimate", "virus"] {
        assert!(traits.contains(&t.to_string()), "missing {t}: {traits:?}");
    }
}

#[test]
fn ad1_003_alt_paths_are_the_printed_circle_and_the_digivolve_box() {
    let card = compiled(ID);
    assert_eq!(card.alt_paths.len(), 2, "standard circle + [Digivolve] box");
    for path in &card.alt_paths {
        assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(path.cost, Some(CompiledCost::Literal(3)), "both cost 3");
        assert!(
            !path.ignore_requirements,
            "digivolution requirements are respected"
        );
    }
}

#[test]
fn ad1_003_clause_shape() {
    let clauses = triggered(ID);
    let play = clauses
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert!(play.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(
        !play.optional && !play.once_per_turn,
        "each half carries its own 'may'; no [Once Per Turn]"
    );

    let card = compiled(ID);
    let replacement = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
            scope,
            optional,
            once_per_turn,
            ..
        }) => Some((*scope, *optional, *once_per_turn)),
        _ => None,
    });
    let (scope, optional, opt) = replacement.expect("inherited would-leave clause");
    assert_eq!(scope, CompiledScope::Inherited);
    assert!(optional, "'you may play'");
    assert!(!opt, "no [Once Per Turn] is printed");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — <Raid>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_003_face_up_raid_installs_on_the_carrier() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Raid));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Digivolve routes (standard circle + [Digivolve] box)
// ═══════════════════════════════════════════════════════════════════════════

/// Stage AD1-003 in hand over `base` on the field (Main phase, turn 1).
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

/// Standard circle: a RED Lv.4 base for cost 3.
#[test]
fn ad1_003_standard_circle_red_lv4_costs_3() {
    let (mut r, slot) = digivolve_over("RED4");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// Box: a Lv.4 Digimon with [Growlmon] in its name digivolves for 3 whatever
/// its colour (the box replaces the colour gate).
#[test]
fn ad1_003_box_accepts_a_growlmon_named_lv4_of_any_colour() {
    let (mut r, slot) = digivolve_over("GROWL4");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// Box: a Lv.4 Digimon with the [Hero] trait digivolves for 3 whatever its
/// colour.
#[test]
fn ad1_003_box_accepts_a_hero_trait_lv4_of_any_colour() {
    let (mut r, slot) = digivolve_over("HERO4");
    assert!(offered(&r, slot));
    let mem = r.memory();
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(mem - r.memory(), 3);
}

/// Negative: an off-colour Lv.4 with neither the name nor the trait matches no
/// circle.
#[test]
fn ad1_003_rejects_an_off_colour_lv4_without_the_name_or_trait() {
    let (mut r, slot) = digivolve_over("PLAIN4");
    assert!(!offered(&r, slot));
    let mem = r.memory();
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    assert_eq!(r.memory(), mem, "nothing is paid on rejection");
}

/// Negative: the box is level 4 — a [Growlmon] at Lv.3 is not a legal base.
#[test]
fn ad1_003_box_is_level_4_only() {
    let (mut r, slot) = digivolve_over("GROWL3");
    assert!(!offered(&r, slot));
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [On Play] / [When Digivolving]: may play Takato Matsuki;
//             then may delete a Digimon with 6000 DP or less
// ═══════════════════════════════════════════════════════════════════════════

const BOTH_TIMINGS: [EffectTiming; 2] = [EffectTiming::OnPlay, EffectTiming::WhenDigivolving];

/// Play Takato from the hand, then delete the 6000 DP Digimon (7000 DP is
/// protected by the limit). Both timings.
#[test]
fn ad1_003_plays_takato_from_hand_then_deletes_a_6000_dp_digimon() {
    for t in BOTH_TIMINGS {
        let mut r = builder().hand(0, &["TAKATO"]).start();
        let h = r.place_on_field(0, ID, Some(0));
        r.place_on_field(1, "OPP-6K", Some(0));
        r.place_on_field(1, "OPP-7K", Some(0));
        let mem = r.memory();
        trigger(&mut r, h, t);

        // Optional Takato pick (hand).
        assert!(r.pending_is_optional(), "'You may play': PASS available");
        assert_eq!(non_pass(&r).len(), 1);
        pick_nth(&mut r, 0);
        assert!(on_field(&r, 0, "TAKATO"), "{t:?}: Takato was played");
        assert_eq!(r.memory(), mem, "played without paying the cost");
        assert_eq!(r.hand_size(0), 0);

        // Optional delete of exactly one legal target.
        assert!(r.pending_is_optional(), "'you may delete': PASS available");
        assert_eq!(non_pass(&r).len(), 1, "only the 6000 DP Digimon qualifies");
        pick_nth(&mut r, 0);
        r.auto_resolve().unwrap();
        assert!(!on_field(&r, 1, "OPP-6K"), "{t:?}: 6000 DP Digimon deleted");
        assert!(on_field(&r, 1, "OPP-7K"), "7000 DP Digimon is out of range");
    }
}

/// Takato may be played from the trash as well as from the hand, and both
/// zones are offered when both hold one.
#[test]
fn ad1_003_takato_can_come_from_hand_or_trash() {
    let mut r = builder().hand(0, &["TAKATO"]).start();
    r.inject_trash(0, "TAKATO2");
    let h = r.place_on_field(0, ID, Some(0));
    trigger(&mut r, h, EffectTiming::OnPlay);
    let choices = non_pass(&r);
    assert_eq!(choices.len(), 2, "one hand copy + one trash copy");
    // Hand candidates are listed before trash candidates: take the trash copy.
    pick(&mut r, choices[1]);
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "TAKATO2"), "the trash copy was played");
    assert_eq!(r.hand_size(0), 1, "the hand copy stayed in hand");
    assert_eq!(r.game.players[0].trash.len(), 0);
}

/// Negative: only the exact name [Takato Matsuki] is a candidate.
#[test]
fn ad1_003_only_the_exact_name_takato_matsuki_is_offered() {
    let mut r = builder().hand(0, &["TAKA", "OPP-TAMER"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(
        r.pending_selection().is_none(),
        "no Takato Matsuki anywhere"
    );
    assert_eq!(r.hand_size(0), 2);
}

/// The two halves are independent: declining the play still offers the delete.
#[test]
fn ad1_003_declining_the_play_still_offers_the_delete() {
    let mut r = builder().hand(0, &["TAKATO"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, PASS); // decline Takato
    assert!(!on_field(&r, 0, "TAKATO"));
    assert!(
        r.pending_selection().is_some(),
        "the delete is still offered"
    );
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(!on_field(&r, 1, "OPP-6K"));
}

/// Declining the delete leaves the Digimon (and the played Takato stays).
#[test]
fn ad1_003_declining_the_delete_keeps_the_target() {
    let mut r = builder().hand(0, &["TAKATO"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick_nth(&mut r, 0);
    pick(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "TAKATO"));
    assert!(on_field(&r, 1, "OPP-6K"));
}

/// With no Takato the clause goes straight to the delete half.
#[test]
fn ad1_003_without_takato_only_the_delete_is_offered() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "OPP-6K", Some(0));
    trigger(&mut r, h, EffectTiming::WhenDigivolving);
    assert!(r.pending_is_optional());
    assert_eq!(non_pass(&r).len(), 1);
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(!on_field(&r, 1, "OPP-6K"));
}

/// Boundary: a 7000 DP Digimon is not a legal delete target, and neither is a
/// Tamer; with no legal target no prompt is installed.
#[test]
fn ad1_003_no_delete_prompt_without_a_6000_dp_or_less_digimon() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "OPP-7K", Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
    assert!(on_field(&r, 1, "OPP-7K"));
}

/// Integration through the real digivolve flow ([When Digivolving]): digivolving
/// WarGrowlmon in from the hand runs the same clause.
#[test]
fn ad1_003_when_digivolving_runs_through_a_real_digivolve() {
    let mut r = builder().hand(0, &[ID, "TAKATO"]).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "RED4", Some(0)).index as usize;
    r.place_on_field(1, "OPP-6K", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_some(), "Takato prompt appears");
    pick_nth(&mut r, 0);
    pick_nth(&mut r, 0); // delete the 6000 DP Digimon
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "TAKATO"));
    assert!(!on_field(&r, 1, "OPP-6K"));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [All Turns]: would leave other than by your effects →
//             may play 1 each of Takato Matsuki and Guilmon from its sources
// ═══════════════════════════════════════════════════════════════════════════

fn leaving_stack(sources: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().start();
    let h = r.place_stack(0, sources);
    (r, h)
}

/// Q6053: with both present the effect plays BOTH (never just one), the leaving
/// Digimon still leaves, and nothing is paid. Both non-own causes.
#[test]
fn ad1_003_inherited_plays_takato_and_guilmon_and_the_digimon_still_leaves() {
    for cause in [ReplacementCause::OpponentEffect, ReplacementCause::Battle] {
        let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", ID, "GALLANTMON"]);
        let mem = r.memory();
        r.game.delete_permanent_with_cause(host, cause);

        assert!(r.pending_is_optional(), "the outer 'you may' prompt");
        assert!(
            on_field(&r, 0, "GALLANTMON"),
            "the original waits for the choice"
        );
        pick_nth(&mut r, 0); // accept

        // Q6053: the pair is mandatory — no PASS while both are playable.
        assert!(
            !r.pending_is_optional(),
            "{cause:?}: can't play just one of them (Q6053)"
        );
        assert_eq!(non_pass(&r).len(), 2, "Takato and Guilmon are the picks");
        pick_nth(&mut r, 0);
        if r.pending_selection().is_some() {
            assert!(
                !r.pending_is_optional(),
                "the second card is still required"
            );
            pick_nth(&mut r, 0);
        }
        r.auto_resolve().unwrap();

        assert!(on_field(&r, 0, "TAKATO"), "{cause:?}");
        assert!(on_field(&r, 0, "GUILMON"), "{cause:?}");
        assert!(
            !on_field(&r, 0, "GALLANTMON"),
            "the play does not prevent the leave"
        );
        assert_eq!(r.memory(), mem, "free play");
    }
}

/// Q6054: with only [Takato Matsuki] under it, just that card is played.
#[test]
fn ad1_003_inherited_plays_just_takato_when_there_is_no_guilmon() {
    let (mut r, host) = leaving_stack(&["TAKATO", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick_nth(&mut r, 0); // accept
    if r.pending_selection().is_some() {
        pick_nth(&mut r, 0);
    }
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "TAKATO"));
    assert!(!on_field(&r, 0, "GUILMON"));
    assert!(!on_field(&r, 0, "GALLANTMON"));
}

/// The mirror: only [Guilmon] under it plays just Guilmon.
#[test]
fn ad1_003_inherited_plays_just_guilmon_when_there_is_no_takato() {
    let (mut r, host) = leaving_stack(&["GUILMON", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::Battle);
    pick_nth(&mut r, 0);
    if r.pending_selection().is_some() {
        pick_nth(&mut r, 0);
    }
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "GUILMON"));
    assert!(!on_field(&r, 0, "TAKATO"));
}

/// With two [Takato Matsuki] under it, which one is played stays the player's
/// choice (one is picked for the Takato role, never auto-chosen).
#[test]
fn ad1_003_inherited_which_takato_is_a_player_choice() {
    let (mut r, host) = leaving_stack(&["TAKATO", "TAKATO2", "GUILMON", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick_nth(&mut r, 0); // accept
    assert_eq!(
        non_pass(&r).len(),
        3,
        "both Takatos and the Guilmon are legal first picks"
    );
    // Pick the SECOND Takato explicitly, then finish the pair.
    pick_nth(&mut r, 1);
    assert!(r.pending_selection().is_some(), "the pair is incomplete");
    assert_eq!(
        non_pass(&r).len(),
        1,
        "only the Guilmon can complete the one-of-each pair"
    );
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 0, "TAKATO2"), "the chosen Takato was played");
    assert!(!on_field(&r, 0, "TAKATO"), "the other Takato was not");
    assert!(on_field(&r, 0, "GUILMON"));
}

/// Declining the outer prompt plays nothing and the Digimon leaves.
#[test]
fn ad1_003_inherited_decline_plays_nothing() {
    let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert!(!on_field(&r, 0, "TAKATO"));
    assert!(!on_field(&r, 0, "GUILMON"));
    assert!(!on_field(&r, 0, "GALLANTMON"));
}

/// "Other than by your effects": the controller's own effect does not trigger
/// it.
#[test]
fn ad1_003_inherited_does_not_apply_to_your_own_effects() {
    let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OwnEffect);
    assert!(r.pending_selection().is_none());
    assert!(!on_field(&r, 0, "TAKATO"));
    assert!(!on_field(&r, 0, "GALLANTMON"));
}

/// The carrier's name must contain [Gallantmon]: a different top card does not
/// qualify.
#[test]
fn ad1_003_inherited_requires_gallantmon_in_the_carrier_name() {
    let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", ID, "TOP"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert!(!on_field(&r, 0, "TAKATO"));
}

/// Only cards in ITS digivolution cards count: with neither Takato nor Guilmon
/// under it nothing is offered.
#[test]
fn ad1_003_inherited_needs_takato_or_guilmon_under_it() {
    let (mut r, host) = leaving_stack(&[ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
}

/// It is an inherited effect: WarGrowlmon must be under the Digimon (a Takato
/// and Guilmon under a Gallantmon without WarGrowlmon give nothing).
#[test]
fn ad1_003_inherited_requires_wargrowlmon_in_the_stack() {
    let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert!(!on_field(&r, 0, "TAKATO"));
}

/// The pending inherited choice clones independently (clone-safety: the pair
/// selection is a data frame, not a closure).
#[test]
fn ad1_003_inherited_pending_choice_clones_independently() {
    let (mut r, host) = leaving_stack(&["TAKATO", "GUILMON", ID, "GALLANTMON"]);
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    pick_nth(&mut r, 0); // accept
    let cloned = r.game.clone();
    pick_nth(&mut r, 0);
    pick_nth(&mut r, 0);
    r.auto_resolve().unwrap();
    assert!(
        cloned.pending_selection.is_some(),
        "the clone keeps its own unresolved pair choice"
    );
    assert!(on_field(&r, 0, "TAKATO"));
}
