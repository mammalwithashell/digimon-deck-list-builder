//! Tasks 1.1 + 1.2 — `Infoset` extraction: the inverse of
//! `server/state_filter.py`'s redaction. Never exposes hidden information;
//! the viewer's own hidden model is one JOINT unseen pool (own deck ∪ own
//! face-down security); face-up security is pinned/known.

use crate::support::*;
use digimon_engine::determinize::{DeckPrior, Infoset};

/// Advance through both mulligan decisions so security is dealt.
fn game_past_mulligan(seed: u64) -> digimon_engine::game::Game {
    let mut g = new_game(seed);
    let mut lcg = Lcg(seed ^ 0xABCD);
    while g.mulligan_current_player().is_some() {
        advance_random(&mut g, &mut lcg, 1);
    }
    assert_eq!(g.players[0].security.len(), 5, "security dealt after mulligan");
    g
}

#[test]
fn infoset_invariant_to_opponent_concealed_positions() {
    with_big_stack(|| {
        let mut g = game_past_mulligan(101);
        let before = Infoset::extract(&g, 0);

        // Swap an opponent hand card with an opponent deck card that has a
        // DIFFERENT id — a pure hidden-to-hidden move. The infoset for the
        // viewer must not change (it may not know which concealed card sits
        // where).
        let (hi, di) = {
            let p = &g.players[1];
            let mut found = None;
            'outer: for (hi, hc) in p.hand.iter().enumerate() {
                for (di, dc) in p.deck.iter().enumerate() {
                    if hc.card_id(&g.card_data) != dc.card_id(&g.card_data) {
                        found = Some((hi, di));
                        break 'outer;
                    }
                }
            }
            found.expect("opponent hand + deck contain at least two distinct ids")
        };
        {
            let p = &mut g.players[1];
            let (hand, deck) = (&mut p.hand, &mut p.deck);
            std::mem::swap(&mut hand[hi], &mut deck[di]);
        }
        let after = Infoset::extract(&g, 0);
        assert_eq!(before, after, "hidden-to-hidden swap leaked into the infoset");

        // A PUBLIC change (hand card into trash) must change the infoset.
        {
            let p = &mut g.players[1];
            let c = p.hand.pop().expect("opponent has a hand");
            p.trash.push(c);
        }
        let public_change = Infoset::extract(&g, 0);
        assert_ne!(before, public_change, "public trash change must be observable");
    });
}

#[test]
fn opponent_model_is_counts_plus_exact_prior() {
    with_big_stack(|| {
        let g = game_past_mulligan(202);
        let info = Infoset::extract(&g, 0);
        let opp = &g.players[1];

        assert_eq!(info.opponent_model.hand_count, opp.hand.len());
        assert_eq!(info.opponent_model.deck_count, opp.deck.len());
        assert_eq!(
            info.opponent_model.face_down_security_count,
            opp.security.len(),
            "all security is face-down at setup"
        );

        // Self-play regime: the prior is the exact unseen multiset —
        // hand ∪ deck ∪ face-down security.
        let mut expected = zone_multiset(&opp.hand, &g);
        for (id, n) in zone_multiset(&opp.deck, &g) {
            *expected.entry(id).or_insert(0) += n;
        }
        for (id, n) in zone_multiset(&opp.security, &g) {
            *expected.entry(id).or_insert(0) += n;
        }
        let DeckPrior::Exact(prior) = &info.opponent_model.deck_prior;
        assert_eq!(prior, &expected);
        assert_eq!(
            prior.values().sum::<usize>(),
            opp.hand.len() + opp.deck.len() + opp.security.len()
        );

        // Egg pool: composition of the remaining digitama deck.
        assert_eq!(
            info.opponent_model.digitama_pool,
            zone_multiset(&opp.digitama_deck, &g)
        );
    });
}

#[test]
fn own_hidden_model_is_joint_pool_with_partition() {
    with_big_stack(|| {
        let g = game_past_mulligan(303);
        let info = Infoset::extract(&g, 0);
        let me = &g.players[0];

        // Own hand: exact ids in order.
        assert_eq!(info.own.hand, zone_ids(&me.hand, &g));

        // Own unseen pool = deck ∪ face-down security (own security is
        // hidden from its owner too — D2 sharpening).
        let mut expected = zone_multiset(&me.deck, &g);
        for (id, n) in zone_multiset(&me.security, &g) {
            *expected.entry(id).or_insert(0) += n;
        }
        assert_eq!(info.own.unseen_pool, expected);

        // Count partition: how many of the pool sit in deck vs face-down
        // security.
        assert_eq!(info.own.deck_count, me.deck.len());
        assert_eq!(info.own.face_down_security_count, me.security.len());
        assert_eq!(
            info.own.unseen_pool.values().sum::<usize>(),
            info.own.deck_count + info.own.face_down_security_count
        );

        assert_eq!(info.own.digitama_pool, zone_multiset(&me.digitama_deck, &g));
    });
}

#[test]
fn face_up_security_is_pinned_and_excluded_from_hidden_pools() {
    with_big_stack(|| {
        let mut g = game_past_mulligan(404);

        // Flip one OPPONENT security card face-up (as an effect would).
        let flip_idx = 2;
        let (flip_ci, flip_id) = {
            let c = &g.players[1].security[flip_idx];
            (c.card_index, c.card_id(&g.card_data).to_string())
        };
        g.players[1].face_up_security.insert(flip_ci);

        // And one of the VIEWER's own security cards.
        let own_idx = 1;
        let (own_ci, own_id) = {
            let c = &g.players[0].security[own_idx];
            (c.card_index, c.card_id(&g.card_data).to_string())
        };
        g.players[0].face_up_security.insert(own_ci);

        let info = Infoset::extract(&g, 0);

        // Opponent pin: known card at a known slot, excluded from the prior.
        assert_eq!(
            info.public[1].security_pins,
            vec![(flip_idx, flip_id.clone())]
        );
        assert_eq!(
            info.opponent_model.face_down_security_count,
            g.players[1].security.len() - 1
        );
        let DeckPrior::Exact(prior) = &info.opponent_model.deck_prior;
        let mut full = zone_multiset(&g.players[1].hand, &g);
        for (id, n) in zone_multiset(&g.players[1].deck, &g) {
            *full.entry(id).or_insert(0) += n;
        }
        for (id, n) in zone_multiset(&g.players[1].security, &g) {
            *full.entry(id).or_insert(0) += n;
        }
        *full.get_mut(&flip_id).expect("flipped id present") -= 1;
        full.retain(|_, n| *n > 0);
        assert_eq!(prior, &full, "pinned card must be excluded from the prior");

        // Own pin: known to the owner, excluded from the own joint pool.
        assert_eq!(info.public[0].security_pins, vec![(own_idx, own_id.clone())]);
        assert_eq!(
            info.own.face_down_security_count,
            g.players[0].security.len() - 1
        );
        let mut own_full = zone_multiset(&g.players[0].deck, &g);
        for (id, n) in zone_multiset(&g.players[0].security, &g) {
            *own_full.entry(id).or_insert(0) += n;
        }
        *own_full.get_mut(&own_id).expect("own flipped id present") -= 1;
        own_full.retain(|_, n| *n > 0);
        assert_eq!(info.own.unseen_pool, own_full);
    });
}

#[test]
fn public_state_is_captured() {
    with_big_stack(|| {
        // Advance into the game so battle areas / trash are non-trivial.
        let mut g = new_game(505);
        let mut lcg = Lcg(505);
        advance_random(&mut g, &mut lcg, 60);
        let info = Infoset::extract(&g, 0);

        assert_eq!(info.viewer, 0);
        assert_eq!(info.turn_count, g.turn_count);
        assert_eq!(info.current_phase, g.current_phase);
        assert_eq!(info.memory, g.memory);
        assert_eq!(info.game_over, g.game_over);
        for pid in 0..2usize {
            let p = &g.players[pid];
            assert_eq!(info.public[pid].trash, zone_ids(&p.trash, &g));
            assert_eq!(info.public[pid].battle_area.len(), p.battle_area.len());
            assert_eq!(info.public[pid].breeding.is_some(), p.breeding_area.is_some());
            assert_eq!(info.public[pid].hand_count, p.hand.len());
            assert_eq!(info.public[pid].deck_count, p.deck.len());
            assert_eq!(info.public[pid].security_count, p.security.len());
            assert_eq!(info.public[pid].digitama_count, p.digitama_deck.len());
        }
    });
}
