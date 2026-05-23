use digimon_engine::action::space::{
    encode_breeding_select, BREEDING_SOURCE_SELECT_START, PASS, PLAY_HAND_START,
    SOURCE_SELECT_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 3;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card data present");
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

#[test]
fn select_union_zone_can_offer_trash_or_breeding_sources_in_one_prompt() {
    let yaml = r#"
card: RK-UNION-SOURCE
name: Union Source Test
kind: digimon
level: 6
color: [red]
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_own_breeding_permanent:
          bind_as: king
          filter: { name_is: "King Drasil_7D6" }
          prompt: "Choose King Drasil"
      - select_union_zone:
          of: you
          zones: [trash, material]
          material_of: king
          optional: true
          bind_as: pick
          filter:
            all_of:
              - kind: digimon
              - none_of:
                  - name_is: "Gankoomon"
                  - name_is: "Omnimon"
              - any_of:
                  - name_contains: "Sistermon"
                  - trait_has: "Royal Knight"
          prompt: "Play Sistermon from trash or Royal Knight from sources"
      - play_union_bound_free:
          binding: pick
          bind_as: played
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("union-source YAML parses")
        .add_card(digimon("KING", "King Drasil_7D6", &[]))
        .add_card(digimon("SISTER", "Sistermon Blanc", &[]))
        .add_card(digimon("RK-ALPHA", "Alphamon", &["Royal Knight"]))
        .add_card(digimon("RK-OMNI", "Omnimon", &["Royal Knight"]))
        .hand(0, &["RK-UNION-SOURCE"])
        .memory(10)
        .build();
    push_to_trash(&mut runner, 0, "SISTER");
    let king = runner.place_stack(0, &["RK-ALPHA", "RK-OMNI", "KING"]);
    let breeding = runner.game.players[0]
        .battle_area
        .remove(king.index as usize);
    runner.game.players[0].breeding_area = Some(breeding);

    runner.play(0, 0).expect("play source card");

    let breeding_prompt = runner
        .pending_selection_view()
        .expect("breeding carrier selection opens");
    assert_eq!(breeding_prompt.kind, SelectionKind::BreedingPermanent);
    runner
        .execute_action(
            0,
            encode_breeding_select(0).expect("encode breeding select"),
        )
        .expect("choose King Drasil");

    let union_prompt = runner
        .pending_selection_view()
        .expect("union prompt opens after choosing King Drasil");
    assert_eq!(
        union_prompt.kind,
        SelectionKind::UnionZone {
            zones: digimon_engine::selection::UnionZoneSet::TRASH
                | digimon_engine::selection::UnionZoneSet::MATERIAL
        }
    );
    assert!(
        union_prompt.valid_action_ids.contains(&TRASH_EFFECT_START),
        "Sistermon from trash must be a union candidate"
    );
    assert!(
        union_prompt
            .valid_action_ids
            .contains(&BREEDING_SOURCE_SELECT_START),
        "Alphamon source under breeding King Drasil must be a union candidate"
    );
    assert!(
        !union_prompt
            .valid_action_ids
            .contains(&(BREEDING_SOURCE_SELECT_START + 1)),
        "Omnimon source is excluded by printed BT13-019 name restriction"
    );

    runner
        .execute_action(0, BREEDING_SOURCE_SELECT_START)
        .expect("choose Alphamon source");
    runner.auto_resolve().expect("play chosen source");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "RK-ALPHA"),
        "selected breeding source should be played as a new permanent"
    );
}

fn source_cost_yaml(optional: bool) -> String {
    format!(
        r#"
card: RK-SOURCE-COST
name: Source Cost Test
kind: digimon
level: 6
color: [red]
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: {optional}
          bind_as: cost_card
          filter:
            all_of:
              - kind: digimon
              - trait_has: "Royal Knight"
          prompt: "Place a Royal Knight as this Digimon's bottom source"
          then:
            - place_as_bottom_source:
                source: cost_card
                target: source
            - gain_memory: 2
"#
    )
}

#[test]
fn hand_or_trash_source_cost_places_card_before_success_body() {
    let yaml = source_cost_yaml(true);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("source-cost YAML parses")
        .add_card(digimon("RK-HAND", "Craniamon", &["Royal Knight"]))
        .hand(0, &["RK-SOURCE-COST", "RK-HAND"])
        .memory(10)
        .build();

    runner.play(0, 0).expect("play source-cost card");
    let memory_before = runner.game.memory;
    let prompt = runner
        .pending_selection_view()
        .expect("optional hand/trash source-cost prompt opens");
    assert!(
        prompt.valid_action_ids.contains(&PLAY_HAND_START),
        "Royal Knight in hand must be a payable source-cost candidate"
    );

    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("pay source cost from hand");
    runner.auto_resolve().expect("resolve success body");

    let source_perm = &runner.game.players[0].battle_area[0];
    assert_eq!(runner.game.memory, memory_before + 2);
    assert!(
        source_perm
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "RK-HAND"),
        "selected Royal Knight should be placed as a source before the benefit"
    );
}

#[test]
fn optional_hand_or_trash_source_cost_decline_skips_success_body() {
    let yaml = source_cost_yaml(true);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("source-cost YAML parses")
        .add_card(digimon("RK-HAND", "Craniamon", &["Royal Knight"]))
        .hand(0, &["RK-SOURCE-COST", "RK-HAND"])
        .memory(10)
        .build();

    runner.play(0, 0).expect("play source-cost card");
    let memory_before = runner.game.memory;
    runner
        .execute_action(0, PASS)
        .expect("decline optional source cost");
    runner.auto_resolve().expect("finish after decline");

    assert_eq!(
        runner.game.memory, memory_before,
        "declining the cost must skip the success body"
    );
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        1,
        "declining the cost must not place a source"
    );
}

#[test]
fn required_hand_or_trash_source_cost_without_candidates_skips_success_body() {
    let yaml = source_cost_yaml(false);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("source-cost YAML parses")
        .hand(0, &["RK-SOURCE-COST"])
        .memory(10)
        .build();

    runner.play(0, 0).expect("play source-cost card");
    let memory_before = runner.game.memory;
    runner
        .auto_resolve()
        .expect("required unpayable cost no-ops");

    assert!(
        runner.pending_selection_view().is_none(),
        "unpayable required cost must not open a prompt"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "unpayable source cost must suppress the success body"
    );
}

#[test]
fn union_source_flows_do_not_change_action_space_size() {
    assert_eq!(digimon_engine::action::space::ACTION_SPACE_SIZE, 2192);
}

fn play_union_source_yaml(suppress_on_play: bool) -> String {
    format!(
        r#"
card: RK-ATTACHER
name: Royal Knight Attacher
kind: digimon
level: 6
color: [red]
cost: 0
dp: 12000
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, material]
          material_of: source
          optional: false
          bind_as: play_pick
          filter:
            all_of:
              - kind: digimon
              - trait_has: "Royal Knight"
          prompt: "Play a Royal Knight from hand or sources"
      - play_union_bound_free:
          binding: play_pick
          bind_as: played
          suppress_on_play: {suppress_on_play}
      - place_as_bottom_source:
          source: source
          target: played
"#
    )
}

const PLAYED_SOURCE_YAML: &str = r#"
card: RK-PLAYED-SOURCE
name: Played Source
kind: digimon
level: 4
color: [red]
cost: 3
dp: 5000
traits: [Royal Knight]
effects:
  - when: on_play
    process:
      - gain_memory: 7
"#;

#[test]
fn hand_or_source_play_binds_played_permanent_for_attach_self_follow_up() {
    let yaml = play_union_source_yaml(true);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("attacher YAML parses")
        .from_dsl_yaml(PLAYED_SOURCE_YAML)
        .expect("played source YAML parses")
        .add_card(digimon("RK-HAND-PICK", "Hand Pick", &["Royal Knight"]))
        .hand(0, &["RK-HAND-PICK"])
        .memory(10)
        .build();
    let carrier = runner.place_stack(0, &["RK-PLAYED-SOURCE", "RK-ATTACHER"]);

    runner.fire_on_play(0, carrier.index as usize);

    let prompt = runner
        .pending_selection_view()
        .expect("hand-or-source play prompt opens");
    assert!(
        prompt.valid_action_ids.contains(&PLAY_HAND_START),
        "hand candidate must share the same union prompt"
    );
    assert!(
        prompt.valid_action_ids.contains(&SOURCE_SELECT_START),
        "source candidate must share the same union prompt"
    );

    let memory_before = runner.game.memory;
    runner
        .execute_action(0, SOURCE_SELECT_START)
        .expect("choose source candidate");
    runner.auto_resolve().expect("play source and attach self");

    assert_eq!(
        runner.game.memory, memory_before,
        "suppress_on_play must prevent the played source card's On Play gain"
    );

    let played = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "RK-PLAYED-SOURCE")
        .expect("selected source should be played");
    assert!(
        played
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "RK-ATTACHER"),
        "the resolving source permanent should attach under the played Digimon"
    );
}
