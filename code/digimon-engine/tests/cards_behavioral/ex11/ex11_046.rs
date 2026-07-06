//! EX11-046 Galacticmon — Digimon, Lv.6, Black, DP 14000, Cost 14.
//! Traits: Unknown / LIBERATOR. Form: Mega. Attribute: Unknown.
//!
//! # Card text (card image EX11-046.webp + data/card_bundles/EX11-046.md —
//! official Bandai DB, authoritative; matches data/cards.json verbatim)
//!
//! [On Play] [When Digivolving] Choose 1 of your opponent's highest play
//! cost Digimon and delete all of their other Digimon. Then, if this
//! Digimon has 4 or more [Vemmon] in its digivolution cards, until your
//! opponent's turn ends, it gains <Blocker> and isn't affected by their
//! effects.
//! [End of Opponent's Turn] This Digimon may digivolve into [Galacticmon]
//! in the hand or trash, ignoring digivolution requirements and without
//! paying the cost.
//!
//! Assembly -6: 8 cards w/[Vemmon] in text.
//!
//! NOTE: the task brief said "Lv7"; the printed card (image + official
//! Bandai DB bundle + cards.json) is unambiguously Lv.6. The printed face
//! is used here per CLAUDE.md source-priority rules.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Black/EX11_046.cs
//!
//! # Patterns this test covers
//! - Structural: standard + 2 named alt-digivolve paths (Snatchmon / self)
//!   + Assembly alt-path; 2 triggered clauses ([OP][WD] shared, [EOT]).
//! - Section 2: [OP]/[WD] highest-play-cost keep-1-delete-rest, including
//!   the tie-break (>1 legal keep candidate) and the single-opponent-Digimon
//!   degenerate case (nothing left to delete).
//! - Section 3: conditional Blocker + 4-source-kind opponent-effect immunity
//!   gated on self_source_count >= 4 [Vemmon], positive and negative.
//! - Section 4: [End of Opponent's Turn] free digivolve into Galacticmon
//!   from hand or trash, ignoring requirements, with an event-log assertion
//!   that no memory/cost was paid; OPT-shape/gating negative cases.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 YAML parses and compiles")
        .build()
}

fn make_opp(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.play_cost = play_cost;
    c.level = Some(4);
    c
}

/// A stand-in digivolution-source card named exactly "Vemmon" (matches the
/// printed "4 or more [Vemmon] in its digivolution cards" name-count gate).
fn make_vemmon_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.level = Some(3);
    c
}

fn make_other_source(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Other"));
    c.level = Some(3);
    c
}

/// A stand-in [Galacticmon] card for the free-redigivolve clause.
fn make_galacticmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Galacticmon");
    c.level = Some(6);
    c.dp = Some(14000);
    c.play_cost = 14;
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

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn fire_end_of_opponents_turn(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex11_046_metadata_matches_printed_card() {
    let runner = runner();
    let card = runner.compiled_card("EX11-046").expect("compiled card");

    assert_eq!(card.card, "EX11-046");
    assert_eq!(card.name, "Galacticmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(14));
    assert_eq!(card.dp, Some(14000));
    assert!(card.traits.contains(&"Unknown".to_string()));
    assert!(card.traits.contains(&"LIBERATOR".to_string()));
}

#[test]
fn ex11_046_has_standard_and_named_alt_digivolve_paths() {
    let runner = runner();
    let card = runner.compiled_card("EX11-046").expect("compiled card");

    // Standard box: Black Lv.5 / cost 6.
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(6))
                && p.from
                    .as_ref()
                    .is_some_and(|f| f.level_eq == Some(5) && f.color_is.is_some())
        }),
        "standard Black Lv.5 / cost 6 digivolve path must compile"
    );

    // [Digivolve] [Snatchmon]: Cost 9.
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(9))
                && p.from
                    .as_ref()
                    .and_then(|f| f.name_is.as_deref())
                    .map(|n| n == "Snatchmon")
                    .unwrap_or(false)
        }),
        "[Digivolve] [Snatchmon]: Cost 9 alt-path must compile"
    );

    // [Digivolve] [Galacticmon]: Cost 5.
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(5))
                && p.from
                    .as_ref()
                    .and_then(|f| f.name_is.as_deref())
                    .map(|n| n == "Galacticmon")
                    .unwrap_or(false)
        }),
        "[Digivolve] [Galacticmon]: Cost 5 alt-path must compile"
    );
}

#[test]
fn ex11_046_has_assembly_alt_path() {
    let runner = runner();
    let card = runner.compiled_card("EX11-046").expect("compiled card");

    assert!(
        card.alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Assembly),
        "Assembly -6 (8 cards w/[Vemmon] in text) alt-path must compile"
    );
}

#[test]
fn ex11_046_has_shared_op_wd_clause_and_eot_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX11-046").expect("compiled card");

    let triggered: Vec<&_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let shared = triggered
        .iter()
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("a shared [On Play]/[When Digivolving] clause must exist");
    assert_eq!(shared.scope, CompiledScope::FaceUp);
    assert!(
        !shared.optional,
        "the choose-1-keep-delete-rest clause is mandatory (DCGO isOptional: false)"
    );

    let eot = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::EndOfOpponentsTurn])
        .expect("an [End of Opponent's Turn] clause must exist");
    assert_eq!(eot.scope, CompiledScope::FaceUp);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [OP]/[WD]: choose highest play-cost opp Digimon, delete rest
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: two opp Digimon at different costs — only the higher-cost one
/// is offered as the keep target; the other is deleted.
#[test]
fn ex11_046_on_play_keeps_highest_cost_deletes_the_rest() {
    let cheap = make_opp("EX11-046-CHEAP", 6);
    let expensive = make_opp("EX11-046-EXP", 10);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(cheap)
        .add_card(expensive)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.place_on_field(1, "EX11-046-CHEAP", None);
    runner.place_on_field(1, "EX11-046-EXP", None);

    fire_on_play(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("keep-target selection installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the highest-play-cost opponent Digimon may be kept"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("keep the highest-cost Digimon");
    runner.auto_resolve().expect("finish delete-the-rest");

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "only the cost-10 (kept) Digimon survives; cost-6 is deleted"
    );
    let survivor = runner.game.players[1].battle_area[0]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(survivor, "EX11-046-EXP", "the surviving Digimon must be the kept highest-cost one");
}

/// Tie-break: two opp Digimon share the (single) highest cost — both are
/// legal keep candidates; whichever is picked, the OTHER opp Digimon (the
/// cheaper third one) is deleted, and the picked one survives alongside its
/// tied sibling per the printed "delete all of their OTHER Digimon" text
/// (only the non-selected, non-tied Digimon are deleted).
#[test]
fn ex11_046_on_play_tie_break_offers_both_max_cost_candidates() {
    let tied_a = make_opp("EX11-046-TIE-A", 10);
    let tied_b = make_opp("EX11-046-TIE-B", 10);
    let cheap = make_opp("EX11-046-TIE-CHEAP", 4);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(tied_a)
        .add_card(tied_b)
        .add_card(cheap)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.place_on_field(1, "EX11-046-TIE-A", None);
    runner.place_on_field(1, "EX11-046-TIE-B", None);
    runner.place_on_field(1, "EX11-046-TIE-CHEAP", None);

    fire_on_play(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("keep-target selection installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both tied max-cost Digimon must be legal keep candidates"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("keep one of the tied max-cost Digimon");
    runner.auto_resolve().expect("finish delete-the-rest");

    // Exactly 1 of the tied pair remains (the kept one), plus the cheap one
    // is gone; only the non-kept opponent Digimon are deleted.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "the kept Digimon survives; both the tied sibling and the cheap Digimon are deleted"
    );
}

/// Degenerate: opponent has exactly 1 Digimon — it is auto-offered as the
/// only keep candidate (a 1-option select, no auto-resolve per rule 17), and
/// nothing is deleted since there are no OTHER opponent Digimon.
#[test]
fn ex11_046_on_play_single_opp_digimon_has_nothing_else_to_delete() {
    let only = make_opp("EX11-046-ONLY", 8);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(only)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.place_on_field(1, "EX11-046-ONLY", None);

    fire_on_play(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("keep-target selection installs even with 1 candidate");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.valid_action_ids.len(), 1, "the lone Digimon is the only legal keep candidate");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("keep the only Digimon");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "the lone opponent Digimon survives — there was nothing else to delete"
    );
}

/// Negative: opponent has zero Digimon — no selection installs and no
/// action taken.
#[test]
fn ex11_046_on_play_no_opp_digimon_does_not_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    fire_on_play(&mut runner, galacticmon);

    assert!(
        runner.pending_selection_view().is_none(),
        "with no opponent Digimon, no keep-target selection should install"
    );
    assert_eq!(runner.game.players[1].battle_area.len(), 0);
}

/// [When Digivolving] fires the same shared clause.
#[test]
fn ex11_046_when_digivolving_also_fires_keep_delete_clause() {
    let cheap = make_opp("EX11-046-WD-CHEAP", 3);
    let expensive = make_opp("EX11-046-WD-EXP", 9);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(cheap)
        .add_card(expensive)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.place_on_field(1, "EX11-046-WD-CHEAP", None);
    runner.place_on_field(1, "EX11-046-WD-EXP", None);

    fire_when_digivolving(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving installs the same keep-target selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.valid_action_ids.len(), 1);
    runner.auto_resolve().expect("keep + delete the rest");

    assert_eq!(runner.game.players[1].battle_area.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Conditional Blocker + opponent-effect immunity (4+ Vemmon)
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: 4 Vemmon-named digivolution sources -> Blocker + the
/// 4-source-kind opponent-effect immunity bundle, both scoped to the
/// opponent only (own effects still apply) until end of opponent's turn.
#[test]
fn ex11_046_four_vemmon_sources_grants_blocker_and_opponent_immunity() {
    let v1 = make_vemmon_source("EX11-046-V1");
    let v2 = make_vemmon_source("EX11-046-V2");
    let v3 = make_vemmon_source("EX11-046-V3");
    let v4 = make_vemmon_source("EX11-046-V4");
    let opp = make_opp("EX11-046-4V-OPP", 5);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(v1)
        .add_card(v2)
        .add_card(v3)
        .add_card(v4)
        .add_card(opp)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.push_source(galacticmon, "EX11-046-V1");
    runner.push_source(galacticmon, "EX11-046-V2");
    runner.push_source(galacticmon, "EX11-046-V3");
    runner.push_source(galacticmon, "EX11-046-V4");
    runner.place_on_field(1, "EX11-046-4V-OPP", None);

    fire_on_play(&mut runner, galacticmon);
    runner.auto_resolve().ok();
    // Drain any further prompts (keep-target selection resolves first).
    let mut iterations = 0;
    while runner.pending_selection_view().is_some() && iterations < 10 {
        runner.auto_resolve().ok();
        iterations += 1;
    }

    assert!(
        runner.game.has_keyword(galacticmon, Keyword::Blocker),
        "4+ Vemmon sources must grant Blocker"
    );
    for source_kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
        EffectSourceKind::Rule,
    ] {
        assert!(
            runner
                .game
                .permanent_is_unaffected_by_effect(galacticmon, 1, source_kind),
            "opponent {source_kind:?} effects must not affect Galacticmon"
        );
        assert!(
            !runner
                .game
                .permanent_is_unaffected_by_effect(galacticmon, 0, source_kind),
            "own {source_kind:?} effects must still affect Galacticmon"
        );
    }
}

/// Negative: fewer than 4 Vemmon-named sources (3) -> no Blocker, no
/// immunity grant.
#[test]
fn ex11_046_fewer_than_four_vemmon_sources_grants_nothing() {
    let v1 = make_vemmon_source("EX11-046-3V-V1");
    let v2 = make_vemmon_source("EX11-046-3V-V2");
    let v3 = make_vemmon_source("EX11-046-3V-V3");
    let opp = make_opp("EX11-046-3V-OPP", 5);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(v1)
        .add_card(v2)
        .add_card(v3)
        .add_card(opp)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.push_source(galacticmon, "EX11-046-3V-V1");
    runner.push_source(galacticmon, "EX11-046-3V-V2");
    runner.push_source(galacticmon, "EX11-046-3V-V3");
    runner.place_on_field(1, "EX11-046-3V-OPP", None);

    fire_on_play(&mut runner, galacticmon);
    let mut iterations = 0;
    while runner.pending_selection_view().is_some() && iterations < 10 {
        runner.auto_resolve().ok();
        iterations += 1;
    }

    assert!(
        !runner.game.has_keyword(galacticmon, Keyword::Blocker),
        "3 Vemmon sources (< 4) must NOT grant Blocker"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(galacticmon, 1, EffectSourceKind::Digimon),
        "3 Vemmon sources (< 4) must NOT grant opponent-effect immunity"
    );
}

/// Negative: 4 sources present but none named Vemmon -> no Blocker, no
/// immunity (the count must be name-filtered, not a raw source-count check).
#[test]
fn ex11_046_four_non_vemmon_sources_grants_nothing() {
    let o1 = make_other_source("EX11-046-NV-O1");
    let o2 = make_other_source("EX11-046-NV-O2");
    let o3 = make_other_source("EX11-046-NV-O3");
    let o4 = make_other_source("EX11-046-NV-O4");
    let opp = make_opp("EX11-046-NV-OPP", 5);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(o1)
        .add_card(o2)
        .add_card(o3)
        .add_card(o4)
        .add_card(opp)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    runner.push_source(galacticmon, "EX11-046-NV-O1");
    runner.push_source(galacticmon, "EX11-046-NV-O2");
    runner.push_source(galacticmon, "EX11-046-NV-O3");
    runner.push_source(galacticmon, "EX11-046-NV-O4");
    runner.place_on_field(1, "EX11-046-NV-OPP", None);

    fire_on_play(&mut runner, galacticmon);
    let mut iterations = 0;
    while runner.pending_selection_view().is_some() && iterations < 10 {
        runner.auto_resolve().ok();
        iterations += 1;
    }

    assert!(
        !runner.game.has_keyword(galacticmon, Keyword::Blocker),
        "4 non-Vemmon sources must NOT grant Blocker (name-filtered count)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [End of Opponent's Turn] free digivolve into Galacticmon
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: a [Galacticmon] card in hand -> the union-zone prompt installs
/// and offers it; accepting digivolves without paying memory (event-log /
/// memory-gauge assertion that no cost was paid).
#[test]
fn ex11_046_eot_may_digivolve_from_hand_for_free() {
    let next_form = make_galacticmon("EX11-046-EOT-HAND");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(next_form)
        .hand(0, &["EX11-046-EOT-HAND"])
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    let memory_before = runner.memory();

    fire_end_of_opponents_turn(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("union-zone digivolve-target selection installs");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the free-digivolve prompt must be a union-zone (hand+trash) selection, got: {:?}",
        view.kind
    );
    let concrete: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(
        concrete.len(),
        1,
        "exactly the 1 [Galacticmon] hand card must be offered"
    );
    runner
        .execute_action(0, concrete[0])
        .expect("digivolve into Galacticmon from hand");

    let top_card_name = runner.game.players[0].battle_area[galacticmon.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        top_card_name, "EX11-046-EOT-HAND",
        "Galacticmon must digivolve into the hand copy"
    );
    assert_eq!(runner.hand_size(0), 0, "the hand copy left the hand");
    assert_eq!(
        runner.memory(),
        memory_before,
        "the free digivolve must not change the memory gauge (no cost paid)"
    );
}

/// Positive: a [Galacticmon] card in trash (no hand copy) -> the union-zone
/// prompt offers the trash copy; accepting digivolves from trash.
#[test]
fn ex11_046_eot_may_digivolve_from_trash_for_free() {
    let next_form = make_galacticmon("EX11-046-EOT-TRASH");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(next_form)
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    push_to_trash(&mut runner, 0, "EX11-046-EOT-TRASH");

    fire_end_of_opponents_turn(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("union-zone digivolve-target selection installs");
    let concrete: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != digimon_engine::action::space::PASS)
        .collect();
    assert_eq!(concrete.len(), 1, "exactly the 1 [Galacticmon] trash card must be offered");
    runner
        .execute_action(0, concrete[0])
        .expect("digivolve into Galacticmon from trash");

    let top_card_name = runner.game.players[0].battle_area[galacticmon.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(top_card_name, "EX11-046-EOT-TRASH");
    assert_eq!(runner.trash_size(0), 0, "the trash copy left the trash");
}

/// Decline: the "may" is honored — passing the union-zone prompt leaves
/// Galacticmon un-digivolved and the hand copy untouched.
#[test]
fn ex11_046_eot_declining_leaves_galacticmon_unchanged() {
    let next_form = make_galacticmon("EX11-046-EOT-DECLINE");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(next_form)
        .hand(0, &["EX11-046-EOT-DECLINE"])
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    fire_end_of_opponents_turn(&mut runner, galacticmon);

    let view = runner
        .pending_selection_view()
        .expect("union-zone digivolve-target selection installs");
    assert!(
        view.is_optional,
        "the free-digivolve prompt must be optional ('may') — PASS is legal via is_optional, \
         not baked into valid_action_ids (which lists only concrete candidates)"
    );
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the free digivolve");

    let top_card_name = runner.game.players[0].battle_area[galacticmon.index as usize]
        .top_card()
        .card_id(&runner.game.card_data);
    assert_eq!(
        top_card_name, "EX11-046",
        "declining leaves the original Galacticmon on top"
    );
    assert_eq!(runner.hand_size(0), 1, "the hand copy stays in hand when declined");
}

/// Negative: no [Galacticmon] in hand or trash -> no prompt installs.
#[test]
fn ex11_046_eot_no_galacticmon_anywhere_does_not_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    fire_end_of_opponents_turn(&mut runner, galacticmon);

    assert!(
        runner.pending_selection_view().is_none(),
        "with no [Galacticmon] card in hand or trash, no selection should install"
    );
}

/// Negative: EndOfOpponentsTurn clause must not offer a same-named-but-
/// different card (e.g. a plain filler card) as a digivolve target.
#[test]
fn ex11_046_eot_ignores_non_galacticmon_hand_cards() {
    let filler = make_test_card("EX11-046-FILLER", "Not Galacticmon");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-046")
        .expect("EX11-046 parses")
        .add_card(filler)
        .hand(0, &["EX11-046-FILLER"])
        .memory(10)
        .start();

    let galacticmon = runner.place_on_field(0, "EX11-046", None);
    fire_end_of_opponents_turn(&mut runner, galacticmon);

    assert!(
        runner.pending_selection_view().is_none(),
        "a non-[Galacticmon] hand card must not trigger the free-digivolve prompt"
    );
}
