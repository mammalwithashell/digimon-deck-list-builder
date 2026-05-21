//! `EffectContext::trash_card_source` — unit tests.
//!
//! The primitive removes a specific `CardSource` from anywhere in a
//! permanent's `card_sources` stack (not just the top) and pushes it to the
//! controller's trash.
//!
//! Used by:
//! - `<Fragment (N)>` keyword auto-install (Phase D Task 4)
//! - `<Partition>` keyword auto-install (Phase D Task 9)

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
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

/// Helper: push a card onto an existing permanent's stack (makes it the new top).
fn push_source_on_top(
    r: &mut DebugRunner,
    perm_player: u8,
    perm_idx: usize,
    card_id: &str,
) -> digimon_engine::card_source::CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_on_top: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, perm_player, card_index);
    let handle = card.handle();
    r.game.players[perm_player as usize].battle_area[perm_idx]
        .card_sources
        .push(card);
    handle
}

// ── Test 1: Trash a mid-stack source ─────────────────────────────────────────

/// Stack: [BASE, MID-SOURCE, TOP].
/// Call trash_card_source(perm, MID-SOURCE).
/// Expected: stack = [BASE, TOP], MID-SOURCE in trash.
#[test]
fn trash_card_source_removes_mid_stack_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("MID-SOURCE"))
        .add_card(make_digimon("TOP"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;

    let mid_handle = push_source_on_top(&mut r, tp, perm_idx, "MID-SOURCE");
    push_source_on_top(&mut r, tp, perm_idx, "TOP");

    // Pre-condition: 3-card stack, no trash.
    {
        let perm = &r.game.players[tp as usize].battle_area[perm_idx];
        assert_eq!(perm.card_sources.len(), 3, "3-card stack before");
    }
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash empty before"
    );

    // Apply trash_card_source targeting MID-SOURCE.
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.trash_card_source(perm_handle, mid_handle);
    }

    // Stack should now be [BASE, TOP] (MID-SOURCE removed from middle).
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 2, "stack shrunk to 2");
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "BASE",
        "BASE remains at bottom"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TOP",
        "TOP remains as top"
    );

    // MID-SOURCE is in trash.
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "one card in trash");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "MID-SOURCE",
        "MID-SOURCE went to trash"
    );
}

// ── Test 2: Trash the bottom source ──────────────────────────────────────────

/// Stack: [BASE, TOP].
/// Call trash_card_source(perm, BASE).
/// Expected: stack = [TOP], BASE in trash. The permanent is now a single-card stack.
#[test]
fn trash_card_source_removes_bottom_card() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("TOP"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE", Some(0));
    let perm_idx = perm_handle.index as usize;

    // Capture BASE handle before pushing TOP.
    let base_handle = r.game.players[tp as usize].battle_area[perm_idx].card_sources[0].handle();

    push_source_on_top(&mut r, tp, perm_idx, "TOP");

    // Pre-condition: 2-card stack.
    assert_eq!(
        r.game.players[tp as usize].battle_area[perm_idx]
            .card_sources
            .len(),
        2
    );

    // Trash BASE (bottom of stack).
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.trash_card_source(perm_handle, base_handle);
    }

    // Stack = [TOP] only.
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 1, "single card remains");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TOP",
        "TOP is now the only (top) card"
    );

    // BASE in trash.
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "one card in trash");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "BASE",
        "BASE went to trash"
    );
}
