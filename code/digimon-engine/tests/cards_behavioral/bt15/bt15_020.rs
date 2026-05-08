//! BT15-020 Gabumon — Digimon, Lv.3, Blue, DP 1000, Cost 3.
//! Traits: Reptile.
//!
//! # Card text (cards.json)
//!
//! [Start of Your Main Phase] 1 of your Digimon gains ＜Blocker＞ until the
//! end of your opponent's turn. Then, if you have a Tamer with [Matt Ishida]
//! in its name, ＜Draw 1＞.
//!
//! Inherited Effect [When Attacking] [Once Per Turn] ＜Draw 1＞.
//!
//! Digivolve: Lv.2 Blue / cost 0. Bonus digivolve: cost 0 from Tsunomon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT15/Blue/BT15_020.cs
//!
//! Notes from DCGO:
//!   * Start-of-main effect is mandatory (`isOptional: false` in
//!     `SetUpActivateClass`). Selection of the Blocker target is mandatory
//!     (`canNoSelect: false`). The "draw 1 if Matt Ishida" sub-clause runs
//!     independently of whether a Blocker target was available.
//!   * Inherited When Attacking Draw 1 is mandatory + OPT
//!     (`SetUpActivateClass(..., 1, false, ...)`, `IsInheritedEffect=true`,
//!     `SetHashString("Draw1_BT15_020")`).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 Start-of-main-phase Digimon clause (memory phase trigger, conditional)
//! - H5 Blocker keyword grant via `grant_keyword` step + `end_of_opponents_turn`
//!   expiry (procedural variant — not a `kind: grant_keyword` declarative)
//! - G4 Inherited [When Attacking][OPT] Draw 1 (scope: inherited)
//! - E2-adjacent: independent sub-clauses gated by separate `if` conditions
//!   (Blocker requires own Digimon; Draw requires Matt Ishida tamer)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (a) [SoMP] grant Blocker to selected Digimon | none | PASS |
//! | (a-then) Draw 1 if Matt Ishida tamer present | none | PASS |
//! | (b) Inherited [When Attacking][OPT] Draw 1 (structural) | none | PASS |
//! | (b) Inherited [When Attacking] fires & draws on attack | depends on inherited dispatch from below-top — covered by carrier pattern | PASS |

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

fn make_lv4_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

fn make_matt_tamer(id: &str) -> CardData {
    use digimon_engine::enums::CardKind;
    let mut c = make_test_card(id, "Matt Ishida");
    c.card_kind = CardKind::Tamer;
    c.traits = vec![];
    c
}

fn make_other_tamer(id: &str) -> CardData {
    use digimon_engine::enums::CardKind;
    let mut c = make_test_card(id, "Tai Kamiya");
    c.card_kind = CardKind::Tamer;
    c.traits = vec![];
    c
}

/// Base runner with BT15-020 registered from the embedded DSL pack and a
/// modest filler deck for both players (so begin_turn draws don't deckout).
fn gabumon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT15-020")
        .expect("BT15-020 in embedded DSL pack")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_filler("CARRIER"))
        .add_card(make_lv4_carrier("LV4-CARRIER"))
        .add_card(make_matt_tamer("MATT"))
        .add_card(make_other_tamer("OTHER-TAMER"))
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .deck(
            1,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(5)
        .start()
}

/// Place BT15-020 on P0's field as a face-up Digimon.
fn place_gabumon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_on_field(0, "BT15-020", None)
}

/// Place a CARRIER permanent on P0 with BT15-020 inserted as a bottom
/// digivolution source. Mirrors the BT21-008 inherited-source pattern.
fn place_gabumon_as_source(runner: &mut DebugRunner) -> PermanentHandle {
    let handle = runner.place_on_field(0, "LV4-CARRIER", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BT15-020")
            .expect("BT15-020 registered in card_data");
        let next = game.next_card_index();
        let mut gabumon_src = CardSource::new(data_idx, 0, next);
        gabumon_src.card_index = next;
        let perm = &mut game.players[0].battle_area[handle.index as usize];
        // Insert at bottom; carrier remains on top.
        perm.card_sources.insert(0, gabumon_src);
    }
    handle
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions on the compiled card
// ═══════════════════════════════════════════════════════════════════════════════

/// BT15-020 must be present in the embedded pack (YAML compiles cleanly).
#[test]
fn bt15_020_yaml_compiles_and_is_in_pack() {
    let runner = gabumon_runner();
    assert!(
        runner.compiled_card("BT15-020").is_some(),
        "BT15-020 must be present in the embedded DSL pack"
    );
}

/// Two triggered clauses: start_of_your_main_phase + inherited when_attacking.
#[test]
fn bt15_020_has_two_triggered_clauses() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card("BT15-020")
        .expect("BT15-020 in embedded pack");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();

    assert_eq!(
        triggered_count, 2,
        "BT15-020 must have exactly 2 triggered clauses: start_of_main + inherited when_attacking"
    );
}

/// The Start-of-Your-Main-Phase clause is FaceUp scope and not optional
/// (printed text is mandatory — no "you may"). Selection inside the clause
/// is its own thing; the clause itself fires automatically.
#[test]
fn bt15_020_start_of_main_clause_is_face_up_and_not_optional() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card("BT15-020")
        .expect("BT15-020 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "start_of_main clause must be FaceUp (own) scope"
    );
    assert!(
        !clause.optional,
        "start_of_main clause is mandatory per printed text (no 'you may')"
    );
    assert!(
        !clause.once_per_turn,
        "start_of_main clause has no [OPT] restriction in printed text"
    );
}

/// The inherited When-Attacking clause is Inherited scope and OPT.
#[test]
fn bt15_020_inherited_when_attacking_is_opt_inherited() {
    let runner = gabumon_runner();
    let card = runner
        .compiled_card("BT15-020")
        .expect("BT15-020 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenAttacking) && t.scope == CompiledScope::Inherited
        })
        .expect("inherited when_attacking clause must exist");

    assert!(
        clause.once_per_turn,
        "inherited When Attacking clause must be [Once Per Turn]"
    );
    assert!(
        !clause.optional,
        "inherited When Attacking Draw 1 is mandatory per printed text (no 'you may')"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: [Start of Your Main Phase] grant Blocker
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Gabumon on field with at least one own Digimon → start of main
/// installs a SelectOwnPermanent prompt (mandatory pick of the Blocker target).
#[test]
fn bt15_020_start_of_main_installs_select_own_permanent() {
    let mut runner = gabumon_runner();
    let _g = place_gabumon(&mut runner);

    // Trigger start-of-main-phase manually.
    runner.game.enter_main_phase();

    let kind = runner.pending_kind();
    assert!(
        kind.is_some(),
        "start_of_main clause must install a selection when an own Digimon exists"
    );
    assert_eq!(
        kind,
        Some(SelectionKind::OwnField),
        "selection kind must be OwnField for select_own_permanent"
    );
}

/// POSITIVE: After picking the Digimon (Gabumon itself in this minimal setup),
/// Blocker is granted. has_keyword(Blocker) must be true on the picked target.
#[test]
fn bt15_020_start_of_main_grants_blocker_to_picked_digimon() {
    let mut runner = gabumon_runner();
    let g = place_gabumon(&mut runner);

    // Sanity: no Blocker before the effect runs.
    assert!(
        !runner.game.has_keyword(g, Keyword::Blocker),
        "Gabumon must not start with Blocker"
    );

    runner.game.enter_main_phase();
    assert!(
        runner.pending_selection().is_some(),
        "selection must install"
    );

    // Resolve: pick the first valid action (Gabumon — only own Digimon on board).
    let action = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        assert!(
            !pending.valid_action_ids.is_empty(),
            "at least one own Digimon must be a valid target"
        );
        pending.valid_action_ids[0]
    };
    runner
        .game
        .resolve_selection(0, action)
        .expect("selection resolves");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.has_keyword(g, Keyword::Blocker),
        "selected Digimon must gain Blocker after Gabumon's start-of-main effect resolves"
    );
}

/// POSITIVE: The granted Blocker survives end-of-your-turn and is still
/// active during the opponent's turn (expiry = end_of_opponents_turn).
#[test]
fn bt15_020_blocker_persists_through_your_turn_end() {
    let mut runner = gabumon_runner();
    let g = place_gabumon(&mut runner);

    runner.game.enter_main_phase();
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();
    assert!(runner.game.has_keyword(g, Keyword::Blocker));

    // End P0's turn → P1's turn begins. Blocker should still be on Gabumon.
    runner.game.end_turn();
    assert!(
        runner.game.has_keyword(g, Keyword::Blocker),
        "Blocker must still be active during opponent's turn (expiry is end_of_opponents_turn)"
    );
}

/// POSITIVE: The granted Blocker expires when the opponent's turn ends.
#[test]
fn bt15_020_blocker_expires_at_end_of_opponents_turn() {
    let mut runner = gabumon_runner();
    let g = place_gabumon(&mut runner);

    runner.game.enter_main_phase();
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();
    assert!(runner.game.has_keyword(g, Keyword::Blocker));

    // End P0's turn → P1 turn → end P1's turn → back to P0.
    runner.game.end_turn();
    runner.game.end_turn();

    assert!(
        !runner.game.has_keyword(g, Keyword::Blocker),
        "Blocker must expire after the opponent's turn ends"
    );
}

/// NEGATIVE: Gabumon on field with NO other own Digimon and the Matt Ishida
/// branch absent — the start-of-main prompt should still install for picking
/// Gabumon itself ("1 of your Digimon" includes self).
///
/// Dual purpose: confirms the Blocker selection accepts BT15-020 itself as a
/// valid target (no `other: true` filter).
#[test]
fn bt15_020_start_of_main_target_includes_gabumon_itself() {
    let mut runner = gabumon_runner();
    let _g = place_gabumon(&mut runner);

    runner.game.enter_main_phase();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("selection installs");
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "only Gabumon is on field, so exactly one valid target"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: "Then, if you have a Tamer named Matt Ishida, draw 1"
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: With Gabumon and a Matt Ishida tamer on field, after resolving
/// the Blocker selection the player draws exactly 1 card.
#[test]
fn bt15_020_start_of_main_draws_when_matt_ishida_tamer_present() {
    let mut runner = gabumon_runner();
    let _g = place_gabumon(&mut runner);
    runner.place_on_field(0, "MATT", None);

    let hand_before = runner.hand_size(0);

    runner.game.enter_main_phase();
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after,
        hand_before + 1,
        "must draw 1 when a Tamer named Matt Ishida is on field; \
         hand before={hand_before} after={hand_after}"
    );
}

/// NEGATIVE: With Gabumon on field but a non-Matt tamer (e.g. Tai Kamiya),
/// no draw should occur after resolving the Blocker selection.
#[test]
fn bt15_020_start_of_main_does_not_draw_when_tamer_is_not_matt() {
    let mut runner = gabumon_runner();
    let _g = place_gabumon(&mut runner);
    runner.place_on_field(0, "OTHER-TAMER", None);

    let hand_before = runner.hand_size(0);

    runner.game.enter_main_phase();
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, hand_before,
        "must NOT draw when the tamer's name does not contain 'Matt Ishida'; \
         hand before={hand_before} after={hand_after}"
    );
}

/// NEGATIVE: With Gabumon on field but NO tamer at all, no draw should occur.
#[test]
fn bt15_020_start_of_main_does_not_draw_when_no_tamer() {
    let mut runner = gabumon_runner();
    let _g = place_gabumon(&mut runner);

    let hand_before = runner.hand_size(0);

    runner.game.enter_main_phase();
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert_eq!(
        hand_after, hand_before,
        "must NOT draw when no Matt Ishida tamer is in scope"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Behavioral: Inherited [When Attacking][OPT] Draw 1
// ═══════════════════════════════════════════════════════════════════════════════

/// POSITIVE: Gabumon as a digivolution source under a higher carrier — when
/// the carrier attacks, the inherited [When Attacking] Draw 1 must fire.
///
/// Mirrors the bt21_008 inherited-source dispatch pattern. Relies on the
/// G-INHERITED-DISPATCH fix landing for battle-area observers (closed
/// 2026-04-29).
#[test]
fn bt15_020_inherited_when_attacking_draws_one_from_carrier() {
    let mut runner = gabumon_runner();
    // Make sure P1 has a security card the carrier can attack.
    runner.game_mut().players[1].security.clear();
    let carrier = place_gabumon_as_source(&mut runner);

    let hand_before = runner.hand_size(0);

    // Attack opponent player (no security → direct hit). The trigger fires
    // on attack declaration regardless of attack target.
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();

    let hand_after = runner.hand_size(0);
    assert!(
        hand_after >= hand_before + 1,
        "carrier with Gabumon source must draw at least 1 from inherited When Attacking; \
         hand before={hand_before} after={hand_after}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Negative gating: no own Digimon → no selection installed
// ═══════════════════════════════════════════════════════════════════════════════

/// NEGATIVE: If Gabumon's own perm is removed before start-of-main, no
/// selection should install (the trigger source is gone).
///
/// We sanity-check the absence-of-Gabumon path: place no Gabumon, no
/// selection installs because the trigger has no carrier.
#[test]
fn bt15_020_no_selection_when_gabumon_not_on_field() {
    let mut runner = gabumon_runner();
    // Don't place Gabumon. Trigger main phase.
    runner.game.enter_main_phase();
    assert!(
        runner.pending_kind().is_none(),
        "no selection should install when Gabumon is not on the field"
    );
}
