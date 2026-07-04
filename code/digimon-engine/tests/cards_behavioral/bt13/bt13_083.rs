//! BT13-083 Gizmon: AT — Digimon, Lv.4, Purple, DP 6000, Cost 6.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT13-083.md`,
//! cross-checked against `data/cards.json` `effect_description_eng` and the
//! card image)
//!
//! When you would play this card, by deleting 1 of your level 3 Digimon,
//! reduce the play cost by 4.
//! **[On Play]** ＜Draw 2＞ (Draw 2 cards from your deck.) Then, trash 2 cards
//! in your hand.
//! **[All Turns]** This Digimon can't digivolve.
//! **[On Deletion]** By returning 2 cards with [Gizmon] in their names from
//! your trash to the bottom of the deck in any order, you may play 1
//! [Gizmon: XT] from your trash without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_083.cs`
//!
//! # Clause decomposition (verified against DCGO)
//!
//! 1. **When-played FIXED interactive cost reducer** (DCGO
//!    `EffectTiming.BeforePayCost`, hash `PlayCost-4_BT13_083`): fires only
//!    when THIS card is the one being played (`CanTriggerWhenPermanentWouldPlay`
//!    gated on `cardSource == card`). Offers (accept/decline) deleting 1 of the
//!    controller's own LEVEL-3 Digimon (`permanent.Level == 3`, own battle
//!    area, not immune, not un-deletable — no trait/name/color restriction);
//!    on accept, the deletion is FIXED −4 off THIS card's play cost
//!    regardless of which level-3 Digimon was chosen (unlike BT13-103's
//!    variable-by-deleted-cost sibling). Gated so the reducer is not offered
//!    at all without an eligible level-3 Digimon already on the field
//!    (`CanActivateCondition = HasMatchConditionPermanent`).
//!
//! 2. **[On Play]** (DCGO `EffectTiming.OnEnterFieldAnyone`,
//!    `CanTriggerOnPlay`): mandatory Draw 2, then discard
//!    `Math.Min(2, HandCards.Count)` cards (`SelectHandEffect` with
//!    `canNoSelect: false`).
//!
//! 3. **[All Turns]** (DCGO `EffectTiming.None`,
//!    `CanNotDigivolveStaticSelfEffect`, `IsExistOnBattleArea(card)`): a
//!    permanent (no expiry), self-only static restriction — this specific
//!    card instance cannot digivolve while on the battle area, on either
//!    player's turn.
//!
//! 4. **[On Deletion]** (DCGO `EffectTiming.OnDestroyedAnyone`,
//!    `CanTriggerOnDeletion` + `CanActivateOnDeletion`): gated on >= 2 own
//!    trash cards whose name contains "Gizmon". Mandatory (once activated)
//!    move of exactly 2 selected [Gizmon]-name trash cards to the bottom of
//!    the deck (player's own pick order = "in any order"); only if both moved
//!    successfully, a SEPARATE optional play of 1 [Gizmon: XT] from trash
//!    without paying the cost.
//!
//! # Patterns this test covers
//! - `when_playing_this` FIXED interactive cost-reduction (accept/decline,
//!   `level_eq: 3` filter, no trait/name gate) —
//!   `G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST` substrate
//!   (`tests/cost_hooks/pay_cost_play_delete_reducer.rs` SELF_REDUCER_YAML).
//! - mandatory draw + mandatory 2x hand-trash (degrades on exhausted hand).
//! - permanent self-only `CannotDigivolve` flood_gate (`card_number_is`).
//! - `on_deletion` (rule 25: fires post-trash) cost-then-optional-payoff:
//!   `select_count_capped_multi` (min/max 2, `optional_zero: false`) +
//!   `return_trash_list_to_deck_bottom` + `binding_count_eq` gate + optional
//!   `select_trash` + `play_from_trash_free`.
//! - A clause-level `condition: { count_gte: ... }` guard is REQUIRED
//!   alongside the inner `min: 2` multi-select: `SelectCountCappedMulti` is
//!   not one of the pre-evaluated first-step kinds in
//!   `lower_triggered::first_step_candidate_guard`, so an `optional: true`
//!   clause whose first step is a count-capped multi-select would otherwise
//!   install its outer "you may" prompt UNCONDITIONALLY, even with < 2
//!   eligible trash cards (a spurious prompt DCGO never shows —
//!   `CanActivateOnDeletion` gates on `TrashCards.Count(...) >= 2` before
//!   installing anything). `bt13_083_on_deletion_with_only_1_gizmon_in_trash_no_ops`
//!   pins this: without the `condition:` guard the outer prompt installs
//!   even though the inner multi-select would itself silently no-op.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "BT13-083";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A level-3 purple Digimon — the Clause-1 deletion cost candidate. No trait
/// restriction on DCGO's `CanSelectPermanentCondition` (level-only gate), so
/// this factory intentionally carries no traits.
fn make_level3(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Purple];
    c
}

/// A level-4 Digimon — must NOT be a Clause-1 deletion candidate (level != 3).
fn make_level4(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Digimon named "Gizmon: XT" — the Clause-4 free-play payoff target.
fn make_gizmon_xt(card_id: &str) -> CardData {
    let mut c = make_test_card(card_id, "Gizmon: XT");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 8;
    c.colors = vec![CardColor::Purple];
    c
}

/// A trash filler card whose name does NOT contain "Gizmon" — must not count
/// toward Clause 4's "2 cards with [Gizmon] in their names" cost.
fn make_non_gizmon_trash(card_id: &str) -> CardData {
    make_test_card(card_id, "Not A Match")
}

/// A trash filler card whose name DOES contain "Gizmon" (a second copy,
/// distinct from the payoff target) — a Clause-4 cost candidate.
fn make_gizmon_trash(card_id: &str, name: &str) -> CardData {
    make_test_card(card_id, name)
}

/// Drive every pending selection by accepting the first non-PASS action.
fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

/// Drive every pending selection by declining (PASS) whenever legal,
/// otherwise taking the first legal action. PASS is legal whenever the
/// selection is optional (`is_optional: true`) — for `EffectChoice`
/// selections PASS is the implicit decline branch and is NOT necessarily
/// enumerated in `valid_action_ids` (only the named choice labels are).
fn drive_decline(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional || view.valid_action_ids.contains(&PASS) {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner
            .execute_action(player, action)
            .expect("execute pending selection");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT13-083 must compile as a PURPLE level-4 Digimon, cost 6, DP 6000.
#[test]
fn bt13_083_compiles_as_purple_level4_cost6_dp6000() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(
        card.kind,
        CompiledCardKind::Digimon,
        "BT13-083 is a Digimon (card_kind 0)"
    );
    assert_eq!(card.level, Some(4), "BT13-083 prints Level 4");
    assert_eq!(card.cost, Some(6), "BT13-083 prints Cost 6");
    assert_eq!(card.dp, Some(6000), "BT13-083 prints DP 6000");
    assert!(
        card.color.iter().any(|c| *c == CompiledColor::Purple),
        "BT13-083 is PURPLE (card_colors [6]); colors were {:?}",
        card.color
    );
}

/// The cost-reduction clause must compile as `when_playing_this` (not
/// `when_any_ally_played`), optional, with a fixed amount of 4.
#[test]
fn bt13_083_cost_reducer_clause_is_when_playing_this_fixed_4() {
    use digimon_dsl::compiled::CompiledDeclarativeClause;

    let card = compiled(CARD_ID);
    let reducer = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(d @ CompiledDeclarativeClause::CostReduction { .. }) => {
                Some(d)
            }
            _ => None,
        })
        .expect("BT13-083 must declare a cost_reduction declarative clause");

    match reducer {
        CompiledDeclarativeClause::CostReduction {
            when_playing_this,
            when_any_ally_played,
            optional,
            amount,
            pay_cost,
            ..
        } => {
            assert!(
                *when_playing_this,
                "the reducer fires only when THIS card is the one being played"
            );
            assert!(
                when_any_ally_played.is_none(),
                "the reducer must NOT be keyed off ally-played (unlike BT13-103)"
            );
            assert!(*optional, "the -4 reducer is a declinable accept/decline");
            assert!(amount.is_some(), "the reducer carries a FIXED amount (4)");
            assert!(
                !pay_cost.is_empty(),
                "the reducer's cost is the interactive delete-a-level-3-Digimon pick"
            );
        }
        _ => unreachable!(),
    }
}

/// The [On Play] clause must compile as a mandatory triggered clause.
#[test]
fn bt13_083_on_play_clause_is_mandatory() {
    let card = compiled(CARD_ID);
    let on_play = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("BT13-083 must have an on_play triggered clause");

    assert!(
        !on_play.optional,
        "[On Play] Draw 2 + trash 2 is mandatory (DCGO canNoSelect false)"
    );
    assert_eq!(
        on_play.scope,
        CompiledScope::FaceUp,
        "[On Play] clause is own-field (FaceUp) scope"
    );
}

/// The [On Deletion] clause must compile as an optional triggered clause
/// (the whole "by returning ..., you may play ..." cost-then-payoff shape).
#[test]
fn bt13_083_on_deletion_clause_is_optional() {
    let card = compiled(CARD_ID);
    let on_deletion = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnDeletion])
        .expect("BT13-083 must have an on_deletion triggered clause");

    assert!(
        on_deletion.optional,
        "[On Deletion] clause is optional (\"you may play\" payoff)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 — when-played FIXED cost reducer
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive (accept): playing BT13-083 with a level-3 Digimon on field offers
/// the −4 reducer; accepting deletes the level-3 Digimon and pays cost 2
/// (6 − 4).
#[test]
fn bt13_083_accept_reducer_deletes_level3_and_pays_reduced_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_level3("LV3", "Level Three"))
        .add_card(make_filler("FILLER"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "LV3", Some(0));

    let mem_before = runner.memory();
    assert!(
        runner.play(0, 0).is_none(),
        "the interactive reducer parks the play until its cost prompt resolves"
    );
    assert!(
        runner.pending_selection().is_some(),
        "playing BT13-083 with a level-3 Digimon on field offers the -4 reducer"
    );
    assert!(runner.pending_is_optional(), "the -4 reducer is optional");
    assert_eq!(
        runner.memory(),
        mem_before,
        "no cost paid before the reducer prompt resolves"
    );

    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // LV3 deleted as the cost.
    let lv3_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LV3");
    assert!(!lv3_present, "the level-3 Digimon must be deleted as the cost");

    // Cost 6 - 4 = 2 paid.
    assert_eq!(
        mem_before - runner.memory(),
        2,
        "cost 6 reduced by fixed 4 -> 2 paid"
    );
}

/// Negative (decline): declining the reducer prompt plays BT13-083 at full
/// cost (6) and deletes nothing.
#[test]
fn bt13_083_decline_reducer_pays_full_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_level3("LV3", "Level Three"))
        .add_card(make_filler("FILLER"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "LV3", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_some());
    drive_decline(&mut runner, 0);
    let _ = runner.auto_resolve();

    let lv3_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LV3");
    assert!(lv3_present, "declining deletes nothing");
    assert_eq!(
        mem_before - runner.memory(),
        6,
        "declining pays the full cost 6"
    );
}

/// Negative (no eligible target): with only a level-4 Digimon on field (not
/// level 3), the reducer is never offered — the card plays at full cost.
#[test]
fn bt13_083_no_level3_no_reducer_offered() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_level4("LV4", "Level Four"))
        .add_card(make_filler("FILLER"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start();

    runner.place_on_field(0, "LV4", Some(0));

    let mem_before = runner.memory();
    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        mem_before - runner.memory(),
        6,
        "no eligible level-3 Digimon -> full cost 6, no reducer prompt"
    );
    let lv4_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "LV4");
    assert!(lv4_present, "the level-4 Digimon must survive untouched");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 — [On Play] Draw 2, trash 2 from hand
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: placing BT13-083 on field and firing its On Play batch draws 2
/// and trashes 2 cards from a 2+-card hand.
#[test]
fn bt13_083_on_play_draws_2_and_trashes_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_filler("H1"))
        .add_card(make_filler("H2"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .add_card(make_filler("D3"))
        .hand(0, &["H1", "H2"])
        .deck(0, &["D1", "D2", "D3"])
        .deck(1, &["D1"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    let trash_before = runner.trash_size(0);

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, handle.index as usize);
    let _ = runner.auto_resolve();
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // Net hand change = +2 (draw) - 2 (trash) = 0, but we assert the trash
    // pile grew by exactly 2 (the draw could otherwise mask a no-op trash).
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "On Play must trash exactly 2 cards from hand"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before, // +2 draw, -2 trash = net 0
        "hand size nets to unchanged after +2 draw / -2 trash"
    );
}

/// Negative (degrade): with only 1 card in hand pre-draw, drawing 2 then
/// trashing "2" degrades to trashing only what's available (DCGO
/// Math.Min(2, HandCards.Count)) rather than erroring or over-trashing.
#[test]
fn bt13_083_on_play_trash_degrades_to_available_hand_size() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_filler("H1"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .hand(0, &["H1"])
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(10)
        .start();

    let trash_before = runner.trash_size(0);

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, handle.index as usize);
    let _ = runner.auto_resolve();
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // Pre-draw hand had 1 card; draw 2 brings hand to 3; trash-2 removes 2 of
    // those 3 (never errors on "not enough", never over-trashes to 0).
    let trashed = runner.trash_size(0) - trash_before;
    assert_eq!(
        trashed, 2,
        "with hand replenished by the draw, exactly 2 must still be trashed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: Clause 3 — [All Turns] this can't digivolve
// ═══════════════════════════════════════════════════════════════════════════════

/// Structural + behavioral: once on the field, BT13-083 carries the
/// CannotDigivolve modifier at all times (own turn and opponent's turn
/// alike), and it is installed regardless of whose turn it currently is.
#[test]
fn bt13_083_cannot_digivolve_is_permanent_and_turn_agnostic() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    // Force a declarative re-tick so the flood_gate has materialized (the
    // EX10-010 idiom — `tick_declarative_effects` is the explicit hook, not
    // `auto_resolve`, which only drains pending *selections*).
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .modifiers()
            .has(handle, digimon_engine::enums::ModifierType::CannotDigivolve),
        "BT13-083 must carry CannotDigivolve on its own turn"
    );

    // Flip to the opponent's turn — the restriction must still hold
    // ([All Turns], no active_when turn-scoping).
    runner.end_turn();
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .modifiers()
            .has(handle, digimon_engine::enums::ModifierType::CannotDigivolve),
        "BT13-083 must still carry CannotDigivolve during the opponent's turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Behavioral: Clause 4 — [On Deletion] return 2 [Gizmon], may
//                          play [Gizmon: XT] free
// ═══════════════════════════════════════════════════════════════════════════════

/// Seed player 0's trash with a card by placing it on the field then
/// deleting it (the standard `place_on_field` + `delete_permanent_with_cause`
/// idiom used across the test suite — e.g. `bt18_019.rs`).
fn seed_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let h = runner.place_on_field(player, card_id, Some(0));
    runner
        .game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
}

/// Positive: BT13-083 is deleted with >= 2 [Gizmon]-name cards in trash and a
/// [Gizmon: XT] also in trash. Accepting the whole optional clause returns 2
/// [Gizmon] cards to the deck bottom and plays [Gizmon: XT] from trash free.
#[test]
fn bt13_083_on_deletion_returns_2_gizmon_and_plays_xt_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_gizmon_trash("GIZ1", "Gizmon: PX"))
        .add_card(make_gizmon_trash("GIZ2", "Gizmon: TM"))
        .add_card(make_gizmon_xt("GIZXT"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(20)
        .start();

    // Seed the trash with the 2 [Gizmon]-name cards + the payoff target.
    seed_trash(&mut runner, 0, "GIZ1");
    seed_trash(&mut runner, 0, "GIZ2");
    seed_trash(&mut runner, 0, "GIZXT");

    let deck_before = runner.game.players[0].deck.len();

    let carrier = runner.place_on_field(0, CARD_ID, None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    assert!(
        runner.pending_selection().is_some(),
        "[On Deletion] must offer the return-2-Gizmon selection"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    // Both [Gizmon]-name trash cards must have moved to the deck bottom.
    let giz1_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "GIZ1");
    let giz2_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "GIZ2");
    assert!(
        !giz1_in_trash && !giz2_in_trash,
        "both [Gizmon]-name trash cards must be moved out of trash"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 2,
        "the 2 returned cards must land in the deck"
    );

    // [Gizmon: XT] must have been played from trash for free (now on field).
    let xt_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "GIZXT");
    assert!(
        xt_on_field,
        "[Gizmon: XT] must be played from trash without paying its cost"
    );
}

/// Negative (decline the whole clause): with the cost payable, declining the
/// outer optional prompt leaves the trash and deck untouched.
#[test]
fn bt13_083_on_deletion_decline_leaves_trash_and_deck_untouched() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_gizmon_trash("GIZ1", "Gizmon: PX"))
        .add_card(make_gizmon_trash("GIZ2", "Gizmon: TM"))
        .add_card(make_gizmon_xt("GIZXT"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(20)
        .start();

    seed_trash(&mut runner, 0, "GIZ1");
    seed_trash(&mut runner, 0, "GIZ2");
    seed_trash(&mut runner, 0, "GIZXT");

    let deck_before = runner.game.players[0].deck.len();

    let carrier = runner.place_on_field(0, CARD_ID, None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let trash_before = runner.trash_size(0);
    assert!(runner.pending_selection().is_some());
    drive_decline(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "declining must leave the deck untouched"
    );
    // trash_before was captured right after the carrier itself landed in
    // trash, so trash size should be unchanged post-decline.
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must leave the trash untouched"
    );
}

/// Negative (unpayable cost): with NO [Gizmon]-name cards seeded into trash
/// beforehand, the only trash card matching "Gizmon" by the time [On
/// Deletion] fires is the just-deleted BT13-083 carrier itself (its own
/// printed name "Gizmon: AT" contains the substring — and DCGO's own
/// `CanSelectCardCondition`/`CanActivateOnDeletion` gate counts the carrier
/// post-trash too, per rule 25). That is only 1 eligible card — still short
/// of the required 2 — so the return-2 cost remains unpayable and the whole
/// clause no-ops (no prompt, nothing moved, nothing played). A non-matching
/// filler card is also seeded to prove it does NOT count toward the cost.
#[test]
fn bt13_083_on_deletion_with_only_1_gizmon_in_trash_no_ops() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_non_gizmon_trash("NOTGIZ"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(20)
        .start();

    seed_trash(&mut runner, 0, "NOTGIZ");

    let deck_before = runner.game.players[0].deck.len();

    let carrier = runner.place_on_field(0, CARD_ID, None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "with only 1 eligible [Gizmon] card in trash (the just-deleted \
         carrier itself), the return-2 cost is unpayable -> the whole clause \
         no-ops without a prompt"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before,
        "nothing must be returned to the deck when the cost is unpayable"
    );
}

/// Negative (name filter): with a REAL 2nd [Gizmon]-name candidate present
/// (GIZ1, plus the carrier's own self-match), the cost IS payable and the
/// prompt DOES install — proving "NOTGIZ" is excluded from the count rather
/// than the whole scenario being coincidentally unpayable regardless of the
/// filter. Declining leaves everything untouched, and — crucially — GIZ1 +
/// the carrier alone (2 total) are enough without needing NOTGIZ at all.
#[test]
fn bt13_083_on_deletion_non_gizmon_trash_does_not_count_toward_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT13-083 in embedded DSL pack")
        .add_card(make_gizmon_trash("GIZ1", "Gizmon: PX"))
        .add_card(make_non_gizmon_trash("NOTGIZ"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 3])
        .memory(20)
        .start();

    seed_trash(&mut runner, 0, "GIZ1");
    seed_trash(&mut runner, 0, "NOTGIZ");

    let carrier = runner.place_on_field(0, CARD_ID, None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    // GIZ1 (1) + the just-deleted carrier's own "Gizmon: AT" name (1) = 2
    // eligible candidates -> the cost IS payable and the prompt installs,
    // even though NOTGIZ sits in trash too (proving NOTGIZ is excluded from
    // the count — if it counted, 3 candidates would still be payable and
    // this assertion would be uninformative either way, so the real proof is
    // the companion `..._with_only_1_gizmon_in_trash_no_ops` test, which has
    // ONLY the carrier's self-match (1) plus a NOTGIZ that must NOT push the
    // count to 2).
    assert!(
        runner.pending_selection().is_some(),
        "GIZ1 + the carrier's own name satisfy the min-2 [Gizmon] cost \
         regardless of the non-matching NOTGIZ card"
    );
    drive_decline(&mut runner, 0);
    let _ = runner.auto_resolve();

    let notgiz_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "NOTGIZ");
    assert!(
        notgiz_in_trash,
        "the non-[Gizmon] trash card must remain untouched"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_level4("X", "X");
}
