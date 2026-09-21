//! P-212 Asuna Shiroki — Tamer, Purple, Cost 3. Traits: [TS].
//!
//! # Card text (card image / official DB — authoritative)
//!
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [On Play] ＜Draw 1＞ and trash 1 card in your hand. If this effect trashed a
//! card with the [Three Musketeers] or [TS] trait, delete 1 of your
//! opponent's level 3 Digimon.
//!
//! Inherited (Security):
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Purple/P_212.cs
//!   - OnStartMainPhase → `Gain1MemoryTamerOpponentDigimonEffect` (gate:
//!     `card.Owner.Enemy.GetBattleAreaDigimons().Count >= 1`).
//!   - OnEnterFieldAnyone (own on-play), isOptional = false →
//!     `DrawAndDiscardCards(draw 1, trash 1)`; `trashedTargetCard` =
//!     selected card HasThreeMusketeersTraits || HasTSTraits; if so and an
//!     opponent level-3 Digimon exists → mandatory SelectPermanentEffect
//!     (Destroy, maxCount Min(1, count), canNoSelect false).
//!   - SecuritySkill → `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers
//! - B1 start-of-main tamer (conditional memory gain) — positive + negative gate
//! - B3 on-play tamer: draw → mandatory hand trash → result-conditioned delete
//!   (`trashed_hand_card_matching`, the result predicate widened for this card:
//!   G-DSL-EFFECT-TRASHED-HAND-CARD)
//! - Selection: `SelectionKind::Hand` (mandatory) then `SelectionKind::OppField`
//!   filtered to level 3
//! - Tamer [Security] play-self

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "P-212";

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![CardColor::Purple];
    c.level = Some(level);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-212 must load from the embedded DSL pack")
        .add_card(digimon("FILL", 3, &[]))
        .add_card(digimon("PLAIN", 4, &[]))
        .add_card(digimon("TM", 3, &["Three Musketeers"]))
        .add_card(digimon("TS", 4, &["TS"]))
        .add_card(digimon("OPP-L3", 3, &[]))
        .add_card(digimon("OPP-L4", 4, &[]))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn hand_action_for(runner: &DebugRunner, card_id: &str) -> u16 {
    let view = runner.pending_selection_view().expect("hand selection parked");
    assert_eq!(view.kind, SelectionKind::Hand);
    let idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in P0's hand"));
    let id = digimon_engine::action::space::PLAY_HAND_START + idx as u16;
    assert!(
        view.valid_action_ids.contains(&id),
        "hand pick {card_id} (index {idx}) must be a legal action"
    );
    id
}

fn asuna_handle(runner: &DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Asuna on P0's field");
    runner.perm_handle(0, idx)
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn p_212_metadata_and_three_triggered_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 3);

    let som = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("start_of_your_main_phase clause");
    assert!(!som.optional);
    assert_eq!(som.scope, CompiledScope::FaceUp);

    let on_play = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnPlay])
        .expect("on_play clause");
    assert!(!on_play.optional, "[On Play] draw + trash is mandatory (DCGO isOptional = false)");
    assert!(!on_play.once_per_turn);

    let sec = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("on_security clause");
    assert!(!sec.optional);
}

// ─── Section 2: [Start of Your Main Phase] gate ──────────────────────────────

#[test]
fn p_212_start_of_main_gains_memory_when_opponent_has_a_digimon() {
    let mut runner = base().memory(3).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L4", Some(0));
    let asuna = asuna_handle(&runner);
    let before = runner.memory();
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(asuna));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before + 1, "gain 1 memory while the opponent has a Digimon");
}

#[test]
fn p_212_start_of_main_does_nothing_without_opponent_digimon() {
    let mut runner = base().memory(3).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let asuna = asuna_handle(&runner);
    let before = runner.memory();
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(asuna));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), before, "no opponent Digimon → condition gates the gain");
}

// ─── Section 3: [On Play] draw → trash → conditional delete ──────────────────

#[test]
fn p_212_on_play_draws_then_demands_a_mandatory_hand_trash() {
    let mut runner = base().hand(0, &[CARD_ID, "PLAIN"]).memory(8).start();
    let deck_before = runner.deck_size(0);
    let field_index = runner.play(0, 0).expect("Asuna plays from hand");
    if runner.pending_selection().is_none() {
        runner.fire_on_play(0, field_index);
    }

    assert_eq!(runner.deck_size(0), deck_before - 1, "<Draw 1> resolved first");
    let view = runner.pending_selection_view().expect("hand trash prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(!view.is_optional, "'trash 1 card in your hand' is mandatory — no PASS");
    assert_eq!(
        view.valid_action_ids.len(),
        runner.hand_size(0),
        "every hand card is a legal trash pick"
    );
}

#[test]
fn p_212_trashing_a_musketeer_offers_only_opponent_level_3_digimon() {
    let mut runner = base().hand(0, &[CARD_ID, "TM", "PLAIN"]).memory(8).start();
    let opp_l3 = runner.place_on_field(1, "OPP-L3", Some(0));
    let _opp_l4 = runner.place_on_field(1, "OPP-L4", Some(0));
    let field_index = runner.play(0, 0).expect("Asuna plays");
    if runner.pending_selection().is_none() {
        runner.fire_on_play(0, field_index);
    }

    let pick = hand_action_for(&runner, "TM");
    runner.execute_action(0, pick).expect("trash the [Three Musketeers] card");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM"),
        "the chosen card was trashed"
    );

    let view = runner
        .pending_selection_view()
        .expect("trashing a [Three Musketeers] card unlocks the delete");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "the delete is mandatory once unlocked (DCGO canNoSelect false)");
    assert_eq!(view.valid_action_ids.len(), 1, "only the level-3 opponent Digimon is offered");

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("delete the level-3 Digimon");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OPP-L3"),
        "the level-3 opponent Digimon was deleted"
    );
    assert_eq!(runner.battle_area_size(1), 1, "the level-4 Digimon survives");
    let _ = opp_l3;
}

#[test]
fn p_212_trashing_a_ts_card_also_unlocks_the_delete() {
    let mut runner = base().hand(0, &[CARD_ID, "TS"]).memory(8).start();
    runner.place_on_field(1, "OPP-L3", Some(0));
    let field_index = runner.play(0, 0).expect("Asuna plays");
    if runner.pending_selection().is_none() {
        runner.fire_on_play(0, field_index);
    }
    let pick = hand_action_for(&runner, "TS");
    runner.execute_action(0, pick).expect("trash the [TS] card");
    let view = runner.pending_selection_view().expect("delete prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
}

#[test]
fn p_212_trashing_a_plain_card_does_not_delete() {
    let mut runner = base().hand(0, &[CARD_ID, "PLAIN", "TM"]).memory(8).start();
    runner.place_on_field(1, "OPP-L3", Some(0));
    let field_index = runner.play(0, 0).expect("Asuna plays");
    if runner.pending_selection().is_none() {
        runner.fire_on_play(0, field_index);
    }
    let pick = hand_action_for(&runner, "PLAIN");
    runner.execute_action(0, pick).expect("trash the plain card");
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "a non-[TS]/[Three Musketeers] trash unlocks nothing"
    );
    assert_eq!(runner.battle_area_size(1), 1, "the opponent's level-3 Digimon survives");
}

#[test]
fn p_212_musketeer_trash_with_no_level_3_target_deletes_nothing() {
    let mut runner = base().hand(0, &[CARD_ID, "TM"]).memory(8).start();
    runner.place_on_field(1, "OPP-L4", Some(0));
    let field_index = runner.play(0, 0).expect("Asuna plays");
    if runner.pending_selection().is_none() {
        runner.fire_on_play(0, field_index);
    }
    let pick = hand_action_for(&runner, "TM");
    runner.execute_action(0, pick).expect("trash the [Three Musketeers] card");
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none(), "no level-3 target → no prompt");
    assert_eq!(runner.battle_area_size(1), 1, "the level-4 Digimon is untouched");
}

// ─── Section 4: [Security] play without paying the cost ──────────────────────

#[test]
fn p_212_security_plays_itself_free() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut runner = base()
        .add_card(attacker)
        .hand(1, &["PLAIN"])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0);
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "Asuna was played into P1's battle area by her [Security] effect"
    );
    assert_eq!(runner.memory(), memory_before, "played without paying the cost");
}
