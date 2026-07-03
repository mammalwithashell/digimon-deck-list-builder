
//! P-193 The Wicked God Emerges! — Option, Purple, Cost 3.
//!
//! # Card text (code/digimon-engine/cards/p/P-193.json, official bundle
//! # data/card_bundles/P-193.md — card image confirmed)
//!
//! [Main] By trashing 1 card with the [Composite] or [Wicked God] trait from
//! your hand, ＜Draw 2＞ (Draw 2 cards from your deck.) Then, place this card
//! in the battle area.
//!
//! [End of All Turns] ＜Delay＞ (By trashing this card after the placing
//! turn, activate the effect below.)
//! ・By deleting 1 of your [Millenniummon], you may play 1 [Wicked God]
//!   trait Digimon card from your hand or trash without paying the cost.
//!
//! Inherited (Security Effect):
//! [Security] Activate this card's [Main] effects.
//!
//! # Official Q&A
//! "No, you can't. If you don't trash 1 card with the [Composite] or
//! [Wicked God] trait from your hand, you can't use the part after 'then'
//! in this card's [Main] effect to place this card in the battle area."
//! → the Draw 2 AND the self-placement are BOTH gated behind actually
//! trashing the cost card (`G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE`, the
//! `select_hand { cost: true }` idiom, BT24-008 precedent).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Purple/P_193.cs
//!
//! - `EffectTiming.OptionSkill` — Main clause. `SelectHandEffect` with
//!   `mode: Discard`, `maxCount: 1`, `canNoSelect: true`. Only on a
//!   successful discard does it `Draw(2)` then `PlaceDelayOptionCards`.
//! - `EffectTiming.OnEndTurn` — the Delay body. `CanUseCondition:
//!   CanDeclareOptionDelayEffect` (placing turn has passed — same gate as
//!   every other `<Delay>` in the pool), `CanActivateCondition:
//!   IsExistOnField`. This is the SAME player-choice-driven "trash this
//!   card to activate" mechanic as the standard `[Main]<Delay>` (P-039,
//!   P-107) and the sibling `[End of Your Turn]<Delay>` (BT21-097) —
//!   DCGO's own `SetUpActivateClass(..., isOptional: true, ...)` and the
//!   `CanDeclareOptionDelayEffect` gate are byte-for-byte identical to the
//!   Main-phase-activated Delay pattern; only the *window* the activation
//!   check runs in differs (End-of-Turn scan vs Main-phase action), which
//!   the engine's `DelayTrigger::MainPhaseActivated` (`kind: delay,
//!   trigger: delayed`) already models faithfully per the BT21-097
//!   precedent (see file header there — `[End of Your Turn]<Delay>` is the
//!   same `kind: delay` substrate). `[End of All Turns]` and `[End of Your
//!   Turn]` are BOTH lowered as the standard controller-choice `<Delay>` —
//!   confirmed against `docs/digimon-rules/keyword-semantics.md`'s `<Delay>`
//!   row (16-16): "Trash this card to activate the linked effect; optional;
//!   not the turn it entered the battle area."
//! - `EffectTiming.SecuritySkill` — `AddActivateMainOptionSecurityEffect`,
//!   the standard "[Security] Activate this card's [Main] effects" replay
//!   (P-151 idiom).
//!
//! # Patterns this test file covers
//! - E2: optional cost-as-trashing [Main] (`select_hand { cost: true }`,
//!   BT24-008 idiom) gating BOTH Draw 2 and self-placement.
//! - Standard `<Delay>` (`kind: delay, trigger: delayed`) — BT21-097 /
//!   P-039 / P-107 idiom, here under the `[End of All Turns]` timing
//!   prefix.
//! - Delay body: optional `select_own_permanent` (name-filtered,
//!   `name_is: "Millenniummon"`) → `delete_permanent`, then optional
//!   `select_union_zone` (hand+trash, trait-filtered `Wicked God` +
//!   `kind: digimon`) → `play_union_bound_free` (BT17-095 / AD1-016 idiom).
//! - [Security] (inherited): "Activate this card's [Main] effects" — mirrors
//!   the Main clause's process body (P-151 idiom).

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-193";

// ─── Helper cards ────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Purple Digimon used purely to satisfy the printed Option color
/// requirement (16-10-ish: playing a colored Option requires a same-color
/// Digimon/Tamer on the field/breeding area). P-193 carries no color-bypass
/// clause, so every [Main]-from-hand test must seat one of these first.
fn purple_color_anchor(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c
}

/// A hand-eligible Composite or Wicked God trait card usable as the [Main]
/// cost (any card kind; DCGO's `CanSelectHandCondition` checks trait only).
fn cost_card(id: &str, trait_name: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c.traits = vec![trait_name.to_string()];
    c
}

/// A Digimon named exactly "Millenniummon" for the Delay-body deletion pick.
fn millenniummon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Millenniummon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Purple];
    c
}

/// A non-Millenniummon own Digimon (negative-filter target).
fn other_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "OtherDigimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c.colors = vec![CardColor::Purple];
    c
}

/// A [Wicked God] trait Digimon card eligible for the free-play reward.
fn wicked_god_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(13000);
    c.play_cost = 13;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Wicked God".to_string()];
    c
}

/// A Digimon WITHOUT the [Wicked God] trait (negative-filter target for the
/// free-play reward).
fn non_wicked_god_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Purple];
    c
}

fn p193_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .memory(10)
        .build()
}

/// Seat P-193 as a Delay-Option permanent (past its placing turn) so its
/// Delay body's Main-phase activation is legal, mirroring
/// `seat_and_advance_to_activatable_delay` in bt21_097.rs.
fn seat_and_advance_to_activatable_delay(
    runner: &mut DebugRunner,
) -> digimon_engine::permanent::PermanentHandle {
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let placing_turn = runner.game.turn_count;
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: placing_turn,
        };
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 0);
    runner.game.enter_main_phase();
    handle
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p193_yaml_loads_without_error() {
    let _runner = p193_runner();
}

/// Three compiled clauses: [Main] (triggered), the Delay body (declarative
/// Delay), [Security] (triggered, inherited scope).
#[test]
fn p193_has_three_clauses() {
    let runner = p193_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    assert_eq!(
        compiled.effects.len(),
        3,
        "P-193 must have exactly 3 compiled clauses (Main, Delay, Security); got {}",
        compiled.effects.len()
    );
}

/// [Main] clause: `main_from_hand` timing, optional (the whole tail is
/// cost-gated), own (FaceUp) scope.
#[test]
fn p193_main_clause_is_optional_face_up() {
    let runner = p193_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("Main clause with MainFromHand timing must exist");

    assert!(
        main_clause.optional,
        "P-193 Main clause must be optional (cost-gated 'By trashing…')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "P-193 Main clause must have FaceUp scope (Option played from hand)"
    );
}

/// The Delay body is a declarative `Delay` clause with `CompiledTiming::Delayed`
/// (standard controller-choice `<Delay>` — `DelayTrigger::MainPhaseActivated`).
#[test]
fn p193_has_standard_delay_clause() {
    let runner = p193_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let delay = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger,
                process,
                ..
            }) => Some((trigger, process)),
            _ => None,
        })
        .expect("standard <Delay> clause present");

    assert_eq!(
        *delay.0,
        CompiledTiming::Delayed,
        "the [End of All Turns] <Delay> lowers to the standard controller-choice Delay trigger"
    );
}

/// Security clause: `on_security` timing, optional (mirrors Main), Inherited
/// scope.
#[test]
fn p193_security_clause_is_optional_inherited() {
    let runner = p193_runner();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled card");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("Security clause with OnSecurity timing must exist");

    assert!(
        sec_clause.optional,
        "P-193 Security clause must be optional (mirrors the cost-gated Main body)"
    );
    assert_eq!(
        sec_clause.scope,
        CompiledScope::Inherited,
        "P-193 Security clause must have Inherited scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [Main]: positive branch (trashes a Composite/Wicked God card)
// ═══════════════════════════════════════════════════════════════════════════

/// Happy path: hand has a [Composite]-trait card as the cost. Accepting the
/// prompt trashes it, draws 2, and places P-193 in the battle area as a Delay
/// Option.
#[test]
fn p193_main_composite_cost_trashes_draws_two_and_places_self() {
    let composite = cost_card("P193-COMPOSITE", "Composite");
    let fill = filler("FILL");
    let anchor = purple_color_anchor("P193-ANCHOR");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(composite)
        .add_card(fill)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-COMPOSITE"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    // Printed Option color requirement: P-193 needs a Purple Digimon/Tamer
    // on the field to be playable (no color-bypass clause on this card).
    runner.place_on_field(0, "P193-ANCHOR", Some(0));

    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "Main should install a Hand selection for the cost trash"
    );
    assert!(
        runner.pending_is_optional(),
        "the cost-trash prompt must be optional (declinable)"
    );

    runner
        .auto_resolve()
        .expect("resolve picks the eligible Composite card");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the Composite cost card must be trashed"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "deck shrinks by 2 after Draw 2"
    );
    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("P-193 must be placed in the battle area");
    assert!(
        matches!(
            placed.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        ),
        "P-193 must park as a standard <Delay> Option after Main resolves"
    );
}

/// A [Wicked God]-trait card is also a valid cost (the printed "or").
#[test]
fn p193_main_wicked_god_cost_is_valid() {
    let wg_cost = cost_card("P193-WG-COST", "Wicked God");
    let fill = filler("FILL");
    let anchor = purple_color_anchor("P193-ANCHOR2");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(wg_cost)
        .add_card(fill)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-WG-COST"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR2", Some(0));

    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "Wicked God cost card must be offered as an eligible cost"
    );
    runner
        .auto_resolve()
        .expect("resolve picks the Wicked God cost card");

    assert_eq!(runner.trash_size(0), trash_before + 1);
    assert_eq!(runner.deck_size(0), deck_before - 2);
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "P-193 must be placed on field"
    );
}

/// A card with neither trait is NOT offered as a cost.
#[test]
fn p193_main_ineligible_card_not_offered_as_cost() {
    let ineligible = filler("P193-INELIGIBLE");
    let composite = cost_card("P193-COMPOSITE2", "Composite");
    let anchor = purple_color_anchor("P193-ANCHOR3");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(ineligible)
        .add_card(composite)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-INELIGIBLE", "P193-COMPOSITE2"])
        .deck(0, &["P193-COMPOSITE2"; 5])
        .deck(1, &["P193-COMPOSITE2"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR3", Some(0));

    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    let view = runner
        .pending_selection_view()
        .expect("Main cost-trash prompt installs");
    // Only the Composite-trait card (hand index shifts after P-193 leaves
    // hand) should be a legal non-PASS pick — the ineligible filler must not
    // appear among valid actions.
    use digimon_engine::action::space::PASS as PASS_ACTION;
    let non_pass_count = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS_ACTION)
        .count();
    assert_eq!(
        non_pass_count, 1,
        "only the Composite-trait card is a legal cost pick; got {:?}",
        view.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Main]: decline branch (official Q&A — decline aborts the tail)
// ═══════════════════════════════════════════════════════════════════════════

/// Decline branch: an eligible cost card exists in hand, but the player
/// declines the prompt. Per official Q&A, Draw 2 must NOT occur, and the
/// cost card must NOT be trashed — the `cost: true` decline-abort
/// (`G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE`) correctly skips both.
///
/// The self-placement half of this same official-Q&A gate is a SEPARATE,
/// confirmed engine gap — see `p193_main_decline_does_not_place_self`
/// (ignored) below.
#[test]
fn p193_main_decline_does_not_trash_or_draw() {
    let composite = cost_card("P193-COMPOSITE3", "Composite");
    let anchor = purple_color_anchor("P193-ANCHOR4");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(composite)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-COMPOSITE3"])
        .deck(0, &["P193-COMPOSITE3"])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR4", Some(0));

    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    assert!(
        runner.pending_is_optional(),
        "cost-trash prompt must allow decline"
    );
    let player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("PASS declines the cost");

    assert!(
        runner.pending_selection().is_none(),
        "no further selection after decline"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no card trashed on decline"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "no cards drawn on decline (official Q&A: 'then' tail is cost-gated)"
    );
}

/// **Engine gap — G-DELAY-DISPOSAL-IGNORES-CLAUSE-ABORT.**
///
/// Official Q&A: declining the [Main] cost must ALSO prevent "Then, place
/// this card in the battle area." The DSL correctly aborts the process body
/// (`dsl_clause_aborted`, verified above), but the REAL `play_option_from_hand`
/// disposal pipeline (`Game::play_option_core` step 8 → `Game::dispose_option`,
/// `code/digimon-engine/src/game_actions/options.rs`) determines an Option's
/// end-of-play disposition from a STATIC `OptionSubtype` computed once at
/// play time (`classify_option_modes`, `game_actions/mod.rs`) from the
/// card's effect list — ANY effect with a `delay_trigger` (i.e. any
/// `kind: delay` clause) makes the WHOLE card classify as
/// `OptionSubtype::Delay`. `dispose_option`'s `Delay` branch (line ~1284)
/// unconditionally pushes the in-flight `pending_option` onto the
/// battle area as a `Delayed` permanent — it never consults
/// `Game::dsl_clause_aborted`, so a cost-gated "Then, place this card in the
/// battle area" that P-193's process body correctly skipped still happens
/// via this separate, unconditional disposal path.
///
/// This is DISTINCT from the already-resolved `G-OPTION-PLACE-SELF-AS-DELAY-
/// ON-PLAY-PATH` (which fixed the opposite direction: a Standard-subtype
/// Option's body claiming `pending_option` to BECOME Delay). Here the card
/// IS statically Delay-subtype (P-193 has a real `kind: delay` clause for
/// its `[End of All Turns] <Delay>` body), and the gap is that the
/// automatic Delay-disposal fallback has no way to be told "the [Main] body
/// aborted before reaching its own `place_self_as_delay_option` step, so do
/// NOT park this card — resolve it like an ordinary spent Option instead."
///
/// Fix shape: `dispose_option`'s `OptionSubtype::Delay` arm should check
/// `self.dsl_clause_aborted` (or an equivalent explicit-placement marker)
/// before parking, falling through to ordinary trash disposal when the
/// process body never reached `place_self_as_delay_option` itself.
#[test]
#[ignore = "pending: G-DELAY-DISPOSAL-IGNORES-CLAUSE-ABORT — dispose_option's \
            OptionSubtype::Delay branch (game_actions/options.rs) unconditionally \
            parks the Option on decline, ignoring dsl_clause_aborted; the printed \
            'Then, place this card in the battle area' is NOT actually cost-gated \
            on the real play_option_from_hand path even though the DSL process body \
            correctly skips its own place_self_as_delay_option step"]
fn p193_main_decline_does_not_place_self() {
    let composite = cost_card("P193-COMPOSITE3B", "Composite");
    let anchor = purple_color_anchor("P193-ANCHOR4B");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(composite)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-COMPOSITE3B"])
        .deck(0, &["P193-COMPOSITE3B"])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR4B", Some(0));

    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);
    let player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(player, PASS)
        .expect("PASS declines the cost");

    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "P-193 is NOT placed in the battle area when the cost is declined (official Q&A)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Main]: no-eligible-card gate
// ═══════════════════════════════════════════════════════════════════════════

/// With no [Composite]/[Wicked God] card anywhere in hand, the clause-level
/// `condition: count_gte` gate prevents the [Main] `OptionMain` effect from
/// firing at all — no selection installs, and (critically) no Draw 2 occurs.
/// This is the `condition` gate working as intended for a zero-candidate
/// `select_hand` (which itself installs no prompt at all — see the YAML
/// header comment on `G-OPTIONAL-COST-DECLINE-ABORTS-CLAUSE` zero-candidate
/// interaction).
#[test]
fn p193_main_no_eligible_cost_card_no_draw() {
    let non_eligible = filler("P193-NOELIG");
    let anchor = purple_color_anchor("P193-ANCHOR5");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(non_eligible)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-NOELIG"])
        .deck(0, &["P193-NOELIG"])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR5", Some(0));

    let deck_before = runner.deck_size(0);
    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    if let Some(view) = runner.pending_selection_view() {
        use digimon_engine::action::space::PASS as PASS_ACTION;
        assert!(
            view.valid_action_ids.iter().all(|&a| a == PASS_ACTION),
            "with no eligible cost card, only PASS may be legal; got {:?}",
            view.valid_action_ids
        );
        runner
            .execute_action(view.selecting_player, PASS_ACTION)
            .expect("PASS resolves the empty prompt");
    }

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "no draw occurs when no eligible cost card exists"
    );
}

/// **Engine gap — G-DELAY-DISPOSAL-IGNORES-CLAUSE-ABORT** (see
/// `p193_main_decline_does_not_place_self` for the full gap writeup). With
/// no eligible cost card, the clause-level `condition: count_gte` gate
/// prevents the `OptionMain` effect from running at all — but the card's
/// static `OptionSubtype::Delay` classification (driven by the sibling
/// `kind: delay` clause) still causes `dispose_option` to unconditionally
/// park the Option on the real `play_option_from_hand` path.
#[test]
#[ignore = "pending: G-DELAY-DISPOSAL-IGNORES-CLAUSE-ABORT — see \
            p193_main_decline_does_not_place_self for the full writeup"]
fn p193_main_no_eligible_cost_card_not_placed() {
    let non_eligible = filler("P193-NOELIG2");
    let anchor = purple_color_anchor("P193-ANCHOR5B");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(non_eligible)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-NOELIG2"])
        .deck(0, &["P193-NOELIG2"])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR5B", Some(0));

    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    if let Some(view) = runner.pending_selection_view() {
        use digimon_engine::action::space::PASS as PASS_ACTION;
        runner
            .execute_action(view.selecting_player, PASS_ACTION)
            .expect("PASS resolves the empty prompt");
    }

    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "P-193 is not placed when no eligible cost card exists"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [Main]: event-log assertion for the cost trash
// ═══════════════════════════════════════════════════════════════════════════

/// Accepting the cost fires a Trash event for the cost card (cost-firing
/// clause event-log coverage, test API §5 Section 4).
///
/// **Pre-existing engine gap (also open on BT24-008's identical pattern):**
/// `Game::trash_from_hand_by_index` (`code/digimon-engine/src/game_actions/
/// zones.rs`) moves the card directly (`player.trash.push(card)`) instead of
/// routing through the canonical `Game::trash_card` helper — the latter is
/// documented as "every individual card moving into a trash zone SHALL emit
/// a Trash event" but `trash_from_hand_by_index` bypasses it entirely, so no
/// `GameEvent::Trash` fires for this DSL step. Confirmed still open (not
/// specific to P-193 — the shared verb is missing event emission).
#[test]
#[ignore = "pending: engine-trash-event-from-hand — Game::trash_from_hand_by_index \
            (game_actions/zones.rs) does not route through Game::trash_card and so \
            emits no GameEvent::Trash; same pre-existing gap noted in BT24-008's test"]
fn p193_main_accept_emits_trash_event_for_cost_card() {
    let composite = cost_card("P193-EVT-COST", "Composite");
    let fill = filler("FILL-EVT");
    let anchor = purple_color_anchor("P193-ANCHOR6");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(composite)
        .add_card(fill)
        .add_card(anchor)
        .hand(0, &[CARD_ID, "P193-EVT-COST"])
        .deck(0, &["FILL-EVT"; 3])
        .deck(1, &["FILL-EVT"; 3])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-ANCHOR6", Some(0));

    // P-193 is an Option — `runner.play()` routes through the generic
    // Digimon/Tamer `OnPlay` path, which does NOT dispatch `MainFromHand` /
    // `OptionMain` bodies. Options must go through `play_option_from_hand`
    // (the BT21-097 precedent), which requires the Main phase.
    runner.game.enter_main_phase();
    let _ = runner.game.play_option_from_hand(0, 0);

    let cp = runner.event_checkpoint();
    runner
        .auto_resolve()
        .expect("resolve picks the Composite cost card");

    let events = runner.events_since(cp);
    let trash_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .collect();

    assert!(
        !trash_events.is_empty(),
        "a Trash event must fire when the cost card is trashed from hand: {events:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Delay body: delete Millenniummon → free-play [Wicked God]
// ═══════════════════════════════════════════════════════════════════════════

/// Happy path: P-193 seated as an activatable Delay Option; controller has an
/// own Digimon named "Millenniummon" on the field and a [Wicked God]-trait
/// Digimon card in hand. Activating the Delay deletes the Millenniummon, then
/// offers the free-play pick; accepting plays the Wicked God card without
/// paying its cost.
#[test]
fn p193_delay_deletes_millenniummon_then_plays_wicked_god_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(millenniummon("P193-MM"))
        .add_card(wicked_god_digimon("P193-WG-HAND"))
        .add_card(filler("FILL"))
        .hand(0, &["P193-WG-HAND"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();

    let mm_handle = runner.place_on_field(0, "P193-MM", None);
    let mm_card = runner.top_card(mm_handle);

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(
        runner.game.activate_field_main(0, handle.index as usize),
        "the standard <Delay> [Main] must be activatable after the placing turn"
    );

    // First prompt: optional own-permanent pick (Millenniummon).
    let view = runner
        .pending_selection_view()
        .expect("delay body offers the Millenniummon delete pick");
    assert!(
        runner.pending_is_optional(),
        "the deletion pick must be optional (declinable)"
    );
    let delete_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete Millenniummon delete action");
    runner
        .execute_action(view.selecting_player, delete_action)
        .expect("choose the Millenniummon to delete");

    // Second prompt: optional union-zone (hand/trash) free-play pick.
    let view2 = runner
        .pending_selection_view()
        .expect("after deletion, the free-play union pick must install");
    let play_action = view2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete Wicked God free-play action");
    runner
        .execute_action(view2.selecting_player, play_action)
        .expect("choose the Wicked God card to play free");
    runner.auto_resolve().expect("settle the free play");

    // Millenniummon was deleted (in trash).
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the Millenniummon must be deleted (trashed)"
    );
    // The Wicked God card is now on the field (played free from hand).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-WG-HAND"),
        "the Wicked God card must be played onto the battle area"
    );
    // P-193 itself was trashed as the Delay activation cost.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-193 must be trashed as the <Delay> activation cost"
    );
    // Memory was NOT spent (played "without paying the cost").
    assert_eq!(
        runner.memory(),
        10,
        "the free play must not consume memory"
    );
}

/// The free-play reward also accepts a [Wicked God]-trait card from TRASH
/// (the printed "hand or trash").
#[test]
fn p193_delay_plays_wicked_god_from_trash_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(millenniummon("P193-MM2"))
        .add_card(wicked_god_digimon("P193-WG-TRASH"))
        .add_card(filler("FILL2"))
        .deck(0, &["FILL2"; 6])
        .deck(1, &["FILL2"; 6])
        .memory(10)
        .start();

    let mm_handle = runner.place_on_field(0, "P193-MM2", None);
    let mm_card = runner.top_card(mm_handle);

    // Seed the Wicked God card directly into P0's trash (not hand).
    let wg_data_index = runner
        .game
        .card_data
        .iter()
        .position(|d| d.card_id == "P193-WG-TRASH")
        .expect("Wicked God card data registered");
    let wg_card_index = runner.game.next_card_index();
    runner.game.players[0]
        .trash
        .push(digimon_engine::card_source::CardSource::new(
            wg_data_index,
            0,
            wg_card_index,
        ));

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(runner.game.activate_field_main(0, handle.index as usize));

    let view = runner
        .pending_selection_view()
        .expect("delay body offers the Millenniummon delete pick");
    let delete_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete Millenniummon delete action");
    runner
        .execute_action(view.selecting_player, delete_action)
        .expect("choose the Millenniummon to delete");

    let view2 = runner
        .pending_selection_view()
        .expect("after deletion, the free-play union pick must install");
    let play_action = view2
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete Wicked God free-play action from trash");
    runner
        .execute_action(view2.selecting_player, play_action)
        .expect("choose the trash Wicked God card to play free");
    runner.auto_resolve().expect("settle the free play");

    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == mm_card),
        "the Millenniummon must be deleted"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-WG-TRASH"),
        "the trash-resident Wicked God card must be played onto the battle area"
    );
}

/// The deletion pick is declinable ("may"). Declining leaves Millenniummon on
/// the field and the free-play pick never installs (DCGO: `DeleteSuccessProcess`
/// only runs after a successful delete).
#[test]
fn p193_delay_declining_deletion_skips_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(millenniummon("P193-MM3"))
        .add_card(wicked_god_digimon("P193-WG-DECLINE"))
        .add_card(filler("FILL3"))
        .hand(0, &["P193-WG-DECLINE"])
        .deck(0, &["FILL3"; 6])
        .deck(1, &["FILL3"; 6])
        .memory(10)
        .start();

    let mm_handle = runner.place_on_field(0, "P193-MM3", None);

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(runner.game.activate_field_main(0, handle.index as usize));

    let view = runner
        .pending_selection_view()
        .expect("delay body offers the Millenniummon delete pick");
    assert!(
        runner.pending_is_optional(),
        "the deletion pick must allow decline"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the deletion");

    // No follow-up free-play prompt should install after a decline.
    assert!(
        runner.pending_selection().is_none(),
        "declining the deletion must not surface the free-play prompt"
    );
    // Millenniummon remains on the field.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().handle() == runner.top_card(mm_handle)),
        "the Millenniummon must remain on the field when the deletion is declined"
    );
    // Wicked God card remains in hand (not played).
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P193-WG-DECLINE"),
        "the Wicked God card is never played when the deletion is declined"
    );
    // P-193's activation cost (self-trash) still fires — the <Delay>
    // activation cost is independent of the body's internal decline.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-193's Delay activation cost (self-trash) still applies even when the body declines"
    );
}

/// Negative: with no Digimon named "Millenniummon" on the field, the deletion
/// pick has no eligible target (a non-matching own Digimon does not satisfy
/// the name filter) — activating still trashes P-193 as the Delay cost, but
/// installs no further selection.
#[test]
fn p193_delay_no_millenniummon_on_field_skips_delete_and_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(other_digimon("P193-OTHER"))
        .add_card(filler("FILL4"))
        .deck(0, &["FILL4"; 6])
        .deck(1, &["FILL4"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-OTHER", None);

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(runner.game.activate_field_main(0, handle.index as usize));
    runner.auto_resolve().ok();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-OTHER"),
        "the non-Millenniummon Digimon must remain on the field (name filter rejects)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "P-193 is still trashed as the Delay activation cost"
    );
}

/// Negative: after deleting a Millenniummon, a hand card WITHOUT the [Wicked
/// God] trait is not offered for the free play.
#[test]
fn p193_delay_non_wicked_god_card_not_offered_for_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(millenniummon("P193-MM5"))
        .add_card(non_wicked_god_digimon("P193-PLAIN"))
        .add_card(filler("FILL5"))
        .hand(0, &["P193-PLAIN"])
        .deck(0, &["FILL5"; 6])
        .deck(1, &["FILL5"; 6])
        .memory(10)
        .start();

    runner.place_on_field(0, "P193-MM5", None);

    let handle = seat_and_advance_to_activatable_delay(&mut runner);
    runner.game.set_memory(10);

    assert!(runner.game.activate_field_main(0, handle.index as usize));

    let view = runner
        .pending_selection_view()
        .expect("delay body offers the Millenniummon delete pick");
    let delete_action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a concrete Millenniummon delete action");
    runner
        .execute_action(view.selecting_player, delete_action)
        .expect("choose the Millenniummon to delete");

    // The follow-up union-zone pick installs but with only PASS legal (no
    // [Wicked God] trait card anywhere in hand or trash).
    if let Some(view2) = runner.pending_selection_view() {
        let non_pass_count = view2.valid_action_ids.iter().filter(|&&a| a != PASS).count();
        assert_eq!(
            non_pass_count, 0,
            "no non-Wicked-God card may be offered for the free play; got {:?}",
            view2.valid_action_ids
        );
        runner
            .execute_action(view2.selecting_player, PASS)
            .expect("PASS the empty free-play prompt");
    }

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P193-PLAIN"),
        "the non-Wicked-God card must remain in hand (never played)"
    );
}

/// The <Delay> activation is unavailable on the placing turn (16-16-3).
#[test]
fn p193_delay_not_activatable_on_placing_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(filler("FILL6"))
        .deck(0, &["FILL6"; 3])
        .deck(1, &["FILL6"; 3])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.player_mut(0).battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: runner.game.turn_count,
        };
    runner.game.enter_main_phase();

    assert!(
        !runner
            .game
            .delayed_option_main_activation_available(handle),
        "the <Delay> must NOT be activatable on the placing turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 7 — [Security] (inherited): Activate this card's [Main] effects
// ═══════════════════════════════════════════════════════════════════════════

/// A real attack on the defender's security with P-193 face-up, no eligible
/// cost card in hand: the mandatory-vs-optional shape still resolves without
/// panicking, and since the cost cannot be paid, P-193 does not enter the
/// defender's battle area.
#[test]
fn p193_security_no_eligible_cost_no_panic_and_no_self_placement() {
    let mut attacker = filler("P193-SEC-ATK");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(attacker.clone())
        .add_card(filler("FILL7"))
        .memory(10)
        .deck(0, &["FILL7"; 5])
        .deck(1, &["FILL7"; 5])
        .security(1, &[CARD_ID])
        .start();

    let attacker_handle = runner.place_on_field(0, "P193-SEC-ATK", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: P-193 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // No eligible cost card anywhere in defender's hand → the "then" tail
    // (place self in battle area) cannot fire.
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "P-193 must not enter the battle area when no eligible cost card exists"
    );
}

/// Positive: defender's security reveals P-193, and the defender's hand has
/// an eligible [Composite]/[Wicked God] card. The security clause mirrors the
/// Main body: trashing the cost card draws 2 and places P-193 on the
/// defender's field as a Delay Option.
#[test]
fn p193_security_with_eligible_cost_draws_two_and_places_self() {
    let mut attacker = filler("P193-SEC-ATK2");
    attacker.dp = Some(6000);
    let composite = cost_card("P193-SEC-COST", "Composite");

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-193 must be in embedded pack")
        .add_card(attacker.clone())
        .add_card(composite)
        .memory(10)
        .deck(0, &["P193-SEC-COST"; 2])
        .deck(1, &["P193-SEC-COST"; 6])
        .security(1, &[CARD_ID])
        .start();

    // Give the defender an eligible cost card in hand.
    let hand_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("cost-card seed in deck");
    runner.game.players[1].hand.push(hand_seed);

    let attacker_handle = runner.place_on_field(0, "P193-SEC-ATK2", Some(0));
    let deck_before = runner.deck_size(1);

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.deck_size(1),
        deck_before - 2,
        "defender's deck must shrink by 2 (Draw 2 from the Security-triggered Main body)"
    );
    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("P-193 must be placed on the defender's field");
    assert!(
        matches!(
            placed.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::MainPhaseActivated,
                ..
            }
        ),
        "P-193 must park as a standard <Delay> Option after the Security-triggered Main body"
    );
}
