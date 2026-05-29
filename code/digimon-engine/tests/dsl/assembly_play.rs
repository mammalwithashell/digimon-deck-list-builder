//! Assembly play execution (G-ASSEMBLY-PLAY-EXECUTION / change `fix-ad1-025-assembly-data`).
//!
//! Faithful to DCGO `SelectAssemblyClass.cs` + the user-provided AD1-025 screenshots:
//! play the card via the normal play action, then — only if the named materials are
//! present in the controller's TRASH — surface an Assembly flow that selects them
//! (exact count, per element), places them at the BOTTOM of the played card's
//! digivolution stack, and pays the reduced cost (base − reduction).
//!
//! These tests are RED until the executor lands.

#![allow(unused_imports, dead_code)]

use digimon_engine::action::{build_action_mask, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

/// A plain Lv6 Digimon material card (e.g. a stand-in WarGreymon / MetalGarurumon).
fn material(card_id: &str, name: &str) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c
}

/// Inline YAML for a Lv7 card playable via Assembly: place 1 "WarGreymon" + 1
/// "MetalGarurumon" from trash under it, reducing the play cost by 6.
const ASSEMBLY_CARD_YAML: &str = r#"
card: T-ASM
name: Assembly Test
kind: digimon
level: 7
color: [red]
cost: 15
dp: 15000
alt_paths:
  - kind: assembly
    materials:
      - { name_contains: "WarGreymon", zones: [trash], stack_under: true }
      - { name_contains: "MetalGarurumon", zones: [trash], stack_under: true }
    cost: 6
"#;

/// A cheaper assembly card (cost 8, reduction 3) so a FULL-cost play stays
/// within the ±10 memory gauge — letting the decline test observe the paid
/// amount directly without a turn-ending memory overdraw.
const ASSEMBLY_CHEAP_YAML: &str = r#"
card: T-ASM2
name: Assembly Cheap
kind: digimon
level: 7
color: [red]
cost: 8
dp: 15000
alt_paths:
  - kind: assembly
    materials:
      - { name_contains: "WarGreymon", zones: [trash], stack_under: true }
      - { name_contains: "MetalGarurumon", zones: [trash], stack_under: true }
    cost: 3
"#;

/// Push `card_id` (already in card_data) onto player `p`'s trash; return its handle.
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let idx = runner.game.next_card_index();
    let cs = CardSource::new(data_idx, p, idx);
    let h = cs.handle();
    runner.game.players[p as usize].trash.push(cs);
    h
}

/// Playing an Assembly card with both materials in trash must surface the
/// Assembly flow (a pending selection), not silently play at full cost.
#[test]
fn assembly_play_with_materials_in_trash_installs_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CARD_YAML)
        .expect("assembly card compiles")
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM"])
        .memory(20)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "WARG");
    push_to_trash(&mut runner, 0, "METG");

    // Play the assembly card from hand (slot 0).
    let _ = runner.game.decode_action(PLAY_HAND_START, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "playing an Assembly card with its materials in trash must surface the \
         Assembly flow (G-ASSEMBLY-PLAY-EXECUTION)"
    );
}

/// Full happy path: declare Assembly, pick both materials from trash; the card
/// enters with both materials placed UNDER it (bottom of the digivolution
/// stack) and the play cost is reduced by 6 (15 → 9).
#[test]
fn assembly_play_full_flow_places_materials_under_and_reduces_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CARD_YAML)
        .expect("assembly card compiles")
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM"])
        .memory(20)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "WARG"); // trash index 0
    push_to_trash(&mut runner, 0, "METG"); // trash index 1

    let mem_before = runner.game.memory;

    // Play the assembly card → element-0 (WarGreymon) gate selection surfaces.
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(runner.game.pending_selection.is_some(), "element-0 selection");

    // Pick WarGreymon (trash index 0) → element-1 (MetalGarurumon) surfaces.
    runner.game.decode_action(TRASH_EFFECT_START, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "element-1 selection must surface after picking the first material"
    );

    // Pick MetalGarurumon (trash index 1) → play resolves.
    runner.game.decode_action(TRASH_EFFECT_START + 1, 0);
    assert!(
        runner.game.pending_selection.is_none(),
        "Assembly flow must be fully resolved after both materials are chosen"
    );

    // The played card is on the field with BOTH materials placed under it.
    let battle = &runner.game.players[0].battle_area;
    assert_eq!(battle.len(), 1, "the assembly card entered the battle area");
    let perm = &battle[0];
    assert_eq!(
        perm.card_sources.len(),
        3,
        "Omnimon-top + 2 materials underneath"
    );
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "T-ASM",
        "the assembly card is on top of the stack"
    );
    let under_ids: Vec<&str> = perm.card_sources[..2]
        .iter()
        .map(|cs| cs.card_id(&runner.game.card_data))
        .collect();
    assert!(
        under_ids.contains(&"WARG") && under_ids.contains(&"METG"),
        "both chosen materials are under the played card, got {under_ids:?}"
    );

    // Materials left the trash (placed under the permanent).
    assert!(
        runner.game.players[0].trash.is_empty(),
        "chosen materials must be removed from trash"
    );

    // Cost was reduced by 6 (15 → 9): memory dropped by exactly 9.
    assert_eq!(
        mem_before - runner.game.memory,
        9,
        "Assembly play pays the reduced cost (15 base − 6 reduction = 9)"
    );
}

/// AD1-025 Omnimon (the real card from the embedded pack) carries its printed
/// `[Assembly] −6 [WarGreymon] x [MetalGarurumon]`: with both materials in
/// trash it plays via Assembly, placing them under it and paying 15 − 6 = 9.
/// Its `dna_digivolve` path and other clauses are untouched (separate alt-path).
#[test]
fn ad1_025_omnimon_plays_via_assembly_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 loads from the embedded pack")
        .add_card(material("T-WARG", "WarGreymon"))
        .add_card(material("T-METG", "MetalGarurumon"))
        .hand(0, &["AD1-025"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "T-WARG"); // trash index 0
    push_to_trash(&mut runner, 0, "T-METG"); // trash index 1

    let mem_before = runner.game.memory;

    // Play Omnimon → Assembly gate (element 0: WarGreymon) surfaces.
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "AD1-025 with its materials in trash must surface the Assembly flow"
    );
    runner.game.decode_action(TRASH_EFFECT_START, 0); // pick WarGreymon
    assert!(
        runner.game.pending_selection.is_some(),
        "element-1 (MetalGarurumon) selection surfaces"
    );
    runner.game.decode_action(TRASH_EFFECT_START + 1, 0); // pick MetalGarurumon

    // The [On Play] body has no opponent Digimon to act on → resolves cleanly.
    assert!(
        runner.game.pending_selection.is_none(),
        "Assembly + [On Play] fully resolve (empty opponent board)"
    );

    let perm = &runner.game.players[0].battle_area[0];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "AD1-025",
        "Omnimon is on top of the stack"
    );
    assert_eq!(
        perm.card_sources.len(),
        3,
        "Omnimon + WarGreymon + MetalGarurumon underneath"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "both materials moved from trash to under Omnimon"
    );
    assert_eq!(
        mem_before - runner.game.memory,
        9,
        "AD1-025 Assembly pays 15 − 6 = 9"
    );
}

/// Declare-then-pay (Q5): at memory 0 the full cost (15) overdraws past the
/// −10 floor (0 − 15 = −15), so the play is masked out — UNLESS an eligible
/// Assembly path reduces it (15 − 6 = 9 → 0 − 9 = −9 ≥ −10). With both
/// materials in trash the play must be offered.
#[test]
fn assembly_declare_then_pay_offers_play_when_only_reduced_cost_affordable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CARD_YAML)
        .expect("assembly card compiles")
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM"])
        .memory(0)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "WARG");
    push_to_trash(&mut runner, 0, "METG");

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "Assembly makes the play legal at memory 0 (reduced cost 9 ≥ −10 floor)"
    );
}

/// Counter-case: same card and memory, but NO materials in trash → no
/// assembly reduction, so the full cost (15) overdraws and the play is masked
/// out. Confirms the mask change is gated on actual assembly eligibility.
#[test]
fn assembly_play_masked_out_when_materials_absent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CARD_YAML)
        .expect("assembly card compiles")
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM"])
        .memory(0)
        .start();
    runner.skip_mulligan();
    // Trash is empty — assembly ineligible.

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "without assembly materials the full cost (15) overdraws → play masked out"
    );
}

/// Declining the optional gate (No Selection) plays the card at FULL cost with
/// no materials placed under it.
#[test]
fn assembly_play_decline_gate_plays_at_full_cost() {
    use digimon_engine::action::PASS;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CHEAP_YAML)
        .expect("assembly card compiles")
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM2"])
        .memory(10)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "WARG");
    push_to_trash(&mut runner, 0, "METG");

    let mem_before = runner.game.memory;

    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(runner.game.pending_selection.is_some(), "gate selection");

    // Decline the gate (No Selection / PASS).
    runner.game.decode_action(PASS, 0);
    assert!(
        runner.game.pending_selection.is_none(),
        "declining the gate finishes the play"
    );

    let perm = &runner.game.players[0].battle_area[0];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "declined Assembly → no materials placed under"
    );
    assert!(
        !runner.game.players[0].trash.is_empty(),
        "declined Assembly leaves the materials in trash"
    );
    assert_eq!(
        mem_before - runner.game.memory,
        8,
        "declining Assembly pays the full printed cost (8), not the reduced 5"
    );
}
