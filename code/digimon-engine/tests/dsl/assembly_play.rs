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

use digimon_engine::action::{build_action_mask, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
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

fn assembly_payment_fixture(memory: i16) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ASSEMBLY_CARD_YAML).unwrap()
        .add_card(material("WARG", "WarGreymon"))
        .add_card(material("METG", "MetalGarurumon"))
        .hand(0, &["T-ASM"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    push_to_trash(&mut runner, 0, "WARG");
    push_to_trash(&mut runner, 0, "METG");
    runner
}

#[test]
fn assembly_cannot_decline_when_reduction_is_required_to_pay_in_live_and_cloned_games() {
    // CR 1-3-11-4: declaring a 15-cost play at zero memory commits the
    // player to the Assembly reduction needed to keep payment within -10.
    let mut runner = assembly_payment_fixture(0);
    assert_eq!(build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize], 1.0);
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(!runner.game.pending_selection.as_ref().unwrap().is_optional,
        "Assembly cannot be declined when the unreduced play cannot be paid");
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 0.0);
    assert!(!runner.game.pending_selection.as_ref().unwrap().prompt.contains("skip"));

    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        for action in [TRASH_EFFECT_START, TRASH_EFFECT_START + 1] {
            let mask = build_action_mask(game, 0);
            assert_eq!(mask[PASS as usize], 0.0, "required materials stay mandatory after cloning/picking");
            assert_eq!(mask[action as usize], 1.0);
            game.decode_action(action, 0);
        }
        assert!(game.pending_selection.is_none());
        assert!(game.players[0].hand.is_empty());
        assert!(game.players[0].trash.is_empty());
        assert_eq!(game.players[0].battle_area[0].card_sources.len(), 3);
        assert_eq!(game.turn_player(), 1);
        assert_eq!(game.memory, 9, "the opponent receives the reduced 9-cost payment");
    }
}

#[test]
fn assembly_decline_tracks_the_exact_unreduced_payment_boundary() {
    for (memory, may_decline) in [(4, false), (5, true)] {
        let mut runner = assembly_payment_fixture(memory);
        runner.game.decode_action(PLAY_HAND_START, 0);
        assert_eq!(runner.game.pending_selection.as_ref().unwrap().is_optional, may_decline);
        assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize] > 0.5, may_decline);
        if may_decline {
            runner.game.decode_action(PASS, 0);
            assert!(runner.game.players[0].hand.is_empty());
            assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 1);
            assert_eq!(runner.game.players[0].trash.len(), 2);
            assert_eq!(runner.game.memory, 10);
        }
    }
}

#[test]
fn assembly_decline_uses_the_effects_reduced_cost_instead_of_printed_cost() {
    use digimon_engine::enums::{CostDelta, PlaySource};
    let mut runner = assembly_payment_fixture(0);
    runner.game.play_from_hand_with_cost(0, 0, CostDelta::Reduce(5), PlaySource::ByEffect);
    assert!(runner.game.pending_selection.as_ref().unwrap().is_optional,
        "15 minus the effect's 5-cost reduction is payable without Assembly");
    assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 1.0);
    runner.game.decode_action(PASS, 0);
    assert!(runner.game.players[0].hand.is_empty());
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(runner.game.players[0].trash.len(), 2);
    assert_eq!(runner.game.memory, 10);
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
    assert!(
        runner.game.pending_selection.is_some(),
        "element-0 selection"
    );

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

#[test]
fn assembly_material_prompts_clone_faithfully() {
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

    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "Assembly element-0 prompt must be resume-driven before cloning"
    );
    let first_action = runner
        .game
        .pending_selection
        .as_ref()
        .expect("element-0 prompt installed")
        .valid_action_ids[0];

    let mut clone_at_first = runner.game.clone();
    clone_at_first
        .resolve_selection(0, first_action)
        .expect("clone picks first Assembly material");
    assert!(
        clone_at_first.pending_selection_resume.is_some(),
        "Assembly element-1 prompt installed by clone must also be resume-driven"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone leaves the original at element 0"
    );

    let second_action = clone_at_first
        .pending_selection
        .as_ref()
        .expect("element-1 prompt installed")
        .valid_action_ids[0];
    let mut clone_at_second = clone_at_first.clone();
    clone_at_second
        .resolve_selection(0, second_action)
        .expect("clone picks second Assembly material");

    assert!(
        clone_at_second.pending_selection.is_none(),
        "clone completes the Assembly flow"
    );
    assert_eq!(
        clone_at_second.player(0).battle_area.len(),
        1,
        "clone: Assembly card entered"
    );
    assert_eq!(
        clone_at_second.player(0).battle_area[0].card_sources.len(),
        3,
        "clone: result has both materials underneath"
    );
    assert!(
        clone_at_first.pending_selection.is_some(),
        "resolving the element-1 clone leaves its parent clone at element 1"
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

// Bounded Assembly cardinality regressions. Synthetic materials isolate the
// count/name decision; they do not approve material identities or card effects.
// Official general_rule.pdf p15: 7-3-2-4/5 exact count, 7-3-2-7/9 optional zero.
mod counted_assembly_regressions {
    use digimon_engine::action::{build_action_mask, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
    use digimon_engine::card_source::CardSource;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner};
    use digimon_engine::enums::{CardKind, GamePhase};
    use digimon_engine::selection::SelectionKind;

    const EX12_076: &str = include_str!("../../cards/ex12/EX12-076.yaml");
    const TWO_THEN_ONE: &str = r#"
card: T-ASM-EXACT
name: Counted Assembly Test
kind: digimon
level: 7
color: [red]
cost: 8
dp: 15000
alt_paths:
  - kind: assembly
    materials:
      - filter: { trait_has: Hybrid }
        repeat: { min: 2, max: 2 }
        distinct_by: name
        zones: [trash]
        stack_under: true
      - filter: { trait_has: Shambala }
        zones: [trash]
        stack_under: true
    cost: 3
"#;

    fn fixture(seat: u8, yaml: &str, target: &str, traits: &[&str]) -> DebugRunner {
        let mut builder = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().hand(seat, &[target]);
        for (i, trait_name) in traits.iter().enumerate() {
            let mut card = make_test_card(&format!("MAT-{i}"), &format!("Material {i}"));
            card.card_kind = CardKind::Digimon;
            card.traits = vec![(*trait_name).into()];
            builder = builder.add_card(card);
        }
        let mut r = builder.start();
        r.skip_mulligan();
        r.game.turn_player_idx = seat as usize;
        r.game.memory_pair = (seat, 1 - seat);
        r.game.current_phase = GamePhase::Main;
        r.game.set_memory(10);
        for i in 0..traits.len() {
            let id = format!("MAT-{i}");
            let data_index = r.game.card_data.iter().position(|d| d.card_id == id).unwrap();
            let card_index = r.game.next_card_index();
            r.game.players[seat as usize].trash.push(CardSource::new(data_index, seat, card_index));
        }
        r
    }

    fn ex12(seat: u8) -> DebugRunner {
        fixture(seat, EX12_076, "EX12-076", &["Hybrid", "Shambala", "Hybrid", "Shambala", "Hybrid", "Shambala", "Hybrid", "Shambala"])
    }

    fn legal(r: &mut DebugRunner, seat: u8, action: u16) {
        assert_eq!(build_action_mask(&r.game, seat)[action as usize], 1.0, "chosen action must be legal");
        r.game.decode_action(action, seat);
    }

    fn unchanged_pending(r: &DebugRunner, seat: u8, picked: u8, count: u8, trash: usize) {
        let pending = r.game.pending_selection.as_ref().expect("counted selection remains pending");
        assert_eq!(build_action_mask(&r.game, seat)[PASS as usize], 0.0);
        assert!(!pending.is_optional, "a nonzero incomplete prefix cannot finish");
        assert!(matches!(pending.kind, SelectionKind::CountCappedMultiSelect { min, max, picked: p, .. } if min == count && max == count && p == picked));
        assert_eq!(build_action_mask(&r.game, 1 - seat)[PASS as usize], 0.0);
        assert_eq!(r.game.memory, 10);
        assert!(r.game.players[seat as usize].battle_area.is_empty());
        assert_eq!(r.game.players[seat as usize].hand.len(), 1);
        assert_eq!(r.game.players[seat as usize].trash.len(), trash);
    }

    fn prefix(seat: u8, count: u8) {
        let mut r = ex12(seat);
        legal(&mut r, seat, PLAY_HAND_START);
        for i in 0..count { legal(&mut r, seat, TRASH_EFFECT_START + u16::from(i)); }
        unchanged_pending(&r, seat, count, 8, 8);
        assert!(r.game.resolve_selection(seat, PASS).is_err(), "resolver rejects an incomplete finish too");
        unchanged_pending(&r, seat, count, 8, 8);
    }

    macro_rules! prefix_case { ($name:ident, $seat:expr, $n:expr) => { #[test] fn $name() { prefix($seat, $n); } }; }
    prefix_case!(ex12_prefix_1_seat0, 0, 1); prefix_case!(ex12_prefix_1_seat1, 1, 1);
    prefix_case!(ex12_prefix_2_seat0, 0, 2); prefix_case!(ex12_prefix_2_seat1, 1, 2);
    prefix_case!(ex12_prefix_3_seat0, 0, 3); prefix_case!(ex12_prefix_3_seat1, 1, 3);
    prefix_case!(ex12_prefix_4_seat0, 0, 4); prefix_case!(ex12_prefix_4_seat1, 1, 4);
    prefix_case!(ex12_prefix_5_seat0, 0, 5); prefix_case!(ex12_prefix_5_seat1, 1, 5);
    prefix_case!(ex12_prefix_6_seat0, 0, 6); prefix_case!(ex12_prefix_6_seat1, 1, 6);
    prefix_case!(ex12_prefix_7_seat0, 0, 7); prefix_case!(ex12_prefix_7_seat1, 1, 7);

    fn decline(seat: u8) {
        let mut r = ex12(seat);
        legal(&mut r, seat, PLAY_HAND_START);
        legal(&mut r, seat, PASS);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[seat as usize].battle_area[0].card_sources.len(), 1);
        assert_eq!(r.game.players[seat as usize].trash.len(), 8);
        assert_eq!(r.game.turn_player_idx, (1 - seat) as usize);
        assert_eq!(r.game.memory, 6, "full cost16 paid from10; memory renormalizes on opponent turn");
    }
    #[test] fn ex12_zero_decline_seat0() { decline(0); }
    #[test] fn ex12_zero_decline_seat1() { decline(1); }

    fn complete(seat: u8, callback: bool) {
        let mut r = ex12(seat);
        legal(&mut r, seat, PLAY_HAND_START);
        if callback { r.game.pending_selection_resume = None; }
        for i in (0..8).rev() {
            legal(&mut r, seat, TRASH_EFFECT_START + i);
            if callback && i > 0 { unchanged_pending(&r, seat, (8 - i) as u8, 8, 8); }
        }
        let p = &r.game.players[seat as usize];
        assert_eq!(p.battle_area.len(), 1);
        assert_eq!(p.battle_area[0].card_sources.len(), 9);
        assert_eq!(p.battle_area[0].top_card().card_id(&r.game.card_data), "EX12-076");
        assert_eq!(p.trash.len(), 0);
        assert_eq!(r.game.memory, 3, "cost16 minus9 pays7 only after all eight materials");
    }
    #[test] fn ex12_all_eight_seat0() { complete(0, false); }
    #[test] fn ex12_all_eight_seat1() { complete(1, false); }
    #[test] fn ex12_callback_zero_or_eight_seat0() { complete(0, true); }
    #[test] fn ex12_callback_zero_or_eight_seat1() { complete(1, true); }

    fn cloned(seat: u8) {
        let mut r = ex12(seat);
        legal(&mut r, seat, PLAY_HAND_START);
        for i in 0..3 { legal(&mut r, seat, TRASH_EFFECT_START + i); }
        let mut clone = r.game.clone();
        assert_eq!(build_action_mask(&clone, seat)[PASS as usize], 0.0);
        assert!(clone.resolve_selection(seat, PASS).is_err());
        for i in 3..8 {
            assert_eq!(build_action_mask(&clone, seat)[(TRASH_EFFECT_START + i) as usize], 1.0);
            clone.resolve_selection(seat, TRASH_EFFECT_START + i).unwrap();
        }
        assert_eq!(clone.players[seat as usize].battle_area[0].card_sources.len(), 9);
        assert_eq!(clone.memory, 3);
        unchanged_pending(&r, seat, 3, 8, 8);
    }
    #[test] fn ex12_clone_after_three_seat0() { cloned(0); }
    #[test] fn ex12_clone_after_three_seat1() { cloned(1); }

    fn multi_element(seat: u8) {
        let mut r = fixture(seat, TWO_THEN_ONE, "T-ASM-EXACT", &["Hybrid", "Hybrid", "Shambala"]);
        legal(&mut r, seat, PLAY_HAND_START);
        assert_eq!(build_action_mask(&r.game, seat)[(TRASH_EFFECT_START + 2) as usize], 0.0);
        legal(&mut r, seat, TRASH_EFFECT_START);
        unchanged_pending(&r, seat, 1, 2, 3);
        legal(&mut r, seat, TRASH_EFFECT_START + 1);
        unchanged_pending(&r, seat, 0, 1, 3);
        assert!(r.game.resolve_selection(seat, PASS).is_err(), "later recipe element cannot be skipped");
        legal(&mut r, seat, TRASH_EFFECT_START + 2);
        assert_eq!(r.game.players[seat as usize].battle_area[0].card_sources.len(), 4);
        assert_eq!(r.game.players[seat as usize].trash.len(), 0);
        assert_eq!(r.game.memory, 5, "complete recipe pays8-3, never a partial recipe");
    }
    #[test] fn repeated_first_then_required_second_seat0() { multi_element(0); }
    #[test] fn repeated_first_then_required_second_seat1() { multi_element(1); }

    fn too_few(seat: u8) {
        let mut r = fixture(seat, EX12_076, "EX12-076", &["Hybrid"; 7]);
        r.game.set_memory(0);
        assert_eq!(build_action_mask(&r.game, seat)[PLAY_HAND_START as usize], 0.0,
            "seven materials cannot make the reduced play affordable");
    }
    #[test] fn insufficient_seven_no_reduction_seat0() { too_few(0); }
    #[test] fn insufficient_seven_no_reduction_seat1() { too_few(1); }
}
