//! Task 1.3 — `SamplingRevealSource`: a seeded uniform sampler over a
//! multiset implementing `opaque_deck::RevealSource`.

use digimon_engine::determinize::{CardMultiset, SamplingRevealSource};
use digimon_engine::opaque_deck::{RevealKind, RevealSource};

fn ms(pairs: &[(&str, usize)]) -> CardMultiset {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

#[test]
fn draws_exactly_the_multiset_then_exhausts() {
    let mut s = SamplingRevealSource::new(&ms(&[("BT1-001", 2), ("BT1-010", 1)]), 7);
    assert_eq!(s.remaining(), 3);
    let mut drawn = CardMultiset::new();
    for _ in 0..3 {
        let id = s.next_reveal(RevealKind::Draw).expect("has cards");
        *drawn.entry(id).or_insert(0) += 1;
    }
    assert_eq!(drawn, ms(&[("BT1-001", 2), ("BT1-010", 1)]));
    assert_eq!(s.remaining(), 0);
    let err = s.next_reveal(RevealKind::Draw).unwrap_err();
    assert_eq!(err.requested_kind, RevealKind::Draw);
}

#[test]
fn deterministic_per_seed() {
    // 20 distinct ids so sequence collisions across seeds are unlikely.
    let pool: CardMultiset = (0..20).map(|i| (format!("C-{i:02}"), 1)).collect();
    let drain = |seed: u64| -> Vec<String> {
        let mut s = SamplingRevealSource::new(&pool, seed);
        (0..20)
            .map(|_| s.next_reveal(RevealKind::Draw).unwrap())
            .collect()
    };
    assert_eq!(drain(42), drain(42), "same seed must replay the same draws");
    assert_ne!(
        drain(42),
        drain(43),
        "different seeds should diverge on a 20-card pool"
    );
}

#[test]
fn clone_box_replays_identical_future_draws() {
    // D8: a verbatim clone shares RNG state — identical future draws is the
    // intended same-world-lookahead semantic.
    let pool: CardMultiset = (0..15).map(|i| (format!("C-{i:02}"), 1)).collect();
    let mut a = SamplingRevealSource::new(&pool, 9);
    // Advance a couple of draws first, then fork.
    a.next_reveal(RevealKind::Draw).unwrap();
    a.next_reveal(RevealKind::Security).unwrap();
    let mut b = a.clone_box();
    let rest_a: Vec<String> = (0..13)
        .map(|_| a.next_reveal(RevealKind::Draw).unwrap())
        .collect();
    let rest_b: Vec<String> = (0..13)
        .map(|_| b.next_reveal(RevealKind::Draw).unwrap())
        .collect();
    assert_eq!(rest_a, rest_b, "cloned source must replay identical draws");
}

#[test]
fn first_draw_roughly_uniform_across_seeds() {
    let pool = ms(&[("A", 1), ("B", 1)]);
    let mut a_first = 0;
    for seed in 0..200 {
        let mut s = SamplingRevealSource::new(&pool, seed);
        if s.next_reveal(RevealKind::Draw).unwrap() == "A" {
            a_first += 1;
        }
    }
    assert!(
        (60..=140).contains(&a_first),
        "first-draw split badly skewed: A first in {a_first}/200 seeds"
    );
}
