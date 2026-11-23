use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, PlayerModifierEntry};
use digimon_engine::selection::SelectionKind;

const TAMER: &str = r#"
card: TEST-KURATA
name: "Nanami, Kouki, & Ivan"
kind: tamer
color: [purple]
cost: 4
traits: ["Kurata's Army"]
"#;

const RESULT: &str = r#"
card: TEST-THUNDERBIRD
name: Thunderbirdmon
kind: digimon
level: 4
color: [blue, purple]
cost: 5
dp: 6000
traits: ["Kurata's Army"]
alt_paths:
  - kind: digivolve
    from: { kind: tamer, trait_has: "Kurata's Army" }
    source_treated_as: tamer
    cost: 2
effects:
  - when: when_digivolving
    process:
      - gain_memory: 1
"#;

const OBSERVER: &str = r#"
card: TEST-OBSERVER
name: Digivolution Observer
kind: tamer
color: [purple]
cost: 3
effects:
  - when: on_digivolve
    process:
      - gain_memory: 1
"#;

const OPTION: &str = r#"
card: TEST-BIO-HYBRID
name: Bio Hybrid Digivolution
kind: option
color: [purple]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          optional: true
          bind_as: host
          filter:
            kind: tamer
            has_digivolve_candidate:
              of: you
              zone: [hand, trash]
              filter: { kind: digimon, trait_has: "Kurata's Army" }
      - if:
          condition: { binding_present: host }
          then:
            - select_union_zone:
                of: you
                zones: [hand, trash]
                optional: true
                bind_as: result
                filter:
                  kind: digimon
                  trait_has: "Kurata's Army"
                  can_digivolve_onto: host
            - effect_initiated_digivolve:
                target: host
                source: result
                cost: free
"#;

fn setup(result: &str, hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(TAMER)
        .unwrap()
        .from_dsl_yaml(result)
        .unwrap()
        .from_dsl_yaml(OBSERVER)
        .unwrap()
        .from_dsl_yaml(OPTION)
        .unwrap()
        .hand(0, hand)
        .deck(0, &["TEST-OBSERVER"; 6])
        .memory(5)
        .start();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "TEST-OBSERVER", Some(0));
    r
}

#[test]
fn printed_thunderbirdmon_route_stacks_and_draws_without_digimon_triggers() {
    let mut r = setup(RESULT, &["TEST-THUNDERBIRD"]);
    let host = r.place_stack(0, &["TEST-OBSERVER", "TEST-KURATA"]);
    let other_tamer = r.place_on_field(0, "TEST-OBSERVER", Some(0));
    r.game.players[0].battle_area[host.index as usize].card_sources[0].face_down = true;
    r.game.players[0].battle_area[host.index as usize].is_suspended = true;
    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 1),
    );

    let action = encode_digivolve(0, host.index.into());
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[action as usize], 1.0);
    assert_eq!(
        mask[encode_digivolve(0, other_tamer.index.into()) as usize],
        0.0
    );
    let before = r.hand_size(0);
    r.game.decode_action(action, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.hand_size(0), before);
    let perm = &r.game.players[0].battle_area[host.index as usize];
    let cards = &r.game.card_data;
    assert_eq!(
        perm.card_sources
            .iter()
            .map(|c| c.card_id(cards))
            .collect::<Vec<_>>(),
        ["TEST-OBSERVER", "TEST-KURATA", "TEST-THUNDERBIRD"]
    );
    assert!(perm.is_suspended);
    assert!(perm.card_sources[0].face_down);
}

#[test]
fn bio_hybrid_effect_can_offer_and_digivolve_the_tamer_from_trash_for_free() {
    let mut r = setup(RESULT, &["TEST-BIO-HYBRID", "TEST-THUNDERBIRD"]);
    let host = r.place_on_field(0, "TEST-KURATA", Some(0));
    let result = r.game.players[0].hand.remove(1);
    r.game.players[0].put_in_trash(result);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotDigivolveDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    r.choose_prompt_card("TEST-KURATA", None).unwrap();
    r.choose_prompt_card("TEST-THUNDERBIRD", None).unwrap();
    assert_eq!(r.memory(), 5);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-THUNDERBIRD"
    );
    assert!(!r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "TEST-THUNDERBIRD"));
}

#[test]
fn legacy_named_tamer_route_still_dispatches_ordinary_digivolution_triggers() {
    let result = RESULT
        .replace("    source_treated_as: tamer\n", "")
        .replace(
            "trait_has: \"Kurata's Army\"",
            "name_is: \"Nanami, Kouki, & Ivan\"",
        );
    let mut r = setup(&result, &["TEST-THUNDERBIRD"]);
    let host = r.place_on_field(0, "TEST-KURATA", Some(0));
    let action = encode_digivolve(0, host.index.into());
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    if let Some(prompt) = r.pending_selection() {
        assert_eq!(prompt.kind, SelectionKind::TriggerOrder);
        let action = prompt.valid_action_ids[0];
        r.execute_action(0, action).unwrap();
    }
    assert_eq!(
        r.memory(),
        5,
        "pay 2, then both digivolution triggers grant 1"
    );
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEST-THUNDERBIRD"
    );
}
