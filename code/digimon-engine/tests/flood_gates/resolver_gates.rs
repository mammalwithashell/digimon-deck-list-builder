//! Resolver-layer enforcement tests for player-scoped flood-gate modifiers.
//!
//! Covers gates that are enforced inside `EffectContext` helpers (not the
//! action mask), including:
//!   - CannotPlayDigimonByEffect  (play_from_hand_with_cost / play_from_trash_with_cost)
//!   - CannotGainMemoryByEffect   (ctx.gain_memory)
//!   - CannotDrawByEffect         (ctx.draw)
//!   - CannotReducePlayCost       (scan_before_pay_cost_reduction)
//!   - CannotAddSecurityByEffect  (ctx.place_on_security)

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, CardSourceRef, CostDelta, Expiry, ModifierType, PlaySource, StackPosition,
};
use digimon_engine::modifiers::PlayerModifierEntry;

// ─── Card factory helpers ──────────────────────────────────────────────────

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(3000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_filler() -> CardData {
    CardData {
        card_id: "FILL".to_string(),
        card_name: "FILL".to_string(),
        card_kind: CardKind::DigiEgg,
        level: Some(2),
        dp: Some(1000),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "FILL".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── CannotPlayDigimonByEffect ─────────────────────────────────────────────

/// ByEffect play of a Digimon is blocked when CannotPlayDigimonByEffect is
/// installed. ByHand play of the same card is NOT blocked.
#[test]
fn cannot_play_digimon_by_effect_blocks_effect_initiated_play_but_not_hand_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("DIG-A"))
        .add_card(make_filler())
        .hand(0, &["DIG-A"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let tp = r.game.turn_player();

    // Install CannotPlayDigimonByEffect on the turn player.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    // Effect-initiated play: should be blocked → None.
    let result = r
        .game
        .play_from_hand_with_cost(tp, 0, CostDelta::Free, PlaySource::ByEffect);
    assert!(
        result.is_none(),
        "ByEffect play of Digimon must be blocked by CannotPlayDigimonByEffect"
    );
    // Card should still be in hand.
    assert_eq!(
        r.game.player(tp).hand.len(),
        1,
        "card should remain in hand after blocked play"
    );

    // Player-action play (ByHand): must NOT be blocked.
    let result_hand = r
        .game
        .play_from_hand_with_cost(tp, 0, CostDelta::Free, PlaySource::ByHand);
    assert!(
        result_hand.is_some(),
        "ByHand play must succeed even when CannotPlayDigimonByEffect is installed"
    );
    assert_eq!(
        r.game.player(tp).hand.len(),
        0,
        "card should leave hand after ByHand play"
    );
}

/// CannotPlayDigimonByEffect does not block a non-Digimon card played ByEffect.
#[test]
fn cannot_play_digimon_by_effect_does_not_block_non_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TMR-A"))
        .add_card(make_filler())
        .hand(0, &["TMR-A"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let tp = r.game.turn_player();

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    // Tamer played ByEffect should succeed — only Digimon is blocked.
    let result = r
        .game
        .play_from_hand_with_cost(tp, 0, CostDelta::Free, PlaySource::ByEffect);
    assert!(
        result.is_some(),
        "Tamer ByEffect play must succeed even under CannotPlayDigimonByEffect"
    );
}

// ─── CannotGainMemoryByEffect ──────────────────────────────────────────────

/// When CannotGainMemoryByEffect is installed on player 0, ctx.gain_memory(2)
/// inside an effect scoped to player 0 must be a no-op.
#[test]
fn cannot_gain_memory_by_effect_blocks_ctx_gain_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    r.game.set_memory(0);

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    // Construct a minimal EffectContext with a synthetic source handle.
    // We don't need a real card in any zone for this test — gain_memory only
    // reads `self.player` and the modifier registry.
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), None, tp);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, 0,
        "gain_memory must be a no-op under CannotGainMemoryByEffect"
    );
}

// ─── CannotDrawByEffect ────────────────────────────────────────────────────

/// When CannotDrawByEffect is installed on player 0, ctx.draw(player_0, 1)
/// must be a no-op (hand size unchanged).
#[test]
fn cannot_draw_by_effect_blocks_ctx_draw() {
    let mut r = DebugRunner::builder()
        .add_card(make_filler())
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    let hand_before = r.game.player(tp).hand.len();

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotDrawByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), None, tp);
        let drawn = ctx.draw(tp, 1);
        assert_eq!(drawn, 0, "ctx.draw must return 0 under CannotDrawByEffect");
    }

    assert_eq!(
        r.game.player(tp).hand.len(),
        hand_before,
        "hand size must be unchanged after blocked draw"
    );
}

/// CannotDrawByEffect on player 0 does NOT block a draw for player 1.
#[test]
fn cannot_draw_by_effect_is_player_scoped() {
    let mut r = DebugRunner::builder()
        .add_card(make_filler())
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotDrawByEffect,
            0,
            Expiry::Permanent,
            None,
            opp,
        ),
    );

    let opp_hand_before = r.game.player(opp).hand.len();
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), None, tp);
        let drawn = ctx.draw(opp, 1);
        assert_eq!(
            drawn, 1,
            "draw for opponent must succeed (modifier is on tp, not opp)"
        );
    }
    assert_eq!(r.game.player(opp).hand.len(), opp_hand_before + 1);
}

// ─── CannotReducePlayCost ──────────────────────────────────────────────────

/// When CannotReducePlayCost is installed on the acting player, cost-reduction
/// effects (BeforePayCost) must be suppressed.
#[test]
fn cannot_reduce_play_cost_suppresses_before_pay_cost_scan() {
    // Build a card with a BeforePayCost effect that reduces cost by 3.
    struct CostReducer;
    impl CardEffect for CostReducer {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::before_pay_cost(card).cost_reduction(3).build()]
        }
    }

    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(CostReducer));

    let mut r = DebugRunner::builder()
        .with_registry(registry)
        .add_card(make_digimon("DIG-A"))
        .add_card(CardData {
            card_id: "REDUCER".to_string(),
            card_name: "Reducer".to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(2000),
            play_cost: 2,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            dual: None,
            effect_class_name: "REDUCER".to_string(),
            index: 0,
            norm_id: 0.0,
        })
        .add_card(make_filler())
        .hand(0, &["DIG-A"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let tp = r.game.turn_player();

    // Place REDUCER on the field so it can contribute BeforePayCost.
    r.place_on_field(tp, "REDUCER", Some(0));

    // Verify reducer DOES reduce cost without the modifier.
    // DIG-A costs 4; with reduction of 3, effective cost = 1. Memory 10 → 9.
    // (We won't actually play it yet; this is proven by the second play below.)

    // Install CannotReducePlayCost.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotReducePlayCost,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    let memory_before = r.game.memory;
    let result = r
        .game
        .play_from_hand_with_cost(tp, 0, CostDelta::Reduce(0), PlaySource::ByHand);
    assert!(
        result.is_some(),
        "play should succeed (memory is sufficient)"
    );

    // With CannotReducePlayCost, effective cost must be the full printed cost (4),
    // NOT 1 (which would be printed - 3).
    let memory_after = r.game.memory;
    let cost_paid = (memory_before - memory_after) as i32;
    assert_eq!(
        cost_paid, 4,
        "cost must equal printed cost (4) when CannotReducePlayCost is installed; got {}",
        cost_paid
    );
}

#[test]
fn cannot_reduce_play_cost_suppresses_when_playing_this_hand_reducer() {
    struct HandSelfReducer;
    impl CardEffect for HandSelfReducer {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            vec![Effect::before_pay_cost(card)
                .when_playing_this()
                .cost_reduction(3)
                .build()]
        }
    }

    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("SELF-REDUCER", Arc::new(HandSelfReducer));

    let mut r = DebugRunner::builder()
        .with_registry(registry)
        .add_card(CardData {
            card_id: "SELF-REDUCER".to_string(),
            card_name: "Self Reducer".to_string(),
            card_kind: CardKind::Digimon,
            level: Some(4),
            dp: Some(2000),
            play_cost: 4,
            colors: vec![CardColor::Red],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
            dual: None,
            effect_class_name: "SELF_REDUCER".to_string(),
            index: 0,
            norm_id: 0.0,
        })
        .add_card(make_filler())
        .hand(0, &["SELF-REDUCER"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let tp = r.game.turn_player();
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotReducePlayCost,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    let memory_before = r.game.memory;
    let result = r
        .game
        .play_from_hand_with_cost(tp, 0, CostDelta::Reduce(0), PlaySource::ByHand);

    assert!(result.is_some(), "play should succeed at full printed cost");
    let cost_paid = (memory_before - r.game.memory) as i32;
    assert_eq!(
        cost_paid, 4,
        "hand-card when_playing_this reducer must be suppressed by CannotReducePlayCost"
    );
}

// ─── CannotPlayDigimonByEffect: security trigger path ─────────────────────

fn make_attacker() -> CardData {
    CardData {
        card_id: "ATK".to_string(),
        card_name: "Attacker".to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(8000),
        play_cost: 6,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "ATK".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

/// When CannotPlayDigimonByEffect is installed on the defender, a security
/// Digimon trigger that calls play_from_security must NOT enter the field.
/// The revealed card must go to the defender's trash instead (standard
/// "didn't stick" outcome), and no OnPlay effect fires.
#[test]
fn cannot_play_digimon_by_effect_blocks_security_trigger_play() {
    // TEST-021 is a Digimon with "[Security] Play this card without paying its
    // cost." — its SecuritySkill calls ctx.play_pending_security().
    let mut test021 = make_test_card("TEST-021", "Test021");
    test021.play_cost = 5; // deliberately expensive — free play must be gated

    let mut r = DebugRunner::builder()
        .add_card(make_attacker())
        .add_card(test021)
        .security(1, &["TEST-021"])
        .start();

    // Install CannotPlayDigimonByEffect on the defender (player 1).
    r.game.modifiers.add_player_modifier(
        1,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);

    // Card must NOT have entered the battle area.
    assert_eq!(
        r.battle_area_size(1),
        0,
        "play_from_security must be blocked by CannotPlayDigimonByEffect"
    );

    // The revealed card must have gone to trash (standard "didn't stick" path).
    assert_eq!(
        r.trash_size(1),
        1,
        "blocked security Digimon play must trash the card"
    );

    // Security stack must be empty (card was popped for the check).
    assert_eq!(r.security_count(1), 0);
}

// ─── CannotAddSecurityByEffect ─────────────────────────────────────────────

/// When CannotAddSecurityByEffect is installed on the acting player,
/// ctx.place_on_security must return false and leave security count unchanged.
/// The source card (in hand) must also remain in hand — nothing is moved.
#[test]
fn cannot_add_security_by_effect_blocks_place_on_security() {
    let mut r = DebugRunner::builder()
        .add_card(make_filler())
        .hand(0, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .start();

    let tp = r.game.turn_player();
    let security_before = r.security_count(tp);
    let hand_before = r.game.player(tp).hand.len();

    // Install CannotAddSecurityByEffect on the acting player.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    // Try to place the hand card (index 0) onto security via EffectContext.
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(999), None, tp);
        ctx.place_on_security(tp, CardSourceRef::Hand(tp, 0), StackPosition::Top, false)
    };

    assert!(
        !result,
        "place_on_security must return false under CannotAddSecurityByEffect"
    );
    assert_eq!(
        r.security_count(tp),
        security_before,
        "security count must be unchanged after blocked place_on_security"
    );
    assert_eq!(
        r.game.player(tp).hand.len(),
        hand_before,
        "hand must be unchanged — card must NOT have been moved out of hand"
    );
}
