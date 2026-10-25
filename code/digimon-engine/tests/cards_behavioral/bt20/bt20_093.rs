//! BT20-093: official observation 074c2baf, image, Q4433-Q4436; rules 16-17 (p.38).
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;

fn material(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.effect_text = "Dracomon".into();
    c.level = Some(6);
    c.dp = Some(11000);
    c.colors = vec![color];
    c
}

fn setup(same_turn: bool) -> DebugRunner {
    let mut result = material("RESULT", CardColor::Blue);
    result.card_name = "Examon".into();
    result.level = Some(7);
    let req = |color| DnaRequirement {
        level: 6,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    result.dna_costs = vec![DnaCost {
        requirement1: req(CardColor::Blue),
        requirement2: req(CardColor::Green),
        memory_cost: 2,
    }];
    let mut wrong = result.clone();
    wrong.card_id = "WRONG".into();
    wrong.card_name = "Examon X Antibody".into();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cards/bt20/BT20-093.yaml"
            ))
            .expect("production YAML"),
        )
        .unwrap()
        .add_card(result)
        .add_card(wrong)
        .add_card(material("BLUE", CardColor::Blue))
        .add_card(material("GREEN", CardColor::Green))
        .add_card(material("RED", CardColor::Red))
        .add_card(material("FILL", CardColor::Red))
        .memory(10)
        .hand(0, &["RESULT", "WRONG"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    runner.game.turn_count = 3;
    let option = runner.place_on_field(0, "BT20-093", None);
    runner.game.players[0].battle_area[option.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        placed_on_turn: if same_turn { 3 } else { 2 },
        trash_on_turn: 100,
        trigger: DelayTrigger::EndOfYourNextTurn,
    };
    runner.place_on_field(0, "BLUE", None);
    runner.place_on_field(0, "GREEN", None);
    runner.place_on_field(0, "RED", None);
    runner
}

fn trigger(runner: &mut DebugRunner, index: u8) {
    runner.game.delete_permanent_with_cause(
        digimon_engine::permanent::PermanentHandle { player: 0, index },
        ReplacementCause::OpponentEffect,
    );
}
fn pick(game: &mut digimon_engine::game::Game, label: Option<&str>) {
    let sel = game.pending_selection.as_ref().expect("selection");
    let action = label
        .map(|label| {
            sel.effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .find(|c| c.label == label)
                .expect(label)
                .action_id
        })
        .unwrap_or(sel.valid_action_ids[0]);
    let player = sel.selecting_player;
    let mask = digimon_engine::action::mask::build_action_mask(game, player);
    let actual: Vec<_> = mask
        .iter()
        .enumerate()
        .filter(|(_, v)| **v > 0.5)
        .map(|(i, _)| i as u16)
        .collect();
    let mut expected = sel.valid_action_ids.clone();
    if sel.is_optional && !expected.contains(&digimon_engine::action::space::PASS) {
        expected.push(digimon_engine::action::space::PASS);
    }
    expected.sort_unstable();
    assert_eq!(actual, expected, "mask exposes exactly the pending choices");
    game.resolve_selection(player, action).unwrap();
}
fn top_ids(game: &digimon_engine::game::Game) -> Vec<String> {
    game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&game.card_data).to_string())
        .collect()
}

#[test]
fn delay_field_dna_same_turn_not_offered() {
    let mut runner = setup(true);
    trigger(&mut runner, 1);
    assert!(
        runner.game.pending_selection.is_none(),
        "Delay must be unavailable on placing turn"
    );
    assert!(top_ids(&runner.game).contains(&"BT20-093".into()));
}

#[test]
fn delay_field_dna_paid_legal_pair_cost_and_clone_each_prompt() {
    for threatened in [1, 2, 3] {
        let mut runner = setup(false);
        trigger(&mut runner, threatened);
        for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
            let mut cloned = runner.game.clone();
            pick(&mut cloned, choice);
            pick(&mut runner.game, choice);
            assert_eq!(top_ids(&cloned), top_ids(&runner.game));
            assert_eq!(cloned.memory, runner.game.memory);
            assert_eq!(
                cloned.players[0]
                    .trash
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>(),
                runner.game.players[0]
                    .trash
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                cloned
                    .pending_selection
                    .as_ref()
                    .map(|s| &s.valid_action_ids),
                runner
                    .game
                    .pending_selection
                    .as_ref()
                    .map(|s| &s.valid_action_ids)
            );
            if let Some(selection) = runner.game.pending_selection.as_ref() {
                let labels: Vec<_> = selection
                    .effect_choices
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect();
                assert!(!labels.contains(&"Examon X Antibody"));
                assert!(!labels.contains(&"RED"));
            }
        }
        assert!(runner.game.pending_selection.is_none());
        assert!(
            top_ids(&runner.game).contains(&"RESULT".into()),
            "merged Digimon must survive"
        );
        assert_eq!(runner.game.memory, 8, "printed DNA cost must be paid");
        let trash: Vec<_> = runner.game.players[0]
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect();
        assert!(trash.contains(&"BT20-093"));
        assert_eq!(
            trash.contains(&"RED"),
            threatened == 3,
            "excluded threatened subject still leaves after field compaction"
        );
        assert!(!trash.contains(&"BLUE") && !trash.contains(&"GREEN"));
    }
}

#[test]
fn delay_field_dna_decline_after_payment_preserves_original_subject() {
    let mut runner = setup(false);
    trigger(&mut runner, 3);
    pick(&mut runner.game, None);
    runner
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert_eq!(top_ids(&runner.game), vec!["BLUE", "GREEN"]);
    let trash: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(trash.contains(&"BT20-093") && trash.contains(&"RED"));
}

#[test]
fn delay_field_dna_material_selection_order_does_not_change_recipe_stack_order() {
    for order in [["BLUE", "GREEN"], ["GREEN", "BLUE"]] {
        let mut runner = setup(false);
        trigger(&mut runner, 1);
        for choice in [None, Some("Examon"), Some(order[0]), Some(order[1])] {
            pick(&mut runner.game, choice);
        }
        let merged = runner.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == "RESULT")
            .expect("DNA result survives the replaced departure");
        let stack: Vec<_> = merged
            .card_sources
            .iter()
            .map(|card| card.card_id(&runner.game.card_data))
            .collect();
        assert_eq!(
            stack,
            ["GREEN", "BLUE", "RESULT"],
            "recipe order must not depend on pick order {order:?}"
        );
    }
}

#[test]
fn delay_field_dna_opponent_turn_pays_for_controller() {
    let mut runner = setup(false);
    runner.game.turn_player_idx = 1;
    runner.game.memory = 3;
    trigger(&mut runner, 1);
    for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
        pick(&mut runner.game, choice);
    }
    assert_eq!(runner.game.memory, 5, "P0 pays toward P1 on P1's turn");
    assert!(top_ids(&runner.game).contains(&"RESULT".into()));
}

#[test]
fn delay_rejects_battle_unrelated_and_opponent_subjects() {
    for case in 0..3 {
        let mut r = setup(false);
        let h = if case == 2 {
            r.place_on_field(1, "BLUE", None)
        } else {
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 1,
            }
        };
        if case == 1 {
            let index = r.game.players[0].battle_area[1].top_card().data_index as usize;
            std::sync::Arc::make_mut(&mut r.game.card_data.0)[index]
                .effect_text
                .clear();
        }
        r.game.delete_permanent_with_cause(
            h,
            if case == 0 {
                ReplacementCause::Battle
            } else {
                ReplacementCause::OpponentEffect
            },
        );
        assert!(r.game.pending_selection.is_none());
        assert!(top_ids(&r.game).contains(&"BT20-093".into()));
    }
}
#[test]
fn delay_decline_before_payment_preserves_option() {
    let mut r = setup(false);
    trigger(&mut r, 1);
    r.game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert!(top_ids(&r.game).contains(&"BT20-093".into()));
    assert!(!top_ids(&r.game).contains(&"BLUE".into()));
}

fn play_setup(security: bool) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/cards/bt20/BT20-093.yaml"
            ))
            .expect("production YAML"),
        )
        .unwrap();
    for (id, name, text, kind) in [
        ("DRACO", "Dracomon X", "", CardKind::Digimon),
        ("TEXT", "Other", "Examon", CardKind::Digimon),
        ("BAD", "Unrelated", "", CardKind::Digimon),
        ("TAMER", "Dracomon", "Examon", CardKind::Tamer),
        ("TRASH", "Dracomon", "", CardKind::Digimon),
        ("ATTACKER", "Attacker", "", CardKind::Digimon),
        ("FILL", "Filler", "", CardKind::Digimon),
    ] {
        let mut c = make_test_card(id, name);
        c.effect_text = text.into();
        c.card_kind = kind;
        c.colors = vec![CardColor::Red];
        c.play_cost = 7;
        c.dp = Some(10000);
        b = b.add_card(c);
    }
    let owner = if security { 1 } else { 0 };
    let mut r = b
        .memory(10)
        .hand(
            owner,
            if security {
                &["DRACO", "TEXT", "BAD", "TAMER"][..]
            } else {
                &["BT20-093", "DRACO", "TEXT", "BAD", "TAMER"][..]
            },
        )
        .security(1, if security { &["BT20-093"][..] } else { &[][..] })
        .deck(owner, &["FILL", "FILL", "TRASH"])
        .deck(1 - owner, &["FILL"; 5])
        .start();
    r.skip_mulligan();
    let c = r.game.players[owner as usize].deck.pop().unwrap();
    r.game.players[owner as usize].trash.push(c);
    r
}
fn pick_card(game: &mut digimon_engine::game::Game, id: &str) {
    let player = game.pending_selection.as_ref().unwrap().selecting_player;
    let a = if game.pending_selection.as_ref().and_then(|p|p.card_choice.as_ref()).is_some() {
        owned_union_fixture::action(game, player, id, false)
            .or_else(||owned_union_fixture::action(game, player, id, true))
            .expect("printed eligible card has a physical union row")
    } else {
        let row:digimon_engine::dcgo_recording::SelectionRow=serde_json::from_value(serde_json::json!({"step":0,"actor":player,"prompt":"SelectCardEffect","phase":"Main","card_ids":[id]})).unwrap();
        digimon_engine::runners::selection_resolve::resolve_next(game, &row, 0).unwrap().unwrap()
    };
    assert!(digimon_engine::action::mask::build_action_mask(game, player)[a as usize] > 0.5);
    game.resolve_selection(player, a).unwrap();
}
fn assert_delayed(game: &digimon_engine::game::Game, owner: usize) {
    let p = game.players[owner]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&game.card_data) == "BT20-093")
        .expect("mandatory placement even when play declined");
    assert!(matches!(p.option_state, OptionState::Delayed { .. }));
}
#[test]
fn main_optional_play_filters_and_reduces_three_then_places_option() {
    for choice in [Some("DRACO"), Some("TEXT"), None] {
        let mut r = play_setup(false);
        r.place_on_field(0, "FILL", None);
        r.game.play_option_from_hand(0, 0);
        let s = r.game.pending_selection.as_ref().unwrap();
        assert_eq!(s.valid_action_ids.len(), 2, "two eligible Digimon");
        assert!(s.is_optional);
        let mut clone = r.game.clone();
        for g in [&mut r.game, &mut clone] {
            if let Some(id) = choice {
                pick_card(g, id)
            } else {
                g.resolve_selection(0, digimon_engine::action::space::PASS)
                    .unwrap();
            }
            assert_delayed(g, 0);
            assert_eq!(g.memory, if choice.is_some() { 4 } else { 8 });
            assert!(g.pending_selection.is_none());
        }
    }
}
#[test]
fn security_plays_name_only_from_both_zones_free_and_always_places_option() {
    for choice in [Some("DRACO"), Some("TRASH"), None] {
        let mut r = play_setup(true);
        let attacker = r.place_on_field(0, "ATTACKER", Some(0));
        let _ = r.attack_player(attacker, 1, false);
        let s = r
            .game
            .pending_selection
            .as_ref()
            .expect("security union selection");
        assert_eq!(s.valid_action_ids.iter().filter(|&&a|a!=digimon_engine::action::space::PASS).count(), 2, "hand/trash Dracomon commits only");
        assert!(s.valid_action_ids.contains(&digimon_engine::action::space::PASS), "explicit optional decline");
        assert!(s.is_optional);
        let mut clone = r.game.clone();
        for g in [&mut r.game, &mut clone] {
            if let Some(id) = choice {
                pick_card(g, id)
            } else {
                g.resolve_selection(1, digimon_engine::action::space::PASS)
                    .unwrap();
            }
            assert_delayed(g, 1);
            assert_eq!(g.memory, 10, "security play free");
            if let Some(id) = choice {
                assert!(g.players[1]
                    .battle_area
                    .iter()
                    .any(|p| p.top_card().card_id(&g.card_data) == id));
            }
        }
    }
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
