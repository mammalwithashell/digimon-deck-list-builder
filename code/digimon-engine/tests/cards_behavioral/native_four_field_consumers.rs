//! Bounded consumers for the four remaining captured metadata differences.
//!
//! Both origins use the complete repository YAML. `loaded=true` supplies the
//! exact four-record generated fixture through CardData::load_from_str first;
//! `loaded=false` exercises the YAML metadata fallback. The fixture is consumer
//! input, not printed-data authority. Raw sequences and duplicates are retained.
//! These are synthetic grammar/zone fixtures, not legal full-deck executions or
//! proof of all card effects, arbitrary trait-list consumers, or oracle parity.

use digimon_engine::action::{build_action_mask, space::{PASS, PLAY_HAND_START}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use std::collections::HashMap;

const JUPITER: &str = "BT26-033";

fn yaml(id: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("cards").join(id.split('-').next().unwrap().to_lowercase())
        .join(format!("{id}.yaml"))).expect("complete repository YAML")
}

fn production() -> HashMap<String, CardData> {
    CardData::load_from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/native_four_field_consumers_v6.json")))
        .expect("exact generated four-record consumer fixture")
}

fn expected_traits(id: &str, loaded: bool) -> Vec<String> {
    let values: &[&str] = match (id, loaded) {
        ("BT21-071", true) => &["Sup.", "Appmon", "Tool", "Monitoring", "Appmon"],
        ("BT21-071", false) => &["Tool", "Sup.", "Appmon", "Tool", "Monitoring"],
        ("BT21-074", true) => &["Ult.", "Appmon", "Navi", "GPS", "Appmon"],
        ("BT21-074", false) => &["Navi", "Ult.", "Appmon", "GPS"],
        ("EX12-076", true) => &["Mega", "Hybrid", "Vaccine", "Shaman", "Shambala", "SW", "TB", "TS"],
        ("EX12-076", false) => &["Mega", "Vaccine", "Shaman", "Hybrid", "Shambala", "SW", "TB", "TS"],
        _ => panic!("no captured trait expectation for {id}"),
    };
    values.iter().map(|s| (*s).to_string()).collect()
}

fn witness(id: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![CardColor::White];
    card.traits = vec![trait_name.to_string()];
    card
}

fn hand(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize].hand.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn trash(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize].trash.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn field(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize].battle_area.iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string()).collect()
}

fn main_phase(r: &mut DebugRunner, seat: u8, memory: i16) {
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = memory;
    r.game.current_phase = GamePhase::Main;
    r.game.turn_count = 2;
    assert!(r.pending_selection().is_none());
}

fn probe(kind: &str, query: &str) -> String {
    format!(r#"
card: MEMBERSHIP-PROBE
name: Membership Probe
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
          filter: {{ {kind}: "{query}" }}
          prompt: Select a card with the requested trait
      - trash_from_hand_by_index: {{ of: you, hand_index: picked }}
"#)
}

fn membership(id: &str, loaded: bool, seat: u8, kind: &str, query: &str, offered: bool) {
    let mut builder = DebugRunner::builder();
    if loaded { builder = builder.with_card_data(production()); }
    let mut r = builder.from_dsl_yaml(&yaml(id)).expect("full subject YAML")
        .from_dsl_yaml(&probe(kind, query)).expect("explicit selection probe")
        .add_card(witness("MATCH-A", query)).add_card(witness("MATCH-B", query))
        .add_card(witness("WRONG", "Control-Not-A-Trait"))
        .add_card(witness("INERT", "Control-Not-A-Trait"))
        .hand(seat, &["MEMBERSHIP-PROBE", id, "MATCH-A", "MATCH-B", "WRONG"])
        .hand(1 - seat, &[id, "MATCH-A"])
        .deck(seat, &["INERT"; 8]).deck(1 - seat, &["INERT"; 8]).build();
    let actual = r.game.card_data.iter().find(|c| c.card_id == id).unwrap();
    assert_eq!(actual.traits, expected_traits(id, loaded), "no order/multiplicity coercion");
    assert_eq!(serde_json::to_value(actual).unwrap()["traits"],
        serde_json::json!(expected_traits(id, loaded)), "raw serialization stays observable");

    // A separate fixture clone isolates permanent membership from the hand
    // selection. Direct placement is setup only, not a claim about play legality.
    let mut on_field = DebugRunner::wrap(r.game.clone());
    let handle = on_field.place_on_field(seat, id, Some(0));
    let perm = &on_field.game.players[seat as usize].battle_area[handle.index as usize];
    assert_eq!(perm.has_trait(query, &on_field.game.card_data), offered);
    assert_eq!(perm.has_trait_for_rules(query, &on_field.game.card_data,
        &on_field.game.modifiers, handle), offered);

    main_phase(&mut r, seat, 10);
    assert_eq!(build_action_mask(&r.game, seat)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, seat);
    let view = r.pending_selection_view().expect("two witnesses keep the choice visible");
    assert_eq!(view.selecting_player, seat);
    assert!(!view.is_optional);
    let before = hand(&r, seat);
    assert_eq!(before, vec![id, "MATCH-A", "MATCH-B", "WRONG"]);
    let candidates: Vec<String> = view.valid_action_ids.iter().map(|a| {
        before[a.checked_sub(PLAY_HAND_START).expect("hand selection action") as usize].clone()
    }).collect();
    let expected: Vec<String> = (if offered { vec![id, "MATCH-A", "MATCH-B"] }
        else { vec!["MATCH-A", "MATCH-B"] }).into_iter().map(String::from).collect();
    assert_eq!(candidates, expected, "{id} loaded={loaded} seat={seat} {kind}={query}");
    let mask = build_action_mask(&r.game, seat);
    let subject = PLAY_HAND_START + before.iter().position(|x| x == id).unwrap() as u16;
    let wrong = PLAY_HAND_START + before.iter().position(|x| x == "WRONG").unwrap() as u16;
    assert_eq!(mask[subject as usize], if offered { 1.0 } else { 0.0 });
    assert_eq!(mask[wrong as usize], 0.0);
    assert_eq!(mask[PASS as usize], 0.0);
    assert!(build_action_mask(&r.game, 1 - seat).iter().all(|x| *x == 0.0));
    let chosen = if offered { id } else { "MATCH-A" };
    let action = PLAY_HAND_START + before.iter().position(|x| x == chosen).unwrap() as u16;
    assert!(view.valid_action_ids.contains(&action));
    assert_eq!(mask[action as usize], 1.0);
    let mut wrong_actor = DebugRunner::wrap(r.game.clone());
    assert!(wrong_actor.execute_action(1 - seat, action).is_err());
    assert_eq!(hand(&wrong_actor, seat), before);
    r.execute_action(view.selecting_player, action).expect("explicit legal owner choice");
    assert!(r.pending_selection().is_none());
    assert_eq!(trash(&r, seat), vec![chosen]);
    assert_eq!(r.hand_size(seat), 3);
    assert_eq!(hand(&r, 1 - seat), vec![id, "MATCH-A"]);
    assert_eq!(r.memory(), 10);
    assert_eq!(hand(&r, seat).contains(&id.to_string()), !offered);
}

fn appmon(id: &str, loaded: bool, grade: &str, attribute: &str, type_name: &str,
          wrong_grade: &str, wrong_attribute: &str) {
    for seat in [0, 1] {
        for (kind, query, offered) in [
            ("form_is", grade, true), ("form_is", "Appmon", true),
            ("attribute_is", attribute, true), ("trait_has", type_name, true),
            ("trait_has", "Appmon", true), ("form_is", wrong_grade, false),
            ("attribute_is", wrong_attribute, false), ("trait_has", "Virus", false),
        ] { membership(id, loaded, seat, kind, query, offered); }
    }
}

#[test] fn scopemon_json() { appmon("BT21-071", true, "Sup.", "Tool", "Monitoring", "Ult.", "Navi"); }
#[test] fn scopemon_yaml() { appmon("BT21-071", false, "Sup.", "Tool", "Monitoring", "Ult.", "Navi"); }
#[test] fn satellamon_json() { appmon("BT21-074", true, "Ult.", "Navi", "GPS", "Sup.", "Tool"); }
#[test] fn satellamon_yaml() { appmon("BT21-074", false, "Ult.", "Navi", "GPS", "Sup.", "Tool"); }

fn susanoomon(loaded: bool) {
    for seat in [0, 1] {
        for (kind, query, offered) in [
            ("form_is", "Mega", true), ("form_is", "Hybrid", true),
            ("trait_has", "Hybrid", true), ("attribute_is", "Vaccine", true),
            ("trait_has", "Shaman", true), ("trait_has", "Shambala", true),
            ("trait_has", "SW", true), ("trait_has", "TB", true), ("trait_has", "TS", true),
            ("form_is", "Champion", false), ("attribute_is", "Virus", false),
            ("trait_has", "Appmon", false),
        ] { membership("EX12-076", loaded, seat, kind, query, offered); }
    }
}

#[test] fn susanoomon_json() { susanoomon(true); }
#[test] fn susanoomon_yaml() { susanoomon(false); }

fn enabler(id: &str, color: CardColor, ts: bool) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = vec![color];
    card.traits = if ts { vec!["TS".to_string()] } else { vec![] };
    card
}

fn two_color_enabler() -> CardData {
    let mut card = enabler("YELLOW-RED-PLAIN", CardColor::Yellow, false);
    card.colors.push(CardColor::Red);
    card
}

fn jupiter(loaded: bool, seat: u8, security: usize, memory: i16) -> DebugRunner {
    let mut builder = DebugRunner::builder();
    if loaded { builder = builder.with_card_data(production()); }
    let mut ordinary = make_test_card("ORDINARY", "Ordinary Digimon");
    ordinary.colors = vec![CardColor::White];
    ordinary.play_cost = 3;
    let mut high = make_test_card("HIGH", "Higher DP");
    high.dp = Some(4000);
    let mut r = builder.from_dsl_yaml(&yaml(JUPITER)).expect("complete Dual YAML")
        .add_card(ordinary).add_card(high).add_card(make_test_card("LOW", "Lower DP"))
        .add_card(make_test_card("INERT", "Inert"))
        .add_card(enabler("WHITE-TS", CardColor::White, true))
        .add_card(enabler("WHITE-PLAIN", CardColor::White, false))
        .add_card(enabler("YELLOW-PLAIN", CardColor::Yellow, false))
        .add_card(enabler("RED-PLAIN", CardColor::Red, false))
        .add_card(two_color_enabler())
        .hand(seat, &[JUPITER, "ORDINARY"])
        .security(seat, &vec!["INERT"; security])
        .deck(seat, &["INERT"; 8]).deck(1 - seat, &["INERT"; 8]).build();
    let data = r.game.card_data.iter().find(|c| c.card_id == JUPITER).unwrap();
    assert_eq!(data.card_kind, CardKind::Dual);
    assert_eq!(data.option_use_cost(), Some(2));
    // Captured compatibility value 2 and YAML absent-cost fallback 0 are NOT
    // field-play permissions. The compiler's cost remains absent, not zero.
    assert_eq!(data.play_cost, if loaded { 2 } else { 0 });
    assert!(r.compiled_card(JUPITER).unwrap().cost.is_none());
    main_phase(&mut r, seat, memory);
    r
}

fn no_option_eligibility(loaded: bool) {
    for seat in [0, 1] {
        for scenario in ["empty", "wrong-own-trait", "opponent-only-ts", "yellow-only", "red-only"] {
            let mut r = jupiter(loaded, seat, 0, 10);
            if scenario == "wrong-own-trait" { r.place_on_field(seat, "WHITE-PLAIN", Some(0)); }
            if scenario == "opponent-only-ts" { r.place_on_field(1 - seat, "WHITE-TS", Some(0)); }
            if scenario == "yellow-only" { r.place_on_field(seat, "YELLOW-PLAIN", Some(0)); }
            if scenario == "red-only" { r.place_on_field(seat, "RED-PLAIN", Some(0)); }
            let before = field(&r, seat);
            let mask = build_action_mask(&r.game, seat);
            assert_eq!(mask[PLAY_HAND_START as usize], 0.0,
                "no field-play fallback for Dual: loaded={loaded} seat={seat} {scenario}");
            assert_eq!(mask[(PLAY_HAND_START + 1) as usize], 1.0, "ordinary Digimon is playable");
            r.game.decode_action(PLAY_HAND_START + 1, seat);
            assert_eq!(hand(&r, seat), vec![JUPITER]);
            let mut expected = before;
            expected.push("ORDINARY".to_string());
            assert_eq!(field(&r, seat), expected);
            assert!(!field(&r, seat).contains(&JUPITER.to_string()));
            assert_eq!(r.memory(), 7);
            assert!(r.pending_selection().is_none());
            assert_eq!(build_action_mask(&r.game, seat)[PLAY_HAND_START as usize], 0.0);
        }
    }
}

#[test] fn jupiter_no_option_eligibility_json() { no_option_eligibility(true); }
#[test] fn jupiter_no_option_eligibility_yaml() { no_option_eligibility(false); }

fn legal_option(loaded: bool) {
    for seat in [0, 1] {
        for coverage in ["ts-bypass", "separate-colors", "one-multicolor"] {
            for security in [0, 3] {
                let mut r = jupiter(loaded, seat, security, 10);
                // Tamer-only own board deliberately has no Arts evolution target.
                match coverage {
                    "ts-bypass" => { r.place_on_field(seat, "WHITE-TS", Some(0)); }
                    "separate-colors" => {
                        r.place_on_field(seat, "YELLOW-PLAIN", Some(0));
                        r.place_on_field(seat, "RED-PLAIN", Some(0));
                    }
                    "one-multicolor" => { r.place_on_field(seat, "YELLOW-RED-PLAIN", Some(0)); }
                    _ => unreachable!(),
                }
                let own_before = field(&r, seat);
                r.place_on_field(1 - seat, "LOW", Some(0));
                r.place_on_field(1 - seat, "LOW", Some(0));
                r.place_on_field(1 - seat, "HIGH", Some(0));
                let card = r.game.players[seat as usize].hand[0].clone();
                assert_eq!(r.game.option_use_cost(&card, seat), 2 + security as u16);
                assert_eq!(build_action_mask(&r.game, seat)[PLAY_HAND_START as usize], 1.0);
                r.game.decode_action(PLAY_HAND_START, seat);
                assert!(r.pending_selection().is_none(), "no eligible Arts target, no auto-choice");
                assert!(r.game.pending_option.is_none(), "Option lifecycle completed");
                assert_eq!(hand(&r, seat), vec!["ORDINARY"]);
                assert_eq!(field(&r, seat), own_before);
                assert_eq!(trash(&r, seat), vec![JUPITER], "used as Option, never field played");
                assert_eq!(field(&r, 1 - seat), vec!["HIGH"]);
                assert_eq!(trash(&r, 1 - seat), vec!["LOW", "LOW"]);
                assert_eq!(r.security_count(seat), security + 1);
                assert_eq!(r.game.players[seat as usize].deck.len(), 7);
                assert_eq!(r.memory(), 10 - (2 + security as i16));
                assert_eq!(r.game.current_phase, GamePhase::Main);
                assert_eq!(r.game.turn_player(), seat);
            }
        }
    }
}

#[test] fn jupiter_legal_option_json() { legal_option(true); }
#[test] fn jupiter_legal_option_yaml() { legal_option(false); }

fn live_affordability(loaded: bool) {
    for seat in [0, 1] {
        let mut r = jupiter(loaded, seat, 3, -7);
        r.place_on_field(seat, "WHITE-TS", Some(0));
        // This explicit pre-payment snapshot tests the engine's -10 floor.
        // It is a mask fixture, not a claim of a stable negative-memory turn.
        for (security, expected_cost, legal) in [(3, 5, false), (2, 4, false), (1, 3, true)] {
            assert_eq!(r.security_count(seat), security);
            let card = r.game.players[seat as usize].hand[0].clone();
            assert_eq!(r.game.option_use_cost(&card, seat), expected_cost);
            let mask = build_action_mask(&r.game, seat);
            assert_eq!(mask[PLAY_HAND_START as usize], if legal { 1.0 } else { 0.0 });
            assert_eq!(mask[(PLAY_HAND_START + 1) as usize], 1.0,
                "cost-3 ordinary Digimon is exactly affordable");
            assert_eq!(hand(&r, seat), vec![JUPITER, "ORDINARY"]);
            assert_eq!(r.memory(), -7);
            assert_eq!(field(&r, seat), vec!["WHITE-TS"]);
            if security > 1 { r.game.players[seat as usize].security.pop().unwrap(); }
        }
    }
}

#[test] fn jupiter_live_affordability_json() { live_affordability(true); }
#[test] fn jupiter_live_affordability_yaml() { live_affordability(false); }
