//! Tests for closure-valued cost reductions via BeforePayCost scan.
//!
//! These verify that cost_reduction_fn closures attached to battle-area
//! permanents correctly reduce play costs, stack with static reductions,
//! are clamped appropriately, and are applied on all digivolve paths.

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, EffectTiming, PlaySource};
use digimon_engine::permanent::Permanent;
use std::sync::Arc;

fn plain_digimon(card_id: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(2000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// CardEffect that reduces cost by the size of player 0's trash (live state).
struct TrashSizeCostReduction;
impl CardEffect for TrashSizeCostReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("reduce by trash size")
            .condition(|_| true)
            .cost_reduction_fn(|ctx| ctx.player_state(0).trash.len() as i32)
            .build()]
    }
}

/// CardEffect with a static cost_reduction of 1.
struct StaticReductionOne;
impl CardEffect for StaticReductionOne {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("static -1")
            .cost_reduction(1)
            .build()]
    }
}

/// CardEffect with a closure that returns 2.
struct ClosureReductionTwo;
impl CardEffect for ClosureReductionTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("closure -2")
            .cost_reduction_fn(|_| 2)
            .build()]
    }
}

/// CardEffect whose closure returns a negative value (should clamp to 0).
struct NegativeReturnClosure;
impl CardEffect for NegativeReturnClosure {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("negative closure")
            .cost_reduction_fn(|_| -5)
            .build()]
    }
}

/// CardEffect whose closure returns an oversized reduction (99).
struct HugeCostReduction;
impl CardEffect for HugeCostReduction {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("huge reduction")
            .cost_reduction_fn(|_| 99)
            .build()]
    }
}

/// No-effect card for the permanent providing no reduction (control group).
struct NoEffect;
impl CardEffect for NoEffect {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        vec![]
    }
}

#[test]
fn closure_valued_cost_reduction_reads_live_state() {
    // Player 0 has 2 cards in trash — we set this up by pre-loading trash via
    // place_on_field then deleting them, but it's simpler to just push cards
    // into trash directly via game_mut().
    //
    // Setup: P0 has a reducer permanent on field; P0 has 2 trash cards.
    // Play a Digimon from hand with printed_cost = 7.
    // Memory starts at 10 (so play is affordable).
    // Expected: effective cost = 7 - 2 = 5; memory = 10 - 5 = 5.
    let target = plain_digimon("TARGET", 7);
    let reducer = make_test_card("REDUCER", "Reducer");
    let filler = make_test_card("FILLER", "Filler");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer.clone())
        .add_card(filler.clone())
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("REDUCER", Arc::new(TrashSizeCostReduction));
    r.register_effect("FILLER", Arc::new(NoEffect));

    // Place the reducer on the field.
    r.place_on_field(0, "REDUCER", Some(0));

    // Add 2 cards to player 0's trash directly.
    let game = r.game_mut();
    let filler_card1 = {
        let idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "FILLER")
            .unwrap();
        let ci = game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, ci)
    };
    let filler_card2 = {
        let idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "FILLER")
            .unwrap();
        let ci = game.next_card_index();
        digimon_engine::card_source::CardSource::new(idx, 0, ci)
    };
    game.players[0].trash.push(filler_card1);
    game.players[0].trash.push(filler_card2);

    assert_eq!(r.game.players[0].trash.len(), 2);

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // effective cost = 7 - 2 = 5; memory = 10 - 5 = 5
    assert_eq!(
        r.memory(),
        memory_before - 5,
        "cost should be 7 - 2 (trash size) = 5"
    );
}

#[test]
fn static_cost_reduction_stacks_with_closure_reduction() {
    // Two separate battle-area permanents:
    //   A: static cost_reduction(1)
    //   B: closure cost_reduction_fn(|_| 2)
    // Play a Digimon with printed_cost = 5.
    // Expected: effective cost = 5 - 3 = 2.
    let target = plain_digimon("TARGET", 5);
    let perm_a = make_test_card("PERM-A", "PermA");
    let perm_b = make_test_card("PERM-B", "PermB");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(perm_a)
        .add_card(perm_b)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("PERM-A", Arc::new(StaticReductionOne));
    r.register_effect("PERM-B", Arc::new(ClosureReductionTwo));

    r.place_on_field(0, "PERM-A", Some(0));
    r.place_on_field(0, "PERM-B", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    // effective cost = 5 - 1 - 2 = 2; memory = 10 - 2 = 8
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "static 1 + closure 2 = total reduction 3; cost 5 - 3 = 2"
    );
}

#[test]
fn cost_reduction_fn_returning_negative_does_not_increase_cost() {
    // Closure returns -5. Per-effect clamp: max(0, -5) = 0.
    // No reduction applied.
    // Play a Digimon with printed_cost = 3.
    // Expected: effective cost = 3 (no reduction).
    let target = plain_digimon("TARGET", 3);
    let reducer = make_test_card("NEG-REDUCER", "NegReducer");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("NEG-REDUCER", Arc::new(NegativeReturnClosure));
    r.place_on_field(0, "NEG-REDUCER", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before - 3,
        "negative closure return should not increase cost; effective cost = 3"
    );
}

#[test]
fn effective_cost_floors_at_zero() {
    // Closure returns 99. Printed cost = 3.
    // Total reduction = 99, but effective_cost = max(0, 3 - 99) = 0.
    // Expected: effective cost = 0 (not negative); memory unchanged.
    let target = plain_digimon("TARGET", 3);
    let reducer = make_test_card("HUGE-REDUCER", "HugeReducer");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(reducer)
        .hand(0, &["TARGET"])
        .memory(10)
        .start();

    r.register_effect("HUGE-REDUCER", Arc::new(HugeCostReduction));
    r.place_on_field(0, "HUGE-REDUCER", Some(0));

    let memory_before = r.memory(); // 10
    r.play(0, 0);

    assert_eq!(
        r.memory(),
        memory_before, // cost = 0, no memory change
        "effective cost should floor at 0 when reduction exceeds printed cost"
    );
}

// ─── Digivolve-path wiring tests ─────────────────────────────────────────────

/// Build a Lv3 Digimon CardData for use as the base of a digivolution stack.
fn lv3_digimon(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(2000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        // evo_costs populated separately on the Lv4 card
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Build a Lv4 Digimon CardData with a Red/Lv3 evo_cost of `digi_cost`.
fn lv4_digimon(card_id: &str, digi_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: digi_cost,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn digivolve_from_hand_applies_before_pay_cost_reduction() {
    // Setup:
    //   - P0 has a Lv3 Digimon (BASE) on field.
    //   - A separate permanent (REDUCER) on P0's field carries a static
    //     BeforePayCost cost_reduction(2).
    //   - P0's hand contains a Lv4 Digimon (DIGI) with evo_cost 3 onto Lv3 Red.
    //   - Memory = 10.
    // Expected: effective digivolve cost = 3 - 2 = 1; memory = 10 - 1 = 9.
    let base = lv3_digimon("BASE");
    let digi = lv4_digimon("DIGI", 3);
    let reducer = make_test_card("REDUCER", "Reducer");

    let mut r = DebugRunner::builder()
        .add_card(base)
        .add_card(digi)
        .add_card(reducer)
        .hand(0, &["DIGI"])
        .memory(10)
        .start();

    // Place BASE and REDUCER on the field.
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "REDUCER", Some(0));

    // Register the static -2 reducer.
    r.register_effect("REDUCER", Arc::new(StaticReductionTwo));

    // Advance from Breeding to Main phase (DebugRunner.start() leaves us in Breeding).
    r.game_mut().enter_main_phase();

    let memory_before = r.memory(); // 10
                                    // Digivolve hand[0] (DIGI) onto field[0] (BASE); REDUCER is at field[1].
    let ok = r
        .game_mut()
        .digivolve_from_hand(0, 0, 0, PlaySource::ByDigivolve);
    assert!(ok, "digivolve should succeed");

    // Effective cost = 3 - 2 = 1.
    assert_eq!(
        r.memory(),
        memory_before - 1,
        "BeforePayCost reduction should apply on digivolve_from_hand; effective cost = 3 - 2 = 1"
    );
    // Stack grew: BASE (Lv3) + DIGI (Lv4) = 2 sources.
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        2,
        "digivolution stack should grow to 2"
    );
}

/// Static cost_reduction(2) — used by digivolve tests above.
struct StaticReductionTwo;
impl CardEffect for StaticReductionTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("static -2")
            .cost_reduction(2)
            .build()]
    }
}

#[test]
fn digivolve_onto_breeding_applies_before_pay_cost_reduction() {
    // Setup:
    //   - P0 has a Lv3 Digimon (BASE-EGG) in the breeding area.
    //   - A permanent (REDUCER) on P0's field carries cost_reduction(1).
    //   - P0's hand contains a Lv4 Digimon (DIGI-B) with evo_cost 3 onto Lv3 Red.
    //   - Memory = 10.
    // Expected: effective digivolve cost = 3 - 1 = 2; memory = 10 - 2 = 8.
    let base_egg = lv3_digimon("BASE-EGG");
    let digi_b = lv4_digimon("DIGI-B", 3);
    let reducer = make_test_card("REDUCER-B", "ReducerB");

    let mut r = DebugRunner::builder()
        .add_card(base_egg)
        .add_card(digi_b)
        .add_card(reducer)
        .hand(0, &["DIGI-B"])
        .memory(10)
        .start();

    // Place REDUCER-B on the battle area.
    r.place_on_field(0, "REDUCER-B", Some(0));
    r.register_effect("REDUCER-B", Arc::new(StaticReductionOne));

    // Manually place BASE-EGG into P0's breeding area (bypass hatch machinery).
    {
        let game = r.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE-EGG")
            .unwrap();
        let ci = game.next_card_index();
        let card = CardSource::new(data_idx, 0, ci);
        let turn = game.turn_count;
        game.players[0].breeding_area = Some(Permanent::new(card, turn));
    }

    // Advance from Breeding to Main phase (DebugRunner.start() leaves us in Breeding).
    r.game_mut().enter_main_phase();

    let memory_before = r.memory(); // 10
    let ok = r
        .game_mut()
        .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByDigivolve);
    assert!(ok, "digivolve onto breeding should succeed");

    // Effective cost = 3 - 1 = 2.
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "BeforePayCost reduction should apply on digivolve_from_hand_onto_breeding; effective cost = 3 - 1 = 2"
    );
    let breeding = r.game.players[0].breeding_area.as_ref().unwrap();
    assert_eq!(
        breeding.card_sources.len(),
        2,
        "breeding stack should grow to 2"
    );
}

#[test]
fn effect_initiated_digivolve_stacks_cost_delta_with_reduction() {
    // Setup:
    //   - P0 has a Lv3 Digimon (TARGET-EID) on the field.
    //   - A permanent (REDUCER-EID) on P0's field carries BeforePayCost
    //     cost_reduction(2).
    //   - P0's hand contains a Lv4 Digimon (DIGI-EID) with printed evo_cost 5.
    //   - We call effect_initiated_digivolve with CostDelta::Reduce(1).
    //   - Memory = 10.
    // Expected:
    //   base_cost = CostDelta::Reduce(1).resolve(5) = 4
    //   scan reduction = 2
    //   effective_cost = (4 - 2).max(0) = 2
    //   memory = 10 - 2 = 8.
    let target = lv3_digimon("TARGET-EID");
    let digi_eid = lv4_digimon("DIGI-EID", 5);
    let reducer = make_test_card("REDUCER-EID", "ReducerEid");

    let mut r = DebugRunner::builder()
        .add_card(target)
        .add_card(digi_eid)
        .add_card(reducer)
        .hand(0, &["DIGI-EID"])
        .memory(10)
        .start();

    // Place TARGET-EID and REDUCER-EID on the field.
    let target_handle = r.place_on_field(0, "TARGET-EID", Some(0));
    r.place_on_field(0, "REDUCER-EID", Some(0));
    r.register_effect("REDUCER-EID", Arc::new(StaticReductionTwo));

    let memory_before = r.memory(); // 10
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // hand_index
        target_handle,
        CostDelta::Reduce(1),
        false, // do not ignore color
        PlaySource::ByEffect,
    );
    assert!(ok, "effect_initiated_digivolve should succeed");

    // base_cost = 5 - 1 = 4 (CostDelta::Reduce(1))
    // scan reduction = 2
    // effective_cost = (4 - 2).max(0) = 2
    assert_eq!(
        r.memory(),
        memory_before - 2,
        "CostDelta::Reduce(1) + BeforePayCost(-2) should yield effective cost = 5 - 1 - 2 = 2"
    );
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        2,
        "stack should grow to 2 after effect_initiated_digivolve"
    );
}

// ─── Inherited-filter test ────────────────────────────────────────────────────

/// A BeforePayCost effect with inherited=true (cost_reduction 3).
/// Simulates a card that is "in the stack" of a digivolved permanent.
struct InheritedBeforePayCost;
impl CardEffect for InheritedBeforePayCost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        // Effect::inherited starts with timing=None, inherited=true.
        // Override timing to BeforePayCost so the scan includes it, but only
        // when the source is in an under position (is_under == true).
        vec![Effect::inherited(card)
            .timing(EffectTiming::BeforePayCost)
            .name("inherited BeforePayCost -3")
            .cost_reduction(3)
            .build()]
    }
}

/// A BeforePayCost effect with inherited=false (cost_reduction 3) — top-card only.
struct TopCardBeforePayCost;
impl CardEffect for TopCardBeforePayCost {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card)
            .name("top-card BeforePayCost -3")
            .cost_reduction(3)
            .build()]
    }
}

#[test]
fn inherited_effect_in_stack_fires_from_under_position() {
    // Verifies Issue 1 (inherited filter): a BeforePayCost effect with
    // inherited=true should fire when the source is in an under-card position,
    // but NOT when it is the top card. Conversely, a non-inherited effect
    // should fire from the top card but NOT from an under position.
    //
    // Scenario:
    //   - P0 has a two-card digivolve stack on field[0]:
    //       stack[0] (bottom / under): INHERIT-SRC (inherited=true, reduction=3)
    //       stack[1] (top):            TOP-SRC      (inherited=false, reduction=3)
    //   - P0's hand has TARGET with play_cost=8.
    //   - Memory = 10.
    //
    // Expected reductions:
    //   INHERIT-SRC at position 0 (is_under=true):  inherited=true  → fires → -3
    //   TOP-SRC     at position 1 (is_under=false): inherited=false → fires → -3
    //   Total reduction = 6; effective_cost = 8 - 6 = 2; memory = 10 - 2 = 8.
    //
    // If the inherited filter were ABSENT, both cards would fire regardless of
    // position, yielding the same total=6. To specifically exercise the guard,
    // we run a SECOND scenario where the cards' positions are swapped so the
    // under card has inherited=false (should NOT fire) and the top card has
    // inherited=true (should NOT fire): total reduction = 0, effective cost = 8.

    let target = plain_digimon("TARGET-IHF", 8);
    let inherit_src = make_test_card("INHERIT-SRC", "InheritSrc");
    let top_src = make_test_card("TOP-SRC", "TopSrc");

    // ── Part A: correct positions → both fire ─────────────────────────────
    {
        let mut r = DebugRunner::builder()
            .add_card(target.clone())
            .add_card(inherit_src.clone())
            .add_card(top_src.clone())
            .hand(0, &["TARGET-IHF"])
            .memory(10)
            .start();

        r.register_effect("INHERIT-SRC", Arc::new(InheritedBeforePayCost));
        r.register_effect("TOP-SRC", Arc::new(TopCardBeforePayCost));

        // Build a 2-card stack on field[0]:
        //   position 0 (bottom/under) = INHERIT-SRC
        //   position 1 (top)          = TOP-SRC
        {
            let game = r.game_mut();
            let inh_idx = game
                .card_data
                .iter()
                .position(|c| c.card_id == "INHERIT-SRC")
                .unwrap();
            let top_idx = game
                .card_data
                .iter()
                .position(|c| c.card_id == "TOP-SRC")
                .unwrap();
            let ci0 = game.next_card_index();
            let ci1 = game.next_card_index();
            let under = CardSource::new(inh_idx, 0, ci0);
            let top = CardSource::new(top_idx, 0, ci1);
            let turn = game.turn_count;
            let mut perm = Permanent::new(under, turn);
            perm.card_sources.push(top);
            game.players[0].battle_area.push(perm);
        }

        let memory_before = r.memory();
        r.play(0, 0);
        // total reduction = 3 (inherited under) + 3 (top-card top) = 6
        assert_eq!(
            r.memory(),
            memory_before - 2,
            "Part A: both effects fire from correct positions; effective cost = 8 - 6 = 2"
        );
    }

    // ── Part B: swapped positions → neither fires ─────────────────────────
    {
        let target_b = plain_digimon("TARGET-IHF", 8);
        let mut r = DebugRunner::builder()
            .add_card(target_b)
            .add_card(inherit_src)
            .add_card(top_src)
            .hand(0, &["TARGET-IHF"])
            .memory(10)
            .start();

        r.register_effect("INHERIT-SRC", Arc::new(InheritedBeforePayCost));
        r.register_effect("TOP-SRC", Arc::new(TopCardBeforePayCost));

        // Swapped: position 0 (under) = TOP-SRC (inherited=false → should NOT fire from under)
        //          position 1 (top)   = INHERIT-SRC (inherited=true → should NOT fire from top)
        {
            let game = r.game_mut();
            let inh_idx = game
                .card_data
                .iter()
                .position(|c| c.card_id == "INHERIT-SRC")
                .unwrap();
            let top_idx = game
                .card_data
                .iter()
                .position(|c| c.card_id == "TOP-SRC")
                .unwrap();
            let ci0 = game.next_card_index();
            let ci1 = game.next_card_index();
            let under = CardSource::new(top_idx, 0, ci0); // TOP-SRC at under position
            let top = CardSource::new(inh_idx, 0, ci1); // INHERIT-SRC at top position
            let turn = game.turn_count;
            let mut perm = Permanent::new(under, turn);
            perm.card_sources.push(top);
            game.players[0].battle_area.push(perm);
        }

        let memory_before = r.memory();
        r.play(0, 0);
        // Neither effect fires: reduction = 0; effective cost = 8
        assert_eq!(
            r.memory(),
            memory_before - 8,
            "Part B: swapped positions; inherited filter blocks both effects; effective cost = 8"
        );
    }
}
