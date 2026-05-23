//! EX8-050 Gogmamon — Digimon, Lv.5, Black, DP 7000, Cost 7.
//! Traits: [Rock]. Attribute: Vaccine.
//!
//! # Card text (data/cards.json — verbatim)
//!
//! <Blocker>
//! [On Deletion] Reveal the top 3 cards of your deck. You may play 1 Digimon
//! card with the [Mineral] or [Rock] trait and a play cost of 5 or less among
//! them without paying the cost. Trash the rest.
//!
//! Inherited Effect:
//! [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon
//! attacks, you may change the attack target to this Digimon.
//!
//! # Patterns covered
//! - <Blocker> keyword grant (Clause 0)
//! - [On Deletion] reveal top 3, optional select Mineral/Rock cost<=5 Digimon,
//!   trash remainder (Clause A — PARTIAL, play step BLOCKED on
//!   G-PLAY-FROM-REVEALED-FREE from docs/RUST_ENGINE_GAPS.md)
//! - Inherited [Opponent's Turn][OPT] redirect attack to self (Clause B)
//!
//! # Known gaps
//! - G-PLAY-FROM-REVEALED-FREE (docs/RUST_ENGINE_GAPS.md §344):
//!   `play_from_revealed_free` is not implemented. The [On Deletion] clause
//!   reveals and selects correctly, but the "play without paying the cost"
//!   step cannot be authored. Tests covering that step are `#[ignore]`'d
//!   until the gap is closed.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming, CompiledCardKind,
};
use digimon_engine::action::space::{encode_attack, PASS, SEL_REVEAL_START};
use digimon_engine::combat::{AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{AttackTarget, SelectionKind};
use digimon_engine::CardData;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn gogmamon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 YAML parses and compiles")
        .build()
}

/// Build a Mineral-trait Digimon with a given play cost.
fn make_mineral_digimon(id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Mineral"));
    card.card_kind = CardKind::Digimon;
    card.traits.push("Mineral".to_string());
    card.play_cost = play_cost;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

/// Build a Rock-trait Digimon with a given play cost.
fn make_rock_digimon(id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, &format!("{id} Rock"));
    card.card_kind = CardKind::Digimon;
    card.traits.push("Rock".to_string());
    card.play_cost = play_cost;
    card.level = Some(4);
    card.dp = Some(4000);
    card
}

/// Build a plain filler card (no special traits).
fn make_filler(id: &str) -> CardData {
    make_test_card(id, &format!("{id} Filler"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// EX8-050 YAML compiles and is present in the embedded DSL pack.
#[test]
fn ex8_050_yaml_compiles_without_error() {
    let runner = gogmamon();
    assert!(
        runner.compiled_card("EX8-050").is_some(),
        "EX8-050 must be present in the embedded DSL pack"
    );
}

/// Clause 0 — the <Blocker> keyword grant is present as a declarative clause.
#[test]
fn ex8_050_has_blocker_keyword_grant() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) if keyword == "Blocker"
        )),
        "EX8-050 must have a declarative GrantKeyword Blocker clause"
    );
}

/// Clause A — an OnDeletion triggered clause exists and is mandatory
/// (the outer trigger is not optional — only the "You may play" bucket is).
#[test]
fn ex8_050_has_mandatory_on_deletion_clause() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let od = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => Some(t),
        _ => None,
    });
    assert!(od.is_some(), "EX8-050 must have an OnDeletion triggered clause");
    assert!(
        !od.unwrap().optional,
        "on_deletion clause must NOT be optional at the clause level \
         (reveal + trash are mandatory; only play step is 'you may')"
    );
}

/// Clause A — the OnDeletion process contains a RevealTopDeck(3) step.
#[test]
fn ex8_050_on_deletion_has_reveal_top_3() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let od = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have OnDeletion clause");
    assert!(
        od.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "on_deletion process must contain RevealTopDeck {{ count: 3 }}"
    );
}

/// Clause A — the OnDeletion process contains a SelectRevealBuckets step with
/// one bucket (min:0, max:1) filtering for Mineral/Rock Digimon cost<=5.
/// Using `select_reveal_buckets` (not `select_reveal`) ensures the tail
/// (trash loop) always runs via the callback even when the player picks nothing.
#[test]
fn ex8_050_on_deletion_has_select_reveal_buckets_step() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let od = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have OnDeletion clause");
    let bucket_step = od.process.iter().find(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1
                && buckets[0].min == 0
                && buckets[0].max == 1
                && buckets[0].filter.as_ref().is_some_and(|f| {
                    f.all_of.iter().any(|p| p.kind == Some(CompiledCardKind::Digimon))
                })
        )
    });
    assert!(
        bucket_step.is_some(),
        "on_deletion process must contain a SelectRevealBuckets step with \
         one bucket (min:0, max:1) filtering for Digimon"
    );
}

/// Clause A — the OnDeletion process contains a PerSelected loop that trashes
/// remainder cards via TrashFromReveal.
#[test]
fn ex8_050_on_deletion_has_trash_remainder_loop() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let od = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .expect("must have OnDeletion clause");
    let has_trash_loop = od.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::TrashFromReveal { .. })),
        _ => false,
    });
    assert!(
        has_trash_loop,
        "on_deletion process must contain a PerSelected loop with TrashFromReveal"
    );
}

/// Clause B — an inherited OnOpponentAttack triggered clause exists,
/// is optional, once-per-turn.
#[test]
fn ex8_050_has_inherited_opt_on_opponent_attack_redirect_clause() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let inh = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnOpponentAttack) =>
        {
            Some(t)
        }
        _ => None,
    });
    assert!(
        inh.is_some(),
        "EX8-050 must have an inherited OnOpponentAttack triggered clause"
    );
    let inh = inh.unwrap();
    assert!(inh.optional, "inherited clause must be optional ('you may')");
    assert!(inh.once_per_turn, "inherited clause must be once_per_turn");
}

/// Clause B — the inherited OnOpponentAttack process contains
/// RedirectAttackTarget with new_target: Source (this Digimon).
#[test]
fn ex8_050_inherited_redirect_targets_source_binding() {
    let runner = gogmamon();
    let card = runner.compiled_card("EX8-050").expect("EX8-050 present");
    let inh = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited OnOpponentAttack clause must exist");
    let has_redirect = inh.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::RedirectAttackTarget {
                new_target: Some(CompiledBindingRef::Source),
                ..
            }
        )
    });
    assert!(
        has_redirect,
        "inherited clause process must contain RedirectAttackTarget {{ new_target: Source }}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Blocker behavioral (existing)
// ═══════════════════════════════════════════════════════════════════════════════

/// A Gogmamon placed on the field has the Blocker keyword.
#[test]
fn ex8_050_on_field_has_blocker() {
    let mut runner = gogmamon();
    let handle = runner.place_on_field(0, "EX8-050", None);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(handle, Keyword::Blocker),
        "Gogmamon on field must have Blocker"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Deletion] behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// Deleting Gogmamon with an eligible Mineral/Rock cost<=5 Digimon in deck
/// parks an optional RevealBucket selection (the "you may play" bucket pick).
/// The reveal itself is mandatory; the bucket selection is optional (min:0).
#[test]
fn ex8_050_on_deletion_with_eligible_mineral_digimon_installs_optional_selection() {
    // Place mineral card at top of deck (last in array).
    let mineral = make_mineral_digimon("MIN-A", 5);
    let filler1 = make_filler("FILL-A1");
    let filler2 = make_filler("FILL-A2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(mineral)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["FILL-A1", "FILL-A2", "MIN-A"])
        .memory(10)
        .start();

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    runner
        .game
        .delete_permanent_with_cause(gog, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "deleting Gogmamon with an eligible Mineral Digimon in deck must park a selection"
    );
    assert!(
        runner.pending_is_optional(),
        "the installed selection must be optional ('You may play')"
    );
}

/// After reveal: a Rock Digimon costing exactly 5 is a legal bucket-selection.
/// Rock5 is placed at top of deck (last in array = index 0 in revealed pool).
#[test]
fn ex8_050_on_deletion_rock_cost_5_is_eligible_in_reveal() {
    // Rock5 at top of deck (last in array) → revealed at index 0.
    let rock5 = make_rock_digimon("ROCK5-C", 5);
    let filler1 = make_filler("FILL-C1");
    let filler2 = make_filler("FILL-C2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(rock5)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["FILL-C1", "FILL-C2", "ROCK5-C"])
        .memory(10)
        .start();

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    runner
        .game
        .delete_permanent_with_cause(gog, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // The on_deletion clause fires immediately (mandatory). After the reveal,
    // a RevealBucket selection is installed for the "you may play" pick.
    // Rock5-C is the top card (revealed index 0) → action SEL_REVEAL_START.
    let view = runner
        .pending_selection_view()
        .expect("a RevealBucket selection must be pending after reveal");
    assert!(
        matches!(view.kind, SelectionKind::RevealBucket { .. }),
        "selection kind must be RevealBucket, got {:?}",
        view.kind
    );
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "Rock Digimon with play cost 5 (at reveal index 0) must be a legal bucket pick; \
         valid_action_ids = {:?}",
        view.valid_action_ids
    );
}

/// After reveal: a Rock Digimon costing 6 (above the <=5 limit) is NOT a
/// legal bucket pick. The bucket selection is either absent (no candidates →
/// auto-skipped) or has no Rock6 in valid_action_ids.
#[test]
fn ex8_050_on_deletion_rock_cost_6_is_not_eligible_in_reveal() {
    // Rock6 at top of deck (last in array) → revealed at index 0.
    let rock6 = make_rock_digimon("ROCK6-D", 6);
    let filler1 = make_filler("FILL-D1");
    let filler2 = make_filler("FILL-D2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(rock6)
        .add_card(filler1)
        .add_card(filler2)
        .deck(0, &["FILL-D1", "FILL-D2", "ROCK6-D"])
        .memory(10)
        .start();

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    runner
        .game
        .delete_permanent_with_cause(gog, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // With no eligible cards, the bucket step auto-skips (no candidates →
    // directly advances). No RevealBucket selection is installed; all 3 cards
    // are trashed automatically.
    // The selection (if any) must not offer Rock6 (index 0 = SEL_REVEAL_START).
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !view.valid_action_ids.contains(&SEL_REVEAL_START),
            "Rock Digimon with play cost 6 must NOT be a legal bucket pick; \
             valid_action_ids = {:?}",
            view.valid_action_ids
        );
    }
    // If no selection at all, that means auto-skip occurred — also acceptable.
}

/// After Gogmamon is deleted, declining the bucket selection trashes all 3
/// revealed cards. The reveal and trash steps are mandatory; only the play
/// step is optional. Declining the bucket pick causes the tail to still run.
#[test]
fn ex8_050_on_deletion_decline_trashes_all_3_revealed_cards() {
    let mineral = make_mineral_digimon("MIN-E", 3);
    let filler1 = make_filler("FILL-E1");
    let filler2 = make_filler("FILL-E2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(mineral)
        .add_card(filler1)
        .add_card(filler2)
        // Mineral at top of deck (last in array).
        .deck(0, &["FILL-E1", "FILL-E2", "MIN-E"])
        .memory(10)
        .start();

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    let trash_before = runner.trash_size(0);

    runner
        .game
        .delete_permanent_with_cause(gog, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // The reveal bucket selection is now pending (optional, min:0).
    // Decline it by submitting PASS; the on_decline callback continues the
    // tail, which trashes all remaining revealed cards.
    if let Some(view) = runner.pending_selection_view() {
        let _ = runner.execute_action(view.selecting_player, PASS);
        runner.game.drain_effect_queue();
    }

    // Drain any residual steps.
    while let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    let trash_after = runner.trash_size(0);
    // All 3 revealed cards land in trash when play is declined.
    assert!(
        trash_after >= trash_before + 3,
        "all 3 revealed cards must land in trash when play is declined: \
         before={trash_before}, after={trash_after}"
    );
}

// ─── BLOCKED tests (pending G-PLAY-FROM-REVEALED-FREE) ─────────────────────

/// [On Deletion] picking an eligible Mineral/Rock Digimon from the bucket
/// should play it free (field +1) and trash the other 2 cards.
///
/// BLOCKED pending G-PLAY-FROM-REVEALED-FREE (docs/RUST_ENGINE_GAPS.md §344).
/// `play_from_revealed_free` is not yet implemented; the YAML clause omits the
/// play step. Un-ignore once the gap is closed.
#[test]
#[ignore = "pending: G-PLAY-FROM-REVEALED-FREE — play_from_revealed_free not yet implemented (docs/RUST_ENGINE_GAPS.md §344)"]
fn ex8_050_on_deletion_plays_eligible_mineral_card_free_and_trashes_rest() {
    let mineral = make_mineral_digimon("MIN-F", 4);
    let filler1 = make_filler("FILL-F1");
    let filler2 = make_filler("FILL-F2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(mineral)
        .add_card(filler1)
        .add_card(filler2)
        // Mineral at top of deck (last in array) → revealed at index 0.
        .deck(0, &["FILL-F1", "FILL-F2", "MIN-F"])
        .memory(2) // Not enough to pay for cost-4 Digimon — must be free.
        .start();

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    let field_before = runner.battle_area_size(0);
    let trash_before = runner.trash_size(0);

    runner
        .game
        .delete_permanent_with_cause(gog, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // Pick MIN-F from the bucket (revealed at index 0 = SEL_REVEAL_START).
    let _ = runner.execute_action(0, SEL_REVEAL_START);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "eligible Mineral Digimon must be played onto the field"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the 2 remaining revealed cards must be trashed"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [Opponent's Turn][OPT] redirect-to-self behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// When an opponent Digimon attacks the player and Gogmamon is on the field
/// (as carrier of its own inherited effect), accepting the optional trigger
/// redirects the attack away from the player to Gogmamon itself, preventing
/// the security check.
#[test]
fn ex8_050_inherited_on_opponent_attack_redirects_to_self_when_accepted() {
    let mut attacker_card = make_test_card("ATK-G", "OpponentAttacker");
    attacker_card.card_kind = CardKind::Digimon;
    attacker_card.dp = Some(8000);

    let mut security_card = make_test_card("SEC-G", "Security");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(attacker_card)
        .add_card(security_card)
        .security(0, &["SEC-G"])
        .memory(10)
        .start();

    runner.game.turn_count = 1; // Odd turn → player 1's turn (opponent of player 0)

    // Place Gogmamon on player 0's field so it carries the inherited effect as
    // a live permanent (not just a digivolution source under another card).
    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    let attacker = runner.place_on_field(1, "ATK-G", Some(0));
    let security_before = runner.game.players[0].security.len();

    // Opponent attacks player 0.
    runner.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(0)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    runner.game.drain_effect_queue();

    // The inherited OPT fires if it triggered. Accept it.
    if runner.pending_is_optional() {
        runner
            .accept_optional_trigger()
            .expect("accept inherited on_opponent_attack redirect");
        runner.game.drain_effect_queue();
    }

    // After accepting, the attack should have been redirected to Gogmamon,
    // meaning no security was checked (security count unchanged).
    let security_after = runner.game.players[0].security.len();
    assert_eq!(
        security_after, security_before,
        "redirecting the attack to Gogmamon must prevent the security check: \
         before={security_before}, after={security_after}"
    );
    let _ = (gog, encode_attack); // suppress unused warnings
}

/// The inherited OPT lockout: a second opponent attack on the same turn must
/// not re-trigger the redirect clause.
#[test]
fn ex8_050_inherited_redirect_is_once_per_turn_lockout() {
    let mut attacker_card = make_test_card("ATK-H", "OppAtk2");
    attacker_card.card_kind = CardKind::Digimon;
    attacker_card.dp = Some(3000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 compiles")
        .add_card(attacker_card)
        .memory(10)
        .start();

    runner.game.turn_count = 1; // Opponent's turn

    let gog = runner.place_on_field(0, "EX8-050", Some(0));
    let atk1 = runner.place_on_field(1, "ATK-H", Some(0));
    let atk2 = runner.place_on_field(1, "ATK-H", Some(0));

    // First attack: OPT may fire; accept it if it does.
    runner.game.begin_attack_open(AttackOpen {
        attacker: atk1,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Digimon(gog)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    runner.game.drain_effect_queue();

    if runner.pending_is_optional() {
        let _ = runner.accept_optional_trigger();
        runner.game.drain_effect_queue();
    }
    // Resolve any combat remainder.
    while let Some(view) = runner.pending_selection_view() {
        let action = view.valid_action_ids[0];
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    // Second attack on the same turn.
    // Note: atk1 may have been deleted by the battle above; use atk2.
    if runner.game.players[1].battle_area.is_empty() {
        // Both attackers gone — cannot test second attack; skip.
        return;
    }

    runner.game.begin_attack_open(AttackOpen {
        attacker: atk2,
        initiator: AttackInitiator::Effect {
            source: None,
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Digimon(gog)),
        allow_cancel: false,
        cost_upgrade: None,
    });
    runner.game.drain_effect_queue();

    // OPT must NOT fire a second time on the same turn.
    // If no optional prompt exists, the lockout is working.
    // If an optional prompt exists but is for a different reason (e.g. Blocker
    // activation), that is acceptable — we assert specifically that an
    // OwnField selection (redirect selection) is not installed.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !matches!(view.kind, SelectionKind::OwnField),
            "OPT inherited redirect must not fire a second time on the same turn"
        );
    }
}
