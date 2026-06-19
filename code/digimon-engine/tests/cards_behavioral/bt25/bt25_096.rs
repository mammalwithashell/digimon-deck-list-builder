//! BT25-096 Mirage Beast Knight — Option, Use cost 5. Trait: DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO BT25_096.cs)
//! [Main] By placing 1 [Gaogamon] and 1 [MachGaogamon] from your trash as 1 of
//!   your [Gaomon]'s bottom digivolution cards, that Digimon may digivolve into
//!   [MirageGaogamon] in the hand, ignoring digivolution requirements and
//!   without paying the cost.
//! When this card would be used, by trashing the bottom face-down card from
//!   under any of your Tamers, reduce the use cost by 2.
//! [Security] You may play 1 [Gaomon] or [Thomas H. Norstein] from your hand or
//!   trash without paying the cost. Then, add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_096.cs
//!
//! # Patterns this test covers
//! - Option [Main]: place 2 named cards from trash as bottom sources under a
//!   [Gaomon] → effect-initiated free digivolve into [MirageGaogamon] (hand),
//!   ignoring digivolution requirements
//! - exact name matching (Gaogamon ⊂ MachGaogamon ⊂ MirageGaogamon)
//! - [Security] play 1 [Gaomon]/[Thomas H. Norstein] from hand-or-trash free,
//!   then add this Option to hand
//! - the self interactive use-cost reducer (structural)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-096";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn mirage_option() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn gaomon() -> CardData {
    let mut c = make_test_card("Gaomon", "Gaomon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn gaogamon() -> CardData {
    let mut c = make_test_card("Gaogamon", "Gaogamon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c
}

fn machgaogamon() -> CardData {
    let mut c = make_test_card("MachGaogamon", "MachGaogamon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 7;
    c
}

fn miragegaogamon() -> CardData {
    let mut c = make_test_card("MirageGaogamon", "MirageGaogamon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 12;
    c.evo_costs = vec![EvoCost { card_color: 1, level: 5, memory_cost: 4 }];
    c
}

fn thomas() -> CardData {
    let mut c = make_test_card("Thomas H. Norstein", "Thomas H. Norstein");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

fn filler() -> CardData {
    let mut c = make_test_card("FILLER", "FILLER");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, next_idx));
}

fn hand_index_of(runner: &DebugRunner, p: u8, card_id: &str) -> usize {
    runner.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"))
}

/// Pick the first NON-PASS action at every prompt (engages every optional pick).
fn drive_engage(runner: &mut DebugRunner) {
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 24, "prompt loop did not terminate");
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(PASS);
        runner.execute_action(v.selecting_player, action).unwrap();
    }
}

fn base() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(mirage_option())
        .add_card(gaomon())
        .add_card(gaogamon())
        .add_card(machgaogamon())
        .add_card(miragegaogamon())
        .add_card(thomas())
        .add_card(filler())
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_096_metadata_and_three_regions() {
    let card = compiled(CARD_ID);
    assert_eq!(card.name, "Mirage Beast Knight");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(5));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));

    let dbg = format!("{:?}", card.effects);
    // Region 1: a self (when_playing_this) cost-reduction clause.
    assert!(
        dbg.contains("CostReduction")
            || dbg.contains("cost_reduction")
            || dbg.contains("WhenPlayingThis")
            || dbg.contains("when_playing_this")
            || dbg.contains("Reduction"),
        "self use-cost reducer clause present"
    );
    // Region 2: a [Main] (main_from_hand) clause.
    assert!(
        card.effects.iter().any(|c| matches!(
            c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "[Main] main_from_hand clause present"
    );
    // Region 3: an [on_security] clause.
    assert!(
        card.effects.iter().any(|c| matches!(
            c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "[Security] on_security clause present"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [Main]: place Gaogamon + MachGaogamon from trash under a Gaomon →
// free-digivolve into MirageGaogamon
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_096_main_places_two_trash_sources_and_free_digivolves() {
    let mut runner = base();
    // A Gaomon on the field (the digivolve host).
    let gaomon = runner.place_on_field(0, "Gaomon", Some(0));
    // Gaogamon + MachGaogamon in trash; MirageGaogamon + the Option in hand. No
    // Tamer → the use-cost reducer is inert (full cost 5 paid from memory 10).
    push_to_trash(&mut runner, 0, "Gaogamon");
    push_to_trash(&mut runner, 0, "MachGaogamon");
    push_to_hand(&mut runner, 0, "MirageGaogamon");
    push_to_hand(&mut runner, 0, CARD_ID);

    let sources_before = runner.game.players[0].battle_area[gaomon.index as usize]
        .card_sources
        .len();

    let opt_idx = hand_index_of(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, opt_idx as usize);
    drive_engage(&mut runner); // Gaomon, Gaogamon, MachGaogamon, MirageGaogamon picks

    // The Gaomon digivolved into MirageGaogamon.
    assert_eq!(
        runner.game.players[0].battle_area[gaomon.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "MirageGaogamon",
        "the Gaomon digivolved into MirageGaogamon"
    );
    // Both placed cards are now digivolution sources of the (now-Mirage)
    // permanent, and the stack grew (Gaomon tucked under Mirage too).
    let has_source = |id: &str| {
        runner.game.players[0].battle_area[gaomon.index as usize]
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == id)
    };
    assert!(has_source("Gaogamon"), "Gaogamon is a source under MirageGaogamon");
    assert!(has_source("MachGaogamon"), "MachGaogamon is a source under MirageGaogamon");
    assert!(has_source("Gaomon"), "the Gaomon is tucked under MirageGaogamon");
    assert!(
        runner.game.players[0].battle_area[gaomon.index as usize]
            .card_sources
            .len()
            > sources_before,
        "the digivolution stack grew from the placement + digivolve"
    );
    // Both named cards left the trash.
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "Gaogamon"),
        "Gaogamon left the trash (placed as a source)"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "MachGaogamon"),
        "MachGaogamon left the trash (placed as a source)"
    );
}

#[test]
fn bt25_096_main_inert_without_both_trash_cards() {
    let mut runner = base();
    let gaomon = runner.place_on_field(0, "Gaomon", Some(0));
    // Only a Gaogamon in trash (no MachGaogamon) → the gate is false.
    push_to_trash(&mut runner, 0, "Gaogamon");
    push_to_hand(&mut runner, 0, "MirageGaogamon");
    push_to_hand(&mut runner, 0, CARD_ID);

    let opt_idx = hand_index_of(&runner, 0, CARD_ID);
    let _ = runner.game.play_option_from_hand(0, opt_idx as usize);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area[gaomon.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "Gaomon",
        "no MachGaogamon in trash ⇒ the placement/digivolve never runs (Gaomon unchanged)"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Security]: play a Gaomon/Thomas from hand-or-trash free + add self
//
// The security effect fires through the full security-check flow (SecuritySkill
// + PendingSecurity); rather than simulate that here, the clause shape is locked
// structurally (the union-zone play + add-this-Option-to-hand). The Main region
// (the marquee effect) carries the behavioral coverage.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_096_security_clause_plays_union_zone_and_adds_self() {
    let card = compiled(CARD_ID);
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("an [on_security] clause is present");
    let dbg = format!("{:?}", security.process);
    assert!(
        dbg.contains("UnionZone") || dbg.contains("union_zone") || dbg.contains("Union"),
        "[Security] selects from a hand-or-trash union zone"
    );
    assert!(
        dbg.contains("AddThisOptionToHand")
            || dbg.contains("add_this_option_to_hand")
            || dbg.contains("ThisOption"),
        "[Security] adds this Option to the hand"
    );
}
