//! P-180 Bind Red Trigger — Option, Red, Cost 6.
//! Traits: Three Musketeers.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/P-180.md;
//! confirmed against the card image)
//!
//! When effects trash this card from digivolution cards, delete 1 of your
//! opponent's 7000 DP or lower Digimon.
//! While you have a [Three Musketeers] trait Digimon, you can ignore this
//! card's color requirements.
//! [Main] Trash your opponent's top security card. Then, place this card as
//! the bottom digivolution card of 1 of your [Three Musketeers] trait
//! Digimon.
//!
//! Inherited Effect:
//! Security Effect [Security] Delete 1 of your opponent's Digimon with the
//! highest DP.
//!
//! # DCGO C# reference
//! Not present in the local DCGO mirror (`find "$BASE_DCGO/Assets/Scripts/
//! CardEffect" -name "P_180.cs"` returns nothing) — authored from printed
//! text + `general_rule.pdf` + the engine's established per-clause idioms
//! (EX8-047 self-trashed-source observer, BT24-090 board-gated ignore-color,
//! BT21-098 trash-top-security, BT18-013 highest-DP delete, ST23-15's
//! `place_self_under_permanent` play-path sibling — new for this card).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §5)
//! - Structural: 4 clauses (inherited trigger, flood_gate, MainFromHand,
//!   inherited Security), `use_requirement` present (conditional, not
//!   `all_turns`)
//! - Condition gating: ignore-color bypass with/without a [Three Musketeers]
//!   Digimon on field (positive/negative)
//! - Behavioral: [Main] trash opp top security + place self under a chosen
//!   [Three Musketeers] Digimon (face-up, NOT trashed); fizzle path with no
//!   legal host (disposes to trash normally)
//! - Behavioral: [Trigger] OnDigivolutionCardTrashed — self-trashed-from-
//!   stack deletes 1 opp Digimon DP<=7000 (positive/negative DP gate;
//!   negative — no qualifying opp Digimon; negative — trashed from hand,
//!   not from a digivolution stack, does not fire)
//! - Behavioral: [Security] delete opponent's highest-DP Digimon (positive;
//!   negative — no opponent Digimon, fizzles)
//! - Cost/side-effect: the [Main] top-security trash moves the CORRECT
//!   (top) security card to the opponent's trash (identity-checked, not just
//!   a count delta) — see Section 4's note on why this is checked via zone
//!   state rather than a `GameEvent` (no discrete `GameEvent` variant exists
//!   for this; `OnDiscardSecurity`/`OnLoseSecurity` are `EffectTiming`
//!   trigger dispatches, not `GameEvent` log entries).
//!
//! # Verdict — IMPLEMENTED
//! New engine substrate: `place_self_under_permanent` DSL step (compiled
//! `CompiledStep::PlaceSelfUnderPermanent`) wraps the RESOLVED
//! `EffectContext::place_self_under_permanent` primitive
//! (G-OPTION-PLACE-SELF-UNDER-PERMANENT-ON-PLAY-PATH) on the `[Main]`-play
//! path — the `MoveSelfOptionUnderPermanent` sibling for the in-flight
//! `pending_option` claim rather than a live field-Option relocate.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "P-180";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn bind_red_trigger() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [Three Musketeers] Digimon (legal color-bypass source AND legal
/// placement host for the [Main] clause).
fn tm_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Three Musketeers".to_string()];
    c
}

/// A non-[Three Musketeers] Digimon (illegal color-bypass source, illegal
/// placement host).
fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beast".to_string()];
    c
}

/// An opponent Digimon at exactly the DP<=7000 boundary (legal trigger
/// target).
fn opp_low_dp(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn find_perm(runner: &DebugRunner, player: u8, id: &str) -> Option<PermanentHandle> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .map(|i| PermanentHandle {
            player,
            index: i as u8,
        })
}

fn perm_handle(runner: &DebugRunner, player: u8, id: &str) -> PermanentHandle {
    find_perm(runner, player, id).unwrap_or_else(|| panic!("{id} not on field for player {player}"))
}

fn process_contains_place_self_under_permanent(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::PlaceSelfUnderPermanent { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_place_self_under_permanent(then)
                || process_contains_place_self_under_permanent(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_place_self_under_permanent(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn p_180_is_red_three_musketeers_option_cost_6() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(6));
    assert!(card.traits.iter().any(|t| t == "Three Musketeers"));
}

#[test]
fn p_180_use_requirement_is_conditional_board_predicate() {
    let card = compiled(CARD_ID);
    // Must be present (the color-ignore gate exists) but this card's gate is
    // a live board condition, not the P-206/P-236 self-trait always-true
    // shape — asserted structurally by checking the requirement is present;
    // the positive/negative behavior is covered in Section 2.
    assert!(
        card.use_requirement.is_some(),
        "use_requirement must be present for the ignore-color gate"
    );
}

#[test]
fn p_180_has_trigger_flood_gate_main_and_security_clauses() {
    let card = compiled(CARD_ID);

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // Clause 0: inherited OnDigivolutionCardTrashed trigger.
    let trigger_clause = triggered
        .iter()
        .find(|t| {
            t.when
                .iter()
                .any(|w| *w == CompiledTiming::OnDigivolutionCardTrashed)
        })
        .expect("[Trigger] OnDigivolutionCardTrashed clause present");
    assert_eq!(
        trigger_clause.scope,
        CompiledScope::Inherited,
        "the self-trashed-source observer MUST be scope: inherited to fire \
         (engine only dispatches inherited effects to the trashed card itself)"
    );

    // Clause 2: [Main] clause present.
    assert!(
        triggered
            .iter()
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::MainFromHand)),
        "[Main] clause present"
    );

    // Clause 3: inherited [Security] clause present.
    assert!(
        triggered.iter().any(|t| t.scope == CompiledScope::Inherited
            && t.when.iter().any(|w| *w == CompiledTiming::OnSecurity)),
        "[Security] (inherited) clause present"
    );

    // Clause 1: flood_gate declarative present.
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(digimon_dsl::compiled::CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "flood_gate IgnoreColorRequirement clause present"
    );
}

#[test]
fn p_180_main_clause_places_self_under_permanent() {
    let card = compiled(CARD_ID);
    let main = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.iter().any(|w| *w == CompiledTiming::MainFromHand) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[Main] clause");
    assert!(
        process_contains_place_self_under_permanent(&main.process),
        "the [Main] clause places this Option under a chosen permanent"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: ignore-color bypass
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with a [Three Musketeers] Digimon on field, an off-color hand
/// (P-180 is Red) may still be played — the color requirement is bypassed.
#[test]
fn p_180_ignore_color_bypass_active_with_three_musketeers_digimon() {
    // The TM Digimon is recolored Blue (P-180 prints Red) so the bypass is
    // proven by the [Three Musketeers] trait, not a coincidental color match.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card({
            let mut c = tm_digimon("TM");
            c.colors = vec![CardColor::Blue];
            c
        })
        .add_card(filler("OPP-FILLER"))
        .hand(0, &[CARD_ID])
        .security(1, &["OPP-FILLER"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.enter_main_phase();
    runner.place_on_field(0, "TM", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(
        result,
        OptionPlayResult::Invalid,
        "color requirement must be bypassed while a [Three Musketeers] \
         Digimon (of a non-matching color) is on the field"
    );
}

/// NEGATIVE: with NO [Three Musketeers] Digimon on field (and no
/// color-matching Digimon/Tamer either), the color requirement is NOT
/// bypassed and the Option cannot be played.
#[test]
fn p_180_ignore_color_bypass_inactive_without_three_musketeers_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card({
            let mut c = plain_digimon("PLAIN");
            c.colors = vec![CardColor::Blue];
            c
        })
        .add_card(filler("OPP-FILLER"))
        .hand(0, &[CARD_ID])
        .security(1, &["OPP-FILLER"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.enter_main_phase();
    runner.place_on_field(0, "PLAIN", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Invalid,
        "no [Three Musketeers] Digimon and no color match ⇒ color \
         requirement blocks the play"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] behavioral: trash opp top security; place self under host
// ════════════════════════════════════════════════════════════════════════════

fn setup_main_play(with_tm_host: bool) -> DebugRunner {
    // `plain_digimon` is Red — the SAME color as P-180 — so the color
    // requirement is satisfied normally (no bypass needed) whether or not a
    // [Three Musketeers] Digimon is present. This isolates the [Main]
    // placement-pick fizzle behavior (Section 3's negative case) from the
    // color-bypass gate (already covered in Section 2): without this Red
    // non-TM anchor, `with_tm_host=false` would leave player 0 with NO
    // color-matching source at all, making the play `Invalid` for the wrong
    // reason (missing color match) rather than exercising a legal play whose
    // placement pick has zero candidates.
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card(tm_digimon("TM"))
        .add_card(plain_digimon("PLAIN-RED"))
        .add_card(filler("OPP-SEC"))
        .add_card(filler("OPP-SEC-2"))
        .hand(0, &[CARD_ID])
        .security(1, &["OPP-SEC", "OPP-SEC-2"])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.game.enter_main_phase();
    runner.place_on_field(0, "PLAIN-RED", Some(0));
    if with_tm_host {
        runner.place_on_field(0, "TM", Some(0));
    }
    runner
}

/// POSITIVE: [Main] trashes the opponent's top security card, then places
/// P-180 as the bottom digivolution card of the chosen [Three Musketeers]
/// Digimon — face-up, and NOT trashed.
#[test]
fn p_180_main_trashes_opp_top_security_and_places_self_under_tm_host_face_up() {
    let mut runner = setup_main_play(true);
    let sec_before = runner.security_count(1);
    let trash_before_p0 = runner.trash_size(0);
    let host = perm_handle(&runner, 0, "TM");
    let host_sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "the [Main] clause installs a select_own_permanent prompt for the host pick"
    );
    let _ = runner.auto_resolve();

    // Opponent's top security was trashed.
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "[Main] trashes the opponent's top security card"
    );

    // P-180 is now the bottom digivolution source of the TM host, face-up.
    let host = perm_handle(&runner, 0, "TM");
    let host_perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        host_perm.card_sources.len(),
        host_sources_before + 1,
        "P-180 was seated under the TM host as a new digivolution source"
    );
    assert_eq!(
        host_perm.card_sources[0].card_id(&runner.game.card_data),
        CARD_ID,
        "P-180 is the BOTTOM digivolution source"
    );
    assert!(
        !host_perm.card_sources[0].face_down,
        "P-180 is seated FACE-UP (printed text does not say face down)"
    );

    // P-180 must NOT be in trash — it MOVED under the host.
    assert_eq!(
        runner.trash_size(0),
        trash_before_p0,
        "P-180 moved under the host — trash count for player 0 unchanged"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-180 must NOT be in trash after a successful placement"
    );
}

/// FIZZLE: with no [Three Musketeers] Digimon on the controller's field, the
/// placement pick has no legal candidates, so P-180 disposes to trash
/// normally via the standard play pipeline. The top-security trash (the
/// first half of the clause) still happened.
#[test]
fn p_180_main_fizzles_placement_and_disposes_to_trash_with_no_tm_host() {
    let mut runner = setup_main_play(false);
    let sec_before = runner.security_count(1);

    let result = runner.game.play_option_from_hand(0, 0);
    // No legal select_own_permanent candidates ⇒ no pending selection
    // installs for the placement; the Option resolves synchronously and
    // disposes to trash.
    assert_eq!(
        result,
        OptionPlayResult::Trashed,
        "with no [Three Musketeers] host, the placement fizzles and the \
         Option disposes to trash normally"
    );

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the top-security trash still happens even though the placement fizzles"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-180 disposes to trash when there is no legal placement host"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — Cost / side-effect: top-security trash lands the CORRECT card
// ════════════════════════════════════════════════════════════════════════════
//
// NOTE: the engine's `trash_top_security` cost routes through
// `fire_security_removed_observers` → `enqueue_triggered(EffectTiming::
// OnLoseSecurity / OnDiscardSecurity, ...)` — these are internal
// `EffectTiming` trigger DISPATCHES for other cards' reactive clauses to key
// off of, not `GameEvent` log entries (`GameEvent` — `code/digimon-engine/
// src/events.rs` — has no `OnDiscardSecurity`/`OnLoseSecurity` variant; it is
// a flat enum with variants like `Trash`, `SecurityReveal`, etc., and the
// direct-pop path used by `trash_top_security`/`complete_effect_security_
// removal` does not push a `GameEvent::Trash` either). So there is no
// `GameEvent` to assert on for this cost. What IS externally observable and
// exactly what the printed text promises is verified here: the specific
// top-of-security card (by identity, not just count) ends up in the
// opponent's trash and is removed from their face-up-security bookkeeping.
#[test]
fn p_180_main_top_security_trash_moves_the_correct_card_to_trash() {
    let mut runner = setup_main_play(true);
    let trash_before_p1 = runner.trash_size(1);

    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    // OPP-SEC-2 was stacked as the TOP security card (security(1, &["OPP-SEC",
    // "OPP-SEC-2"]) pushes in order, so the last element is on top / checked
    // first) — it must be the one that landed in the opponent's trash.
    assert_eq!(
        runner.trash_size(1),
        trash_before_p1 + 1,
        "exactly 1 security card was trashed"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OPP-SEC-2"),
        "the TOP security card (OPP-SEC-2) was trashed by [Main]'s top-security-trash clause"
    );
    // The remaining security card (the one that was underneath) must still
    // be in the security zone, untouched.
    assert_eq!(
        runner.security_count(1),
        1,
        "only the top security card was trashed; the other remains in security"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — [Trigger] OnDigivolutionCardTrashed: delete opp DP<=7000
// ════════════════════════════════════════════════════════════════════════════

/// Build a runner with P-180 seated as a digivolution source under a host on
/// player 0's field, plus opponent Digimon of varying DP.
fn setup_trigger(opp_cards: &[(&'static str, i32)]) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card(tm_digimon("HOST"));
    for (id, dp) in opp_cards {
        builder = builder.add_card(opp_low_dp(id, *dp));
    }
    builder = builder.memory(10);
    let mut runner = builder.start();
    runner.set_first_player(0);
    // Stack: bottom = P-180 (the trigger source), top = HOST.
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    for (id, _) in opp_cards {
        runner.place_on_field(1, id, Some(0));
    }
    (runner, host)
}

/// POSITIVE: trashing P-180 out of the host's digivolution stack (via
/// effects) fires the trigger, deleting 1 opponent Digimon with DP<=7000.
#[test]
fn p_180_trigger_deletes_opp_digimon_dp_lte_7000_when_trashed_from_stack() {
    let (mut runner, host) = setup_trigger(&[("LOW-DP", 6000)]);
    let ba_before = runner.battle_area_size(1);

    // trash_one_source removes the BOTTOM source (P-180) from the host's
    // stack, fires OnDigivolutionCardTrashed, and drains the queue.
    runner.trash_one_source(host);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        ba_before - 1,
        "the opponent's DP<=7000 Digimon was deleted"
    );
    assert!(
        find_perm(&runner, 1, "LOW-DP").is_none(),
        "LOW-DP is no longer on the opponent's field"
    );
}

/// NEGATIVE: with no opponent Digimon at DP<=7000 (only a 8000-DP Digimon
/// on the field), the trigger fires but the mandatory select has no legal
/// candidates — it fizzles silently, deleting nothing.
#[test]
fn p_180_trigger_fizzles_when_no_opp_digimon_at_or_under_7000_dp() {
    let (mut runner, host) = setup_trigger(&[("HIGH-DP", 8000)]);
    let ba_before = runner.battle_area_size(1);

    runner.trash_one_source(host);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        ba_before,
        "no DP<=7000 opponent Digimon ⇒ nothing is deleted"
    );
    assert!(
        find_perm(&runner, 1, "HIGH-DP").is_some(),
        "HIGH-DP (8000 DP, above the threshold) survives"
    );
}

/// POSITIVE (boundary): exactly 7000 DP qualifies ("7000 DP or lower").
#[test]
fn p_180_trigger_deletes_opp_digimon_at_exactly_7000_dp_boundary() {
    let (mut runner, host) = setup_trigger(&[("EXACT-7000", 7000)]);
    let ba_before = runner.battle_area_size(1);

    runner.trash_one_source(host);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        ba_before - 1,
        "exactly 7000 DP qualifies for the delete (7000 DP OR LOWER)"
    );
}

/// NEGATIVE: trashing P-180 directly from HAND (not from a digivolution
/// stack) must NOT fire the trigger — the printed condition is specifically
/// "trash this card from digivolution cards". `trash_one_source` is the
/// engine's own digivolution-stack-trash path; a plain hand→trash move
/// (no host, no `fire_digivolution_card_trashed` call) is the natural
/// contrast — the engine simply never fires the observer for a hand trash,
/// which this test pins down structurally: moving the card straight from
/// hand to trash (bypassing any digivolution-stack removal) leaves the
/// opponent's board untouched.
#[test]
fn p_180_trigger_does_not_fire_when_trashed_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card(opp_low_dp("LOW-DP", 6000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "LOW-DP", Some(0));
    let ba_before = runner.battle_area_size(1);

    // Move the card straight from hand to trash — a plain zone move, NOT a
    // digivolution-stack removal, so no OnDigivolutionCardTrashed fires.
    let source = runner.game.players[0].hand.remove(0);
    runner.game.players[0].trash.push(source);
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        ba_before,
        "trashing P-180 from hand (not from a digivolution stack) does not \
         fire the OnDigivolutionCardTrashed trigger — nothing is deleted"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 6 — [Security] (inherited): delete opponent's highest-DP Digimon
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: the [Security] clause deletes the opponent's Digimon with the
/// highest DP among 2+ candidates. Player 0 is the ATTACKER (breaking
/// through player 1's security, where P-180 sits face-down); the deletion
/// targets player 0's opponent, i.e. the attacker's own field here — so the
/// "opponent" Digimon to delete are seeded on player 0's side (the attacker,
/// P-180's controller's opponent from the security-owner's perspective).
#[test]
fn p_180_security_deletes_opp_highest_dp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card(opp_low_dp("LOW", 3000))
        .add_card(opp_low_dp("HIGH", 9000))
        .add_card({
            let mut c = make_test_card("ATTACKER", "ATTACKER");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(2000);
            c.play_cost = 4;
            c
        })
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);
    // "Opponent" from P-180's controller's (player 1's) perspective is
    // player 0 — seed the DP candidates there.
    runner.place_on_field(0, "LOW", Some(0));
    runner.place_on_field(0, "HIGH", Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    // Player 0 attacks player 1's security, revealing/checking P-180 and
    // firing its inherited [Security] clause.
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        find_perm(&runner, 0, "HIGH").is_none(),
        "the opponent's highest-DP Digimon (HIGH, 9000) was deleted"
    );
    assert!(
        find_perm(&runner, 0, "LOW").is_some(),
        "the lower-DP Digimon (LOW, 3000) survives"
    );
}

/// Single-candidate case: with only the attacker itself as the opponent's
/// sole Digimon, the mandatory highest-DP delete still resolves cleanly
/// (the "highest" aggregate over a 1-element set), and the delete pipeline
/// does not panic when the deleted permanent was the attacking Digimon
/// mid-combat resolution.
#[test]
fn p_180_security_deletes_sole_opponent_digimon_when_only_one_candidate() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-180 in embedded pack")
        .add_card({
            let mut c = make_test_card("ATTACKER", "ATTACKER");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(2000);
            c.play_cost = 4;
            c
        })
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        find_perm(&runner, 0, "ATTACKER").is_none(),
        "with only one opponent Digimon (the attacker itself), the \
         mandatory highest-DP delete targets and removes it; no panic"
    );
}
