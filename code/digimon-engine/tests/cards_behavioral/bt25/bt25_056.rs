//! BT25-056 Bootmon — Digimon, Lv.5, Green/Blue, DP 7000, Cost 7.
//! Trait line: Super Boot (App Name) — Tool.
//!
//! # Card text (DCGO BT25_056.cs — authoritative)
//! <Barrier> (self).
//! Self link-condition: link onto an [Appmon] host for link cost 3.
//! Alt-digivolve: from a Lv.4 [Super App] for cost 3.
//! App Fusion (Logimon & Craftmon) — BLOCKED (no App Fuse primitive). OMITTED.
//! [On Play]/[When Digivolving]/[When Attacking]: if your turn, you may link 1
//!   [Social]/[Tool]/[Game] Digimon card from your HAND or this Digimon's
//!   digivolution cards to THIS Digimon, cost reduced by 2.
//! [All Turns] When this Digimon gets linked: suspend 1 opp Digimon/Tamer.
//! Inherited [When Linking]: return 1 opp suspended Digimon to deck bottom.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_056.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-056";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-056 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-HAND", 4, 4000, 4, &["Tool"]))
        .add_card(make_digimon("OPP-A", 4, 4000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

#[test]
fn bt25_056_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Bootmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
}

#[test]
fn bt25_056_has_link_condition_appmon_cost_3() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 3
        )
    });
    assert!(has, "BT25-056 declares a self link-condition with cost 3");
}

#[test]
fn bt25_056_grants_barrier() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let boot = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(boot, Keyword::Barrier),
        "BT25-056 has <Barrier>"
    );
}

#[test]
fn bt25_056_on_play_links_tool_from_hand_then_suspends_opponent() {
    let mut r = base()
        .hand(0, &[CARD_ID, "TOOL-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let boot_idx = r.play(0, 0).expect("Bootmon played");
    // On Play (your turn) installs the link selection over the hand Tool card.
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[boot_idx].linked_cards.len(),
        1,
        "the Tool card from hand attached to Bootmon"
    );

    // When-linked: suspend 1 opponent permanent.
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked suspend prompt surfaces"
    );
    let suspend_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, suspend_action);

    assert!(
        r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "opponent Digimon suspended after Bootmon got linked"
    );
}
