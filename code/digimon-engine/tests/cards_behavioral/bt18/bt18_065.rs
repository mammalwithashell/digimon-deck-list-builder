//! BT18-065 Snatchmon — Digimon, Lv.4, Black, DP 6000, Cost 6.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT18-065.md, matches
//! the card image)
//!
//! "While you have no Digimon other than [Vemmon], cards in your trash can
//! also be placed for this card's DigiXros."
//! "[When Digivolving] You may place up to 2 [Vemmon] from your trash as
//! this Digimon's bottom digivolution cards."
//! "[End of Your Turn] If this Digimon has 4 or more digivolution cards, it
//! may digivolve into a Digimon card with [Vemmon] in its text in the hand."
//!
//! Special Play Condition: "DigiXros -1: 4 [Vemmon]. When this would be
//! played, you may place specified cards from the hand/battle area under
//! it. Each one reduces the play cost."
//!
//! Inherited: "[All Turns] [Once Per Turn] When any [Vemmon] return to the
//! bottom of the deck from this Digimon's digivolution cards, this Digimon
//! unsuspends and gains <Blocker> until the end of your opponent's turn."
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Black/BT18_065.cs
//!
//! # Patterns this test covers
//! - C4  DigiXros source play (4x [Vemmon], hand/battle-area, cost_delta -1)
//! - Gap 4 conditional trash-origin extension (extra_material_zones)
//! - A6  Trash-to-digi-stack (place_as_bottom_source x up to 2)
//! - Gap-driver: self_source_count threshold gate ([End of Your Turn])
//! - effect_initiated_digivolve with cost: printed + in_text_contains filter
//! - Gap 1 on_source_returned_to_deck_bottom inherited observer (unsuspend + Blocker)

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT18-065")
}

// ─── Test-card helpers ────────────────────────────────────────────────────────

/// A [Vemmon] Digimon card (any zone). Level 4, no evo cost needed for
/// trash/DigiXros material use.
fn make_vemmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// A generic filler Digimon (not named Vemmon, no [Vemmon] in text).
fn make_filler_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// A hand-playable Digimon card with "[Vemmon]" in its printed effect text
/// (satisfies `in_text_contains: "Vemmon"`), with an `evo_costs` entry so
/// `effect_initiated_digivolve { cost: printed }` can actually pay it.
fn make_vemmon_text_result(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Result"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 8;
    c.effect_text = "[On Play] Search for [Vemmon].".to_string();
    c.evo_costs = vec![EvoCost {
        level: 4,
        memory_cost: 2,
        card_color: 5, // Black
    }];
    c
}

/// A hand-playable Digimon card with NO [Vemmon] anywhere in its text/name.
fn make_non_vemmon_text_result(id: &str) -> CardData {
    let mut c = make_test_card(id, &format!("{id} Plain"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 8;
    c.effect_text = "[On Play] Draw 1 card.".to_string();
    c.evo_costs = vec![EvoCost {
        level: 4,
        memory_cost: 2,
        card_color: 5, // Black
    }];
    c
}

fn push_to_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown {card_id}"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_idx, player as u8, card_index));
}

/// Build a runner with BT18-065 on the field (as the digivolve base target
/// for the [When Digivolving] clause tests) at a given stack size, plus
/// filler sources beneath it so `self_source_count` is deterministic.
fn snatchmon_on_field(extra_sources: usize) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 YAML parses and compiles")
        .add_card(make_filler_digimon("FILLER-BASE", "Filler Base"));

    for i in 0..extra_sources {
        builder = builder.add_card(make_filler_digimon(
            &format!("FILLER-SRC-{i}"),
            "Filler Source",
        ));
    }

    let mut runner = builder.memory(10).start();

    let mut stack: Vec<String> = Vec::new();
    for i in 0..extra_sources {
        stack.push(format!("FILLER-SRC-{i}"));
    }
    stack.push("BT18-065".to_string());
    let stack_refs: Vec<&str> = stack.iter().map(|s| s.as_str()).collect();
    let handle = runner.place_stack(0, &stack_refs);
    (runner, handle)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_065_yaml_compiles() {
    let _ = compiled();
}

#[test]
fn bt18_065_is_black_level4_cost6() {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};
    let card = compiled();
    assert_eq!(card.kind, CompiledCardKind::Digimon, "kind must be Digimon");
    assert_eq!(card.level, Some(4), "level must be 4");
    assert_eq!(card.cost, Some(6), "play cost must be 6");
    assert_eq!(card.dp, Some(6000), "DP must be 6000");
    assert!(
        card.color.contains(&CompiledColor::Black),
        "BT18-065 must be Black"
    );
}

#[test]
fn bt18_065_has_digixros_alt_path_with_4_vemmon_materials() {
    use digimon_dsl::compiled::CompiledAltPathKind;
    let card = compiled();
    let digixros = card
        .alt_paths
        .iter()
        .find(|ap| ap.kind == CompiledAltPathKind::DigiXros)
        .expect("must have a digixros alt-path");
    assert_eq!(
        digixros.materials.len(),
        1,
        "one material slot (repeated 4x via repeat: {{min:4,max:4}})"
    );
    assert!(
        !digixros.extra_material_zones.is_empty(),
        "must carry the Gap-4 extra_material_zones (conditional trash origin)"
    );
}

#[test]
fn bt18_065_has_when_digivolving_clause() {
    let card = compiled();
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
            Some(t)
        }
        _ => None,
    });
    assert!(clause.is_some(), "must have a WhenDigivolving clause");
    assert_eq!(clause.unwrap().scope, CompiledScope::FaceUp);
}

#[test]
fn bt18_065_has_end_of_your_turn_clause() {
    let card = compiled();
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
            Some(t)
        }
        _ => None,
    });
    assert!(clause.is_some(), "must have an EndOfYourTurn clause");
}

#[test]
fn bt18_065_has_inherited_on_source_returned_clause() {
    let card = compiled();
    let clause = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.when
                .contains(&CompiledTiming::OnDigivolutionCardReturnedToDeckBottom) =>
        {
            Some(t)
        }
        _ => None,
    });
    let clause = clause.expect("must have an OnDigivolutionCardReturnedToDeckBottom clause");
    assert_eq!(clause.scope, CompiledScope::Inherited, "must be inherited");
    assert!(clause.once_per_turn, "must be Once Per Turn");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — DigiXros: 4 [Vemmon] material requirement
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_065_digixros_completes_with_4_vemmon_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("VEM-1"))
        .add_card(make_vemmon("VEM-2"))
        .add_card(make_vemmon("VEM-3"))
        .add_card(make_vemmon("VEM-4"))
        .hand(0, &["BT18-065", "VEM-1", "VEM-2", "VEM-3", "VEM-4"])
        .memory(20)
        .start();

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");

    for _ in 0..4 {
        let action = runner
            .pending_selection()
            .expect("material prompt still open")
            .valid_action_ids[0];
        runner
            .execute_action(0, action)
            .expect("select a Vemmon material");
    }
    runner
        .execute_action(0, PASS)
        .expect("finish DigiXros material selection");

    assert!(
        runner.pending_selection().is_none(),
        "DigiXros transaction must be fully resolved"
    );
    let snatchmon_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-065");
    assert!(snatchmon_on_field, "Snatchmon must land on the field");
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "all 4 Vemmon + Snatchmon must have left the hand"
    );
}

#[test]
fn bt18_065_digixros_reduces_cost_by_1_per_material() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("VEM-1B"))
        .add_card(make_vemmon("VEM-2B"))
        .add_card(make_vemmon("VEM-3B"))
        .add_card(make_vemmon("VEM-4B"))
        .hand(0, &["BT18-065", "VEM-1B", "VEM-2B", "VEM-3B", "VEM-4B"])
        .memory(20)
        .start();

    let memory_before = runner.memory();
    assert_eq!(runner.play(0, 0), None);
    for _ in 0..4 {
        let action = runner.pending_selection().unwrap().valid_action_ids[0];
        runner.execute_action(0, action).unwrap();
    }
    runner.execute_action(0, PASS).unwrap();

    assert_eq!(
        runner.memory(),
        memory_before - (6 - 4),
        "base cost 6 minus 4 materials x -1 each = net cost 2"
    );
}

#[test]
fn bt18_065_digixros_trash_material_hidden_when_non_vemmon_digimon_present() {
    // Conditional trash-origin extension: a non-[Vemmon] own Digimon on the
    // field breaks "no Digimon other than [Vemmon]" — the trash zone must
    // NOT be exposed as a legal DigiXros material source.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("VEM-1C"))
        .add_card(make_vemmon("VEM-2C"))
        .add_card(make_vemmon("VEM-3C"))
        .add_card(make_filler_digimon("AGUMON-C", "Agumon"))
        .hand(0, &["BT18-065", "VEM-1C", "VEM-2C", "VEM-3C"])
        .memory(20)
        .start();
    runner.place_on_field(0, "AGUMON-C", None);
    push_to_trash(&mut runner, 0, "VEM-1C");

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");
    let prompt = runner.pending_selection().expect("material prompt");
    assert!(
        !prompt.valid_action_ids.contains(&TRASH_EFFECT_START),
        "a non-[Vemmon] own Digimon on the field must hide the trash material zone"
    );
}

#[test]
fn bt18_065_digixros_trash_material_exposed_when_only_vemmon_on_field() {
    // Condition MET: no own Digimon other than [Vemmon] on the field — the
    // trash zone must be exposed as a legal DigiXros material source.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("VEM-1D"))
        .add_card(make_vemmon("VEM-FIELD-D"))
        .hand(0, &["BT18-065", "VEM-1D"])
        .memory(20)
        .start();
    runner.place_on_field(0, "VEM-FIELD-D", None);
    push_to_trash(&mut runner, 0, "VEM-1D");

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");
    let prompt = runner.pending_selection().expect("material prompt");
    assert!(
        prompt.valid_action_ids.contains(&TRASH_EFFECT_START),
        "with only [Vemmon] on the field, the trash material zone must be exposed"
    );
}

#[test]
fn bt18_065_digixros_material_from_battle_area_offered() {
    // Materials may also come from the battle area (not only hand).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("VEM-FIELD-E"))
        .add_card(make_vemmon("VEM-1E"))
        .add_card(make_vemmon("VEM-2E"))
        .add_card(make_vemmon("VEM-3E"))
        .hand(0, &["BT18-065", "VEM-1E", "VEM-2E", "VEM-3E"])
        .memory(20)
        .start();
    runner.place_on_field(0, "VEM-FIELD-E", None);

    assert_eq!(runner.play(0, 0), None, "DigiXros play installs a material prompt");
    let prompt = runner.pending_selection().expect("material prompt");
    // Battle-area DigiXros material candidates are encoded as the raw
    // battle-area index (see `Game::pending_digixros_material_candidates`,
    // code/digimon-engine/src/game_actions/mod.rs) — VEM-FIELD-E sits at
    // battle-area index 0, so action id 0 is the field-material pick.
    assert!(
        prompt.valid_action_ids.contains(&0),
        "a battle-area [Vemmon] at index 0 must be offered as a legal DigiXros material \
         (valid_action_ids = {:?})",
        prompt.valid_action_ids
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving] up to 2 [Vemmon] from trash as bottom sources
// ═══════════════════════════════════════════════════════════════════════════════

/// Directly fire WhenDigivolving on an existing Snatchmon permanent (skips
/// the play/digivolve action resolution — the observer-firing pattern from
/// ex8_067.rs's `fire_digivolve_onto`, adapted to fire directly on the
/// carrier rather than an observing Tamer).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(
            digimon_engine::enums::EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(handle),
        );
    runner.game.drain_effect_queue();
}

#[test]
fn bt18_065_when_digivolving_installs_optional_multiselect_with_vemmon_in_trash() {
    let (mut runner, handle) = snatchmon_on_field(0);
    push_to_trash(&mut runner, 0, "FILLER-BASE"); // ensure trash zone non-empty baseline
    let vemmon = make_vemmon("WD-VEM-1");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(vemmon);
        let idx = g.next_card_index();
        g.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, idx));
    }

    fire_when_digivolving(&mut runner, handle);

    assert!(
        runner.pending_selection().is_some(),
        "a [Vemmon] in trash must install the up-to-2 multiselect prompt"
    );
    assert!(
        runner.pending_is_optional(),
        "the multiselect must be declinable (0 is a legal pick count)"
    );
}

#[test]
fn bt18_065_when_digivolving_no_prompt_when_no_vemmon_in_trash() {
    let (mut runner, handle) = snatchmon_on_field(0);
    // Trash has no [Vemmon] at all.
    fire_when_digivolving(&mut runner, handle);

    // select_count_capped_multi with zero candidates and optional_zero: true
    // must not install a selection (nothing to pick).
    assert!(
        runner.pending_selection().is_none(),
        "no prompt should install when trash has no [Vemmon]"
    );
}

#[test]
fn bt18_065_when_digivolving_placing_vemmon_increases_source_count() {
    let (mut runner, handle) = snatchmon_on_field(0);
    let vemmon = make_vemmon("WD-VEM-2");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(vemmon);
        let idx = g.next_card_index();
        g.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, idx));
    }

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    fire_when_digivolving(&mut runner, handle);
    assert!(runner.pending_selection().is_some(), "prompt must install");
    let action = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, action).expect("select 1st Vemmon");
    runner.auto_resolve().expect("finish multiselect");

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        sources_after > sources_before,
        "placing a Vemmon from trash must increase this Digimon's source count \
         (before={sources_before}, after={sources_after})"
    );
}

#[test]
fn bt18_065_when_digivolving_declining_places_nothing() {
    let (mut runner, handle) = snatchmon_on_field(0);
    let vemmon = make_vemmon("WD-VEM-3");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(vemmon);
        let idx = g.next_card_index();
        g.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, idx));
    }

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    fire_when_digivolving(&mut runner, handle);
    assert!(runner.pending_selection().is_some(), "prompt must install");
    runner.execute_action(0, PASS).expect("decline (0 picks)");

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert_eq!(
        sources_after, sources_before,
        "declining the multiselect must not place any bottom sources"
    );
}

#[test]
fn bt18_065_when_digivolving_places_up_to_two_not_more() {
    let (mut runner, handle) = snatchmon_on_field(0);
    for i in 0..3 {
        let vemmon = make_vemmon(&format!("WD-VEM-MANY-{i}"));
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(vemmon);
        let idx = g.next_card_index();
        g.players[0].trash.push(CardSource::new(data_idx, 0, idx));
    }

    let sources_before = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    fire_when_digivolving(&mut runner, handle);
    assert!(runner.pending_selection().is_some(), "prompt must install");
    runner.auto_resolve().expect("resolve multiselect");

    let sources_after = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    assert!(
        sources_after - sources_before <= 2,
        "at most 2 [Vemmon] may be placed even with 3 available \
         (before={sources_before}, after={sources_after})"
    );
    assert!(
        sources_after > sources_before,
        "at least 1 must have been placed (auto_resolve picks greedily)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [End of Your Turn] digivolve into a [Vemmon]-text hand card
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_065_end_of_turn_condition_blocks_with_fewer_than_4_sources() {
    let (mut runner, _handle) = snatchmon_on_field(2); // 2 filler sources -> 2 total, < 4
    let result = make_vemmon_text_result("EOT-RESULT-1");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(result);
        let idx = g.next_card_index();
        g.players[0].hand.push(CardSource::new(data_idx, 0, idx));
    }

    runner.end_turn();

    assert!(
        runner.pending_selection().is_none(),
        "with fewer than 4 digivolution cards, the End of Turn clause must not fire"
    );
}

#[test]
fn bt18_065_end_of_turn_condition_passes_with_4_or_more_sources() {
    let (mut runner, _handle) = snatchmon_on_field(4); // 4 filler sources -> exactly 4
    let result = make_vemmon_text_result("EOT-RESULT-2");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(result);
        let idx = g.next_card_index();
        g.players[0].hand.push(CardSource::new(data_idx, 0, idx));
    }

    runner.end_turn();

    assert!(
        runner.pending_selection().is_some(),
        "with 4+ digivolution cards, the End of Turn clause must offer the hand pick"
    );
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(
        runner.pending_is_optional(),
        "the hand-card pick must be declinable ('may digivolve')"
    );
}

#[test]
fn bt18_065_end_of_turn_filters_to_vemmon_text_only() {
    let (mut runner, _handle) = snatchmon_on_field(4);
    let plain = make_non_vemmon_text_result("EOT-PLAIN-1");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(plain);
        let idx = g.next_card_index();
        g.players[0].hand.push(CardSource::new(data_idx, 0, idx));
    }

    runner.end_turn();

    assert!(
        runner.pending_selection().is_none(),
        "a hand card with no [Vemmon] in its text must not be offered, \
         so the clause naturally fizzles with zero candidates"
    );
}

#[test]
fn bt18_065_end_of_turn_digivolves_into_picked_vemmon_text_card() {
    let (mut runner, handle) = snatchmon_on_field(4);
    let result = make_vemmon_text_result("EOT-RESULT-3");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(result);
        let idx = g.next_card_index();
        g.players[0].hand.push(CardSource::new(data_idx, 0, idx));
    }

    runner.end_turn();
    assert!(runner.pending_selection().is_some(), "hand pick must install");
    let action = runner.pending_selection().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, action)
        .expect("pick the Vemmon-text hand card");
    runner.auto_resolve().expect("finish digivolve resolution");

    let top_is_result = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.top_card().card_id(&runner.game.card_data) == "EOT-RESULT-3")
        .unwrap_or(false);
    assert!(
        top_is_result,
        "Snatchmon must have digivolved into the picked [Vemmon]-text card"
    );
}

#[test]
fn bt18_065_end_of_turn_declining_leaves_snatchmon_unchanged() {
    let (mut runner, handle) = snatchmon_on_field(4);
    let result = make_vemmon_text_result("EOT-RESULT-4");
    {
        let g = runner.game_mut();
        let data_idx = g.card_data.len();
        g.card_data.push(result);
        let idx = g.next_card_index();
        g.players[0].hand.push(CardSource::new(data_idx, 0, idx));
    }

    runner.end_turn();
    assert!(runner.pending_selection().is_some(), "hand pick must install");
    runner.execute_action(0, PASS).expect("decline the digivolve");

    let top_is_snatchmon = runner
        .game
        .player(0)
        .battle_area
        .get(handle.index as usize)
        .map(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-065")
        .unwrap_or(false);
    assert!(
        top_is_snatchmon,
        "declining must leave Snatchmon as the top card (no digivolve)"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "the hand card must remain in hand when declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [All Turns][OPT] Vemmon returned to deck bottom
// ═══════════════════════════════════════════════════════════════════════════════

/// Seed a black host permanent whose bottom source is BT18-065 (inherited),
/// then the named `under` card stacked ABOVE it, then a `top` card as the
/// visible top. Mirrors the proof pattern from
/// `source_returned_to_deck_bottom_observer.rs`.
fn seed_stack(
    r: &mut DebugRunner,
    under_id: &str,
    top_id: &str,
) -> (PermanentHandle, digimon_engine::card_source::CardHandle) {
    let mut sources = Vec::new();
    for card_id in ["BT18-065", under_id, top_id] {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("card {card_id} in registry"));
        let card_idx = g.next_card_index();
        sources.push(CardSource::new(data_idx, 0, card_idx));
    }
    let under_handle = sources[1].handle();
    let g = r.game_mut();
    let turn = g.turn_count;
    let bottom = sources.remove(0);
    let mut perm = Permanent::new(bottom, turn);
    perm.card_sources.extend(sources);
    g.players[0].battle_area.push(perm);
    (
        PermanentHandle {
            player: 0,
            index: (r.game.player(0).battle_area.len() - 1) as u8,
        },
        under_handle,
    )
}

fn inherited_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("INH-VEM-SRC"))
        .add_card(make_filler_digimon("INH-AGUMON-SRC", "Agumon"))
        .add_card(make_filler_digimon("INH-HOST-TOP", "Host Top"))
        .memory(10)
        .start()
}

#[test]
fn bt18_065_inherited_unsuspends_and_grants_blocker_on_vemmon_return() {
    let mut r = inherited_runner();
    let (host, vemmon) = seed_stack(&mut r, "INH-VEM-SRC", "INH-HOST-TOP");

    // Pre-suspend the host so the unsuspend is observable.
    {
        let g = r.game_mut();
        g.players[0]
            .battle_area
            .get_mut(host.index as usize)
            .unwrap()
            .is_suspended = true;
    }

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(
            ctx.return_card_source_to_deck(host, vemmon, /* to_bottom */ true),
            "the Vemmon source must be found and returned"
        );
    }

    let is_suspended = r
        .game
        .player(0)
        .battle_area
        .get(host.index as usize)
        .unwrap()
        .is_suspended;
    assert!(!is_suspended, "the host must unsuspend");
    assert!(
        r.game.has_keyword(host, Keyword::Blocker),
        "the host must gain <Blocker>"
    );
}

#[test]
fn bt18_065_inherited_does_not_fire_for_non_vemmon_return() {
    let mut r = inherited_runner();
    let (host, agumon) = seed_stack(&mut r, "INH-AGUMON-SRC", "INH-HOST-TOP");
    {
        let g = r.game_mut();
        g.players[0]
            .battle_area
            .get_mut(host.index as usize)
            .unwrap()
            .is_suspended = true;
    }

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, agumon, true));
    }

    let is_suspended = r
        .game
        .player(0)
        .battle_area
        .get(host.index as usize)
        .unwrap()
        .is_suspended;
    assert!(
        is_suspended,
        "a non-[Vemmon] return must NOT unsuspend the host"
    );
    assert!(
        !r.game.has_keyword(host, Keyword::Blocker),
        "a non-[Vemmon] return must NOT grant <Blocker>"
    );
}

#[test]
fn bt18_065_inherited_does_not_fire_on_deck_top_return() {
    let mut r = inherited_runner();
    let (host, vemmon) = seed_stack(&mut r, "INH-VEM-SRC", "INH-HOST-TOP");
    {
        let g = r.game_mut();
        g.players[0]
            .battle_area
            .get_mut(host.index as usize)
            .unwrap()
            .is_suspended = true;
    }

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon, /* to_bottom */ false));
    }

    let is_suspended = r
        .game
        .player(0)
        .battle_area
        .get(host.index as usize)
        .unwrap()
        .is_suspended;
    assert!(
        is_suspended,
        "a deck-TOP return must NOT unsuspend the host (bottom-scoped observer)"
    );
}

#[test]
fn bt18_065_inherited_is_once_per_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-065")
        .expect("BT18-065 in embedded pack")
        .add_card(make_vemmon("OPT-VEM-A"))
        .add_card(make_vemmon("OPT-VEM-B"))
        .add_card(make_filler_digimon("OPT-HOST-TOP", "Host Top"))
        .memory(10)
        .start();

    let mut sources = Vec::new();
    for id in ["BT18-065", "OPT-VEM-A", "OPT-VEM-B", "OPT-HOST-TOP"] {
        let g = r.game_mut();
        let data_idx = g.card_data.iter().position(|c| c.card_id == id).unwrap();
        let card_idx = g.next_card_index();
        sources.push(CardSource::new(data_idx, 0, card_idx));
    }
    let vem_a = sources[1].handle();
    let vem_b = sources[2].handle();
    {
        let g = r.game_mut();
        let turn = g.turn_count;
        let bottom = sources.remove(0);
        let mut perm = Permanent::new(bottom, turn);
        perm.card_sources.extend(sources);
        perm.is_suspended = true;
        g.players[0].battle_area.push(perm);
    }
    let host = PermanentHandle {
        player: 0,
        index: (r.game.player(0).battle_area.len() - 1) as u8,
    };

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vem_a, true));
    }
    let suspended_after_first = r
        .game
        .player(0)
        .battle_area
        .get(host.index as usize)
        .unwrap()
        .is_suspended;
    assert!(!suspended_after_first, "first return must unsuspend");

    // Re-suspend to make a SECOND unsuspend observable if it fired again.
    {
        let g = r.game_mut();
        g.players[0]
            .battle_area
            .get_mut(host.index as usize)
            .unwrap()
            .is_suspended = true;
    }

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vem_b, true));
    }
    let suspended_after_second = r
        .game
        .player(0)
        .battle_area
        .get(host.index as usize)
        .unwrap()
        .is_suspended;
    assert!(
        suspended_after_second,
        "the once-per-turn observer must NOT fire a second time this turn \
         (host remains suspended since we manually re-suspended it)"
    );
}
