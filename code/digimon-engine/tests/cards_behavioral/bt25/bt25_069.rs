//! BT25-069 Raremon — Digimon, Lv.4, Purple, DP 4000, Cost 4.
//! Trait line: Undead / Titan / TS.
//!
//! # Card text (DCGO BT25_069.cs — authoritative)
//! <Jamming> (self).
//! [On Play]/[When Digivolving]: you may link 1 [TS] trait card from your trash
//!   to 1 of YOUR Digimon without paying the cost.
//! Inherited [All Turns]: this Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_069.cs
//!
//! # Patterns covered
//! - link_card_to_self { to: chosen_own_digimon } — facet #9 chosen-host extension

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-069";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player].trash.push(CardSource::new(idx, player as u8, iid));
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-069 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TS-IN-TRASH", 3, 2000, 3, &["TS"]))
        .add_card(make_digimon("MY-HOST", 4, 4000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

#[test]
fn bt25_069_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Raremon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(4000));
}

#[test]
fn bt25_069_grants_jamming() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let rare = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(rare, Keyword::Jamming),
        "BT25-069 has <Jamming>"
    );
}

#[test]
fn bt25_069_on_play_links_ts_from_trash_to_chosen_own_digimon() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "TS-IN-TRASH");
    // A separate friendly host to receive the link.
    let my_host = r.place_on_field(0, "MY-HOST", Some(0));
    advance_to_main(&mut r);

    let rare_idx = r.play(0, 0).expect("Raremon played");
    // On Play installs the card selection (TS in trash).
    assert!(
        r.game.pending_selection.is_some(),
        "On Play link installs a card selection"
    );
    let card_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, card_action);

    // Second prompt: choose which of my Digimon receives the link.
    assert!(
        r.game.pending_selection.is_some(),
        "host-selection prompt surfaces after choosing the card"
    );
    // Pick the separate MY-HOST (not Raremon) to prove arbitrary-host linking.
    let host_action = digimon_engine::action::space::encode_attack(0, my_host.index as u16);
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .contains(&host_action),
        "MY-HOST is an eligible link host"
    );
    let _ = r.game.resolve_selection(0, host_action);

    assert_eq!(
        r.game.player(0).battle_area[my_host.index as usize]
            .linked_cards
            .len(),
        1,
        "the TS card from trash attached to the chosen friendly Digimon"
    );
    assert_eq!(r.trash_size(0), 0, "TS card left the trash");
}
