//! BT25-052 Logimon — Digimon, Lv.4, Green/Red, DP 6000, Cost 5.
//! Trait line: Login (App Name) — Social.
//!
//! # Card text (DCGO BT25_052.cs — authoritative)
//! Self link-condition: link onto an [Appmon] host for link cost 2.
//! Alt-digivolve: from a Lv.3 [Standard App] for cost 2.
//! App Fusion (Onmon & Gatchmon) — BLOCKED (no App Fuse primitive). OMITTED.
//! [Main][OPT][Once Per Turn]: link 1 [Social]/[Tool]/[Game] Digimon card from
//!   your hand or this Digimon's digivolution cards to THIS Digimon, cost -1.
//! [Your Turn][OPT][Once Per Turn] When this Digimon gets linked: if <=1 Tamers,
//!   play 1 [Kazuki & Itsuki] from hand free.
//! Inherited [When Linking]: suspend 1 opp Digimon/Tamer.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_052.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-052";

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
        .expect("BT25-052 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-HAND", 3, 2000, 3, &["Tool"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Fire the on-field [Main] activated effect of the permanent at `field_index`.
fn fire_main(runner: &mut DebugRunner, player: PlayerId, field_index: usize) -> bool {
    let handle = runner.perm_handle(player, field_index);
    runner
        .game
        .enqueue_triggered(EffectTiming::MainOnField, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
    runner.pending_selection().is_some()
}

#[test]
fn bt25_052_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Logimon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(6000));
}

#[test]
fn bt25_052_has_link_condition_appmon_cost_2() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
        )
    });
    assert!(has, "BT25-052 declares a self link-condition with cost 2");
}

#[test]
fn bt25_052_main_links_tool_from_hand_to_self() {
    let mut r = base()
        .hand(0, &["TOOL-IN-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let logi = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    // [Main] activated link from hand installs a selection over the Tool card.
    assert!(
        fire_main(&mut r, 0, logi.index as usize),
        "[Main] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[logi.index as usize]
            .linked_cards
            .len(),
        1,
        "the Tool card from hand attached to Logimon"
    );
    assert_eq!(r.hand_size(0), 0, "Tool card left the hand");
}
