//! Task 1.5 — the sampler invariant checker.
//!
//! A buggy world sampler corrupts search *silently* (design "Risks": worlds
//! with over-copied cards or cards in two zones bias every value estimate),
//! so every sampled world can be audited against its source:
//!
//! 1. **Counts balance**: every zone of every player has exactly the
//!    source's count.
//! 2. **Conservation / copy limits**: each player's full multiset across
//!    ALL zones equals the source's, and never exceeds the decklist copy
//!    count (`Player::original_deck`) for any card. Tokens (not from a
//!    deck) are excluded.
//! 3. **Pins honored**: every face-up security card sits at its source slot
//!    with its source identity (and instance).
//! 4. **Viewer-visible state identical**: public zones of both players,
//!    plus the viewer's exact hand, plus the public scalars.
//! 5. **No live reveal source** on the world.
//! 6. **Opaque books reconciled** (defensive — Phase-1 worlds are never
//!    opaque, but the helper is public for future PvP callers): see
//!    [`reconcile_opaque_books`].

use std::collections::BTreeMap;

use super::infoset::{security_pins, zone_ids, CardMultiset};
use crate::enums::PlayerId;
use crate::game::Game;
use crate::opaque_deck::OpaqueDeckState;
use crate::player::Player;

/// Reconcile BOTH books of an [`OpaqueDeckState`].
///
/// The two books are deliberately desynced mid-materialization:
/// `reserve_placeholders` debits `total_remaining` without touching the
/// per-card multiset, and `consume_per_card_only` debits the per-card
/// multiset without touching `total_remaining`. The reconciliation
/// invariant is
///
/// ```text
/// Σ per-card counts == total_remaining + outstanding_placeholders
/// ```
///
/// where `outstanding_placeholders` is the number of physically-placed but
/// not-yet-materialized placeholder cards (`CardSource::is_opaque_placeholder`)
/// currently in the player's zones. Note `restore()` never errors — a
/// restore of an id that was never in the decklist moves both books together
/// (this invariant stays green) but silently diverges the composition from
/// the decklist; that drift is caught by the decklist ≤ check in
/// [`check_world_invariants`], not here.
pub fn reconcile_opaque_books(
    state: &OpaqueDeckState,
    outstanding_placeholders: usize,
) -> Result<(), String> {
    let per_card_sum: usize = state.composition().values().sum();
    let expected = state.total_remaining() + outstanding_placeholders;
    if per_card_sum != expected {
        return Err(format!(
            "opaque books desynced: per-card multiset sums to {per_card_sum} but \
             total_remaining ({}) + outstanding placeholders ({outstanding_placeholders}) = {expected}",
            state.total_remaining()
        ));
    }
    Ok(())
}

/// Count placeholder card sources across all of a player's zones.
fn outstanding_placeholders(p: &Player) -> usize {
    let count_in = |cards: &[crate::card_source::CardSource]| {
        cards.iter().filter(|c| c.is_opaque_placeholder).count()
    };
    let mut n = count_in(&p.hand)
        + count_in(&p.deck)
        + count_in(&p.security)
        + count_in(&p.digitama_deck)
        + count_in(&p.trash);
    for perm in &p.battle_area {
        n += count_in(&perm.card_sources) + count_in(&perm.linked_cards);
    }
    if let Some(perm) = &p.breeding_area {
        n += count_in(&perm.card_sources) + count_in(&perm.linked_cards);
    }
    n
}

/// Full per-player multiset across all zones (hand, deck, security,
/// digitama, trash, battle-area stacks + linked, breeding, commander,
/// plus revealed cards owned by the player). Tokens and opaque
/// placeholders are skipped (not decklist cards / identity unknown).
fn all_zone_multiset(game: &Game, pid: PlayerId) -> CardMultiset {
    let p = &game.players[pid as usize];
    let mut ms = CardMultiset::new();
    let mut add_all = |cards: &[crate::card_source::CardSource], ms: &mut CardMultiset| {
        for c in cards {
            if c.is_token || c.is_opaque_placeholder {
                continue;
            }
            *ms.entry(c.card_id(&game.card_data).to_string()).or_insert(0) += 1;
        }
    };
    add_all(&p.hand, &mut ms);
    add_all(&p.deck, &mut ms);
    add_all(&p.security, &mut ms);
    add_all(&p.digitama_deck, &mut ms);
    add_all(&p.trash, &mut ms);
    for perm in &p.battle_area {
        add_all(&perm.card_sources, &mut ms);
        add_all(&perm.linked_cards, &mut ms);
    }
    if let Some(perm) = &p.breeding_area {
        add_all(&perm.card_sources, &mut ms);
        add_all(&perm.linked_cards, &mut ms);
    }
    if let Some(c) = &p.commander_zone {
        add_all(std::slice::from_ref(c), &mut ms);
    }
    for c in &game.revealed_cards {
        if c.owner == pid {
            add_all(std::slice::from_ref(c), &mut ms);
        }
    }
    ms
}

fn decklist_multiset(p: &Player) -> BTreeMap<String, usize> {
    p.original_deck
        .iter()
        .map(|e| (e.card_id.clone(), e.count as usize))
        .collect()
}

/// Verify a (source game, viewer, sampled world) triple. See module docs
/// for the checked invariants. Returns a descriptive error on the first
/// violation.
pub fn check_world_invariants(
    source: &Game,
    viewer: PlayerId,
    world: &Game,
) -> Result<(), String> {
    if world.reveal_source.is_some() {
        return Err("world carries a live reveal_source (must be None)".into());
    }
    if source.players.len() != world.players.len() {
        return Err("player count mismatch".into());
    }

    for pid in 0..source.players.len() as PlayerId {
        let sp = &source.players[pid as usize];
        let wp = &world.players[pid as usize];

        // 1. Per-zone counts.
        let zones = [
            ("hand", sp.hand.len(), wp.hand.len()),
            ("deck", sp.deck.len(), wp.deck.len()),
            ("security", sp.security.len(), wp.security.len()),
            ("digitama", sp.digitama_deck.len(), wp.digitama_deck.len()),
            ("trash", sp.trash.len(), wp.trash.len()),
            ("battle_area", sp.battle_area.len(), wp.battle_area.len()),
        ];
        for (name, s, w) in zones {
            if s != w {
                return Err(format!(
                    "player {pid} zone `{name}` count drifted: source {s}, world {w}"
                ));
            }
        }
        if sp.breeding_area.is_some() != wp.breeding_area.is_some() {
            return Err(format!("player {pid} breeding presence drifted"));
        }

        // 2. Conservation + copy limits.
        let s_ms = all_zone_multiset(source, pid);
        let w_ms = all_zone_multiset(world, pid);
        if s_ms != w_ms {
            return Err(format!(
                "player {pid} all-zone multiset drifted between source and world \
                 (a card was invented, dropped, or double-placed)"
            ));
        }
        let deck_ms = decklist_multiset(wp);
        for (id, n) in &w_ms {
            match deck_ms.get(id) {
                Some(limit) if n <= limit => {}
                Some(limit) => {
                    return Err(format!(
                        "player {pid} card `{id}` appears {n}x across zones, \
                         exceeding its decklist count {limit}"
                    ));
                }
                None => {
                    return Err(format!(
                        "player {pid} card `{id}` appears in the world but not in the decklist"
                    ));
                }
            }
        }

        // 3. Pins: face-up security identical (slot, id, instance).
        let s_pins = security_pins(sp, source);
        let w_pins = security_pins(wp, world);
        if s_pins != w_pins {
            return Err(format!(
                "player {pid} face-up security pins drifted: source {s_pins:?}, world {w_pins:?}"
            ));
        }
        for (idx, _) in &s_pins {
            if sp.security[*idx].card_index != wp.security[*idx].card_index {
                return Err(format!(
                    "player {pid} pinned security instance at slot {idx} was replaced"
                ));
            }
        }

        // 6. Opaque books (defensive; Phase-1 worlds are never opaque).
        for (label, g, p) in [("source", source, sp), ("world", world, wp)] {
            let _ = g;
            if let Some(state) = &p.opaque_deck_state {
                reconcile_opaque_books(state, outstanding_placeholders(p))
                    .map_err(|e| format!("{label} player {pid}: {e}"))?;
            }
        }

        // 4a. Public zones identical (both players).
        if zone_ids(&sp.trash, source) != zone_ids(&wp.trash, world) {
            return Err(format!("player {pid} trash (public) drifted"));
        }
        for (i, (sperm, wperm)) in sp.battle_area.iter().zip(&wp.battle_area).enumerate() {
            if zone_ids(&sperm.card_sources, source) != zone_ids(&wperm.card_sources, world)
                || zone_ids(&sperm.linked_cards, source) != zone_ids(&wperm.linked_cards, world)
                || sperm.is_suspended != wperm.is_suspended
            {
                return Err(format!("player {pid} battle-area permanent {i} (public) drifted"));
            }
        }
        if let (Some(sperm), Some(wperm)) = (&sp.breeding_area, &wp.breeding_area) {
            if zone_ids(&sperm.card_sources, source) != zone_ids(&wperm.card_sources, world) {
                return Err(format!("player {pid} breeding stack (public) drifted"));
            }
        }
    }

    // 4b. Viewer's own hand: exact, in order.
    let sv = &source.players[viewer as usize];
    let wv = &world.players[viewer as usize];
    if zone_ids(&sv.hand, source) != zone_ids(&wv.hand, world) {
        return Err("viewer's own hand drifted".into());
    }

    // 4c. Public scalars + revealed cards.
    if source.current_phase != world.current_phase
        || source.turn_count != world.turn_count
        || source.memory != world.memory
        || source.memory_pair != world.memory_pair
        || source.turn_order != world.turn_order
        || source.turn_player_idx != world.turn_player_idx
        || source.game_over != world.game_over
        || source.winner != world.winner
        || source.mulligan_pending != world.mulligan_pending
        || source.mulligan_used != world.mulligan_used
    {
        return Err("public scalar state drifted".into());
    }
    let reveal = |g: &Game| -> Vec<(PlayerId, String)> {
        g.revealed_cards
            .iter()
            .map(|c| (c.owner, c.card_id(&g.card_data).to_string()))
            .collect()
    };
    if reveal(source) != reveal(world) {
        return Err("revealed cards (public) drifted".into());
    }

    Ok(())
}
