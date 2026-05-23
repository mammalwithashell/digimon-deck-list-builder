//! BT24-016 Lamiamon — Lv5, Red, Dragonkin / LIBERATOR.
//!
//! # Card text (cards.json / printed)
//!
//! **[Hand] [Main]** If you have [Owen Dreadnought], by placing 1
//! [Dimetromon] from your trash as any of your [Elizamon]'s bottom
//! digivolution card, it digivolves into this card for a digivolution
//! cost of 3, ignoring digivolution requirements.
//!
//! **[When Digivolving] [When Attacking] [Once Per Turn]** Your
//! opponent places 1 card from their hand as the bottom security card.
//! Then, trash their top security card.
//!
//! **Inherited — [All Turns] [Once Per Turn]** When your opponent's
//! security stack is removed from, you may play 1 5000 DP or lower
//! [Reptile] or [Dragonkin] Digimon card from your hand without paying
//! the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Red/BT24_016.cs
//!
//! # Patterns this test covers
//! - Structural: 1 alt_path (standard Digivolve), 3 triggered clauses
//!   (main_from_hand + own OPT + inherited OPT)
//! - Clause 1 [Hand][Main] activated digivolve: Owen Dreadnought `condition:`
//!   gate → select Elizamon → select Dimetromon from trash →
//!   place_as_bottom_source → effect_initiated_digivolve (cost 3)
//! - Clause 2 [WhenDigivolving/WhenAttacking] OPT: as_selecting_player →
//!   select_hand → place_on_security(bottom) → trash_top_security
//! - Clause 3 inherited [All Turns] OnOpponentSecurityRemoved OPT: play
//!   Reptile/Dragonkin (DP <= 5000) from hand free; fires on BOTH turns
//!
//! # Gap status (re-verified against qa/resolved-gaps.md + qa/dsl-vocab-gaps.md
//! on 2026-05-21)
//! - **G-ALT-PATH-CONDITION** — RESOLVED 2026-05-15. `AltPathSpec.condition`
//!   field lands + lowers. The Owen Dreadnought gate is now populated on the
//!   activated_digivolve alt-path. Structural coverage in §2.
//! - **G-OPT-TRIGGERED** — closed (phantom; OPT enforcement always worked).
//! - **G-INHERITED-DISPATCH** — RESOLVED 2026-05-17 (Track D). Inherited
//!   triggered effects dispatch from the digivolution stack.
//! - **G-PRED-DP-LTE** — RESOLVED 2026-05-17. `dp_lte` works on hand-zone
//!   card subjects.
//! - **G-SELECT-EMPTY-OUTER-TAIL** — RESOLVED 2026-05-20. `trash_top_security`
//!   after `as_selecting_player` fires even when the opponent's hand is empty.
//! - **as_selecting_player** — wired
//!   (`code/digimon-engine/src/dsl_cards/step/as_selecting_player.rs`); the
//!   identical shape (as_selecting_player → select_hand → place_on_security →
//!   trash_top_security) is exercised live by BT21-024's behavioral tests.
//!
//! # Clause 1 — `[Hand][Main]` activated digivolve (G-ACTIVATED-DIGIVOLVE-EXECUTION)
//! The `kind: activated_digivolve` alt-path kind has no engine execution route.
//! Clause 1 is instead modelled as a `when: main_from_hand` triggered clause
//! that runs `select Elizamon → select Dimetromon → place_as_bottom_source →
//! effect_initiated_digivolve` — all working engine machinery (the engine masks
//! a Hand [Main] action for any card kind, and `activate_hand_main` runs it).
//! Clause 1 now has full behavioral coverage (§2). See the
//! `unblock-medusamon-tier3-cards` change design.md (D1-REVISED).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

use super::super::dsl_card_data::compiled;

// ── Fixture builders ────────────────────────────────────────────────────────

/// A plain filler card — Digimon, Red, no traits.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// A carrier permanent under which BT24-016 can sit as an inherited source.
fn make_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(6);
    c.dp = Some(8000);
    c
}

/// A Reptile or Dragonkin Digimon with a chosen DP — eligible for the
/// inherited free-play when DP <= 5000.
fn make_trait_digimon(id: &str, trait_name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.traits = vec![trait_name.to_string()];
    c
}

/// A Lv.4 Digimon with an explicit card name (for `name_is` / `name_contains`).
fn make_named_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

/// A Tamer with an explicit card name.
fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c
}

/// Push a registered card into player 0's trash zone.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let next = game.next_card_index();
    game.players[0]
        .trash
        .push(CardSource::new(data_idx, 0, next));
}

/// Insert BT24-016 as the bottom digivolution source under `carrier` for
/// player 0, so the inherited [All Turns] clause is dispatched from the stack.
/// Mirrors the P-189 / BT24-012 inherited-stack test idiom.
fn bury_bt24_016_under(runner: &mut DebugRunner, carrier_index: usize) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT24-016")
        .expect("BT24-016 registered in card_data");
    let next = game.next_card_index();
    let mut src = CardSource::new(data_idx, 0, next);
    src.card_index = next;
    let perm = &mut game.players[0].battle_area[carrier_index as usize];
    perm.card_sources.insert(0, src);
}

/// Helper: borrow the triggered clauses of the compiled card.
fn triggered_clauses(card: &digimon_dsl::compiled::CompiledCard) -> Vec<&CompiledTriggeredClause> {
    card.effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// §1 Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Lamiamon has exactly one alt_path: the standard digivolve (Lv4, cost 3).
/// The `[Hand][Main]` activated digivolve is modelled as a `main_from_hand`
/// triggered clause (clause 1 in `effects`), not an alt-path.
#[test]
fn bt24_016_has_one_standard_alt_path() {
    let card = compiled("BT24-016");
    assert_eq!(
        card.alt_paths.len(),
        1,
        "expected exactly 1 alt_path (standard Digivolve), got {}",
        card.alt_paths.len()
    );
}

#[test]
fn bt24_016_first_alt_path_is_digivolve_lv4_cost3() {
    let card = compiled("BT24-016");
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(
        path.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(3)),
        "standard digivolve path must cost 3"
    );
    assert!(
        path.ignore_requirements == false,
        "standard digivolve path must NOT ignore requirements"
    );
}

/// Clause 1 is a `main_from_hand` triggered clause — the [Hand][Main] activated
/// digivolve — carrying the Owen Dreadnought + Elizamon `condition:` gate.
#[test]
fn bt24_016_has_main_from_hand_activated_digivolve_clause() {
    let card = compiled("BT24-016");
    let clause1 = triggered_clauses(&card)
        .into_iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("clause 1 must be a main_from_hand triggered clause");
    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "clause 1 is a face-up own effect"
    );
    assert!(
        clause1.condition.is_some(),
        "clause 1 must carry a condition (the Owen Dreadnought + Elizamon gate)"
    );
}

/// Card has exactly 3 triggered effects: the `main_from_hand` activated
/// digivolve (clause 1), one own-scope OPT on [WhenDigivolving, WhenAttacking]
/// (clause 2), and one inherited OPT on OnOpponentSecurityRemoved (clause 3).
#[test]
fn bt24_016_has_exactly_three_triggered_clauses() {
    let card = compiled("BT24-016");
    let triggered = triggered_clauses(&card);
    assert_eq!(
        triggered.len(),
        3,
        "expected 3 triggered clauses, got {}: {:?}",
        triggered.len(),
        triggered.iter().map(|t| &t.when).collect::<Vec<_>>()
    );
}

#[test]
fn bt24_016_own_triggered_clause_has_correct_shape() {
    let card = compiled("BT24-016");
    let triggered = triggered_clauses(&card);
    // Clause 2 is the [WhenDigivolving, WhenAttacking] OPT.
    let clause2 = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("clause 2 must be present: [WhenDigivolving, WhenAttacking]");
    assert_eq!(
        clause2.scope,
        CompiledScope::FaceUp,
        "clause 2 is own-scope"
    );
    assert!(clause2.once_per_turn, "clause 2 must be OPT");
    assert!(
        !clause2.optional,
        "clause 2 is NOT optional (opponent is forced to place a card)"
    );
}

#[test]
fn bt24_016_inherited_clause_has_correct_shape() {
    let card = compiled("BT24-016");
    let triggered = triggered_clauses(&card);
    let clause3 = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved))
        .expect("clause 3 must be present: OnOpponentSecurityRemoved");
    assert_eq!(
        clause3.scope,
        CompiledScope::Inherited,
        "clause 3 must be inherited scope"
    );
    assert!(clause3.once_per_turn, "clause 3 must be OPT");
    assert!(clause3.optional, "clause 3 is optional (\"you may play\")");
    // active_when must be set (all_turns: true)
    assert!(
        clause3.active_when.is_some(),
        "clause 3 active_when must be set to all_turns"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// §2 Clause 1 behavioral — [Hand][Main] activated digivolve
//
// Clause 1 is a `main_from_hand` triggered clause: it digivolves Lamiamon (this
// card, from hand) onto an [Elizamon] for cost 3, placing a [Dimetromon] from
// trash as the Elizamon's bottom digivolution card. Driven through
// `activate_hand_main` — the engine API the Hand [Main] action decodes to.
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a runner for clause 1: BT24-016 in P0's hand; Owen Dreadnought + an
/// Elizamon on P0's field; a Dimetromon in P0's trash.
fn clause1_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_named_tamer("OWEN", "Owen Dreadnought"))
        .add_card(make_named_digimon("ELIZAMON", "Elizamon"))
        .add_card(make_named_digimon("DIMETROMON", "Dimetromon"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-016"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWEN", Some(0));
    runner.place_on_field(0, "ELIZAMON", Some(1));
    push_to_trash(&mut runner, "DIMETROMON");
    runner
}

/// Positive: with Owen Dreadnought + an Elizamon on the field and a Dimetromon
/// in trash, activating BT24-016's [Hand][Main] effect digivolves Lamiamon onto
/// the Elizamon and places the Dimetromon as the Elizamon's bottom source.
#[test]
fn bt24_016_hand_main_activated_digivolve() {
    let mut runner = clause1_runner();

    assert!(
        runner.game.activate_hand_main(0, 0),
        "the [Hand][Main] activated-digivolve effect must fire (Owen + Elizamon present)"
    );

    // Step 1: select the Elizamon target.
    let view = runner
        .pending_selection_view()
        .expect("the Elizamon select prompt must install");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the Elizamon");

    // Step 2: select the Dimetromon from trash (the printed cost).
    let view = runner
        .pending_selection_view()
        .expect("the Dimetromon trash-select prompt must install");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the Dimetromon");
    let _ = runner.auto_resolve();

    // Lamiamon digivolved onto the Elizamon permanent.
    let elizamon_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "ELIZAMON")
        })
        .expect("the Elizamon permanent must still be on the field");
    assert_eq!(
        elizamon_perm.top_card().card_id(&runner.game.card_data),
        "BT24-016",
        "Lamiamon must be the top card of the Elizamon stack after the activated digivolve"
    );
    assert!(
        elizamon_perm
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == "DIMETROMON"),
        "the Dimetromon must be placed as a digivolution source under the Elizamon"
    );
    assert!(
        !runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "DIMETROMON"),
        "the Dimetromon must leave the trash"
    );
    assert_eq!(runner.hand_size(0), 0, "Lamiamon must leave the hand");
}

/// Condition gate: without Owen Dreadnought on the field, the [Hand][Main]
/// activated digivolve is not offered — `activate_hand_main` does not fire it.
#[test]
fn bt24_016_hand_main_not_offered_without_owen_dreadnought() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_named_digimon("ELIZAMON", "Elizamon"))
        .add_card(make_named_digimon("DIMETROMON", "Dimetromon"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-016"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    // Elizamon on field, Dimetromon in trash — but NO Owen Dreadnought.
    runner.place_on_field(0, "ELIZAMON", Some(0));
    push_to_trash(&mut runner, "DIMETROMON");

    assert!(
        !runner.game.activate_hand_main(0, 0),
        "without Owen Dreadnought the [Hand][Main] condition fails — the effect must not fire"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection installs when the activated digivolve is not offered"
    );
}

/// Condition gate: without an Elizamon on the field, the [Hand][Main] activated
/// digivolve is not offered — there is no legal digivolve target.
#[test]
fn bt24_016_hand_main_not_offered_without_elizamon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_named_tamer("OWEN", "Owen Dreadnought"))
        .add_card(make_named_digimon("DIMETROMON", "Dimetromon"))
        .add_card(make_filler("FILL"))
        .hand(0, &["BT24-016"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWEN", Some(0));
    push_to_trash(&mut runner, "DIMETROMON");

    assert!(
        !runner.game.activate_hand_main(0, 0),
        "without an Elizamon target the [Hand][Main] condition fails — the effect must not fire"
    );
}

/// Negative structural check: the STANDARD digivolve alt-path (clause 1's
/// sibling) carries NO `condition:` — the Owen gate is specific to the
/// activated path only.
#[test]
fn bt24_016_standard_digivolve_path_has_no_condition() {
    let card = compiled("BT24-016");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("standard Digivolve path must be present");
    assert!(
        path.condition.is_none(),
        "standard digivolve path must NOT carry a condition (Owen gate is activated-path only)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// §3 Clause 2 behavioral — [When Digivolving] / [When Attacking] OPT
//
// Drives the clause through `enqueue_triggered` + `drain_effect_queue` (the
// established Medusamon-archetype idiom — see BT22-015, BT21-025). The
// as_selecting_player → select_hand → place_on_security → trash_top_security
// chain is the same shape proven live by BT21-024's behavioral tests.
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a runner with Lamiamon on player 0's field and an opponent who has
/// `opp_security` security cards and `opp_hand` hand cards.
fn clause2_runner(opp_security: usize, opp_hand: usize) -> (DebugRunner, PermanentHandle) {
    let sec: Vec<&str> = vec!["FILLER"; opp_security];
    let hand: Vec<&str> = vec!["OPP-HAND"; opp_hand];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .add_card(make_filler("OPP-HAND"))
        .security(1, &sec)
        .hand(1, &hand)
        .deck(0, &["FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start();
    let lamiamon = runner.place_on_field(0, "BT24-016", Some(0));
    (runner, lamiamon)
}

fn trigger_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

fn trigger_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Positive: firing [When Digivolving] installs a pending selection on the
/// OPPONENT (they must place a hand card as bottom security).
#[test]
fn bt24_016_when_digivolving_installs_opponent_placement_prompt() {
    let (mut runner, lamiamon) = clause2_runner(3, 1);
    trigger_when_digivolving(&mut runner, lamiamon);

    let pending = runner
        .pending_selection()
        .expect("clause 2 must install a placement prompt on WhenDigivolving");
    assert_eq!(
        pending.selecting_player, 1,
        "the OPPONENT (player 1) must be the selecting player"
    );
}

/// Positive: full resolution — opponent places a hand card as bottom security,
/// then their top security is trashed. Net security count unchanged (+1 bottom,
/// -1 top); opponent loses 1 hand card; opponent trash grows by 1.
#[test]
fn bt24_016_when_digivolving_opponent_places_and_top_security_trashed() {
    let (mut runner, lamiamon) = clause2_runner(3, 1);

    let opp_hand_before = runner.hand_size(1);
    let opp_sec_before = runner.security_count(1);
    let opp_trash_before = runner.trash_size(1);

    trigger_when_digivolving(&mut runner, lamiamon);

    // Opponent resolves the placement selection.
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "no further pending selections after the placement resolves"
    );
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before - 1,
        "opponent hand must shrink by 1 (placed a card as bottom security)"
    );
    assert_eq!(
        runner.security_count(1),
        opp_sec_before,
        "net opponent security unchanged (added bottom, trashed top)"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "opponent trash must grow by 1 (top security trashed)"
    );
}

/// Positive: the same clause fires on [When Attacking] (the second printed
/// timing). The opponent is prompted to place a card.
#[test]
fn bt24_016_when_attacking_installs_opponent_placement_prompt() {
    let (mut runner, lamiamon) = clause2_runner(3, 1);
    trigger_when_attacking(&mut runner, lamiamon);

    let pending = runner
        .pending_selection()
        .expect("clause 2 must install a placement prompt on WhenAttacking");
    assert_eq!(pending.selecting_player, 1, "opponent (player 1) selects");
}

/// Negative / empty-hand path: when the opponent has NO hand cards, no
/// placement prompt installs, but the printed "trash their top security card"
/// still fires (G-SELECT-EMPTY-OUTER-TAIL RESOLVED 2026-05-20).
#[test]
fn bt24_016_when_digivolving_empty_opponent_hand_still_trashes_top_security() {
    let (mut runner, lamiamon) = clause2_runner(3, 0);

    let opp_sec_before = runner.security_count(1);
    let opp_trash_before = runner.trash_size(1);

    trigger_when_digivolving(&mut runner, lamiamon);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(1),
        opp_sec_before - 1,
        "with an empty opponent hand, only the top-security trash fires (-1 net)"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "top security must still be trashed even when opponent hand is empty"
    );
}

/// OPT lockout: clause 2 must not fire a second time the same turn even if
/// WhenDigivolving and WhenAttacking both trigger. After end_turn → back to
/// player 0, the lockout has cleared.
#[test]
fn bt24_016_clause2_opt_fires_at_most_once_per_turn() {
    let (mut runner, lamiamon) = clause2_runner(4, 2);

    // First trigger — installs the prompt.
    trigger_when_digivolving(&mut runner, lamiamon);
    assert!(
        runner.pending_selection().is_some(),
        "first clause-2 trigger must install the placement prompt"
    );
    let _ = runner.auto_resolve();

    // Second trigger SAME turn (the other timing) — OPT must block it.
    trigger_when_attacking(&mut runner, lamiamon);
    assert!(
        runner.pending_selection().is_none(),
        "OPT must block the second clause-2 activation in the same turn"
    );

    // Cycle turns back to player 0; the OPT lockout clears.
    runner.end_turn();
    if runner.game_over() {
        return;
    }
    runner.end_turn();
    if runner.game_over() {
        return;
    }

    let lamiamon_after = runner.perm_handle(0, 0);
    trigger_when_attacking(&mut runner, lamiamon_after);
    assert!(
        runner.pending_selection().is_some(),
        "OPT lockout must clear after a full turn cycle"
    );
}

/// Side-effect / event-log assertion for clause 2's security trash.
///
/// The `GameEvent::Trash` / `GameEvent::SecurityReveal` variants are defined
/// but "not emitted yet" (see `events.rs`) — only `MemoryChange`, `TurnStart`,
/// `PhaseChange`, `Play`, `Digivolve`, `GameOver` are reliably emitted. The
/// security trash therefore produces no `GameEvent`. This test instead pins
/// the observable side-effect (the opponent's top security moved to trash)
/// recorded over a checkpoint window — the regression guard rule-6 asks for,
/// without asserting an un-emitted event variant.
#[test]
fn bt24_016_clause2_top_security_trash_recorded_over_checkpoint() {
    let (mut runner, lamiamon) = clause2_runner(3, 1);

    let opp_trash_before = runner.trash_size(1);
    // event_checkpoint() captures the event-log cursor — kept as the rule-6
    // checkpoint anchor even though the security-trash side-effect is not yet
    // surfaced as a discrete GameEvent variant.
    let cp = runner.event_checkpoint();

    trigger_when_digivolving(&mut runner, lamiamon);
    let _ = runner.auto_resolve();

    // The security-removal side-effect is observable in game state: the
    // opponent's trash grew by exactly 1 (their trashed top security card).
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "clause 2 must trash the opponent's top security card"
    );
    // The event-log cursor is monotonic — `events_since` returns a valid
    // (possibly empty) slice; trash events are not yet emitted by the engine.
    let _ = runner.events_since(cp);
}

// ═══════════════════════════════════════════════════════════════════════════════
// §4 Clause 3 behavioral — Inherited [All Turns] [OPT] OnOpponentSecurityRemoved
//
// Drives the inherited clause through the P-189 / BT24-012 idiom: BT24-016 is
// buried as the bottom source under a carrier permanent, then the opponent's
// security is removed by an attack — firing OnOpponentSecurityRemoved.
// ═══════════════════════════════════════════════════════════════════════════════

/// Build a runner where BT24-016 is an inherited source under a carrier on
/// player 0's field, the opponent has 1 security card, and player 0 holds the
/// `hand_card` in hand.
fn clause3_runner(hand_card: CardData) -> (DebugRunner, PermanentHandle) {
    let hand_id = hand_card.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_carrier("CARRIER"))
        .add_card(make_filler("SEC-CARD"))
        .add_card(make_filler("FILL"))
        .add_card(hand_card)
        .hand(0, &[hand_id.as_str()])
        .security(1, &["SEC-CARD"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    bury_bt24_016_under(&mut runner, carrier.index as usize);
    (runner, carrier)
}

/// Positive: when the opponent's security is removed on the controller's turn
/// and a qualifying Reptile Digimon (DP <= 5000) is in hand, the controller may
/// play it free — accepting the optional prompt plays the card.
#[test]
fn bt24_016_inherited_on_security_removed_play_free_positive() {
    let (mut runner, carrier) = clause3_runner(make_trait_digimon("REPTILE-3K", "Reptile", 3000));

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the qualifying Reptile Digimon must be played to the field free"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the played card must leave hand"
    );
}

/// Negative (target filter): a Dragonkin Digimon with DP > 5000 must NOT be a
/// legal free-play target. With only a 7000-DP Dragonkin in hand, no card is
/// played (the optional prompt either does not install or only offers PASS).
#[test]
fn bt24_016_inherited_high_dp_digimon_not_a_legal_target() {
    let (mut runner, carrier) =
        clause3_runner(make_trait_digimon("DRAGONKIN-7K", "Dragonkin", 7000));

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "a DP>5000 Dragonkin must NOT be playable via the inherited free-play"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand must be unchanged — no eligible target"
    );
}

/// Negative (trait filter): a non-Reptile / non-Dragonkin Digimon (DP <= 5000)
/// must NOT be a legal free-play target.
#[test]
fn bt24_016_inherited_non_matching_trait_digimon_not_a_legal_target() {
    let (mut runner, carrier) = clause3_runner(make_trait_digimon("CYBORG-3K", "Cyborg", 3000));

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "a non-Reptile/non-Dragonkin Digimon must NOT be a legal free-play target"
    );
    assert_eq!(runner.hand_size(0), hand_before, "hand must be unchanged");
}

/// Optional decline: the inherited clause is "you may play" — declining the
/// prompt (PASS) plays nothing even with a qualifying card in hand.
#[test]
fn bt24_016_inherited_decline_plays_nothing() {
    let (mut runner, carrier) = clause3_runner(make_trait_digimon("REPTILE-3K", "Reptile", 3000));

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);

    let sel = runner
        .pending_selection()
        .expect("optional free-play prompt must install with a qualifying card in hand");
    assert!(
        sel.is_optional,
        "the inherited \"you may play\" prompt must be optional (PASS legal)"
    );
    let selecting = sel.selecting_player;
    runner
        .execute_action(selecting, PASS)
        .expect("declining the optional clause must resolve cleanly");

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "declining must play nothing"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must leave hand unchanged"
    );
}

/// [All Turns] verification (G-ALL-TURNS-FILTER): the inherited clause is
/// `[All Turns]` — it must ALSO fire on the OPPONENT's turn, not only the
/// controller's. A `[Your Turn]` clause (e.g. P-189's inherited) would be
/// gated by `active_when: { your_turn: true }` and would NOT fire here.
///
/// BT24-016 is buried under player 0's carrier. We advance to player 1's
/// turn, then drive an `OnOpponentSecurityRemoved` event via a
/// `TriggerSource::SecurityRemoved` constructed exactly as the engine's
/// combat fire-site builds it (`combat.rs::SecurityRemoved`): the affected
/// player is player 1 (their security stack is removed from), the observer
/// is player 0 (BT24-016's controller). On player 1's turn the [All Turns]
/// inherited clause must still dispatch.
#[test]
fn bt24_016_inherited_all_turns_fires_on_opponents_turn() {
    let (mut runner, _carrier) = clause3_runner(make_trait_digimon("REPTILE-3K", "Reptile", 3000));

    // Advance to the opponent's (player 1's) turn.
    runner.end_turn();
    if runner.game_over() {
        return;
    }
    assert_eq!(
        runner.game.turn_player(),
        1,
        "test precondition: it must be the opponent's turn"
    );

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Use a real card handle from player 1's security stack as the removed
    // card — `SecurityRemoved` carries the removed card as its event subject.
    let removed_card = runner.game.players[1]
        .security
        .first()
        .expect("opponent must have a security card")
        .handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: removed_card,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "[All Turns] inherited clause must fire on the OPPONENT's turn too"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the qualifying card must be played free on the opponent's turn"
    );
}

/// OPT lockout (inherited clause): two security removals in the same turn
/// fire the inherited clause at most once. After a full turn cycle the
/// lockout clears.
#[test]
fn bt24_016_inherited_clause_opt_fires_at_most_once_per_turn() {
    // Two security cards on the opponent so two removals are possible.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_carrier("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("FILL"))
        .add_card(make_trait_digimon("REPTILE-A", "Reptile", 3000))
        .add_card(make_trait_digimon("REPTILE-B", "Reptile", 3000))
        .hand(0, &["REPTILE-A", "REPTILE-B"])
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(8)
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    bury_bt24_016_under(&mut runner, carrier.index as usize);

    let field_before = runner.battle_area_size(0);

    // First security removal — the inherited clause fires.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let field_after_first = runner.battle_area_size(0);
    assert_eq!(
        field_after_first,
        field_before + 1,
        "first security removal must fire the inherited clause"
    );

    if runner.game_over() {
        return;
    }

    // Second security removal SAME turn — OPT must block the clause.
    let carrier2 = runner.perm_handle(0, 0);
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.battle_area_size(0),
        field_after_first,
        "OPT must block the second inherited activation in the same turn"
    );
}

/// OPT lockout clears after a full turn cycle.
#[test]
fn bt24_016_inherited_clause_opt_clears_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_carrier("CARRIER"))
        .add_card(make_filler("SEC-1"))
        .add_card(make_filler("SEC-2"))
        .add_card(make_filler("FILL"))
        .add_card(make_trait_digimon("REPTILE-A", "Reptile", 3000))
        .add_card(make_trait_digimon("REPTILE-B", "Reptile", 3000))
        .hand(0, &["REPTILE-A", "REPTILE-B"])
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(8)
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    bury_bt24_016_under(&mut runner, carrier.index as usize);

    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    let field_after_first = runner.battle_area_size(0);
    assert_eq!(
        field_after_first,
        field_before + 1,
        "first inherited firing must play a card"
    );

    if runner.game_over() {
        return;
    }

    // Cycle: end player 0's turn, then player 1's turn back to player 0.
    runner.end_turn();
    if runner.game_over() {
        return;
    }
    runner.end_turn();
    if runner.game_over() {
        return;
    }

    let carrier_after = runner.perm_handle(0, 0);
    let field_before_second = runner.battle_area_size(0);
    runner.attack_player(carrier_after, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.battle_area_size(0),
        field_before_second + 1,
        "OPT lockout must clear after a full turn cycle — clause fires again"
    );
}

/// The inherited clause must NOT fire when BT24-016 is no longer a
/// digivolution source (it is not in any stack). With no carrier hosting
/// BT24-016, removing the opponent's security plays nothing.
#[test]
fn bt24_016_inherited_does_not_fire_when_not_in_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 in embedded DSL pack")
        .add_card(make_carrier("CARRIER"))
        .add_card(make_filler("SEC-CARD"))
        .add_card(make_filler("FILL"))
        .add_card(make_trait_digimon("REPTILE-3K", "Reptile", 3000))
        .hand(0, &["REPTILE-3K"])
        .security(1, &["SEC-CARD"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    // Place a bare carrier on player 0's field WITHOUT burying BT24-016 under it.
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "inherited clause must not fire — BT24-016 is not a digivolution source"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand unchanged — inherited clause did not fire"
    );
}
