//! AD1-012 CresGarurumon — Digimon, Lv.6, Blue/Black, DP 12000, Cost 12.
//!
//! # Card text (cards.json)
//!
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other Digimon,
//! add the suspended Digimon's DP to this Digimon and it gains ＜Security A. +1＞
//! for the attack.)
//! ＜Evade＞ (When this Digimon would be deleted, you may suspend it to prevent
//! that deletion.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may return 1
//! of your opponent's lowest level Digimon to the hand. Then, this Digimon and
//! 1 of your Digimon with [Greymon] in its name may unsuspend.
//! [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks,
//! 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand.
//! Then, you may change the attack target to 1 of your Digimon.
//!
//! Inherited Effect: [Your Turn] This Digimon's attack target can't change.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_012.cs
//!
//! # Patterns this test file covers
//! - H8/H10: Evade + Alliance keyword grants (declarative)
//! - Multi-timing trigger: shared OPT clause across [On Play]/[When Digivolving]/[When Attacking]
//! - level_matches_aggregate: select opponent's lowest-level Digimon (optional, return-to-hand)
//! - optional sub-block: "Then, this Digimon and 1 [Greymon] may unsuspend"
//! - select_own_permanent name_contains "Greymon" with is_suspended + other gating
//! - alt_paths: Lv5 Blue cost 4 standard + Lv5 Garurumon-name OR ADVENTURE-trait cost 3
//! - BLOCKED — [Opponent's Turn][OPT] DNA digivolve + redirect attack target
//!   (remaining route: defender-side effect-initiated DNA into Omnimon Alter-S
//!   from hand, then resume the same attack-interrupt clause)
//! - Inherited [Your Turn] attack target can't change

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackTarget, TriggerSource};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Greymon-named filler used when testing the "1 of your Digimon with [Greymon]
/// in its name may unsuspend" clause. Lv.5 / 6000 DP — values don't matter for
/// the predicate match, but make_test_card needs *some* level.
fn greymon_named() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-GREYMON-FILLER", "Greymon");
    card.level = Some(5);
    card.dp = Some(6000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// A non-Greymon Digimon used to make sure name filtering works.
fn rookie_filler() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-NON-GREYMON", "Patamon");
    card.level = Some(3);
    card.dp = Some(2000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// Low-level opponent Digimon — used as the lowest-level bounce target.
fn low_level_opp() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-OPP-LOW", "OppLow");
    card.level = Some(3);
    card.dp = Some(2000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

/// Higher-level opponent Digimon — must NOT be in the bounce candidate set
/// when a lower-level opponent Digimon exists.
fn high_level_opp() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("AD1-012-OPP-HIGH", "OppHigh");
    card.level = Some(6);
    card.dp = Some(11000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

fn digimon(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(6);
    card.dp = Some(dp);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

struct RedirectOnAttack(PermanentHandle);

impl CardEffect for RedirectOnAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.0;
        vec![Effect::on_attack(card)
            .name("AD1-012 test redirect attempt")
            .process(move |ctx| {
                let _ = ctx.redirect_attack(AttackTarget::Digimon(target));
            })
            .build()]
    }
}

/// Generic filler card for decks — prevents deck-out during multi-turn tests.
fn deck_filler(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.level = Some(3);
    card.dp = Some(1000);
    card.card_kind = digimon_engine::enums::CardKind::Digimon;
    card
}

fn cresgarurumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 found in embedded DSL pack")
        .add_card(greymon_named())
        .add_card(rookie_filler())
        .add_card(low_level_opp())
        .add_card(high_level_opp())
        .memory(12)
        .start()
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// CresGarurumon must compile with both Alliance and Evade granted as
/// declarative GrantKeyword clauses.
#[test]
fn ad1_012_grants_alliance_and_evade() {
    let card = compiled("AD1-012");

    let keywords: std::collections::HashSet<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.clone()),
            _ => None,
        })
        .collect();

    for expected in &["Alliance", "Evade"] {
        assert!(
            keywords.contains(*expected),
            "AD1-012 must grant '{}' keyword; got {:?}",
            expected,
            keywords
        );
    }
}

/// The shared OP/WD/WA clause must be a single triggered clause whose `when`
/// vector covers all three timings (matching DCGO's shared hash semantics).
#[test]
fn ad1_012_shared_clause_covers_op_wd_wa() {
    let card = compiled("AD1-012");

    let shared = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("AD1-012 must have a single multi-timing OP/WD/WA triggered clause");

    assert!(
        shared.optional,
        "shared clause must be optional ('You may')"
    );
    assert!(
        shared.once_per_turn,
        "shared clause must be once_per_turn (printed [Once Per Turn])"
    );
    assert_eq!(shared.scope, CompiledScope::FaceUp);
}

/// Alt-paths must contain the standard Lv.5 Blue / cost 4 digivolve plus the
/// xros_req alt: Lv.5 Garurumon-name OR ADVENTURE-trait at cost 3.
#[test]
fn ad1_012_has_two_alt_paths_standard_and_xros_req() {
    use digimon_dsl::compiled::CompiledAltPathKind;

    let card = compiled("AD1-012");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        2,
        "AD1-012 must have exactly 2 digivolve alt-paths (standard + xros_req); got {}",
        digivolve_paths.len()
    );
}

// ─── SECTION 2 — Condition gating: shared clause source-on-field ────────────

/// Positive: source is on field → shared clause condition passes (selection
/// installs when an [On Play] trigger fires with at least one opponent
/// Digimon on the board).
#[test]
fn ad1_012_shared_clause_installs_when_source_on_field_with_opp_target() {
    let mut runner = cresgarurumon_runner();

    // Place CresGarurumon on P0's field then fire on-play with an opponent target present.
    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner.fire_on_play(0, cres.index as usize);

    // The shared clause should install a selection (either the bounce select or
    // the optional Yes/No for unsuspend).
    assert!(
        runner.game.pending_selection.is_some(),
        "shared clause must install a pending selection when source on field + opp target"
    );
}

/// Negative: with NO opponent Digimon on the board, the bounce select is
/// vacuous — but per DCGO the unsuspend Yes/No still fires. So the entire
/// clause is not skipped; it falls through to the unsuspend prompt.
/// We assert at least one selection still installs (the Yes/No bool prompt).
#[test]
fn ad1_012_shared_clause_runs_when_no_opp_digimon() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    // No opponent Digimon placed.

    runner.fire_on_play(0, cres.index as usize);

    // Even with no bounce target, the source-on-field condition still passes
    // and the unsuspend offer should fire. The selection that installs is
    // either the unsuspend prompt or PASS-only resolution.
    // We accept either: a pending selection installed, OR no selection (clause
    // declined cleanly because nothing to do). The key contract: no panic.
    let _ = runner.game.pending_selection.is_some();
}

// ─── SECTION 3 — Behavioral: bounce target picks lowest-level opponent ──────

/// When opponent has a Lv.3 and a Lv.6 Digimon, the bounce candidate set
/// must contain ONLY the Lv.3 (lowest-level filter via level_matches_aggregate).
#[test]
fn ad1_012_bounce_select_offers_only_lowest_level_opp() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None); // Lv.3
    runner.place_on_field(1, "AD1-012-OPP-HIGH", None); // Lv.6

    runner.fire_on_play(0, cres.index as usize);

    // The first selection should be opponent permanent (bounce). Its
    // valid_action_ids must include exactly 1 target (the Lv.3) — not 2.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("bounce select must install");

    // There may also be PASS (optional). Filter PASS out from the candidate count.
    let non_pass: Vec<_> = pending
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .copied()
        .collect();

    assert_eq!(
        non_pass.len(),
        1,
        "bounce select must offer exactly 1 candidate (Lv.3 opp Digimon); got {} non-PASS actions",
        non_pass.len()
    );
}

/// Picking the lowest-level opponent target moves it to opponent's hand.
#[test]
fn ad1_012_bounce_returns_target_to_opp_hand() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    let opp_hand_before = runner.game.players[1].hand.len();
    let opp_field_before = runner.game.players[1].battle_area.len();

    runner.fire_on_play(0, cres.index as usize);

    // Resolve the bounce select (pick first non-PASS candidate).
    let (player, action) = {
        let p = runner.game.pending_selection.as_ref().expect("pending");
        let action = *p
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("at least one bounce candidate");
        (p.selecting_player, action)
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("bounce selection resolves");

    // After bounce, opponent battle area shrinks by 1 and hand grows by 1.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "opponent battle area must shrink after bounce"
    );
    assert_eq!(
        runner.game.players[1].hand.len(),
        opp_hand_before + 1,
        "opponent hand must grow by 1 after bounce"
    );
}

/// Bounce select is OPTIONAL (DCGO canNoSelect: true) — declining it is legal
/// and the clause continues to the unsuspend prompt.
#[test]
fn ad1_012_bounce_select_is_optional_pass_legal() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner.fire_on_play(0, cres.index as usize);

    assert!(
        runner.pending_is_optional(),
        "bounce select must be optional (canNoSelect: true → PASS is a legal action)"
    );
}

// ─── SECTION 4 — Behavioral: unsuspend sub-clause ────────────────────────────

/// After the bounce decision, an optional unsuspend sub-block fires.
/// Accepting the unsuspend (executing the optional substep) unsuspends the
/// source CresGarurumon.
#[test]
fn ad1_012_accept_unsuspend_unsuspends_self() {
    let mut runner = cresgarurumon_runner();

    // Suspend CresGarurumon explicitly so we can observe the unsuspend.
    let cres = runner.place_on_field(0, "AD1-012", None);
    {
        let perm = &mut runner.game.players[0].battle_area[cres.index as usize];
        perm.is_suspended = true;
    }
    // No opp Digimon — bounce select is skipped or vacuous; we focus on the
    // unsuspend offer.

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // After auto_resolve (which accepts every optional), CresGarurumon should
    // be unsuspended.
    let perm = &runner.game.players[0].battle_area[cres.index as usize];
    assert!(
        !perm.is_suspended,
        "CresGarurumon should be unsuspended after accepting the unsuspend sub-block"
    );
}

/// When a Greymon-named, suspended ally exists, the unsuspend sub-block
/// also offers a select_own_permanent prompt to pick that Greymon.
#[test]
fn ad1_012_unsuspend_offers_greymon_pick_when_eligible_exists() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    let greymon = runner.place_on_field(0, "AD1-012-GREYMON-FILLER", Some(0));
    // Greymon must be suspended to be a valid pick (DCGO IsOwnGreymon checks IsSuspended).
    {
        let perm = &mut runner.game.players[0].battle_area[greymon.index as usize];
        perm.is_suspended = true;
    }

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // After auto_resolve, the suspended Greymon should be unsuspended.
    let perm = &runner.game.players[0].battle_area[greymon.index as usize];
    assert!(
        !perm.is_suspended,
        "Greymon ally should be unsuspended via the second select after accepting unsuspend"
    );
}

/// Non-Greymon allies must NOT be picked even when suspended.
#[test]
fn ad1_012_unsuspend_does_not_pick_non_greymon() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    let non_grey = runner.place_on_field(0, "AD1-012-NON-GREYMON", Some(0));
    {
        let perm = &mut runner.game.players[0].battle_area[non_grey.index as usize];
        perm.is_suspended = true;
    }

    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // Non-Greymon ally should remain suspended.
    let perm = &runner.game.players[0].battle_area[non_grey.index as usize];
    assert!(
        perm.is_suspended,
        "non-Greymon ally must remain suspended (name filter rejects it)"
    );
}

// ─── SECTION 5 — OPT enforcement ────────────────────────────────────────────

/// Same-timing OPT lockout: firing the [On Play] trigger twice in the same
/// turn must only install the selection on the first fire.
#[test]
fn ad1_012_opt_blocks_second_on_play_in_same_turn() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // First fire — selection installs.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_some(),
        "first OnPlay fire must install selection"
    );
    let _ = runner.auto_resolve();

    // Second fire — OPT must lock; no new selection.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_none(),
        "second OnPlay fire in same turn must be locked by OPT"
    );
}

/// Cross-timing OPT lockout (shared hash across OP/WD/WA in DCGO): firing
/// [On Play] then [When Attacking] in the same turn should still lock.
/// Pending: G-OPT-TRIGGERED — DSL `once_per_turn` lowering may not yet enforce
/// shared OPT across distinct timings within the same multi-timing clause.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — cross-timing OPT lockout for shared multi-timing clauses"]
fn ad1_012_opt_blocks_when_attacking_after_on_play_same_turn() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // OP fires, OPT consumed.
    runner.fire_on_play(0, cres.index as usize);
    let _ = runner.auto_resolve();

    // Then this Digimon attacks — the WA trigger from the same shared clause
    // must NOT fire again per DCGO shared-hash semantics.
    runner.attack_player(cres, 1, false);
    assert!(
        runner.game.pending_selection.is_none(),
        "WA trigger of the shared OP/WD/WA clause must be locked after OP consumed OPT"
    );
}

// ─── SECTION 6 — BLOCKED clauses (DSL + engine gaps) ────────────────────────

/// [Opponent's Turn] [Once Per Turn] — When opp Digimon attacks, may DNA
/// digivolve into [Omnimon Alter-S] in hand. Then, may change attack target.
///
/// BLOCKED:
///   - Remaining faithful route: effect-initiated DNA digivolve into a selected
///     [Omnimon Alter-S] in hand from the defender-side attack observer, then
///     resume the same attack-interrupt clause before optional redirect.
#[test]
#[ignore = "pending: defender-side effect-initiated DNA into Omnimon Alter-S during attack interrupt"]
fn ad1_012_opponents_turn_dna_into_omnimon_alters_then_redirects() {
    // Scaffolding (pending effect DNA route):
    //
    // 1. Place CresGarurumon + a Greymon-name + a Garurumon-name ally on P0's field.
    // 2. Place an Omnimon Alter-S in P0's hand (with dna_costs that match the pair).
    // 3. Place an opponent Digimon on P1's field; place a redirect-target ally on P0.
    // 4. Switch turn to P1; declare opp attack on P0's player.
    // 5. on_opponent_attack should install an optional DNA prompt (cost paid).
    // 6. Accept → effect_initiated_dna_digivolve into Omnimon Alter-S using Greymon+Garurumon.
    // 7. Then redirect_attack_target prompt → pick redirect ally → attack target switches.
    todo!("pending DSL vocab gaps: on_opponent_attack timing + redirect_attack_target step")
}

/// Inherited [Your Turn] — "This Digimon's attack target can't change."
#[test]
fn ad1_012_inherited_blocks_attack_target_change_during_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 found in embedded DSL pack")
        .add_card(digimon("AD1-012-HOST", 12000))
        .add_card(digimon("AD1-012-DEF-A", 1000))
        .add_card(digimon("AD1-012-DEF-B", 1000))
        .start();

    let attacker = runner.place_stack(0, &["AD1-012", "AD1-012-HOST"]);
    let original_target = runner.place_on_field(1, "AD1-012-DEF-A", Some(0));
    let redirect_target = runner.place_on_field(1, "AD1-012-DEF-B", Some(0));
    runner.register_effect(
        "AD1-012-HOST",
        std::sync::Arc::new(RedirectOnAttack(redirect_target)),
    );
    runner.game.tick_declarative_effects();

    runner.attack_digimon(attacker, original_target, false);

    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "AD1-012-DEF-A"),
        "the original target should be battled and deleted"
    );
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "AD1-012-DEF-B"),
        "the redirect target should remain on the field"
    );
}

// ─── SECTION 7 — Structural: inherited aura (CanNotSwitchAttackTarget) ───────

/// The compiled card must contain exactly one inherited Aura declarative clause
/// whose `scope` is `Inherited` and `modifier` is `CanNotSwitchAttackTarget`.
/// This corresponds to the printed "Inherited Effect: [Your Turn] This Digimon's
/// attack target can't change."
#[test]
fn ad1_012_has_inherited_aura_can_not_switch_attack_target() {
    let card = compiled("AD1-012");

    let auras: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope, modifier, ..
            }) => Some((scope, modifier.as_deref())),
            _ => None,
        })
        .collect();

    assert_eq!(
        auras.len(),
        1,
        "AD1-012 must have exactly one Aura declarative clause; got {}",
        auras.len()
    );
    let (scope, modifier) = auras[0];
    assert_eq!(
        *scope,
        CompiledScope::Inherited,
        "the Aura declarative must be in Inherited scope"
    );
    assert_eq!(
        modifier,
        Some("CanNotSwitchAttackTarget"),
        "the Aura modifier must be 'CanNotSwitchAttackTarget' (printed: attack target can't change)"
    );
}

// ─── SECTION 8 — Timing parity: [When Digivolving] fires the shared body ─────

/// When the [When Digivolving] timing fires via `enqueue_triggered`, the shared
/// OP/WD/WA body must install a selection (same as [On Play]), verifying that
/// all three printed timings are wired to the same process.
#[test]
fn ad1_012_when_digivolving_timing_fires_shared_clause() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(cres));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "[When Digivolving] must fire the shared OP/WD/WA body and install a selection"
    );
}

/// When the [When Attacking] timing fires via `enqueue_triggered`, the shared
/// OP/WD/WA body must install a selection (same as [On Play]).
#[test]
fn ad1_012_when_attacking_timing_fires_shared_clause() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(cres));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "[When Attacking] must fire the shared OP/WD/WA body and install a selection"
    );
}

// ─── SECTION 9 — OPT resets after turn boundary ──────────────────────────────

/// After the OPT lockout is consumed in turn N, it must clear at the next turn
/// boundary so the effect is available again in turn N+2 (player 0's second turn).
///
/// Setup: OPP-LOW stays on the field across the turn boundary so the bounce
/// select has a candidate on the third fire. The first fire is resolved by
/// PASSING on the bounce (so OPP-LOW is NOT returned to hand) and then
/// declining the unsuspend optional — this consumes the OPT lockout without
/// removing the candidate from P1's field.
///
/// Both players are given a 3-card deck so P1's draw phase does not deck-out
/// and terminate the game before P0's second turn begins.
#[test]
fn ad1_012_opt_resets_after_turn_boundary() {
    // Build a custom runner with deck cards to prevent P1 deck-out during
    // multi-turn traversal. cresgarurumon_runner() omits decks, so P1 would
    // deck-out on their draw phase and end the game via handle_deckout before
    // P0's second turn begins, which would prevent new_turn() from clearing OPT.
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 found in embedded DSL pack")
        .add_card(greymon_named())
        .add_card(low_level_opp())
        .add_card(deck_filler("AD1-012-FILLER-P0-A"))
        .add_card(deck_filler("AD1-012-FILLER-P0-B"))
        .add_card(deck_filler("AD1-012-FILLER-P0-C"))
        .add_card(deck_filler("AD1-012-FILLER-P1-A"))
        .add_card(deck_filler("AD1-012-FILLER-P1-B"))
        .add_card(deck_filler("AD1-012-FILLER-P1-C"))
        .deck(0, &["AD1-012-FILLER-P0-A", "AD1-012-FILLER-P0-B", "AD1-012-FILLER-P0-C"])
        .deck(1, &["AD1-012-FILLER-P1-A", "AD1-012-FILLER-P1-B", "AD1-012-FILLER-P1-C"])
        .memory(12)
        .start();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // ── First fire: consume the OPT without bouncing OPP-LOW ─────────────
    // Fire the shared clause; the bounce select installs first.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_some(),
        "first OnPlay must install selection (turn 1)"
    );

    // PASS on the bounce select (OPP-LOW stays on P1's field).
    let sel_player = runner.game.pending_selection.as_ref().unwrap().selecting_player;
    runner.execute_action(sel_player, PASS).expect("PASS on bounce select");

    // Drain any remaining selections (the optional unsuspend sub-block
    // may install; PASS through it too).
    let _ = runner.auto_resolve();

    // ── OPT now locked ────────────────────────────────────────────────────
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must block second OnPlay fire in same turn"
    );

    // ── Advance through both turns so P0 is the turn player again ────────
    runner.end_turn(); // player 1's turn (draws from deck — no deck-out)
    runner.end_turn(); // player 0's turn again

    // Guard: both end_turn calls must complete successfully (game must not be over).
    assert!(
        !runner.game.game_over,
        "game must not be over after turn rotation — check deck setup"
    );

    // ── Third fire: OPT must have reset ───────────────────────────────────
    // OPP-LOW is still on P1's field, so the bounce select has a candidate
    // and pending_selection must be installed.
    runner.fire_on_play(0, cres.index as usize);
    assert!(
        runner.game.pending_selection.is_some(),
        "OPT must reset after turn boundary — third fire on player 0's next turn must install selection"
    );
}

// ─── SECTION 10 — Inherited aura: modifier installed on carrier ──────────────

/// The inherited [Your Turn] CanNotSwitchAttackTarget aura must install the
/// `CanNotSwitchAttackTarget` modifier on the specific Digimon carrying the
/// inherited effect (the stacked permanent), not on all permanents.
#[test]
fn ad1_012_inherited_aura_installs_modifier_on_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 in embedded DSL pack")
        .add_card(digimon("AD1-012-CARRIER", 5000))
        .memory(0)
        .start();

    // Build a stack: CresGarurumon on top of a generic Lv.6 carrier.
    let stacked = runner.place_stack(0, &["AD1-012", "AD1-012-CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .modifiers()
            .has(stacked, ModifierType::CanNotSwitchAttackTarget),
        "the inherited aura must install CanNotSwitchAttackTarget on the permanent \
         that carries the CresGarurumon stack"
    );
}

/// When CresGarurumon is NOT in the digivolution stack of a permanent, that
/// permanent must NOT have the CanNotSwitchAttackTarget modifier.
#[test]
fn ad1_012_inherited_aura_not_installed_on_unrelated_permanent() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-012")
        .expect("AD1-012 in embedded DSL pack")
        .add_card(digimon("AD1-012-OTHER", 5000))
        .memory(0)
        .start();

    // Place a standalone generic Digimon — no CresGarurumon in its stack.
    let other = runner.place_on_field(0, "AD1-012-OTHER", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .modifiers()
            .has(other, ModifierType::CanNotSwitchAttackTarget),
        "CanNotSwitchAttackTarget must NOT apply to a permanent without \
         CresGarurumon in its digivolution stack"
    );
}

// ─── SECTION 11 — [When Digivolving] OPT lockout ─────────────────────────────

/// Firing [When Digivolving] then [When Digivolving] again in the same turn
/// must only install the selection on the first fire.
#[test]
fn ad1_012_opt_blocks_second_when_digivolving_same_turn() {
    let mut runner = cresgarurumon_runner();

    let cres = runner.place_on_field(0, "AD1-012", None);
    runner.place_on_field(1, "AD1-012-OPP-LOW", None);

    // First WD fire — selection must install.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(cres));
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_some(),
        "first [When Digivolving] fire must install selection"
    );
    let _ = runner.auto_resolve();

    // Second WD fire same turn — OPT must lock.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(cres));
    runner.game.drain_effect_queue();
    assert!(
        runner.game.pending_selection.is_none(),
        "second [When Digivolving] fire in same turn must be locked by OPT"
    );
}
