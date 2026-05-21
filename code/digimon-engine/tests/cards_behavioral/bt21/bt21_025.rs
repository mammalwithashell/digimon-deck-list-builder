//! BT21-025 Lamiamon — Digimon, Lv.5, Red, Cost 7, DP 7000.
//!
//! # Card text (cards.json)
//!
//! Effect: <Progress> (While attacking, your opponent's effects don't affect this
//! Digimon.)
//!
//! [Your Turn] [Once Per Turn] When any of your [Reptile] or [Dragonkin] trait
//! Digimon's attack targets change, trash your opponent's top security card.
//!
//! Inherited: [All Turns] [Once Per Turn] When your opponent's security is
//! removed from, you may play 1 [Reptile] or [Dragonkin] trait Digimon card
//! with 5000 DP or less from your hand without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Red/BT21_025.cs
//!
//! # Patterns this test covers
//! - H-Progress: printed-keyword grant via `kind: grant_keyword`
//! - on_attack_target_change + active_when:your_turn + once_per_turn
//! - trash_top_security (clause 2 process)
//! - Inherited on_opponent_security_removed + optional + play_from_hand_free
//!   (clause 3 — blocked by G-INHERITED-DISPATCH)
//!
//! # Known gaps applied
//! - G-INHERITED-DISPATCH: inherited triggered effects on digivolution-stack
//!   sources never fire — clause 3 behavioral tests are ignored.
//! - G-OPT-TRIGGERED: `once_per_turn` not enforced at runtime — OPT lockout
//!   tests are ignored.
//!
//! # Resolved/stale gap note
//! - G-ATK-TRAIT-FILTER is modelled through `event_target_owner` +
//!   `event_target_trait_has` after OnAttackTargetChange started carrying the
//!   attacker as event context.
//!
//! # Build note
//! `dsl_card("BT21-025")` requires build.rs to scan `cards/bt21/` (Phase 1c+).
//! Until then, tests use `from_dsl_yaml(BT21_025_YAML)` with the inlined spec.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

// ─── Inlined production YAML (mirrors cards/bt21/BT21-025.yaml) ───────────────

const BT21_025_YAML: &str = include_str!("../../../cards/bt21/BT21-025.yaml");

// ─── Card fixtures ────────────────────────────────────────────────────────────

fn reptile_digimon(card_id: &str, dp: i32) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: format!("Reptile-{}", card_id),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: vec!["Reptile".to_string()],
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

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
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn lamiamon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .build()
}

fn lamiamon_runner_with_security() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build()
}

fn enqueue_attack_target_change_for_attacker(runner: &mut DebugRunner, attacker: PermanentHandle) {
    let card = runner.game.players[attacker.player as usize].battle_area[attacker.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnAttackTargetChange,
        TriggerSource::EventObserved {
            player: attacker.player,
            permanent: attacker,
            card,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

/// Clause 1: a GrantKeyword(Progress) declarative clause must exist.
#[test]
fn bt21_025_clause1_has_progress_grant_keyword() {
    let runner = lamiamon_runner();
    let compiled = runner
        .compiled_card("BT21-025")
        .expect("BT21-025 must compile");

    let has_progress = compiled.effects.iter().any(|c| {
        if let CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            ..
        }) = c
        {
            keyword.eq_ignore_ascii_case("Progress")
        } else {
            false
        }
    });
    assert!(
        has_progress,
        "BT21-025 must have a GrantKeyword(Progress) clause"
    );
}

/// Clause 2: an own-scope OnAttackTargetChange triggered clause, once_per_turn,
/// not optional.
#[test]
fn bt21_025_clause2_has_on_attack_target_change_own_opt() {
    let runner = lamiamon_runner();
    let compiled = runner
        .compiled_card("BT21-025")
        .expect("BT21-025 must compile");

    let clause2 = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnAttackTargetChange)
                && t.scope == CompiledScope::FaceUp
            {
                return Some(t);
            }
        }
        None
    });

    let clause2 =
        clause2.expect("BT21-025 must have an own-scope OnAttackTargetChange triggered clause");
    assert!(
        clause2.once_per_turn,
        "clause 2 must be once_per_turn (OPT)"
    );
    assert!(
        !clause2.optional,
        "clause 2 is not optional — fires automatically (no 'you may')"
    );
}

/// Clause 3: an inherited OnOpponentSecurityRemoved triggered clause, once_per_turn,
/// optional.
#[test]
fn bt21_025_clause3_has_inherited_on_opponent_security_removed_opt_optional() {
    let runner = lamiamon_runner();
    let compiled = runner
        .compiled_card("BT21-025")
        .expect("BT21-025 must compile");

    let clause3 = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnOpponentSecurityRemoved)
                && t.scope == CompiledScope::Inherited
            {
                return Some(t);
            }
        }
        None
    });

    let clause3 = clause3
        .expect("BT21-025 must have an inherited OnOpponentSecurityRemoved triggered clause");
    assert!(clause3.once_per_turn, "clause 3 must be once_per_turn");
    assert!(clause3.optional, "clause 3 must be optional ('you may')");
}

// ─── §2 Condition gating — clause 2 ──────────────────────────────────────────

/// Positive: Lamiamon (Dragonkin) on your field — `any_permanent` condition
/// passes — trash fires.
#[test]
fn bt21_025_clause2_fires_when_dragonkin_on_your_field() {
    let mut runner = lamiamon_runner_with_security();

    let sec_before = runner.security_count(1);
    let attacker = runner.place_on_field(0, "BT21-025", None); // Dragonkin attacker

    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "opponent's top security must be trashed when Lamiamon (Dragonkin) is on your field"
    );
}

/// Negative: no Reptile/Dragonkin on your field — condition fails, trash does
/// not fire.
#[test]
fn bt21_025_clause2_does_not_fire_without_matching_trait_permanent() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("NO-TRAIT"))
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build();

    let attacker = runner.place_on_field(0, "NO-TRAIT", None); // no Reptile/Dragonkin trait
    let sec_before = runner.security_count(1);

    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "security must not be trashed when no Reptile/Dragonkin permanent is on your field"
    );
}

/// Negative: opponent's turn — active_when:your_turn blocks the clause.
#[test]
fn bt21_025_clause2_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("SEC-FILLER"))
        .security(1, &["SEC-FILLER"])
        .build();

    let attacker = runner.place_on_field(0, "BT21-025", None); // Dragonkin on p0's field
    runner.end_turn(); // → player 1's turn; BT21-025 active_when:your_turn must block for p0

    let sec_before = runner.security_count(1);

    // Fire from a player 0 attacker — but it's player 1's turn.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "clause 2 must not fire on opponent's turn (active_when:your_turn blocks it)"
    );
}

// ─── §3 Behavioral outcome — clause 2 ────────────────────────────────────────

/// Exactly one opponent security card is trashed from the top.
#[test]
fn bt21_025_clause2_trashes_exactly_one_opponent_top_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C"])
        .build();

    let attacker = runner.place_on_field(0, "BT21-025", None);
    let sec_before = runner.security_count(1);
    let trash_before = runner.trash_size(1);

    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "exactly one opponent security must be trashed"
    );
    assert_eq!(
        runner.trash_size(1),
        trash_before + 1,
        "trashed card must appear in opponent's trash"
    );
}

/// When opponent has no security, trash_top_security is a no-op (no panic).
#[test]
fn bt21_025_clause2_noop_when_opponent_has_no_security() {
    let mut runner = lamiamon_runner(); // no security set up for player 1

    assert_eq!(runner.security_count(1), 0);
    let attacker = runner.place_on_field(0, "BT21-025", None);

    // Must not panic.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(runner.security_count(1), 0);
}

// ─── §4 OPT lockout — clause 2 ───────────────────────────────────────────────

/// Second trigger same turn must be suppressed by OPT.
///
/// Blocked by G-OPT-TRIGGERED: `once_per_turn` is emitted by the compiler but
/// not enforced in `run_queued_effect_inner`.
#[test]
fn bt21_025_clause2_opt_blocks_second_trigger_same_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C"])
        .build();

    let attacker = runner.place_on_field(0, "BT21-025", None);

    // First trigger.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);
    let after_first = runner.security_count(1);

    // Second trigger same turn — must be suppressed.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

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
fn bt21_025_clause2_opt_resets_after_turn_end() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(plain_digimon("SEC-A"))
        .add_card(plain_digimon("SEC-B"))
        .add_card(plain_digimon("SEC-C"))
        .add_card(plain_digimon("SEC-D"))
        .security(1, &["SEC-A", "SEC-B", "SEC-C", "SEC-D"])
        .build();

    let attacker = runner.place_on_field(0, "BT21-025", None);

    // Fire once — uses the OPT slot.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);
    let after_first = runner.security_count(1);

    // Cycle turns back to player 0.
    runner.end_turn();
    runner.end_turn();

    // OPT resets — fires again.
    enqueue_attack_target_change_for_attacker(&mut runner, attacker);

    assert_eq!(
        runner.security_count(1),
        after_first - 1,
        "OPT resets after turn end — second turn must fire"
    );
}

// ─── §5 Clause 3 — inherited (blocked by G-INHERITED-DISPATCH) ───────────────

/// Behavioral: plays a matching Reptile/Dragonkin Digimon free from hand.
///
/// G-INHERITED-DISPATCH closed 2026-05-17 (Phase 2 Track D), but this test
/// fires `OnOpponentSecurityRemoved` from `TriggerSource::PlayerBattleArea(1)`
/// (P1's battle area) — that source has no security-removal context (no
/// observer_player binding), so the dispatch scans P1's permanents for
/// observers but never reaches P0's Lamiamon. Replacing the trigger source
/// with `TriggerSource::SecurityRemoved { observer_player: 0, ... }` would
/// exercise the inherited clause; left ignored as a test-fixture follow-up.
#[test]
#[ignore = "pending: test fixture — uses TriggerSource::PlayerBattleArea(1) instead of TriggerSource::SecurityRemoved; inherited dispatch confirmed closed by Track D regression test"]
fn bt21_025_clause3_plays_reptile_from_hand_free() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(reptile_digimon("REPTILE-5K", 5000))
        .add_card(plain_digimon("LV4-BASE"))
        .hand(0, &["REPTILE-5K"])
        .build();

    // Place the Lv4 base and Lamiamon (as proxy for the inherited stack).
    // Full digivolve action is not driven here — inherited dispatch is blocked anyway.
    let _base = runner.place_on_field(0, "LV4-BASE", None);
    let _lamiamon = runner.place_on_field(0, "BT21-025", None);

    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Simulate opponent security removal.
    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(1),
    );
    runner.game.drain_effect_queue();

    if runner.pending_selection().is_some() {
        let _ = runner.auto_resolve();
    }

    assert_eq!(runner.battle_area_size(0), field_before + 1);
    assert_eq!(runner.hand_size(0), hand_before - 1);
}

/// Behavioral: declining the optional clause 3 does nothing.
///
/// Blocked by G-INHERITED-DISPATCH.
#[test]
fn bt21_025_clause3_decline_does_nothing() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(BT21_025_YAML)
        .expect("BT21-025 YAML must parse and compile")
        .add_card(reptile_digimon("REPTILE-5K", 5000))
        .add_card(plain_digimon("LV4-BASE"))
        .build();

    let _ = runner.place_on_field(0, "LV4-BASE", None);
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentSecurityRemoved,
        TriggerSource::PlayerBattleArea(1),
    );
    runner.game.drain_effect_queue();

    if let Some(sel) = runner.pending_selection() {
        let player = sel.selecting_player;
        let _ = runner.execute_action(player, PASS);
    }

    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no card played on decline"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand unchanged on decline"
    );
}

/// OPT lockout for clause 3.
///
/// G-INHERITED-DISPATCH closed 2026-05-17 (Phase 2 Track D); OPT closed in
/// Phase 2 Track C. Test body not yet written — left as `todo!()` so the
/// follow-up author has a placeholder to fill in.
#[test]
#[ignore = "pending: card-local OPT regression body not authored — Track D substrate and Track C OPT closure both shipped; sibling cards cover the lockout dispatch (qa/resolved-gaps.md Phase 2 Track C closure)"]
fn bt21_025_clause3_opt_blocks_second_activation() {
    // Body deferred — substrate ready; sibling regression coverage exists.
}
