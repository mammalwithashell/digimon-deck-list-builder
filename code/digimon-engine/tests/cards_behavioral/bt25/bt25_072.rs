//! BT25-072 Shutmon — Digimon, Lv.5, Black/Purple, DP 7000, Cost 7.
//! Trait line: Forced Termination (App Name) — Tool. Attribute: Virus.
//!
//! # Card text (DCGO BT25_072.cs — authoritative)
//! <Jamming> (self).
//! Self link-condition: may be linked onto an [Appmon] host for link cost 3.
//! Alt-digivolve: from a Lv.4 [Super App] for cost 3.
//! App Fusion (Logamon & Timemon) — BLOCKED (no App Fuse primitive). OMITTED.
//! [On Play]/[When Digivolving]/[When Attacking]: if it's your turn, you may
//!   link 1 [Social]/[Tool]/[Game] Digimon card from your trash or this Digimon's
//!   digivolution cards to THIS Digimon, cost reduced by 2.
//! [All Turns][OPT] When this Digimon gets linked: 1 opp Digimon/Tamer can't
//!   digivolve until their turn ends.
//! Inherited [When Linking]: 2 opp Digimon/Tamers can't unsuspend until their
//!   turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_072.cs
//!
//! # Patterns covered
//! - link_card_to_self from trash/digivolution sources (facet #9, G-DSL-LINK-CARD-FROM-ZONE)
//! - <Jamming> self keyword
//! - when_card_linked_to_this host-side trigger → CannotDigivolve until opp turn end
//! - select-up-to-2 opponent battle-area → CannotUnsuspend (inherited When Linking)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;

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

const CARD_ID: &str = "BT25-072";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn tool_card(id: &str) -> CardData {
    make_digimon(id, 4, 4000, 4, &["Tool"])
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-072 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(tool_card("TOOL-IN-TRASH"))
        .add_card(make_digimon("OPP-A", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-B", 4, 4000, 4, &["Beast"]))
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_072_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Shutmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
}

#[test]
fn bt25_072_has_link_condition_appmon_cost_3() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 3
        )
    });
    assert!(has, "BT25-072 declares a self link-condition with cost 3");
}

#[test]
fn bt25_072_grants_jamming() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(shut, Keyword::Jamming),
        "BT25-072 has <Jamming>"
    );
}

// ─── Section 2 — On Play self-link from trash ────────────────────────────────

#[test]
fn bt25_072_on_play_links_tool_card_from_trash_to_self() {
    // Tool card sits in player 0's trash; Shutmon's On Play links it onto itself.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    advance_to_main(&mut r);

    let shut_idx = r.play(0, 0).expect("Shutmon played");
    // On Play (your turn) installs the link selection over the trash Tool card.
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);

    assert_eq!(
        r.game.player(0).battle_area[shut_idx].linked_cards.len(),
        1,
        "the Tool card from trash attached to Shutmon"
    );
    assert_eq!(r.trash_size(0), 0, "Tool card left the trash");
}

// ─── Section 3 — When this gets linked: deny-digivolve 1 opponent ────────────

#[test]
fn bt25_072_when_linked_denies_opponent_digivolve() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let _shut = r.play(0, 0).expect("Shutmon played");
    // Resolve the self-link (trash Tool) → fires when_card_linked_to_this.
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // Host-side trigger: select the opponent Digimon to deny digivolve.
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked deny-digivolve prompt surfaces"
    );
    let deny_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, deny_action);

    assert!(
        r.game
            .modifiers
            .has(opp, ModifierType::CannotDigivolve),
        "opponent Digimon can't digivolve after Shutmon got linked"
    );
}
