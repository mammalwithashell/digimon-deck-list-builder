//! ST23-14 Reina Sakuya & Makoto Kuonji — Tamer, Cost 4. Colors: Purple, Black.
//! Traits: Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json — verbatim)
//! [Start of Your Main Phase] [On Play] You may place the top card of your deck
//! face down under this Tamer. Then, if your opponent has a Digimon, gain 1
//! memory.
//! [Your Turn] When effects trash cards from under this Tamer, by suspending
//! this Tamer, 1 of your [Glowing Dawn] trait Digimon gains ＜Jamming＞ for the
//! turn.
//! Inherited: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Black/ST23_14.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - B1 SOMP/On-Play place-top-deck-FD-under-self + conditional memory gain
//! - on_digivolution_card_trashed host-scoped trigger (event_host_permanent_is_source)
//! - suspend-self activation cost → grant <Jamming> until end of turn
//! - H2 Jamming keyword grant; Tamer [Security] play-self

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST23-14";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tamer_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_14_compiles_as_dual_color_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
    assert!(card.traits.iter().any(|t| t == "BEATBREAK"));
    assert_eq!(card.color.len(), 2, "dual-color (purple + black)");
}

#[test]
fn st23_14_has_somp_onplay_trash_trigger_and_security_clauses() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert!(
        triggered.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)
            && t.when.contains(&CompiledTiming::StartOfYourMainPhase)),
        "[SOMP][On Play] shared clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnDigivolutionCardTrashed]),
        "on_digivolution_card_trashed clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "[Security] play-self clause present"
    );
}

// ─── Section 2 — [SOMP][On Play] place top deck FD + conditional memory ──────

#[test]
fn st23_14_on_play_places_and_gains_memory_when_opp_has_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(plain_digimon("OPP"))
        .add_card(filler("DECKTOP"))
        .deck(0, &["DECKTOP"; 3])
        .deck(1, &["DECKTOP"; 3])
        .memory(3)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP", Some(0));
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let deck_before = runner.deck_size(0);
    let sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, tamer.index as usize);
    let v = runner.pending_selection_view().expect("place Yes/No installs");
    runner.execute_branch(0).expect("accept the place");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        sources_before + 1,
        "top deck card placed face-down under the Tamer"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1, "1 card left the deck");
    assert_eq!(runner.memory(), mem_before + 1, "gained 1 memory");
}

#[test]
fn st23_14_memory_gain_fires_even_when_place_declined() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(plain_digimon("OPP"))
        .add_card(filler("DECKTOP"))
        .deck(0, &["DECKTOP"; 3])
        .deck(1, &["DECKTOP"; 3])
        .memory(3)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP", Some(0));
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.fire_on_play(0, tamer.index as usize);
    let v = runner.pending_selection_view().expect("place Yes/No installs");
    let last = v.effect_choices.as_ref().unwrap().len() - 1;
    runner.execute_branch(last).expect("decline the place");
    let _ = runner.auto_resolve();

    assert_eq!(runner.deck_size(0), deck_before, "declined ⇒ nothing placed");
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "memory still gained even when the place is declined"
    );
}

#[test]
fn st23_14_no_memory_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(filler("DECKTOP"))
        .deck(0, &["DECKTOP"; 3])
        .deck(1, &["DECKTOP"; 3])
        .memory(3)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();

    runner.fire_on_play(0, tamer.index as usize);
    if runner.pending_selection().is_some() {
        let v = runner.pending_selection_view().unwrap();
        let last = v.effect_choices.as_ref().map(|c| c.len() - 1).unwrap_or(0);
        let _ = runner.execute_branch(last);
    }
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), mem_before, "no opponent Digimon ⇒ no memory");
}

// ─── Section 3 — trash-from-under-this-Tamer trigger → suspend → Jamming ─────

/// POSITIVE: a trash event on a source under THIS Tamer (your turn) offers the
/// optional suspend-self → grant a chosen [Glowing Dawn] Digimon <Jamming> for
/// the turn.
#[test]
fn st23_14_trash_under_tamer_suspends_self_and_grants_jamming() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(gd_digimon("GD"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let tamer = runner.place_stack(0, &["STASH", CARD_ID]);
    let gd = runner.place_on_field(0, "GD", Some(0));
    assert!(
        !runner.game.has_keyword(gd, Keyword::Jamming),
        "precondition: target does not yet have Jamming"
    );

    let host_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .handle();

    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: tamer,
            host_card,
            card: trashed_card,
            cause: EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "the host-scoped trash trigger installs an optional accept prompt"
    );
    runner.accept_optional_trigger().expect("accept suspend+grant");
    let v = runner.pending_selection_view().expect("target pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer is suspended (activation cost paid)"
    );
    assert!(
        runner.game.has_keyword(gd, Keyword::Jamming),
        "the chosen [Glowing Dawn] Digimon gains <Jamming> for the turn"
    );
}

/// The granted <Jamming> expires at end of turn.
#[test]
fn st23_14_jamming_grant_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(gd_digimon("GD"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_stack(0, &["STASH", CARD_ID]);
    let gd = runner.place_on_field(0, "GD", Some(0));

    let host_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: tamer,
            host_card,
            card: trashed_card,
            cause: EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();
    runner.accept_optional_trigger().expect("accept");
    let v = runner.pending_selection_view().expect("target pick");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();
    assert!(runner.game.has_keyword(gd, Keyword::Jamming), "Jamming active");

    runner.end_turn(); // end of your turn → Jamming should clear
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.has_keyword(gd, Keyword::Jamming),
        "the granted <Jamming> expires at end of turn"
    );
}

/// NEGATIVE (host gate): a trash on a DIFFERENT permanent's stack does NOT
/// trigger this Tamer's clause.
#[test]
fn st23_14_trash_under_other_permanent_does_not_trigger() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(gd_digimon("GD"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_stack(0, &["STASH", "GD"]);

    let host_card = runner.game.players[0].battle_area[other.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].battle_area[other.index as usize].card_sources[0]
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host: other,
            host_card,
            card: trashed_card,
            cause: EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "a trash on a different permanent's stack must NOT suspend this Tamer"
    );
}
