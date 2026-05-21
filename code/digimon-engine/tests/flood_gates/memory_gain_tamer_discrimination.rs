//! Tests for `source_is_tamer` helper and `CannotGainMemoryExceptFromTamers`
//! flood-gate discrimination.
//!
//! `source_is_tamer` is mirrored on both `EffectContext` and `EffectReadContext`
//! so condition closures can use it for Tamer-source discrimination, matching
//! DCGO's `ICardEffect.IsTamerEffect` property.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{EffectContext, EffectReadContext};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn make_option(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

// ─── source_is_tamer on EffectContext ─────────────────────────────────────

/// When the source_card is a Tamer in a field permanent, source_is_tamer()
/// must return true.
#[test]
fn source_is_tamer_helper_returns_true_for_tamer_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TMR-A"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();

    // Place a Tamer on the field and use its card handle as the source.
    let h = r.place_on_field(tp, "TMR-A", None);
    let source_handle = r.game.player(tp).battle_area[h.index as usize]
        .top_card()
        .handle();

    let ctx = EffectContext::new(&mut r.game, source_handle, Some(h), tp);
    assert!(
        ctx.source_is_tamer(),
        "source_is_tamer must return true when source card is a Tamer"
    );
}

/// source_is_tamer() returns false when the source card is a Digimon.
#[test]
fn source_is_tamer_helper_returns_false_for_digimon_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("DIG-A"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    let h = r.place_on_field(tp, "DIG-A", None);
    let source_handle = r.game.player(tp).battle_area[h.index as usize]
        .top_card()
        .handle();

    let ctx = EffectContext::new(&mut r.game, source_handle, Some(h), tp);
    assert!(
        !ctx.source_is_tamer(),
        "source_is_tamer must return false when source card is a Digimon"
    );
}

/// source_is_tamer() returns false when the source card is an Option.
#[test]
fn source_is_tamer_helper_returns_false_for_option_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-A"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();

    // Options don't go on the field normally, but card_kind_for_handle scans
    // hand and trash too. Put the Option in hand so the scanner can find it.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPT-A")
        .unwrap();
    let next = r.game.next_card_index();
    let option_card = CardSource::new(data_idx, tp, next);
    let source_handle = option_card.handle();
    r.game.players[tp as usize].hand.push(option_card);

    let ctx = EffectContext::new(&mut r.game, source_handle, None, tp);
    assert!(
        !ctx.source_is_tamer(),
        "source_is_tamer must return false when source card is an Option"
    );
}

// ─── source_is_tamer mirrored on EffectReadContext ────────────────────────

/// EffectReadContext must expose source_is_tamer with identical semantics
/// to EffectContext's version.
#[test]
fn source_is_tamer_helper_mirrored_on_effect_read_context() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TMR-B"))
        .add_card(make_digimon("DIG-B"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();

    let tamer_h = r.place_on_field(tp, "TMR-B", None);
    let tamer_src = r.game.player(tp).battle_area[tamer_h.index as usize]
        .top_card()
        .handle();

    let dig_h = r.place_on_field(tp, "DIG-B", None);
    let dig_src = r.game.player(tp).battle_area[dig_h.index as usize]
        .top_card()
        .handle();

    // EffectReadContext for Tamer source.
    let read_tamer = EffectReadContext::new(&r.game, tamer_src, Some(tamer_h), tp);
    assert!(
        read_tamer.source_is_tamer(),
        "EffectReadContext::source_is_tamer must return true for Tamer source"
    );

    // EffectReadContext for Digimon source.
    let read_dig = EffectReadContext::new(&r.game, dig_src, Some(dig_h), tp);
    assert!(
        !read_dig.source_is_tamer(),
        "EffectReadContext::source_is_tamer must return false for Digimon source"
    );
}

// ─── CannotGainMemoryExceptFromTamers ─────────────────────────────────────

/// When CannotGainMemoryExceptFromTamers is installed on player 0,
/// ctx.gain_memory(2) with a Tamer source MUST succeed.
#[test]
fn cannot_gain_memory_except_from_tamers_allows_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TMR-C"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    r.game.set_memory(0);

    // Place Tamer on field so card_kind_for_handle can resolve it.
    let h = r.place_on_field(tp, "TMR-C", None);
    let tamer_src = r.game.player(tp).battle_area[h.index as usize]
        .top_card()
        .handle();

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryExceptFromTamers,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, tamer_src, Some(h), tp);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, 2,
        "Tamer-sourced gain_memory must succeed under CannotGainMemoryExceptFromTamers"
    );
}

/// When CannotGainMemoryExceptFromTamers is installed on player 0,
/// ctx.gain_memory(2) with a Digimon source must be a no-op.
#[test]
fn cannot_gain_memory_except_from_tamers_blocks_digimon_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("DIG-C"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    r.game.set_memory(0);

    let h = r.place_on_field(tp, "DIG-C", None);
    let dig_src = r.game.player(tp).battle_area[h.index as usize]
        .top_card()
        .handle();

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryExceptFromTamers,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, dig_src, Some(h), tp);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, 0,
        "Digimon-sourced gain_memory must be blocked by CannotGainMemoryExceptFromTamers"
    );
}

/// When CannotGainMemoryExceptFromTamers is installed on player 0,
/// ctx.gain_memory(2) with an Option source must be a no-op.
#[test]
fn cannot_gain_memory_except_from_tamers_blocks_option_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-B"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    r.game.set_memory(0);

    // Put Option in hand so the scanner finds it.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPT-B")
        .unwrap();
    let next = r.game.next_card_index();
    let option_card = CardSource::new(data_idx, tp, next);
    let opt_src = option_card.handle();
    r.game.players[tp as usize].hand.push(option_card);

    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry::simple(
            ModifierType::CannotGainMemoryExceptFromTamers,
            0,
            Expiry::Permanent,
            None,
            1 - tp,
        ),
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, opt_src, None, tp);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, 0,
        "Option-sourced gain_memory must be blocked by CannotGainMemoryExceptFromTamers"
    );
}

/// CannotGainMemoryByEffect takes priority over CannotGainMemoryExceptFromTamers —
/// even a Tamer source is blocked when the absolute blocker is active.
#[test]
fn cannot_gain_memory_by_effect_blocks_even_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TMR-D"))
        .add_card(make_filler())
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .start();

    let tp = r.game.turn_player();
    r.game.set_memory(0);

    let h = r.place_on_field(tp, "TMR-D", None);
    let tamer_src = r.game.player(tp).battle_area[h.index as usize]
        .top_card()
        .handle();

    // Install the absolute blocker (not the Tamer-exception one).
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

    {
        let mut ctx = EffectContext::new(&mut r.game, tamer_src, Some(h), tp);
        ctx.gain_memory(2);
    }

    assert_eq!(
        r.game.memory, 0,
        "CannotGainMemoryByEffect must block even Tamer-sourced gains"
    );
}
