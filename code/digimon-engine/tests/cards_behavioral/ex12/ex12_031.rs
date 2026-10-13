use digimon_engine::enums::CardColor;

use super::support::{
    bottom_source_id, field_contains, hand_contains, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, source_count, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-031";

#[test]
fn ex12_031_aquatic_compiled_registry_retains_rule_grant() {
    let registry = digimon_engine::dsl_registry::from_embedded()
        .expect("ordinary embedded card registry loads");
    let card = registry.lookup(CARD_ID).expect("MarineBullmon is registered");
    let traits = card.all_traits();
    for expected in ["Mollusk", "Shambala", "TB", "Data", "Aquatic"] {
        assert!(
            traits
                .iter()
                .any(|actual| actual.eq_ignore_ascii_case(expected)),
            "{CARD_ID}: expected {expected} in ordinary compiled traits {traits:?}"
        );
    }
}

/// Explicit source-profile diagnostic; the default baked database has a separate
/// producer/adoption gate. Never replace its card rows with DSL fixture metadata.
#[test]
#[ignore = "requires explicitly supplied source-reviewed full cards JSON"]
fn ex12_031_aquatic_ordinary_full_r4_subjects_and_stripped_witness() {
    use digimon_engine::card_data::CardData;
    use sha2::{Digest, Sha256};

    const R4_SHA: &str = "ba7d7f85da1553fb35435131181c598ef555acd0f0890d979a74beee7e60398f";
    let path = std::env::var("DIGIMON_AQUATIC_CARDS_JSON")
        .expect("set DIGIMON_AQUATIC_CARDS_JSON to the reviewed full R4 cards.json");
    let bytes = std::fs::read(&path).expect("read explicitly supplied R4 cards.json");
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), R4_SHA);
    let database = CardData::load_from_file(std::path::Path::new(&path))
        .expect("load the complete R4 database through the ordinary CardData loader");
    assert_eq!(database.len(), 4483);
    let original = database.get(CARD_ID).expect("R4 contains MarineBullmon").clone();
    for expected in ["Ultimate", "Data", "Mollusk", "Shambala", "TB", "Aquatic"] {
        assert!(
            original.traits.iter().any(|t| t == expected),
            "missing {expected}"
        );
    }
    assert!(original.effect_text.contains("Aquatic"));
    assert_ex12_031_aquatic_subjects(database.clone(), true);

    // The discriminator retains TB and the Rule's printed text. Remove only
    // Aquatic from a private database clone; registry/effects/other rows stay real.
    let mut stripped = database.clone();
    stripped
        .get_mut(CARD_ID)
        .unwrap()
        .traits
        .retain(|t| t != "Aquatic");
    let mut expected = original;
    expected.traits.retain(|t| t != "Aquatic");
    assert_eq!(stripped.get(CARD_ID).unwrap(), &expected);
    for (id, data) in &database {
        if id != CARD_ID {
            assert_eq!(stripped.get(id).unwrap(), data);
        }
    }
    assert_ex12_031_aquatic_subjects(stripped, false);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "input remains immutable"
    );
    println!("EX12-031 ordinary full R4: rows=4483 sha256={R4_SHA}; native and Aquatic-only stripped subjects/zone filters passed");
}

// Direct zone fixtures test trait predicates and shared searches, not legal gameplay.
fn assert_ex12_031_aquatic_subjects(
    database: std::collections::HashMap<String, digimon_engine::card_data::CardData>,
    aquatic: bool,
) {
    use digimon_dsl::compiled::{
        CompiledCountAggregate, CompiledDpConstraint, CompiledPredicate, CompiledZone,
    };
    use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
    use digimon_engine::effect_context::EffectReadContext;
    use digimon_engine::selection::SourceSelectionRef;

    const CONTROL: &str = "BT1-010"; // real non-Aquatic Agumon
    assert!(database[CONTROL]
        .traits
        .iter()
        .all(|t| !t.to_lowercase().contains("aqua")));
    let mut runner = DebugRunner::builder()
        .with_card_data(database)
        .hand(0, &[CARD_ID, CARD_ID, CARD_ID, CONTROL])
        .deck(0, &[CARD_ID, CONTROL])
        .security(0, &[CARD_ID, CONTROL])
        .build();
    assert!(runner
        .game
        .effect_registry
        .registered_card_ids()
        .iter()
        .any(|id| id == CARD_ID));
    let trash = runner.game.players[0].hand.remove(1);
    runner.game.players[0].trash.push(trash);
    let revealed = runner.game.players[0].hand.remove(1);
    runner.game.revealed_cards.push(revealed);
    runner.place_in_breeding(0, CARD_ID);
    let field = runner.place_on_field(0, CARD_ID, Some(0));
    let stack = runner.place_stack(0, &[CARD_ID, CONTROL]);
    let source = runner.game.players[0].battle_area[stack.index as usize].card_sources[0].handle();
    let player = &runner.game.players[0];
    let rctx = EffectReadContext::new(&runner.game, player.hand[0].handle(), None, 0);
    let subjects = [
        ("hand", PredicateSubject::Card(player.hand[0].handle())),
        ("deck", PredicateSubject::Card(player.deck[0].handle())),
        ("trash", PredicateSubject::Card(player.trash[0].handle())),
        (
            "security",
            PredicateSubject::Card(player.security[0].handle()),
        ),
        (
            "reveal",
            PredicateSubject::RevealedCard(runner.game.revealed_cards[0].handle()),
        ),
        ("battle", PredicateSubject::Permanent(field)),
        ("breeding", PredicateSubject::BreedingPermanent(0)),
        (
            "source",
            PredicateSubject::Source(SourceSelectionRef {
                permanent: stack,
                field_index: stack.index,
                source_index: 0,
                card: source,
            }),
        ),
    ];
    let exact = CompiledPredicate {
        trait_has: Some("Aquatic".into()),
        ..Default::default()
    };
    let substring = CompiledPredicate {
        trait_contains: Some("Aqua".into()),
        ..Default::default()
    };
    // Each probe stands alone; a TB alternative would accept both witnesses.
    for (label, subject) in subjects {
        for (name, predicate) in [("exact Aquatic", &exact), ("substring Aqua", &substring)] {
            assert_eq!(
                eval_predicate(predicate, &rctx, subject),
                aquatic,
                "{label}: {name}"
            );
        }
        for absent in ["Aqua", "Sea Animal"] {
            let predicate = CompiledPredicate {
                trait_has: Some(absent.into()),
                ..Default::default()
            };
            assert!(
                !eval_predicate(&predicate, &rctx, subject),
                "{label}: exact {absent}"
            );
        }
        let tb = CompiledPredicate {
            trait_has: Some("TB".into()),
            ..Default::default()
        };
        assert!(
            eval_predicate(&tb, &rctx, subject),
            "{label}: stripped witness retains TB"
        );
    }
    let control = PredicateSubject::Card(player.hand[1].handle());
    assert!(!eval_predicate(&exact, &rctx, control));
    assert!(!eval_predicate(&substring, &rctx, control));

    // Exercise the shared zone-search/filter path, in addition to direct subjects.
    for zone in [
        CompiledZone::Hand,
        CompiledZone::Deck,
        CompiledZone::Trash,
        CompiledZone::Security,
        CompiledZone::Reveal,
        CompiledZone::BattleArea,
        CompiledZone::Breeding,
    ] {
        for predicate in [&exact, &substring] {
            let mut filter = predicate.clone();
            filter.zone = vec![zone];
            filter.card_number_is = Some(CARD_ID.into());
            let aggregate = CompiledPredicate {
                count_gte: Some(CompiledCountAggregate {
                    filter: Box::new(filter),
                    n: CompiledDpConstraint::Literal(1),
                }),
                ..Default::default()
            };
            assert_eq!(
                eval_predicate(&aggregate, &rctx, PredicateSubject::None),
                aquatic,
                "{zone:?}: shared zone filter {predicate:?}"
            );
        }
    }
}

#[test]
fn ex12_031_on_play_places_matching_hand_card_as_bottom_source_then_bounces_low_source_digimon() {
    let aqua = super::support::digimon("AQUATIC-HAND", CardColor::Blue, 4, 4, 5000, &["Aquatic"]);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(aqua)
        .add_card(plain_digimon("OPP-SRC", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID, "AQUATIC-HAND"])
        .memory(10)
        .start();
    runner.place_stack(1, &["OPP-SRC", "OPP-TOP"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-031");
    select_hand_card(&mut runner, 0, "AQUATIC-HAND");
    assert_eq!(bottom_source_id(&runner, 0, 0), "AQUATIC-HAND");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish bounce");

    assert_eq!(source_count(&runner, 0, 0), 1);
    assert!(hand_contains(&runner, 1, "OPP-TOP"));
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn ex12_031_decode_plays_matching_tb_source_on_non_battle_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(tb_digimon("TB-SRC", CardColor::Blue, 4, 5000))
        .memory(10)
        .start();
    let marine_bullmon = runner.place_stack(0, &["TB-SRC", CARD_ID]);

    runner.game.return_to_hand_from_effect(marine_bullmon, 1);
    select_first_non_pass(&mut runner);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish Decode");

    assert!(
        field_contains(&runner, 0, "TB-SRC"),
        "Decode plays the matching [TB] source"
    );
    assert_eq!(
        runner.hand_size(0),
        1,
        "the original non-battle leave still returns EX12-031 to hand"
    );
}

/// TDD RED — task_69f10a66 ruling, EX12-031#effect#1 sub-finding: `<Decode>`
/// is SELF-scoped. Rule 16-35-1: "When the Digimon WITH THIS EFFECT would
/// leave the battle area other than by a battle, you may play 1 Digimon card
/// specified by this effect FROM THAT DIGIMON'S digivolution cards" —
/// another permanent leaving is not this Digimon's trigger. DCGO gates the
/// same way (`Decode`'s CanUse/CanActivate are keyed to the carrier) and
/// asks nothing when a different Digimon is bounced.
///
/// Today this FAILS: `keyword_to_auto_effect(Keyword::Decode)` synthesizes a
/// legacy "<Decode> (hand)"/"(deck)" redirect pair whose self-scope check
/// lives only INSIDE the post-accept process — `collect_candidates` has no
/// `replacement_condition` to filter it, so bouncing ANY permanent while a
/// Decode carrier sits on either field parks a vacuous
/// "May accept replacement: <Decode> (hand)" prompt — handed to the LEAVING
/// permanent's controller, not even the keyword carrier's. Exam witness:
/// qa/dcgo-exams/EX12/EX12-031-effect1.yaml step 9 (our engine asks, DCGO
/// does not — the scripted line desyncs at the spurious gate).
///
/// The carrier here is a synthetic card with `keywords = [Decode]` because
/// that is what PRODUCTION card data holds for EX12-031: `card_data.rs`
/// parses the printed "<Decode ...>" text into `CardData.keywords`, and
/// `build_effects_for_card` then synthesizes the auto-effect pair. The
/// DebugRunner's YAML-derived CardData carries no text-parsed keywords
/// (only `grant_keyword` clauses), which would silently mask the prod bug.
#[test]
fn ex12_031_decode_does_not_open_a_window_for_another_permanents_leave() {
    use digimon_engine::enums::Keyword;
    let decode_carrier = {
        let mut c = plain_digimon("DECODE-CARRIER", CardColor::Blue, 5, 7000);
        // Prod parity: EX12-031's cards.json text "<Decode (...)>" parses to
        // Keyword::Decode in CardData.keywords.
        c.keywords = vec![Keyword::Decode];
        c
    };
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-031 YAML loads")
        .add_card(decode_carrier)
        .add_card(plain_digimon("OPP-BOUNCE", CardColor::Blue, 3, 3000))
        .memory(10)
        .start();
    // Decode carrier on P0's field; a plain Digimon (no Decode) on P1's.
    let _carrier = runner.place_on_field(0, "DECODE-CARRIER", Some(0));
    let opp = runner.place_on_field(1, "OPP-BOUNCE", Some(0));

    // P0's effect bounces P1's Digimon — the exam line's shape.
    runner.game.return_to_hand_from_effect(opp, 0);

    assert!(
        runner.game.pending_selection.is_none(),
        "no replacement window may open: <Decode> is self-scoped (16-35-1); \
         the leaving Digimon has no Decode and MarineBullmon is not leaving \
         (got: {:?})",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|s| s.prompt.clone())
    );
    assert!(
        hand_contains(&runner, 1, "OPP-BOUNCE"),
        "the bounce itself must have committed"
    );
}
