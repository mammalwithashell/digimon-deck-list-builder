//! EX1-068 Ice Wall! — blue Option, cost 1.
//! [Main] All opponent Digimon gain [When Attacking] Lose 2 memory until
//! end of their next turn. [Security] Gain 2 memory.
//! Official observation 472a2180bbbd77ac42801729b5b7f630ae104bdd298e1770d2b02b477bf011c7,
//! actual EX1-068.webp, and Q3255–3257; base DCGO EX1/Blue/EX1_068.cs.
//! Q3256 explicitly includes Digimon played or moved from breeding later.
//! Patterns: native Option mask/cost, mandatory granted trigger, expiry,
//! dynamic recipient scope, clone isolation, security memory attribution.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCard, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "EX1-068";
const YAML: &str = include_str!("../../../cards/ex1/EX1-068.yaml");

// ── Card-data factories ──────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions (YAML parse + clause shape)
// ═══════════════════════════════════════════════════════════════════════════════

/// EX1-068 YAML must parse and compile without errors.
#[test]
fn ex1_068_yaml_parses_and_compiles() {
    let _runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX1-068 YAML must parse and compile without errors");
}

/// EX1-068 must compile as an Option card with cost 1 and Blue color.
#[test]
fn ex1_068_is_option_blue_cost_1() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX1-068 compiled card must be registered");

    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "EX1-068 must be an Option card"
    );
    assert_eq!(card.cost, Some(1), "EX1-068 prints Cost 1");
    assert!(
        card.color
            .iter()
            .any(|c| matches!(c, digimon_dsl::compiled::CompiledColor::Blue)),
        "EX1-068 must be a Blue Option (printed card_colors=[1]); got {:?}",
        card.color
    );
}

/// Two clauses total — the [Main] dynamic grant includes later entrants (Q3256)
/// (G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT resolved), alongside the
/// [Security] clause:
///   [0] main_from_hand (grant_triggered_effect → opponent Digimon)
///   [1] on_security (triggered, FaceUp scope, gain_memory: 2)
#[test]
fn ex1_068_has_two_clauses_after_main_grant_implemented() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");
    assert_eq!(
        card.effects.len(),
        2,
        "expected 2 clauses ([Main] grant + [Security] gain_memory: 2); got {} clauses",
        card.effects.len()
    );
}

/// The [Main] grant clause is a duration-scoped dynamic grant: a
/// `MainFromHand` triggered clause whose process is a
/// `GrantTriggeredEffect` with `timing = when_attacking` and a body that
/// loses 2 memory. Defends against regression to the omitted state AND
/// against a wrong-timing/wrong-body half-implementation.
#[test]
fn ex1_068_main_from_hand_grant_clause_shape() {
    use digimon_dsl::compiled::CompiledModifierTarget;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    let main_clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX1-068 must carry a MainFromHand [Main] clause");

    let grant = main_clause
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::GrantTriggeredEffect {
                target,
                timing,
                expiry,
                body,
                ..
            } => Some((target, timing, expiry, body)),
            _ => None,
        })
        .expect("[Main] process must contain a grant_triggered_effect step");

    let (target, timing, expiry, body) = grant;
    assert_eq!(
        timing, "when_attacking",
        "granted trigger fires when attacking"
    );
    assert_eq!(
        expiry, "end_of_opponents_next_turn",
        "grant lasts until the end of the opponent's next turn"
    );
    assert!(
        matches!(target, CompiledModifierTarget::Filter(_)),
        "target must be a predicate filter (opponent Digimon), got {target:?}"
    );
    assert!(
        body.iter()
            .any(|s| matches!(s, CompiledStep::LoseMemory(2))),
        "granted body must lose 2 memory; got {body:?}"
    );
}

/// Security clause — `OnSecurity` timing,
/// FaceUp scope (not inherited; this Option's printed text routes through
/// security as a normal Option-security effect, not as a digivolution-
/// source inherited effect), mandatory.
#[test]
fn ex1_068_security_clause_shape() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    let security_clause = card
        .effects
        .iter()
        .find(|c| matches!(c, CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)))
        .expect("EX1-068 must carry an OnSecurity [Security] clause");

    match security_clause {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::OnSecurity),
                "security clause must fire at OnSecurity; got {:?}",
                t.when
            );
            assert_eq!(
                t.scope,
                CompiledScope::FaceUp,
                "Option [Security] effects use FaceUp scope (no separate Security \
                 variant); got {:?}",
                t.scope
            );
            assert!(
                !t.optional,
                "Printed [Security] effect is mandatory; \
                 DCGO has no canNoSelect on the SecuritySkill ActivateClass"
            );
            assert!(
                !t.once_per_turn,
                "no [Once Per Turn] in printed [Security] text"
            );
        }
        other => panic!("clause 0 must be Triggered(on_security); got {:?}", other),
    }
}

/// The security clause's process must contain `GainMemory(2)`.
#[test]
fn ex1_068_security_process_contains_gain_memory_2() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let card = runner.compiled_card(CARD_ID).expect("EX1-068 compiled");

    let security_clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("EX1-068 must carry an OnSecurity [Security] clause");

    let has_gain_memory_2 = security_clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::GainMemory(2)));
    assert!(
        has_gain_memory_2,
        "[Security] process must contain GainMemory(2) — printed text 'Gain 2 memory'; \
         got {:?}",
        security_clause.process
    );
}

fn audit_runner() -> DebugRunner {
    use digimon_engine::enums::GamePhase;
    let mut b = DebugRunner::builder().from_dsl_yaml(YAML).unwrap();
    for (id, kind) in [
        ("A-DIG", CardKind::Digimon),
        ("A-TAM", CardKind::Tamer),
        ("A-OPT", CardKind::Option),
    ] {
        let mut c = make_test_card(id, id);
        c.card_kind = kind;
        c.colors = vec![CardColor::Blue];
        c.dp = Some(5000);
        c.level = Some(3);
        c.play_cost = 3;
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, &[CARD_ID])
        .deck(0, &["A-DIG"; 10])
        .deck(1, &["A-DIG"; 10])
        .security(0, &[CARD_ID])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r
}

fn use_ice(r: &mut DebugRunner) {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    assert!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        r.game.pending_selection.is_none(),
        "mandatory grant has no optional choice"
    );
    assert!(r.game.players[0].hand.is_empty());
}

fn fire_attack(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn ex1_068_native_use_color_mask_cost_and_current_recipients() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    let mut r = audit_runner();
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
    let own = r.place_on_field(0, "A-DIG", None);
    let a = r.place_on_field(1, "A-DIG", None);
    let b = r.place_on_field(1, "A-DIG", None);
    let t = r.place_on_field(1, "A-TAM", None);
    let o = r.place_on_field(1, "A-OPT", None);
    use_ice(&mut r);
    assert_eq!(r.memory(), 9);
    for h in [own, t, o] {
        fire_attack(&mut r, h);
        assert_eq!(r.memory(), 9);
    }
    fire_attack(&mut r, a);
    assert_eq!(r.memory(), 10, "P1 loss increases P0 gauge, capped at 10");
    r.game.set_memory(0);
    fire_attack(&mut r, a);
    fire_attack(&mut r, b);
    assert_eq!(r.memory(), 4, "each carrier loses 2, no OPT");
}

#[test]
fn ex1_068_grant_expiry_and_clone_are_independent() {
    let mut r = audit_runner();
    r.place_on_field(0, "A-DIG", None);
    let h = r.place_on_field(1, "A-DIG", None);
    use_ice(&mut r);
    let mut fork = audit_runner();
    fork.game = r.game.clone();
    r.game.modifiers.expire_end_of_turn(0);
    r.game.set_memory(0);
    fire_attack(&mut r, h);
    assert_eq!(r.memory(), 2);
    r.game.modifiers.expire_end_of_turn(1);
    r.game.set_memory(0);
    fire_attack(&mut r, h);
    assert_eq!(r.memory(), 0);
    fork.game.set_memory(0);
    fire_attack(&mut fork, h);
    assert_eq!(fork.memory(), 2);
}

#[test]
fn ex1_068_security_gain_is_owner_relative() {
    use digimon_engine::{enums::EffectTiming, selection::TriggerSource};
    for turn in [0, 1] {
        let mut r = audit_runner();
        r.game.turn_player_idx = turn;
        let card = r.game.players[0]
            .security
            .pop()
            .expect("real Ice Wall security card");
        let card_handle = card.handle();
        r.game.pending_security = Some(digimon_engine::selection::PendingSecurity {
            defender: 0,
            card,
            played: false,
        });
        r.game.set_memory(0);
        r.game.enqueue_triggered(
            EffectTiming::SecuritySkill,
            TriggerSource::SecurityRevealed {
                defender: 0,
                card: card_handle,
            },
        );
        r.game.drain_effect_queue();
        assert_eq!(r.memory(), if turn == 0 { 2 } else { -2 });
        assert!(r.game.pending_selection.is_none());
    }
}

// Production YAML is loaded unmodified: these tests expose missing Q3256 behavior.
fn tail_runner() -> DebugRunner {
    let mut r = audit_runner();
    r.place_on_field(0, "A-DIG", None);
    r
}
fn tail_attack(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.set_memory(0);
    fire_attack(r, h);
}
#[test]
fn ex1_068_tail_includes_later_entrants_and_repeated_attacks() {
    let mut r = tail_runner();
    use_ice(&mut r);
    let later = r.place_on_field(1, "A-DIG", None);
    tail_attack(&mut r, later);
    assert_eq!(
        r.memory(),
        2,
        "Q3256 includes opponent Digimon entering after resolution"
    );
    tail_attack(&mut r, later);
    assert_eq!(
        r.memory(),
        2,
        "duration grant is reusable, not consumed on firing"
    );
}
#[test]
fn ex1_068_tail_expiry_and_clone_are_independent() {
    let mut r = tail_runner();
    use_ice(&mut r);
    let mut fork = tail_runner();
    fork.game = r.game.clone();
    r.game.modifiers.expire_end_of_turn(0);
    let later = r.place_on_field(1, "A-DIG", None);
    tail_attack(&mut r, later);
    assert_eq!(r.memory(), 2);
    r.game.modifiers.expire_end_of_turn(1);
    tail_attack(&mut r, later);
    assert_eq!(r.memory(), 0);
    let fork_later = fork.place_on_field(1, "A-DIG", None);
    tail_attack(&mut fork, fork_later);
    assert_eq!(fork.memory(), 2);
}
#[test]
fn ex1_068_tail_includes_breeding_entrant_after_option_has_left_play() {
    let mut r = tail_runner();
    r.place_in_breeding(1, "A-DIG");
    use_ice(&mut r);
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "EX1-068"));
    assert!(r.move_from_breeding(1));
    let h = digimon_engine::permanent::PermanentHandle {
        player: 1,
        index: 0,
    };
    tail_attack(&mut r, h);
    assert_eq!(r.memory(), 2);
}
#[test]
fn ex1_068_tail_rechecks_immunity_using_grantor_option_kind() {
    use digimon_engine::enums::{EffectSourceKind, Expiry, ModifierType};
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
    let mut r = tail_runner();
    let h = r.place_on_field(1, "A-DIG", None);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::EndOfYourTurn, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Option),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    use_ice(&mut r);
    tail_attack(&mut r, h);
    assert_eq!(r.memory(), 0, "immune during install and trigger");
    // Remove only the immunity; the Ice Wall duration ends on the other player's turn.
    r.game.modifiers.clear_permanent(h);
    tail_attack(&mut r, h);
    assert_eq!(r.memory(), 2, "dynamic grant applies once immunity is gone");
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Digimon),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    tail_attack(&mut r, h);
    assert_eq!(
        r.memory(),
        2,
        "Digimon-only immunity does not block an Option grant"
    );
}
#[test]
fn ex1_068_tail_preserves_option_attribution_when_grantor_is_absent() {
    use digimon_engine::enums::{EffectSourceKind, Expiry, ModifierType};
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter, ModifierEntry};
    let mut r = tail_runner();
    use_ice(&mut r);
    r.game.players[0].trash.clear();
    let h = r.place_on_field(1, "A-DIG", None);
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, 1)
            .with_effect_immunity_filter(EffectImmunityFilter {
                source_kind: Some(EffectSourceKind::Option),
                controller: EffectControllerFilter::OpponentOnly,
            }),
    );
    tail_attack(&mut r, h);
    assert_eq!(
        r.memory(),
        0,
        "stored Option grantor kind survives source removal"
    );
    r.game.modifiers.clear_permanent(h);
    tail_attack(&mut r, h);
    assert_eq!(
        r.memory(),
        2,
        "duration survives the grantor card's disappearance"
    );
}

#[test]
fn ex1_068_tail_multiple_uses_stack_on_later_entrant() {
    use digimon_engine::action::build_action_mask;
    let mut r = tail_runner();
    use_ice(&mut r);
    r.add_to_hand(0, CARD_ID);
    use_ice(&mut r);
    let h = r.place_on_field(1, "A-DIG", None);
    r.game.set_memory(0);
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenAttacking,
        digimon_engine::selection::TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
    while let Some(p) = r.game.pending_selection.as_ref() {
        assert_eq!(
            p.kind,
            digimon_engine::selection::SelectionKind::TriggerOrder
        );
        let actor = p.selecting_player;
        let action = p.valid_action_ids[0];
        assert!(build_action_mask(&r.game, actor)[action as usize] > 0.0);
        r.game.decode_action(action, actor);
    }
    assert_eq!(r.memory(), 4, "two Ice Walls each grant one mandatory loss");
}

#[test]
fn ex1_068_tail_native_later_play_then_attack_loses_memory() {
    use digimon_engine::action::{build_action_mask, space::PLAY_HAND_START};
    let mut r = tail_runner();
    use_ice(&mut r);
    r.game.turn_player_idx = 1;
    r.game.set_memory(10);
    let index = r.add_to_hand(1, "A-DIG");
    let action = PLAY_HAND_START + index as u16;
    assert!(build_action_mask(&r.game, 1)[action as usize] > 0.0);
    r.game.decode_action(action, 1);
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    let h = digimon_engine::permanent::PermanentHandle {
        player: 1,
        index: 0,
    };
    fire_attack(&mut r, h);
    assert_eq!(
        r.memory(),
        5,
        "Q3255/Q3256: later-played carrier loses its own memory"
    );
}
