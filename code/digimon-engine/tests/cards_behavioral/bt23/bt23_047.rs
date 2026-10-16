//! BT23-047 Examon — Digimon, Lv.7, Green/Red/Blue, DP 15000, Cost 15.
//!
//! # Card text (official observation 19bd6db6... and inspected card image)
//!
//! <Piercing> <Security A. +1> <Partition (green Lv.5 & blue Lv.5)>
//! [On Play] [When Digivolving] Suspend 5 of your opponent's Digimon or
//! Tamers. None of your opponent's Digimon can unsuspend in their next
//! unsuspend phase. Then, this Digimon may attack.
//! [Your Turn] [Once Per Turn] When your opponent's security stack is removed
//! from, trash 1 of their Option cards in the battle area. Then, delete 1 of
//! their suspended Digimon or Tamers.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Green/BT23_047.cs
//!
//! # Patterns this test covers
//! - Multi-suspend (select_count_capped_multi, clamp_to_available)
//! - Player-scoped phase-only unsuspend lock, including later entrants
//! - may_attack_now ("this Digimon may attack", PASS exposed)
//! - Opponent-security-removed observer → opponent Option trash + suspended delete
//! - Kept: Piercing / Security A. +1 / Partition

use digimon_dsl::compiled::{
    CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::trigger_context::EventCause;

const CARD_ID: &str = "BT23-047";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 4;
    card
}

fn opp_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn opp_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 2;
    card
}

fn examon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt23/BT23-047.yaml"))
        .expect("BT23-047 must load from production YAML")
}

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

/// Fire `OnOpponentSecurityRemoved`: the opponent (player 1) had their security
/// removed; player 0 (Examon's controller) is the observer.
fn fire_opp_security_removed(runner: &mut DebugRunner) {
    let removed = runner.game.players[1]
        .security
        .first()
        .map(|c| c.handle())
        .unwrap_or(digimon_engine::card_source::CardHandle(0));
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 1,
            observer_player: 0,
            source_player: 0,
            card: removed,
            cause: EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions (kept keywords + new clauses) ──────────

#[test]
fn bt23_047_keeps_partition_green_lv5_and_blue_lv5() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let partition = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            scope: CompiledScope::FaceUp,
            sources,
            ..
        }) => Some(sources),
        _ => None,
    });
    let sources = partition.expect("BT23-047 Partition clause");
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Green)));
    assert!(sources
        .iter()
        .any(|p| p.level_eq == Some(5) && p.color_is == Some(CompiledColor::Blue)));
}

#[test]
fn bt23_047_keeps_piercing_and_security_attack() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Piercing"
        )),
        "Piercing kept"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "SecurityAttackPlus"
        )),
        "Security A. +1 kept"
    );
}

#[test]
fn bt23_047_has_on_play_when_digivolving_suspend_clause() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("On Play / When Digivolving clause");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectCountCappedMulti { .. })),
        "suspend clause must use a count-capped multi-select (suspend up to 5)"
    );
}

#[test]
fn bt23_047_has_your_turn_opt_security_removed_clause() {
    let runner = examon_runner().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let clause = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnOpponentSecurityRemoved clause");
    assert!(clause.once_per_turn, "the [Your Turn] clause is OPT");
}

// ─── Section 2: [On Play] suspend 5 + unsuspend lock + may attack ────────────

/// Suspend 5 opponent Digimon/Tamers; with 3 present, all 3 are suspended
/// (clamp_to_available), and the opponent gets a phase-only Digimon lock.
#[test]
fn bt23_047_on_play_suspends_opponents_and_locks_digimon_unsuspend() {
    let mut runner = examon_runner()
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 6000))
        .add_card(opp_tamer("OPP-T"))
        .memory(20)
        .start();
    let examon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let opp_t = runner.place_on_field(1, "OPP-T", Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, examon);

    // Multi-suspend selection over opponent Digimon + Tamers.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "suspend selection must install"
    );
    runner
        .execute_action(0, encode_attack(0, opp_a.index as u16))
        .expect("suspend A");
    runner
        .execute_action(0, encode_attack(0, opp_b.index as u16))
        .expect("suspend B");
    runner
        .execute_action(0, encode_attack(0, opp_t.index as u16))
        .expect("suspend Tamer");

    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_b.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_t.index as usize].is_suspended);

    // The opponent gets the phase-only player lock, without effect-unsuspend prohibition.
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotUnsuspendInUnsuspendPhase),
        "opponent must get the player-scoped phase-only lock"
    );
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotUnsuspendInUnsuspendPhase),
        "opponent must get the player-scoped phase-only lock"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_t, ModifierType::CannotUnsuspend),
        "the unsuspend lock targets Digimon only, not Tamers"
    );

    // "Then, this Digimon may attack." — an optional attack prompt for Examon.
    let view = runner
        .pending_selection_view()
        .expect("may-attack prompt must install");
    assert!(
        view.is_optional,
        "the 'may attack' prompt must be optional (PASS exposed)"
    );
}

/// With no opponent Digimon/Tamers, the suspend no-ops but the may-attack
/// prompt still surfaces (Examon may attack regardless).
#[test]
fn bt23_047_on_play_may_attack_even_with_no_suspend_targets() {
    let mut runner = examon_runner().memory(20).start();
    let examon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.turn_count = 1;

    fire_on_play(&mut runner, examon);

    let view = runner
        .pending_selection_view()
        .expect("may-attack prompt must install even with no suspend targets");
    assert!(view.is_optional, "may-attack is optional");
}

// ─── Section 3: [Your Turn][OPT] opponent-security-removed tail ──────────────

/// On the controller's turn, when the opponent's security is removed: trash 1
/// opponent Option, then delete 1 suspended opponent Digimon/Tamer.
#[test]
fn bt23_047_security_removed_trashes_option_then_deletes_suspended() {
    let mut runner = examon_runner()
        .add_card(opp_option("OPP-OPT"))
        .add_card(opp_digimon("OPP-SUSP", 3000))
        .security(1, &["OPP-SUSP", "OPP-SUSP"])
        .memory(20)
        .start();
    let _examon = runner.place_on_field(0, CARD_ID, Some(0));
    let opt = runner.place_on_field(1, "OPP-OPT", Some(0));
    let susp = runner.place_on_field(1, "OPP-SUSP", Some(0));
    runner.game.players[1].battle_area[susp.index as usize].is_suspended = true;
    runner.game.turn_count = 1; // player 0's turn

    let opp_field_before = runner.game.players[1].battle_area.len();

    fire_opp_security_removed(&mut runner);

    // First: trash the opponent's Option (mandatory pick when one exists).
    let view = runner
        .pending_selection_view()
        .expect("opponent Option trash prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(view.selecting_player, encode_attack(0, opt.index as u16))
        .expect("trash opponent Option");
    let _ = runner.auto_resolve();

    // After resolution, the Option is gone and the suspended Digimon was deleted.
    assert!(
        runner.game.players[1].battle_area.len() < opp_field_before,
        "the opponent's Option and suspended Digimon must both be removed"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-OPT"),
        "the opponent's Option card must be trashed"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-SUSP"),
        "the suspended opponent Digimon must be deleted"
    );
}

/// Negative: the [Your Turn] clause must not fire on the opponent's turn.
#[test]
fn bt23_047_security_removed_clause_does_not_fire_on_opponents_turn() {
    let mut runner = examon_runner()
        .add_card(opp_option("OPP-OPT"))
        .security(1, &["OPP-OPT"])
        .deck(0, &["OPP-OPT"; 10])
        .deck(1, &["OPP-OPT"; 10])
        .memory(20)
        .start();
    let _examon = runner.place_on_field(0, CARD_ID, Some(0));
    let _opt = runner.place_on_field(1, "OPP-OPT", Some(0));
    runner.game.turn_count = 1;
    runner.end_turn(); // -> opponent's (player 1's) turn
    assert!(!runner.game_over());
    assert_eq!(runner.turn_player(), 1);

    let field_before = runner.game.players[1].battle_area.len();
    fire_opp_security_removed(&mut runner);

    // On the opponent's turn the [Your Turn] clause is inert — no Option trash.
    assert!(
        runner.pending_selection().is_none(),
        "the [Your Turn] clause must not install a selection on the opponent's turn"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        field_before,
        "no opponent permanent should be removed on the opponent's turn"
    );
}

// Official BT23-047 observation 19bd6db6..., corroborated by the card image.
fn phase_runner() -> DebugRunner {
    let mut token = opp_digimon("TOKEN", 1000);
    token.card_kind = CardKind::Token;
    let mut builder = examon_runner()
        .add_card(token)
        .add_card(opp_option("OPTION"));
    for (id, color, level) in [
        ("G6", digimon_engine::enums::CardColor::Green, 6),
        ("B6", digimon_engine::enums::CardColor::Blue, 6),
        ("G5", digimon_engine::enums::CardColor::Green, 5),
        ("B5", digimon_engine::enums::CardColor::Blue, 5),
    ] {
        let mut c = opp_digimon(id, 6000);
        c.colors = vec![color];
        c.level = Some(level);
        builder = builder.add_card(c);
    }
    let mut r = builder
        .add_card(opp_digimon("FILL", 1000))
        .add_card(opp_tamer("TAMER"))
        .deck(0, &["FILL"; 15])
        .deck(1, &["FILL"; 15])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(20)
        .start();
    r.skip_mulligan();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    r.game.turn_count = 2;
    r
}
fn install_lock(r: &mut DebugRunner) {
    let e = r.place_on_field(0, CARD_ID, Some(0));
    fire_on_play(r, e);
    while let Some(v) = r.pending_selection_view() {
        let a = if v.is_optional {
            digimon_engine::action::space::PASS
        } else {
            v.valid_action_ids[0]
        };
        r.execute_action(v.selecting_player, a).unwrap();
    }
}
#[test]
fn bt23_047_phase_lock_allows_effect_unsuspend() {
    let mut r = phase_runner();
    let d = r.place_on_field(1, "FILL", Some(0));
    install_lock(&mut r);
    assert!(r.game.players[1].battle_area[d.index as usize].is_suspended);
    r.game.unsuspend_with_cause(d, true);
    assert!(
        !r.game.players[1].battle_area[d.index as usize].is_suspended,
        "printed lock must permit effect unsuspension"
    );
}
#[test]
fn bt23_047_phase_lock_covers_later_digimon_but_not_tamers() {
    let mut r = phase_runner();
    install_lock(&mut r);
    let d = r.place_on_field(1, "FILL", Some(0));
    let t = r.place_on_field(1, "TAMER", Some(0));
    r.game.players[1].battle_area[d.index as usize].is_suspended = true;
    r.game.players[1].battle_area[t.index as usize].is_suspended = true;
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(
        r.game.players[1].battle_area[d.index as usize].is_suspended,
        "none of opponent Digimon includes ones entering after resolution"
    );
    assert!(!r.game.players[1].battle_area[t.index as usize].is_suspended);
}
#[test]
fn bt23_047_printed_native_dna_route_exists() {
    let r = phase_runner();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert!(
        c.alt_paths
            .iter()
            .any(|p| p.kind == digimon_dsl::compiled::CompiledAltPathKind::DnaDigivolve),
        "printed green Lv6 plus blue Lv6 cost zero route"
    );
}

#[test]
fn bt23_047_suspend_five_mandatory_distinct_excludes_options() {
    let mut r = phase_runner();
    let e = r.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..6 {
        r.place_on_field(1, "FILL", Some(0));
    }
    r.place_on_field(1, "OPTION", Some(0));
    r.place_on_field(0, "FILL", Some(0));
    fire_on_play(&mut r, e);
    for n in 0..5 {
        let v = r.pending_selection_view().unwrap();
        assert!(!v.is_optional);
        assert_eq!(v.valid_action_ids.len(), 6 - n);
        assert!(!v
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS));
        r.execute_action(0, v.valid_action_ids[0]).unwrap();
    }
    assert_eq!(
        r.game.players[1]
            .battle_area
            .iter()
            .filter(|p| p.is_suspended)
            .count(),
        5
    );
    assert!(r.pending_selection_view().unwrap().is_optional);
}
#[test]
fn bt23_047_security_tail_without_option_is_mandatory_and_opt() {
    let mut r = phase_runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_on_field(1, "FILL", Some(0));
    let b = r.place_on_field(1, "TAMER", Some(0));
    r.game.players[1].battle_area[a.index as usize].is_suspended = true;
    r.game.players[1].battle_area[b.index as usize].is_suspended = true;
    fire_opp_security_removed(&mut r);
    let v = r
        .pending_selection_view()
        .expect("delete still offered without Option");
    assert!(!v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 2);
    r.execute_action(0, encode_attack(0, b.index as u16))
        .unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    fire_opp_security_removed(&mut r);
    assert!(r.pending_selection_view().is_none(), "OPT already consumed");
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn bt23_047_phase_lock_survives_source_leaving_and_covers_tokens() {
    let mut r = phase_runner();
    install_lock(&mut r);
    r.game.return_to_hand_from_effect(
        digimon_engine::PermanentHandle {
            player: 0,
            index: 0,
        },
        0,
    );
    assert!(r.game.players[0].battle_area.is_empty());
    let d = r.place_on_field(1, "TOKEN", Some(0));
    r.game.players[1].battle_area[d.index as usize].is_suspended = true;
    r.game = r.game.clone();
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(r.game.players[1].battle_area[d.index as usize].is_suspended);
    r.end_turn();
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(
        !r.game.players[1].battle_area[d.index as usize].is_suspended,
        "only the next opponent unsuspend phase is prevented"
    );
}
#[test]
fn bt23_047_opponent_turn_invocation_waits_for_next_opponent_phase() {
    let mut r = phase_runner();
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    r.game.memory = 3;
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    install_lock(&mut r);
    r.game = r.game.clone();
    let d = r.place_on_field(1, "FILL", Some(0));
    r.game.players[1].battle_area[d.index as usize].is_suspended = true;
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(r.game.players[1].battle_area[d.index as usize].is_suspended);
    r.game.unsuspend_with_cause(d, true);
    assert!(!r.game.players[1].battle_area[d.index as usize].is_suspended);
    r.end_turn();
    assert_eq!(r.turn_player(), 0);
    assert!(
        !r.game
            .modifiers
            .player_has(1, ModifierType::CannotUnsuspendInUnsuspendPhase),
        "phase lock expires at the end of the next opponent turn"
    );
    r.game.players[1].battle_area[d.index as usize].is_suspended = true;
    r.end_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(
        !r.game.players[1].battle_area[d.index as usize].is_suspended,
        "subsequent opponent phase unsuspends normally"
    );
}

#[test]
fn bt23_047_dna_exposes_both_material_choices_and_pays_zero() {
    use digimon_engine::action::{build_action_mask, space::DNA_DIGIVOLVE_START};
    let mut r = phase_runner();
    r.add_to_hand(0, CARD_ID);
    r.place_on_field(0, "G6", Some(0));
    r.place_on_field(0, "B6", Some(0));
    assert!(build_action_mask(&r.game, 0)[DNA_DIGIVOLVE_START as usize] > 0.0);
    let before = r.memory();
    r.game.decode_action(DNA_DIGIVOLVE_START, 0);
    for _ in 0..2 {
        let v = r.pending_selection_view().expect("DNA material choice");
        let action = v.valid_action_ids[0];
        assert!(build_action_mask(&r.game, 0)[action as usize] > 0.0);
        r.execute_action(0, action).unwrap();
    }
    assert_eq!(r.memory(), before);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID
    );
}

#[test]
fn bt23_047_partition_prompts_and_plays_exact_colored_level_five_sources() {
    let mut r = phase_runner();
    let host = r.place_stack(0, &["G5", "B5", CARD_ID]);
    r.game.delete_permanent_with_cause(
        host,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let view = r
        .pending_selection_view()
        .expect("Partition must expose optional choice");
    assert!(view.is_optional);
    r.accept_optional_trigger().unwrap();
    while let Some(v) = r.pending_selection_view() {
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != digimon_engine::action::space::PASS)
            .unwrap();
        r.execute_action(v.selecting_player, action).unwrap();
    }
    let ids: Vec<_> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data))
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"G5") && ids.contains(&"B5"));
}

#[test]
fn bt23_047_q5315_security_placed_option_is_available_to_pending_observer() {
    let mut r = examon_runner()
        .from_dsl_yaml(include_str!("../../../cards/p/P-035.yaml"))
        .unwrap()
        .add_card(opp_digimon("FILL", 1000))
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .security(1, &["FILL", "FILL", "P-035"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let e = r.place_on_field(0, CARD_ID, Some(0));
    r.game.turn_count = 2;
    r.attack_player(e, 1, false);
    let v = r
        .pending_selection_view()
        .expect("security effect places Option before observer resolves");
    assert_eq!(v.kind, SelectionKind::OppField);
    assert!(!v.is_optional);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "P-035"
    );
    r.execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    assert!(!r.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "P-035"));
}
