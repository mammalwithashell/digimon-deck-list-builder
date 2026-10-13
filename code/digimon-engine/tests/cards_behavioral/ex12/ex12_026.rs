use digimon_engine::enums::{CardColor, Keyword, ModifierType};

use super::support::{
    hand_contains, hand_index, plain_digimon, select_first_non_pass, source_count, DebugRunner,
};

const CARD_ID: &str = "EX12-026";

#[test]
fn ex12_026_aquatic_compiled_registry_retains_rule_grant() {
    let registry = digimon_engine::dsl_registry::from_embedded()
        .expect("ordinary embedded card registry loads");
    let card = registry.lookup(CARD_ID).expect("Shellmon is registered");
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
fn ex12_026_aquatic_ordinary_full_r4_subjects_and_stripped_witness() {
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
    let original = database.get(CARD_ID).expect("R4 contains Shellmon").clone();
    for expected in ["Champion", "Data", "Mollusk", "Shambala", "TB", "Aquatic"] {
        assert!(
            original.traits.iter().any(|t| t == expected),
            "missing {expected}"
        );
    }
    assert!(original.effect_text.contains("Aquatic"));
    assert_ex12_026_aquatic_subjects(database.clone(), true);

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
    assert_ex12_026_aquatic_subjects(stripped, false);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "input remains immutable"
    );
    println!("EX12-026 ordinary full R4: rows=4483 sha256={R4_SHA}; native and Aquatic-only stripped subjects/zone filters passed");
}

fn assert_ex12_026_aquatic_subjects(
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
fn ex12_026_has_blocker_keyword() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-026 YAML loads")
        .start();

    let shellmon = runner.place_on_field(0, CARD_ID, Some(0));

    assert!(
        runner.game.has_keyword(shellmon, Keyword::Blocker),
        "Shellmon has printed <Blocker>"
    );
}

#[test]
fn ex12_026_on_play_trashes_bottom_two_sources_then_locks_attack_and_block() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-026 YAML loads")
        .add_card(plain_digimon("SRC-A", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("SRC-B", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("OPP-TOP", CardColor::Blue, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP-TOP"]);

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-026");
    select_first_non_pass(&mut runner);
    assert_eq!(
        source_count(&runner, 1, opp.index as usize),
        0,
        "bottom two sources are trashed"
    );

    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish lock");

    assert!(
        runner.modifiers().has(opp, ModifierType::CannotAttack),
        "target cannot attack"
    );
    assert!(
        runner.modifiers().has(opp, ModifierType::CannotBlock),
        "target cannot block"
    );
}

#[test]
fn ex12_026_inherited_when_attacking_draws_if_hand_has_7_or_fewer_cards() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-026 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("DRAW", CardColor::Blue, 3, 3000))
        .add_card(plain_digimon("SEC", CardColor::Blue, 3, 3000))
        .deck(0, &["DRAW"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let hand_before = runner.game.players[0].hand.len();

    runner.attack_player(carrier, 1, false);
    runner.auto_resolve().expect("finish inherited draw");

    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
    assert!(hand_contains(&runner, 0, "DRAW"));
}
