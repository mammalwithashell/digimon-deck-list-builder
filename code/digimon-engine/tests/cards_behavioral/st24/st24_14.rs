//! ST24-14 Yoshino Fujieda & Keenan Crier — Tamer, Cost 4. Colors: Green, Purple.
//! Traits: DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! [Start of Your Main Phase] [On Play] You may place the top card of your deck
//!   face down under this Tamer. Then, if your opponent has a Digimon, gain 1
//!   memory.
//! [All Turns] When effects trash cards from under this Tamer, by suspending this
//!   Tamer, suspend 1 of your opponent's Digimon.
//! Inherited: [Security] Play this card without paying the cost.
//!
//! (DCGO carries a [Rule] name-alias — Yoshino Fujieda / Keenan Crier — which is
//!  metadata only and not authored, exactly as ST23-13/14 omit theirs.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Green/ST24_14.cs
//!
//! # Patterns — sister to ST23-14 (trash-trigger) but [All Turns] (no your-turn
//! gate) and the payoff is suspend an opponent Digimon (DCGO Mode.Tap).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-14";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tamer_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn trigger_trash_under(
    runner: &mut DebugRunner,
    host: PermanentHandle,
) {
    let host_card = runner.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();
    let trashed_card = runner.game.players[0].battle_area[host.index as usize].card_sources[0]
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolutionCardTrashed,
        TriggerSource::SourceTrashedFromStack {
            player: 0,
            host,
            host_card,
            card: trashed_card,
            cause: EventCause::OwnEffect,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_14_compiles_as_dual_color_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    assert_eq!(card.color.len(), 2, "dual-color (green + purple)");
}

#[test]
fn st24_14_has_somp_onplay_trash_trigger_and_security_clauses() {
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

/// The trash trigger is [All Turns] — it carries NO your-turn gate.
#[test]
fn st24_14_trash_trigger_is_all_turns() {
    use digimon_dsl::compiled::CompiledPredicate;
    let card = compiled(CARD_ID);
    let trash = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnDigivolutionCardTrashed] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("trash trigger");

    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }
    assert!(
        trash.active_when.as_ref().map_or(true, |p| !has_your_turn(p)),
        "[All Turns] trash trigger must NOT carry a your-turn gate"
    );
}

// ─── Section 2 — [SOMP][On Play] place top deck FD + conditional memory ──────

#[test]
fn st24_14_on_play_places_and_gains_memory_when_opp_has_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(opp_digimon("OPP"))
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
    let _v = runner.pending_selection_view().expect("place Yes/No installs");
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
fn st24_14_memory_gain_fires_even_when_place_declined() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(opp_digimon("OPP"))
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

// ─── Section 3 — trash-under-this-Tamer → suspend self → suspend opp Digimon ──

#[test]
fn st24_14_trash_under_tamer_suspends_self_and_suspends_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(opp_digimon("OPP"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let tamer = runner.place_stack(0, &["STASH", CARD_ID]);
    let opp = runner.place_on_field(1, "OPP", Some(0));
    assert!(!runner.game.players[1].battle_area[opp.index as usize].is_suspended);

    trigger_trash_under(&mut runner, tamer);

    assert!(
        runner.pending_selection().is_some(),
        "the host-scoped trash trigger installs an optional accept prompt"
    );
    runner.accept_optional_trigger().expect("accept suspend+suspend");
    let v = runner
        .pending_selection_view()
        .expect("opponent-Digimon pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer is suspended (activation cost paid)"
    );
    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the chosen opponent Digimon is suspended"
    );
}

/// NEGATIVE (host gate): a trash on a DIFFERENT permanent's stack does NOT
/// trigger this Tamer's clause.
#[test]
fn st24_14_trash_under_other_permanent_does_not_trigger() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(opp_digimon("OPP"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_stack(0, &["STASH", "STASH"]);

    trigger_trash_under(&mut runner, other);
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "a trash on a different permanent's stack must NOT suspend this Tamer"
    );
}
