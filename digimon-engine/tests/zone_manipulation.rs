//! Phase 2 zone-manipulation integration tests.
//!
//! See docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta};
use std::sync::Arc;

/// Helper: a Lv.3 Red Digimon with configurable play_cost and no effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn play_from_hand_free_ignores_printed_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("COSTLY", "Costly", 10))
        .hand(0, &["COSTLY"])
        .memory(0)
        .start();

    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);

    let result = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Free);

    assert_eq!(result, Some(0), "play should succeed at free cost");
    assert_eq!(r.hand_size(0), 0, "card leaves hand");
    assert_eq!(r.battle_area_size(0), 1, "card enters battle area");
    assert_eq!(r.memory(), 0, "memory unchanged — CostDelta::Free pays 0");
}

#[test]
fn play_from_hand_reduce_subtracts_from_cost() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C6", "SixCost", 6))
        .hand(0, &["C6"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(4));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before - 2, "6 - 4 = 2 memory paid");
}

#[test]
fn play_from_hand_reduce_clamps_at_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C3", "ThreeCost", 3))
        .hand(0, &["C3"])
        .memory(5)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Reduce(10));
    assert_eq!(res, Some(0));
    assert_eq!(r.memory(), before, "reducing below 0 pays 0, not negative");
}

#[test]
fn play_from_hand_fixed_pays_exactly() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C10", "TenCost", 10))
        .hand(0, &["C10"])
        .memory(0)
        .start();

    let before = r.memory();
    let res = r.game_mut().play_from_hand_with_cost(0, 0, CostDelta::Fixed(5));
    assert_eq!(res, Some(0), "fixed cost 5 at memory 0 is affordable (goes to -5)");
    assert_eq!(r.memory(), before - 5, "exactly 5 memory paid");
}

// ─── Script-driven test: EffectContext::play_from_hand_with_cost ─────────────

/// TEST-P2-001: on play, if hand slot 0 has a card, play it free via
/// EffectContext::play_from_hand_with_cost.
struct TestP2_001;
impl CardEffect for TestP2_001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play top of hand free")
            .process(|ctx| {
                let me = ctx.player;
                if ctx.hand(me).is_empty() {
                    return;
                }
                ctx.play_from_hand_with_cost(me, 0, CostDelta::Free);
            })
            .build()]
    }
}

#[test]
fn ctx_play_from_hand_free_plays_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TEST-P2-001", "P2-001", 3))
        .add_card(plain_digimon("TARGET", "Target", 10))
        .hand(0, &["TEST-P2-001", "TARGET"])
        .memory(3)
        .start();

    r.register_effect("TEST-P2-001", Arc::new(TestP2_001));

    // Play TEST-P2-001 (hand slot 0). After OnPlay fires, it should have
    // played TARGET (now hand slot 0 since TEST-P2-001 was removed first)
    // for free.
    let res = r.play(0, 0);
    assert_eq!(res, Some(0));
    assert_eq!(r.battle_area_size(0), 2, "both cards entered battle area");
    assert_eq!(r.hand_size(0), 0, "hand emptied");
    // Memory: started 3, paid 3 for TEST-P2-001, then 0 for TARGET (free).
    assert_eq!(r.memory(), 0);
}

// ─── play_from_trash_with_cost ────────────────────────────────────────────────

#[test]
fn play_from_trash_free_moves_card_to_field() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BURIED", "Buried", 6))
        .memory(0)
        .start();

    // Seed trash directly using the same CardSource idiom as place_on_field.
    {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BURIED")
            .expect("BURIED not in card_data");
        let next_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
        g.players[0].trash.push(card);
    }

    assert_eq!(r.trash_size(0), 1);
    let res = r
        .game_mut()
        .play_from_trash_with_cost(0, 0, CostDelta::Free);
    assert_eq!(res, Some(0));
    assert_eq!(r.trash_size(0), 0, "card left trash");
    assert_eq!(r.battle_area_size(0), 1, "card entered battle area");
}

// ─── add_to_hand_from_deck / add_to_hand_from_trash / shuffle_deck ────────────

#[test]
fn add_to_hand_from_deck_moves_specific_card() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("WANTED", "Wanted", 4))
        .add_card(plain_digimon("FILLER", "Filler", 4))
        .deck(0, &["FILLER", "WANTED", "FILLER"])
        .start();

    // Grab the CardHandle of the WANTED card (deck slot 1).
    let target_handle = r.game_mut().player(0).deck[1].handle();

    let ok = r.game_mut().add_to_hand_from_deck(0, target_handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.deck_size(0), 2, "one card left deck");

    // Confirm the correct card moved.
    let moved_id = {
        let g = r.game_mut();
        g.player(0).hand[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(moved_id, "WANTED");
}

#[test]
fn add_to_hand_from_trash_moves_card() {
    // Seed trash the same way Task 4's test does — mirror that idiom.
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .start();

    let handle = {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "DEAD")
            .expect("DEAD not in card_data");
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        let h = card.handle();
        g.players[0].trash.push(card);
        h
    };

    let ok = r.game_mut().add_to_hand_from_trash(0, handle);
    assert!(ok);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(0), 0);
}

#[test]
fn add_to_hand_missing_handle_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DEAD", "Dead", 5))
        .deck(0, &["DEAD"])
        .start();

    // CardHandle(u16::MAX) is guaranteed to never be assigned by the builder
    // (the builder starts card indices at 0 and counts up; u16::MAX is sentinel).
    let bogus_handle = CardHandle(u16::MAX);

    let ok = r.game_mut().add_to_hand_from_deck(0, bogus_handle);
    assert!(!ok);
    assert_eq!(r.hand_size(0), 0);
}

// ─── reveal_top_deck ──────────────────────────────────────────────────────────

#[test]
fn reveal_top_deck_populates_reveal_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .add_card(plain_digimon("C", "C", 1))
        .deck(0, &["A", "B", "C"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 2);
    assert_eq!(revealed.len(), 2);
    assert_eq!(r.deck_size(0), 1);
    assert_eq!(r.game_mut().revealed_cards.len(), 2);

    // Pop() returns the top of Vec, so reveal order is the reverse of
    // insertion order. deck was built [A, B, C] with C on top. First
    // reveal should be C, second B.
    let card_ids: Vec<String> = {
        let g = r.game_mut();
        g.revealed_cards
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect()
    };
    assert_eq!(card_ids, vec!["C".to_string(), "B".to_string()], "revealed top-first");
}

#[test]
fn reveal_top_deck_handles_empty_deck() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .deck(0, &["A"])
        .start();
    let revealed = r.game_mut().reveal_top_deck(0, 5);
    assert_eq!(revealed.len(), 1);
    assert_eq!(r.deck_size(0), 0);
}

// ─── return_to_hand ───────────────────────────────────────────────────────────

#[test]
fn return_to_hand_moves_top_card_to_hand_and_sources_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP", "Top", 5))
        .add_card(plain_digimon("UNDER", "Under", 3))
        .start();

    // Seed a permanent with UNDER on bottom, TOP on top.
    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_data = g.card_data.iter().position(|c| c.card_id == "UNDER").unwrap();
        let top_data = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let idx_under = g.next_card_index();
        let idx_top = g.next_card_index();
        let under = digimon_engine::card_source::CardSource::new(under_data, 0, idx_under);
        let top = digimon_engine::card_source::CardSource::new(top_data, 0, idx_top);
        let mut perm = digimon_engine::permanent::Permanent::new(under, turn);
        perm.card_sources.push(top);
        g.players[0].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle { player: 0, index: 0 }
    };

    let returned = r.game_mut().return_to_hand(handle);
    assert!(returned.is_some(), "returned a card handle");
    assert_eq!(r.battle_area_size(0), 0, "permanent gone");
    assert_eq!(r.hand_size(0), 1, "top card went to hand");
    assert_eq!(r.trash_size(0), 1, "under card went to trash");

    let hand_id = {
        let g = r.game_mut();
        g.player(0).hand[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(hand_id, "TOP");
    let trash_id = {
        let g = r.game_mut();
        g.player(0).trash[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(trash_id, "UNDER");
}

#[test]
fn return_to_hand_bad_handle_returns_none() {
    let mut r = DebugRunner::builder().start();
    let returned = r.game_mut().return_to_hand(
        digimon_engine::permanent::PermanentHandle { player: 0, index: 99 }
    );
    assert!(returned.is_none());
}

// ─── return_to_deck ───────────────────────────────────────────────────────────

use digimon_engine::enums::StackPosition;
use digimon_engine::permanent::PermanentHandle;

fn seed_single_card_permanent(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g.card_data.iter().position(|c| c.card_id == card_id).unwrap();
    let card_idx = g.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
    g.players[0].battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
    PermanentHandle { player: 0, index: 0 }
}

#[test]
fn return_to_deck_top_places_on_top() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER", "FILLER"])
        .start();

    let handle = seed_single_card_permanent(&mut r, "TOP");
    let ok = r.game_mut().return_to_deck(handle, StackPosition::Top);
    assert!(ok);
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.deck_size(0), 3);
    let top_id = {
        let g = r.game_mut();
        g.player(0).deck.last().unwrap().card_id(&g.card_data).to_string()
    };
    assert_eq!(top_id, "TOP");
}

#[test]
fn return_to_deck_bottom_places_at_position_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOTTOM", "Bot", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER", "FILLER"])
        .start();

    let handle = seed_single_card_permanent(&mut r, "BOTTOM");
    let ok = r.game_mut().return_to_deck(handle, StackPosition::Bottom);
    assert!(ok);
    let bottom_id = {
        let g = r.game_mut();
        g.player(0).deck.first().unwrap().card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "BOTTOM");
}

#[test]
fn return_to_deck_random_inserts_somewhere() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("RANDOM", "R", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
        .start();

    let handle = seed_single_card_permanent(&mut r, "RANDOM");
    let ok = r.game_mut().return_to_deck(handle, StackPosition::Random);
    assert!(ok);
    assert_eq!(r.deck_size(0), 6);
    let positions: Vec<usize> = {
        let g = r.game_mut();
        g.player(0)
            .deck
            .iter()
            .enumerate()
            .filter(|(_, c)| c.card_id(&g.card_data) == "RANDOM")
            .map(|(i, _)| i)
            .collect()
    };
    assert_eq!(positions.len(), 1, "exactly one copy in deck");
}

#[test]
fn return_to_deck_bad_handle_returns_false() {
    let mut r = DebugRunner::builder().start();
    let ok = r.game_mut().return_to_deck(
        PermanentHandle { player: 0, index: 99 },
        StackPosition::Top,
    );
    assert!(!ok);
}

// ─── trash_from_hand_by_index ─────────────────────────────────────────────────

#[test]
fn trash_from_hand_by_index_moves_card_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("DOOMED", "D", 3))
        .hand(0, &["DOOMED"])
        .start();

    let trashed = r.game_mut().trash_from_hand_by_index(0, 0);
    assert!(trashed.is_some());
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn trash_from_hand_bad_index_is_noop() {
    let mut r = DebugRunner::builder().start();
    assert!(r.game_mut().trash_from_hand_by_index(0, 10).is_none());
}

// ─── reveal-pool movers ───────────────────────────────────────────────────────

#[test]
fn add_to_hand_from_reveal_moves_and_shrinks_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .deck(0, &["A", "B"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 2);
    assert_eq!(revealed.len(), 2);

    let handle = revealed[0];
    let ok = r.game_mut().add_to_hand_from_reveal(0, handle);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 1);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn trash_from_reveal_moves_and_shrinks_pool() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .deck(0, &["A"])
        .start();

    let revealed = r.game_mut().reveal_top_deck(0, 1);
    let h = revealed[0];
    let ok = r.game_mut().trash_from_reveal(0, h);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 0);
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn return_to_deck_from_reveal_top_puts_card_on_top() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A", "A", 1))
        .add_card(plain_digimon("B", "B", 1))
        .deck(0, &["A", "B"])
        .start();

    // reveal top 2 (B then A pops out in top-first order)
    let revealed = r.game_mut().reveal_top_deck(0, 2);
    let first = revealed[0];

    let ok = r.game_mut().return_to_deck_from_reveal(0, first, StackPosition::Top);
    assert!(ok);
    // Card back on top of now-1-card deck.
    assert_eq!(r.deck_size(0), 1);
    assert_eq!(r.game_mut().revealed_cards.len(), 1);
}

#[test]
fn reveal_mover_missing_handle_is_noop() {
    let mut r = DebugRunner::builder().start();
    let bogus = digimon_engine::card_source::CardHandle(u16::MAX);
    assert!(!r.game_mut().add_to_hand_from_reveal(0, bogus));
    assert!(!r.game_mut().trash_from_reveal(0, bogus));
    assert!(!r.game_mut().return_to_deck_from_reveal(0, bogus, StackPosition::Top));
}

// ─── place_as_bottom_source ───────────────────────────────────────────────────

use digimon_engine::enums::CardSourceRef;

fn seed_single_card_permanent_with_id(
    r: &mut DebugRunner,
    card_id: &str,
) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g.card_data.iter().position(|c| c.card_id == card_id).unwrap();
    let card_idx = g.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
    g.players[0].battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
    PermanentHandle { player: 0, index: 0 }
}

#[test]
fn place_as_bottom_source_from_hand_stacks_under_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("FUEL", "Fuel", 2))
        .hand(0, &["FUEL"])
        .start();

    let target = seed_single_card_permanent_with_id(&mut r, "BASE");

    let ok = r.game_mut().place_as_bottom_source(CardSourceRef::Hand(0, 0), target);
    assert!(ok);
    assert_eq!(r.hand_size(0), 0);

    let (bottom_id, top_id) = {
        let g = r.game_mut();
        let perm = &g.player(0).battle_area[0];
        assert_eq!(perm.card_sources.len(), 2);
        (
            perm.card_sources[0].card_id(&g.card_data).to_string(),
            perm.card_sources[1].card_id(&g.card_data).to_string(),
        )
    };
    assert_eq!(bottom_id, "FUEL");
    assert_eq!(top_id, "BASE");
}

#[test]
fn place_as_bottom_source_from_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("DEAD", "Dead", 2))
        .start();

    let target = seed_single_card_permanent_with_id(&mut r, "BASE");

    // Seed trash
    {
        let g = r.game_mut();
        let data_idx = g.card_data.iter().position(|c| c.card_id == "DEAD").unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.player_mut(0).trash.push(card);
    }
    assert_eq!(r.trash_size(0), 1);

    let ok = r.game_mut().place_as_bottom_source(CardSourceRef::Trash(0, 0), target);
    assert!(ok);
    assert_eq!(r.trash_size(0), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "DEAD");
}

#[test]
fn place_as_bottom_source_from_deck_top() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("TOP", "DeckTop", 2))
        .deck(0, &["TOP"])
        .start();

    let target = seed_single_card_permanent_with_id(&mut r, "BASE");
    assert_eq!(r.deck_size(0), 1);

    let ok = r.game_mut().place_as_bottom_source(CardSourceRef::DeckTop(0), target);
    assert!(ok);
    assert_eq!(r.deck_size(0), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "TOP");
}

#[test]
fn place_as_bottom_source_from_reveal() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("RV", "Rev", 2))
        .deck(0, &["RV"])
        .start();

    let target = seed_single_card_permanent_with_id(&mut r, "BASE");
    let revealed = r.game_mut().reveal_top_deck(0, 1);
    let handle = revealed[0];

    let ok = r.game_mut().place_as_bottom_source(CardSourceRef::Reveal(handle), target);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "RV");
}

#[test]
fn place_as_bottom_source_bad_source_index_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .start();
    let target = seed_single_card_permanent_with_id(&mut r, "BASE");

    assert!(!r.game_mut().place_as_bottom_source(CardSourceRef::Hand(0, 99), target));
    assert!(!r.game_mut().place_as_bottom_source(CardSourceRef::Trash(0, 99), target));
    assert!(!r.game_mut().place_as_bottom_source(CardSourceRef::DeckTop(0), target)); // empty deck
}

// ─── effect_initiated_digivolve ───────────────────────────────────────────────

/// Build a plain Digimon with a specific level and evo_costs, for digivolve tests.
fn digimon_with_evo_costs(
    card_id: &str,
    name: &str,
    level: u8,
    evo_costs: Vec<digimon_engine::card_data::EvoCost>,
) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(level),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs,
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn effect_initiated_digivolve_places_card_on_target_for_free() {
    // BASE Lv.3 Red on field, EVO Lv.4 with evo_cost (from Lv3, Red=0, cost=2) in hand.
    let base = plain_digimon("BASE3", "Base3", 3);
    let evo = digimon_with_evo_costs(
        "EVO4",
        "Evo4",
        4,
        vec![digimon_engine::card_data::EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: 2,
        }],
    );

    let mut r = DebugRunner::builder()
        .add_card(base.clone())
        .add_card(evo.clone())
        .hand(0, &["EVO4"])
        .memory(0)
        .start();

    // Seed BASE3 on the field directly.
    let target = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g.card_data.iter().position(|c| c.card_id == "BASE3").unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.players[0].battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let memory_before = r.memory();
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // hand_index of EVO4
        target,
        CostDelta::Free,
        false,
    );
    assert!(ok, "digivolve should succeed");
    assert_eq!(r.hand_size(0), 0, "EVO4 left hand");
    assert_eq!(r.battle_area_size(0), 1, "stack grew, didn't split");

    let stack_size = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources.len()
    };
    assert_eq!(stack_size, 2, "EVO4 stacked on top of BASE3");
    assert_eq!(r.memory(), memory_before, "CostDelta::Free paid 0");
}

#[test]
fn effect_initiated_digivolve_ignore_color_bypasses_color_check() {
    // BASE is Red (card_color=0), EVO requires Blue (card_color=1).
    // ignore_color=false should fail, ignore_color=true should succeed.
    let base = plain_digimon("B3", "Base3", 3); // Red by default
    let evo = digimon_with_evo_costs(
        "E4",
        "Evo4",
        4,
        vec![digimon_engine::card_data::EvoCost {
            card_color: 1, // Blue
            level: 3,
            memory_cost: 0,
        }],
    );

    let mut r = DebugRunner::builder()
        .add_card(base.clone())
        .add_card(evo.clone())
        .hand(0, &["E4"])
        .memory(5)
        .start();

    let target = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g.card_data.iter().position(|c| c.card_id == "B3").unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.players[0].battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    // With ignore_color = false, should fail (Red base vs Blue evo cost).
    let ok_strict = r.game_mut().effect_initiated_digivolve(
        0, 0, target, CostDelta::Free, false,
    );
    assert!(!ok_strict, "color mismatch should block without ignore_color");
    assert_eq!(r.hand_size(0), 1, "hand untouched after failure");

    // With ignore_color = true, should succeed.
    let ok_loose = r.game_mut().effect_initiated_digivolve(
        0, 0, target, CostDelta::Free, true,
    );
    assert!(ok_loose, "ignore_color bypasses color check");
    assert_eq!(r.hand_size(0), 0, "EVO moved to stack");
}

#[test]
fn effect_initiated_digivolve_bad_level_returns_false() {
    // EVO requires Lv.5, target is Lv.3 → no matching evo_cost.
    let base = plain_digimon("B3", "Base3", 3);
    let evo = digimon_with_evo_costs(
        "E4",
        "Evo4",
        4,
        vec![digimon_engine::card_data::EvoCost {
            card_color: 0, // Red
            level: 5,      // requires Lv.5, but target is Lv.3
            memory_cost: 0,
        }],
    );

    let mut r = DebugRunner::builder()
        .add_card(base.clone())
        .add_card(evo.clone())
        .hand(0, &["E4"])
        .memory(5)
        .start();

    let target = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g.card_data.iter().position(|c| c.card_id == "B3").unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.players[0].battle_area.push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle { player: 0, index: 0 }
    };

    let ok = r.game_mut().effect_initiated_digivolve(
        0, 0, target, CostDelta::Free, true,
    );
    assert!(!ok, "level mismatch should return false even with ignore_color=true");
    assert_eq!(r.hand_size(0), 1, "hand untouched after failure");
}

// ─── EffectContext::hatch ─────────────────────────────────────────────────────

/// TEST-P2-Hatch: on play, hatch the controller's top digitama into
/// the breeding area via EffectContext::hatch.
struct TestP2Hatch;
impl CardEffect for TestP2Hatch {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Hatch")
            .process(|ctx| {
                let me = ctx.player;
                ctx.hatch(me);
            })
            .build()]
    }
}

#[test]
fn ctx_hatch_moves_top_of_digitama_to_breeding() {
    let mut egg = plain_digimon("EGG", "Egg", 0);
    egg.level = Some(2);
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("HATCHER", "Hatch", 3))
        .add_card(egg)
        .hand(0, &["HATCHER"])
        .digitama(0, &["EGG"])
        .memory(3)
        .start();

    r.register_effect("HATCHER", Arc::new(TestP2Hatch));

    assert!(r.game_mut().player(0).breeding_area.is_none());
    let played = r.play(0, 0);
    assert_eq!(played, Some(0));
    assert!(
        r.game_mut().player(0).breeding_area.is_some(),
        "egg hatched into breeding area"
    );
}

// ─── place_on_security ────────────────────────────────────────────────────────

#[test]
fn place_on_security_from_hand_grows_security_stack() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SHIELD", "Shield", 4))
        .hand(0, &["SHIELD"])
        .start();

    let before = r.security_count(0);
    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Top,
        /* face_up = */ false,
    );
    assert!(ok);
    assert_eq!(r.security_count(0), before + 1);
    assert_eq!(r.hand_size(0), 0);
}

#[test]
fn place_on_security_face_up_marks_card_visible() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("VIS", "Visible", 4))
        .hand(0, &["VIS"])
        .start();

    // Capture the card's face_up_security key BEFORE moving it.
    // face_up_security is keyed by card_index (u16) from CardSource.
    let card_key = {
        let g = r.game_mut();
        g.player(0).hand[0].card_index
    };

    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Top,
        /* face_up = */ true,
    );
    assert!(ok);
    assert!(r.game_mut().player(0).face_up_security.contains(&card_key));
}

#[test]
fn place_on_security_bottom_places_at_index_zero() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOT", "Bottom", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .hand(0, &["BOT"])
        .security(0, &["FILLER", "FILLER"])
        .start();

    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 0),
        StackPosition::Bottom,
        false,
    );
    assert!(ok);
    // security is Vec; bottom = index 0
    let bottom_id = {
        let g = r.game_mut();
        g.player(0).security.first().unwrap().card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "BOT");
}

#[test]
fn place_on_security_bad_source_returns_false() {
    let mut r = DebugRunner::builder().start();
    let ok = r.game_mut().place_on_security(
        0,
        CardSourceRef::Hand(0, 99),
        StackPosition::Top,
        false,
    );
    assert!(!ok);
}
