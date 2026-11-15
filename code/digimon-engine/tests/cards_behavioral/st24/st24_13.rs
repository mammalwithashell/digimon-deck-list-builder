//! ST24-13 Marcus Damon & Thomas H. Norstein — Tamer, Cost 4. Colors: Yellow, Blue.
//! Traits: DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! [Start of Your Main Phase] [On Play] You may place the top card of your deck
//!   face down under this Tamer. Then, if your opponent has a Digimon, gain 1
//!   memory.
//! [Your Turn] When effects trash cards from under this Tamer, by suspending this
//!   Tamer, 1 of your [DATA SQUAD] trait Digimon gains ＜Jamming＞ for the turn.
//! Inherited: [Security] Play this card without paying the cost.
//!
//! <Rule> Name: Also treated as [Marcus Damon]/[Thomas H. Norstein] — authored as
//!  `also_treated_as` (deck audit 2026-11-10: without it Gaogamon BT25-023,
//!  ShineGreymon: Burst Mode BT25-104 and ShineGreymon AD1-016 could not find it).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Yellow/ST24_13.cs
//!
//! # Patterns — sister to ST23-14 (Jamming trash-trigger) with ST23-13's
//! [Your Turn] gate, trait-swapped to [DATA SQUAD].

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

const CARD_ID: &str = "ST24-13";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tamer_data() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn ds_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_13_compiles_as_dual_color_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    assert_eq!(card.color.len(), 2, "dual-color (yellow + blue)");
}

#[test]
fn st24_13_has_somp_onplay_trash_trigger_and_security_clauses() {
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
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::OnPlay)
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
fn st24_13_on_play_places_and_gains_memory_when_opp_has_digimon() {
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
    let _v = runner
        .pending_selection_view()
        .expect("place Yes/No installs");
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
fn st24_13_memory_gain_fires_even_when_place_declined() {
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
    let v = runner
        .pending_selection_view()
        .expect("place Yes/No installs");
    let last = v.effect_choices.as_ref().unwrap().len() - 1;
    runner.execute_branch(last).expect("decline the place");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declined ⇒ nothing placed"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "memory still gained even when the place is declined"
    );
}

#[test]
fn st24_13_no_memory_when_opponent_has_no_digimon() {
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

    assert_eq!(
        runner.memory(),
        mem_before,
        "no opponent Digimon ⇒ no memory"
    );
}

// ─── Section 3 — [Your Turn] trash-under-this-Tamer → suspend self → Jamming ──

#[test]
fn st24_13_trash_under_tamer_suspends_self_and_grants_jamming() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(ds_digimon("DS"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);

    let tamer = runner.place_stack(0, &["STASH", CARD_ID]);
    let ds = runner.place_on_field(0, "DS", Some(0));
    assert!(
        !runner.game.has_keyword(ds, Keyword::Jamming),
        "no Jamming yet"
    );

    let host_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let trashed_card =
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle();

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
    runner
        .accept_optional_trigger()
        .expect("accept suspend+grant");
    let v = runner
        .pending_selection_view()
        .expect("target pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer is suspended (activation cost paid)"
    );
    assert!(
        runner.game.has_keyword(ds, Keyword::Jamming),
        "the chosen [DATA SQUAD] Digimon gains <Jamming> for the turn"
    );
}

#[test]
fn st24_13_jamming_grant_expires_at_end_of_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(ds_digimon("DS"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_stack(0, &["STASH", CARD_ID]);
    let ds = runner.place_on_field(0, "DS", Some(0));

    let host_card = runner.game.players[0].battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let trashed_card =
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].handle();
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
    assert!(
        runner.game.has_keyword(ds, Keyword::Jamming),
        "Jamming active"
    );

    runner.end_turn();
    let _ = runner.auto_resolve();
    assert!(
        !runner.game.has_keyword(ds, Keyword::Jamming),
        "the granted <Jamming> expires at end of turn"
    );
}

/// NEGATIVE (host gate): a trash on a DIFFERENT permanent's stack does NOT
/// trigger this Tamer's clause.
#[test]
fn st24_13_trash_under_other_permanent_does_not_trigger() {
    let mut runner = DebugRunner::builder()
        .add_card(tamer_data())
        .add_card(ds_digimon("DS"))
        .add_card(filler("STASH"))
        .deck(0, &["STASH"; 5])
        .deck(1, &["STASH"; 5])
        .memory(8)
        .start();
    runner.set_first_player(0);
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_stack(0, &["STASH", "DS"]);

    let host_card = runner.game.players[0].battle_area[other.index as usize]
        .top_card()
        .handle();
    let trashed_card =
        runner.game.players[0].battle_area[other.index as usize].card_sources[0].handle();
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

// ─── Section 4 — <Rule> Name alias ───────────────────────────────────────────

#[test]
fn st24_13_is_also_treated_as_marcus_damon_and_thomas_h_norstein() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.also_treated_as,
        vec!["Marcus Damon".to_string(), "Thomas H. Norstein".to_string()]
    );
    let data = tamer_data();
    assert!(data.also_treated_as.contains(&"Marcus Damon".to_string()));
    assert!(data.also_treated_as.contains(&"Thomas H. Norstein".to_string()));
}

/// Gaogamon (BT25-023): "you may play 1 [Thomas H. Norstein] from your hand
/// without paying the cost" finds this Tamer through its name alias.
#[test]
fn st24_13_is_found_by_gaogamons_thomas_h_norstein_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-13 compiles")
        .dsl_card("BT25-023")
        .expect("BT25-023 compiles")
        .add_card(filler("FILL"))
        .hand(0, &["BT25-023", CARD_ID])
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    runner.set_first_player(0);
    runner.play(0, 0);
    let _ = runner.accept_optional_trigger();
    runner.auto_resolve().expect("Gaogamon's play resolves");
    let on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(on_field, "Marcus & Thomas is played as [Thomas H. Norstein]");
}

// ─── Section 5 — [Start of Your Main Phase] alongside DNA Charge (ST24-15) ───
//
// DCGO PvP game raw-20261113T025510Z#g6: at P1's Start of Main Phase both
// Marcus Damon & Thomas H. Norstein (ST24-13)'s shared [On Play]/[Start of Your
// Main Phase] effect (ST24_13.cs:37-45, optional:false) and DNA Charge
// (ST24-15)'s [Start of Your Main Phase] (ST24_15.cs:113-190) trigger. They
// trigger simultaneously (CR 4.3 §15-16-13-1, §15-4-3-2) and the turn player
// picks the order (§15-4-3-5-1). DCGO picked DNA Charge first (MultipleSkills),
// placed it face down under this Tamer (ST24_15.cs:182
// AddDigivolutionCardsBottom), then still resolved this Tamer's Yes/No
// (ST24_13.cs:54-64) and +1 memory (:75-77). The Tamer never left the battle
// area, so it is not a new card and its pending effect can still activate
// (§15-4-4-3 cancels only an effect whose card became a new card).
//
// The engine dropped it: DNA Charge's relocate removed the Option from
// `battle_area` (a Vec) without re-keying the queued effects, so a Tamer in a
// LATER slot kept its old index and `queued_effect_source_is_live` read it as
// gone. The DNA-Charge-in-a-lower-slot shape is the g6 board.

const DNA: &str = "ST24-15";

/// ST24-13 and (optionally) DNA Charge (ST24-15) as an ordinary field Option on
/// player 0's battle area, an opponent Digimon (the memory clause's condition),
/// memory 0. `dna_first` puts DNA Charge in the LOWER slot (the g6 board).
fn somp_board(with_dna: bool, dna_first: bool) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-13 compiles")
        .dsl_card(DNA)
        .expect("ST24-15 compiles")
        .add_card(plain_digimon("OPP"))
        .add_card(filler("DECKTOP"))
        .deck(0, &["DECKTOP"; 6])
        .deck(1, &["DECKTOP"; 6])
        .memory(0)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP", Some(0));
    let place_dna = |r: &mut DebugRunner| {
        let h = r.place_on_field(0, DNA, Some(0));
        r.game.player_mut(0).battle_area[h.index as usize].option_state =
            digimon_engine::permanent::OptionState::OrdinaryFieldOption;
    };
    if with_dna && dna_first {
        place_dna(&mut runner);
    }
    runner.place_on_field(0, CARD_ID, Some(0));
    if with_dna && !dna_first {
        place_dna(&mut runner);
    }
    runner
}

fn tamer_slot(runner: &DebugRunner) -> usize {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST24-13 on the field")
}

const PLACE_PROMPT: &str = "Place the top card of your deck face down under this Tamer?";

/// Enter the main phase and answer every prompt: the `TriggerOrder` picks
/// `first`'s branch, ST24-13's Yes/No answers "Yes" (branch 0), DNA Charge's
/// Tamer pick takes the only legal Tamer. Returns how many times ST24-13's
/// Yes/No was asked.
fn run_start_of_main(runner: &mut DebugRunner, first: &str) -> usize {
    runner.game.enter_main_phase();
    let mut asked = 0;
    for _ in 0..16 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.kind == digimon_engine::selection::SelectionKind::TriggerOrder {
            let action = v
                .effect_choices
                .as_ref()
                .expect("TriggerOrder branches")
                .iter()
                .find(|c| c.label.starts_with(&format!("{first} ")))
                .unwrap_or_else(|| panic!("no {first} branch: {v:?}"))
                .action_id;
            runner.execute_action(v.selecting_player, action).unwrap();
        } else if v.prompt == PLACE_PROMPT {
            asked += 1;
            runner.execute_branch(0).expect("Yes: place the top card");
        } else {
            let action = *v
                .valid_action_ids
                .first()
                .unwrap_or_else(|| panic!("no legal action: {v:?}"));
            runner.execute_action(v.selecting_player, action).unwrap();
        }
    }
    assert!(
        runner.pending_selection().is_none(),
        "every Start of Main Phase prompt answered"
    );
    asked
}

/// Both effects resolved: ST24-13 asked once and gained 1 memory; DNA Charge
/// drew 1, gained 1 memory and sits face down under the Tamer with the placed
/// deck top card.
fn assert_both_resolved(runner: &DebugRunner, asked: usize, hand_before: usize) {
    assert_eq!(
        asked, 1,
        "ST24-13's [Start of Your Main Phase] Yes/No is asked exactly once (ST24_13.cs:54-64)"
    );
    assert_eq!(
        runner.memory(),
        2,
        "DNA Charge +1 (ST24_15.cs:186) and ST24-13 +1 (ST24_13.cs:75-77)"
    );
    assert_eq!(runner.hand_size(0), hand_before + 1, "DNA Charge <Draw 1>");
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "DNA Charge left the battle area; only the Tamer remains"
    );
    let tamer = &runner.game.players[0].battle_area[tamer_slot(runner)];
    assert_eq!(
        tamer.card_sources.len(),
        3,
        "Tamer + DNA Charge + the placed deck top card"
    );
    let ids: Vec<String> = tamer
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(ids.iter().any(|id| id == DNA), "DNA Charge under the Tamer: {ids:?}");
    assert!(
        ids.iter().any(|id| id == "DECKTOP"),
        "the deck top card under the Tamer: {ids:?}"
    );
    assert!(
        tamer.card_sources[..2].iter().all(|c| c.face_down),
        "both placed cards are face down"
    );
}

/// ST24-13 alone at the real Start of Main Phase entry (`enter_main_phase`):
/// one trigger, no order prompt, Yes places the deck top card, +1 memory.
#[test]
fn st24_13_start_of_main_phase_alone_places_and_gains_memory() {
    let mut runner = somp_board(false, false);
    let asked = run_start_of_main(&mut runner, CARD_ID);
    assert_eq!(asked, 1, "the Yes/No is asked");
    assert_eq!(runner.memory(), 1, "opponent has a Digimon => +1 memory");
    let tamer = &runner.game.players[0].battle_area[tamer_slot(&runner)];
    assert_eq!(tamer.card_sources.len(), 2, "deck top card placed under the Tamer");
    assert!(tamer.card_sources[0].face_down, "placed face down");
}

/// THE g6 SHAPE (raw-20261113T025510Z#g6, file steps 28-30): DNA Charge in the
/// lower slot, resolved first. ST24-13's pending effect must still resolve.
#[test]
fn st24_13_start_of_main_survives_dna_charge_resolving_first_from_a_lower_slot() {
    let mut runner = somp_board(true, true);
    let hand_before = runner.hand_size(0);
    let asked = run_start_of_main(&mut runner, DNA);
    assert_both_resolved(&runner, asked, hand_before);
}

/// DNA Charge in the higher slot, resolved first: its removal shifts nothing
/// below it.
#[test]
fn st24_13_start_of_main_survives_dna_charge_resolving_first_from_a_higher_slot() {
    let mut runner = somp_board(true, false);
    let hand_before = runner.hand_size(0);
    let asked = run_start_of_main(&mut runner, DNA);
    assert_both_resolved(&runner, asked, hand_before);
}

/// The other order: ST24-13 first, then DNA Charge (lower slot).
#[test]
fn st24_13_start_of_main_then_dna_charge_from_a_lower_slot() {
    let mut runner = somp_board(true, true);
    let hand_before = runner.hand_size(0);
    let asked = run_start_of_main(&mut runner, CARD_ID);
    assert_both_resolved(&runner, asked, hand_before);
}

/// The other order: ST24-13 first, then DNA Charge (higher slot).
#[test]
fn st24_13_start_of_main_then_dna_charge_from_a_higher_slot() {
    let mut runner = somp_board(true, false);
    let hand_before = runner.hand_size(0);
    let asked = run_start_of_main(&mut runner, CARD_ID);
    assert_both_resolved(&runner, asked, hand_before);
}
