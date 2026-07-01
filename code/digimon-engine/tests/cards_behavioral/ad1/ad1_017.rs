//! AD1-017 Dynasmon — Digimon, Lv.6, Yellow/Red, DP 11000, Cost 11.
//! Traits: [Holy Warrior, Royal Knight]. Attribute: Data.
//!
//! # Printed card text (dossier — authoritative)
//!
//! When this card would be played, if you have 4 or more cards with [Lucemon]
//!   or [Witchelny] in its text in your trash, reduce the play cost by 5.
//! [On Play] [When Digivolving] By trashing your top or bottom security card,
//!   all of your opponent's Digimon get -6000 DP for the turn.
//! [All Turns] [Once Per Turn] When your security stack is removed from, you
//!   may delete 1 of your opponent's lowest DP Digimon.
//! Inherited [Security]: Give 1 of your opponent's Digimon <Security A. -1> for
//!   the turn. Then, 1 of their Digimon gets -3000 DP until your turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Yellow/AD1_017.cs

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCountAggregate, CompiledDeclarativeClause, CompiledDpConstraint,
    CompiledPredicate, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c
}

/// A trash card whose printed effect text contains `text` (for the cost
/// reduction count). Body unused in these tests — only `effect_text` matters.
fn text_card(id: &str, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.effect_text = text.to_string();
    c
}

/// Recursively search a CompiledPredicate tree for a node satisfying `f`.
fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p) || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().map(|q| pred_any(q, f)).unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 0 — load
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_017_loads_with_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-017").expect("compiled card");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.cost, Some(11));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Cost reduction: 4+ [Lucemon]/[Witchelny]-text cards in trash → -5
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_017_cost_reduction_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .start();
    let card = runner.compiled_card("AD1-017").expect("compiled");

    let (when_playing_this, condition, amount) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                when_playing_this,
                condition,
                amount,
                ..
            }) => Some((*when_playing_this, condition.clone(), *amount)),
            _ => None,
        })
        .expect("AD1-017 must carry a play-cost reduction clause");

    assert!(when_playing_this, "cost reduction applies to THIS card");
    assert_eq!(amount, Some(5), "reduce the play cost by 5");

    let cond = condition.expect("cost reduction must be conditional on the trash count");
    // The condition must be a count_gte(>=4) over trash cards whose text
    // contains Lucemon OR Witchelny.
    let agg: &CompiledCountAggregate = cond
        .count_gte
        .as_ref()
        .or_else(|| cond.all_of.iter().find_map(|p| p.count_gte.as_ref()))
        .expect("cost reduction condition must use count_gte over trash");
    assert_eq!(
        agg.n,
        CompiledDpConstraint::Literal(4),
        "count threshold must be 4 ([4 or more])"
    );
    assert!(
        pred_any(&agg.filter, |q| q.effect_text_contains.as_deref()
            == Some("Lucemon")),
        "count filter must include effect_text_contains: Lucemon"
    );
    assert!(
        pred_any(&agg.filter, |q| {
            q.effect_text_contains.as_deref() == Some("Witchelny")
        }),
        "count filter must include effect_text_contains: Witchelny"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] top/bottom security trash → board -6000
// ═══════════════════════════════════════════════════════════════════════════

/// The security-trash choice surfaces a 2-way select_effect_choice; picking the
/// "top" branch trashes the TOP security card, then every opponent Digimon gets
/// -6000 DP for the turn.
#[test]
fn ad1_017_on_play_trash_top_security_debuffs_all_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .add_card(make_test_card("SEC-TOP", "Top security"))
        .add_card(make_test_card("SEC-BOT", "Bottom security"))
        .add_card(opp_digimon("OPP-A", 7000))
        .add_card(opp_digimon("OPP-B", 9000))
        .memory(15)
        .start();
    // Own security: bottom-to-top order is push order, so SEC-BOT then SEC-TOP.
    runner.game.players[0].security.clear();
    for id in ["SEC-BOT", "SEC-TOP"] {
        let di = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == id)
            .unwrap();
        let ni = runner.game.next_card_index();
        runner.game.players[0]
            .security
            .push(CardSource::new(di, 0, ni));
    }

    let dyn_ = runner.place_on_field(0, "AD1-017", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);
    let opp_a_dp = runner.effective_dp(opp_a).unwrap();
    let opp_b_dp = runner.effective_dp(opp_b).unwrap();
    let sec_before = runner.security_count(0);

    runner.fire_on_play(0, dyn_.index as usize);

    // First: the 3-way choice (trash top / trash bottom / do not trash) —
    // DCGO AD1_017.cs offers all three; the trash is OPTIONAL.
    let view = runner
        .pending_selection_view()
        .expect("the top-or-bottom security trash choice must surface");
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        3,
        "three branches: trash top / trash bottom / do not trash (DCGO 3-way SetIntSelection)"
    );
    runner.execute_branch(0).expect("pick the trash-top branch");
    runner.game.drain_effect_queue();

    // Trashing OUR OWN top security legitimately fires the [All Turns][OPT]
    // own-security-removed observer (a faithful cascade). Decline it so this
    // test isolates the board-wide -6000 DP from the security-trash clause.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.is_optional,
            "the cascading own-security-removed delete is optional"
        );
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline the cascading own-security-removed delete");
        runner.game.drain_effect_queue();
    }

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "trashing the top security card removes exactly one own security card"
    );
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(opp_a_dp - 6000),
        "all opponent Digimon get -6000 DP for the turn"
    );
    assert_eq!(
        runner.effective_dp(opp_b),
        Some(opp_b_dp - 6000),
        "all opponent Digimon get -6000 DP for the turn"
    );
}

/// Picking the "bottom" branch trashes the BOTTOM security card (and still
/// applies the board-wide -6000 DP).
#[test]
fn ad1_017_on_play_trash_bottom_security_branch() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .add_card(make_test_card("SEC-BOT", "Bottom security"))
        .add_card(make_test_card("SEC-TOP", "Top security"))
        .add_card(opp_digimon("OPP-A", 8000))
        .memory(15)
        .start();
    runner.game.players[0].security.clear();
    let bot_idx;
    {
        let di = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "SEC-BOT")
            .unwrap();
        let ni = runner.game.next_card_index();
        bot_idx = ni;
        runner.game.players[0]
            .security
            .push(CardSource::new(di, 0, ni));
        let di2 = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "SEC-TOP")
            .unwrap();
        let ni2 = runner.game.next_card_index();
        runner.game.players[0]
            .security
            .push(CardSource::new(di2, 0, ni2));
    }

    let dyn_ = runner.place_on_field(0, "AD1-017", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_a_dp = runner.effective_dp(opp_a).unwrap();

    runner.fire_on_play(0, dyn_.index as usize);
    runner
        .execute_branch(1)
        .expect("pick the trash-bottom branch");
    runner.game.drain_effect_queue();

    // Decline the cascading own-security-removed observer (faithful cascade).
    if let Some(view) = runner.pending_selection_view() {
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline the cascading own-security-removed delete");
        runner.game.drain_effect_queue();
    }

    // The bottom security card (bot_idx) must be gone from the stack.
    let bottom_gone = !runner.game.players[0]
        .security
        .iter()
        .any(|c| c.card_index == bot_idx);
    assert!(
        bottom_gone,
        "trash-bottom must remove the bottom security card"
    );
    assert_eq!(
        runner.effective_dp(opp_a),
        Some(opp_a_dp - 6000),
        "the -6000 DP debuff still fires on the bottom branch"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns][OPT] own-security removed → may delete lowest-DP opp
// ═══════════════════════════════════════════════════════════════════════════

/// When player 0's own security is removed, AD1-017 may delete 1 of the
/// opponent's LOWEST-DP Digimon. The select is optional ("you may delete").
#[test]
fn ad1_017_own_security_removed_deletes_lowest_dp_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .add_card(make_test_card("SEC", "Security"))
        .add_card(opp_digimon("OPP-LOW", 3000))
        .add_card(opp_digimon("OPP-HIGH", 9000))
        .memory(15)
        .start();

    let dyn_ = runner.place_on_field(0, "AD1-017", Some(0));
    let low = runner.place_on_field(1, "OPP-LOW", None);
    let high = runner.place_on_field(1, "OPP-HIGH", None);
    let opp_before = runner.battle_area_size(1);

    // Fire the own-security-removed observer for player 0.
    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: digimon_engine::card_source::CardHandle(0),
            cause: digimon_engine::trigger_context::EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("own-security removal must offer the optional delete");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "delete target is OppField"
    );
    assert!(
        view.is_optional,
        "the delete is 'you may delete' (optional)"
    );
    // Only the lowest-DP Digimon (3000) is a valid pick; the 9000 one is not.
    let want_low = encode_attack(0, low.index as u16);
    let want_high = encode_attack(0, high.index as u16);
    assert!(
        view.valid_action_ids.contains(&want_low),
        "the lowest-DP opponent Digimon (3000) must be selectable"
    );
    assert!(
        !view.valid_action_ids.contains(&want_high),
        "only the LOWEST-DP Digimon is a valid target; the 9000-DP one must not be"
    );

    runner
        .execute_action(0, want_low)
        .expect("delete lowest-DP");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the lowest-DP opponent Digimon must be deleted"
    );
    let _ = dyn_;
    let _ = high;
}

/// The own-security-removed delete is declinable (PASS leaves the board intact).
#[test]
fn ad1_017_own_security_removed_delete_is_declinable() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .add_card(opp_digimon("OPP-LOW", 3000))
        .memory(15)
        .start();

    let _dyn = runner.place_on_field(0, "AD1-017", Some(0));
    runner.place_on_field(1, "OPP-LOW", None);
    let opp_before = runner.battle_area_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: digimon_engine::card_source::CardHandle(0),
            cause: digimon_engine::trigger_context::EventCause::SecurityRemoval,
        },
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("optional delete prompt installs");
    assert!(view.is_optional);
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional delete");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "declining the optional delete must leave the opponent's board intact"
    );
}

/// [Once Per Turn]: a second own-security removal on the same turn does not
/// re-offer the delete.
#[test]
fn ad1_017_own_security_removed_is_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .start();
    let card = runner.compiled_card("AD1-017").expect("compiled");
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnOwnSecurityRemoved))
        .expect("AD1-017 must carry an OnOwnSecurityRemoved clause");
    assert!(clause.once_per_turn, "[Once Per Turn] is printed");
    assert!(clause.optional, "'you may delete' → optional");
    assert!(
        clause
            .active_when
            .as_ref()
            .map(|p| pred_any(p, |q| q.all_turns == Some(true)))
            .unwrap_or(false),
        "[All Turns] gate so the observer fires on the opponent's turn too"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [Security]: Security A. -1 + -3000 DP debuffs
// ═══════════════════════════════════════════════════════════════════════════

/// Structural: the inherited [Security] clause must install a SecurityAttackChange
/// -1 modifier on one opponent Digimon and a -3000 DP modifier on another.
#[test]
fn ad1_017_inherited_security_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .start();
    let card = runner.compiled_card("AD1-017").expect("compiled");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        })
        .expect("AD1-017 must carry an inherited [Security] clause");

    use digimon_dsl::compiled::CompiledModifierValue;
    // A SecurityAttackChange add_modifier (value -1) step must be present.
    let has_sec_attack = clause.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::AddModifier { modifier, value: CompiledModifierValue::Literal(-1), .. }
                if modifier == "SecurityAttackChange"
        )
    });
    assert!(
        has_sec_attack,
        "inherited [Security] must give 1 opp Digimon SecurityAttackChange -1; got {:?}",
        clause.process
    );
    // A -3000 DP modifier (AddDpModifier) step must be present.
    let has_dp = clause.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::AddDpModifier {
                value: CompiledModifierValue::Literal(-3000),
                ..
            }
        )
    });
    assert!(
        has_dp,
        "inherited [Security] must give 1 opp Digimon -3000 DP; got {:?}",
        clause.process
    );
}

/// Behavioral: firing the inherited [Security] effect (AD1-017 buried as a
/// digivolution source under a host the security owner controls) installs the
/// two debuffs on the chosen opponent Digimon(s).
#[test]
fn ad1_017_inherited_security_installs_debuffs() {
    let mut host_top = make_test_card_with_level("HOST", "Host carrier", 6);
    host_top.card_kind = CardKind::Digimon;
    host_top.dp = Some(11000);

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 loads")
        .add_card(host_top)
        .add_card(opp_digimon("OPP-A", 8000))
        .add_card(opp_digimon("OPP-B", 8000))
        .memory(15)
        .start();
    // Host stack for the security owner (p0): AD1-017 is a buried digivolution
    // source so its INHERITED [Security] clause is the one that fires.
    let host = runner.place_on_field(0, "HOST", Some(0));
    runner.push_source(host, "AD1-017");
    // The security owner (p0)'s opponent is p1 — two p1 Digimon are the targets.
    let a = runner.place_on_field(1, "OPP-A", None);
    let b = runner.place_on_field(1, "OPP-B", None);
    let a_dp = runner.effective_dp(a).unwrap();
    let b_dp = runner.effective_dp(b).unwrap();

    // Fire the SecuritySkill timing on the host — the inherited AD1-017 source
    // contributes the [Security] clause.
    runner
        .game
        .enqueue_triggered(EffectTiming::SecuritySkill, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    // Drive the two mandatory target picks (Security A. -1 target, then DP target).
    let _ = runner.auto_resolve();

    // One opponent Digimon must carry the SecurityAttackChange (-1) debuff and
    // one must carry a -3000 DP reduction.
    let sec_a = runner
        .game
        .modifiers
        .has(a, ModifierType::SecurityAttackChange);
    let sec_b = runner
        .game
        .modifiers
        .has(b, ModifierType::SecurityAttackChange);
    assert!(
        sec_a || sec_b,
        "an opponent Digimon must receive the Security A. -1 debuff"
    );
    let dp_drop =
        runner.effective_dp(a) == Some(a_dp - 3000) || runner.effective_dp(b) == Some(b_dp - 3000);
    assert!(
        dp_drop,
        "an opponent Digimon must receive the -3000 DP debuff"
    );
}
