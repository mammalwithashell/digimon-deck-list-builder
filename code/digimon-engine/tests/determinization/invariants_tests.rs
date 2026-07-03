//! Task 1.5 — the sampler invariant checker: counts balance, copy limits,
//! pins honored, viewer-visible state identical, and BOTH `OpaqueDeckState`
//! books reconciled (`total_remaining` vs the per-card multiset, which
//! placeholder accounting deliberately desyncs).

use crate::support::*;
use digimon_engine::determinize::{check_world_invariants, materialize, reconcile_opaque_books};
use digimon_engine::opaque_deck::OpaqueDeckState;

#[test]
fn opaque_books_reconcile_through_placeholder_lifecycle() {
    let deck: Vec<String> = ["A", "A", "B", "C", "C", "C"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut s = OpaqueDeckState::from_decklist(&deck);
    reconcile_opaque_books(&s, 0).expect("fresh state balances");

    // reserve_placeholders debits total_remaining WITHOUT touching the
    // per-card multiset — the books are reconciled only via the outstanding
    // placeholder count.
    s.reserve_placeholders(2);
    reconcile_opaque_books(&s, 2).expect("2 outstanding placeholders balance");
    assert!(
        reconcile_opaque_books(&s, 0).is_err(),
        "missing placeholder count must be caught as a book desync"
    );

    // Materializing one placeholder debits the per-card book only.
    s.consume_per_card_only("A").expect("A available");
    reconcile_opaque_books(&s, 1).expect("1 outstanding placeholder balances");

    // A normal consume debits both books.
    s.consume("B").expect("B available");
    reconcile_opaque_books(&s, 1).expect("still balanced");

    // restore() never errors — restoring an id keeps both books in step,
    // even for an id that was never in the decklist (the documented silent
    // composition-divergence path; the decklist ≤ check catches it later).
    s.restore("ZZ-999");
    reconcile_opaque_books(&s, 1).expect("restore moves both books together");
}

#[test]
fn checker_accepts_untampered_worlds_and_catches_tampering() {
    with_big_stack(|| {
        let mut g = new_game(1212);
        let mut lcg = Lcg(1212);
        // Past mulligan so security exists, then a few more decisions.
        advance_random(&mut g, &mut lcg, 12);
        assert!(!g.game_over);

        // Pin a face-up security card so the pin check is exercised.
        if !g.players[1].security.is_empty() {
            let ci = g.players[1].security[0].card_index;
            g.players[1].face_up_security.insert(ci);
        }

        let w0 = materialize(&g, 0, 3);
        check_world_invariants(&g, 0, &w0).expect("untampered world passes");

        // (a) Zone-count violation: move a world opp deck card into the hand.
        let mut w = w0.clone();
        let c = w.players[1].deck.pop().expect("opp deck non-empty");
        w.players[1].hand.push(c);
        assert!(
            check_world_invariants(&g, 0, &w).is_err(),
            "count drift must be caught"
        );

        // (b) Copy/conservation violation: overwrite a hidden opp deck card
        // with a duplicate of another (multiset drifts from the source).
        let mut w = w0.clone();
        {
            assert!(w.players[1].deck.len() >= 2);
            // Pick a card whose id differs from deck[0], then overwrite it
            // with a duplicate of deck[0].
            let ids: Vec<String> = zone_ids(&w.players[1].deck, &w);
            let tgt = ids
                .iter()
                .position(|id| *id != ids[0])
                .expect("opp deck holds at least two distinct ids");
            let dup = w.players[1].deck[0].clone();
            w.players[1].deck[tgt] = dup;
        }
        assert!(
            check_world_invariants(&g, 0, &w).is_err(),
            "conservation drift must be caught"
        );

        // (c) Pin violation: swap the pinned face-up security card with a
        // deck card.
        if !g.players[1].security.is_empty() {
            let mut w = w0.clone();
            {
                let p = &mut w.players[1];
                let (sec, deck) = (&mut p.security, &mut p.deck);
                std::mem::swap(&mut sec[0], &mut deck[0]);
            }
            assert!(
                check_world_invariants(&g, 0, &w).is_err(),
                "moved pin must be caught"
            );
        }

        // (d) Viewer-visible violation: tamper with the viewer's own hand.
        let mut w = w0.clone();
        {
            let p = &mut w.players[0];
            let c = p.hand.pop().expect("viewer has a hand");
            p.deck.push(c);
        }
        assert!(
            check_world_invariants(&g, 0, &w).is_err(),
            "viewer hand tampering must be caught"
        );

        // (e) A lingering reveal source must be rejected.
        let mut w = w0.clone();
        w.reveal_source = Some(Box::new(digimon_engine::opaque_deck::RevealQueue::new()));
        assert!(
            check_world_invariants(&g, 0, &w).is_err(),
            "worlds must not carry a live reveal source"
        );
    });
}
