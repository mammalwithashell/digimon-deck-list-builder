use digimon_engine::enums::CardColor;

use super::support::{
    bottom_source_id, field_contains, hand_contains, hand_index, plain_digimon,
    select_first_non_pass, select_hand_card, source_count, tb_digimon, DebugRunner,
};

const CARD_ID: &str = "EX12-031";

// Printed-predicate audit: official EX12-031 observations (both printings),
// Q6767, and general_rule.pdf Â§Â§7-3 (p15), 16-35 (p39), read 2026-10-13.
// Synthetic level-bearing Tamer/Option cards below are grammar witnesses:
// they distinguish printed "card" from "Digimon card", without asserting
// that a corresponding real printing exists. Decode retains Digimon-only.
fn predicate_witness(
    id: &str,
    level: Option<u8>,
    kind: digimon_engine::enums::CardKind,
    traits: &[&str],
) -> digimon_engine::card_data::CardData {
    let mut card = super::support::digimon(id, CardColor::Blue, 4, 3, 4000, traits);
    card.level = level;
    card.card_kind = kind;
    card
}

fn predicate_active_seat(runner: &mut DebugRunner, seat: u8) {
    runner.game.turn_player_idx = seat as usize;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner.game.set_memory(10);
}

fn predicate_masked_action(runner: &mut DebugRunner, seat: u8, action: u16) {
    assert_eq!(
        digimon_engine::action::build_action_mask(&runner.game, seat)[action as usize],
        1.0,
        "seat {seat}: action {action} must be legal; pending={:?}",
        runner.pending_selection_view()
    );
    runner.game.decode_action(action, seat);
}

fn assert_printed_assembly_accepts(kind: digimon_engine::enums::CardKind, trait_name: &str) {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
    use digimon_engine::selection::SelectionKind;
    for seat in [0, 1] {
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .unwrap()
            .add_card(predicate_witness(
                "ASM-WITNESS",
                Some(4),
                kind,
                &[trait_name],
            ))
            .hand(seat, &[CARD_ID])
            .memory(10)
            .start();
        predicate_active_seat(&mut runner, seat);
        super::support::push_to_trash(&mut runner, seat, "ASM-WITNESS");
        predicate_masked_action(&mut runner, seat, PLAY_HAND_START);
        let view = runner
            .pending_selection_view()
            .expect("printed Assembly candidate must open material selection");
        assert_eq!(
            view.kind,
            SelectionKind::CountCappedMultiSelect {
                min: 1,
                max: 1,
                picked: 0,
                distinct: false,
            }
        );
        assert_eq!(view.selecting_player, seat);
        assert!(view.is_optional, "Assembly is optional (7-3-2-9)");
        assert_eq!(view.valid_action_ids, vec![TRASH_EFFECT_START]);
        assert_eq!(
            runner.memory(),
            10,
            "Assembly chooses before paying (7-3-2-2)"
        );
        assert_eq!(runner.battle_area_size(seat), 0);

        // A cloned pending choice must resume on its own game state.
        let mut clone = DebugRunner::wrap(runner.game.clone());
        predicate_masked_action(&mut clone, seat, TRASH_EFFECT_START);
        assert!(clone.pending_selection().is_none());
        assert_eq!(clone.memory(), 5, "Assembly pays 7 minus 2");
        assert_eq!(bottom_source_id(&clone, seat, 0), "ASM-WITNESS");
        assert_eq!(source_count(&clone, seat, 0), 1);
        assert_eq!(clone.trash_size(seat), 0);
        assert_eq!(runner.trash_size(seat), 1, "original is still parked");
        predicate_masked_action(&mut runner, seat, PASS);
        assert!(runner.pending_selection().is_none());
        assert_eq!(runner.memory(), 3, "declining Assembly pays full 7");
        assert_eq!(source_count(&runner, seat, 0), 0);
        assert_eq!(runner.trash_size(seat), 1);
    }
}

#[test]
fn ex12_031_printed_assembly_aqua_substring_only_is_selectable() {
    assert_printed_assembly_accepts(digimon_engine::enums::CardKind::Digimon, "Aquabeast");
}

#[test]
fn ex12_031_printed_assembly_sea_animal_substring_only_is_selectable() {
    assert_printed_assembly_accepts(
        digimon_engine::enums::CardKind::Digimon,
        "Ancient Sea Animal",
    );
}

#[test]
fn ex12_031_printed_assembly_exact_tb_is_selectable() {
    assert_printed_assembly_accepts(digimon_engine::enums::CardKind::Digimon, "TB");
}

#[test]
fn ex12_031_printed_assembly_level_bearing_tamer_card_is_selectable() {
    assert_printed_assembly_accepts(digimon_engine::enums::CardKind::Tamer, "TB");
}

#[test]
fn ex12_031_printed_assembly_level_bearing_option_card_is_selectable() {
    assert_printed_assembly_accepts(digimon_engine::enums::CardKind::Option, "TB");
}

#[test]
fn ex12_031_printed_assembly_rejects_wrong_level_trait_zone_and_no_level() {
    use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
    use digimon_engine::enums::CardKind;
    for seat in [0, 1] {
        let mut builder = DebugRunner::builder().dsl_card(CARD_ID).unwrap();
        let cases = [
            ("GOOD-TB", Some(4), "TB"),
            ("HIGH-AQUA", Some(5), "Aquabeast"),
            ("HIGH-SEA", Some(5), "Ancient Sea Animal"),
            ("HIGH-TB", Some(5), "TB"),
            ("NO-LEVEL", None, "TB"),
            ("WRONG-TRAIT", Some(4), "Dragon"),
            ("TB-SUBSTRING", Some(4), "TB-like"),
        ];
        for (id, level, trait_name) in cases {
            builder = builder.add_card(predicate_witness(
                id,
                level,
                CardKind::Digimon,
                &[trait_name],
            ));
        }
        let mut runner = builder
            .hand(seat, &[CARD_ID, "GOOD-TB"])
            .hand(1 - seat, &["GOOD-TB"])
            .deck(seat, &["GOOD-TB"])
            .security(seat, &["GOOD-TB"])
            .memory(10)
            .start();
        predicate_active_seat(&mut runner, seat);
        for (id, _, _) in cases {
            super::support::push_to_trash(&mut runner, seat, id);
        }
        super::support::push_to_trash(&mut runner, 1 - seat, "GOOD-TB");
        predicate_masked_action(&mut runner, seat, PLAY_HAND_START);
        let view = runner
            .pending_selection_view()
            .expect("good TB material opens Assembly");
        assert_eq!(
            view.valid_action_ids,
            vec![TRASH_EFFECT_START],
            "only own-trash level-4 exact TB qualifies"
        );
        let mask = digimon_engine::action::build_action_mask(&runner.game, seat);
        for offset in 1..cases.len() {
            assert_eq!(mask[TRASH_EFFECT_START as usize + offset], 0.0);
        }
        assert!(
            digimon_engine::action::build_action_mask(&runner.game, 1 - seat)
                .iter()
                .all(|x| *x == 0.0)
        );
    }
}

fn printed_hand_cost_runner(
    seat: u8,
    when_digivolving: bool,
    candidate: digimon_engine::card_data::CardData,
) -> DebugRunner {
    use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
    let candidate_id = candidate.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .unwrap()
        .add_card(candidate)
        .add_card(plain_digimon("BASE", CardColor::Blue, 4, 4000))
        .add_card(plain_digimon("OLD-SOURCE", CardColor::Blue, 3, 2000))
        .add_card(plain_digimon("TARGET-ZERO", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("TARGET-ONE", CardColor::Blue, 4, 5000))
        .add_card(plain_digimon("TARGET-TWO", CardColor::Blue, 4, 5000))
        .add_card(super::support::tamer("TARGET-TAMER", CardColor::Blue))
        .hand(seat, &[CARD_ID, &candidate_id])
        .deck(seat, &["OLD-SOURCE"])
        .memory(10)
        .start();
    predicate_active_seat(&mut runner, seat);
    runner.place_on_field(1 - seat, "TARGET-ZERO", Some(0));
    runner.place_stack(1 - seat, &["OLD-SOURCE", "TARGET-ONE"]);
    runner.place_stack(1 - seat, &["OLD-SOURCE", "BASE", "TARGET-TWO"]);
    runner.place_on_field(1 - seat, "TARGET-TAMER", Some(0));
    runner.place_on_field(seat, "TARGET-ZERO", Some(0)); // own Digimon never a bounce target
    if when_digivolving {
        runner.place_stack(seat, &["OLD-SOURCE", "BASE"]);
        predicate_masked_action(&mut runner, seat, encode_digivolve(0, 1));
    } else {
        predicate_masked_action(&mut runner, seat, PLAY_HAND_START);
    }
    runner
}

fn assert_printed_hand_cost_accepts(kind: digimon_engine::enums::CardKind, trait_name: &str) {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    for seat in [0, 1] {
        for when_digivolving in [false, true] {
            let mut runner = printed_hand_cost_runner(
                seat,
                when_digivolving,
                predicate_witness("HAND-WITNESS", Some(6), kind, &[trait_name]),
            );
            let view = runner
                .pending_selection_view()
                .expect("sole printed card candidate must activate OP/WD hand cost");
            assert_eq!(view.kind, SelectionKind::Hand);
            assert_eq!(view.selecting_player, seat);
            assert!(view.is_optional);
            let action = PLAY_HAND_START + hand_index(&runner, seat, "HAND-WITNESS") as u16;
            assert_eq!(view.valid_action_ids, vec![action]);
            let source_before = source_count(&runner, seat, 1);
            let memory_before = runner.memory();
            let mut clone = DebugRunner::wrap(runner.game.clone());
            predicate_masked_action(&mut clone, seat, PASS);
            assert!(
                clone.pending_selection().is_none(),
                "decline skips the bounce"
            );
            assert!(hand_contains(&clone, seat, "HAND-WITNESS"));
            assert_eq!(source_count(&clone, seat, 1), source_before);
            assert_eq!(clone.battle_area_size(1 - seat), 4);

            predicate_masked_action(&mut runner, seat, action);
            assert_eq!(bottom_source_id(&runner, seat, 1), "HAND-WITNESS");
            assert_eq!(source_count(&runner, seat, 1), source_before + 1);
            assert!(!hand_contains(&runner, seat, "HAND-WITNESS"));
            assert_eq!(
                runner.memory(),
                memory_before,
                "hand placement pays no memory"
            );
            let ids: Vec<_> = runner.game.players[seat as usize].battle_area[1]
                .card_sources
                .iter()
                .map(|s| s.card_id(&runner.game.card_data))
                .collect();
            if when_digivolving {
                assert_eq!(ids, vec!["HAND-WITNESS", "OLD-SOURCE", "BASE", CARD_ID]);
            } else {
                assert_eq!(ids, vec!["HAND-WITNESS", CARD_ID]);
            }
            let bounce = runner
                .pending_selection_view()
                .expect("paid hand cost opens bounce");
            assert_eq!(bounce.kind, SelectionKind::OppField);
            assert_eq!(bounce.selecting_player, seat);
            assert_eq!(
                bounce.valid_action_ids.len(),
                2,
                "zero/one source Digimon only; two-source and Tamer excluded"
            );
            assert!(!bounce.is_optional, "bounce after cost is mandatory");
            // Field selection order is opponent presentation order: select the
            // one-source target explicitly, keeping the zero-source target.
            predicate_masked_action(&mut runner, seat, bounce.valid_action_ids[1]);
            assert!(runner.pending_selection().is_none());
            assert!(hand_contains(&runner, 1 - seat, "TARGET-ONE"));
            assert!(super::support::trash_contains(
                &runner,
                1 - seat,
                "OLD-SOURCE"
            ));
            for id in ["TARGET-ZERO", "TARGET-TWO", "TARGET-TAMER"] {
                assert!(field_contains(&runner, 1 - seat, id));
            }
            assert!(field_contains(&runner, seat, "TARGET-ZERO"));
        }
    }
}

#[test]
fn ex12_031_printed_op_wd_aqua_substring_only_is_selectable() {
    assert_printed_hand_cost_accepts(digimon_engine::enums::CardKind::Digimon, "Aquabeast");
}

#[test]
fn ex12_031_printed_op_wd_sea_animal_substring_only_is_selectable() {
    assert_printed_hand_cost_accepts(
        digimon_engine::enums::CardKind::Digimon,
        "Ancient Sea Animal",
    );
}

#[test]
fn ex12_031_printed_op_wd_exact_tb_is_selectable() {
    assert_printed_hand_cost_accepts(digimon_engine::enums::CardKind::Digimon, "TB");
}

#[test]
fn ex12_031_printed_op_wd_level_bearing_tamer_card_is_selectable() {
    assert_printed_hand_cost_accepts(digimon_engine::enums::CardKind::Tamer, "TB");
}

#[test]
fn ex12_031_printed_op_wd_level_bearing_option_card_is_selectable() {
    assert_printed_hand_cost_accepts(digimon_engine::enums::CardKind::Option, "TB");
}

#[test]
fn ex12_031_printed_op_wd_rejects_level_seven_no_level_unrelated_and_partial_tb() {
    use digimon_engine::enums::CardKind;
    for seat in [0, 1] {
        for when_digivolving in [false, true] {
            for (level, trait_name) in [
                (Some(7), "Aquabeast"),
                (Some(7), "Ancient Sea Animal"),
                (Some(7), "TB"),
                (None, "TB"),
                (Some(6), "Dragon"),
                (Some(6), "TB-like"),
            ] {
                let runner = printed_hand_cost_runner(
                    seat,
                    when_digivolving,
                    predicate_witness("HAND-NEGATIVE", level, CardKind::Digimon, &[trait_name]),
                );
                assert!(
                    runner.pending_selection().is_none(),
                    "ineligible sole card cannot activate OP/WD: {level:?} {trait_name}"
                );
                assert!(hand_contains(&runner, seat, "HAND-NEGATIVE"));
                assert_eq!(runner.battle_area_size(1 - seat), 4);
                assert_eq!(
                    source_count(&runner, seat, 1),
                    if when_digivolving { 2 } else { 0 }
                );
            }
        }
    }
}

#[test]
fn ex12_031_printed_op_wd_hand_selector_includes_card_kinds_and_excludes_bad_cards() {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::enums::CardKind;
    // A valid Digimon enables activation independently, exposing a distinct
    // regression in the hand selector even if its count condition is repaired.
    for seat in [0, 1] {
        for when_digivolving in [false, true] {
            let mut runner = printed_hand_cost_runner(
                seat,
                when_digivolving,
                predicate_witness("GOOD-HAND", Some(6), CardKind::Digimon, &["TB"]),
            );
            // Install a mixed hand before re-firing this card's event. The
            // initial event is declined legally, without paying the hand cost.
            predicate_masked_action(&mut runner, seat, PASS);
            for (id, level, kind, trait_name) in [
                ("HAND-TAMER", Some(6), CardKind::Tamer, "TB"),
                ("HAND-OPTION", Some(6), CardKind::Option, "TB"),
                ("HIGH-AQUA", Some(7), CardKind::Digimon, "Aquabeast"),
                ("HIGH-SEA", Some(7), CardKind::Digimon, "Ancient Sea Animal"),
                ("HIGH-TB", Some(7), CardKind::Digimon, "TB"),
                ("NO-LEVEL", None, CardKind::Digimon, "TB"),
                ("WRONG-TRAIT", Some(6), CardKind::Digimon, "Dragon"),
                ("TB-SUBSTRING", Some(6), CardKind::Digimon, "TB-like"),
            ] {
                let data_index = runner.game.card_data.len();
                runner
                    .game
                    .card_data
                    .push(predicate_witness(id, level, kind, &[trait_name]));
                let index = runner.game.next_card_index();
                runner.game.players[seat as usize].hand.push(
                    digimon_engine::card_source::CardSource::new(data_index, seat, index),
                );
            }
            if when_digivolving {
                let handle = digimon_engine::permanent::PermanentHandle { player: seat, index: 1 };
                super::support::fire_when_digivolving(&mut runner, handle);
            } else {
                runner.fire_on_play(seat, 1);
            }
            let view = runner
                .pending_selection_view()
                .expect("matching Digimon enables activation");
            let expected: Vec<_> = ["GOOD-HAND", "HAND-TAMER", "HAND-OPTION"]
                .iter()
                .map(|id| PLAY_HAND_START + hand_index(&runner, seat, id) as u16)
                .collect();
            assert_eq!(view.valid_action_ids, expected, "printed card selector allows qualifying Tamer/Option witnesses, rejects all level/trait negatives");
            let mask = digimon_engine::action::build_action_mask(&runner.game, seat);
            for id in [
                "HIGH-AQUA",
                "HIGH-SEA",
                "HIGH-TB",
                "NO-LEVEL",
                "WRONG-TRAIT",
                "TB-SUBSTRING",
            ] {
                assert_eq!(mask[hand_index(&runner, seat, id)], 0.0);
            }
        }
    }
}

#[test]
fn ex12_031_printed_op_wd_only_own_hand_qualifies_for_cost() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::enums::CardKind;
    for seat in [0, 1] {
        for when_digivolving in [false, true] {
            let mut runner = printed_hand_cost_runner(
                seat,
                when_digivolving,
                predicate_witness("ZONE-WITNESS", Some(4), CardKind::Digimon, &["TB"]),
            );
            predicate_masked_action(&mut runner, seat, PASS);
            let slot = hand_index(&runner, seat, "ZONE-WITNESS");
            let candidate = runner.game.players[seat as usize].hand.remove(slot);
            runner.game.players[seat as usize].trash.push(candidate);
            // Matching cards in other zones never satisfy the own-hand cost.
            runner.place_on_field(seat, "ZONE-WITNESS", Some(0));
            runner.place_in_breeding(seat, "ZONE-WITNESS");
            super::support::push_to_trash(&mut runner, 1 - seat, "ZONE-WITNESS");
            let candidate = runner.game.players[(1 - seat) as usize]
                .trash
                .pop()
                .unwrap();
            runner.game.players[(1 - seat) as usize]
                .hand
                .push(candidate);
            if when_digivolving {
                let handle = digimon_engine::permanent::PermanentHandle { player: seat, index: 1 };
                super::support::fire_when_digivolving(&mut runner, handle);
            } else {
                runner.fire_on_play(seat, 1);
            }
            assert!(runner.pending_selection().is_none());
            assert_eq!(runner.battle_area_size(1 - seat), 4);
        }
    }
}

#[test]
fn ex12_031_printed_decode_main_and_inherited_preserve_digimon_kind_and_level() {
    use digimon_engine::action::space::{PASS, SOURCE_SELECT_START};
    use digimon_engine::enums::CardKind;
    use digimon_engine::selection::SelectionKind;
    for seat in [0, 1] {
        for inherited in [false, true] {
            let mut builder = DebugRunner::builder()
                .dsl_card(CARD_ID)
                .unwrap()
                .add_card(plain_digimon("DECODE-TOP", CardColor::Blue, 6, 10000));
            let candidates = [
                ("DECODE-GOOD", Some(4), CardKind::Digimon, "Aquabeast"),
                ("DECODE-TAMER", Some(4), CardKind::Tamer, "TB"),
                ("DECODE-OPTION", Some(4), CardKind::Option, "TB"),
                ("DECODE-HIGH", Some(5), CardKind::Digimon, "TB"),
                ("DECODE-NO-LEVEL", None, CardKind::Digimon, "TB"),
                ("DECODE-TRAIT", Some(4), CardKind::Digimon, "Dragon"),
            ];
            for (id, level, kind, trait_name) in candidates {
                builder = builder.add_card(predicate_witness(id, level, kind, &[trait_name]));
            }
            let mut runner = builder.memory(10).start();
            predicate_active_seat(&mut runner, seat);
            let mut stack: Vec<_> = candidates.iter().map(|(id, _, _, _)| *id).collect();
            stack.push(CARD_ID);
            if inherited {
                stack.push("DECODE-TOP");
            }
            let carrier = runner.place_stack(seat, &stack);
            runner.game.return_to_hand_from_effect(carrier, 1 - seat);
            let replacement = runner
                .pending_selection_view()
                .expect("Decode optional replacement");
            assert_eq!(replacement.kind, SelectionKind::Replacement);
            let mut declined = DebugRunner::wrap(runner.game.clone());
            predicate_masked_action(&mut declined, seat, PASS);
            assert!(declined.pending_selection().is_none());
            assert!(!field_contains(&declined, seat, "DECODE-GOOD"));
            predicate_masked_action(&mut runner, seat, replacement.valid_action_ids[0]);
            let selection = runner
                .pending_selection_view()
                .expect("Decode source selection");
            assert_eq!(selection.kind, SelectionKind::Material);
            assert_eq!(
                selection.valid_action_ids,
                vec![SOURCE_SELECT_START],
                "only the level-4 Digimon Aqua witness qualifies for Decode"
            );
            predicate_masked_action(&mut runner, seat, SOURCE_SELECT_START);
            assert!(runner.pending_selection().is_none());
            assert!(field_contains(&runner, seat, "DECODE-GOOD"));
            assert_eq!(runner.memory(), 10, "Decode plays without cost");
        }
    }
}

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
