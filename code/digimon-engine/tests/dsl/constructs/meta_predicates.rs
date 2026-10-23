//! Meta-deck predicate constructs. CR 4.3 §§4-1-2/3 (controller-relative
//! memory), 4-2-2/3 (alternate costs), 3-4-7-7/8 (breeding isolation), and
//! 4-3-2 (digivolution cards are not additional Digimon).
//! Card-text idioms: EX13-060 / BT26-078 (opponent has at least 5 memory),
//! BT25-077 (sum of battle-area Digimon levels), BT26-032 / BT26-080
//! (printed play-cost requirements, independently of the amount paid).

use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;
use digimon_engine::{CardData, CardKind};

const OWN_MEMORY: &str = r#"
card: TST-META-MEMORY
name: Controller memory probe
kind: digimon
level: 3
color: [red]
cost: 8
dp: 2000
effects:
  - when: on_play
    process:
      - if:
          condition: { own_memory_lte: -5 }
          then:
            - draw: { of: you, count: 1 }
"#;

const LEVEL_SUM: &str = r#"
card: TST-META-LEVEL-SUM
name: Board level sum probe
kind: digimon
level: 6
color: [red]
cost: 8
dp: 12000
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    when_playing_this: true
    condition:
      level_sum_gte:
        filter: { owner: any, zone: [battle_area], kind: digimon }
        n: 12
    amount: 5
"#;

const PLAY_COST_FIELD: &str = r#"
card: TST-META-COST-FIELD
name: Field play cost probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon, play_cost_gte: 12 }
          prompt: Choose a Digimon with play cost at least 12
      - delete_permanent: { target: victim }
"#;

const PLAY_COST_HAND: &str = r#"
card: TST-META-COST-HAND
name: Hand play cost probe
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter: { kind: digimon, play_cost_gte: 12 }
          prompt: Play a card with printed play cost at least 12
      - play_from_hand: { of: you, hand_index: chosen, cost_delta: { reduce: 5 } }
"#;

fn data(id: &str, kind: CardKind, level: Option<u8>, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = kind;
    card.level = level;
    card.play_cost = cost;
    if kind != CardKind::Digimon {
        card.dp = None;
    }
    card
}

fn runner(yaml: &str, variable_level: u8) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(data("FILLER", CardKind::Digimon, Some(3), 0))
        .add_card(data("LEVEL-SIX", CardKind::Digimon, Some(6), 0))
        .add_card(data(
            "LEVEL-VARIABLE",
            CardKind::Digimon,
            Some(variable_level),
            0,
        ))
        .add_card(data("LEVEL-SEVEN", CardKind::Digimon, Some(7), 0))
        .add_card(data("TAMER", CardKind::Tamer, None, 0))
        .add_card(data("OPTION", CardKind::Option, None, 0))
        .add_card(data("COST-ELEVEN", CardKind::Digimon, Some(6), 11))
        .add_card(data("COST-TWELVE", CardKind::Digimon, Some(6), 12))
        .add_card(data("COST-THIRTEEN", CardKind::Digimon, Some(6), 13))
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(9)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn play(r: &mut DebugRunner, owner: u8, id: &str) {
    assert_eq!(r.game.turn_player(), owner);
    r.add_to_hand(owner, id);
    let action = (r.hand_size(owner) - 1) as u16;
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.game.decode_action(action, owner);
}

fn selection(r: &DebugRunner, owner: u8, kind: SelectionKind, ids: &[u16], optional: bool) {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("a real player choice");
    assert_eq!(pending.selecting_player, owner);
    assert_eq!(pending.kind, kind);
    assert_eq!(pending.is_optional, optional);
    assert_eq!(pending.valid_action_ids, ids);
    for (action, value) in build_action_mask(&r.game, owner).iter().enumerate() {
        let legal = ids.contains(&(action as u16)) || (optional && action == PASS as usize);
        assert_eq!(*value, if legal { 1.0 } else { 0.0 }, "action {action}");
    }
}

#[test]
fn own_memory_lte_uses_controller_sign_for_both_players_and_turns() {
    for owner in [0, 1] {
        for turn_player in [0, 1] {
            // Changing only the gauge crosses the inclusive -5 threshold.
            // Positive values also reject an accidental absolute-value check.
            for own_memory in [-6, -5, -4, 4, 5, 6] {
                let mut r = runner(OWN_MEMORY, 6);
                r.set_first_player(turn_player);
                let gauge = if owner == turn_player {
                    own_memory
                } else {
                    -own_memory
                };
                r.game.set_memory(gauge);
                let source = r.place_on_field(owner, "TST-META-MEMORY", Some(0));
                let before = r.hand_size(owner);
                r.fire_on_play(owner, source.index as usize);
                assert_eq!(
                    r.hand_size(owner),
                    before + usize::from(own_memory <= -5),
                    "controller {owner}, turn player {turn_player}, own memory {own_memory}"
                );
                assert_eq!(r.hand_size(1 - owner), 0);
                assert_eq!(
                    r.memory(),
                    gauge,
                    "the predicate itself does not alter memory"
                );
                assert!(r.game.pending_selection.is_none());
            }
        }
    }
}

#[test]
fn own_memory_lte_reads_the_gauge_after_a_real_play_pays_its_cost() {
    for owner in [0, 1] {
        for initial_memory in [2, 3, 4] {
            let mut r = runner(OWN_MEMORY, 6);
            r.set_first_player(owner);
            r.game.set_memory(initial_memory);
            play(&mut r, owner, "TST-META-MEMORY");
            let own_memory = if r.game.turn_player() == owner {
                r.memory()
            } else {
                -r.memory()
            };
            assert_eq!(own_memory, initial_memory - 8);
            assert_eq!(r.hand_size(owner), usize::from(initial_memory <= 3));
            assert_eq!(r.game.players[owner as usize].battle_area.len(), 1);
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn level_sum_gte_sums_both_battle_areas_and_changes_the_actual_payment() {
    for owner in [0, 1] {
        for other_level in [5, 6, 7] {
            let mut r = runner(LEVEL_SUM, other_level);
            r.set_first_player(owner);
            r.place_stack(owner, &["LEVEL-SEVEN", "LEVEL-SEVEN", "LEVEL-SIX"]);
            r.place_stack(1 - owner, &["LEVEL-SEVEN", "LEVEL-VARIABLE"]);
            for player in [0, 1] {
                r.place_in_breeding(player, "LEVEL-SEVEN");
                r.place_on_field(player, "TAMER", Some(0));
                let option = r.place_on_field(player, "OPTION", Some(0));
                r.game.players[player as usize].battle_area[option.index as usize].option_state =
                    digimon_engine::permanent::OptionState::OrdinaryFieldOption;
            }
            // Only the opponent's top-card level differs: sums 11/12/13.
            // The level-6 card still in hand, sources, and breeding cards do
            // not contribute to the would-play cost-reduction condition.
            let before = r.game.players[owner as usize].battle_area.len();
            play(&mut r, owner, "TST-META-LEVEL-SUM");
            assert_eq!(r.memory(), if other_level >= 6 { 6 } else { 1 });
            assert_eq!(r.game.players[owner as usize].battle_area.len(), before + 1);
            assert_eq!(r.hand_size(owner), 0);
            assert!(
                r.game.pending_selection.is_none(),
                "the reduction is mandatory"
            );
            for player in [0, 1] {
                assert!(r.game.players[player].breeding_area.is_some());
                assert_eq!(r.game.players[player].trash.len(), 0);
            }
        }
    }
}

#[test]
fn level_sum_gte_does_not_count_sources_or_breeding_as_additional_digimon() {
    for decoys in [false, true] {
        let mut r = runner(LEVEL_SUM, 6);
        if decoys {
            r.place_stack(0, &["LEVEL-SEVEN", "LEVEL-SEVEN", "LEVEL-SIX"]);
            r.place_in_breeding(0, "LEVEL-SEVEN");
            r.place_in_breeding(1, "LEVEL-SEVEN");
            r.place_on_field(0, "TAMER", Some(0));
            let option = r.place_on_field(1, "OPTION", Some(0));
            r.game.players[1].battle_area[option.index as usize].option_state =
                digimon_engine::permanent::OptionState::OrdinaryFieldOption;
        } else {
            r.place_on_field(0, "LEVEL-SIX", Some(0));
        }
        play(&mut r, 0, "TST-META-LEVEL-SUM");
        assert_eq!(
            r.memory(),
            1,
            "only six levels exist before the card is played"
        );
        assert_eq!(r.hand_size(0), 0);
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn play_cost_gte_field_filter_keeps_exact_and_above_but_excludes_below() {
    for owner in [0, 1] {
        for chosen in [1, 2] {
            let mut r = runner(PLAY_COST_FIELD, 6);
            r.set_first_player(owner);
            let opponent = 1 - owner;
            for id in ["COST-ELEVEN", "COST-TWELVE", "COST-THIRTEEN"] {
                r.place_on_field(opponent, id, Some(0));
            }
            let victim = r.game.players[opponent as usize].battle_area[chosen]
                .top_card()
                .handle();
            let survivor = r.game.players[opponent as usize].battle_area[0]
                .top_card()
                .handle();
            play(&mut r, owner, "TST-META-COST-FIELD");
            selection(&r, owner, SelectionKind::OppField, &[101, 102], false);
            r.execute_action(owner, 100 + chosen as u16).unwrap();
            assert_eq!(r.game.players[opponent as usize].battle_area.len(), 2);
            assert_eq!(r.game.players[opponent as usize].trash.len(), 1);
            assert_eq!(r.game.players[opponent as usize].trash[0].handle(), victim);
            assert_eq!(
                r.game.players[opponent as usize].battle_area[0]
                    .top_card()
                    .handle(),
                survivor
            );
            assert_eq!(r.memory(), 9);
            assert!(r.game.pending_selection.is_none());
        }
    }
}

#[test]
fn play_cost_gte_hand_filter_uses_printed_cost_before_a_reduced_payment() {
    for chosen in [1, 2, PASS] {
        let mut r = runner(PLAY_COST_HAND, 6);
        for id in ["COST-ELEVEN", "COST-TWELVE", "COST-THIRTEEN"] {
            r.add_to_hand(0, id);
        }
        let selected = if chosen != PASS {
            Some(r.game.players[0].hand[chosen as usize].handle())
        } else {
            None
        };
        play(&mut r, 0, "TST-META-COST-HAND");
        selection(&r, 0, SelectionKind::Hand, &[1, 2], true);
        r.execute_action(0, chosen).unwrap();
        if let Some(selected) = selected {
            let played = r.game.players[0]
                .battle_area
                .last()
                .unwrap()
                .top_card()
                .handle();
            assert_eq!(played, selected);
            let printed_cost = r.game.card_data_for_handle(played).unwrap().play_cost;
            assert_eq!(printed_cost, if chosen == 1 { 12 } else { 13 });
            assert_eq!(r.memory(), 9 - (printed_cost as i16 - 5));
            assert_eq!(r.game.players[0].battle_area.len(), 2);
            assert_eq!(r.hand_size(0), 2);
        } else {
            assert_eq!(r.memory(), 9, "declining pays nothing");
            assert_eq!(r.game.players[0].battle_area.len(), 1);
            assert_eq!(r.hand_size(0), 3);
        }
        assert_eq!(
            r.game
                .card_data_for_handle(r.game.players[0].hand[0].handle())
                .unwrap()
                .card_id,
            "COST-ELEVEN"
        );
        assert_eq!(r.trash_size(0), 0);
        assert!(r.game.pending_selection.is_none());
    }
}
