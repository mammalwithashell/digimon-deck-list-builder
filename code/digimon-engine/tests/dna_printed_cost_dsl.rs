//! G-ENGINE-DNA-PRINTED-COST (gap 1) — DSL end-to-end.
//!
//! A DSL card whose body runs `effect_initiated_dna_digivolve` with
//! `cost: printed` must pay the RESULT card's printed `dna_costs[].memory_cost`
//! (DCGO `PlayCardClass(..., payCost: true)` → `condition.cost`), NOT 0.
//!
//! Before the fix, `cost: printed` lowered to `CostDelta::Reduce(0)` which the
//! DNA step clamped to `0` — the printed DNA cost was silently never paid.
//! This test authors an inline source card (`on_play` → select partner →
//! select hand result → DNA digivolve `cost: printed`) and a result card with
//! an explicit printed DNA cost, then drives the flow and asserts memory drops
//! by that printed cost.
//!
//! Regression: the shipped `cost: 0` / `cost: free` DNA users must remain
//! no-ops (they lower to `CostDelta::Fixed(0)` / `Free`, NOT `Reduce`), so the
//! fix must not perturb them — covered by `dna_cost_zero_is_a_noop`.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

/// Source card: [On Play] this Digimon + another of your Digimon may DNA
/// digivolve into a Digimon card in the hand for its printed DNA cost.
const SOURCE_YAML: &str = r#"
card: DNA-SRC
name: "Dna Source"
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
effects:
  - when: on_play
    optional: true
    process:
      - select_own_permanent:
          bind_as: dna_partner
          filter:
            all_of:
              - kind: digimon
              - other: true
          prompt: "Select your other DNA digivolution material"
      - select_hand:
          of: you
          bind_as: dna_result
          filter: { kind: digimon }
          prompt: "DNA digivolve into a Digimon card in your hand for its DNA digivolve cost"
      - effect_initiated_dna_digivolve:
          target_a: source
          target_b: dna_partner
          from_hand: dna_result
          cost: printed
"#;

fn material(id: &str, level: u8, color: CardColor) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(level);
    d.dp = Some(4000);
    d.colors = vec![color];
    d
}

fn req(level: u8, color: CardColor) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

/// Result card with a printed DNA recipe (Lv4 Red + Lv4 Purple) at
/// `memory_cost`.
fn result_card(memory_cost: i16) -> CardData {
    let mut d = make_test_card("DNA-RES", "Dna Result");
    d.card_kind = CardKind::Digimon;
    d.level = Some(7);
    d.dp = Some(14000);
    d.dna_costs = vec![DnaCost {
        memory_cost,
        requirement1: req(4, CardColor::Red),
        requirement2: req(4, CardColor::Purple),
    }];
    d
}

/// Drive the flow: play the source, select the Purple partner, select the
/// hand result. Returns (memory paid, whether the result reached the field).
fn run_dna_flow(result_memory_cost: i16, start_memory: i16) -> (i16, bool) {
    // DNA-SRC is level 4 RED (matches req1). The partner is level 4 PURPLE
    // (matches req2). The result requires Red + Purple.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(SOURCE_YAML)
        .expect("source YAML compiles")
        .add_card(material("PARTNER-PUR", 4, CardColor::Purple))
        .add_card(result_card(result_memory_cost))
        .hand(0, &["DNA-RES"])
        .memory(start_memory)
        .build();

    // Give DNA-SRC the red level-4 identity so it satisfies req1.
    let _src = runner.place_on_field(0, "DNA-SRC", Some(0));
    let _partner = runner.place_on_field(0, "PARTNER-PUR", Some(0));

    let memory_before = runner.game.memory;

    // Fire the source's OnPlay directly (it's already on field).
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(_src));
    runner.game.drain_effect_queue();

    // Drive every pending selection, preferring a non-PASS action so the DNA
    // digivolve is actually performed.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 40 {
        let sel = runner.game.pending_selection.as_ref().unwrap();
        let player = sel.selecting_player;
        let action = sel
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        let _ = runner.game.resolve_selection(player, action);
        runner.game.drain_effect_queue();
        steps += 1;
    }

    let result_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "DNA-RES");
    let paid = memory_before - runner.game.memory;
    (paid, result_on_field)
}

#[test]
fn dna_cost_printed_pays_result_printed_dna_cost() {
    // Result prints a DNA cost of 3. `cost: printed` must charge 3.
    let (paid, on_field) = run_dna_flow(3, 8);
    assert!(
        on_field,
        "DNA digivolve must complete (result on field) with a recipe-satisfying pair"
    );
    assert_eq!(
        paid, 3,
        "`cost: printed` must pay the result's printed DNA cost (3), not 0"
    );
}

#[test]
fn dna_cost_printed_pays_higher_cost() {
    // A different printed cost proves the value is read from the card, not a
    // constant. Result prints 5 → charge 5.
    let (paid, on_field) = run_dna_flow(5, 9);
    assert!(on_field, "DNA digivolve completes");
    assert_eq!(
        paid, 5,
        "`cost: printed` must pay the result's printed DNA cost (5)"
    );
}

#[test]
fn dna_cost_printed_zero_is_a_noop() {
    // A result that prints DNA cost 0 pays 0 — `cost: printed` reading a
    // 0-cost recipe is a no-op, exactly like the shipped `cost: 0` users.
    let (paid, on_field) = run_dna_flow(0, 8);
    assert!(on_field, "DNA digivolve completes");
    assert_eq!(paid, 0, "a printed DNA cost of 0 pays nothing");
}

// ═══════════════════════════════════════════════════════════════════════════
// G-ENGINE-DNA-RECIPE-ENFORCEMENT (gap 2) — mask-level: an ILLEGAL result
// (a hand card whose printed DNA recipe the field pair does NOT satisfy)
// must NEVER be a legal target action in the `may_dna_digivolve_now` chain.
// DCGO parity: `CardFulfillsRequirement` filters the SelectHand candidates.
// ═══════════════════════════════════════════════════════════════════════════

/// Inherited [End of Your Turn] DNA-digivolve card (the `may_dna_digivolve_now`
/// path). Its recipe check must exclude a hand result whose printed DNA recipe
/// the {anchor, partner} pair cannot satisfy.
const EOT_DNA_YAML: &str = r#"
card: EOT-DNA
name: "Eot Dna Anchor"
kind: digimon
level: 4
color: [red]
cost: 3
dp: 4000
effects:
  - when: on_play
    optional: true
    process:
      - may_dna_digivolve_now:
          partner_filter:
            all_of:
              - of: you
              - kind: digimon
              - other: true
          target_filter:
            all_of:
              - of: you
              - kind: digimon
          cost: 0
          ignore_requirements: false
"#;

/// The recipe filter excludes an off-recipe result from the target action set,
/// while a recipe-satisfying result remains selectable. Anchor = Lv4 Red,
/// partner = Lv4 Purple:
///   - LEGAL-RES needs Red + Purple  → satisfied → selectable
///   - ILLEGAL-RES needs Blue + Green → not satisfied → excluded from mask
#[test]
fn eot_dna_recipe_masks_out_illegal_result() {
    let legal = {
        let mut d = make_test_card("LEGAL-RES", "Legal Result");
        d.card_kind = CardKind::Digimon;
        d.level = Some(7);
        d.dp = Some(14000);
        d.dna_costs = vec![DnaCost {
            memory_cost: 0,
            requirement1: req(4, CardColor::Red),
            requirement2: req(4, CardColor::Purple),
        }];
        d
    };
    let illegal = {
        let mut d = make_test_card("ILLEGAL-RES", "Illegal Result");
        d.card_kind = CardKind::Digimon;
        d.level = Some(7);
        d.dp = Some(14000);
        d.dna_costs = vec![DnaCost {
            memory_cost: 0,
            requirement1: req(4, CardColor::Blue),
            requirement2: req(4, CardColor::Green),
        }];
        d
    };

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EOT_DNA_YAML)
        .expect("EoT DNA YAML compiles")
        .add_card(material("PARTNER-PUR", 4, CardColor::Purple))
        .add_card(legal)
        .add_card(illegal)
        .hand(0, &["LEGAL-RES", "ILLEGAL-RES"])
        .memory(10)
        .start();

    let anchor = runner.place_on_field(0, "EOT-DNA", Some(0)); // Lv4 Red
    let _partner = runner.place_on_field(0, "PARTNER-PUR", Some(0)); // Lv4 Purple

    // Fire the face-up OnPlay trigger (the anchor is already on the field).
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::SelectionKind;
    use digimon_engine::selection::TriggerSource;
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(anchor));
    runner.game.drain_effect_queue();

    // Advance the selection chain until the HAND (target) selection installs.
    // The chain is: optional accept/decline → partner (own field) → target
    // (hand). Resolve each non-hand prompt with a non-PASS action.
    use digimon_engine::action::space::PLAY_HAND_START;
    let mut steps = 0;
    loop {
        let kind = runner
            .pending_kind()
            .expect("a selection is pending until the target hand prompt");
        if kind == SelectionKind::Hand {
            break;
        }
        assert!(steps < 10, "chain did not reach the hand target prompt");
        let (player, action) = {
            let sel = runner.game.pending_selection.as_ref().unwrap();
            let action = sel
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .expect("a non-PASS action exists on the pre-target prompts");
            (sel.selecting_player, action)
        };
        runner
            .game
            .resolve_selection(player, action)
            .expect("resolve pre-target prompt");
        steps += 1;
    }

    // Target (hand-result) selection. LEGAL-RES must be selectable; ILLEGAL-RES
    // must NOT be — its recipe (Blue+Green) is unsatisfiable by the Red+Purple
    // pair (mask-level recipe enforcement, gap 2).
    let sel = runner.game.pending_selection.as_ref().unwrap();
    let legal_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LEGAL-RES")
        .expect("LEGAL-RES in hand");
    let illegal_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "ILLEGAL-RES")
        .expect("ILLEGAL-RES in hand");

    let legal_action = PLAY_HAND_START + legal_idx as u16;
    let illegal_action = PLAY_HAND_START + illegal_idx as u16;

    assert!(
        sel.valid_action_ids.contains(&legal_action),
        "recipe-satisfying result (Red+Purple) must be a legal DNA target; \
         valid={:?}",
        sel.valid_action_ids
    );
    assert!(
        !sel.valid_action_ids.contains(&illegal_action),
        "off-recipe result (Blue+Green) must NOT reach the action space \
         (recipe unsatisfiable by the Red+Purple pair); valid={:?}",
        sel.valid_action_ids
    );
}
