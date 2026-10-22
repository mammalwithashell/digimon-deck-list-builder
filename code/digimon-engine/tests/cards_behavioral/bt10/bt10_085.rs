//! Printed image and official bundle BT10-085; DCGO BT10_085.cs.
use digimon_engine::action::{build_action_mask, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, PlaySource};

fn fixture(id: &str, name: &str, level: u8, knight: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.colors = vec![CardColor::Red];
    if knight {
        c.traits.push("Royal Knight".into());
    }
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 3,
    }];
    c
}
fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT10-085")
        .expect("BT10-085 embedded")
        .add_card(fixture("BASE", "Base", 5, false))
        .add_card(fixture("BADBASE", "Bad base", 3, false))
        .add_card(fixture("KNIGHT", "Jesmon", 6, true))
        .add_card(fixture("HUCK", "SaviorHuckmon", 6, false))
        .add_card(fixture("OTHER", "Other", 6, false))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, hand)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start()
}
fn choose(r: &mut DebugRunner, expected: usize) {
    let p = r.pending_selection_view().expect("choice surfaced");
    assert!(p.is_optional);
    assert_eq!(p.valid_action_ids.len(), expected);
    let mask = build_action_mask(&r.game, 0);
    for a in &p.valid_action_ids {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], 1.0);
    r.execute_action(0, p.valid_action_ids[0]).unwrap();
}
#[test]
fn bt10_085_paid_royal_knight_evolution_filters_bases_and_hand() {
    let mut r = runner(&["BT10-085", "KNIGHT", "HUCK", "OTHER"]);
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "BADBASE", Some(0));
    r.place_on_field(1, "BASE", Some(0));
    r.play(0, 0);
    choose(&mut r, 1);
    choose(&mut r, 1);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "KNIGHT"
    );
    assert_eq!(r.memory(), 4, "10 - play 4 - evolve 3 + observer 1");
}
#[test]
fn bt10_085_may_decline_base_or_hand() {
    for decline_hand in [false, true] {
        let mut r = runner(&["BT10-085", "KNIGHT"]);
        r.place_on_field(0, "BASE", Some(0));
        r.play(0, 0);
        if decline_hand {
            choose(&mut r, 1);
        }
        assert!(r.pending_is_optional());
        r.execute_action(0, PASS).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 6);
        assert_eq!(
            r.game.players[0].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "BASE"
        );
    }
}
#[test]
fn bt10_085_no_legal_royal_knight_or_wrong_turn_has_no_prompt() {
    for (hand, wrong_turn) in [
        (vec!["HUCK"], false),
        (vec!["KNIGHT"], true),
        (vec![], false),
    ] {
        let mut r = runner(&hand);
        r.place_on_field(0, "BASE", Some(0));
        let ciel = r.place_on_field(0, "BT10-085", Some(0));
        if wrong_turn {
            r.set_first_player(1);
        }
        r.fire_on_play(0, ciel.index as usize);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn bt10_085_observer_huckmon_and_knight_once_per_turn() {
    for id in ["HUCK", "KNIGHT"] {
        let mut r = runner(&[id, id]);
        r.place_on_field(0, "BT10-085", Some(0));
        r.place_on_field(0, "BASE", Some(0));
        r.place_on_field(0, "BASE", Some(0));
        assert!(r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByDigivolve));
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 8);
        assert!(r.game.digivolve_from_hand(0, 0, 2, PlaySource::ByDigivolve));
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 5, "second evolution does not gain memory");
    }
}
#[test]
fn bt10_085_observer_rejects_unrelated_evolution_and_opponent_turn() {
    for (id, wrong_turn) in [("OTHER", false), ("KNIGHT", true)] {
        let mut r = runner(&[id]);
        r.place_on_field(0, "BT10-085", Some(0));
        r.place_on_field(0, "BASE", Some(0));
        if wrong_turn {
            r.set_first_player(1);
        }
        let before = r.memory();
        assert!(r.game.digivolve_from_hand(0, 0, 1, PlaySource::ByEffect));
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), before - 3);
    }
}

#[test]
fn bt10_085_opponent_evolution_does_not_trigger_own_observer() {
    let mut r = runner(&[]);
    r.place_on_field(0, "BT10-085", Some(0));
    r.place_on_field(1, "BASE", Some(0));
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "KNIGHT")
        .unwrap();
    let instance = r.game.next_card_index();
    r.game.players[1]
        .hand
        .push(CardSource::new(idx, 1, instance));
    let target = digimon_engine::permanent::PermanentHandle {
        player: 1,
        index: 0,
    };
    assert!(r.game.effect_initiated_digivolve(
        1,
        0,
        target,
        digimon_engine::enums::CostDelta::Fixed(0),
        false,
        PlaySource::ByEffect
    ));
    r.auto_resolve().unwrap();
    assert_eq!(
        r.memory(),
        10,
        "free opponent evolution grants no observer memory"
    );
}

#[test]
fn bt10_085_printed_metadata_and_no_legal_base() {
    let mut r = runner(&["KNIGHT"]);
    let c = r.compiled_card("BT10-085").unwrap();
    assert_eq!(c.level, Some(4));
    assert_eq!(c.cost, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert_eq!(c.form.as_deref(), Some("Champion"));
    assert_eq!(c.attribute, ["Data"]);
    assert!(c.traits.iter().any(|t| t == "Puppet"));
    assert!(c.alt_paths.is_empty());
    let ciel = r.place_on_field(0, "BT10-085", Some(0));
    r.place_on_field(0, "BADBASE", Some(0));
    r.place_on_field(1, "BASE", Some(0));
    r.fire_on_play(0, ciel.index as usize);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt10_085_queued_observers_retain_event_after_evolved_target_departs() {
    let mut r = runner(&["KNIGHT"]);
    r.place_on_field(0, "BT10-085", Some(0));
    r.place_on_field(0, "BT10-085", Some(0));
    r.place_on_field(0, "BASE", Some(0));
    assert!(r.game.digivolve_from_hand(0, 0, 2, PlaySource::ByDigivolve));
    assert!(
        r.pending_selection().is_some(),
        "simultaneous observers expose trigger order"
    );
    // Parked trigger-order fixture: remove the evolved subject before resolving observers.
    r.game.players[0].delete_permanent(2);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.memory(),
        9,
        "both copies gain memory from the captured evolution event"
    );
}
