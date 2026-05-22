//! Phase 2 zone-manipulation integration tests.
//!
//! See docs/superpowers/plans/2026-04-19-rust-engine-phase-2-zone-manipulation.md.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, PlaySource};
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
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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

    let result = r
        .game_mut()
        .play_from_hand_with_cost(0, 0, CostDelta::Free, PlaySource::ByHand);

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
    let res = r
        .game_mut()
        .play_from_hand_with_cost(0, 0, CostDelta::Reduce(4), PlaySource::ByHand);
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
    let res =
        r.game_mut()
            .play_from_hand_with_cost(0, 0, CostDelta::Reduce(10), PlaySource::ByHand);
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
    let res = r
        .game_mut()
        .play_from_hand_with_cost(0, 0, CostDelta::Fixed(5), PlaySource::ByHand);
    assert_eq!(
        res,
        Some(0),
        "fixed cost 5 at memory 0 is affordable (goes to -5)"
    );
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
        .play_from_trash_with_cost(0, 0, CostDelta::Free, PlaySource::ByEffect);
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
    assert_eq!(
        card_ids,
        vec!["C".to_string(), "B".to_string()],
        "revealed top-first"
    );
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
        let under_data = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER")
            .unwrap();
        let top_data = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let idx_under = g.next_card_index();
        let idx_top = g.next_card_index();
        let under = digimon_engine::card_source::CardSource::new(under_data, 0, idx_under);
        let top = digimon_engine::card_source::CardSource::new(top_data, 0, idx_top);
        let mut perm = digimon_engine::permanent::Permanent::new(under, turn);
        perm.card_sources.push(top);
        g.players[0].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        }
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
    let returned = r
        .game_mut()
        .return_to_hand(digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 99,
        });
    assert!(returned.is_none());
}

// ─── return_to_deck ───────────────────────────────────────────────────────────

use digimon_engine::enums::StackPosition;
use digimon_engine::permanent::PermanentHandle;

fn seed_single_card_permanent(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_idx = g.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
    g.players[0]
        .battle_area
        .push(digimon_engine::permanent::Permanent::new(card, turn));
    PermanentHandle {
        player: 0,
        index: 0,
    }
}

fn seed_stacked_permanent(r: &mut DebugRunner, card_ids_bottom_to_top: &[&str]) -> PermanentHandle {
    assert!(
        !card_ids_bottom_to_top.is_empty(),
        "test helper needs at least one source"
    );

    let g = r.game_mut();
    let turn = g.turn_count;
    let mut sources = Vec::new();
    for card_id in card_ids_bottom_to_top {
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == *card_id)
            .unwrap();
        let card_idx = g.next_card_index();
        sources.push(digimon_engine::card_source::CardSource::new(
            data_idx, 0, card_idx,
        ));
    }

    let bottom = sources.remove(0);
    let mut perm = digimon_engine::permanent::Permanent::new(bottom, turn);
    perm.card_sources.extend(sources);
    g.players[0].battle_area.push(perm);
    PermanentHandle {
        player: 0,
        index: 0,
    }
}

fn seed_stacked_permanent_with_owners(
    r: &mut DebugRunner,
    cards_bottom_to_top: &[(&str, u8)],
) -> PermanentHandle {
    assert!(
        !cards_bottom_to_top.is_empty(),
        "test helper needs at least one source"
    );

    let g = r.game_mut();
    let turn = g.turn_count;
    let mut sources = Vec::new();
    for (card_id, owner) in cards_bottom_to_top {
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == *card_id)
            .unwrap();
        let card_idx = g.next_card_index();
        sources.push(digimon_engine::card_source::CardSource::new(
            data_idx, *owner, card_idx,
        ));
    }

    let bottom = sources.remove(0);
    let mut perm = digimon_engine::permanent::Permanent::new(bottom, turn);
    perm.card_sources.extend(sources);
    g.players[0].battle_area.push(perm);
    PermanentHandle {
        player: 0,
        index: 0,
    }
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
        g.player(0)
            .deck
            .last()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(top_id, "TOP");
}

#[test]
fn return_to_deck_top_moves_only_top_card_and_trashes_sources() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOTTOM", "Bottom", 4))
        .add_card(plain_digimon("MIDDLE", "Middle", 4))
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER"])
        .start();

    let handle = seed_stacked_permanent(&mut r, &["BOTTOM", "MIDDLE", "TOP"]);
    let ok = r.game_mut().return_to_deck(handle, StackPosition::Top);
    assert!(ok);
    assert_eq!(r.battle_area_size(0), 0, "permanent leaves battle area");
    assert_eq!(r.deck_size(0), 2, "only the top card joins the deck");
    assert_eq!(r.trash_size(0), 2, "lower sources are trashed");

    let (deck_ids, trash_ids): (Vec<String>, Vec<String>) = {
        let g = r.game_mut();
        (
            g.player(0)
                .deck
                .iter()
                .map(|c| c.card_id(&g.card_data).to_string())
                .collect(),
            g.player(0)
                .trash
                .iter()
                .map(|c| c.card_id(&g.card_data).to_string())
                .collect(),
        )
    };
    assert_eq!(
        deck_ids,
        vec!["FILLER".to_string(), "TOP".to_string()],
        "top card is placed on top of deck"
    );
    assert_eq!(
        trash_ids,
        vec!["BOTTOM".to_string(), "MIDDLE".to_string()],
        "sources are trashed bottom-to-top"
    );
}

#[test]
fn return_to_deck_include_sources_top_preserves_full_stack_order() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOTTOM", "Bottom", 4))
        .add_card(plain_digimon("MIDDLE", "Middle", 4))
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("FILLER", "F", 1))
        .deck(0, &["FILLER"])
        .start();

    let handle = seed_stacked_permanent(&mut r, &["BOTTOM", "MIDDLE", "TOP"]);
    let ok = r
        .game_mut()
        .return_stack_to_deck(handle, StackPosition::Top);
    assert!(ok);
    assert_eq!(r.battle_area_size(0), 0, "permanent leaves battle area");
    assert_eq!(r.trash_size(0), 0, "sources are not trashed");

    let deck: Vec<(String, u8)> = {
        let g = r.game_mut();
        g.player(0)
            .deck
            .iter()
            .map(|c| (c.card_id(&g.card_data).to_string(), c.owner))
            .collect()
    };
    assert_eq!(
        deck,
        vec![
            ("FILLER".to_string(), 0),
            ("BOTTOM".to_string(), 0),
            ("MIDDLE".to_string(), 0),
            ("TOP".to_string(), 0),
        ],
        "full stack is appended bottom-to-top so TOP remains deck top"
    );
}

#[test]
fn return_to_deck_include_sources_routes_each_card_to_owners_deck() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("P0-BOTTOM", "P0 Bottom", 4))
        .add_card(plain_digimon("P1-MIDDLE", "P1 Middle", 4))
        .add_card(plain_digimon("P0-TOP", "P0 Top", 4))
        .add_card(plain_digimon("FILLER0", "F0", 1))
        .add_card(plain_digimon("FILLER1", "F1", 1))
        .deck(0, &["FILLER0"])
        .deck(1, &["FILLER1"])
        .start();

    let handle = seed_stacked_permanent_with_owners(
        &mut r,
        &[("P0-BOTTOM", 0), ("P1-MIDDLE", 1), ("P0-TOP", 0)],
    );
    assert!(r
        .game_mut()
        .return_stack_to_deck(handle, StackPosition::Top));

    let (p0_deck, p1_deck): (Vec<String>, Vec<String>) = {
        let g = r.game_mut();
        (
            g.player(0)
                .deck
                .iter()
                .map(|c| c.card_id(&g.card_data).to_string())
                .collect(),
            g.player(1)
                .deck
                .iter()
                .map(|c| c.card_id(&g.card_data).to_string())
                .collect(),
        )
    };

    assert_eq!(
        p0_deck,
        vec![
            "FILLER0".to_string(),
            "P0-BOTTOM".to_string(),
            "P0-TOP".to_string(),
        ]
    );
    assert_eq!(
        p1_deck,
        vec!["FILLER1".to_string(), "P1-MIDDLE".to_string()]
    );
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
        g.player(0)
            .deck
            .first()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
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
        PermanentHandle {
            player: 0,
            index: 99,
        },
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

    let ok = r
        .game_mut()
        .return_to_deck_from_reveal(0, first, StackPosition::Top);
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
    assert!(!r
        .game_mut()
        .return_to_deck_from_reveal(0, bogus, StackPosition::Top));
}

// ─── place_as_bottom_source ───────────────────────────────────────────────────

use digimon_engine::enums::CardSourceRef;

fn seed_single_card_permanent_with_id(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_idx = g.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
    g.players[0]
        .battle_area
        .push(digimon_engine::permanent::Permanent::new(card, turn));
    PermanentHandle {
        player: 0,
        index: 0,
    }
}

#[test]
fn place_as_bottom_source_from_hand_stacks_under_target() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .add_card(plain_digimon("FUEL", "Fuel", 2))
        .hand(0, &["FUEL"])
        .start();

    let target = seed_single_card_permanent_with_id(&mut r, "BASE");

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Hand(0, 0), target, false);
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
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "DEAD")
            .unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.player_mut(0).trash.push(card);
    }
    assert_eq!(r.trash_size(0), 1);

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Trash(0, 0), target, false);
    assert!(ok);
    assert_eq!(r.trash_size(0), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0]
            .card_id(&g.card_data)
            .to_string()
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

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::DeckTop(0), target, false);
    assert!(ok);
    assert_eq!(r.deck_size(0), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0]
            .card_id(&g.card_data)
            .to_string()
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

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Reveal(handle), target, false);
    assert!(ok);
    assert_eq!(r.game_mut().revealed_cards.len(), 0);

    let bottom_id = {
        let g = r.game_mut();
        g.player(0).battle_area[0].card_sources[0]
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(bottom_id, "RV");
}

#[test]
fn place_as_bottom_source_bad_source_index_returns_false() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 4))
        .start();
    let target = seed_single_card_permanent_with_id(&mut r, "BASE");

    assert!(!r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Hand(0, 99), target, false));
    assert!(!r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Trash(0, 99), target, false));
    assert!(!r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::DeckTop(0), target, false)); // empty deck
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
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE3")
            .unwrap();
        let card_idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, card_idx);
        g.players[0]
            .battle_area
            .push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    let memory_before = r.memory();
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // hand_index of EVO4
        target,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
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
        g.players[0]
            .battle_area
            .push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    // With ignore_color = false, should fail (Red base vs Blue evo cost).
    let ok_strict = r.game_mut().effect_initiated_digivolve(
        0,
        0,
        target,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
    );
    assert!(
        !ok_strict,
        "color mismatch should block without ignore_color"
    );
    assert_eq!(r.hand_size(0), 1, "hand untouched after failure");

    // With ignore_color = true, should succeed.
    let ok_loose = r.game_mut().effect_initiated_digivolve(
        0,
        0,
        target,
        CostDelta::Free,
        true,
        PlaySource::ByEffect,
    );
    assert!(ok_loose, "ignore_color bypasses color check");
    assert_eq!(r.hand_size(0), 0, "EVO moved to stack");
}

#[test]
fn effect_initiated_digivolve_bad_level_returns_false() {
    // `ignore_color=true` bypasses only the color check; a level mismatch
    // still aborts. Cards that need to bypass level requirements use the
    // separate `effect_initiated_digivolve_ignore_requirements` entry
    // (Track A iteration that introduced `ignore_requirements: bool`).
    //
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
        g.players[0]
            .battle_area
            .push(digimon_engine::permanent::Permanent::new(card, turn));
        PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0,
        target,
        CostDelta::Free,
        /* ignore_color = */ true,
        PlaySource::ByEffect,
    );
    assert!(
        !ok,
        "level mismatch should return false even with ignore_color=true \
         (use effect_initiated_digivolve_ignore_requirements for level bypass)"
    );
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

    let ok =
        r.game_mut()
            .place_on_security(0, CardSourceRef::Hand(0, 0), StackPosition::Bottom, false);
    assert!(ok);
    // security is Vec; bottom = index 0
    let bottom_id = {
        let g = r.game_mut();
        g.player(0)
            .security
            .first()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(bottom_id, "BOT");
}

#[test]
fn place_on_security_bad_source_returns_false() {
    let mut r = DebugRunner::builder().start();
    let ok =
        r.game_mut()
            .place_on_security(0, CardSourceRef::Hand(0, 99), StackPosition::Top, false);
    assert!(!ok);
}

// ─── bounce_self ──────────────────────────────────────────────────────────────

/// TEST-BOUNCE-SELF: on play, return self to hand via `ctx.bounce_self()`.
struct TestBounceSelf;
impl CardEffect for TestBounceSelf {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Bounce self on play")
            .process(|ctx| {
                ctx.bounce_self();
            })
            .build()]
    }
}

#[test]
fn bounce_self_on_play_returns_card_to_hand() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BOUNCE", "Bouncer", 3))
        .hand(0, &["BOUNCE"])
        .memory(3)
        .start();

    r.register_effect("BOUNCE", Arc::new(TestBounceSelf));

    // Play BOUNCE (hand slot 0). OnPlay fires, calls bounce_self, the card
    // should return to its owner's hand and not stay on the field.
    let res = r.play(0, 0);
    assert_eq!(res, Some(0), "play should succeed");
    assert_eq!(
        r.battle_area_size(0),
        0,
        "card bounced off the field by its own OnPlay"
    );
    assert_eq!(r.hand_size(0), 1, "card returned to its owner's hand");
    let card_id = {
        let g = r.game_mut();
        g.player(0).hand[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(card_id, "BOUNCE", "the same card was returned to hand");
}

/// `bounce_self` must be a no-op when the effect has no source permanent
/// (e.g. an Option-card OptionMain effect or a rule-source effect). Returns
/// `None` rather than panicking.
#[test]
fn bounce_self_with_no_source_permanent_returns_none() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("PLACEHOLDER", "P", 1))
        .hand(0, &["PLACEHOLDER"])
        .start();
    let card_handle = {
        let g = r.game_mut();
        g.player(0).hand[0].handle()
    };
    let mut ctx = EffectContext::new(
        r.game_mut(),
        card_handle,
        /* source_permanent */ None,
        0,
    );
    let result = ctx.bounce_self();
    assert!(
        result.is_none(),
        "bounce_self with no source_permanent must return None, got {:?}",
        result
    );
}

// ─── place_self_at_security ──────────────────────────────────────────────────

/// TEST-PSAS-TOP-FACE-UP: on play, place self at top of own security stack
/// face-up via `ctx.place_self_at_security(Top, true)`. Single-source
/// permanent (no sources to trash). Mirrors EX9-021's printed
/// "place this Digimon as your top security card".
struct TestPlaceSelfTopFaceUp;
impl CardEffect for TestPlaceSelfTopFaceUp {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Place self at top security face-up")
            .process(|ctx| {
                ctx.place_self_at_security(StackPosition::Top, /* face_up = */ true);
            })
            .build()]
    }
}

#[test]
fn place_self_at_security_top_face_up_moves_card_to_top_of_security() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("PSAS_TFU", "Top Face-Up", 3))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["PSAS_TFU"])
        .security(0, &["FILLER", "FILLER"])
        .memory(3)
        .start();

    r.register_effect("PSAS_TFU", Arc::new(TestPlaceSelfTopFaceUp));

    let before_security = r.security_count(0);
    let res = r.play(0, 0);
    assert_eq!(res, Some(0), "play should succeed");
    assert_eq!(
        r.battle_area_size(0),
        0,
        "card moved off the field by its own OnPlay"
    );
    assert_eq!(
        r.security_count(0),
        before_security + 1,
        "security stack grew by 1"
    );
    // Top of security (highest index in the Vec) is the just-placed card.
    let top_id = {
        let g = r.game_mut();
        g.player(0)
            .security
            .last()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(
        top_id, "PSAS_TFU",
        "the moved card sits at the top of security"
    );
    // face_up_security tracking: the placed card's card_index is in the set.
    let placed_key = {
        let g = r.game_mut();
        g.player(0).security.last().unwrap().card_index
    };
    assert!(
        r.game_mut()
            .player(0)
            .face_up_security
            .contains(&placed_key),
        "face_up_security records the face-up placement"
    );
}

/// TEST-PSAS-BOTTOM-FACE-DOWN: at OnPlay, place self at bottom of own security
/// stack face-down. Multi-source permanent — exercises the source-trash
/// dispatch path. Mirrors EX4-060's printed "place this Digimon at the
/// bottom of your security stack face down" (via a self-replacement; here
/// we trigger the same engine path via OnPlay for testing simplicity).
struct TestPlaceSelfBottomFaceDown;
impl CardEffect for TestPlaceSelfBottomFaceDown {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Place self at bottom security face-down")
            .process(|ctx| {
                ctx.place_self_at_security(StackPosition::Bottom, /* face_up = */ false);
            })
            .build()]
    }
}

#[test]
fn place_self_at_security_bottom_face_down_routes_sources_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("PSAS_BFD", "Bottom Face-Down", 3))
        .add_card(plain_digimon("UNDER1", "Under1", 1))
        .add_card(plain_digimon("UNDER2", "Under2", 1))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .security(0, &["FILLER", "FILLER"])
        .memory(3)
        .start();
    r.register_effect("PSAS_BFD", Arc::new(TestPlaceSelfBottomFaceDown));

    // Seed a 3-source permanent on player 0's field with PSAS_BFD on top.
    // The OnPlay effect can't fire from this seeded layout (we didn't play
    // it normally), so we install a Main-on-field effect via a custom path
    // — but keeping the test simple, route the effect through ctx directly.
    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let u1 = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER1")
            .unwrap();
        let u2 = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER2")
            .unwrap();
        let top = g
            .card_data
            .iter()
            .position(|c| c.card_id == "PSAS_BFD")
            .unwrap();
        let i1 = g.next_card_index();
        let i2 = g.next_card_index();
        let it = g.next_card_index();
        let s_under1 = digimon_engine::card_source::CardSource::new(u1, 0, i1);
        let s_under2 = digimon_engine::card_source::CardSource::new(u2, 0, i2);
        let s_top = digimon_engine::card_source::CardSource::new(top, 0, it);
        let mut perm = digimon_engine::permanent::Permanent::new(s_under1, turn);
        perm.card_sources.push(s_under2);
        perm.card_sources.push(s_top);
        g.players[0].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    let top_card_handle = {
        let g = r.game_mut();
        g.player(0).battle_area[0].top_card().handle()
    };

    // Drive the placement directly through EffectContext, simulating the
    // permanent's own effect resolving with source_permanent set.
    {
        use digimon_engine::effect_context::EffectContext;
        let mut ctx = EffectContext::new(r.game_mut(), top_card_handle, Some(handle), 0);
        let ok = ctx.place_self_at_security(StackPosition::Bottom, /* face_up = */ false);
        assert!(ok, "place_self_at_security should succeed");
    }

    assert_eq!(r.battle_area_size(0), 0, "permanent gone from battle area");
    // Security: started with 2 fillers; bottom is now the moved top card.
    assert_eq!(r.security_count(0), 3, "security grew by exactly 1");
    let bottom_id = {
        let g = r.game_mut();
        g.player(0)
            .security
            .first()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(
        bottom_id, "PSAS_BFD",
        "moved top card sits at the bottom of security"
    );

    // Sources routed to owner's trash: 2 sources below top, both owner=0.
    assert_eq!(
        r.trash_size(0),
        2,
        "both sources-below-top went to owner's trash"
    );
    let trash_ids: Vec<String> = {
        let g = r.game_mut();
        g.player(0)
            .trash
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect()
    };
    assert!(trash_ids.contains(&"UNDER1".to_string()));
    assert!(trash_ids.contains(&"UNDER2".to_string()));

    // face-down: the placed card_index must NOT be in face_up_security.
    let placed_key = {
        let g = r.game_mut();
        g.player(0).security.first().unwrap().card_index
    };
    assert!(
        !r.game_mut()
            .player(0)
            .face_up_security
            .contains(&placed_key),
        "face-down placement leaves face_up_security unset"
    );
}

// ─── place_self_option_at_security ───────────────────────────────────────────

/// Helper: a Lv-less Option CardData with configurable play_cost.
fn plain_option(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
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
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// TEST-PSOAS-TOP-FACE-UP: an Option card whose [Main] body places itself at
/// the top of own security stack face-up. Mirrors ST20-15 Island of
/// Adventure's [Main] tail "Then, place this card face up as the top
/// security card."
struct TestPlaceSelfOptionTopFaceUp;
impl CardEffect for TestPlaceSelfOptionTopFaceUp {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        use digimon_engine::effect::EffectBuilder;
        use digimon_engine::enums::EffectTiming;
        vec![EffectBuilder::new(card, EffectTiming::OptionMain)
            .option_main()
            .name("Place self (Option) at top security face-up")
            .process(|ctx| {
                ctx.place_self_option_at_security(StackPosition::Top, /* face_up = */ true);
            })
            .build()]
    }
}

#[test]
fn place_self_option_at_security_top_face_up_routes_option_to_security() {
    use digimon_engine::selection::OptionPlayResult;
    let mut r = DebugRunner::builder()
        .add_card(plain_option("PSOAS_TFU", "Top Face-Up Option", 2))
        .add_card(plain_digimon("ANCHOR", "Anchor", 3))
        .add_card(plain_digimon("FILLER", "F", 1))
        .hand(0, &["PSOAS_TFU"])
        .security(0, &["FILLER", "FILLER"])
        .memory(3)
        .start();

    r.register_effect("PSOAS_TFU", Arc::new(TestPlaceSelfOptionTopFaceUp));
    // Options require a same-color (Red) Digimon on the field to be playable.
    r.place_on_field(0, "ANCHOR", Some(0));
    r.game_mut().enter_main_phase();

    let before_security = r.security_count(0);
    let before_trash = r.trash_size(0);

    let result = r.game_mut().play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Trashed,
        "play_option_from_hand returns Trashed when no pending selection \
         was installed; the dispose flow short-circuits because \
         pending_option was consumed by place_self_option_at_security"
    );

    assert_eq!(r.hand_size(0), 0, "Option left hand");
    assert_eq!(
        r.trash_size(0),
        before_trash,
        "Option did NOT route to trash — it landed in security instead"
    );
    assert_eq!(
        r.security_count(0),
        before_security + 1,
        "security stack grew by 1"
    );
    let top_id = {
        let g = r.game_mut();
        g.player(0)
            .security
            .last()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(top_id, "PSOAS_TFU", "moved Option sits at top of security");
    let placed_key = {
        let g = r.game_mut();
        g.player(0).security.last().unwrap().card_index
    };
    assert!(
        r.game_mut()
            .player(0)
            .face_up_security
            .contains(&placed_key),
        "face_up_security records the face-up placement"
    );
    assert!(
        r.game_mut().pending_option.is_none(),
        "pending_option was consumed; no further dispose"
    );
}

/// `place_self_option_at_security` returns `false` when no Option resolution
/// is in flight (no `pending_option`). Defensive correctness.
#[test]
fn place_self_option_at_security_with_no_pending_option_returns_false() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_option("PLACEHOLDER", "P", 2))
        .hand(0, &["PLACEHOLDER"])
        .start();
    let card_handle = {
        let g = r.game_mut();
        g.player(0).hand[0].handle()
    };
    let mut ctx = EffectContext::new(r.game_mut(), card_handle, None, 0);
    let result = ctx.place_self_option_at_security(StackPosition::Top, true);
    assert!(
        !result,
        "place_self_option_at_security with no pending_option must return false"
    );
}

// ─── security_place_stacked_card ─────────────────────────────────────────────

/// TEST-SPSC-TOP: extract the top stacked source (one below visible top) and
/// place it on top of own security stack face-down. Mirrors Puppets G027
/// "move the top stacked card to top security card."
#[test]
fn security_place_top_stacked_card_extracts_source_and_places_on_security() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("CARRIER", "Carrier", 3))
        .add_card(plain_digimon("UNDER", "Under", 1))
        .add_card(plain_digimon("BOTTOM", "Bottom", 1))
        .start();

    // Seed a 3-source permanent: BOTTOM, UNDER, CARRIER (top).
    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let bot_data = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BOTTOM")
            .unwrap();
        let under_data = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER")
            .unwrap();
        let top_data = g
            .card_data
            .iter()
            .position(|c| c.card_id == "CARRIER")
            .unwrap();
        let i_bot = g.next_card_index();
        let i_under = g.next_card_index();
        let i_top = g.next_card_index();
        let s_bot = digimon_engine::card_source::CardSource::new(bot_data, 0, i_bot);
        let s_under = digimon_engine::card_source::CardSource::new(under_data, 0, i_under);
        let s_top = digimon_engine::card_source::CardSource::new(top_data, 0, i_top);
        let mut perm = digimon_engine::permanent::Permanent::new(s_bot, turn);
        perm.card_sources.push(s_under);
        perm.card_sources.push(s_top);
        g.players[0].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    let before_security = r.security_count(0);
    let top_card_handle = {
        let g = r.game_mut();
        g.player(0).battle_area[0].top_card().handle()
    };

    // Drive via EffectContext directly (avoids needing to play a card).
    {
        let mut ctx = EffectContext::new(r.game_mut(), top_card_handle, Some(handle), 0);
        let ok = ctx.security_place_top_stacked_card(handle, 0, StackPosition::Top, false);
        assert!(ok, "security_place_top_stacked_card should succeed");
    }

    // The carrier permanent still exists with a 2-source stack (BOTTOM + CARRIER).
    let stack_size = r.game_mut().player(0).battle_area[0].stack_size();
    assert_eq!(
        stack_size, 2,
        "the top stacked card (UNDER) was extracted; carrier retains BOTTOM + CARRIER"
    );

    // Security grew by 1; the new top of security is UNDER.
    assert_eq!(r.security_count(0), before_security + 1);
    let placed_id = {
        let g = r.game_mut();
        g.player(0)
            .security
            .last()
            .unwrap()
            .card_id(&g.card_data)
            .to_string()
    };
    assert_eq!(
        placed_id, "UNDER",
        "the extracted source moved to security top"
    );
}

/// `security_place_top_stacked_card` returns `false` when the carrier has
/// fewer than 2 sources (no stacked card below the top).
#[test]
fn security_place_top_stacked_card_returns_false_with_single_source() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("LONE", "Lone", 3))
        .start();

    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "LONE")
            .unwrap();
        let idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, 0, idx);
        let perm = digimon_engine::permanent::Permanent::new(card, turn);
        g.players[0].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 0,
        }
    };

    let card_handle = {
        let g = r.game_mut();
        g.player(0).battle_area[0].top_card().handle()
    };
    let before_security = r.security_count(0);
    let before_battle = r.battle_area_size(0);

    let mut ctx = EffectContext::new(r.game_mut(), card_handle, Some(handle), 0);
    let ok = ctx.security_place_top_stacked_card(handle, 0, StackPosition::Top, false);
    assert!(!ok, "single-source carrier has no stacked card below top");
    assert_eq!(r.security_count(0), before_security, "no security mutation");
    assert_eq!(r.battle_area_size(0), before_battle, "no carrier mutation");
}

// ─── search_own_security_stack ────────────────────────────────────────────────

#[test]
fn search_own_security_stack_installs_selection_with_matching_filter() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card({
            let mut c = plain_digimon("WANTED", "Wanted", 3);
            c.traits = vec!["TARGET".to_string()];
            c
        })
        .add_card(plain_digimon("FILLER", "F", 1))
        .security(0, &["FILLER", "WANTED", "FILLER"])
        .start();

    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, /* acting */ 0);
        ctx.search_own_security_stack(
            "Search own security",
            /* is_optional */ false,
            |_g, c| c.card_id(&_g.card_data) == "WANTED",
            |_ctx, _idx| {
                // Test only verifies installation — no follow-up mutation.
            },
        );
    }
    let pending = r
        .game_mut()
        .pending_selection
        .as_ref()
        .expect("selection installed");
    assert_eq!(
        pending.selecting_player, 0,
        "the controller (acting player) is the selector"
    );
    // Filter matches exactly 1 card → 1 valid action id.
    assert_eq!(
        pending.valid_action_ids.len(),
        1,
        "exactly one slot matches the WANTED filter"
    );
}

#[test]
fn search_own_security_stack_silent_noop_with_no_match() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("FILLER", "F", 1))
        .security(0, &["FILLER", "FILLER"])
        .start();

    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.search_own_security_stack(
            "Search",
            false,
            |_g, _c| false, // matches nothing
            |_ctx, _idx| {},
        );
    }
    assert!(
        r.game_mut().pending_selection.is_none(),
        "no candidates → no selection installed (silent no-op contract)"
    );
}

// ─── trash_opponent_hand_to_count ─────────────────────────────────────────────

#[test]
fn trash_opponent_hand_to_count_noop_when_hand_already_below_target() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("CARD", "C", 1))
        .hand(1, &["CARD"])
        .start();

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, /* acting */ 0);
        ctx.trash_opponent_hand_to_count(/* opponent */ 1, /* target */ 5)
    };
    assert!(!ok, "hand size 1 already <= target 5; no-op");
    assert!(
        r.game_mut().pending_selection.is_none(),
        "no selection installed for no-op path"
    );
    assert_eq!(r.hand_size(1), 1, "hand untouched");
}

#[test]
fn trash_opponent_hand_to_count_installs_opponent_selection_when_hand_over_target() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("C1", "C1", 1))
        .add_card(plain_digimon("C2", "C2", 1))
        .add_card(plain_digimon("C3", "C3", 1))
        .add_card(plain_digimon("C4", "C4", 1))
        .add_card(plain_digimon("C5", "C5", 1))
        .hand(1, &["C1", "C2", "C3", "C4", "C5"])
        .start();

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, /* acting */ 0);
        ctx.trash_opponent_hand_to_count(/* opponent */ 1, /* target */ 2)
    };
    assert!(ok, "hand size 5 > target 2 — must install selection");
    let pending = r
        .game_mut()
        .pending_selection
        .as_ref()
        .expect("multi-pick selection installed");
    assert_eq!(
        pending.selecting_player, 1,
        "the OPPONENT picks which cards to trash (no-approximations rule)"
    );
    assert!(
        !pending.is_optional,
        "forced reduction must not allow PASS at zero picks"
    );
}

// ─── trash_top_n_digivolution_cards_of_each ──────────────────────────────────

#[test]
fn trash_top_n_digivolution_cards_of_each_peels_one_source_per_perm() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP_A", "TopA", 3))
        .add_card(plain_digimon("UNDER_A", "UnderA", 1))
        .add_card(plain_digimon("BASE_A", "BaseA", 1))
        .add_card(plain_digimon("TOP_B", "TopB", 3))
        .add_card(plain_digimon("UNDER_B", "UnderB", 1))
        .add_card(plain_digimon("LONE", "Lone", 1))
        .start();

    // Player 1's field: 3 permanents.
    //   [0] 3-source: BASE_A, UNDER_A, TOP_A (visible top = TOP_A)
    //   [1] 2-source: UNDER_B, TOP_B (visible top = TOP_B)
    //   [2] 1-source: LONE (visible top = LONE; no digivolution sources)
    let _ = {
        let g = r.game_mut();
        let turn = g.turn_count;
        for (top_id, under_ids) in &[
            ("TOP_A", vec!["BASE_A", "UNDER_A"]),
            ("TOP_B", vec!["UNDER_B"]),
            ("LONE", vec![]),
        ] {
            let mut sources = Vec::new();
            for under in under_ids {
                let data_idx = g
                    .card_data
                    .iter()
                    .position(|c| c.card_id == *under)
                    .unwrap();
                let next_idx = g.next_card_index();
                sources.push(digimon_engine::card_source::CardSource::new(
                    data_idx, 1, next_idx,
                ));
            }
            let top_idx = g
                .card_data
                .iter()
                .position(|c| c.card_id == *top_id)
                .unwrap();
            let next_idx = g.next_card_index();
            let top = digimon_engine::card_source::CardSource::new(top_idx, 1, next_idx);
            let mut perm = if let Some(first_under) = sources.into_iter().next() {
                let mut p = digimon_engine::permanent::Permanent::new(first_under, turn);
                // Re-build remaining sources + top
                if *top_id == "TOP_A" {
                    let under_a_idx = g
                        .card_data
                        .iter()
                        .position(|c| c.card_id == "UNDER_A")
                        .unwrap();
                    let ix = g.next_card_index();
                    p.card_sources
                        .push(digimon_engine::card_source::CardSource::new(
                            under_a_idx,
                            1,
                            ix,
                        ));
                }
                p.card_sources.push(top);
                p
            } else {
                digimon_engine::permanent::Permanent::new(top, turn)
            };
            // Quick correctness assert during seed
            let _ = &mut perm;
            g.players[1].battle_area.push(perm);
        }
    };

    // Confirm seeded layout.
    assert_eq!(r.battle_area_size(1), 3);
    let stack_sizes: Vec<usize> = (0..3)
        .map(|i| r.game_mut().player(1).battle_area[i].stack_size())
        .collect();
    assert_eq!(stack_sizes, vec![3, 2, 1]);

    let p1_trash_before = r.trash_size(1);

    let trashed = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, /* acting */ 0);
        ctx.trash_top_n_digivolution_cards_of_each(/* target */ 1, 1)
    };

    // 2 sources peeled (one each from the 3-source and 2-source perms; the
    // 1-source perm is skipped — its visible top is not a digivolution card).
    assert_eq!(trashed, 2, "expected 2 sources peeled");

    // Battle area still has 3 permanents (visible tops untouched).
    assert_eq!(r.battle_area_size(1), 3);
    let stack_sizes_after: Vec<usize> = (0..3)
        .map(|i| r.game_mut().player(1).battle_area[i].stack_size())
        .collect();
    assert_eq!(
        stack_sizes_after,
        vec![2, 1, 1],
        "3-source → 2 (peeled UNDER_A); 2-source → 1 (peeled UNDER_B); 1-source unchanged"
    );

    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 2,
        "two sources routed to player 1's trash (both owned by player 1)"
    );
}

#[test]
fn trash_top_n_digivolution_cards_of_each_handles_n_zero() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder().start();
    let trashed = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.trash_top_n_digivolution_cards_of_each(1, 0)
    };
    assert_eq!(trashed, 0);
}

// ─── return_all_trash_to_deck_bottom ─────────────────────────────────────────

#[test]
fn return_all_trash_to_deck_bottom_drains_trash_to_owners_deck() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("T1", "T1", 1))
        .add_card(plain_digimon("T2", "T2", 1))
        .add_card(plain_digimon("T3", "T3", 1))
        .start();

    // Seed three cards in player 0's trash, all owned by player 0.
    {
        let g = r.game_mut();
        for id in &["T1", "T2", "T3"] {
            let data_idx = g.card_data.iter().position(|c| c.card_id == *id).unwrap();
            let next_idx = g.next_card_index();
            let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
            g.players[0].trash.push(card);
        }
    }

    let p0_deck_before = r.deck_size(0);
    assert_eq!(r.trash_size(0), 3);

    let handles = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.return_all_trash_to_deck_bottom(0)
    };

    assert_eq!(handles.len(), 3, "all 3 cards moved");
    assert_eq!(r.trash_size(0), 0, "trash drained");
    assert_eq!(r.deck_size(0), p0_deck_before + 3, "deck grew by exactly 3");
    // Bottom of deck is at index 0; after `insert(0, card)` for T1 then T2 then T3,
    // T3 is at index 0, T2 at index 1, T1 at index 2 — i.e. T3 is the deck bottom,
    // T1 is closest to the top among the inserted set.
    let bottom_id = {
        let g = r.game_mut();
        g.player(0).deck[0].card_id(&g.card_data).to_string()
    };
    assert_eq!(bottom_id, "T3", "last-drained card sits at deck bottom");
}

#[test]
fn return_all_trash_to_deck_bottom_routes_each_card_to_its_owner() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_A", "A", 1))
        .add_card(plain_digimon("OWNED_BY_B", "B", 1))
        .start();

    // Seed player 1's trash with one card owned by player 0 and one by player 1.
    {
        let g = r.game_mut();
        let a_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "OWNED_BY_A")
            .unwrap();
        let b_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "OWNED_BY_B")
            .unwrap();
        let i_a = g.next_card_index();
        let i_b = g.next_card_index();
        let card_a = digimon_engine::card_source::CardSource::new(a_idx, /* owner */ 0, i_a);
        let card_b = digimon_engine::card_source::CardSource::new(b_idx, /* owner */ 1, i_b);
        g.players[1].trash.push(card_a);
        g.players[1].trash.push(card_b);
    }

    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    let _handles = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.return_all_trash_to_deck_bottom(/* drain player 1's trash */ 1)
    };

    assert_eq!(r.trash_size(1), 0, "player 1's trash drained");
    assert_eq!(
        r.deck_size(0),
        p0_deck_before + 1,
        "player 0's card returned to player 0's deck"
    );
    assert_eq!(
        r.deck_size(1),
        p1_deck_before + 1,
        "player 1's card returned to player 1's deck"
    );
}

// ─── return_trash_cards_to_deck_top ──────────────────────────────────────────
// G-ZONE-SELECTED-TRASH-TO-DECK-TOP.

/// A selected trash card returned to the deck TOP becomes the next card drawn.
#[test]
fn return_trash_cards_to_deck_top_places_card_at_deck_top() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TRASHCARD", "TrashCard", 1))
        .add_card(plain_digimon("DECKCARD", "DeckCard", 1))
        .start();

    // Seed player 0: one card already in the deck, one card in the trash.
    let trash_handle = {
        let g = r.game_mut();
        let deck_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "DECKCARD")
            .unwrap();
        let di = g.next_card_index();
        g.players[0]
            .deck
            .push(digimon_engine::card_source::CardSource::new(deck_idx, 0, di));
        let t_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TRASHCARD")
            .unwrap();
        let ti = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(t_idx, 0, ti);
        let h = card.handle();
        g.players[0].trash.push(card);
        h
    };

    let moved = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.return_trash_cards_to_deck_top(0, &[trash_handle])
    };

    assert_eq!(moved.len(), 1, "the selected trash card moved");
    assert_eq!(r.trash_size(0), 0, "card left the trash");
    // Deck top = Vec end (the slot `draw` pops first).
    let top_id = {
        let g = r.game_mut();
        let deck = &g.player(0).deck;
        deck[deck.len() - 1].card_id(&g.card_data).to_string()
    };
    assert_eq!(
        top_id, "TRASHCARD",
        "the returned trash card sits on top of the deck — the next card drawn"
    );
}

/// Returning several selected trash cards to the top preserves selection
/// order as draw order: the first handle is drawn first.
#[test]
fn return_trash_cards_to_deck_top_preserves_selection_order_as_draw_order() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("CARD_A", "CardA", 1))
        .add_card(plain_digimon("CARD_B", "CardB", 1))
        .start();

    let (a, b) = {
        let g = r.game_mut();
        let mut mk = |id: &str| {
            let idx = g.card_data.iter().position(|c| c.card_id == id).unwrap();
            let ci = g.next_card_index();
            let card = digimon_engine::card_source::CardSource::new(idx, 0, ci);
            let h = card.handle();
            g.players[0].trash.push(card);
            h
        };
        let a = mk("CARD_A");
        let b = mk("CARD_B");
        (a, b)
    };

    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        // Selection order [A, B] → A is drawn first.
        ctx.return_trash_cards_to_deck_top(0, &[a, b]);
    }

    let g = r.game_mut();
    let deck = &g.player(0).deck;
    let n = deck.len();
    assert_eq!(
        deck[n - 1].card_id(&g.card_data),
        "CARD_A",
        "first-selected card is on top (drawn first)"
    );
    assert_eq!(
        deck[n - 2].card_id(&g.card_data),
        "CARD_B",
        "second-selected card sits just beneath it"
    );
}

// ─── Owner-vs-controller routing ─────────────────────────────────────────────

/// Track E owner-routing rule: a permanent owned by player A but
/// controlled by player B (e.g. via a future control-transfer effect)
/// returns to A's deck, not B's, when bounced. Today the engine has no
/// control-transfer mechanic, but the routing must be correct so a future
/// landing card doesn't silently misroute.
#[test]
fn return_to_deck_routes_top_card_to_owner_not_controller() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_A", "OwnedByA", 3))
        .start();

    // Seed a 1-card permanent in player B's (1's) battle area, but with
    // top.owner = 0 (player A is the owner). Simulates a control-transfer
    // having taken place. Direct mutation is the only way to set up this
    // shape today since the engine doesn't yet have a control-transfer
    // effect.
    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "OWNED_BY_A")
            .unwrap();
        let idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, /* owner */ 0, idx);
        let perm = digimon_engine::permanent::Permanent::new(card, turn);
        g.players[1].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0,
        }
    };

    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    let ok = r.game_mut().return_to_deck(handle, StackPosition::Bottom);
    assert!(ok, "return_to_deck should succeed");

    assert_eq!(
        r.deck_size(0),
        p0_deck_before + 1,
        "card returned to OWNER's deck (player 0), not controller's"
    );
    assert_eq!(
        r.deck_size(1),
        p1_deck_before,
        "controller's deck (player 1) unchanged"
    );
    assert_eq!(r.battle_area_size(1), 0, "permanent gone from battle area");
}

#[test]
fn return_to_hand_routes_top_card_to_owner_not_controller() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_A", "OwnedByA", 3))
        .start();

    let handle = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "OWNED_BY_A")
            .unwrap();
        let idx = g.next_card_index();
        let card = digimon_engine::card_source::CardSource::new(data_idx, /* owner */ 0, idx);
        let perm = digimon_engine::permanent::Permanent::new(card, turn);
        g.players[1].battle_area.push(perm);
        digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0,
        }
    };

    let p0_hand_before = r.hand_size(0);
    let p1_hand_before = r.hand_size(1);

    let returned = r.game_mut().return_to_hand(handle);
    assert!(returned.is_some(), "return_to_hand should succeed");

    assert_eq!(
        r.hand_size(0),
        p0_hand_before + 1,
        "card returned to OWNER's hand (player 0), not controller's"
    );
    assert_eq!(
        r.hand_size(1),
        p1_hand_before,
        "controller's hand (player 1) unchanged"
    );
}

/// `place_self_at_security` must return `false` when the effect has no
/// source permanent.
#[test]
fn place_self_at_security_with_no_source_permanent_returns_false() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("PLACEHOLDER", "P", 1))
        .hand(0, &["PLACEHOLDER"])
        .start();
    let card_handle = {
        let g = r.game_mut();
        g.player(0).hand[0].handle()
    };
    let mut ctx = EffectContext::new(
        r.game_mut(),
        card_handle,
        /* source_permanent */ None,
        0,
    );
    let result = ctx.place_self_at_security(StackPosition::Top, true);
    assert!(
        !result,
        "place_self_at_security with no source_permanent must return false"
    );
}
