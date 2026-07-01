//! AD1-004 WarGreymon

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Opponent Digimon with a given DP (target for the source-DP delete).
fn digimon_with_dp(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, id, 5);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c
}

/// A Tamer with a single color (to drive the color-count auras).
fn tamer_with_color(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}

#[test]
fn ad1_004_has_keywords_and_end_turn_attack_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-004").expect("compiled card");

    let keyword_count = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(keyword_count, 2);
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn)
    )));
}

// ─── Face-up keyword grants are runtime-installed (test-gap; AD1-025 idiom) ──

/// Face-up <Raid> must install on AD1-004's carrier at runtime, not just
/// compile structurally (G-DECLARATIVE-KEYWORD resolved).
#[test]
fn ad1_004_face_up_raid_installs_on_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(wg, Keyword::Raid),
        "face-up GrantKeyword(Raid) must install Raid on AD1-004's carrier"
    );
}

/// Face-up <Piercing> must install on AD1-004's carrier at runtime.
#[test]
fn ad1_004_face_up_piercing_installs_on_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(wg, Keyword::Piercing),
        "face-up GrantKeyword(Piercing) must install Piercing on AD1-004's carrier"
    );
}

// ─── [On Play][When Digivolving] delete opp Digimon with DP <= this Digimon ──
//
// G-FORMULA-SOURCE-DP shipped — `dp_lte: { formula: { source_dp: {} } }`
// (AD1-002 / AD1-005 / P-182 idiom). WarGreymon is DP 12000.

/// An opponent Digimon at exactly 12000 DP (= WarGreymon's DP) IS a valid
/// delete target; the delete prompt is mandatory (DCGO canNoSelect: false).
#[test]
fn ad1_004_deletes_opponent_digimon_at_or_below_self_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(digimon_with_dp("OPP-12K", 12000))
        .memory(15)
        .start();

    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    let weak = runner.place_on_field(1, "OPP-12K", None);
    let opp_before = runner.battle_area_size(1);

    runner.fire_on_play(0, wg.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("delete prompt must install for an eligible opponent Digimon");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "delete target select must be OppField"
    );
    assert!(
        !view.is_optional,
        "delete is mandatory when an eligible target exists (DCGO canNoSelect: false)"
    );
    let want = encode_attack(0, weak.index as u16);
    assert!(
        view.valid_action_ids.contains(&want),
        "the 12000 DP opponent (== WarGreymon DP) must be selectable (<=)"
    );

    runner.execute_action(0, want).expect("delete the target");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1),
        opp_before - 1,
        "the eligible 12000 DP opponent Digimon must be deleted"
    );
}

/// An opponent Digimon with DP > WarGreymon's DP is NOT eligible — with no
/// other target the clause installs no prompt and deletes nothing.
#[test]
fn ad1_004_does_not_delete_opponent_above_self_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(digimon_with_dp("OPP-13K", 13000))
        .memory(15)
        .start();

    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.place_on_field(1, "OPP-13K", None);
    let opp_before = runner.battle_area_size(1);

    runner.fire_on_play(0, wg.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "a 13000 DP opponent (> WarGreymon 12000) must not be a valid delete target"
    );
    assert_eq!(
        runner.battle_area_size(1),
        opp_before,
        "the above-DP opponent Digimon must survive"
    );
}

// ─── [All Turns] color-count DP aura + Security A. floor(colors/3) ───────────
//
// "+1000 DP for each of your Tamers' colors; and gains Security A. +1 for
// every 3 of those colors." Tamers contribute DISTINCT colors.

/// With no Tamers in play, WarGreymon gets +0 DP from the aura (base DP only).
#[test]
fn ad1_004_color_dp_aura_zero_without_tamers() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(wg),
        Some(12000),
        "no Tamers → no color-count DP bonus (base 12000)"
    );
}

/// Two own Tamers of DISTINCT colors grant +2000 DP (1000 per distinct color).
#[test]
fn ad1_004_color_dp_aura_counts_distinct_tamer_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(tamer_with_color("TAMER-RED", CardColor::Red))
        .add_card(tamer_with_color("TAMER-BLUE", CardColor::Blue))
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.place_on_field(0, "TAMER-RED", Some(0));
    runner.place_on_field(0, "TAMER-BLUE", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(wg),
        Some(12000 + 2000),
        "two distinct Tamer colors → +2000 DP"
    );
}

/// Two own Tamers of the SAME color grant only +1000 DP (distinct-color count).
#[test]
fn ad1_004_color_dp_aura_collapses_duplicate_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(tamer_with_color("TAMER-R1", CardColor::Red))
        .add_card(tamer_with_color("TAMER-R2", CardColor::Red))
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));
    runner.place_on_field(0, "TAMER-R1", Some(0));
    runner.place_on_field(0, "TAMER-R2", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(wg),
        Some(12000 + 1000),
        "two same-color Tamers count as ONE distinct color → +1000 DP"
    );
}

/// Security check count: base 1 with <3 Tamer colors; 3 distinct colors give
/// floor(3/3)=1 extra check → 2 total (security_attack_fn is base-inclusive).
#[test]
fn ad1_004_security_attack_aura_grants_one_extra_check_per_three_colors() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(tamer_with_color("T-RED", CardColor::Red))
        .add_card(tamer_with_color("T-BLUE", CardColor::Blue))
        .add_card(tamer_with_color("T-GREEN", CardColor::Green))
        .start();
    let wg = runner.place_on_field(0, "AD1-004", Some(0));

    // 0 Tamers: 1 security check (base), no bonus. The base-inclusive
    // security_attack_fn yields floor((3+0)/3) = 1.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(wg),
        1,
        "0 Tamer colors → base 1 security check, floor(0/3)=0 bonus"
    );

    // 3 distinct Tamer colors: 1 + floor(3/3) = 2 checks.
    runner.place_on_field(0, "T-RED", Some(0));
    runner.place_on_field(0, "T-BLUE", Some(0));
    runner.place_on_field(0, "T-GREEN", Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.game.effective_security_strike(wg),
        2,
        "3 distinct Tamer colors → 1 + floor(3/3) = 2 security checks"
    );
}

// ─── End-of-Turn attack: windowed grant (not synchronous inline) ─────────────
// 2026-06-02: AD1-004's "[End of Your Turn] 1 of your Digimon may attack" must
// GRANT a windowed MayAttack to the chosen Digimon (deferred to the EOT-action
// window) rather than declaring + resolving the attack inline. The windowed
// model lets sibling end-of-turn effects (e.g. an inherited DNA digivolve)
// resolve first and remove the attacker, so its attack fizzles — faithful to
// general_rule.pdf §15-4-2-3 (EOT triggers activate one at a time) and the
// "attack ends if the attacker leaves before it resolves" rule.
#[test]
fn ad1_004_eot_attack_is_windowed_grant_not_synchronous() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::{EffectTiming, ModifierType};
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(make_test_card("SEC", "SecCard"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    let wg = runner.place_on_field(0, "AD1-004", None);
    let sec_before = runner.game.players[1].security.len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(wg));
    runner.game.drain_effect_queue();
    // Drive the optional EOT clause: accept + pick WarGreymon as the attacker.
    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 8, "selection loop runaway");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap_or(PASS);
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive EOT clause");
        if runner.game.modifiers.has(wg, ModifierType::MayAttack) {
            break;
        }
    }

    assert!(
        runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "AD1-004 EOT attack must grant a windowed MayAttack to the chosen Digimon (deferred), not resolve inline"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before,
        "windowed grant must NOT check security synchronously during the EOT trigger"
    );
}

// Capstone fizzle: once AD1-004's EOT attack is a windowed grant, removing the
// chosen attacker (the user's DNA-digivolve-into-Omnimon line consuming
// WarGreymon) before it takes its deferred attack leaves the grant orphaned —
// no attack happens and the opponent's security is untouched. This is the
// system-level outcome the inline path could never produce.
#[test]
fn ad1_004_eot_attack_fizzles_when_attacker_is_removed_before_it_acts() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::{EffectTiming, ModifierType};
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(make_test_card("SEC", "SecCard"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    let wg = runner.place_on_field(0, "AD1-004", None);
    let sec_before = runner.game.players[1].security.len();

    // EOT: grant the windowed attack to WarGreymon.
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(wg));
    runner.game.drain_effect_queue();
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 8, "selection loop runaway");
        let view = runner.pending_selection_view().unwrap();
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != digimon_engine::action::space::PASS)
            .unwrap_or(digimon_engine::action::space::PASS);
        runner.execute_action(view.selecting_player, pick).unwrap();
        if runner.game.modifiers.has(wg, ModifierType::MayAttack) {
            break;
        }
    }
    assert!(
        runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "precondition: WarGreymon has the windowed MayAttack grant"
    );

    // The DNA digivolve consumes WarGreymon (simulated by removing it).
    runner.game.delete_permanent_with_effects(wg);

    // The granted attack is orphaned: no attack action for the gone attacker,
    // and security is untouched (the attack fizzled).
    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_attack(wg.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "a removed attacker must expose no end-of-turn attack action"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before,
        "DNA'ing the attacker away before its deferred attack must leave opponent security untouched (fizzle)"
    );
}

// ─── Inherited [When Attacking][OPT] name-gated delete (Greymon/Omnimon) ─────
//
// "If the carrier's name contains [Greymon] or [Omnimon], delete 1 opponent
// Digimon with DP <= this Digimon's DP." `source_name_contains` (AD1-014
// inherited idiom) + dp_lte source_dp.

/// Inherited clause must compile: inherited scope, when_attacking, optional,
/// once_per_turn, gated by source_name_contains Greymon/Omnimon.
#[test]
fn ad1_004_inherited_when_attacking_delete_clause_compiles() {
    use digimon_dsl::compiled::CompiledScope;
    let runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .start();
    let card = runner.compiled_card("AD1-004").expect("compiled card");

    let inh = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("AD1-004 must carry an inherited [When Attacking] clause");

    assert!(inh.optional, "inherited delete is 'you may' (optional)");
    assert!(inh.once_per_turn, "inherited delete is [Once Per Turn]");
    // The Greymon/Omnimon name gate must appear somewhere in the active_when /
    // condition predicate tree.
    let gate = inh.condition.as_ref().or(inh.active_when.as_ref());
    fn pred_has_name(p: &digimon_dsl::compiled::CompiledPredicate, name: &str) -> bool {
        if p.source_name_contains.as_deref() == Some(name) {
            return true;
        }
        p.all_of.iter().any(|q| pred_has_name(q, name))
            || p.any_of.iter().any(|q| pred_has_name(q, name))
            || p.none_of.iter().any(|q| pred_has_name(q, name))
            || p.not
                .as_ref()
                .map(|q| pred_has_name(q, name))
                .unwrap_or(false)
    }
    let g = gate.expect("inherited delete must carry a name-gate condition");
    assert!(
        pred_has_name(g, "Greymon") && pred_has_name(g, "Omnimon"),
        "inherited delete gate must check carrier name contains Greymon OR Omnimon"
    );
}
