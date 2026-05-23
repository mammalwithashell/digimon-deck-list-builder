//! EX7-023 Hexeblaumon — Digimon, Lv.6, Blue, DP 12000, Cost 12.
//! Traits: Magic Knight, Witchelny
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Iceclad＞ ＜Security A. +1＞
//! [When Digivolving] Trash any 4 of your opponent's Digimon's digivolution
//! cards. Then, if your opponent has no Digimon with digivolution cards, return
//! 1 of your opponent's Tamers to the bottom of the deck.
//! [Opponent's Turn] None of your opponent's Digimon with as many or fewer
//! digivolution cards as this Digimon can suspend.
//!
//! # Clause coverage
//!
//! | Clause | Status |
//! |---|---|
//! | Intrinsic \<Security A. +1\> | IMPLEMENTED — `kind: grant_keyword, keyword: SecurityAttackPlus, value: 1` |
//! | Intrinsic \<Iceclad\> | IMPLEMENTED — parsed from JSON effect_text automatically; no DSL clause needed |
//! | \[When Digivolving\] Trash 4 opp divi-cards | IMPLEMENTED — `select_opponent_permanent` + `select_opponent_sources` + `trash_selected_sources` |
//! | \[When Digivolving\] Conditional Tamer return | IMPLEMENTED — `if { no_permanent: ... } then [select_opponent_permanent + return_to_deck]` |
//! | \[Opponent's Turn\] anti-suspend flood gate | IMPLEMENTED — `kind: flood_gate, active_when: opponents_turn, modifier: CannotSuspend, target: materials_count_lte source_material_count` |
//!
//! # DCGO reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Blue/EX7_023.cs
//! (EX7 submodule not available — authored from printed text + RULES_CONTEXT.md.)

#![allow(unused_imports, dead_code, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_opp_digimon(id: &str, dp: i32, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_opp_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn make_filler_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c
}

/// Push `extra` extra CardSource entries under the top of a placed permanent,
/// simulating a digivolution stack of depth `1 + extra`.
/// Mirrors the `stack_extra_sources` helper in `tests/keyword_phase_f/iceclad.rs`.
fn stack_extra_sources(
    r: &mut DebugRunner,
    handle: PermanentHandle,
    filler_id: &str,
    extra: usize,
) {
    let filler_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == filler_id)
        .unwrap_or_else(|| panic!("stack_extra_sources: unknown card_id {filler_id}"));
    for _ in 0..extra {
        let ci = r.game.next_card_index();
        let src = CardSource::new(filler_idx, handle.player, ci);
        r.game.players[handle.player as usize].battle_area[handle.index as usize]
            .card_sources
            .insert(0, src);
    }
}

/// Fire the WhenDigivolving timing on `carrier` and drain the effect queue.
fn fire_when_digivolving(runner: &mut DebugRunner, carrier: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// YAML parses and compiles without error.
#[test]
fn ex7_023_yaml_parses_without_error() {
    DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML must parse and compile")
        .start();
}

/// EX7-023 has exactly 3 compiled clauses:
///   0: GrantKeyword declarative (SecurityAttackPlus)
///   1: Triggered WhenDigivolving (trash 4 + conditional Tamer return)
///   2: FloodGate declarative (CannotSuspend, Opponent's Turn)
#[test]
fn ex7_023_has_three_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-023")
        .expect("EX7-023 in compiled_cards");
    assert_eq!(
        compiled.effects.len(),
        3,
        "EX7-023 must have exactly 3 clauses (GrantKeyword + WhenDigivolving + FloodGate); \
         got {}",
        compiled.effects.len()
    );
}

/// Clause 0 is a declarative GrantKeyword for SecurityAttackPlus (intrinsic
/// <Security A. +1>). It is NOT inherited — intrinsic keyword on this card only.
#[test]
fn ex7_023_clause_0_is_grant_keyword_security_attack_plus() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-023")
        .expect("EX7-023 in compiled_cards");
    let clause = compiled.effects.get(0).expect("clause 0 must exist");
    assert!(
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
        ),
        "Clause 0 must be a declarative GrantKeyword (SecurityAttackPlus); got {:?}",
        clause
    );
    // Verify it targets SecurityAttackPlus with value 1 and is FaceUp (not inherited).
    let found = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            value,
            ..
        }) if keyword == "SecurityAttackPlus" => Some((*scope, *value)),
        _ => None,
    });
    let (scope, value) = found.expect("GrantKeyword SecurityAttackPlus clause must exist");
    assert_eq!(
        scope,
        CompiledScope::FaceUp,
        "SecurityAttackPlus must be FaceUp scope (intrinsic, not inherited)"
    );
    assert_eq!(value, Some(1), "SecurityAttackPlus value must be 1");
}

/// Clause 1 is a triggered WhenDigivolving clause (FaceUp, not optional at
/// the outer level — no "may" on the trash step in printed text).
#[test]
fn ex7_023_clause_1_is_when_digivolving_triggered() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-023")
        .expect("EX7-023 in compiled_cards");
    let clause = compiled.effects.get(1).expect("clause 1 must exist");
    assert!(
        matches!(clause, CompiledClause::Triggered(_)),
        "Clause 1 must be a triggered clause (WhenDigivolving); got {:?}",
        clause
    );
    if let CompiledClause::Triggered(t) = clause {
        assert!(
            t.when.contains(&CompiledTiming::WhenDigivolving),
            "Clause 1 must have WhenDigivolving timing; got {:?}",
            t.when
        );
        assert_eq!(
            t.scope,
            CompiledScope::FaceUp,
            "Clause 1 must be FaceUp scope (own effect, not inherited)"
        );
        assert!(
            !t.once_per_turn,
            "WhenDigivolving clause must not be OPT (printed text has no [Once Per Turn])"
        );
        assert!(
            !t.optional,
            "WhenDigivolving clause must not be optional at clause level (no 'you may' on trash)"
        );
    }
}

/// Clause 2 is a declarative FloodGate (CannotSuspend, Opponent's Turn).
#[test]
fn ex7_023_clause_2_is_flood_gate_cannot_suspend() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-023")
        .expect("EX7-023 in compiled_cards");
    let clause = compiled.effects.get(2).expect("clause 2 must exist");
    assert!(
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
        ),
        "Clause 2 must be a declarative FloodGate (CannotSuspend); got {:?}",
        clause
    );
    // Verify modifier name is CannotSuspend.
    if let CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. }) =
        clause
    {
        assert_eq!(
            modifier, "CannotSuspend",
            "FloodGate modifier must be CannotSuspend; got {modifier}"
        );
    }
}

// ─── Section 2: SecurityAttackPlus (intrinsic) ───────────────────────────────

/// Hexeblaumon on the field gives itself SecurityAttackPlus 1 via the
/// GrantKeyword declarative. `game.has_keyword(h, Keyword::SecurityAttackPlus)`
/// must return true.
#[test]
fn ex7_023_security_attack_plus_is_granted_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .start();
    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner
            .game
            .has_keyword(hexe, Keyword::SecurityAttackPlus(1)),
        "Hexeblaumon must have SecurityAttackPlus(1) after declarative tick"
    );
}

/// Iceclad is parsed from `cards.json` effect_text (contains ＜Iceclad＞ in the
/// printed text). No DSL clause is needed or possible — Iceclad is not in
/// `lookup_keyword()` and is consumed directly by `combat::resolve_battle`
/// via `has_keyword`. This test verifies the raw JSON parsing produces a
/// `CardData` entry for EX7-023 that includes `Keyword::Iceclad` in its
/// pre-parsed keyword list.
#[test]
fn ex7_023_iceclad_is_intrinsic_via_card_data() {
    const CARDS_JSON: &str = include_str!("../../../../../data/cards.json");
    let all_cards =
        CardData::load_from_str(CARDS_JSON).expect("cards.json must parse without error");
    let hexe_data = all_cards
        .get("EX7-023")
        .expect("EX7-023 must exist in cards.json");
    assert!(
        hexe_data.keywords.contains(&Keyword::Iceclad),
        "EX7-023 cards.json effect_text must parse to include Keyword::Iceclad; \
         got keywords: {:?}",
        hexe_data.keywords
    );
}

// ─── Section 3: [When Digivolving] behavioral ────────────────────────────────

/// With an opponent Digimon that has ≥4 digivolution cards, firing
/// WhenDigivolving installs the opp-Digimon selection prompt.
#[test]
fn ex7_023_when_digivolving_installs_opp_digimon_pick() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .memory(10)
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // Give the opponent Digimon 4 extra digivolution cards (total 5 sources — 4 divi-cards).
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 4);
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        5,
        "precondition: OPP has 5 card sources (4 divi-cards)"
    );

    fire_when_digivolving(&mut runner, hexe);

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("WhenDigivolving must install a pending selection (opp Digimon pick)");
    // The first selection is for the opponent Digimon target.
    assert!(
        matches!(sel.kind, SelectionKind::OppField),
        "WhenDigivolving selection must be OppField (select opp Digimon); got {:?}",
        sel.kind
    );
}

/// Opponent Digimon with fewer than 4 digivolution cards is NOT offered as a
/// pick target for the trash step. The selection has no non-PASS valid actions.
#[test]
fn ex7_023_when_digivolving_excludes_opp_digimon_with_fewer_than_4_divi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .memory(10)
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // Only 3 extra sources → 4 total (3 divi-cards under top) — below ≥4 gate.
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 3);
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        4,
        "precondition: OPP has 4 total card sources (3 divi-cards) — below the ≥4 gate"
    );

    fire_when_digivolving(&mut runner, hexe);

    // No opp Digimon with ≥4 divi-cards → selection has no non-pass actions.
    if let Some(sel) = runner.game.pending_selection.as_ref() {
        let card_actions: Vec<u16> = sel
            .valid_action_ids
            .iter()
            .copied()
            .filter(|&a| a != digimon_engine::action::space::PASS)
            .collect();
        assert!(
            card_actions.is_empty(),
            "With no opp Digimon having ≥4 divi-cards, the pick must have no valid targets; \
             got {card_actions:?}"
        );
    }
    // If no pending selection at all, the condition gate blocked correctly.
}

/// Full WhenDigivolving flow: trash exactly 4 digivolution cards from the
/// opponent's Digimon. The Digimon's card_sources.len() shrinks by 4.
#[test]
fn ex7_023_when_digivolving_trashes_4_divi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .memory(10)
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // 5 extra divi-cards → total 6 card_sources (top + 5 divi-cards).
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 5);
    let sources_before = runner.game.players[1].battle_area[0].card_sources.len();
    assert_eq!(sources_before, 6, "precondition: 6 card sources");

    fire_when_digivolving(&mut runner, hexe);

    // The first selection: pick the opp Digimon (mandatory, one valid target).
    if runner.game.pending_selection.is_some() {
        let view = runner
            .pending_selection_view()
            .expect("opp Digimon pick pending");
        let action = *view
            .valid_action_ids
            .iter()
            .find(|&&a| a != digimon_engine::action::space::PASS)
            .expect("opp Digimon pick must have a valid target");
        runner
            .execute_action(view.selecting_player, action)
            .expect("pick opp Digimon");
        runner.game.drain_effect_queue();
    }

    // Resolve all remaining selections (source picks + trash + conditional branch).
    runner.auto_resolve().unwrap_or(());

    let sources_after = runner.game.players[1].battle_area[0].card_sources.len();
    assert_eq!(
        sources_before - sources_after,
        4,
        "WhenDigivolving must trash exactly 4 digivolution cards; \
         before={sources_before}, after={sources_after}"
    );
}

/// Conditional Tamer return: if after trashing all 4 divi-cards the opp has NO
/// Digimon with digivolution cards, a Tamer must be selected for return.
///
/// Setup: 1 opp Digimon with exactly 4 divi-cards (so after trashing all 4 it
/// has 0 divi-cards), plus 1 opp Tamer. After the trash the condition is met.
#[test]
fn ex7_023_tamer_return_fires_when_opp_has_no_divi_cards_after_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .add_card(make_opp_tamer("TAMER"))
        .memory(10)
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    runner.place_on_field(1, "TAMER", Some(1));
    // Exactly 4 extra divi-cards → total 5 sources (4 divi-cards under top).
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 4);
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        5,
        "precondition: OPP has 5 card_sources (4 divi-cards)"
    );

    let deck_before = runner.deck_size(1);
    // Count tamers on opp field before effect.
    let tamer_count_before = runner.game.players[1]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(tamer_count_before, 1, "precondition: 1 opp Tamer on field");

    fire_when_digivolving(&mut runner, hexe);

    // Drive all mandatory selections to completion (opp Digimon + 4 sources + Tamer).
    runner.auto_resolve().unwrap_or(());

    // After auto-resolve: all 4 divi-cards trashed, then the Tamer returned to deck.
    let sources_after = runner.game.players[1].battle_area[0].card_sources.len();
    assert_eq!(
        sources_after, 1,
        "After trashing 4 divi-cards, OPP must have 1 card_source left (top only)"
    );
    let tamer_count_after = runner.game.players[1]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(
        tamer_count_after, 0,
        "After the conditional Tamer return, the opp Tamer must have left the field"
    );
    assert_eq!(
        runner.deck_size(1),
        deck_before + 1,
        "The Tamer card must be in the opponent's deck (returned to bottom)"
    );
}

/// Conditional Tamer return does NOT fire if any opp Digimon still has divi-cards
/// after the trash.
///
/// Setup: 2 opp Digimon, both with ≥4 divi-cards. Hexeblaumon trashes 4 from
/// one; the other still has divi-cards → condition fails → no Tamer return.
#[test]
fn ex7_023_tamer_return_does_not_fire_when_another_opp_digimon_has_divi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP_A", 5000, 5))
        .add_card(make_opp_digimon("OPP_B", 5000, 5))
        .add_card(make_opp_tamer("TAMER"))
        .memory(10)
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_a = runner.place_on_field(1, "OPP_A", Some(1));
    let opp_b = runner.place_on_field(1, "OPP_B", Some(1));
    runner.place_on_field(1, "TAMER", Some(1));
    // OPP_A: exactly 4 divi-cards (Hexeblaumon will trash all 4 from this one).
    stack_extra_sources(&mut runner, opp_a, "FILLER", 4);
    // OPP_B: also 4 divi-cards (remains after Hexeblaumon effects resolve).
    stack_extra_sources(&mut runner, opp_b, "FILLER", 4);

    let tamer_count_before = runner.game.players[1]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(tamer_count_before, 1, "precondition: 1 opp Tamer on field");

    fire_when_digivolving(&mut runner, hexe);

    // Drive all mandatory selections (the Tamer return selection must NOT appear).
    runner.auto_resolve().unwrap_or(());

    let tamer_count_after = runner.game.players[1]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Tamer)
        .count();
    assert_eq!(
        tamer_count_after, 1,
        "Tamer must remain on field when another opp Digimon still has digivolution cards"
    );
}

// ─── Section 4: [Opponent's Turn] CannotSuspend flood gate ───────────────────

/// On the opponent's turn, an opponent Digimon with as many or fewer divi-cards
/// as Hexeblaumon receives the CannotSuspend modifier after a declarative tick.
///
/// Hexeblaumon has 2 divi-cards. A 2-divi-card opp Digimon must be locked.
#[test]
fn ex7_023_flood_gate_installs_cannot_suspend_on_eligible_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // Hexeblaumon: 2 divi-cards (total 3 card_sources).
    stack_extra_sources(&mut runner, hexe, "FILLER", 2);
    // OPP: also 2 divi-cards (total 3 card_sources) — same count as Hexeblaumon.
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 2);

    // Switch to opponent's turn so `opponents_turn: true` active_when fires.
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .has(opp_digi, ModifierType::CannotSuspend),
        "OPP with 2 divi-cards (≤ Hexeblaumon's 2) must receive CannotSuspend on opp's turn"
    );
}

/// Opponent Digimon with MORE divi-cards than Hexeblaumon is NOT locked by the
/// flood gate. Only Digimon with ≤ Hexeblaumon's count are affected.
#[test]
fn ex7_023_flood_gate_does_not_lock_digimon_with_more_divi_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // Hexeblaumon: 1 divi-card (total 2 card_sources).
    stack_extra_sources(&mut runner, hexe, "FILLER", 1);
    // OPP: 3 divi-cards (total 4 card_sources) — MORE than Hexeblaumon.
    stack_extra_sources(&mut runner, opp_digi, "FILLER", 3);

    runner.game.turn_player_idx = 1; // Opponent's turn.
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has(opp_digi, ModifierType::CannotSuspend),
        "OPP with 3 divi-cards (> Hexeblaumon's 1) must NOT receive CannotSuspend"
    );
}

/// On YOUR turn, the flood gate is NOT active — `active_when: opponents_turn:
/// true`. Even an eligible opp Digimon must not receive CannotSuspend.
#[test]
fn ex7_023_flood_gate_inactive_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_opp_digimon("OPP", 5000, 5))
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_digi = runner.place_on_field(1, "OPP", Some(1));
    // Both have 0 divi-cards (1 card_source each) — OPP would be eligible if
    // it were the opponent's turn. Controller's turn: turn_player_idx = 0.
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has(opp_digi, ModifierType::CannotSuspend),
        "CannotSuspend flood gate must be inactive on your own turn"
    );
}

/// The flood gate uses `source_material_count` (Hexeblaumon's own divi-card
/// count) as the comparison threshold. As Hexeblaumon's stack grows, more
/// opponent Digimon fall within the lock window.
///
/// Phase A: Hexeblaumon 0 divi-cards → OPP_SMALL (0 divi-cards) locked,
///          OPP_BIG (4 divi-cards) NOT locked.
/// Phase B: Hexeblaumon grows to 4 divi-cards → OPP_BIG now also locked.
#[test]
fn ex7_023_flood_gate_threshold_tracks_hexeblaumon_divi_card_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_filler_digimon("FILLER"))
        .add_card(make_opp_digimon("OPP_SMALL", 3000, 4)) // 0 divi-cards initially
        .add_card(make_opp_digimon("OPP_BIG", 5000, 6))   // 4 divi-cards
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let opp_small = runner.place_on_field(1, "OPP_SMALL", Some(1));
    let opp_big = runner.place_on_field(1, "OPP_BIG", Some(1));
    // OPP_BIG: 4 divi-cards.
    stack_extra_sources(&mut runner, opp_big, "FILLER", 4);
    // Hexeblaumon: 0 divi-cards initially.

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    // Phase A assertions.
    assert!(
        runner
            .game
            .modifiers
            .has(opp_small, ModifierType::CannotSuspend),
        "Phase A: OPP_SMALL (0 divi-cards) must be locked when Hexeblaumon has 0 divi-cards"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_big, ModifierType::CannotSuspend),
        "Phase A: OPP_BIG (4 divi-cards) must NOT be locked when Hexeblaumon has 0 divi-cards"
    );

    // Now grow Hexeblaumon's stack to 4 divi-cards → OPP_BIG now qualifies.
    stack_extra_sources(&mut runner, hexe, "FILLER", 4);
    runner.game.tick_declarative_effects();

    // Phase B assertions.
    assert!(
        runner
            .game
            .modifiers
            .has(opp_big, ModifierType::CannotSuspend),
        "Phase B: OPP_BIG (4 divi-cards) must now be locked when Hexeblaumon has 4 divi-cards"
    );
}

/// Own Digimon are NOT affected by the flood gate — it only targets opponent
/// Digimon.
#[test]
fn ex7_023_flood_gate_does_not_affect_own_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-023")
        .expect("EX7-023 YAML loads")
        .add_card(make_opp_digimon("OWN_ALLY", 3000, 4))
        .start();

    let hexe = runner.place_on_field(0, "EX7-023", Some(0));
    let ally = runner.place_on_field(0, "OWN_ALLY", Some(0));

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .has(ally, ModifierType::CannotSuspend),
        "Own Digimon must not receive CannotSuspend from the flood gate (targets opp only)"
    );
}
