//! Closed-list belief boundary tests. No real opponent deck is supplied to the provider.
use std::collections::BTreeSet;

use digimon_engine::determinize::beliefs::{
    BeliefEvidence, CandidateDeck, InformationRegime, KnowledgeLedger, KnowledgeZone,
    PriorSnapshot, WeightedDeckPrior,
};

use super::support::{self, D1};

fn cards() -> Vec<String> {
    D1.iter().map(|s| s.to_string()).collect()
}

fn snapshot() -> PriorSnapshot {
    PriorSnapshot::new(
        "standard",
        "repository-test-fixture",
        cards().into_iter().collect::<BTreeSet<_>>(),
        vec![CandidateDeck {
            cards: cards(),
            weight: 1.0,
        }],
        0.1,
        64,
    )
    .unwrap()
}

fn evidence() -> BeliefEvidence {
    BeliefEvidence::from_counts(0, 1, 5, 40, 5, 4, Default::default()).unwrap()
}

#[test]
fn regimes_cannot_be_confused_in_metadata() {
    assert_eq!(
        serde_json::to_string(&InformationRegime::ClosedDecklist).unwrap(),
        "\"closed_decklist\""
    );
    assert_eq!(
        serde_json::to_string(&InformationRegime::OpenDecklist).unwrap(),
        "\"open_decklist\""
    );
}

#[test]
fn sampler_is_seeded_and_never_changes_total_copies() {
    support::with_big_stack(|| {
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        let a = prior.sample(&evidence(), 42).unwrap();
        let b = prior.sample(&evidence(), 42).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.hidden_main.values().sum::<usize>(), 50);
        assert_eq!(a.hidden_eggs.values().sum::<usize>(), 4);
        assert!(digimon_engine::deck_tools::validate_deck(&a.deck).is_valid);
    });
}

#[test]
fn publicly_exhausted_copies_never_reappear_hidden() {
    support::with_big_stack(|| {
        let mut e = evidence();
        e.accounted.insert("ST1-02".into(), 4);
        e.deck_count -= 4;
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        for seed in 0..32 {
            let sample = prior.sample(&e, seed).unwrap();
            assert!(!sample.hidden_main.contains_key("ST1-02"));
        }
    });
}

#[test]
fn invalid_and_unknown_candidate_cards_are_rejected() {
    support::with_big_stack(|| {
        let mut s = snapshot();
        s.candidates[0].cards[4] = "UNKNOWN-001".into();
        assert!(WeightedDeckPrior::new(s).is_err());
        let mut s = snapshot();
        // Five ST1-02s, retaining deck size.
        s.candidates[0].cards[8] = "ST1-02".into();
        assert!(WeightedDeckPrior::new(s).is_err());
    });
}

#[test]
fn restricted_and_combination_banned_candidates_use_existing_validator() {
    support::with_big_stack(|| {
        let mut s = snapshot();
        // These IDs come from the compiled format policy, not a second banlist.
        let fmt = digimon_engine::format::descriptor("standard").unwrap();
        let (id, _) = fmt
            .restriction
            .card_limits
            .iter()
            .find(|(_, n)| **n <= 1)
            .unwrap();
        s.population.insert(id.clone());
        for card in s.candidates[0].cards.iter_mut().skip(4).take(2) {
            *card = id.clone();
        }
        assert!(WeightedDeckPrior::new(s).is_err());

        let mut s = snapshot();
        let (a, b) = &fmt.restriction.choice_groups[0];
        s.population.insert(a[0].clone());
        s.population.insert(b[0].clone());
        s.candidates[0].cards[4] = a[0].clone();
        s.candidates[0].cards[5] = b[0].clone();
        assert!(WeightedDeckPrior::new(s).is_err());
    });
}

#[test]
fn snapshots_are_content_addressed_and_duplicate_sources_do_not_gain_mass() {
    support::with_big_stack(|| {
        let s = snapshot();
        let original = WeightedDeckPrior::new(s.clone()).unwrap();
        let mut duplicate = s.clone();
        duplicate.candidates.push(s.candidates[0].clone());
        let duplicate = WeightedDeckPrior::new(duplicate).unwrap();
        assert_eq!(original.snapshot_hash(), duplicate.snapshot_hash());
        assert_eq!(original.candidate_count(), 1);
        let mut conflicting = s.clone();
        conflicting.candidates.push(CandidateDeck {
            cards: cards(),
            weight: 2.0,
        });
        assert!(WeightedDeckPrior::new(conflicting).is_err());
        let mut reordered = s;
        reordered.candidates[0].cards.reverse();
        assert_eq!(
            original.snapshot_hash(),
            WeightedDeckPrior::new(reordered).unwrap().snapshot_hash()
        );
    });
}

#[test]
fn impossible_evidence_is_an_error_instead_of_true_deck_fallback() {
    support::with_big_stack(|| {
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        let mut e = evidence();
        e.accounted.insert("NOT-IN-POPULATION".into(), 1);
        e.deck_count -= 1;
        assert!(prior.sample(&e, 0).is_err());
        let mut e = evidence();
        e.accounted.insert("ST1-02".into(), 5);
        e.deck_count -= 5;
        assert!(prior.sample(&e, 0).is_err());
    });
}

#[test]
fn known_hidden_cards_are_subtracted_once_and_retained_in_hidden_pool() {
    support::with_big_stack(|| {
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        let mut e = evidence();
        e.known_hidden.push(("ST1-02".into(), KnowledgeZone::Hand));
        let sample = prior.sample(&e, 5).unwrap();
        assert!(sample.hidden_main["ST1-02"] >= 1);
        assert_eq!(sample.hidden_main.values().sum::<usize>(), 50);
        // Known hand identity is not also a current public card.
        e.accounted.insert("ST1-02".into(), 4);
        e.deck_count -= 4;
        assert!(prior.sample(&e, 5).is_err());
    });
}

#[test]
fn unseen_mass_is_preserved_even_with_compatible_candidates() {
    support::with_big_stack(|| {
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        let posterior = prior.posterior(&evidence()).unwrap();
        assert_eq!(posterior.candidates.len(), 1);
        assert!((posterior.fallback_mass - 0.1).abs() < 1e-9);
        assert!((posterior.candidates[0].1 - 0.9).abs() < 1e-9);
    });
}

#[test]
fn contradictory_candidate_uses_legal_completion_not_candidate_resurrection() {
    support::with_big_stack(|| {
        let mut s = snapshot();
        // The starter list contains only two ST1-08; a third rules it out.
        s.fallback_attempts = 256;
        let prior = WeightedDeckPrior::new(s).unwrap();
        let mut e = evidence();
        e.accounted.insert("ST1-08".into(), 3);
        e.deck_count -= 3;
        let posterior = prior.posterior(&e).unwrap();
        assert!(posterior.candidates.is_empty());
        assert_eq!(posterior.fallback_mass, 1.0);
        let sample = prior.sample(&e, 21).unwrap();
        assert!(sample.used_fallback);
        assert!(
            sample
                .deck
                .iter()
                .filter(|c| c.as_str() == "ST1-08")
                .count()
                >= 3
        );
        assert!(digimon_engine::deck_tools::validate_deck(&sample.deck).is_valid);
    });
}

#[test]
fn snapshot_tampering_and_invalid_probability_are_rejected() {
    support::with_big_stack(|| {
        let mut s = snapshot();
        s.legality_hash = "stale".into();
        assert!(WeightedDeckPrior::new(s).is_err());
        let mut s = snapshot();
        s.unseen_mass = 0.0;
        assert!(WeightedDeckPrior::new(s).is_err());
        let mut s = snapshot();
        s.candidates[0].weight = f64::NAN;
        assert!(WeightedDeckPrior::new(s).is_err());
    });
}

#[test]
fn ledger_tracks_identity_without_counting_repeated_sightings() {
    let mut k = KnowledgeLedger::new(0).unwrap();
    k.observe(1, 10, "ST1-02", KnowledgeZone::Public, None)
        .unwrap();
    k.observe(1, 10, "ST1-02", KnowledgeZone::Public, None)
        .unwrap();
    k.move_known(1, 10, KnowledgeZone::Hand, None).unwrap();
    assert_eq!(
        k.known_hidden(1),
        vec![("ST1-02".into(), KnowledgeZone::Hand)]
    );
    k.move_known(1, 10, KnowledgeZone::Public, None).unwrap();
    assert!(k.known_hidden(1).is_empty());
}

#[test]
fn private_peeks_are_seat_local_and_shuffle_forgets_assignments() {
    let mut a = KnowledgeLedger::new(0).unwrap();
    let b = KnowledgeLedger::new(1).unwrap();
    a.observe(1, 12, "ST1-03", KnowledgeZone::Deck, Some(0))
        .unwrap();
    assert!(b.known_hidden(1).is_empty());
    assert_eq!(
        a.known_positions(1),
        vec![("ST1-03".into(), KnowledgeZone::Deck, 0)]
    );
    a.shuffle(1, KnowledgeZone::Deck);
    assert!(a.known_positions(1).is_empty());
    assert_eq!(
        a.known_hidden(1),
        vec![("ST1-03".into(), KnowledgeZone::Deck)]
    );
    let restored: KnowledgeLedger =
        serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
    assert_eq!(a, restored);
    a.forget_zone(1, KnowledgeZone::Deck);
    assert!(a.known_hidden(1).is_empty());
}

#[test]
fn evidence_extraction_does_not_read_opponent_composition() {
    support::with_big_stack(|| {
        let a = support::new_game_with(10, D1, D1);
        let b = support::new_game_with(10, D1, super::support::D3);
        let k = KnowledgeLedger::new(0).unwrap();
        let ea = BeliefEvidence::extract(&a, 0, &k).unwrap();
        let eb = BeliefEvidence::extract(&b, 0, &k).unwrap();
        assert_eq!(ea, eb);
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        assert_eq!(
            prior.sample(&ea, 55).unwrap(),
            prior.sample(&eb, 55).unwrap()
        );
    });
}

#[test]
fn public_trash_and_security_pins_reduce_the_hidden_population() {
    support::with_big_stack(|| {
        let mut game = support::new_game_with(3, D1, D1);
        let mut lcg = support::Lcg(3);
        while game.mulligan_current_player().is_some() {
            support::advance_random(&mut game, &mut lcg, 1);
        }
        let c = game.players[1].hand.pop().unwrap();
        let id = c.card_id(&game.card_data).to_string();
        let mut k = KnowledgeLedger::new(0).unwrap();
        k.observe(1, c.card_index, &id, KnowledgeZone::Hand, None)
            .unwrap();
        game.players[1].trash.push(c);
        let pin = game.players[1].security[0].card_index;
        game.players[1].face_up_security.insert(pin);
        let e = BeliefEvidence::extract(&game, 0, &k).unwrap();
        assert_eq!(e.accounted.values().sum::<usize>(), 2);
        assert!(
            e.known_hidden.is_empty(),
            "stale ledger must not double-count a now-public instance"
        );
        let sample = WeightedDeckPrior::new(snapshot())
            .unwrap()
            .sample(&e, 0)
            .unwrap();
        assert_eq!(sample.hidden_main.values().sum::<usize>(), 48);
    });
}

#[test]
fn printed_copy_exception_is_not_clamped_to_four() {
    support::with_big_stack(|| {
        let s = PriorSnapshot::new(
            "standard",
            "copy-exception-test",
            BTreeSet::from(["BT11-061".to_string()]),
            vec![CandidateDeck {
                cards: vec!["BT11-061".into(); 50],
                weight: 1.0,
            }],
            0.1,
            1,
        )
        .unwrap();
        let prior = WeightedDeckPrior::new(s).unwrap();
        let e = BeliefEvidence::from_counts(0, 1, 5, 40, 5, 0, Default::default()).unwrap();
        assert_eq!(prior.sample(&e, 0).unwrap().hidden_main["BT11-061"], 50);
    });
}

#[test]
fn unavailable_legal_completion_is_bounded_and_explicit() {
    support::with_big_stack(|| {
        let mut s = snapshot();
        s.population = BTreeSet::from(["ST1-02".to_string()]);
        s.candidates.clear();
        s.fallback_attempts = 2;
        let prior = WeightedDeckPrior::new(s).unwrap();
        let e = BeliefEvidence::from_counts(0, 1, 5, 40, 5, 0, Default::default()).unwrap();
        let error = prior.sample(&e, 0).unwrap_err();
        assert!(error.contains("exhausted 2 attempts"), "{error}");
    });
}

#[test]
fn known_egg_cannot_be_dealt_into_a_main_deck_zone() {
    support::with_big_stack(|| {
        let prior = WeightedDeckPrior::new(snapshot()).unwrap();
        let mut e = evidence();
        e.known_hidden.push(("ST1-01".into(), KnowledgeZone::Hand));
        assert!(prior.sample(&e, 1).is_err());
        let mut e = evidence();
        e.known_hidden
            .push(("ST1-02".into(), KnowledgeZone::Digitama));
        assert!(prior.sample(&e, 1).is_err());
    });
}

#[test]
fn prior_round_trip_is_validated_and_reproducible() {
    support::with_big_stack(|| {
        let s = snapshot();
        let restored: PriorSnapshot =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let a = WeightedDeckPrior::new(s).unwrap();
        let b = WeightedDeckPrior::new(restored).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
        assert_eq!(
            a.sample(&evidence(), 22).unwrap(),
            b.sample(&evidence(), 22).unwrap()
        );
    });
}

#[test]
fn public_accounting_follows_card_owner_across_controller_zones() {
    support::with_big_stack(|| {
        let mut game = support::new_game_with(9, D1, D1);
        let card = game.players[1].hand.pop().unwrap();
        let id = card.card_id(&game.card_data).to_string();
        // Card effects can place an opposing-owned card in another player's
        // public area. Ownership, not the containing player, determines
        // which original deck its physical copy is accounted against.
        game.players[0].trash.push(card);
        let e = BeliefEvidence::extract(&game, 0, &KnowledgeLedger::new(0).unwrap()).unwrap();
        assert_eq!(e.accounted.get(&id), Some(&1));
        assert!(WeightedDeckPrior::new(snapshot())
            .unwrap()
            .sample(&e, 6)
            .is_ok());
    });
}
