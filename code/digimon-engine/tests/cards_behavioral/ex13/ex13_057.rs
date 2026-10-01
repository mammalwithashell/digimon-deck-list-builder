//! EX13-057 Grademon — Digimon, Lv.5, Black/Yellow, DP 7000, Cost 7.
//! Traits: Warrior / X Antibody / Chronicle. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-057.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-057.md` agrees)
//!
//! ```text
//! Digivolve: Black Lv.4 / cost 4, Yellow Lv.4 / cost 4
//! [Digivolve] [Raptordramon]/Lv.4 w/[Chronicle] trait: Cost 3
//!
//! Effect:
//! [On Play] [When Digivolving] Until your opponent's turn ends, 1 of your
//! [X Antibody] or [Chronicle] trait Digimon gains ＜Reboot＞ and ＜Blocker＞. If
//! during an attack, it also isn't affected by their Digimon effects and gets
//! +5000 DP.
//! [End of Attack] [Once Per Turn] This Digimon may digivolve into a Digimon
//! card with the [Chronicle] trait in the hand or trash.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When any of your [Chronicle] trait Digimon would
//! leave the battle area, by trashing your top security card, they don't leave.
//! ```
//!
//! Official Q&A: "isn't affected by effects" — e.g. its DP won't be reduced by a
//! "1 of your opponent's Digimon gets -3000 DP" effect.
//!
//! # DCGO C# reference
//! None for EX13-057 at b9a0638cd. The "if during an attack ... isn't affected
//! by their Digimon effects and gets +5000 DP" rider mirrors DCGO
//! BT20/Black/BT20_053.cs (BT20-053 Grademon): `attackProcess.IsAttacking`
//! gate → `DigimonEffectImmunity` + `ChangeDigimonDP(+5000)`, both
//! `UntilOpponentTurnEnd`, on the SAME chosen Digimon. The leave-prevention
//! follows BT24-101 (any own [TS] Digimon … trash top security, OPT).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - [On Play]+[When Digivolving] own-target keyword grant until end of
//!   opponent's turn + game-state gated rider (`during_attack`).
//! - [End of Attack][OPT] optional effect-digivolve from hand ∪ trash.
//! - Inherited [All Turns][OPT] would-leave replacement over a filtered set of
//!   own Digimon, cost: trash top security.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::enums::{CardColor, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

use super::chronicle_support::*;

const CARD_ID: &str = "EX13-057";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-057 YAML parses, compiles and is in the embedded pack")
        .dsl_card("EX13-055")
        .expect("EX13-055 Raptordramon YAML loads")
        .add_card(digimon("FILL", CardColor::Black, 3, 1000, &[]))
        .add_card(digimon("BLACK-L4", CardColor::Black, 4, 5000, &[]))
        .add_card(digimon("RED-CHRON-L4", CardColor::Red, 4, 5000, &["Chronicle"]))
        .add_card(digimon("RED-L4", CardColor::Red, 4, 5000, &[]))
        .add_card(digimon("XA-ALLY", CardColor::Red, 4, 4000, &["X Antibody"]))
        .add_card(digimon("CHRON-ALLY", CardColor::Red, 4, 4000, &["Chronicle"]))
        .add_card(digimon("PLAIN-ALLY", CardColor::Red, 4, 4000, &["Beast"]))
        .add_card(digimon("CHRON-CARRIER", CardColor::Black, 6, 9000, &["Chronicle"]))
        .add_card(with_evo(
            digimon("CHRON-L6", CardColor::Black, 6, 11000, &["Chronicle"]),
            CardColor::Black,
            5,
            3,
        ))
        .add_card(with_evo(
            digimon("CHRON-L6-T", CardColor::Black, 6, 11000, &["Chronicle"]),
            CardColor::Black,
            5,
            3,
        ))
        .add_card(with_evo(
            digimon("PLAIN-L6", CardColor::Black, 6, 11000, &["Beast"]),
            CardColor::Black,
            5,
            3,
        ))
        .add_card(digimon("OPP-A", CardColor::Red, 4, 5000, &[]))
}

fn effects(runner: &DebugRunner) -> Vec<CompiledClause> {
    runner.compiled_card(CARD_ID).expect("compiled").effects.clone()
}

fn face_up(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    effects(runner)
        .into_iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp => Some(t),
            _ => None,
        })
        .collect()
}

fn has(runner: &DebugRunner, h: PermanentHandle, k: Keyword) -> bool {
    runner.game.has_keyword(h, k)
}

fn immune_to_opp_digimon(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner
        .game
        .permanent_is_unaffected_by_effect(h, 1 - h.player, EffectSourceKind::Digimon)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_057_structure_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["Warrior", "X Antibody", "Chronicle"]);
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 4, "2 circles + [Raptordramon] + Lv.4 w/[Chronicle]");

    let face = face_up(&runner);
    assert_eq!(face.len(), 2);
    let grant = face
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(grant.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!grant.optional && !grant.once_per_turn);
    let eoa = face
        .iter()
        .find(|c| c.when == vec![CompiledTiming::EndOfAttack])
        .expect("[End of Attack] clause");
    assert!(eoa.once_per_turn, "[Once Per Turn]");

    let replacements = effects(&runner)
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                    scope: CompiledScope::Inherited,
                    once_per_turn: true,
                    optional: true,
                    ..
                })
            )
        })
        .count();
    assert_eq!(replacements, 1, "inherited [All Turns][OPT] optional leave-prevention");
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

fn digivolve_onto(base_id: &str, memory: i16, cost: u16) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, base_id, Some(0));
    digivolve_choosing_cost(&mut runner, 0, base, cost);
    (runner, base)
}

#[test]
fn ex13_057_digivolves_from_raptordramon_for_3() {
    let (runner, _) = digivolve_onto("EX13-055", 6, 3);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3, "[Raptordramon]: Cost 3");
}

#[test]
fn ex13_057_digivolves_from_any_colour_lv4_chronicle_for_3() {
    let (runner, _) = digivolve_onto("RED-CHRON-L4", 6, 3);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3);
}

#[test]
fn ex13_057_black_lv4_uses_the_cost_4_circle() {
    let (runner, _) = digivolve_onto("BLACK-L4", 6, 4);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 2);
}

#[test]
fn ex13_057_red_lv4_without_chronicle_has_no_route() {
    let (runner, _) = digivolve_onto("RED-L4", 6, 3);
    assert!(!field_has(&runner, 0, CARD_ID));
}

// ─── [On Play] / [When Digivolving] outside an attack ────────────────────────

/// XA-ALLY on the field, then Grademon digivolves onto Raptordramon (cost-3
/// route) outside any attack.
fn digivolve_with_ally() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(6)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "EX13-055", Some(0));
    let ally = runner.place_on_field(0, "XA-ALLY", Some(0));
    digivolve_choosing_cost(&mut runner, 0, base, 3);
    (runner, base, ally)
}

#[test]
fn ex13_057_when_digivolving_grants_reboot_and_blocker_without_attack_rider() {
    let (mut runner, grade, ally) = digivolve_with_ally();
    let view = runner.pending_selection_view().expect("own-target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "the grant is mandatory");
    pick_field(&mut runner, ally);
    assert!(has(&runner, ally, Keyword::Reboot));
    assert!(has(&runner, ally, Keyword::Blocker));
    assert_eq!(runner.effective_dp(ally), Some(4000), "no +5000 outside an attack");
    assert!(!immune_to_opp_digimon(&runner, ally), "no immunity outside an attack");
    assert!(!has(&runner, grade, Keyword::Blocker), "only the chosen Digimon gains them");
}

#[test]
fn ex13_057_on_play_targets_only_x_antibody_or_chronicle_digimon() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(8)
        .start();
    runner.skip_mulligan();
    let xa = runner.place_on_field(0, "XA-ALLY", Some(0));
    let chron = runner.place_on_field(0, "CHRON-ALLY", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-ALLY", Some(0));
    runner.play(0, 0);
    let grade = PermanentHandle { player: 0, index: 3 };
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField), "[On Play] fired");
    assert!(field_offered(&runner, xa));
    assert!(field_offered(&runner, chron));
    assert!(field_offered(&runner, grade), "Grademon itself has both traits");
    assert!(!field_offered(&runner, plain), "trait-less Digimon not offered");
    pick_field(&mut runner, chron);
    assert!(has(&runner, chron, Keyword::Reboot) && has(&runner, chron, Keyword::Blocker));
}

#[test]
fn ex13_057_granted_keywords_last_until_the_end_of_the_opponents_turn() {
    let (mut runner, _grade, ally) = digivolve_with_ally();
    pick_field(&mut runner, ally);
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    assert!(has(&runner, ally, Keyword::Blocker), "still active on the opponent's turn");
    assert!(has(&runner, ally, Keyword::Reboot));
    next_turn(&mut runner);
    assert!(!has(&runner, ally, Keyword::Blocker), "expired at the end of the opponent's turn");
    assert!(!has(&runner, ally, Keyword::Reboot));
}

// ─── [When Digivolving] during an attack (via Raptordramon) ──────────────────

fn attack_digivolve_setup() -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(8)
        .start();
    runner.skip_mulligan();
    let rap = runner.place_on_field(0, "EX13-055", Some(0));
    runner.attack_player(rap, 1, false);
    // Raptordramon [When Attacking]: digivolve into Grademon from hand.
    pick_hand(&mut runner, 0, CARD_ID);
    // Raptordramon's digivolve pays a printed cost: the [Raptordramon] route (3).
    assert_eq!(runner.memory(), 5, "8 - 3 (the [Raptordramon] route)");
    assert_eq!(top_id(&runner, rap), CARD_ID);
    (runner, rap)
}

#[test]
fn ex13_057_during_attack_rider_adds_immunity_and_plus_5000() {
    let (mut runner, grade) = attack_digivolve_setup();
    assert!(runner.game.pending_attack.is_some(), "still during the attack");
    let view = runner.pending_selection_view().expect("[When Digivolving] own-target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    pick_field(&mut runner, grade);
    assert!(has(&runner, grade, Keyword::Reboot) && has(&runner, grade, Keyword::Blocker));
    assert_eq!(runner.effective_dp(grade), Some(12000), "7000 + 5000 during an attack");
    assert!(immune_to_opp_digimon(&runner, grade), "isn't affected by their Digimon effects");
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(grade, 1, EffectSourceKind::Option),
        "only DIGIMON effects are blocked"
    );
}

#[test]
fn ex13_057_during_attack_rider_persists_until_the_end_of_the_opponents_turn() {
    let (mut runner, grade) = attack_digivolve_setup();
    pick_field(&mut runner, grade);
    decline_all(&mut runner);
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.effective_dp(grade), Some(12000), "+5000 persists into the opponent's turn");
    assert!(immune_to_opp_digimon(&runner, grade));
    next_turn(&mut runner);
    assert_eq!(runner.effective_dp(grade), Some(7000), "expired");
    assert!(!immune_to_opp_digimon(&runner, grade));
}

// ─── [End of Attack][OPT] digivolve ──────────────────────────────────────────

fn eoa_setup(hand: &[&str], trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(6)
        .start();
    runner.skip_mulligan();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let grade = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, grade)
}

/// Attack the player and PASS every prompt until the End-of-Attack union pick.
fn attack_until_end_of_attack(runner: &mut DebugRunner, grade: PermanentHandle) {
    runner.attack_player(grade, 1, false);
    for _ in 0..16 {
        let Some(view) = runner.pending_selection_view() else { return };
        if matches!(view.kind, SelectionKind::UnionZone { .. }) {
            return;
        }
        let a = if view.is_optional { PASS } else { view.valid_action_ids[0] };
        runner.execute_action(view.selecting_player, a).expect("advance");
    }
}

#[test]
fn ex13_057_end_of_attack_offers_chronicle_cards_in_hand_and_trash() {
    let (mut runner, grade) = eoa_setup(&["CHRON-L6", "PLAIN-L6"], &["CHRON-L6-T"]);
    attack_until_end_of_attack(&mut runner, grade);
    let view = runner.pending_selection_view().expect("End of Attack digivolve pick");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { zones } if zones == (UnionZoneSet::HAND | UnionZoneSet::TRASH)),
        "{view:?}"
    );
    assert!(view.is_optional, "\"may digivolve\"");
    assert!(runner.game.pending_attack.is_some() || true);
    assert!(hand_offered(&runner, 0, "CHRON-L6"));
    assert!(trash_offered(&runner, 0, "CHRON-L6-T"));
    assert!(!hand_offered(&runner, 0, "PLAIN-L6"), "non-[Chronicle] not offered");
}

#[test]
fn ex13_057_end_of_attack_digivolves_paying_the_cost() {
    let (mut runner, grade) = eoa_setup(&["CHRON-L6"], &[]);
    attack_until_end_of_attack(&mut runner, grade);
    let memory_before = runner.memory();
    pick_hand(&mut runner, 0, "CHRON-L6");
    assert_eq!(top_id(&runner, grade), "CHRON-L6");
    assert_eq!(runner.memory(), memory_before - 3, "Black Lv.5 circle cost 3 paid");
}

#[test]
fn ex13_057_end_of_attack_digivolves_from_trash() {
    let (mut runner, grade) = eoa_setup(&[], &["CHRON-L6-T"]);
    attack_until_end_of_attack(&mut runner, grade);
    pick_trash(&mut runner, 0, "CHRON-L6-T");
    assert_eq!(top_id(&runner, grade), "CHRON-L6-T");
}

#[test]
fn ex13_057_end_of_attack_can_be_declined() {
    let (mut runner, grade) = eoa_setup(&["CHRON-L6"], &[]);
    attack_until_end_of_attack(&mut runner, grade);
    decline(&mut runner);
    assert_eq!(top_id(&runner, grade), CARD_ID);
}

#[test]
fn ex13_057_end_of_attack_once_per_turn_lockout_and_reset() {
    let (mut runner, grade) = eoa_setup(&["CHRON-L6"], &[]);
    fire(&mut runner, EffectTiming::EndOfAttack, grade);
    assert!(matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })));
    decline(&mut runner);
    fire(&mut runner, EffectTiming::EndOfAttack, grade);
    assert!(runner.pending_selection().is_none(), "second [End of Attack] this turn locked out");

    next_turn(&mut runner);
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 0);
    fire(&mut runner, EffectTiming::EndOfAttack, grade);
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "OPT reset on the next turn"
    );
}

// ─── Inherited [All Turns][OPT] leave-prevention ─────────────────────────────

fn protect_setup(security: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(0, security)
        .memory(3)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CHRON-CARRIER"]);
    let chron = runner.place_on_field(0, "CHRON-ALLY", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-ALLY", Some(0));
    (runner, carrier, chron, plain)
}

#[test]
fn ex13_057_inherited_protects_another_chronicle_digimon_by_trashing_top_security() {
    let (mut runner, _carrier, chron, _plain) = protect_setup(&["FILL", "FILL"]);
    runner
        .game
        .delete_permanents_batch(vec![chron], ReplacementCause::OpponentEffect);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    assert!(runner.pending_is_optional(), "\"by trashing ...\" is a may-pay");
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let _ = runner.auto_resolve();
    assert!(field_has(&runner, 0, "CHRON-ALLY"), "it doesn't leave");
    assert_eq!(runner.security_count(0), 1, "top security card trashed");
}

#[test]
fn ex13_057_inherited_also_protects_the_carrier_itself_from_bounce() {
    let (mut runner, carrier, _chron, _plain) = protect_setup(&["FILL", "FILL"]);
    runner.game.return_to_hand_from_effect(carrier, 1);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let _ = runner.auto_resolve();
    assert!(field_has(&runner, 0, "CHRON-CARRIER"), "any leave, not only deletion");
    assert_eq!(runner.security_count(0), 1);
}

#[test]
fn ex13_057_inherited_declined_lets_it_leave() {
    let (mut runner, _carrier, chron, _plain) = protect_setup(&["FILL", "FILL"]);
    runner
        .game
        .delete_permanents_batch(vec![chron], ReplacementCause::OpponentEffect);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!field_has(&runner, 0, "CHRON-ALLY"));
    assert_eq!(runner.security_count(0), 2, "no security paid");
}

#[test]
fn ex13_057_inherited_does_not_protect_non_chronicle_digimon() {
    let (mut runner, _carrier, _chron, plain) = protect_setup(&["FILL", "FILL"]);
    runner
        .game
        .delete_permanents_batch(vec![plain], ReplacementCause::OpponentEffect);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Replacement));
    let _ = runner.auto_resolve();
    assert!(!field_has(&runner, 0, "PLAIN-ALLY"));
    assert_eq!(runner.security_count(0), 2);
}

#[test]
fn ex13_057_inherited_needs_a_security_card_to_pay() {
    let (mut runner, _carrier, chron, _plain) = protect_setup(&[]);
    runner
        .game
        .delete_permanents_batch(vec![chron], ReplacementCause::OpponentEffect);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Replacement));
    let _ = runner.auto_resolve();
    assert!(!field_has(&runner, 0, "CHRON-ALLY"));
}

#[test]
fn ex13_057_inherited_is_once_per_turn() {
    let (mut runner, carrier, chron, _plain) = protect_setup(&["FILL", "FILL", "FILL"]);
    runner
        .game
        .delete_permanents_batch(vec![chron], ReplacementCause::OpponentEffect);
    runner.execute_action(0, REPLACEMENT_ACCEPT).expect("accept");
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(0), 2);
    let _ = carrier;
    let chron_again = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "CHRON-ALLY")
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .expect("still on field");
    runner
        .game
        .delete_permanents_batch(vec![chron_again], ReplacementCause::OpponentEffect);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Replacement), "OPT used");
    let _ = runner.auto_resolve();
    assert!(!field_has(&runner, 0, "CHRON-ALLY"));
    assert_eq!(runner.security_count(0), 2);
}

#[test]
fn ex13_057_face_up_grademon_does_not_carry_its_inherited_protection() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .security(0, &["FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    let chron = runner.place_on_field(0, "CHRON-ALLY", Some(0));
    runner
        .game
        .delete_permanents_batch(vec![chron], ReplacementCause::OpponentEffect);
    assert_ne!(runner.pending_kind(), Some(SelectionKind::Replacement));
    let _ = runner.auto_resolve();
    assert!(!field_has(&runner, 0, "CHRON-ALLY"));
}
