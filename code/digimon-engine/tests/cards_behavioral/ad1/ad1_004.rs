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

// ─── End-of-Turn attack: declared inside the effect ───────────────────────────
// Printed (card image + official bundle): "[End of Your Turn] 1 of your Digimon
// may attack." The attack is the effect's own processing (CR 4.3 15-1-2): DCGO
// AD1_004.cs selects the Digimon (`canNoSelect: true`, the printed "may") and
// then runs `SelectAttackEffect.Activate()` inline in the same coroutine. Any
// other end-of-turn trigger is ordered by the turn player against this whole
// effect (CR 4.3 15-4-2-3); see the BT20-036 official Q&A ("the 2nd [End of
// Your Turn] effect can be activated before the counter timing" of the first
// effect's attack) for the same timing.
//
// These two tests used to pin the opposite: a `windowed: true` MayAttack grant
// that deferred the attack to a later end-of-turn window, letting the player
// choose a Digimon, resolve other effects, and attack later (or never). That
// device is gone from the DSL (scan finding F02, BT25-057).
fn ad1_004_eot_runner() -> (DebugRunner, PermanentHandle, usize) {
    use digimon_engine::debug_runner::make_test_card;
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(make_test_card("SEC", "SecCard"))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 3;
    let wg = runner.place_on_field(0, "AD1-004", Some(0)); // not summoning-sick (can attack)
    let sec_before = runner.game.players[1].security.len();
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(wg));
    runner.game.drain_effect_queue();
    // Accept any outer "may" prompt, then pick WarGreymon as the attacker.
    let mut guard = 0;
    while runner.pending_kind() != Some(SelectionKind::Target) {
        guard += 1;
        assert!(guard < 6, "never reached the attack prompt");
        let view = runner
            .pending_selection_view()
            .expect("a pick before the attack prompt");
        let pick = if view
            .valid_action_ids
            .contains(&encode_attack(0, wg.index as u16))
        {
            encode_attack(0, wg.index as u16)
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|a| *a != PASS)
                .unwrap_or(PASS)
        };
        runner
            .execute_action(view.selecting_player, pick)
            .expect("drive EOT clause");
    }
    (runner, wg, sec_before)
}

#[test]
fn ad1_004_eot_attack_is_declared_inside_the_effect() {
    use digimon_engine::action::space::SECURITY_TARGET;
    use digimon_engine::enums::ModifierType;

    let (mut runner, wg, sec_before) = ad1_004_eot_runner();
    let view = runner.pending_selection_view().unwrap();
    let attack_id = encode_attack(wg.index as u16, SECURITY_TARGET);
    assert!(view.is_optional, "\"may attack\" is declinable");
    assert!(
        view.valid_action_ids.contains(&attack_id),
        "the chosen Digimon may attack the player now: {:?}",
        view.valid_action_ids
    );
    assert!(
        !runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "no deferred MayAttack grant"
    );

    runner.execute_action(0, attack_id).expect("attack");
    runner.game.drain_effect_queue();
    assert!(
        runner.game.players[0].battle_area[wg.index as usize].is_suspended,
        "WarGreymon attacked inside the effect"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        sec_before - 1,
        "the attack checked security inside the end-of-turn effect"
    );
}

#[test]
fn ad1_004_eot_attack_declined_leaves_no_deferred_attack() {
    use digimon_engine::action::space::SECURITY_TARGET;
    use digimon_engine::enums::ModifierType;

    let (mut runner, wg, sec_before) = ad1_004_eot_runner();
    runner.execute_action(0, PASS).expect("decline the attack");
    runner.game.drain_effect_queue();
    assert!(
        !runner.game.modifiers.has(wg, ModifierType::MayAttack),
        "declining leaves no MayAttack grant behind"
    );
    // In the end-of-turn action window, no attack is left over for WarGreymon
    // (the old windowed grant exposed one here).
    runner.set_phase(digimon_engine::enums::GamePhase::EndOfTurnAction);
    let mask = digimon_engine::action::build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[encode_attack(wg.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "no later end-of-turn attack is offered by the resolved effect"
    );
    assert!(!runner.game.players[0].battle_area[wg.index as usize].is_suspended);
    assert_eq!(runner.game.players[1].security.len(), sec_before);
}

// ─── Inherited [When Attacking][OPT] name-gated delete (Greymon/Omnimon) ─────
//
// Printed (card image + official Bandai DB):
//   "[When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with
//    as much or less DP as this Digimon with [Greymon] or [Omnimon] in its name."
// No "may": the effect is MANDATORY (CR 4.3 15-1-5, 15-9-1-2). Official Q&A
// Q6055: "You must activate this card's inherited effect and use up its [Once
// Per Turn], even if the attacking Digimon doesn't have [Greymon] or [Omnimon]
// in its name. However, ... you can't reference DP or delete a Digimon."
// DCGO AD1_004.cs "When Attacking - ESS": `SetUpActivateClass(CanActivateCondition,
// ActivateCoroutine, 1, false, ...)` (isOptional = false), CanActivateCondition =
// IsExistOnBattleAreaDigimon(card) only; the name gate and the target check sit
// INSIDE ActivateCoroutine; the pick is `canNoSelect: false`.
//
// These tests used to pin `optional: true` ("you may") with the name gate as
// the activation `condition`; both were wrong against the card, Q6055 and DCGO.

use digimon_engine::card_source::CardSource;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

/// A Lv.6 top card with the given name and DP, no effects of its own.
fn ad1_004_top(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(id, name, 6);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(dp);
    c
}

fn ad1_004_inherited_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(ad1_004_top("OMNI-TOP", "Omnimon", 15000))
        .add_card(ad1_004_top("PLAIN-TOP", "Plainmon", 15000))
        .add_card(digimon_with_dp("OPP-3K", 3000))
        .memory(15)
        .start()
}

fn ad1_004_fire_when_attacking(runner: &mut DebugRunner, attacker: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(attacker));
    runner.game.drain_effect_queue();
}

/// Inherited clause shape: inherited scope, when_attacking, once per turn,
/// NOT optional, and no activation condition (the name gate is in the body so
/// the effect activates -- and spends its [Once Per Turn] -- regardless).
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

    assert!(
        !inh.optional,
        "inherited delete prints no 'may': it is mandatory (Q6055, DCGO isOptional=false)"
    );
    assert!(inh.once_per_turn, "inherited delete is [Once Per Turn]");
    assert!(
        inh.condition.is_none(),
        "the [Greymon]/[Omnimon] name gate must not gate ACTIVATION (Q6055: the effect \
         activates and uses up its [Once Per Turn] even without the name)"
    );
}

/// The stack's top card is named [Omnimon]: attacking goes straight to the
/// mandatory target pick -- no accept/decline prompt -- and the pick deletes.
#[test]
fn ad1_004_inherited_delete_is_mandatory_with_no_accept_decline_prompt() {
    let mut runner = ad1_004_inherited_runner();
    let attacker = runner.place_stack(0, &["AD1-004", "OMNI-TOP"]);
    let opp = runner.place_on_field(1, "OPP-3K", None);

    ad1_004_fire_when_attacking(&mut runner, attacker);

    let view = runner
        .pending_selection_view()
        .expect("the inherited delete must ask for its target");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "the first prompt is the delete target, not an optional accept/decline"
    );
    assert!(!view.is_optional, "the target pick is mandatory (canNoSelect: false)");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "a mandatory effect offers no decline"
    );
    let want = encode_attack(0, opp.index as u16);
    assert!(view.valid_action_ids.contains(&want));
    runner.execute_action(0, want).expect("delete the target");
    runner.game.drain_effect_queue();
    assert_eq!(runner.battle_area_size(1), 0, "the 3000 DP Digimon is deleted");
}

/// Q6055: a top card WITHOUT [Greymon]/[Omnimon] in its name still activates
/// the inherited effect and uses up its [Once Per Turn], but deletes nothing.
/// Proof of the spent use: the same stack, now topped by an [Omnimon], gets
/// no delete from a second attack in the same turn.
#[test]
fn ad1_004_inherited_delete_spends_once_per_turn_without_the_name_q6055() {
    let mut runner = ad1_004_inherited_runner();
    let attacker = runner.place_stack(0, &["AD1-004", "PLAIN-TOP"]);
    runner.place_on_field(1, "OPP-3K", None);

    ad1_004_fire_when_attacking(&mut runner, attacker);
    assert!(
        runner.pending_selection().is_none(),
        "without the name nothing is selected (and nothing is asked)"
    );
    assert_eq!(runner.battle_area_size(1), 1, "nothing is deleted without the name");

    // Same turn: the stack gains an [Omnimon] top card and attacks again.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OMNI-TOP")
        .expect("OMNI-TOP registered");
    let card_index = runner.game.next_card_index();
    runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .push(CardSource::new(data_idx, 0, card_index));
    runner.game.tick_declarative_effects();

    ad1_004_fire_when_attacking(&mut runner, attacker);
    assert!(
        runner.pending_selection().is_none(),
        "the [Once Per Turn] was used up by the first attack (Q6055)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "no delete: the inherited effect already activated this turn"
    );
}

/// No opponent Digimon at or below this Digimon's DP: nothing to select.
#[test]
fn ad1_004_inherited_delete_has_no_prompt_without_an_eligible_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 in pack")
        .add_card(ad1_004_top("OMNI-TOP", "Omnimon", 15000))
        .add_card(digimon_with_dp("OPP-16K", 16000))
        .memory(15)
        .start();
    let attacker = runner.place_stack(0, &["AD1-004", "OMNI-TOP"]);
    runner.place_on_field(1, "OPP-16K", None);

    ad1_004_fire_when_attacking(&mut runner, attacker);
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 1);
}
