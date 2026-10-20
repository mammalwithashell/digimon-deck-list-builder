//! Printed attribute omissions affect real card-selection eligibility.
//! Complete subject YAML is loaded either alone or with the frozen seven-row
//! source-derived JSON fixture. Synthetic witnesses isolate filtering grammar;
//! these tests make no legal full-deck or complete-card-behavior claim.
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use std::collections::HashMap;

fn yaml(id: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("cards")
            .join(id.split('-').next().unwrap().to_lowercase())
            .join(format!("{id}.yaml")),
    )
    .unwrap()
}
fn production() -> HashMap<String, CardData> {
    CardData::load_from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/focused_seven_attributes_v6.json"
    )))
    .unwrap()
}
fn witness(id: &str, attribute: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::White];
    card.traits = vec![attribute.to_string()];
    card
}
fn probe(attribute: &str) -> String {
    format!(
        r#"
card: ATTRIBUTE-PROBE
name: Attribute Probe
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: picked
          optional: false
          filter: {{ attribute_is: {attribute} }}
          prompt: Select a card with the requested attribute
      - trash_from_hand_by_index: {{ of: you, hand_index: picked }}
"#
    )
}
fn hand_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
fn check(id: &str, printed: &str, loaded: bool) {
    for seat in [0, 1] {
        for (query, offered) in [(printed, true), ("Virus", false)] {
            assert_ne!(printed, "Virus", "wrong-attribute control must differ");
            let mut b = DebugRunner::builder();
            if loaded {
                b = b.with_card_data(production());
            }
            b = b
                .from_dsl_yaml(&yaml(id))
                .expect("complete subject YAML compiles")
                .from_dsl_yaml(&probe(query))
                .expect("synthetic selection probe compiles")
                .add_card(witness("MATCH-A", query))
                .add_card(witness("MATCH-B", query))
                .add_card(witness("WRONG", "Control-Not-An-Attribute"))
                .add_card(witness("INERT", "Control-Not-An-Attribute"));
            let mut r = b
                .hand(
                    seat,
                    &["ATTRIBUTE-PROBE", id, "MATCH-A", "MATCH-B", "WRONG"],
                )
                .hand(1 - seat, &[id, "MATCH-A"])
                .deck(seat, &["INERT"; 8])
                .deck(1 - seat, &["INERT"; 8])
                .build();
            r.game.turn_player_idx = seat as usize;
            r.game.memory_pair = (seat, 1 - seat);
            r.game.memory = 10;
            r.game.current_phase = GamePhase::Main;
            r.game.turn_count = 2;
            assert!(r.pending_selection().is_none());
            assert_eq!(
                build_action_mask(&r.game, seat)[PLAY_HAND_START as usize],
                1.0
            );
            r.game.decode_action(PLAY_HAND_START, seat);
            let view = r
                .pending_selection_view()
                .expect("two matching witnesses force an observable prompt");
            assert_eq!(view.selecting_player, seat);
            assert!(!view.is_optional);
            let hand = hand_ids(&r, seat);
            assert_eq!(hand, vec![id, "MATCH-A", "MATCH-B", "WRONG"]);
            // Hand-selection actions encode the actual post-play hand index.
            // Bind the desired ID through current hand contents, never an ordinal choice.
            let candidates: Vec<String> = view
                .valid_action_ids
                .iter()
                .map(|&a| {
                    let index = a.checked_sub(PLAY_HAND_START).unwrap() as usize;
                    hand.get(index).expect("hand action index in range").clone()
                })
                .collect();
            let expected: Vec<String> = if offered {
                vec![id, "MATCH-A", "MATCH-B"]
            } else {
                vec!["MATCH-A", "MATCH-B"]
            }
            .into_iter()
            .map(String::from)
            .collect();
            assert_eq!(
                candidates, expected,
                "{id}, printed={printed}, query={query}, loaded={loaded}, seat={seat}"
            );
            let mask = build_action_mask(&r.game, seat);
            let subject = PLAY_HAND_START + hand.iter().position(|x| x == id).unwrap() as u16;
            let wrong = PLAY_HAND_START + hand.iter().position(|x| x == "WRONG").unwrap() as u16;
            assert_eq!(mask[subject as usize], if offered { 1.0 } else { 0.0 });
            assert_eq!(mask[wrong as usize], 0.0);
            assert_eq!(mask[PASS as usize], 0.0);
            assert!(build_action_mask(&r.game, 1 - seat)
                .iter()
                .all(|v| *v == 0.0));
            let chosen = if offered { id } else { "MATCH-A" };
            let action = PLAY_HAND_START + hand.iter().position(|x| x == chosen).unwrap() as u16;
            assert!(view.valid_action_ids.contains(&action));
            assert_eq!(mask[action as usize], 1.0);
            let mut rejected = DebugRunner::wrap(r.game.clone());
            assert!(
                rejected.execute_action(1 - seat, action).is_err(),
                "wrong seat cannot answer"
            );
            assert_eq!(hand_ids(&rejected, seat), hand);
            r.execute_action(view.selecting_player, action)
                .expect("explicit legal owner selection");
            assert!(r.pending_selection().is_none());
            assert_eq!(
                r.game.players[seat as usize]
                    .trash
                    .iter()
                    .map(|c| c.card_id(&r.game.card_data))
                    .collect::<Vec<_>>(),
                vec![chosen]
            );
            assert_eq!(r.hand_size(seat), 3);
            assert_eq!(r.hand_size(1 - seat), 2, "opponent hand untouched");
            assert_eq!(r.memory(), 10, "zero-cost probe changes no memory");
            assert_eq!(hand_ids(&r, seat).contains(&id.to_string()), !offered);
        }
    }
}
macro_rules! card_tests {
    ($yaml:ident,$json:ident,$id:literal,$attribute:literal) => {
        #[test]
        fn $yaml() {
            check($id, $attribute, false);
        }
        #[test]
        fn $json() {
            check($id, $attribute, true);
        }
    };
}
mod batch1 {
    use super::*;
    card_tests!(bt21_007_yaml, bt21_007_json, "BT21-007", "Vaccine");
    card_tests!(bt24_018_yaml, bt24_018_json, "BT24-018", "Data");
    card_tests!(bt24_031_yaml, bt24_031_json, "BT24-031", "Data");
    card_tests!(bt24_101_yaml, bt24_101_json, "BT24-101", "Vaccine");
}
mod batch2 {
    use super::*;
    card_tests!(bt25_025_yaml, bt25_025_json, "BT25-025", "Vaccine");
    card_tests!(bt25_033_yaml, bt25_033_json, "BT25-033", "Vaccine");
    card_tests!(st1_07_yaml, st1_07_json, "ST1-07", "Vaccine");
}
