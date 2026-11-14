//! ST24-15 DNA Charge — Option, White, Cost 3.
//! Traits: DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim)
//! <Use Req. ([DATA SQUAD] trait)> (Specified cards let you ignore color
//!   requirements.)
//! [Main] You may play 1 [DATA SQUAD] trait card with a play cost of 4 or less
//!   from your hand or trash without paying the cost. Then, place this card in
//!   the battle area.
//! [Start of Your Main Phase] By placing this card from the battle area face
//!   down under any of your [DATA SQUAD] trait Tamers, <Draw 1> and gain 1 memory.
//! Inherited: [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/White/ST24_15.cs
//!
//! # Patterns — sister to ST23-15 (e-Pulse), trait-swapped BEATBREAK → DATA SQUAD
//! - Option play-or-use from hand/trash free (select_union_zone + play_union_bound_free)
//! - persistent field Option ("place this card in the battle area")
//! - G-MOVE-SELF-OPTION-UNDER-PERMANENT — SOMP relocate self FD under a Tamer

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "ST24-15";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn dna_charge() -> CardData {
    card_data_from_compiled(CARD_ID)
}

/// A [DATA SQUAD] Tamer (legal relocate target).
fn ds_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

/// A NON-[DATA SQUAD] Tamer (illegal relocate target).
fn plain_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A [DATA SQUAD] Digimon with play cost ≤4 (legal hand/trash free-play target).
fn ds_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::White];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn fire_start_of_main(r: &mut DebugRunner) {
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();
}

fn process_contains_move_self_option(steps: &[CompiledStep]) -> bool {
    steps.iter().any(|s| match s {
        CompiledStep::MoveSelfOptionUnderPermanent { .. } => true,
        CompiledStep::If {
            then, else_branch, ..
        } => {
            process_contains_move_self_option(then)
                || process_contains_move_self_option(else_branch)
        }
        CompiledStep::ForEach { body, .. } | CompiledStep::PerSelected { body, .. } => {
            process_contains_move_self_option(body)
        }
        _ => false,
    })
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

// Exercise the paid Option-use path with a genuine color provider in breeding.
// The breeding provider does not alter battle-field slots or target assertions.
fn use_fixture_option(runner: &mut DebugRunner, provider: &str) {
    runner.place_in_breeding(0, provider);
    runner.game.enter_main_phase();
    let action = digimon_engine::action::space::PLAY_HAND_START;
    assert_eq!(
        digimon_engine::action::mask::build_action_mask(&runner.game, 0)[action as usize],
        1.0,
        "fixture Option must be legal before Main action dispatch"
    );
    runner.game.decode_action(action, 0);
}

#[test]
fn st24_15_is_white_data_squad_option() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn st24_15_has_main_somp_and_security_clauses() {
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
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::MainFromHand)),
        "[Main] clause present"
    );
    assert!(
        triggered.iter().any(|t| t
            .when
            .iter()
            .any(|w| *w == CompiledTiming::StartOfYourMainPhase)),
        "[Start of Your Main Phase] relocate clause present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when.iter().any(|w| *w == CompiledTiming::OnSecurity)),
        "[Security] clause present"
    );
}

#[test]
fn st24_15_somp_clause_relocates_self_under_a_tamer() {
    let card = compiled(CARD_ID);
    let somp = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when
                    .iter()
                    .any(|w| *w == CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("SOMP clause");
    assert!(
        process_contains_move_self_option(&somp.process),
        "the SOMP clause relocates this Option under a Tamer"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Main Phase] relocate → Draw 1 + gain 1 memory
// ════════════════════════════════════════════════════════════════════════════

fn setup_field_option(
    with_ds_tamer: bool,
    with_plain_tamer: bool,
) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder()
        .add_card(dna_charge())
        .add_card(ds_tamer("DST"))
        .add_card(plain_tamer("PT"))
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"));
    builder = builder
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5);
    let mut runner = builder.start();
    runner.set_first_player(0);

    if with_ds_tamer {
        runner.place_on_field(0, "DST", Some(0));
    }
    if with_plain_tamer {
        runner.place_on_field(0, "PT", Some(0));
    }
    let option = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.player_mut(0).battle_area[option.index as usize].option_state =
        digimon_engine::permanent::OptionState::OrdinaryFieldOption;
    (runner, option)
}

fn tamer_handle(runner: &DebugRunner, id: &str) -> PermanentHandle {
    runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == id)
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .unwrap_or_else(|| panic!("{id} not on field"))
}

/// POSITIVE: with a [DATA SQUAD] Tamer on field, the SOMP relocate moves
/// DNA Charge face-down under the Tamer, draws 1, and gains 1 memory. The Option
/// is no longer a standalone permanent and is NOT in trash (it moved).
#[test]
fn st24_15_somp_relocates_self_under_ds_tamer_and_draws_and_gains_memory() {
    let (mut runner, _option) = setup_field_option(true, false);
    let tamer = tamer_handle(&runner, "DST");
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    let _ = runner.auto_resolve();

    let tamer = tamer_handle(&runner, "DST");
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before + 1,
        "DNA Charge placed under the [DATA SQUAD] Tamer"
    );
    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "the placed Option is face-down"
    );
    let placed_id = runner.game.players[0].battle_area[tamer.index as usize].card_sources[0]
        .card_id(&runner.game.card_data);
    assert_eq!(placed_id, CARD_ID, "the placed source is DNA Charge");

    assert_eq!(
        runner.battle_area_size(0),
        ba_before - 1,
        "DNA Charge is no longer a standalone field permanent"
    );
    assert_eq!(runner.hand_size(0), hand_before + 1, "<Draw 1>");
    assert_eq!(runner.memory(), mem_before + 1, "gain 1 memory");
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "DNA Charge is moved (not trashed) — trash unchanged"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "DNA Charge must NOT be in trash"
    );
}

#[test]
fn st24_15_somp_relocate_is_declinable() {
    let (mut runner, _option) = setup_field_option(true, false);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    if let Some(v) = runner.pending_selection_view() {
        assert!(
            v.is_optional,
            "the relocate is optional (canNoSelect: true)"
        );
        runner
            .execute_action(v.selecting_player, PASS)
            .expect("decline");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        ba_before,
        "declined ⇒ DNA Charge stays on the field"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declined ⇒ no draw");
    assert_eq!(runner.memory(), mem_before, "declined ⇒ no memory gain");
}

#[test]
fn st24_15_somp_no_ds_tamer_no_relocate() {
    let (mut runner, _option) = setup_field_option(false, true);
    let mem_before = runner.memory();
    let hand_before = runner.hand_size(0);
    let ba_before = runner.battle_area_size(0);

    fire_start_of_main(&mut runner);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no [DATA SQUAD] Tamer ⇒ no relocate prompt"
    );
    assert_eq!(
        runner.battle_area_size(0),
        ba_before,
        "DNA Charge stays on the field"
    );
    assert_eq!(runner.hand_size(0), hand_before, "no draw");
    assert_eq!(runner.memory(), mem_before, "no memory gain");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [Main] play a [DATA SQUAD] cost≤4 card free; persist on field
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn st24_15_main_plays_data_squad_free_and_persists_in_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-15 in embedded pack")
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"))
        .hand(0, &[CARD_ID, "DSD"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let _mem_before = runner.memory();
    use_fixture_option(&mut runner, "FILLER");
    let _ = runner.auto_resolve();

    let dsd_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "DSD");
    assert!(dsd_on_field, "the [DATA SQUAD] Digimon is played for free");
    let dna_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(dna_on_field, "DNA Charge persists in the battle area");
}

#[test]
fn st24_15_paid_main_is_ordinary_field_option_and_survives_next_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("embedded ST24-15")
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"))
        .hand(0, &[CARD_ID, "DSD"])
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        .memory(10)
        .start();
    runner.set_first_player(0);
    use_fixture_option(&mut runner, "FILLER");
    runner.auto_resolve().expect("legal Main selections");
    assert_eq!(
        runner.memory(),
        7,
        "ordinary field placement still pays printed use cost 3"
    );
    let assert_ordinary = |r: &DebugRunner| {
        let option = r.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&r.game.card_data) == CARD_ID)
            .expect("persistent printed field Option must survive");
        assert!(matches!(
            option.option_state,
            digimon_engine::permanent::OptionState::OrdinaryFieldOption
        ));
    };
    assert_ordinary(&runner);
    let mut clone = runner.game.clone();
    for _ in 0..3 {
        runner.end_turn();
        runner.auto_resolve().expect("turn selections");
        assert_ordinary(&runner);
        clone.end_turn();
        assert!(clone.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&clone.card_data) == CARD_ID));
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] activate the [Main] effects (incl. "place this card")
// ════════════════════════════════════════════════════════════════════════════

/// [Security] "Activate this card's [Main] effects" runs the whole [Main],
/// including "Then, place this card in the battle area": after the security
/// check DNA Charge sits in its owner's battle area as a delay Option, not in
/// the trash. Sister of `st23_15_security_places_self_in_battle_area`.
#[test]
fn st24_15_security_places_self_in_battle_area_rose_rave_audit() {
    // Rose Rave audit 2026-11-11: DCGO AddActivateMainOptionSecurityEffect runs the [Main] whole (PlaceDelayOptionCards).
    use digimon_engine::permanent::OptionState;
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(5);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-15 YAML loads")
        .add_card(attacker)
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0, "DNA Charge was checked");
    let placed = runner.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("DNA Charge is placed in its owner's battle area");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { owner: 1, .. }),
        "placed as a delay Option; got {:?}",
        placed.option_state
    );
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "DNA Charge must not be trashed after its [Security] effect"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — Play-pick legality ([Main] and [Security] hand + trash union)
// DCGO ST24_15.cs:47-55 `CanSelectPlayCondition`: `(IsDigimon || IsTamer) &&
// HasPlayCost && EqualsTraits("DATA SQUAD") && GetCostItself <= 4 &&
// CanPlayAsNewPermanent(cardSource, payCost: false, activateClass)`;
// `CanPlayAsNewPermanent` (GameContextDeterminarion.cs:419-432) needs an empty
// battle-area frame the card can be played into. CR 4.3 7-1-1: only a card
// with a play cost is played (a Digi-Egg has none); CR 4.3 1-3-2: a play that
// can't happen is not carried out, so it is not a legal choice.
// ════════════════════════════════════════════════════════════════════════════

const DS_PICK_PROMPT: &str = "You may play 1 [DATA SQUAD]";

/// Non-PASS candidates of the pending [DATA SQUAD] play pick (0 when it is not
/// pending).
fn ds_pick_count(r: &DebugRunner) -> usize {
    match r.pending_selection_view() {
        Some(v) if v.prompt.starts_with(DS_PICK_PROMPT) => {
            v.valid_action_ids.iter().filter(|&&id| id != PASS).count()
        }
        _ => 0,
    }
}

/// Use DNA Charge from hand (p0) with `hand` beside it and `trash` in the
/// trash; `stage` runs before the use. Returns the pick's candidate count.
fn main_pick_count(hand: &[&str], trash: &[&str], stage: impl FnOnce(&mut DebugRunner)) -> usize {
    let mut cards = vec![CARD_ID];
    cards.extend_from_slice(hand);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-15 in embedded pack")
        .add_card(ds_digimon("DSD"))
        .add_card(ds_tamer("DST"))
        .add_card(card_data_from_compiled("ST24-01"))
        .add_card(filler("FILLER"))
        .hand(0, &cards)
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    runner.set_first_player(0);
    for t in trash {
        runner.inject_trash(0, t);
    }
    stage(&mut runner);
    use_fixture_option(&mut runner, "FILLER");
    ds_pick_count(&runner)
}

fn lock(r: &mut DebugRunner, p: u8, m: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    r.game
        .modifiers
        .add_player_modifier(p, PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, 1));
}

/// Regression guard: a hand Digimon, a hand Tamer and a trash Digimon are all
/// offered.
#[test]
fn st24_15_main_pick_offers_hand_and_trash_candidates() {
    assert_eq!(main_pick_count(&["DSD", "DST"], &["DSD"], |_| {}), 3);
}

/// Regression guard: Koromon (ST24-01), a [DATA SQUAD] Digi-Egg with no play
/// cost, is never offered (CR 4.3 7-1-1).
#[test]
fn st24_15_main_pick_excludes_data_squad_digi_egg() {
    assert_eq!(main_pick_count(&["DSD", "ST24-01"], &["ST24-01"], |_| {}), 1);
}

/// "You can't play Digimon by effects": only the Tamer is offered.
#[test]
fn st24_15_main_pick_excludes_digimon_under_effect_play_lock() {
    use digimon_engine::enums::ModifierType;
    let n = main_pick_count(&["DSD", "DST"], &["DSD"], |r| {
        lock(r, 0, ModifierType::CannotPlayDigimonByEffect)
    });
    assert_eq!(n, 1, "only the [DATA SQUAD] Tamer can be played");
}

/// "Can't play cards from the trash": only the hand card is offered.
#[test]
fn st24_15_main_pick_excludes_trash_card_under_trash_play_lock() {
    use digimon_engine::enums::ModifierType;
    let n = main_pick_count(&["DSD"], &["DSD"], |r| lock(r, 0, ModifierType::CannotPlayFromTrash));
    assert_eq!(n, 1, "the trash Digimon can't be played");
}

/// A full battle area: nothing can be played, so nothing is offered.
#[test]
fn st24_15_main_pick_empty_when_battle_area_full() {
    let n = main_pick_count(&["DSD", "DST"], &["DSD"], |r| {
        r.place_on_field(0, "FILLER", Some(0));
        r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
    });
    assert_eq!(n, 0);
}

/// Security DNA Charge (p1 owns it) with a [DATA SQUAD] Digimon in p1's hand;
/// returns p1's pick candidate count after p0's attack checks it.
fn security_pick_count(stage: impl FnOnce(&mut DebugRunner)) -> usize {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.colors = vec![CardColor::Red];
    attacker.level = Some(5);
    attacker.dp = Some(9000);
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST24-15 YAML loads")
        .add_card(attacker)
        .add_card(ds_digimon("DSD"))
        .add_card(filler("FILLER"))
        .hand(1, &["DSD"])
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.set_first_player(0);
    stage(&mut runner);
    let atk = runner.place_on_field(0, "ATTACKER", Some(0));
    let _ = runner.attack_player(atk, 1, false);
    ds_pick_count(&runner)
}

/// Regression guard: the [Security] [Main] offers p1's [DATA SQUAD] Digimon.
#[test]
fn st24_15_security_pick_offers_owner_hand_digimon() {
    assert_eq!(security_pick_count(|_| {}), 1);
}

/// The [Security] copy filters the same way: under "can't play Digimon by
/// effects" p1's Digimon is not offered.
#[test]
fn st24_15_security_pick_empty_under_effect_play_lock() {
    use digimon_engine::enums::ModifierType;
    assert_eq!(security_pick_count(|r| lock(r, 1, ModifierType::CannotPlayDigimonByEffect)), 0);
}
