//! EX9-074 Kimeramon — Digimon, Lv.5, White, DP 10000, Cost 10.
//! Traits: Composite/DM/Ver.3. Attribute: Data.
//!
//! # Card text (official Bandai DB — data/card_bundles/EX9-074.md, verbatim,
//! cross-checked against the card image and cards.json)
//!
//! ＜Rush＞ ＜Security A. +1＞
//! [On Play] [When Digivolving] You may place 1 level 4 or lower [DM] trait
//! Digimon card from your trash as this Digimon's top digivolution card.
//! Then, delete 1 of your opponent's Digimon with the same color as any of
//! this Digimon's digivolution cards. If this Digimon has 6 or more colors
//! in its digivolution cards, instead delete 1 of each of your opponent's
//! Digimon with different colors.
//! [All Turns] This Digimon gets +1000 DP for each color in its digivolution
//! cards.
//!
//! Special Play Condition: Assembly -7: 7 level 4 [DM] trait Digimon cards
//! w/different names.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/White/EX9_074.cs
//!
//! # Patterns this test covers
//! - Assembly alt-path (7 different-named Lv.4 [DM] materials from trash)
//! - Declarative keyword grant (<Rush>) + declarative aura (<Security A. +1>)
//! - E1-adjacent: [On Play]/[When Digivolving] shared body, optional
//!   (non-cost) select_trash + place_as_bottom_source
//! - Formula-driven self DP aura (`source_color_count`, all-turns, symmetric)
//!
//! # Known engine/DSL gaps affecting this card (see YAML header for the full
//! writeup)
//!
//! - **G-DSL-PLACE-AS-TOP-SOURCE** (qa/dsl-vocab-gaps.md #3374-3379, open):
//!   no "insert directly beneath the top card" placement verb exists; the
//!   YAML uses `place_as_bottom_source` with a documented cosmetic-position
//!   divergence (same resolution as BT13-088).
//! - **G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD** (new, filed by this
//!   pass): no YAML-reachable predicate/formula reads "distinct colors in
//!   the effect carrier's own non-flipped source stack" as a numeric value,
//!   so the printed "if this Digimon has 6 or more colors ... instead ..."
//!   branch discriminant cannot be authored. Both branches' step-level
//!   vocabulary (`color_matches_own_source_stack`, `delete_one_per_opponent_
//!   color`) IS wired and confirmed present, but with no way to gate between
//!   them, the ENTIRE delete clause is OMITTED from the YAML (not
//!   half-implemented) per the no-approximations policy and the established
//!   EX5-015 "omit rather than misfire" convention. This test file therefore
//!   has NO tests for the delete clause (there is nothing shipped to test)
//!   and instead has a structural test asserting the clause is indeed absent
//!   (documents the gap mechanically rather than leaving it silently
//!   untested).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledDistinctBy,
    CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/ex9/EX9-074.yaml");

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// A level 4 [DM]-trait Digimon material card. `color` sets its printed
/// color (used to drive the All-Turns DP-boost color-count formula).
fn make_dm_material(id: &str, name: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["DM".to_string()];
    c.colors = vec![color];
    c
}

/// A filler card irrelevant to any of Kimeramon's filters (not level 4, no
/// [DM] trait) — used to pad decks / rejects in negative-filter tests.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(3);
    c
}

fn make_opponent_digimon(id: &str, color: digimon_engine::enums::CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(4000);
    c.colors = vec![color];
    c
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let src = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(src);
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn kimeramon_runner() -> DebugRunner {
    use digimon_engine::enums::CardColor;
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-074 YAML parses")
        .add_card(make_filler("FILL"))
        .add_card(make_dm_material("MAT-RED", "Red Material", CardColor::Red))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex9_074_compiles() {
    let runner = kimeramon_runner();
    assert!(
        runner.compiled_card("EX9-074").is_some(),
        "EX9-074 must compile from YAML"
    );
}

#[test]
fn ex9_074_card_metadata_matches_print() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("EX9-074 compiles");

    assert_eq!(card.card, "EX9-074");
    assert_eq!(card.name, "Kimeramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(10));
    assert_eq!(card.dp, Some(10000));
    assert!(card.traits.contains(&"Composite".to_string()));
    assert!(card.traits.contains(&"DM".to_string()));
    assert!(card.traits.contains(&"Ver.3".to_string()));
}

/// Assembly alt-path: exactly 7 materials, each level-4 [DM]-trait,
/// `distinct_by: name`, sourced from trash, stacked under, reducing cost by
/// 7 (10 -> 3 at full Assembly).
#[test]
fn ex9_074_has_assembly_minus_seven_alt_path() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("EX9-074 compiles");

    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("an Assembly alt-path exists");

    assert_eq!(
        assembly.materials.len(),
        7,
        "Assembly -7 requires exactly 7 materials"
    );
    for (i, material) in assembly.materials.iter().enumerate() {
        assert_eq!(
            material.distinct_by,
            Some(CompiledDistinctBy::Name),
            "material {i} must require a pairwise-different name"
        );
        assert!(
            material.stack_under,
            "material {i} must stack under the evolved card"
        );
        assert!(
            material.zones.contains(&digimon_dsl::compiled::CompiledZone::Trash),
            "material {i} must be sourced from trash"
        );
    }
    assert_eq!(
        assembly.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(7)),
        "Assembly reduces the play cost by 7 (10 -> 3)"
    );
}

/// Exactly one triggered clause (the shared [On Play]/[When Digivolving]
/// optional placement body) — the delete sub-effect is OMITTED (see file
/// header), so it does not add a second clause, and there is no [Security]
/// clause on this card.
#[test]
fn ex9_074_has_exactly_one_triggered_clause() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("compiles");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "EX9-074 ships exactly one triggered clause (the optional trash-placement \
         body); the delete sub-effect is OMITTED pending \
         G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD"
    );
}

/// The single triggered clause fires on BOTH on_play and when_digivolving,
/// FaceUp scope, and is NOT optional at the clause level (DCGO's outer
/// ActivateClass is mandatory; the "may" lives on the inner select_trash).
#[test]
fn ex9_074_on_play_when_digivolving_clause_is_mandatory_faceup_dual_timing() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("an on_play clause exists");

    assert!(
        clause.when.contains(&CompiledTiming::WhenDigivolving),
        "the clause must be shared [On Play] / [When Digivolving]"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "the outer clause is mandatory — the 'you may' lives inside select_trash"
    );
    assert!(!clause.once_per_turn, "no printed [Once Per Turn] here");
}

/// The clause's process includes a `select_trash` step (the optional
/// placement pick) and a `place_as_bottom_source` step (the placement
/// itself — see the G-DSL-PLACE-AS-TOP-SOURCE fidelity note).
#[test]
fn ex9_074_clause_process_has_select_trash_and_place_as_bottom_source() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("on_play clause");

    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectTrash { .. })),
        "must include a select_trash step"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. })),
        "must include a place_as_bottom_source step"
    );
    assert!(
        !clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::DeleteOnePerOpponentColor { .. })),
        "the delete sub-effect (both branches) is OMITTED — no \
         delete_one_per_opponent_color step should be present"
    );
}

/// Declarative clauses: <Rush> keyword grant + <Security A. +1> aura +
/// All-Turns DP-boost aura. Exactly 3 declarative clauses (no more, no
/// fewer — the delete-related declaratives, if any existed, would not be
/// declarative anyway).
#[test]
fn ex9_074_has_three_declarative_clauses() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("EX9-074").expect("compiles");

    let declarative: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(d) => Some(d),
            _ => None,
        })
        .collect();
    assert_eq!(
        declarative.len(),
        3,
        "expected exactly 3 declarative clauses: <Rush>, <Security A. +1>, \
         and the All-Turns DP-boost aura"
    );

    let has_rush_grant = declarative
        .iter()
        .any(|d| matches!(d, CompiledDeclarativeClause::GrantKeyword { .. }));
    assert!(has_rush_grant, "must have a grant_keyword declarative (Rush)");

    let aura_count = declarative
        .iter()
        .filter(|d| matches!(d, CompiledDeclarativeClause::Aura { .. }))
        .count();
    assert_eq!(
        aura_count, 2,
        "must have exactly 2 aura declaratives: <Security A. +1> and the DP boost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 2 — <Rush> keyword grant
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex9_074_grants_rush_keyword() {
    let mut runner = kimeramon_runner();
    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    runner.game_mut().tick_declarative_effects();

    assert!(
        runner.game.has_keyword(kimera, Keyword::Rush),
        "Kimeramon must have <Rush> (can attack the turn it enters play)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 3 — <Security A. +1> aura
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ex9_074_grants_security_attack_plus_one() {
    let mut runner = kimeramon_runner();
    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    runner.game_mut().tick_declarative_effects();

    // The `security_attack: 1` self-aura (a flat literal, not a formula)
    // materializes a `ModifierType::SecurityAttackChange` modifier via
    // `tick_declarative_effects` (dsl_cards/lower_aura.rs's
    // materializes_declarative_state path for a self-target flat grant).
    // `security_attack_keyword_bonus` only reads `Keyword::SecurityAttackPlus`
    // grants (a DIFFERENT declarative path, `grant_keyword`), so it stays 0
    // here — the correct total-strike query is `effective_security_strike`,
    // which folds in base checks (1) + the SecurityAttackChange modifier sum.
    assert_eq!(
        runner.game.effective_security_strike(kimera),
        2,
        "Kimeramon must check 1 (base) + 1 (<Security A. +1>) = 2 security cards"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 4 — [On Play] optional trash-placement (positive / negative /
// filter rejection)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with an eligible Lv.4 [DM] Digimon in trash, the [On Play]
/// clause installs the optional trash-selection prompt.
#[test]
fn ex9_074_on_play_with_eligible_material_in_trash_installs_prompt() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_on_play(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("optional trash-selection prompt installs");
    assert_eq!(
        view.kind,
        digimon_engine::selection::SelectionKind::Trash,
        "select_trash installs a Trash selection"
    );
    assert!(
        runner.pending_is_optional(),
        "the placement pick is a genuine 'you may'"
    );
    let _ = CardColor::Red; // silence unused import if optimized away
}

/// Negative: with NO eligible material in trash, no prompt installs at all
/// (the optional select auto-completes with zero candidates) and no
/// placement occurs.
#[test]
fn ex9_074_on_play_with_no_eligible_material_does_nothing() {
    let mut runner = kimeramon_runner();

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    let sources_before = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .len();
    fire_on_play(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash card -> the optional select auto-completes, no prompt"
    );
    assert_eq!(
        runner.game.players[0].battle_area[kimera.index as usize]
            .card_sources
            .len(),
        sources_before,
        "no digivolution source was added"
    );
}

/// Filter rejection: a trash card that is level 5 (too high) must NOT be a
/// legal placement target.
#[test]
fn ex9_074_on_play_filter_rejects_level_five_material() {
    let mut runner = kimeramon_runner();
    let mut too_high = make_test_card("MAT-LV5", "TooHighLevel");
    too_high.level = Some(5);
    too_high.traits = vec!["DM".to_string()];
    runner.game.card_data.push(too_high);
    push_to_trash(&mut runner, 0, "MAT-LV5");

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_on_play(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "a level-5 card is not level <= 4 -> no eligible target -> no prompt"
    );
}

/// Filter rejection: a trash card without the [DM] trait must NOT be a legal
/// placement target.
#[test]
fn ex9_074_on_play_filter_rejects_non_dm_material() {
    let mut runner = kimeramon_runner();
    let mut non_dm = make_test_card("MAT-NODM", "NotDM");
    non_dm.level = Some(4);
    runner.game.card_data.push(non_dm);
    push_to_trash(&mut runner, 0, "MAT-NODM");

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_on_play(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "no [DM] trait -> no eligible target -> no prompt"
    );
}

/// Filter rejection: a non-Digimon (Tamer) card in trash, even if level/trait
/// otherwise match, must NOT be a legal placement target (`kind: digimon`).
#[test]
fn ex9_074_on_play_filter_rejects_non_digimon() {
    let mut runner = kimeramon_runner();
    let mut tamer = make_test_card("MAT-TAMER", "TamerDM");
    tamer.card_kind = CardKind::Tamer;
    tamer.traits = vec!["DM".to_string()];
    runner.game.card_data.push(tamer);
    push_to_trash(&mut runner, 0, "MAT-TAMER");

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_on_play(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "kind: digimon filter must reject a Tamer card"
    );
}

/// Accepting the placement moves the trash card into Kimeramon's
/// digivolution-source stack and removes it from trash. Position (bottom vs
/// top) is the documented G-DSL-PLACE-AS-TOP-SOURCE divergence.
#[test]
fn ex9_074_on_play_accepting_placement_moves_card_to_source_stack() {
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");
    let trash_before = runner.trash_size(0);

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_on_play(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("trash prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place the material");
    runner.auto_resolve().expect("finish clause");

    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "the placed material must leave the trash"
    );
    let is_source = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .iter()
        .any(|src| src.card_id(&runner.game.card_data) == "MAT-RED");
    assert!(
        is_source,
        "MAT-RED must now be a digivolution source under Kimeramon"
    );
}

/// Declining the placement (optional decline) leaves the trash and the
/// digivolution stack untouched.
#[test]
fn ex9_074_on_play_declining_placement_changes_nothing() {
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");
    let trash_before = runner.trash_size(0);

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    let sources_before = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .len();
    fire_on_play(&mut runner, kimera);

    runner
        .pending_selection_view()
        .expect("trash prompt installs");
    runner.execute_action(0, PASS).expect("decline placement");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must not remove anything from trash"
    );
    assert_eq!(
        runner.game.players[0].battle_area[kimera.index as usize]
            .card_sources
            .len(),
        sources_before,
        "declining must not add a digivolution source"
    );
}

/// The clause also fires on [When Digivolving] (shared body) — same prompt,
/// same outcome shape.
#[test]
fn ex9_074_when_digivolving_also_installs_placement_prompt() {
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");

    let kimera = runner.place_on_field(0, "EX9-074", Some(0));
    fire_when_digivolving(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving installs the same trash prompt");
    assert_eq!(
        view.kind,
        digimon_engine::selection::SelectionKind::Trash,
        "select_trash installs a Trash selection on when_digivolving too"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 5 — [All Turns] +1000 DP per color in digivolution cards
// ═══════════════════════════════════════════════════════════════════════════

/// With ZERO digivolution sources (no materials beneath the top card), the
/// DP boost contributes 0 — base DP is unchanged.
#[test]
fn ex9_074_all_turns_dp_boost_zero_materials_no_bonus() {
    let runner = kimeramon_runner();
    let mut r = runner;
    let kimera = r.place_on_field(0, "EX9-074", Some(0));

    assert_eq!(
        r.effective_dp(kimera),
        Some(10000),
        "0 distinct colors among 0 materials -> +0 DP -> base 10000 unchanged"
    );
}

/// With exactly 1 material of 1 color beneath the top card, the boost is
/// exactly +1000 DP (1 distinct color).
#[test]
fn ex9_074_all_turns_dp_boost_one_color_material_adds_1000() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    let kimera = runner.place_stack(0, &["MAT-RED", "EX9-074"]);

    assert_eq!(
        runner.effective_dp(kimera),
        Some(11000),
        "1 distinct color among sources -> +1000 DP -> 11000 total"
    );
    let _ = CardColor::Red;
}

/// With materials spanning 3 DISTINCT colors beneath the top card, the boost
/// is +3000 DP — same color repeated does not double-count.
#[test]
fn ex9_074_all_turns_dp_boost_three_distinct_colors_adds_3000() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    runner
        .game
        .card_data
        .push(make_dm_material("MAT-BLUE", "Blue Material", CardColor::Blue));
    runner
        .game
        .card_data
        .push(make_dm_material("MAT-YELLOW", "Yellow Material", CardColor::Yellow));
    // A second red material must NOT add a 4th distinct color (dedup).
    runner
        .game
        .card_data
        .push(make_dm_material("MAT-RED2", "Red Material 2", CardColor::Red));

    let kimera = runner.place_stack(
        0,
        &["MAT-RED", "MAT-BLUE", "MAT-YELLOW", "MAT-RED2", "EX9-074"],
    );

    assert_eq!(
        runner.effective_dp(kimera),
        Some(13000),
        "3 distinct colors (red, blue, yellow; the 2nd red does not add a 4th) \
         -> +3000 DP -> 13000 total"
    );
}

/// The DP boost is symmetric / re-evaluated live: pushing an additional
/// distinctly-colored source onto an already-placed Kimeramon raises the
/// effective DP immediately (no "install once" staleness).
#[test]
fn ex9_074_all_turns_dp_boost_updates_live_when_source_added() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    runner
        .game
        .card_data
        .push(make_dm_material("MAT-GREEN", "Green Material", CardColor::Green));

    let kimera = runner.place_stack(0, &["MAT-RED", "EX9-074"]);
    assert_eq!(runner.effective_dp(kimera), Some(11000));

    runner.push_source(kimera, "MAT-GREEN");

    assert_eq!(
        runner.effective_dp(kimera),
        Some(12000),
        "adding a 2nd distinct-color source must raise the live DP total to 12000"
    );
}
