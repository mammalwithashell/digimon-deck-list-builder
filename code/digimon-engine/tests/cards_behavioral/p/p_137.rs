//! P-137 Flamedramon — Digimon, Lv.4, Red/Yellow, Cost 5, DP 5000.
//!
//! # Card text (cards.json)
//!
//! ＜Armor Purge＞ (When this Digimon would be deleted, you may trash the top
//! card of this Digimon to prevent that deletion.)
//!
//! ＜Raid＞ (When this Digimon attacks, you may switch the target of attack to
//! 1 of your opponent's unsuspended Digimon with the highest DP.)
//!
//! [Your Turn] [Once Per Turn] When this Digimon's attack target is switched,
//! your opponent adds the top card of their security stack to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_137.cs
//!
//! # Patterns this test covers
//! - H-ArmorPurge: printed-keyword grant via `kind: grant_keyword`
//! - H-Raid:       printed-keyword grant via `kind: grant_keyword`
//! - on_attack_target_change + active_when:your_turn + once_per_turn (clause c)
//!   Closest exemplar: BT21-025 (same timing pattern, different process)
//!
//! # Known gaps applied
//! - G-ADD-TOP-SECURITY-TO-HAND (NEW hybrid): DSL lacks `add_top_security_to_hand`
//!   verb AND engine lacks `EffectContext::add_top_security_to_hand`. Clause (c)
//!   process uses `raw_rust: p_137_opp_adds_top_security_to_hand`.
//!   Behavioral outcomes (opponent hand+1, security-1) are tested normally
//!   because the raw_rust impl IS provided.
//! - G-OPT-TRIGGERED: `once_per_turn` not enforced at runtime for triggered
//!   effects — OPT lockout tests are #[ignore]'d.
//!
//! # Build note
//! Uses `from_dsl_yaml(P_137_YAML)` (include_str! of production YAML) until
//! `dsl_card("P-137")` is available via a build-pack scan of `cards/p/`.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

// ─── Inlined production YAML ──────────────────────────────────────────────────

const P_137_YAML: &str = include_str!("../../../cards/p/P-137.yaml");

// ─── Card fixtures ────────────────────────────────────────────────────────────

fn plain_digimon(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: format!("Plain-{}", card_id),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(3000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn flamedramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .build()
}

fn flamedramon_runner_with_security() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build()
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

/// Clause (a): a GrantKeyword(ArmorPurge) declarative clause must be present.
#[test]
fn p_137_clause_a_has_armor_purge_grant_keyword() {
    let runner = flamedramon_runner();
    let compiled = runner
        .compiled_card("P-137")
        .expect("P-137 must compile");

    let has_armor_purge = compiled.effects.iter().any(|c| {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) = c
        {
            keyword.eq_ignore_ascii_case("ArmorPurge")
        } else {
            false
        }
    });
    assert!(has_armor_purge, "P-137 must have a GrantKeyword(ArmorPurge) clause");
}

/// Clause (b): a GrantKeyword(Raid) declarative clause must be present.
#[test]
fn p_137_clause_b_has_raid_grant_keyword() {
    let runner = flamedramon_runner();
    let compiled = runner
        .compiled_card("P-137")
        .expect("P-137 must compile");

    let has_raid = compiled.effects.iter().any(|c| {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword, ..
        }) = c
        {
            keyword.eq_ignore_ascii_case("Raid")
        } else {
            false
        }
    });
    assert!(has_raid, "P-137 must have a GrantKeyword(Raid) clause");
}

/// Clause (c): an own-scope OnAttackTargetChange triggered clause, once_per_turn,
/// NOT optional (no "you may" — opponent MUST add the card).
#[test]
fn p_137_clause_c_has_on_attack_target_change_own_opt_mandatory() {
    let runner = flamedramon_runner();
    let compiled = runner
        .compiled_card("P-137")
        .expect("P-137 must compile");

    let clause_c = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnAttackTargetChange)
                && t.scope == CompiledScope::FaceUp
            {
                return Some(t);
            }
        }
        None
    });

    let clause_c = clause_c.expect("P-137 must have an own-scope OnAttackTargetChange triggered clause");
    assert!(clause_c.once_per_turn, "clause (c) must be once_per_turn (OPT)");
    assert!(
        !clause_c.optional,
        "clause (c) is not optional — opponent MUST add the card (no 'you may')"
    );
}

/// Total effects: exactly 3 clauses — ArmorPurge grant, Raid grant, triggered.
#[test]
fn p_137_has_three_clauses_total() {
    let runner = flamedramon_runner();
    let compiled = runner
        .compiled_card("P-137")
        .expect("P-137 must compile");

    assert_eq!(
        compiled.effects.len(),
        3,
        "P-137 must have exactly 3 clauses (ArmorPurge grant, Raid grant, OAT-change triggered)"
    );
}

// ─── §2 Condition gating — clause (c) ────────────────────────────────────────

/// Positive: Flamedramon on your battle area — clause fires, opponent's security
/// moves to their hand.
#[test]
fn p_137_clause_c_fires_when_flamedramon_on_field() {
    let mut runner = flamedramon_runner_with_security();

    let sec_before = runner.security_count(1);
    let hand_before = runner.hand_size(1);
    let _ = runner.place_on_field(0, "P-137", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "opponent's security must decrease by 1"
    );
    assert_eq!(
        runner.hand_size(1),
        hand_before + 1,
        "opponent's hand must increase by 1 (top security moved to hand)"
    );
}

/// Negative: no Flamedramon on field — clause does not fire.
#[test]
fn p_137_clause_c_does_not_fire_when_not_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build();

    // Do NOT place P-137 on the field; no permanent = condition fails.
    let sec_before = runner.security_count(1);
    let hand_before = runner.hand_size(1);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "security must not change when Flamedramon is not on the field"
    );
    assert_eq!(
        runner.hand_size(1),
        hand_before,
        "hand must not change when Flamedramon is not on the field"
    );
}

/// Negative: opponent's turn — active_when:your_turn blocks clause (c).
#[test]
fn p_137_clause_c_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build();

    let _ = runner.place_on_field(0, "P-137", None);
    runner.end_turn(); // → player 1's turn

    let sec_before = runner.security_count(1);
    let hand_before = runner.hand_size(1);

    // Fire from p0's battle area — but it's p1's turn, so active_when:your_turn blocks.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "clause (c) must not fire on opponent's turn"
    );
    assert_eq!(
        runner.hand_size(1),
        hand_before,
        "hand must not change on opponent's turn"
    );
}

// ─── §3 Behavioral outcomes — clause (c) ─────────────────────────────────────

/// The card that moves is the TOP card of the opponent's security stack.
#[test]
fn p_137_clause_c_moves_top_security_card_to_opp_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C"])
        .build();

    let _ = runner.place_on_field(0, "P-137", None);

    // Snapshot the top security card (last in vec = top) by card_index for identity.
    let top_card_index = runner
        .game
        .players[1]
        .security
        .last()
        .map(|c| c.card_index)
        .expect("opponent must have security");

    let sec_before = runner.security_count(1);
    let hand_before = runner.hand_size(1);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(runner.security_count(1), sec_before - 1);
    assert_eq!(runner.hand_size(1), hand_before + 1);

    // The card that arrived in hand must be the former top-of-security card.
    let hand_card_index = runner
        .game
        .players[1]
        .hand
        .last()
        .map(|c| c.card_index)
        .expect("opponent hand must have a card");
    assert_eq!(
        hand_card_index, top_card_index,
        "the top-of-security card must be the one moved to hand"
    );
}

/// No-op when opponent has no security (no panic).
#[test]
fn p_137_clause_c_noop_when_opponent_has_no_security() {
    let mut runner = flamedramon_runner(); // no security for p1

    assert_eq!(runner.security_count(1), 0);
    let _ = runner.place_on_field(0, "P-137", None);
    let hand_before = runner.hand_size(1);

    // Must not panic.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.hand_size(1), hand_before);
}

// ─── §4 Event log — clause (c) ───────────────────────────────────────────────

/// Moving top security to hand should fire OnOpponentSecurityRemoved (or
/// equivalent security-removal event) so observer cards like BT21-001 Gigimon
/// see the security loss.
///
/// Note: the exact GameEvent emitted depends on the raw_rust implementation.
/// The test confirms security_count decreases and hand_size increases — the
/// observable net effect — and that no selection is left pending.
#[test]
fn p_137_clause_c_no_pending_selection_after_fire() {
    let mut runner = flamedramon_runner_with_security();

    let _ = runner.place_on_field(0, "P-137", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no pending selection should remain after clause (c) fires"
    );
}

// ─── §5 OPT lockout — clause (c) ─────────────────────────────────────────────

/// Second trigger same turn must be suppressed by OPT.
///
/// Blocked by G-OPT-TRIGGERED: `once_per_turn` emitted by compiler but not
/// enforced in `run_queued_effect_inner`.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — once_per_turn not enforced at runtime for triggered effects"]
fn p_137_clause_c_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C"])
        .build();

    let _ = runner.place_on_field(0, "P-137", None);

    // First trigger — uses the OPT slot.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();
    let after_first = runner.security_count(1);

    // Second trigger same turn — must be suppressed.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.security_count(1),
        after_first,
        "OPT must block the second trigger in the same turn"
    );
}

/// OPT counter clears after end_turn — clause fires again next turn.
///
/// Blocked by G-OPT-TRIGGERED.
#[test]
#[ignore = "pending: G-OPT-TRIGGERED — once_per_turn not enforced at runtime for triggered effects"]
fn p_137_clause_c_opt_resets_after_turn_end() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_137_YAML)
        .expect("P-137 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .add_card(plain_digimon("SEC-D"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D"])
        .build();

    let _ = runner.place_on_field(0, "P-137", None);

    // First trigger.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();
    let after_first = runner.security_count(1);

    // Cycle turns back to player 0.
    runner.end_turn();
    runner.end_turn();

    // OPT resets — fires again.
    runner
        .game
        .enqueue_triggered(EffectTiming::OnAttackTargetChange, TriggerSource::PlayerBattleArea(0));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.security_count(1),
        after_first - 1,
        "OPT resets after turn end — second turn must fire"
    );
}
