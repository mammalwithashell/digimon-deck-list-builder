//! BT25-070 Logamon — Digimon, Lv.4, Purple, DP 6000, Cost 5.
//!
//! # Card text (DCGO BT25_070.cs — authoritative)
//! Self link-condition: link onto an [Appmon] host for link cost 2.
//! Alt-digivolve: from a Lv.3 [Standard App] for cost 2.
//! App Fusion (Offmon & Hackmon) — BLOCKED (no App Fuse primitive). OMITTED.
//! [Main][OPT][Once Per Turn]: link 1 [Social]/[Tool]/[Game] Digimon card from
//!   your trash or this Digimon's digivolution cards to THIS Digimon, cost -1.
//! [Your Turn][OPT][Once Per Turn] When this Digimon gets linked: delete 1 opp
//!   Digimon with play cost 4 or less.
//! Inherited [When Linking]: 1 opp Digimon/Tamer can't unsuspend until turn end.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_070.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-070";

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
        .expect("BT25-070 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-TRASH", 4, 4000, 4, &["Tool"]))
        .add_card(make_digimon("OPP-SMALL", 3, 3000, 3, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 5, 8000, 8, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn fire_main(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

#[test]
fn bt25_070_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Logamon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
}

#[test]
fn bt25_070_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
        )
    });
    assert!(has, "BT25-070 declares a self link-condition with cost 2");
}

#[test]
fn bt25_070_main_links_then_when_linked_deletes_small_opp() {
    let mut r = base()
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    let loga = r.place_on_field(0, CARD_ID, Some(0));
    let opp_small = r.place_on_field(1, "OPP-SMALL", Some(0)); // cost 3 — deletable
    let opp_big = r.place_on_field(1, "OPP-BIG", Some(0)); // cost 8 — safe
    advance_to_main(&mut r);

    let opp_before = r.battle_area_size(1);

    // [Main] activated self-link from trash.
    assert!(
        fire_main(&mut r, 0, loga.index as usize),
        "[Main] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    assert_eq!(
        r.game.player(0).battle_area[loga.index as usize]
            .linked_cards
            .len(),
        1,
        "Tool card linked from trash"
    );

    // When-linked: delete 1 opp Digimon cost <=4 (only OPP-SMALL eligible).
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked delete prompt surfaces"
    );
    let del_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, del_action);

    assert_eq!(
        r.battle_area_size(1),
        opp_before - 1,
        "the cost-<=4 opponent Digimon was deleted"
    );
}
