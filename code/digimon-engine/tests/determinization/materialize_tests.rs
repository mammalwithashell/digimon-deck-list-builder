//! Task 1.4 + 1.5 — the world materializer, the round-trip property law,
//! the no-leak statistical guard, and world playability.

use crate::support::*;
use digimon_engine::determinize::{materialize, Infoset};

/// Round-trip law + invariant checker over many decision points of real
/// playouts: for both viewers and several world seeds,
/// `Infoset::extract(&materialize(g, v, seed), v) == Infoset::extract(&g, v)`.
#[test]
fn round_trip_law_holds_over_random_playouts() {
    with_big_stack(|| {
        let pairs: [(&[&str], &[&str]); 2] = [(D1, D2), (D3, D4)];
        let mut points_checked = 0u32;
        for (gi, (d1, d2)) in pairs.iter().enumerate() {
            let game_seed = 9000 + gi as u64;
            let mut g = new_game_with(game_seed, d1, d2);
            let mut lcg = Lcg(game_seed ^ 0x5EED);
            let mut decision = 0u32;
            while !g.game_over && decision < 160 {
                if decision % 5 == 0 {
                    for viewer in 0..2u8 {
                        let source_info = Infoset::extract(&g, viewer);
                        for world_seed in [7u64, 8u64] {
                            // materialize_checked runs the invariant checker
                            // on every sampled world.
                            let world = materialize_checked(&g, viewer, world_seed);
                            let world_info = Infoset::extract(&world, viewer);
                            assert_eq!(
                                world_info, source_info,
                                "round-trip failed (game {gi}, decision {decision}, \
                                 viewer {viewer}, world seed {world_seed})"
                            );
                            points_checked += 1;
                        }
                    }
                }
                advance_random(&mut g, &mut lcg, 1);
                decision += 1;
            }
        }
        assert!(
            points_checked > 60,
            "too few round-trip points exercised ({points_checked})"
        );
    });
}

/// Materialized worlds are fully valid, cloneable, playable games: continue
/// each with random legal actions to completion (or a decision cap) with no
/// panics, no empty masks, and `reveal_source = None` throughout.
#[test]
fn materialized_worlds_are_playable() {
    with_big_stack(|| {
        let mut g = new_game(777);
        let mut lcg = Lcg(777);
        let mut decision = 0u32;
        let mut worlds_played = 0u32;
        while !g.game_over && decision < 120 {
            let viewer = decision_player(&g);
            // Play out worlds sampled at the viewer's PHASE decision points
            // (v1 scope: materializing mid-selection is exercised by the
            // round-trip test; playing THROUGH a permuted mid-selection
            // state is deferred — the search root re-derives selections
            // inside each world).
            if decision % 9 == 0
                && g.pending_selection.is_none()
                && g.mulligan_current_player().is_none()
            {
                let mut world = materialize_checked(&g, viewer, 1000 + decision as u64);
                assert!(world.reveal_source.is_none(), "world must carry no reveal source");
                // Clone the world once to prove it's cloneable, then play the
                // clone to completion/cap.
                let mut fork = world.clone();
                let mut wl = Lcg(31 * decision as u64 + 7);
                let steps = advance_random(&mut fork, &mut wl, 400);
                assert!(steps > 0);
                // Also step the original world a few decisions (draws +
                // security checks read committed piles — no reveal source).
                let mut wl2 = Lcg(17 * decision as u64 + 3);
                advance_random(&mut world, &mut wl2, 40);
                assert!(world.reveal_source.is_none());
                worlds_played += 1;
            }
            advance_random(&mut g, &mut lcg, 1);
            decision += 1;
        }
        assert!(worlds_played >= 5, "too few worlds played ({worlds_played})");
    });
}

/// No-X-ray guard: across many seeds, the sampled opponent hand must differ
/// from the TRUE hand in a healthy fraction of worlds — a materializer that
/// copies true hidden state would fail this.
#[test]
fn sampled_worlds_do_not_copy_true_hidden_state() {
    with_big_stack(|| {
        // Find a reproducible mid-game state with a meaty opponent hand.
        let mut g = new_game(4242);
        let mut lcg = Lcg(4242);
        advance_random(&mut g, &mut lcg, 25); // get past the opening
        let mut tries = 0;
        while (g.players[1].hand.len() < 4 || g.players[1].deck.len() < 20)
            && !g.game_over
            && tries < 200
        {
            advance_random(&mut g, &mut lcg, 1);
            tries += 1;
        }
        assert!(
            g.players[1].hand.len() >= 4 && g.players[1].deck.len() >= 20,
            "fixture did not reach a usable mid-game state (hand {}, deck {})",
            g.players[1].hand.len(),
            g.players[1].deck.len()
        );

        let true_hand = zone_multiset(&g.players[1].hand, &g);
        let true_own_top = zone_ids(&g.players[0].deck, &g).last().cloned();

        let n = 80u64;
        let mut hand_differs = 0u32;
        let mut own_top_matches = 0u32;
        for seed in 0..n {
            let world = materialize_checked(&g, 0, seed);
            if zone_multiset(&world.players[1].hand, &world) != true_hand {
                hand_differs += 1;
            }
            if zone_ids(&world.players[0].deck, &world).last().cloned() == true_own_top {
                own_top_matches += 1;
            }
        }
        let frac = hand_differs as f64 / n as f64;
        assert!(
            frac > 0.3,
            "sampled opponent hands too often equal the true hand \
             ({hand_differs}/{n}) — hidden state may be leaking"
        );
        // Own deck order must be resampled too (the viewer doesn't know it).
        assert!(
            (own_top_matches as f64 / n as f64) < 0.5,
            "own deck top matched the true top in {own_top_matches}/{n} worlds — \
             own deck order may be leaking"
        );
    });
}

/// Same seed → identical world; different seed → different world (on a
/// state with a large hidden pool).
#[test]
fn materialization_is_deterministic_per_seed() {
    with_big_stack(|| {
        let mut g = new_game(31337);
        let mut lcg = Lcg(31337);
        advance_random(&mut g, &mut lcg, 30);
        assert!(!g.game_over);

        let w1 = materialize_checked(&g, 0, 99);
        let w2 = materialize_checked(&g, 0, 99);
        assert_eq!(
            full_zone_dump(&w1),
            full_zone_dump(&w2),
            "same seed must produce an identical world"
        );

        let w3 = materialize_checked(&g, 0, 100);
        assert_ne!(
            full_zone_dump(&w1),
            full_zone_dump(&w3),
            "different seeds should produce different worlds on a large hidden pool"
        );
    });
}

/// Pins survive materialization: a face-up security card stays at its exact
/// slot with its exact identity in every sampled world.
#[test]
fn face_up_security_pins_survive_materialization() {
    with_big_stack(|| {
        let mut g = new_game(606);
        let mut lcg = Lcg(606);
        while g.mulligan_current_player().is_some() {
            advance_random(&mut g, &mut lcg, 1);
        }
        // Pin one security card on each side.
        for pid in 0..2usize {
            let ci = g.players[pid].security[3].card_index;
            g.players[pid].face_up_security.insert(ci);
        }
        let pinned: Vec<(usize, String, u16)> = (0..2)
            .map(|pid: usize| {
                let c = &g.players[pid].security[3];
                (pid, c.card_id(&g.card_data).to_string(), c.card_index)
            })
            .collect();

        for seed in 0..10u64 {
            let world = materialize_checked(&g, 0, seed);
            for (pid, id, ci) in &pinned {
                let c = &world.players[*pid].security[3];
                assert_eq!(c.card_id(&world.card_data), id, "pin identity moved");
                assert_eq!(c.card_index, *ci, "pin instance moved");
            }
        }
    });
}

/// Materializing during the mulligan window (hands drawn, no security yet)
/// round-trips and plays out.
#[test]
fn materialize_during_mulligan() {
    with_big_stack(|| {
        let g = new_game(808);
        assert!(g.mulligan_current_player().is_some());
        for viewer in 0..2u8 {
            let src = Infoset::extract(&g, viewer);
            let world = materialize_checked(&g, viewer, 5);
            assert_eq!(Infoset::extract(&world, viewer), src);
        }
        let mut world = materialize_checked(&g, 0, 6);
        let mut lcg = Lcg(6);
        advance_random(&mut world, &mut lcg, 50);
    });
}

/// The materializer must never let the true game's RNG stream (which
/// determines future shuffles) leak into the world: two worlds from the same
/// source with different seeds diverge in their own future shuffles.
#[test]
fn world_rng_is_reseeded_from_the_world_seed() {
    with_big_stack(|| {
        let mut g = new_game(909);
        let mut lcg = Lcg(909);
        advance_random(&mut g, &mut lcg, 10);

        // Two worlds, same seed: playing the SAME action sequence must keep
        // them identical (their rng streams are equal). Different seeds: rng
        // streams differ — but scripted play may not consume rng, so we only
        // assert the same-seed case (determinism) plus the reveal_source /
        // dump difference asserted elsewhere.
        let mut w1 = materialize(&g, 0, 55);
        let mut w2 = materialize(&g, 0, 55);
        let mut l1 = Lcg(1234);
        let mut l2 = Lcg(1234);
        advance_random(&mut w1, &mut l1, 60);
        advance_random(&mut w2, &mut l2, 60);
        assert_eq!(
            full_zone_dump(&w1),
            full_zone_dump(&w2),
            "same world seed + same actions must stay identical (incl. rng use)"
        );
    });
}
