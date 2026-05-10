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
//! - **[Security][All Turns] +2000 DP aura** — BLOCKED on
//!   G-SECURITY-ZONE-AURA-SOURCE (engine-gaps.md L294). Group 6 ticks only
//!   battle-area sources; security-zone aura sources are explicitly listed
//!   as remaining work. The clause is OMITTED entirely — assertion that no
//!   `kind: aura` clause appears in the compiled card is the structural
//!   verification.
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
//! 1. **[Security][All Turns] +2000 DP aura** — OMITTED. Engine gap
//!    G-SECURITY-ZONE-AURA-SOURCE blocks security-zone aura source
//!    dispatch. No clause authored.
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

/// ST20-15 has THREE clauses:
///   [0] flood_gate (declarative, IgnoreColorRequirement)
///   [1] main_from_hand (triggered, add top security then place self option)
///   [2] inherited on_security (triggered, scope: Inherited)
///
/// The [Security][All Turns] DP aura (G-SECURITY-ZONE-AURA-SOURCE) is still
/// omitted.
#[test]
fn st20_15_has_three_clauses_in_expected_order() {
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
        3,
        "expected 3 clauses (flood_gate + main_from_hand + inherited on_security); \
         the [Security][All Turns] DP aura remains omitted under G-SECURITY-ZONE-AURA-SOURCE. Got {}",
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

/// Clause 1: main_from_hand timing. Body must add the top security card to hand
/// and then place the resolving Option card as top face-up security.
#[test]
fn st20_15_clause_1_main_adds_top_security_then_places_self_option() {
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
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 1 must fire from hand as an Option [Main] effect; got {:?}",
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
        other => panic!("clause 1 must be Triggered(main_from_hand); got {other:?}"),
    }
}

/// Clause 2: inherited scope, OnSecurity timing, optional (DCGO `canNoSelect:
/// true` / printed "You may"). Body must contain a `select_hand` step (Tamer
/// filter) followed by `play_from_hand_free`.
#[test]
fn st20_15_clause_2_inherited_security_optional_select_tamer_play_free() {
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
            assert_eq!(
                t.scope,
                CompiledScope::Inherited,
                "clause 2 must have Inherited scope (printed inherited [Security])"
            );
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "clause 2 must fire at OnSecurity; got {:?}",
                t.when
            );
            assert!(
                t.optional,
                "clause 2 outer trigger MUST be optional (printed 'You may'; DCGO `canNoSelect: true`)"
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
                "clause 2 body must contain a SelectHand step; got {:?}",
                t.process
            );
            assert!(
                has_play_free,
                "clause 2 body must contain a PlayFromHandFree step; got {:?}",
                t.process
            );
        }
        other => panic!(
            "clause 2 must be Triggered(inherited on_security); got {:?}",
            other
        ),
    }
}

/// Negative-shape: NO `kind: aura` clause should appear in the compiled card
/// (the [Security][All Turns] +2000 DP aura is OMITTED under
/// G-SECURITY-ZONE-AURA-SOURCE). When the engine gap closes a separate
/// commit will add the aura clause AND remove this assertion.
#[test]
fn st20_15_no_aura_clause_blocked_by_security_zone_aura_source() {
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
        aura_count, 0,
        "expected 0 aura clauses while G-SECURITY-ZONE-AURA-SOURCE is OPEN; got {aura_count}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — BLOCKED behavioral assertions (each #[ignore]'d under its gap)
// ═══════════════════════════════════════════════════════════════════════════════

/// G-PRED-NO-FACE-UP-SECURITY-NAMED + G-IGNORE-COLOR-MASK (mask is RESOLVED;
/// predicate is the blocker). When BOTH close, a non-white player should be
/// able to play ST20-15 from hand even with no white card on field, IFF no
/// face-up [Island of Adventure] sits in own security.
#[test]
#[ignore = "G-PRED-NO-FACE-UP-SECURITY-NAMED: no DSL leaf for face-up named-security predicate"]
fn st20_15_color_bypass_active_when_no_island_in_own_security() {
    // When the gap closes:
    // 1. Set up a non-white-on-field state for P0 (no white sources on field).
    // 2. Put ST20-15 in P0's hand. Empty out the security stack of any
    //    Island of Adventure cards.
    // 3. Assert that the play_option_from_hand action for ST20-15 IS legal in
    //    the action mask (color req bypassed via IgnoreColorRequirement).
    panic!("BLOCKED — see gap header in YAML");
}

/// G-PRED-NO-FACE-UP-SECURITY-NAMED: when an Island of Adventure SITS face-up
/// in own security, the bypass must NOT apply — the Option remains illegal
/// without a matching color source.
#[test]
#[ignore = "G-PRED-NO-FACE-UP-SECURITY-NAMED: no DSL leaf for face-up named-security predicate"]
fn st20_15_color_bypass_inactive_when_island_face_up_in_own_security() {
    // When the gap closes:
    // 1. Put a face-up [Island of Adventure] in P0's security (via
    //    place_self_option_at_security from a previous activation).
    // 2. Assert that the play_option_from_hand action for ST20-15 is NOT
    //    legal in the action mask (color req re-asserted).
    panic!("BLOCKED — see gap header in YAML");
}

/// G-SECURITY-ZONE-AURA-SOURCE: while ST20-15 sits face-up in P0's security,
/// all of P0's level-3+ Digimon must gain +2000 DP.
#[test]
#[ignore = "G-SECURITY-ZONE-AURA-SOURCE: security-zone aura source not iterated by tick_declarative_effects"]
fn st20_15_security_aura_buffs_own_lv3_plus_digimon_by_2000() {
    // When the gap closes:
    // 1. Put ST20-15 face-up in P0's security stack.
    // 2. Put a Lv3 White Digimon in P0's battle area with base DP 3000.
    // 3. Assert effective DP == 5000 (3000 base + 2000 aura).
    // 4. Drop a Lv2 ally — assert no buff applies (level filter).
    panic!("BLOCKED — see gap header in YAML");
}

/// G-SECURITY-ZONE-AURA-SOURCE: opponent's Digimon must NOT gain the +2000
/// buff (printed "All of YOUR" qualifier).
#[test]
#[ignore = "G-SECURITY-ZONE-AURA-SOURCE: security-zone aura source not iterated by tick_declarative_effects"]
fn st20_15_security_aura_does_not_buff_opponent_digimon() {
    // When the gap closes:
    // 1. Put ST20-15 face-up in P0's security stack.
    // 2. Put a Lv3 Digimon in P1's battle area (own-aura should not affect).
    // 3. Assert P1's Digimon DP unchanged.
    panic!("BLOCKED — see gap header in YAML");
}

/// G-SECURITY-ZONE-AURA-SOURCE: when ST20-15 is FACE-DOWN in own security
/// (e.g. placed there as a routine security card during shuffle), the aura
/// must NOT apply. (DCGO `IsExistInSecurity(card, false)` — the second arg
/// distinguishes face-up requirement.)
#[test]
#[ignore = "G-SECURITY-ZONE-AURA-SOURCE: security-zone aura source not iterated by tick_declarative_effects"]
fn st20_15_security_aura_inactive_when_self_face_down() {
    // When the gap closes:
    // 1. Put ST20-15 face-down in P0's security stack.
    // 2. Put a Lv3 White Digimon in P0's battle area.
    // 3. Assert effective DP == 3000 (no buff).
    panic!("BLOCKED — see gap header in YAML");
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

/// After [Main] resolves, the face-up [Island of Adventure] in own security
/// should close the color-bypass gate (Clause 0), but the predicate leaf for
/// "face-up named security card" is still missing.
#[test]
#[ignore = "G-PRED-NO-FACE-UP-SECURITY-NAMED: no DSL leaf for face-up named-security predicate"]
fn st20_15_main_followup_closes_color_bypass_gate() {
    // When the predicate gap closes:
    // 1. Activate [Main] once — places ST20-15 face-up in P0's security.
    // 2. Put a SECOND ST20-15 in P0's hand.
    // 3. Assert that play_option_from_hand for the second copy is NOT legal
    //    without a matching white color source on P0's field.
    panic!("BLOCKED — see gap header in YAML");
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
