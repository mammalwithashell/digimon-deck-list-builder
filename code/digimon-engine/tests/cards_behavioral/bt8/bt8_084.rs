//! BT8-084 Kimeramon — Digimon, Lv.5, White, DP 8000, Cost 8.
//! Traits: Composite. Attribute: Data. Form: Ultimate.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT8-084.md, verbatim)
//!
//! [DNA Digivolve] 0 from Lv.4 + Lv.4 — Digivolve unsuspended with the 2
//! specified Digimon stacked on top of each other.
//!
//! [When Digivolving] You may place 1 level 5 or lower Digimon card from
//! your trash under this Digimon as its bottom digivolution card. Then, up
//! to 4 of your opponent's Digimon get -1000 DP for each of this Digimon's
//! colors until the end of your opponent's next turn.
//! [Your Turn] This Digimon is treated as also having the colors of its
//! digivolution cards. While this Digimon has 4 or more colors, it gets
//! +4000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/White/BT8_084.cs
//!
//! # Patterns this test covers
//! - Printed DNA digivolve alt-path (Lv.4 + Lv.4, cost 0)
//! - [When Digivolving] optional (non-cost) select_trash +
//!   place_as_bottom_source (printed BOTTOM position — sibling of EX9-074's
//!   top-source shape)
//! - Gap tripwires: the DP-minus leg and both [Your Turn] color clauses are
//!   gap-blocked (see below) — structural tests PIN that no approximated
//!   substitute shipped.
//!
//! # Known engine/DSL gaps affecting this card (full writeup in the YAML
//! header + qa/dsl-vocab-gaps.md)
//!
//! - **G-DSL-SOURCE-STACK-UNION-COLOR-COUNT** — no source-anchored formula
//!   for "distinct colors across the carrier's top card + non-flipped
//!   sources" (union incl. top). `source_color_count` is sources-only;
//!   `digivolution_color_count` anchors at the formula TARGET (here: the
//!   opponent Digimon receiving the debuff). Blocks the "-1000 DP for each
//!   of this Digimon's colors" amount, so the whole "Then, up to 4 ..."
//!   leg is unauthorable without approximating.
//! - **G-DSL-OWN-STACK-COLOR-COUNT-GTE** — `own_source_stack_color_count_gte`
//!   counts source colors EXCLUDING the top card; the printed "While this
//!   Digimon has 4 or more colors" needs union-incl-top (white top +
//!   3-color sources = 4 must qualify). Blocks the +4000 DP aura gate.
//! - **G-ENGINE-ADDITIVE-COLOR-TREATMENT** — "[Your Turn] treated as also
//!   having the colors of its digivolution cards" is a cross-card-visible
//!   continuous color grant. `ModifierType::AddColor` exists in the enum but
//!   is never read by `Permanent::synth_identity` (only the replace-style
//!   `ChangeBaseCardColor` is), and no DSL aura surface installs a dynamic
//!   union-color payload. Engine primitive missing.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt8/BT8-084.yaml");

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A level-5 Digimon card — an eligible "level 5 or lower Digimon card"
/// placement target for the [When Digivolving] clause.
fn make_lv5_digimon(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(5);
    c.dp = Some(7000);
    c.colors = vec![color];
    c
}

/// Filler card irrelevant to Kimeramon's filters.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(3);
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

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn kimeramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT8-084 YAML parses")
        .add_card(make_filler("FILL"))
        .add_card(make_lv5_digimon("MAT-RED", "Red Material", CardColor::Red))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt8_084_compiles() {
    let runner = kimeramon_runner();
    assert!(
        runner.compiled_card("BT8-084").is_some(),
        "BT8-084 must compile from YAML"
    );
}

#[test]
fn bt8_084_card_metadata_matches_print() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("BT8-084 compiles");

    assert_eq!(card.card, "BT8-084");
    assert_eq!(card.name, "Kimeramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(8000));
    assert!(card.traits.contains(&"Composite".to_string()));
}

/// Printed DNA path: 0 from Lv.4 + Lv.4, stacked unsuspended.
#[test]
fn bt8_084_has_dna_digivolve_alt_path() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    let dna_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(
        dna_paths.len(),
        1,
        "BT8-084 must have exactly one DNA digivolve alt-path"
    );
    let path = dna_paths[0];
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "printed DNA cost is 0"
    );
    assert_eq!(
        path.materials.len(),
        2,
        "DNA requires exactly 2 materials (Lv.4 + Lv.4)"
    );
    for (i, material) in path.materials.iter().enumerate() {
        assert_eq!(
            material.filter.level_eq,
            Some(4),
            "DNA material {i} must be a level-4 Digimon (Levels_ForJogress \
             .Contains(4) in DCGO BT8_084.cs)"
        );
    }
    assert!(
        path.stacks_unsuspended,
        "printed: 'Digivolve unsuspended with the 2 specified Digimon \
         stacked on top of each other'"
    );
}

/// Exactly one triggered clause: the [When Digivolving] body. It must fire
/// on when_digivolving ONLY (the printed text has no [On Play]) and be
/// mandatory at the clause level — the "you may" lives on the inner
/// select_trash.
#[test]
fn bt8_084_when_digivolving_clause_shape() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "BT8-084 ships exactly one triggered clause (the [When Digivolving] body)"
    );
    let clause = triggered[0];
    assert!(
        clause.when.contains(&CompiledTiming::WhenDigivolving),
        "clause must fire on when_digivolving"
    );
    assert!(
        !clause.when.contains(&CompiledTiming::OnPlay),
        "printed text is [When Digivolving] only — no [On Play]"
    );
    assert!(
        !clause.optional,
        "the outer clause is mandatory — the 'you may' lives inside select_trash"
    );

    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectTrash { .. })),
        "must include the optional select_trash placement pick"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. })),
        "must place the picked card as the BOTTOM digivolution card \
         (printed: 'under this Digimon as its bottom digivolution card')"
    );
    assert!(
        !clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsTopSource { .. })),
        "the printed position is BOTTOM, not top (contrast EX9-074)"
    );
}

/// GAP TRIPWIRE — the "Then, up to 4 of your opponent's Digimon get -1000
/// DP for each of this Digimon's colors" leg is BLOCKED on
/// G-DSL-SOURCE-STACK-UNION-COLOR-COUNT (no source-anchored union-incl-top
/// color-count formula). Until that gap closes, the clause must NOT carry an
/// approximated DP-minus (e.g. `source_color_count`, which undercounts by
/// 1000 whenever white is absent from the sources). When this test starts
/// failing because the leg was authored, verify the amount formula is the
/// NEW union-incl-top vocabulary, then update this tripwire into a positive
/// assertion.
#[test]
fn bt8_084_gap_tripwire_no_approximated_dp_minus_leg() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    for clause in &card.effects {
        if let CompiledClause::Triggered(t) = clause {
            assert!(
                !contains_step_recursive(&t.process, &|s| matches!(
                    s,
                    CompiledStep::AddDpModifier { .. }
                        | CompiledStep::SelectCountCappedMulti { .. }
                )),
                "the DP-minus leg is gap-blocked (G-DSL-SOURCE-STACK-UNION-\
                 COLOR-COUNT); shipping it with an approximated amount \
                 violates no-approximations"
            );
        }
    }
}

/// GAP TRIPWIRE — both [Your Turn] clauses (treated-as-colors + the 4-color
/// +4000 DP boost) are BLOCKED (G-ENGINE-ADDITIVE-COLOR-TREATMENT /
/// G-DSL-OWN-STACK-COLOR-COUNT-GTE). No declarative clause may ship a
/// stand-in (`own_source_stack_color_count_gte: 4` excludes the white top
/// card and is NOT the printed gate).
#[test]
fn bt8_084_gap_tripwire_no_approximated_your_turn_color_clauses() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    let declarative_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(
        declarative_count, 0,
        "both [Your Turn] color clauses are gap-blocked; no declarative \
         stand-in may ship (see YAML header + qa/dsl-vocab-gaps.md)"
    );
}

fn contains_step_recursive(steps: &[CompiledStep], pred: &dyn Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|s| {
        if pred(s) {
            return true;
        }
        // Recurse into the nested step containers this card could plausibly
        // hide a step inside (if/then/else, per_selected, select tails).
        match s {
            CompiledStep::If {
                then, else_branch, ..
            } => contains_step_recursive(then, pred) || contains_step_recursive(else_branch, pred),
            CompiledStep::PerSelected { body, .. } => contains_step_recursive(body, pred),
            CompiledStep::ForEach { body, .. } => contains_step_recursive(body, pred),
            _ => false,
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 2 — [When Digivolving] optional bottom-source placement
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with an eligible Lv.5-or-lower Digimon in trash, the clause
/// installs the optional trash-selection prompt.
#[test]
fn bt8_084_when_digivolving_with_eligible_card_installs_prompt() {
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");

    let kimera = runner.place_on_field(0, "BT8-084", Some(0));
    fire_when_digivolving(&mut runner, kimera);

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
}

/// Accepting the placement moves the trash card to the BOTTOM of
/// Kimeramon's digivolution stack. Kimeramon starts with two pre-existing
/// sources so the bottom slot is distinguishable from the top-source and
/// middle slots.
#[test]
fn bt8_084_accepting_placement_puts_card_at_stack_bottom() {
    let mut runner = kimeramon_runner();
    runner
        .game
        .card_data
        .push(make_lv5_digimon("MAT-BLUE", "Blue Material", CardColor::Blue));
    push_to_trash(&mut runner, 0, "MAT-RED");
    let trash_before = runner.trash_size(0);

    let kimera = runner.place_stack(0, &["MAT-BLUE", "FILL", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);

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
        "the placed card must leave the trash"
    );
    let stack_ids: Vec<&str> = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .iter()
        .map(|src| src.card_id(&runner.game.card_data))
        .collect();
    assert_eq!(
        stack_ids,
        vec!["MAT-RED", "MAT-BLUE", "FILL", "BT8-084"],
        "MAT-RED must land as the BOTTOM digivolution card — beneath the \
         pre-existing sources, whose order (and the top card) is unchanged \
         (printed: 'under this Digimon as its bottom digivolution card')"
    );
}

/// Declining the optional placement changes nothing (non-cost optional: the
/// decline only skips the placement itself).
#[test]
fn bt8_084_declining_placement_changes_nothing() {
    let mut runner = kimeramon_runner();
    push_to_trash(&mut runner, 0, "MAT-RED");
    let trash_before = runner.trash_size(0);

    let kimera = runner.place_on_field(0, "BT8-084", Some(0));
    let sources_before = runner.game.players[0].battle_area[kimera.index as usize]
        .card_sources
        .len();
    fire_when_digivolving(&mut runner, kimera);

    runner
        .pending_selection_view()
        .expect("trash prompt installs");
    runner.execute_action(0, PASS).expect("decline placement");
    runner.auto_resolve().ok();

    assert_eq!(runner.trash_size(0), trash_before, "trash unchanged");
    assert_eq!(
        runner.game.players[0].battle_area[kimera.index as usize]
            .card_sources
            .len(),
        sources_before,
        "no digivolution source was added"
    );
}

/// Negative: an empty / ineligible trash → no prompt at all.
#[test]
fn bt8_084_no_eligible_trash_card_no_prompt() {
    let mut runner = kimeramon_runner();

    let kimera = runner.place_on_field(0, "BT8-084", Some(0));
    fire_when_digivolving(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash card -> the optional select auto-completes, no prompt"
    );
}

/// Filter rejection: a level-6 Digimon is NOT "level 5 or lower".
#[test]
fn bt8_084_filter_rejects_level_six_card() {
    let mut runner = kimeramon_runner();
    let mut too_high = make_test_card("MAT-LV6", "TooHighLevel");
    too_high.level = Some(6);
    runner.game.card_data.push(too_high);
    push_to_trash(&mut runner, 0, "MAT-LV6");

    let kimera = runner.place_on_field(0, "BT8-084", Some(0));
    fire_when_digivolving(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "a level-6 card is not level <= 5 -> no eligible target -> no prompt"
    );
}

/// Filter rejection: a Tamer card in trash must NOT be a legal placement
/// target ("Digimon card").
#[test]
fn bt8_084_filter_rejects_non_digimon() {
    let mut runner = kimeramon_runner();
    let mut tamer = make_test_card("MAT-TAMER", "SomeTamer");
    tamer.card_kind = CardKind::Tamer;
    runner.game.card_data.push(tamer);
    push_to_trash(&mut runner, 0, "MAT-TAMER");

    let kimera = runner.place_on_field(0, "BT8-084", Some(0));
    fire_when_digivolving(&mut runner, kimera);

    assert!(
        runner.pending_selection().is_none(),
        "kind: digimon filter must reject a Tamer card"
    );
}
