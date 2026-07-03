//! BT25-100 Iron Slash — Option, Black, Cost 3, [TS].
//!
//! # Card text (cards.json)
//! <Use Req. ([TS] trait)> (Specified cards let you ignore color requirements.)
//! [Security] Activate this card's [Main] effects.
//! [Main] <De-Digivolve 2> 1 of your opponent's Digimon. (Trash up to 2 cards
//!   from the top. You can't trash past level 3 cards.) Then, you may link this
//!   card to 1 of your Digimon on the field without paying the cost.
//! Inherited: <Piercing>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_100.cs
//!
//! # Patterns this test covers
//! - D3 color ignore / bypass (Use Req. flood gate)
//! - C1/C3-adjacent: link_to_own_digimon (Option self-link)
//! - De-Digivolve as a [Main] effect
//! - [Security] activates [Main]
//! - H3 Piercing (inherited keyword grant)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt25/BT25-100.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_100_structure_use_req_main_security_piercing_and_link_requirement() {
    let runner = iron_runner().start();
    let compiled = runner
        .compiled_card("BT25-100")
        .expect("BT25-100 compiled card present");

    assert_eq!(compiled.card, "BT25-100");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(compiled.use_requirement.is_some());

    // Color-ignore flood gate.
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));

    // [Main]: de-digivolve then optional free link.
    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::DeDigivolve { .. })));
    assert!(main.process.iter().any(|s| matches!(
        s,
        CompiledStep::LinkToOwnDigimon {
            optional: true,
            free: true,
            ..
        }
    )));

    // [Security] activates the same [Main] effects (inherited scope).
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited && t.when == vec![CompiledTiming::OnSecurity]
    )));

    // Inherited link requirement.
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 2, filter, .. })
            if filter.trait_has.as_deref() == Some("TS")
    )));
}

// Link-card inherited (ESS) effects — e.g. <Piercing> here — are aggregated
// onto the host as of G-LINK-INHERITED-ESS closure (2026-07): the collectors'
// linked-card passes now fold a link card in as an inherited-style source, so a
// `scope: inherited` `<Piercing>` grant on the linked Option reaches the host.
#[test]
fn bt25_100_grants_inherited_piercing_to_host() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    runner
        .execute_action(lview.selecting_player, encode_attack(0, host.index as u16))
        .expect("link Iron Slash to host");

    let host_handle = PermanentHandle {
        player: 0,
        index: host.index,
    };
    assert!(
        runner.game.has_keyword(host_handle, Keyword::Piercing),
        "host hosting Iron Slash should gain inherited <Piercing>"
    );
}

// ── Section 2: behavioral — [Main] ───────────────────────────────────────

#[test]
fn bt25_100_main_de_digivolves_two_then_can_decline_link() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    // A TS Tamer satisfies the Use Req. (color ignore) but is not a legal link
    // host (the link filter is kind: digimon), so no link prompt installs.
    runner.place_on_field(0, "TS-TAMER", Some(0));
    // Opponent Lv5 with a 3-card stack (Lv3 → Lv4 → Lv5).
    let opp = runner.place_field_stack(1, &["OPP-LV3", "OPP-LV4", "OPP-LV5"], false, 1);
    runner.game.enter_main_phase();

    let trash_before = runner.trash_size(1);
    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    assert_eq!(tview.kind, SelectionKind::OppField);
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    // De-Digivolve 2 trashes the top two sources (Lv5 + Lv4), leaving Lv3 top.
    assert_eq!(
        runner.trash_size(1),
        trash_before + 2,
        "De-Digivolve 2 trashes two source cards"
    );
    let top_id = runner.game.player(1).battle_area[opp.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top_id, "OPP-LV3", "top card is now the Lv3");

    // Only a TS Tamer is on our field (no Digimon), so the optional link step
    // finds no legal host and resolution completes with no pending selection.
    assert!(
        runner.pending_selection().is_none(),
        "no Digimon host → link step skipped, effect resolves"
    );
}

#[test]
fn bt25_100_main_links_to_chosen_digimon() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    runner
        .execute_action(lview.selecting_player, encode_attack(0, host.index as u16))
        .expect("link to host");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].card_id(&runner.game.card_data), "BT25-100");
}

// ── Section 3: gating — link offered only when a legal host exists ────────

#[test]
fn bt25_100_no_link_prompt_when_no_own_digimon() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    // TS Tamer satisfies Use Req. but is not a legal link host (kind: digimon).
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    // No own Digimon → no link prompt installs; resolution completes.
    assert!(
        runner.pending_selection().is_none(),
        "no link host → no link selection installs"
    );
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn iron_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-100 YAML loads")
        .add_card(make_digimon("OPP-DIGIMON", 4))
        .add_card(make_digimon("OPP-LV3", 3))
        .add_card(make_digimon("OPP-LV4", 4))
        .add_card(make_digimon("OPP-LV5", 5))
}

fn play_iron_standard(runner: &mut digimon_engine::debug_runner::DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let first = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, first)
            .expect("choose Standard [Main] mode");
    }
    result
}

fn make_ts_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_digimon(id, level);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_ts_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["TS".to_string()];
    card
}
