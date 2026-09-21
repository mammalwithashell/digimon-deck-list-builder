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
//! - [When Digivolving] up-to-4 DP-minus scaled by this Digimon's RULES
//!   colors, read after the placement (`per: source_rules_color_count`)
//! - [Your Turn] additive color treatment (`modifier: AddColor` self-aura;
//!   non-flipped sources only) and the 4-color +4000 DP boost
//!   (`self_color_count_gte: 4`)
//!
//! # Gaps closed by this card (2026-09-19)
//!
//! - G-DSL-SOURCE-STACK-UNION-COLOR-COUNT — new per-selector
//!   `source_rules_color_count` (the carrier's synthesized colors).
//! - G-ENGINE-ADDITIVE-COLOR-TREATMENT — `Permanent::synth_identity` now reads
//!   `ModifierType::AddColor` additively (payload `None` = the non-flipped
//!   digivolution cards' colors).
//! - G-DSL-OWN-STACK-COLOR-COUNT-GTE — collapses into `self_color_count_gte`
//!   once the treatment is live.

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

/// The "Then, up to 4 of your opponent's Digimon get -1000 DP for each of
/// this Digimon's colors" leg is authored (G-DSL-SOURCE-STACK-UNION-COLOR-COUNT
/// RESOLVED): an up-to-4 opponent pick + a per-target `add_dp_modifier` whose
/// amount is the carrier's RULES color count (`source_rules_color_count` —
/// DCGO `TopCard.CardColors.Count`, read with the [Your Turn]
/// ChangeCardColorClass live), until the end of the opponent's next turn.
#[test]
fn bt8_084_dp_minus_leg_is_authored() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    let triggered = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("[When Digivolving] clause");
    assert!(
        contains_step_recursive(&triggered.process, &|s| matches!(
            s,
            CompiledStep::SelectCountCappedMulti { .. }
        )),
        "the up-to-4 opponent Digimon pick is authored"
    );
    assert!(
        contains_step_recursive(&triggered.process, &|s| matches!(
            s,
            CompiledStep::AddDpModifier { .. }
        )),
        "the -1000-per-color DP modifier is authored"
    );
}

/// Both [Your Turn] clauses ship as declarative self-auras
/// (G-ENGINE-ADDITIVE-COLOR-TREATMENT / G-DSL-OWN-STACK-COLOR-COUNT-GTE
/// RESOLVED): the additive color treatment and the 4-color +4000 DP boost.
#[test]
fn bt8_084_your_turn_color_clauses_are_authored() {
    let runner = kimeramon_runner();
    let card = runner.compiled_card("BT8-084").expect("compiles");

    let declarative_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Declarative(_)))
        .count();
    assert_eq!(
        declarative_count, 2,
        "[Your Turn] treated-as-colors + [Your Turn] 4-color +4000 DP"
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

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 3 — [Your Turn] additive color treatment + 4-color +4000 DP
// ═══════════════════════════════════════════════════════════════════════════
//
// DCGO BT8_084.cs: ChangeCardColorClass (IsOwnerTurn && >= 1 digivolution
// card) appends every NON-FLIPPED digivolution card's colors to the top
// card's CardColors; ChangeSelfDPStaticEffect(4000) gated on IsOwnerTurn &&
// TopCard.CardColors.Count >= 4. Official Q&A: a white card with red + green
// sources "is treated as a 3-color white/red/green card".

fn colored_digimon(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = colors.to_vec();
    c
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(6);
    c.dp = Some(12000);
    c.colors = vec![CardColor::Red];
    c
}

fn color_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT8-084 YAML parses")
        .add_card(make_filler("FILL"))
        .add_card(colored_digimon("SRC-RG", &[CardColor::Red, CardColor::Green]))
        .add_card(colored_digimon("SRC-BLUE", &[CardColor::Blue]))
        .add_card(colored_digimon("SRC-WHITE", &[CardColor::White]))
        .add_card(make_lv5_digimon("TRASH-YELLOW", "Yellow Material", CardColor::Yellow))
        .add_card(opp_digimon("OPP-A"))
        .add_card(opp_digimon("OPP-B"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start()
}

fn sorted(mut v: Vec<CardColor>) -> Vec<CardColor> {
    v.sort_by_key(|c| *c as u8);
    v
}

fn rules_colors(runner: &mut DebugRunner, h: PermanentHandle) -> Vec<CardColor> {
    runner.game.tick_declarative_effects();
    let perm = &runner.game.players[h.player as usize].battle_area[h.index as usize];
    sorted(perm.colors_for_rules(&runner.game.card_data, &runner.game.modifiers, h))
}

#[test]
fn bt8_084_your_turn_treated_as_also_having_source_colors() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-RG", "BT8-084"]);
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(
        rules_colors(&mut runner, kimera),
        sorted(vec![CardColor::White, CardColor::Red, CardColor::Green]),
        "Q&A: white + red/green source = a 3-color white/red/green card"
    );
}

#[test]
fn bt8_084_opponent_turn_has_only_printed_color() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-RG", "BT8-084"]);
    runner.set_first_player(1);
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        rules_colors(&mut runner, kimera),
        vec![CardColor::White],
        "the treatment is [Your Turn] only (DCGO IsOwnerTurn)"
    );
}

#[test]
fn bt8_084_face_down_source_colors_not_added() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-BLUE", "SRC-RG", "BT8-084"]);
    runner.game.players[0].battle_area[kimera.index as usize].card_sources[1].face_down = true;
    assert_eq!(
        rules_colors(&mut runner, kimera),
        sorted(vec![CardColor::White, CardColor::Blue]),
        "flipped (face-down) sources are skipped (DCGO `if (cardSource1.IsFlipped) continue`)"
    );
}

#[test]
fn bt8_084_four_colors_gets_plus_4000_on_your_turn() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-BLUE", "SRC-RG", "BT8-084"]);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(kimera),
        Some(12000),
        "white + blue + red + green = 4 colors -> +4000"
    );
}

#[test]
fn bt8_084_three_colors_no_boost() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-WHITE", "SRC-RG", "BT8-084"]);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(kimera),
        Some(8000),
        "white (twice) + red + green = 3 distinct colors -> no boost"
    );
}

#[test]
fn bt8_084_four_color_stack_no_boost_on_opponent_turn() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-BLUE", "SRC-RG", "BT8-084"]);
    runner.set_first_player(1);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(kimera), Some(8000));
}

// ═══════════════════════════════════════════════════════════════════════════
// SECTION 4 — [When Digivolving] -1000 DP per color, up to 4 opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════
//
// Responses go through `Game::decode_action` (the real action boundary, which
// refreshes materialized declaratives before resolving the selection) so the
// [Your Turn] AddColor treatment is live when the amount is read — exactly as
// in a real game.

fn finish_multi_select(runner: &mut DebugRunner) {
    // "up to 4": after the first pick the prompt turns optional — PASS ends it
    // (DCGO canEndNotMax: true).
    // With a single candidate the pick completes on its own.
    if let Some(view) = runner.pending_selection_view() {
        assert!(view.is_optional, "may stop before 4 after one pick");
        runner.game.decode_action(PASS, 0);
    }
    assert!(runner.pending_selection().is_none(), "clause finished");
}

/// White top + blue source, then the placement adds a yellow card: 3 colors
/// read AFTER the placement (DCGO computes minusDP after
/// AddDigivolutionCardsBottom) -> -3000 on the single picked target.
#[test]
fn bt8_084_dp_minus_counts_colors_after_placement() {
    let mut runner = color_runner();
    push_to_trash(&mut runner, 0, "TRASH-YELLOW");
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let kimera = runner.place_stack(0, &["SRC-BLUE", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);

    let view = runner.pending_selection_view().expect("trash prompt");
    runner.game.decode_action(view.valid_action_ids[0], 0);

    let view = runner
        .pending_selection_view()
        .expect("up-to-4 opponent Digimon prompt");
    assert!(
        !view.is_optional,
        "DCGO canNoSelect: false — at least one target must be picked"
    );
    assert_eq!(
        view.valid_action_ids.iter().filter(|a| **a != PASS).count(),
        2,
        "both opponent Digimon are candidates"
    );
    runner.game.decode_action(view.valid_action_ids[0], 0);
    finish_multi_select(&mut runner);

    assert_eq!(
        runner.effective_dp(opp_a),
        Some(9000),
        "white + blue + yellow = 3 colors -> -3000"
    );
    assert_eq!(runner.effective_dp(opp_b), Some(12000), "unpicked target untouched");
}

/// Declining the placement still runs the DP-minus leg ("Then" is not
/// conditional on the placement): white + red/green = 3 colors.
#[test]
fn bt8_084_dp_minus_runs_after_declined_placement() {
    let mut runner = color_runner();
    push_to_trash(&mut runner, 0, "TRASH-YELLOW");
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let kimera = runner.place_stack(0, &["SRC-RG", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);

    runner.pending_selection_view().expect("trash prompt");
    runner.game.decode_action(PASS, 0);

    let view = runner
        .pending_selection_view()
        .expect("up-to-4 opponent Digimon prompt");
    runner.game.decode_action(view.valid_action_ids[0], 0);
    finish_multi_select(&mut runner);

    assert_eq!(runner.effective_dp(opp_a), Some(9000));
}

/// The -DP lasts into the opponent's next turn.
#[test]
fn bt8_084_dp_minus_persists_through_opponent_turn() {
    let mut runner = color_runner();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let kimera = runner.place_stack(0, &["SRC-RG", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("up-to-4 opponent Digimon prompt (no trash prompt: trash empty)");
    runner.game.decode_action(view.valid_action_ids[0], 0);
    finish_multi_select(&mut runner);
    assert_eq!(runner.effective_dp(opp_a), Some(9000));

    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(9000),
        "still applied during the opponent's next turn"
    );
}

/// No opponent Digimon -> the DP-minus leg installs no prompt.
#[test]
fn bt8_084_no_opponent_digimon_no_dp_prompt() {
    let mut runner = color_runner();
    let kimera = runner.place_stack(0, &["SRC-RG", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);
    assert!(
        runner.pending_selection().is_none(),
        "empty trash + empty opponent field -> nothing to choose"
    );
}


/// A single opponent Digimon is still a PICK (DCGO SelectPermanentEffect with
/// maxCount 1, canNoSelect: false) — never auto-selected.
#[test]
fn bt8_084_single_opponent_digimon_still_prompts() {
    let mut runner = color_runner();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let kimera = runner.place_stack(0, &["SRC-BLUE", "SRC-RG", "BT8-084"]);
    fire_when_digivolving(&mut runner, kimera);

    let view = runner
        .pending_selection_view()
        .expect("the -DP pick parks even with one candidate");
    runner.game.decode_action(view.valid_action_ids[0], 0);
    finish_multi_select(&mut runner);
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(8000),
        "white + blue + red + green = 4 colors -> -4000"
    );
}
