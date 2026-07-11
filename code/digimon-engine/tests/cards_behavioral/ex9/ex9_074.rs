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
//!   (non-cost) select_trash + place_as_top_source
//! - Formula-driven self DP aura (`source_color_count`, all-turns, symmetric)
//! - Branch-gated deletion: `if: { own_source_stack_color_count_gte: 6 }` —
//!   Branch A (<=5 colors) = mandatory single same-color delete
//!   (`color_matches_own_source_stack` filter), Branch B (>=6 colors) =
//!   `delete_one_per_opponent_color` per-color mandatory picks + batch delete
//!
//! # Known engine/DSL gaps affecting this card (see YAML header for the full
//! writeup)
//!
//! - **G-DSL-PLACE-AS-TOP-SOURCE** (✅ RESOLVED 2026-07-05): the
//!   `place_as_top_source` verb now inserts directly beneath the top card
//!   (engine `Permanent::push_as_top_source`), so the printed "as this
//!   Digimon's top digivolution card" position is exact — the former
//!   cosmetic bottom-position divergence (BT13-088 precedent) is gone.
//!   Substrate proof: tests/dsl/place_as_top_source.rs; position pinned
//!   here in `ex9_074_on_play_accepting_placement_moves_card_to_source_stack`.
//! - **G-DSL-OWN-SOURCE-STACK-COLOR-COUNT-THRESHOLD** (✅ RESOLVED
//!   2026-07-03): the `own_source_stack_color_count_gte` predicate leaf now
//!   reads "distinct colors in the effect carrier's own non-flipped source
//!   stack" as a no-subject numeric gate (shared extraction with
//!   `color_matches_own_source_stack`), so the printed "if this Digimon has
//!   6 or more colors ... instead ..." branch discriminant is authored
//!   natively. Both branches ship; SECTION 6 exercises them behaviorally.
//!   Substrate proof: tests/dsl/kimeramon_color_mass_delete.rs.

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
            material
                .zones
                .contains(&digimon_dsl::compiled::CompiledZone::Trash),
            "material {i} must be sourced from trash"
        );
    }
    assert_eq!(
        assembly.cost,
        Some(digimon_dsl::compiled::CompiledCost::Literal(7)),
        "Assembly reduces the play cost by 7 (10 -> 3)"
    );
}

/// Exactly one triggered clause — the shared [On Play]/[When Digivolving]
/// body carries BOTH Part 1 (the optional trash placement) and Part 2 (the
/// branch-gated deletion, nested inside its `if:` step), and there is no
/// [Security] clause on this card.
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
        "EX9-074 ships exactly one triggered clause (placement + branch-gated \
         deletion share the single [On Play]/[When Digivolving] body)"
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
/// placement pick), a `place_as_top_source` step (the placement itself, at
/// the printed TOP-source position — G-DSL-PLACE-AS-TOP-SOURCE resolved
/// 2026-07-05), and the Part-2 `if:` step gated on
/// `own_source_stack_color_count_gte: 6` — Branch B (then) is
/// `delete_one_per_opponent_color`, Branch A (else) is the mandatory
/// same-color single delete.
#[test]
fn ex9_074_clause_process_has_placement_and_branch_gated_delete() {
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
            .any(|s| matches!(s, CompiledStep::PlaceAsTopSource { .. })),
        "must include a place_as_top_source step (printed: 'as this \
         Digimon's top digivolution card')"
    );
    assert!(
        !clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. })),
        "must NOT use place_as_bottom_source — the printed position is TOP \
         (G-DSL-PLACE-AS-TOP-SOURCE resolved 2026-07-05)"
    );

    let branch = clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::If {
                condition,
                then,
                else_branch,
            } => Some((condition, then, else_branch)),
            _ => None,
        })
        .expect("must include the Part-2 `if:` branch gate");
    let (condition, then, else_branch) = branch;
    assert_eq!(
        condition.own_source_stack_color_count_gte,
        Some(6),
        "the branch discriminant is own_source_stack_color_count_gte: 6"
    );
    assert!(
        then.iter()
            .any(|s| matches!(s, CompiledStep::DeleteOnePerOpponentColor { .. })),
        "Branch B (>=6 colors) is delete_one_per_opponent_color"
    );
    assert!(
        else_branch
            .iter()
            .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })),
        "Branch A (<=5 colors) ends in a delete_permanent of the same-color pick"
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
    assert!(
        has_rush_grant,
        "must have a grant_keyword declarative (Rush)"
    );

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
/// digivolution-source stack — at the printed TOP-source position, directly
/// beneath the top card (G-DSL-PLACE-AS-TOP-SOURCE resolved 2026-07-05).
/// Kimeramon starts with two pre-existing sources so the top-source slot is
/// distinguishable from both the bottom and any middle slot.
#[test]
fn ex9_074_on_play_accepting_placement_moves_card_to_source_stack() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    runner.game.card_data.push(make_dm_material(
        "MAT-BLUE",
        "Blue Material",
        CardColor::Blue,
    ));
    push_to_trash(&mut runner, 0, "MAT-RED");
    let trash_before = runner.trash_size(0);

    let kimera = runner.place_stack(0, &["MAT-BLUE", "FILL", "EX9-074"]);
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
    let stack_ids: Vec<&str> = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .iter()
        .map(|src| src.card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        stack_ids,
        vec!["MAT-BLUE", "FILL", "MAT-RED", "EX9-074"],
        "MAT-RED must land as the TOP digivolution source — directly beneath \
         the top card, with the pre-existing sources' order and the top card \
         unchanged (printed: 'as this Digimon's top digivolution card')"
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
    runner.game.card_data.push(make_dm_material(
        "MAT-BLUE",
        "Blue Material",
        CardColor::Blue,
    ));
    runner.game.card_data.push(make_dm_material(
        "MAT-YELLOW",
        "Yellow Material",
        CardColor::Yellow,
    ));
    // A second red material must NOT add a 4th distinct color (dedup).
    runner.game.card_data.push(make_dm_material(
        "MAT-RED2",
        "Red Material 2",
        CardColor::Red,
    ));

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
    runner.game.card_data.push(make_dm_material(
        "MAT-GREEN",
        "Green Material",
        CardColor::Green,
    ));

    let kimera = runner.place_stack(0, &["MAT-RED", "EX9-074"]);
    assert_eq!(runner.effective_dp(kimera), Some(11000));

    runner.push_source(kimera, "MAT-GREEN");

    assert_eq!(
        runner.effective_dp(kimera),
        Some(12000),
        "adding a 2nd distinct-color source must raise the live DP total to 12000"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 6 — Part-2 branch-gated deletion
// (own_source_stack_color_count_gte: 6 — resolved 2026-07-03)
// ═══════════════════════════════════════════════════════════════════════════

/// Six distinctly-colored [DM] materials — enough to flip the branch gate.
fn add_six_color_materials(runner: &mut DebugRunner) {
    use digimon_engine::enums::CardColor;
    for (id, name, color) in [
        ("MAT-BLUE", "Blue Material", CardColor::Blue),
        ("MAT-YELLOW", "Yellow Material", CardColor::Yellow),
        ("MAT-GREEN", "Green Material", CardColor::Green),
        ("MAT-BLACK", "Black Material", CardColor::Black),
        ("MAT-PURPLE", "Purple Material", CardColor::Purple),
    ] {
        runner
            .game
            .card_data
            .push(make_dm_material(id, name, color));
    }
    // MAT-RED already registered by kimeramon_runner().
}

/// Resolve the currently-pending mandatory opponent pick by taking the first
/// legal action, asserting it is NOT optional.
fn resolve_mandatory_pick(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("a mandatory opponent pick must be pending");
    assert!(
        !runner.pending_is_optional(),
        "the Part-2 deletion picks are MANDATORY (DCGO canNoSelect: false)"
    );
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a non-PASS pick must be available");
    let player = view.selecting_player;
    runner
        .game
        .resolve_selection(player, action)
        .expect("pick resolves");
    runner.game.drain_effect_queue();
}

/// Branch A (<=5 source colors): exactly ONE mandatory pick, offering only
/// opponent Digimon that share a color with a non-flipped digivolution
/// source; the off-color Digimon survives.
#[test]
fn ex9_074_on_play_below_six_colors_deletes_one_same_color_opponent_digimon() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    runner
        .game
        .card_data
        .push(make_opponent_digimon("OPP-RED", CardColor::Red));
    runner
        .game
        .card_data
        .push(make_opponent_digimon("OPP-GREEN", CardColor::Green));

    // 1 red source (1 distinct color -> Branch A). Empty trash -> Part 1
    // self-skips with no prompt, so the first prompt IS the Branch-A pick.
    let kimera = runner.place_stack(0, &["MAT-RED", "EX9-074"]);
    runner.place_on_field(1, "OPP-RED", Some(0));
    runner.place_on_field(1, "OPP-GREEN", Some(1));

    fire_on_play(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("Branch A installs the mandatory same-color pick");
    let candidates = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(
        candidates, 1,
        "only OPP-RED (shares Red with a source) may be offered; OPP-GREEN is excluded"
    );
    resolve_mandatory_pick(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "Branch A makes exactly ONE pick"
    );
    let survivors: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        survivors,
        vec!["OPP-GREEN".to_string()],
        "the same-color Digimon is deleted; the off-color one survives"
    );
}

/// Branch A with NO same-color opponent Digimon (here: no digivolution
/// sources at all -> empty color set): the mandatory pick self-skips (DCGO's
/// HasMatchConditionOpponentsPermanent guard) and nothing is deleted.
#[test]
fn ex9_074_on_play_below_six_colors_no_same_color_target_self_skips() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    runner
        .game
        .card_data
        .push(make_opponent_digimon("OPP-GREEN", CardColor::Green));

    let kimera = runner.place_on_field(0, "EX9-074", Some(0)); // no sources
    runner.place_on_field(1, "OPP-GREEN", Some(0));

    fire_on_play(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "empty source color set -> no legal same-color target -> the pick self-skips"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "nothing is deleted when Branch A has no legal target"
    );
}

/// Branch B (>=6 source colors): one MANDATORY pick per distinct color
/// present among the opponent's Digimon, batch-deleted.
#[test]
fn ex9_074_on_play_six_plus_colors_deletes_one_per_distinct_opponent_color() {
    use digimon_engine::enums::CardColor;
    let mut runner = kimeramon_runner();
    add_six_color_materials(&mut runner);
    runner
        .game
        .card_data
        .push(make_opponent_digimon("OPP-RED", CardColor::Red));
    runner
        .game
        .card_data
        .push(make_opponent_digimon("OPP-GREEN", CardColor::Green));

    // 6 distinct source colors -> Branch B.
    let kimera = runner.place_stack(
        0,
        &[
            "MAT-RED",
            "MAT-BLUE",
            "MAT-YELLOW",
            "MAT-GREEN",
            "MAT-BLACK",
            "MAT-PURPLE",
            "EX9-074",
        ],
    );
    runner.place_on_field(1, "OPP-RED", Some(0));
    runner.place_on_field(1, "OPP-GREEN", Some(1));

    fire_on_play(&mut runner, kimera);

    // Two colors present among opponent Digimon (Red, Green) -> exactly two
    // mandatory picks, then the batch delete fires.
    resolve_mandatory_pick(&mut runner);
    resolve_mandatory_pick(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "both colors resolved -> batch delete, no further prompt"
    );
    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "Branch B deletes one opponent Digimon per distinct color present"
    );
}
