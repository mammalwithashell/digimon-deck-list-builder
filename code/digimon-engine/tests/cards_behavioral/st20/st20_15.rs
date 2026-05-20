//! ST20-15 Island of Adventure — Option, White, Cost 2, traits: [ADVENTURE].
//!
//! # Card text (cards.json — verbatim)
//!
//! While you have no face-up [Island of Adventure] security cards, you can
//! ignore this card's color requirements.
//!
//! **[Security] [All Turns]** All of your level 3 or higher Digimon get
//! +2000 DP.
//!
//! **[Main]** Add your top security card to the hand. Then, place this card
//! face up as the top security card.
//!
//! **Inherited (Security):** [Security] You may play 1 Tamer card from your
//! hand without paying the cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/White/ST20_15.cs`
//!
//! # Patterns this test covers
//!
//! - **Color bypass via `kind: flood_gate` + `IgnoreColorRequirement`**
//!   gated by `no_permanent { of: you, zone: [security], card_number_is:
//!   "ST20-15" }`. WORKAROUND for G-PRED-NO-FACE-UP-SECURITY-NAMED — the
//!   DSL has no leaf for "no face-up <named> security card", so we use the
//!   closest available zone-scoped existential. Gate is structurally
//!   present; runtime semantics #[ignore]'d under both
//!   G-PRED-NO-FACE-UP-SECURITY-NAMED and G-IGNORE-COLOR-MASK (the latter
//!   was RESOLVED 2026-05-02 but the predicate side blocks the end-to-end
//!   test).
//!
//! - **[Security][All Turns] +2000 DP aura** — IMPLEMENTED via
//!   `kind: aura, scope: security` (G-SECURITY-ZONE-AURA-SOURCE CLOSED —
//!   `Game::tick_declarative_effects` iterates face-up security cards). The
//!   filter-aura targets own Lv3+ Digimon with `dp_modifier: 2000`.
//!
//! - **[Main] Add top security to hand → place self as top security
//!   face-up** — IMPLEMENTED via Track E
//!   `place_self_option_at_security: { position: top, face: up }`.
//!
//! - **[Security] (inherited) optional play 1 Tamer from hand free** —
//!   IMPLEMENTED. Standard `select_hand { kind: tamer }` →
//!   `play_from_hand_free` shape (sister to ST21-13 / BT17-093 / BT22-094
//!   on_security clauses). `optional: true` mirrors DCGO `canNoSelect: true`
//!   on the SelectHandEffect.
//!
//! # Faithfulness audit (per clause)
//!
//! 0. **Color bypass while NO face-up [Island of Adventure] in security** —
//!    `flood_gate` + `IgnoreColorRequirement` + `target: { card_number_is:
//!    "ST20-15" }` mirrors DCGO `IgnoreColorConditionClass.SetUpIgnoreColor
//!    ConditionClass(cardCondition: cardSource == card)`. The
//!    `no_permanent` gate is the closest expressible substitute for DCGO's
//!    `!card.Owner.SecurityCards.Any(c => c.EqualsCardName("Island of
//!    Adventure") && !c.IsFlipped)` — the face-up filter is the missing
//!    leaf (G-PRED-NO-FACE-UP-SECURITY-NAMED).
//!
//! 1. **[Security][All Turns] +2000 DP aura** — IMPLEMENTED as clause 1.
//!    `kind: aura, scope: security` with `target: { owner: you, kind:
//!    digimon, level_gte: 3 }` + `dp_modifier: 2000`. While ST20-15 sits
//!    face-up in security, `tick_declarative_effects` dispatches the
//!    filter-aura; the materialized-declarative clear+re-install model
//!    evicts the buff when the source leaves security or flips face-down.
//!
//! 2. **[Main] Add top security; then place self at security top face up**
//!    — IMPLEMENTED. The body is `add_top_security_to_hand` followed by
//!    `place_self_option_at_security`.
//!
//! 3. **[Security] (inherited) optional play 1 Tamer free** — IMPLEMENTED.
//!    `select_hand { kind: tamer }` filter matches DCGO `cardSource.IsTamer`;
//!    `play_from_hand_free` matches DCGO `payCost: false`. `optional: true`
//!    matches DCGO `canNoSelect: true`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST20-15";
const YAML: &str = include_str!("../../../cards/st20/ST20-15.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

/// A neutral filler card.
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A White Tamer for the inherited [Security] play-1-tamer clause.
fn make_tamer(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// A Lv3 White Digimon for the (BLOCKED) +2000 DP aura assertions.
fn make_lv3_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// ST20-15 YAML must parse and compile without errors.
#[test]
fn st20_15_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST20-15 YAML must parse and compile without errors");
}

/// ST20-15 must compile as an Option card with cost 2 and the ADVENTURE trait.
#[test]
fn st20_15_is_option_cost_2_with_adventure_trait() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("ST20-15 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "ST20-15 must be an Option card"
    );
    assert_eq!(card.cost, Some(2), "ST20-15 prints Cost 2");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST20-15 must carry the ADVENTURE trait (printed type_eng = ADVENTURE)"
    );
}

/// ST20-15 has FOUR clauses:
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] aura (declarative, scope: Security, +2000 DP to own Lv3+ Digimon)
///   [2] main_from_hand (triggered, add top security then place self option)
///   [3] inherited on_security (triggered, scope: Inherited)
///
/// The [Security][All Turns] DP aura (G-SECURITY-ZONE-AURA-SOURCE) is now
/// authored as clause 1 — the engine gap is CLOSED.
#[test]
fn st20_15_has_four_clauses_in_expected_order() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");
    assert_eq!(
        card.effects.len(),
        4,
        "expected 4 clauses (flood_gate + security aura + main_from_hand + inherited on_security). Got {}",
        card.effects.len()
    );
}

/// Clause 0: flood_gate declarative carrying the IgnoreColorRequirement modifier.
#[test]
fn st20_15_clause_0_is_flood_gate_with_ignore_color_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");

    let is_flood_gate_with_modifier = match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) => {
            modifier.eq_ignore_ascii_case("IgnoreColorRequirement")
        }
        _ => false,
    };
    assert!(
        is_flood_gate_with_modifier,
        "clause 0 must be a flood_gate carrying IgnoreColorRequirement; got {:?}",
        card.effects[0]
    );
}

/// Clause 1: declarative aura, scope: Security, carrying a +2000 DP modifier.
#[test]
fn st20_15_clause_1_is_security_scope_aura_with_dp_modifier() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");

    match &card.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope,
            dp_modifier,
            ..
        }) => {
            assert_eq!(
                *scope,
                CompiledScope::Security,
                "clause 1 aura must carry scope: Security (security-zone source)"
            );
            assert_eq!(
                *dp_modifier,
                Some(2000),
                "clause 1 aura must grant +2000 DP"
            );
        }
        other => panic!("clause 1 must be a Declarative(Aura); got {other:?}"),
    }
}

/// Clause 2: main_from_hand timing. Body must add the top security card to hand
/// and then place the resolving Option card as top face-up security.
#[test]
fn st20_15_clause_2_main_adds_top_security_then_places_self_option() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");

    match &card.effects[2] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 2 must fire from hand as an Option [Main] effect; got {:?}",
                t.when
            );
            assert!(!t.optional, "printed [Main] text is mandatory after play");
            assert!(matches!(
                t.process.as_slice(),
                [
                    CompiledStep::AddTopSecurityToHand { of },
                    CompiledStep::PlaceSelfOptionAtSecurity { position, face_up }
                ] if *of == digimon_dsl::compiled::CompiledPlayerRef::You
                    && *position == digimon_dsl::compiled::CompiledStackPosition::Top
                    && *face_up
            ));
        }
        other => panic!("clause 2 must be Triggered(main_from_hand); got {other:?}"),
    }
}

/// Clause 3: inherited scope, OnSecurity timing, optional (DCGO `canNoSelect:
/// true` / printed "You may"). Body must contain a `select_hand` step (Tamer
/// filter) followed by `play_from_hand_free`.
#[test]
fn st20_15_clause_3_inherited_security_optional_select_tamer_play_free() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");

    match &card.effects[3] {
        CompiledClause::Triggered(t) => {
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 3 must have Inherited scope (printed inherited [Security])"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 3 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                t.optional,
                "clause 3 outer trigger MUST be optional (printed 'You may'; DCGO `canNoSelect: true`)"
            );
            // Body: select_hand + play_from_hand_free.
            let has_select_hand = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::SelectHand { .. }));
            let has_play_free = t
                .process
                .iter()
                .any(|s| matches!(s, CompiledStep::PlayFromHandFree { .. }));
            assert!(
                has_select_hand,
                "clause 3 body must contain a SelectHand step; got {:?}",
                t.process
            );
            assert!(
                has_play_free,
                "clause 3 body must contain a PlayFromHandFree step; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 3 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

/// Positive-shape: exactly ONE `kind: aura` clause appears in the compiled
/// card (the [Security][All Turns] +2000 DP aura — G-SECURITY-ZONE-AURA-SOURCE
/// CLOSED).
#[test]
fn st20_15_has_exactly_one_security_aura_clause() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("ST20-15 compiled");

    let aura_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    assert_eq!(
        aura_count, 1,
        "expected exactly 1 aura clause (the [Security][All Turns] DP aura); got {aura_count}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — BLOCKED behavioral assertions (each #[ignore]'d under its gap)
// ═══════════════════════════════════════════════════════════════════════════════

/// G-PRED-NO-FACE-UP-SECURITY-NAMED (CLOSED) + G-IGNORE-COLOR-MASK (RESOLVED).
/// A non-white player can play ST20-15 from hand even with no white card on
/// field, because no face-up [Island of Adventure] sits in own security — the
/// card-level `use_requirement` lowers onto the [Main] clause's
/// `option_color_requirement_bypass` condition, which the action mask consults.
#[test]
fn st20_15_color_bypass_active_when_no_island_in_own_security() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        // Security pile has no [Island of Adventure] — color req is bypassed.
        .security(0, &["FILL", "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    // ST20-15 sits at hand index 0 → play action id 0.
    assert_eq!(
        mask[0], 1.0,
        "ST20-15 must be playable from hand when no face-up [Island of Adventure] \
         sits in own security (color requirement bypassed via use_requirement)"
    );
}

/// G-PRED-NO-FACE-UP-SECURITY-NAMED: when an [Island of Adventure] sits
/// face-up in own security, the bypass must NOT apply — the Option remains
/// illegal without a matching white color source on field.
#[test]
fn st20_15_color_bypass_inactive_when_island_face_up_in_own_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        // A second ST20-15 sits in the security pile — flip it face-up so the
        // `no_face_up_security_named` gate closes.
        .security(0, &[CARD_ID, "FILL"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Mark the bottom security card (the ST20-15 copy) face-up.
    let island_card_index = runner.game.players[0]
        .security
        .iter()
        .find(|c| c.card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST20-15 in security pile")
        .card_index;
    runner.game.players[0]
        .face_up_security
        .insert(island_card_index);

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[0], 0.0,
        "ST20-15 must NOT be playable from hand while a face-up [Island of \
         Adventure] sits in own security (color requirement re-asserted)"
    );

    // A face-DOWN [Island of Adventure] must not close the gate — the printed
    // text qualifies on "face-up". Remove it from the face-up set and re-check.
    runner.game.players[0]
        .face_up_security
        .remove(&island_card_index);
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[0], 1.0,
        "a face-DOWN [Island of Adventure] in security must NOT close the \
         color-bypass gate (printed 'face-up' qualifier)"
    );
}

/// A Lv2 White Digimon — below the level-3 filter floor.
fn make_lv2_digimon(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.colors = vec![CardColor::White];
    c
}

/// G-SECURITY-ZONE-AURA-SOURCE CLOSED: while ST20-15 sits face-up in P0's
/// security, all of P0's level-3+ Digimon get +2000 DP; a Lv2 ally does not.
#[test]
fn st20_15_security_aura_buffs_own_lv3_plus_digimon_by_2000() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv3_digimon("LV3-WHITE", "Lv3 White"))
        .add_card(make_lv2_digimon("LV2-WHITE", "Lv2 White"))
        .security(0, &[CARD_ID])
        .build();

    // ST20-15 sits FACE-UP in P0's security (its [Main] effect would place it
    // there; here we go straight to the resulting state).
    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let lv3 = runner.place_on_field(0, "LV3-WHITE", None);
    let lv2 = runner.place_on_field(0, "LV2-WHITE", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(lv3),
        Some(5000),
        "Lv3 ally must be 3000 base + 2000 security aura"
    );
    assert_eq!(
        runner.effective_dp(lv2),
        Some(2000),
        "Lv2 ally is below the level-3 filter floor — no buff"
    );
}

/// G-SECURITY-ZONE-AURA-SOURCE CLOSED: opponent's Lv3+ Digimon must NOT gain
/// the +2000 buff (printed "All of YOUR" qualifier).
#[test]
fn st20_15_security_aura_does_not_buff_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv3_digimon("OPP-LV3", "Opp Lv3"))
        .security(0, &[CARD_ID])
        .build();

    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);

    let opp_lv3 = runner.place_on_field(1, "OPP-LV3", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(opp_lv3),
        Some(3000),
        "opponent Lv3 Digimon must be unbuffed (printed 'All of YOUR' qualifier)"
    );
}

/// G-SECURITY-ZONE-AURA-SOURCE CLOSED: when ST20-15 is FACE-DOWN in own
/// security, the aura must NOT apply (DCGO `IsExistInSecurity(card, false)`).
#[test]
fn st20_15_security_aura_inactive_when_self_face_down() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_lv3_digimon("LV3-FD", "Lv3 White FD"))
        .security(0, &[CARD_ID])
        .build();

    // Deliberately do NOT add ST20-15 to face_up_security — it sits face-down.
    let lv3 = runner.place_on_field(0, "LV3-FD", None);

    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(lv3),
        Some(3000),
        "face-down ST20-15 in security must NOT fire the [Security] DP aura"
    );

    // Flip it face-up — the buff must now apply on the next tick.
    let src_card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(src_card_index);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(lv3),
        Some(5000),
        "once ST20-15 is face-up the +2000 aura applies"
    );
}

/// [Main] activation removes the controller's top security card to hand AND
/// places ST20-15 face up as the new top security card.
#[test]
fn st20_15_main_swaps_top_security_with_self_face_up() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("SEC-BOTTOM"))
        .add_card(filler("SEC-TOP"))
        .add_card(make_lv3_digimon("WHITE-DGM", "White Digimon"))
        .hand(0, &[CARD_ID])
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .memory(10)
        .start();
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner.place_on_field(0, "WHITE-DGM", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(
        result,
        digimon_engine::selection::OptionPlayResult::Invalid,
        "ST20-15 should be playable for this fixture"
    );

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "SEC-TOP"),
        "the previous top security card must move to hand"
    );
    let top_security = runner.game.players[0]
        .security
        .last()
        .expect("new top security");
    assert_eq!(
        top_security.card_id(&runner.game.card_data),
        CARD_ID,
        "ST20-15 must become the new top security card"
    );
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&top_security.card_index),
        "ST20-15 must be placed face-up"
    );
}

/// After [Main] resolves, the face-up [Island of Adventure] placed in own
/// security closes the color-bypass gate (the card-level `use_requirement`):
/// a SECOND ST20-15 in hand can no longer be played without a matching white
/// color source on field.
#[test]
fn st20_15_main_followup_closes_color_bypass_gate() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("SEC-BOTTOM"))
        .add_card(filler("SEC-TOP"))
        .hand(0, &[CARD_ID, CARD_ID])
        .security(0, &["SEC-BOTTOM", "SEC-TOP"])
        .deck(0, &["SEC-BOTTOM"])
        .deck(1, &["SEC-BOTTOM"])
        .memory(10)
        .start();
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;

    // Before the [Main] activation, the gate is open — the second copy IS
    // playable (no face-up Island in security).
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[1], 1.0,
        "second ST20-15 must be playable before any face-up Island is in security"
    );

    // Activate [Main] on the first copy (hand index 0) — its body places
    // ST20-15 face-up as the new top security card.
    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(
        result,
        digimon_engine::selection::OptionPlayResult::Invalid,
        "ST20-15 [Main] should resolve for this fixture"
    );
    runner.auto_resolve().ok();

    // The placed ST20-15 must now sit face-up in own security.
    let top = runner.game.players[0]
        .security
        .last()
        .expect("new top security");
    assert_eq!(top.card_id(&runner.game.card_data), CARD_ID);
    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&top.card_index),
        "ST20-15 must be placed face-up"
    );

    // The remaining ST20-15 in hand (now at index 0 after the first was
    // consumed) must NOT be playable — the face-up Island closed the gate.
    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[0], 0.0,
        "second ST20-15 must NOT be playable once a face-up [Island of \
         Adventure] sits in own security"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Inherited [Security] behavioral assertions
// ═══════════════════════════════════════════════════════════════════════════════
//
// G-INHERITED-DISPATCH (general gap on inherited triggered effects from the
// digivolution stack) does not block this clause: ST20-15 itself is an OPTION
// card; the inherited [Security] clause fires when ST20-15 is REVEALED by a
// security check (engine `OnSecurity` dispatch on raw security cards), not
// from a digivolution-stack source. The on_security path is wired (sister
// cards: ST21-13, BT22-094, P-189 all rely on the same dispatch).
//
// We assert structural shape only — full security-check execution paths
// require coordinating with the security-attack runner, which several sister
// cards leave as a deferred behavioral pass per the existing test discipline.
// The structural test in Section 1 (`st20_15_clause_1_inherited_security_optional_select_tamer_play_free`)
// is the canonical correctness check for this clause.

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence unused-warning across test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_tamer("X", "X");
    let _ = make_lv3_digimon("Y", "Y");
}
