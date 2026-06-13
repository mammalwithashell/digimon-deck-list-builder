//! EX11-015 Frigimon — Digimon, Lv.4, Blue/Yellow, DP 6000, Cost 5.
//! Traits: Ice-Snow, LIBERATOR. Attribute: Vaccine. Form: Champion. Rarity: U.
//!
//! # Card text (card image EX11-015.webp, authoritative; cards.json agrees)
//!
//! Evo box: Lv.3 from blue: Cost 3.
//! Evo box: "Digivolve Lv.3 w/[Ice-Snow] trait: Cost 2" (cards.json `xros_req`).
//!
//! [When Digivolving] If you have 1 or fewer Tamers, you may play 1
//! [Suzune Kazuki] from your hand without paying the cost.
//!
//! Inherited Effect: <Jamming> (This Digimon can't be deleted in battles
//! against Security Digimon.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_015.cs
//!   - Alt digivolution requirement (None): AddSelfDigivolutionRequirement-
//!     StaticEffect(permanentCondition: TopCard.EqualsTraits("Ice-Snow") &&
//!     TopCard.IsLevel3, digivolutionCost: 2) — trait + level gate, no color.
//!   - When Digivolving (OnEnterFieldAnyone): ActivateClass with
//!     CanActivateCondition requiring battle-area Tamer count <= 1 AND a
//!     matching hand card; the hand filter is IsTamer &&
//!     EqualsCardName("Suzune Kazuki") — EXACT name. SelectHandEffect with
//!     maxCount: 1, canNoSelect: true ("you may" — decline is legal), then
//!     PlayPermanentCards(payCost: false).
//!   - Inherited <Jamming> (None): JammingSelfStaticEffect(
//!     isInheritedEffect: true).
//!
//! # Patterns this test file covers
//! - [When Digivolving] conditional optional free-play from hand
//!   (count_lte Tamer condition + select_hand name_is + play_from_hand_free)
//!   — EX8-048 / BT21-017 idiom.
//! - Inherited static keyword grant (<Jamming>) verified in a real losing
//!   security-Digimon battle — EX11-014 / security_effects.rs combat idiom.
//! - Alt digivolve recipe registration (standard Lv.3 blue + Lv.3 [Ice-Snow]).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "EX11-015";

// ─── Card factories ───────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A [Suzune Kazuki] Tamer — the free-play target (EX8-066/EX11-057 are not
/// implemented yet; synthetic stand-in carries the exact printed name).
fn suzune(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

/// A Tamer whose name merely CONTAINS "Suzune Kazuki" — must NOT match
/// (DCGO uses EqualsCardName, an exact-name match).
fn near_name_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki Drone");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Blue];
    c
}

/// A generic non-Suzune Tamer (field-count fodder / non-candidate hand card).
fn generic_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

/// A plain carrier Digimon to stack EX11-015 under (inherited Jamming test).
fn carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(3000);
    c.colors = vec![CardColor::Blue];
    c
}

/// A high-DP security Digimon the carrier loses the DP battle against.
fn digimon_security(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Red];
    c
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Fire WhenDigivolving for the given permanent handle.
fn fire_when_digivolving(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn ex11_015_compiles_with_two_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-015 compiled card present");

    assert_eq!(
        card.effects.len(),
        2,
        "EX11-015 must have exactly 2 clauses: [When Digivolving] free-play + inherited <Jamming>"
    );
}

#[test]
fn ex11_015_when_digivolving_clause_is_optional_not_opt() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-015 compiled card present");

    let wd = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX11-015 must have a [When Digivolving] clause");

    assert!(wd.optional, "printed 'you may' — the clause is optional");
    assert!(
        !wd.once_per_turn,
        "no printed [Once Per Turn] — must not be once_per_turn"
    );
    assert_eq!(
        wd.scope,
        CompiledScope::FaceUp,
        "main-text clause — default FaceUp scope"
    );
}

#[test]
fn ex11_015_has_inherited_jamming_grant() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-015 compiled card present");

    let jamming = card.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => *scope == CompiledScope::Inherited && keyword.eq_ignore_ascii_case("Jamming"),
        _ => false,
    });
    assert!(jamming, "EX11-015 must grant inherited <Jamming>");
}

#[test]
fn ex11_015_registers_standard_and_ice_snow_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .build();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX11-015 compiled card present");

    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(
        digivolve_paths, 2,
        "EX11-015 must register the standard Lv.3-blue cost-3 path and the \
         Lv.3 [Ice-Snow] cost-2 alt path"
    );
}

// ─── Section 2: Condition gating ([When Digivolving] clause) ────────────────

/// Positive: 0 Tamers on field + [Suzune Kazuki] in hand → the effect fires
/// and installs the hand selection.
#[test]
fn ex11_015_fires_with_zero_tamers_and_suzune_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(suzune("SUZ-1"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_some(),
        "with 0 Tamers and [Suzune Kazuki] in hand the free-play selection must be offered"
    );
}

/// Boundary: exactly 1 Tamer on field — "1 or fewer" still passes.
#[test]
fn ex11_015_fires_with_exactly_one_tamer_boundary() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(generic_tamer("TAMER-1"))
        .add_card(suzune("SUZ-1"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_some(),
        "exactly 1 Tamer satisfies '1 or fewer' — the selection must be offered"
    );
}

/// Negative: 2 Tamers on field → the condition fails, the effect must NOT fire.
#[test]
fn ex11_015_does_not_fire_with_two_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(generic_tamer("TAMER-1"))
        .add_card(generic_tamer("TAMER-2"))
        .add_card(suzune("SUZ-1"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let _ = runner.place_on_field(0, "TAMER-1", Some(0));
    let _ = runner.place_on_field(0, "TAMER-2", Some(0));
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_none(),
        "with 2 Tamers the '1 or fewer' condition fails — no selection"
    );
}

/// Negative: no [Suzune Kazuki] in hand → no candidates, no prompt.
#[test]
fn ex11_015_does_not_prompt_without_suzune_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(filler("PAD"))
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_none(),
        "no [Suzune Kazuki] in hand — nothing to select"
    );
}

/// Exact-name fidelity: a Tamer named "Suzune Kazuki Drone" must NOT be a
/// candidate (DCGO EqualsCardName).
#[test]
fn ex11_015_name_filter_requires_exact_suzune_kazuki() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(near_name_tamer("NEAR"))
        .add_card(filler("PAD"))
        .hand(0, &["NEAR"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_none(),
        "\"Suzune Kazuki Drone\" must NOT satisfy the exact-name filter"
    );
}

/// Candidate precision: with a [Suzune Kazuki] AND a generic Tamer in hand,
/// only the Suzune is a selectable candidate.
#[test]
fn ex11_015_non_suzune_tamers_in_hand_are_not_candidates() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(suzune("SUZ-1"))
        .add_card(generic_tamer("TAMER-H"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1", "TAMER-H"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    fire_when_digivolving(&mut runner, handle);

    let view = runner
        .pending_selection_view()
        .expect("selection pending with a Suzune candidate in hand");
    let non_pass: Vec<_> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the [Suzune Kazuki] card is a candidate — the generic Tamer is not; \
         valid_action_ids={:?}",
        view.valid_action_ids
    );
}

// ─── Section 3: Behavioral outcome ──────────────────────────────────────────

/// Selecting [Suzune Kazuki] plays it onto the field WITHOUT paying the cost:
/// field +1, hand -1, memory unchanged.
#[test]
fn ex11_015_plays_suzune_from_hand_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(suzune("SUZ-1"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(5)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, handle);
    let memory_before = runner.memory();

    {
        let view = runner.pending_selection_view().expect("hand selection");
        let pick = *view
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("a non-PASS candidate");
        runner.execute_action(0, pick).expect("select Suzune");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "[Suzune Kazuki] must be on the field after the free play"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "[Suzune Kazuki] must leave the hand"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "played WITHOUT paying the cost — memory must not change"
    );
    // The played permanent is the Suzune Tamer.
    let has_suzune = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUZ-1");
    assert!(has_suzune, "the new permanent is the played [Suzune Kazuki]");
}

/// Declining ("you may"): PASS is legal and leaves everything unchanged.
#[test]
fn ex11_015_declining_leaves_suzune_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(suzune("SUZ-1"))
        .add_card(filler("PAD"))
        .hand(0, &["SUZ-1"])
        .deck(0, &["PAD", "PAD", "PAD"])
        .memory(10)
        .start();

    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_is_optional(),
        "printed 'you may' — the selection must be declinable"
    );
    runner
        .execute_action(0, PASS)
        .expect("pass the optional selection");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "[Suzune Kazuki] stays in hand after declining"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no new permanent after declining"
    );
}

// ─── Section 4: Behavioral — inherited <Jamming> ────────────────────────────

/// Stacked under a carrier, EX11-015's inherited <Jamming> keeps the carrier
/// alive through a LOSING security-Digimon battle.
#[test]
fn ex11_015_inherited_jamming_keeps_carrier_alive_vs_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    // EX11-015 as a digivolution source under the carrier.
    let atk = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(atk, Keyword::Jamming),
        "the carrier must hold <Jamming> from EX11-015's inherited grant"
    );

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "Jamming carrier (3000 DP) must survive the losing battle vs the \
         9000 DP security Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the carrier stays on the field"
    );
    assert_eq!(runner.trash_size(1), 1, "the security card still trashes");
}

/// Baseline: the same carrier WITHOUT EX11-015 stacked dies to the same
/// security Digimon — proving the inherited grant is what does the work.
#[test]
fn ex11_015_baseline_carrier_without_frigimon_dies_vs_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-015 YAML loads from embedded DSL pack")
        .add_card(carrier("CARRIER"))
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "CARRIER", Some(0));
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(atk, Keyword::Jamming),
        "no Jamming without the EX11-015 source"
    );

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without Jamming the 3000 DP carrier loses to the 9000 DP security \
         Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0, "the carrier is deleted");
}
