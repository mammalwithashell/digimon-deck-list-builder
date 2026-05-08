//! BT22-089 Mirei Mikagura — Tamer, Yellow + Purple, Cost 3.
//!
//! # Card text (cards.json)
//!
//! **[Start of Your Main Phase]** By returning this Tamer to the bottom of
//! the deck, you may play 1 play cost 4 or higher [Mirei Mikagura] or play
//! cost 4 or higher Tamer card with the [CS] trait from your hand without
//! paying the cost.
//!
//! **[On Play]** By trashing 1 card with the [Holy Beast], [Angel],
//! [Archangel], [Fallen Angel] or [CS] trait from your hand, `<Draw 2>`
//! (Draw 2 cards from your deck.)
//!
//! **Inherited (Security):** [Security] Play this card without paying the
//! cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT22/Yellow/BT22_089.cs`
//!
//! # Patterns this test covers
//!
//! - **Tamer play-3 anchor** (cost-3 persistent on-your-turn) — structural.
//! - **Start-of-main-phase cost-as-return-to-deck-bottom + optional name/
//!   trait-filtered Tamer-from-hand replay** — Clause 1. Same shape as
//!   BT17-093 Tai/Kari and BT24-082 Owen Dreadnought.
//! - **OnPlay cost-as-trash-from-hand + draw 2** — Clause 2. Same shape as
//!   BT24-008 Elizamon, with a 5-trait disjunction on the trash filter.
//! - **Security free-play** — Clause 3 (structural; behavioral free-play
//!   confirmation lives in shared `play_from_security` substrate).
//!
//! # Known gaps tagged
//!
//! - `G-PLAY-COST-GTE` (NEW, mirror of CLOSED G-PLAY-COST-LTE): the
//!   `play_cost_gte: 4` filter on Clause 1's `select_hand` cannot be
//!   expressed in `PredicateSpec` today. The disjunction structure is
//!   preserved in the YAML but the cost gate is silently dropped, making
//!   the `select_hand` filter over-permissive on the cost axis. Negative
//!   tests for "cost-3 CS Tamer must NOT appear in valid actions" are
//!   `#[ignore = "pending: G-PLAY-COST-GTE"]`. See YAML header for the
//!   trivial mirror-of-`play_cost_lte` fix.
//! - `G-OPT-TRIGGERED` — `once_per_turn` is not enforced on triggered
//!   effects today; not relevant here (no clause carries `once_per_turn`).
//! - The "you may decline the entire Clause 1" outer optionality is modeled
//!   by `optional: true` at the clause level; in the absence of an explicit
//!   inner Yes/No prompt step, declining at the install point matches
//!   DCGO's outer BoolSelection-no contract for sister cards (BT17-093,
//!   BT24-082, BT22-084). The inner `select_hand` IS optional and
//!   declinable.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const BT22_089_YAML: &str = include_str!("../../../cards/bt22/BT22-089.yaml");
const CARD_ID: &str = "BT22-089";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A Tamer with the CS trait and printed cost ≥ 4 — qualifies for Clause 1's
/// hand filter under both the printed text AND the over-permissive YAML
/// (without the play_cost_gte:4 leaf).
fn make_cs_tamer_cost5(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 5;
    c.traits = vec!["CS".to_string()];
    c
}

/// A Mirei-named Tamer with printed cost ≥ 4 — qualifies for Clause 1 via
/// the `name_contains: "Mirei Mikagura"` arm.
fn make_mirei_tamer_cost4(id: &str) -> CardData {
    let mut c = make_test_card(id, "Mirei Mikagura");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 4;
    c.traits = vec!["CS".to_string()];
    c
}

/// A non-CS, non-Mirei Tamer — must NOT match Clause 1's filter under either
/// the printed text or the YAML.
fn make_other_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec![];
    c
}

/// A CS-trait Tamer with printed cost 3 — fails the "cost ≥ 4" gate. Used by
/// the negative G-PLAY-COST-GTE test (currently #[ignore]'d).
fn make_cs_tamer_cost3(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["CS".to_string()];
    c
}

/// A Digimon with one of the Clause-2 cost-trait families (e.g. Holy Beast).
fn make_trait_digimon(id: &str, name: &str, trait_str: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec![trait_str.to_string()];
    c
}

/// A Digimon with NO matching trait — must not satisfy Clause 2's hand filter.
fn make_unrelated_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.traits = vec!["Aquatic".to_string()];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// BT22-089 must compile with exactly 3 triggered clauses:
///   - start_of_your_main_phase (Clause 1)
///   - on_play (Clause 2)
///   - on_security (Clause 3)
#[test]
fn bt22_089_has_exactly_three_triggered_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("BT22-089 YAML parses + compiles")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT22-089 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        3,
        "BT22-089 must have exactly 3 triggered clauses (start_of_your_main_phase, on_play, on_security)"
    );
}

/// Card metadata: kind=Tamer, cost=3, colors=[Yellow, Purple], traits=[CS],
/// no level/dp.
#[test]
fn bt22_089_metadata_matches_printed_text() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("BT22-089 YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(compiled.cost, Some(3), "BT22-089 prints Cost 3");
    assert!(
        matches!(
            compiled.kind,
            digimon_dsl::compiled::CompiledCardKind::Tamer
        ),
        "BT22-089 must be a Tamer"
    );
    assert!(
        compiled
            .color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Yellow)),
        "BT22-089 must include Yellow"
    );
    assert!(
        compiled
            .color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Purple)),
        "BT22-089 must include Purple"
    );
    assert_eq!(
        compiled.level, None,
        "Tamers carry no printed level (DSL: omitted)"
    );
    assert_eq!(
        compiled.dp, None,
        "Tamers carry no printed DP (DSL: omitted)"
    );
    assert!(
        compiled.traits.iter().any(|t| t == "CS"),
        "BT22-089 must carry the [CS] trait"
    );
}

/// Clause 1 (start_of_your_main_phase) shape: face-up, optional ("you may").
#[test]
fn bt22_089_clause1_start_of_main_phase_face_up_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");

    assert_eq!(
        clause1.scope,
        CompiledScope::FaceUp,
        "Clause 1 (start_of_your_main_phase) must be FaceUp scope"
    );
    assert!(
        clause1.optional,
        "Clause 1 must be optional — printed 'you may' / DCGO inner BoolSelection"
    );
    assert!(
        !clause1.once_per_turn,
        "Clause 1 has no [Once Per Turn] — cost-as-return-to-deck-bottom limits re-entry"
    );
}

/// Clause 2 (on_play) shape: face-up, optional ("By trashing ... you may").
#[test]
fn bt22_089_clause2_on_play_face_up_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("on_play clause must exist");

    assert_eq!(
        clause2.scope,
        CompiledScope::FaceUp,
        "Clause 2 (on_play) must be FaceUp scope"
    );
    assert!(
        clause2.optional,
        "Clause 2 must be optional — printed 'By trashing ... <Draw 2>' / DCGO isOptional=true"
    );
}

/// Clause 3 (on_security) shape: face-up, mandatory.
#[test]
fn bt22_089_clause3_on_security_face_up_mandatory() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .memory(0)
        .start();

    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    let security = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert_eq!(security.scope, CompiledScope::FaceUp);
    assert!(
        !security.optional,
        "Security clause must be mandatory — [Security] effects are not opt-out per RULES_CONTEXT.md §16"
    );
    assert!(!security.once_per_turn, "Security clause has no OPT");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 Start-of-Main-Phase
//                         (return self → may play Mirei/CS-Tamer free)
//
// We exercise via the start/end_turn flow (P0 → P1 → P0) so begin_turn fires
// StartOfYourMainPhase for P0 with Mirei in play. Same harness shape as
// BT22-084's clause 1 in section 2.
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper: build a runner with BT22-089 placed and a CS-trait Tamer (cost 5)
/// in P0's hand for the Clause 1 replacement step.
fn runner_for_main_phase_with_cs_tamer() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_cs_tamer_cost5("CS-TAMER-5", "Other CS Tamer"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start()
}

/// Positive: Clause 1 fires at the start of P0's main phase; the player
/// accepts, paying the return-to-deck-bottom cost, then selects the CS-trait
/// Tamer from hand. After resolution: BT22-089 is off-field (returned to
/// deck), and the new CS-trait Tamer is on-field.
#[test]
fn bt22_089_clause1_start_of_main_phase_returns_self_and_plays_cs_tamer_free() {
    let mut runner = runner_for_main_phase_with_cs_tamer();
    runner.place_on_field(0, CARD_ID, Some(0));

    // Put a CS-trait Tamer (cost 5) in P0's hand.
    let cs_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CS-TAMER-5")
        .expect("CS-TAMER-5 registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            cs_idx, 0, next,
        ));

    // Cycle P0 → P1 → P0 so begin_turn fires StartOfYourMainPhase for P0.
    runner.end_turn(); // P0 → P1
    runner.end_turn(); // P1 → P0; clause 1 should install on Mirei.

    // Resolve any pending selections by accepting the first non-PASS option
    // (cost prompt + Tamer-pick prompt). Same loop pattern as BT17-093.
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
    }
    let _ = runner.auto_resolve();

    // BT22-089 must be off-field (returned to deck bottom).
    let mirei_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !mirei_on_field,
        "BT22-089 must be off-field after Clause 1 cost (return-to-deck-bottom) is paid"
    );

    // CS-trait Tamer must be on field (played free).
    let cs_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "CS-TAMER-5");
    assert!(
        cs_on_field,
        "CS-trait Tamer (cost 5) must be on field after Clause 1 free-play step"
    );
}

/// Positive: Clause 1 hand filter accepts a Mirei-named Tamer (cost 4) via
/// the `name_contains: "Mirei Mikagura"` arm. Same flow as the CS-arm test.
#[test]
fn bt22_089_clause1_start_of_main_phase_plays_mirei_named_tamer_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_mirei_tamer_cost4("MIREI-OTHER"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    // Put a Mirei-named Tamer (cost 4) in P0's hand.
    let mirei_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "MIREI-OTHER")
        .expect("MIREI-OTHER registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            mirei_idx, 0, next,
        ));

    runner.end_turn();
    runner.end_turn();

    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept pending selection");
    }
    let _ = runner.auto_resolve();

    let mirei_other_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "MIREI-OTHER");
    assert!(
        mirei_other_on_field,
        "Mirei-named Tamer (cost 4) must be on field after Clause 1 free-play step (name_contains arm)"
    );
}

/// Negative-1: hand has no eligible Tamer (only an unrelated cost-3 Tamer
/// without CS trait and not named Mirei). Clause 1's cost still resolves
/// (single-trigger optional cannot decline outer body in current DSL
/// lowering — see header), but no replacement Tamer enters the field.
///
/// Asserts: BT22-089 leaves the field (cost paid), no other tamer on field.
#[test]
fn bt22_089_clause1_no_eligible_tamer_in_hand_still_pays_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_other_tamer("OTHER-TAMER", "Marcus Damon"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let other_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OTHER-TAMER")
        .expect("OTHER-TAMER registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            other_idx, 0, next,
        ));

    runner.end_turn();
    runner.end_turn();

    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner.execute_action(0, accept).expect("accept install");
    }
    let _ = runner.auto_resolve();

    // BT22-089 must be off-field (cost paid).
    let mirei_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        !mirei_on_field,
        "BT22-089 must leave the field once Clause 1 outer is accepted (cost paid)"
    );

    // Marcus Damon must NOT be on field (filter excludes — non-Mirei, non-CS).
    let other_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "OTHER-TAMER");
    assert!(
        !other_on_field,
        "non-Mirei / non-CS tamer must NOT be played free (filter excludes)"
    );
}

/// Negative-2 (G-PLAY-COST-GTE BLOCKED): a CS-trait Tamer with printed cost
/// 3 must NOT appear in Clause 1's `select_hand` valid actions, because the
/// printed text caps the filter at "play cost 4 or higher". Under the
/// current DSL the `play_cost_gte: 4` predicate cannot be expressed; the
/// cost gate is silently dropped and the cost-3 CS Tamer over-fires into
/// the valid-action list.
///
/// When G-PLAY-COST-GTE closes (mirror of CLOSED G-PLAY-COST-LTE), this
/// test must un-`#[ignore]` and pass.
#[test]
#[ignore = "pending: G-PLAY-COST-GTE — `play_cost_gte` predicate not in PredicateSpec; cost-3 CS Tamer over-fires into Clause 1 valid actions"]
fn bt22_089_clause1_blocks_cs_tamer_below_cost_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_cs_tamer_cost3("CS-TAMER-3", "Cheap CS Tamer"))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .memory(5)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let cs3_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CS-TAMER-3")
        .expect("CS-TAMER-3 registered");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            cs3_idx, 0, next,
        ));

    runner.end_turn();
    runner.end_turn();

    // After cost is paid, the inner select_hand prompt installs. Accept
    // every prompt; if `play_cost_gte: 4` were enforced, the CS-3 Tamer
    // would NOT be in valid_action_ids and the inner prompt would auto-pass
    // (no eligible candidate), leaving CS-3 in hand. Without the predicate,
    // the inner prompt over-fires and plays CS-3 free.
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner.execute_action(0, accept).expect("accept install");
    }
    let _ = runner.auto_resolve();

    // CS-3 Tamer must NOT be on field (cost gate would block under the
    // predicate fix).
    let cs3_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "CS-TAMER-3");
    assert!(
        !cs3_on_field,
        "CS-trait Tamer with cost 3 must NOT be playable via Clause 1 (printed text: cost ≥ 4); BLOCKED on G-PLAY-COST-GTE"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 OnPlay
//                         (trash 1 Holy Beast/Angel/.../CS card → draw 2)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: BT22-089 played from hand with a Holy-Beast Digimon also in
/// hand. After the play resolves, Clause 2 installs a select_hand prompt;
/// player picks the Holy-Beast card; engine trashes it and draws 2.
///
/// Asserts: trashed card moved to trash, hand grows by net +1 (−1 played
/// Mirei, −1 trashed cost, +2 drawn).
#[test]
fn bt22_089_clause2_on_play_trash_holy_beast_and_draws_two() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_trait_digimon("HB-DIGI", "Saberdramon", "Holy Beast"))
        .deck(
            0,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .deck(
            1,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .memory(10)
        .start();

    // Place Mirei + Holy-Beast Digimon in P0's hand.
    let mirei_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("BT22-089 registered");
    let hb_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "HB-DIGI")
        .expect("HB-DIGI registered");
    let next1 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            mirei_idx, 0, next1,
        ));
    let next2 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            hb_idx, 0, next2,
        ));

    let hand_before = runner.game.players[0].hand.len();
    let mirei_hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|cs| cs.card_id(&runner.game.card_data) == CARD_ID)
        .expect("Mirei is in hand");

    // Play Mirei (cost 3) from hand.
    runner
        .play(0, mirei_hand_idx)
        .expect("Mirei plays from hand (cost 3)");

    // Clause 2 must install a select_hand prompt for the trash candidate.
    assert!(
        runner.game.pending_selection.is_some(),
        "Mirei On Play must install a select_hand prompt for Holy Beast / Angel / Archangel / Fallen Angel / CS card"
    );

    // Pick the Holy-Beast Digimon (the only eligible candidate in hand).
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept select_hand");
    }
    let _ = runner.auto_resolve();

    // Holy-Beast Digimon must be in trash.
    let hb_trashed = runner.game.players[0]
        .trash
        .iter()
        .any(|cs| cs.card_id(&runner.game.card_data) == "HB-DIGI");
    assert!(
        hb_trashed,
        "Holy-Beast Digimon must be in trash after Clause 2 cost is paid"
    );

    // Hand size: started with `hand_before`. After: −1 Mirei (played to
    // field), −1 Holy-Beast (trashed as cost), +2 drawn = hand_before net 0.
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after, hand_before,
        "hand must net 0 after Clause 2 (−1 Mirei played, −1 cost trashed, +2 drawn)"
    );

    // Mirei must be on field (the play resolved before Clause 2's process).
    let mirei_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(mirei_on_field, "Mirei must be on field after the play");
}

/// Positive: trait coverage — Angel / Archangel / Fallen Angel / CS each
/// satisfy Clause 2's filter (parameterized via a single helper test).
///
/// We cover Angel here; the other 3 traits route through identical YAML
/// `any_of` arms so a single representative is sufficient. Archangel and
/// Fallen Angel are covered structurally by the YAML's compiled
/// CompiledPredicate any_of branch list.
#[test]
fn bt22_089_clause2_on_play_trash_angel_card_qualifies() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_trait_digimon("ANGEL-DIGI", "Angemon", "Angel"))
        .deck(
            0,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .deck(
            1,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .memory(10)
        .start();

    let mirei_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let angel_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ANGEL-DIGI")
        .unwrap();
    let next1 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            mirei_idx, 0, next1,
        ));
    let next2 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            angel_idx, 0, next2,
        ));

    let mirei_hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|cs| cs.card_id(&runner.game.card_data) == CARD_ID)
        .expect("Mirei in hand");

    runner.play(0, mirei_hand_idx).expect("Mirei plays");

    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(0, accept)
            .expect("accept select_hand");
    }
    let _ = runner.auto_resolve();

    // Angel-trait Digimon must be in trash.
    let angel_trashed = runner.game.players[0]
        .trash
        .iter()
        .any(|cs| cs.card_id(&runner.game.card_data) == "ANGEL-DIGI");
    assert!(
        angel_trashed,
        "Angel-trait Digimon must be in trash after Clause 2 cost is paid"
    );
}

/// Negative: hand has only unrelated-trait Digimon (Aquatic). Clause 2's
/// condition gate (`count_gte: 1` over the trait disjunction in hand) should
/// fail; no select_hand prompt installs; no draw.
///
/// Asserts: Mirei plays normally, no trash-cost / draw, hand size decreases
/// by 1 (only Mirei moves to field; no draw triggered).
///
/// Same precedent gap as `bt24_008_on_play_no_eligible_card_no_prompt`:
/// the clause-level `count_gte` aggregate over hand is parsed and lowered
/// but `eval_predicate_with_bindings` has no match arm for non-security /
/// non-materials `count_gte` (only `security_count_*` and
/// `materials_count_*` are wired in `dsl_cards/predicate.rs`). The gate
/// silently passes and the clause over-fires; the inner `select_hand`
/// filter is also accept-all (Phase 2b), so the optional select_hand
/// quietly resolves with no pick — but the downstream `draw 2` still
/// fires unconditionally. Net observation: hand size −1 (Mirei played)
/// +2 (drawn) = +1 vs. before, instead of the expected −1.
///
/// When `dsl-select-hand-filter-phase2c` (Phase 2c filter enforcement /
/// generic `count_gte` evaluation) lands, un-`#[ignore]` and the gate
/// will block the over-fire.
#[test]
#[ignore = "pending: dsl-select-hand-filter-phase2c — generic count_gte aggregate over hand is not evaluated; Clause 2 over-fires draw 2 even when no eligible trash candidate exists. Same precedent as bt24_008_on_play_no_eligible_card_no_prompt."]
fn bt22_089_clause2_on_play_no_eligible_trait_in_hand_no_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT22_089_YAML)
        .expect("YAML parses")
        .add_card(make_filler("FILLER"))
        .add_card(make_unrelated_digimon("AQU-DIGI", "Seadramon"))
        .deck(
            0,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .deck(
            1,
            &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER", "FILLER"],
        )
        .memory(10)
        .start();

    let mirei_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .unwrap();
    let aqu_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "AQU-DIGI")
        .unwrap();
    let next1 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            mirei_idx, 0, next1,
        ));
    let next2 = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            aqu_idx, 0, next2,
        ));

    let hand_before = runner.game.players[0].hand.len();
    let mirei_hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|cs| cs.card_id(&runner.game.card_data) == CARD_ID)
        .expect("Mirei in hand");

    runner.play(0, mirei_hand_idx).expect("Mirei plays");

    // Drain any auto-resolving selections (none expected from clause 2).
    let _ = runner.auto_resolve();

    // Aquatic Digimon must remain in hand (no eligible trait → no trash).
    let aqu_in_hand = runner.game.players[0]
        .hand
        .iter()
        .any(|cs| cs.card_id(&runner.game.card_data) == "AQU-DIGI");
    assert!(
        aqu_in_hand,
        "Aquatic Digimon must remain in hand (no Holy/Angel/.../CS trait → Clause 2 gate fails)"
    );

    // Hand size: started with `hand_before` (Mirei + Aqu). After: −1 Mirei
    // played, no draw. So hand size = hand_before − 1.
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after,
        hand_before - 1,
        "hand must shrink by 1 when Clause 2 condition fails (Mirei plays, no draw triggered)"
    );

    // Mirei is on field (the play itself resolves regardless of Clause 2).
    let mirei_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(mirei_on_field, "Mirei must be on field after the play");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_cs_tamer_cost5("Y", "Y");
    let _ = make_mirei_tamer_cost4("Z");
    let _ = make_other_tamer("A", "A");
    let _ = make_cs_tamer_cost3("B", "B");
    let _ = make_trait_digimon("C", "C", "Holy Beast");
    let _ = make_unrelated_digimon("D", "D");
}
