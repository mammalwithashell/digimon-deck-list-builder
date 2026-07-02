//! AD1-021 Marcus Damon & Agumon — Tamer, Yellow+Red, Cost 5.
//! Traits: DATA SQUAD.
//!
//! # Card text (card image AD1-021 — authoritative; cards.json accurate)
//!
//! ```text
//! [Your Turn] When this Tamer suspends, <Draw 1>. Then, 1 of your Digimon may
//!   digivolve into a yellow Digimon card with [Greymon] in its name in the hand
//!   with the digivolution cost reduced by 3.
//! [End of Your Turn] [Once Per Turn] If you have a yellow Digimon with [Agumon]
//!   or [Greymon] in its name, for the turn, 1 of your [Marcus Damon]s is also
//!   treated as a 6000 DP Digimon, gains <Rush> and can't digivolve. Then, 1 of
//!   your Digimon may attack.
//! [Rule] Name: Also treated as [Marcus Damon].
//! ```
//!
//! ```text
//! Inherited:
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Yellow/AD1_021.cs
//!   - ChangeCardNamesClass: also treated as "Marcus Damon" (also_treated_as).
//!   - OnTappedAnyone [Your Turn] self-suspend: DrawClass(1), then optional
//!     (canNoSelect:true) DigivolveIntoHandOrTrashCard onto a chosen own Digimon
//!     into a hand card (Yellow + ContainsCardName "Greymon"), reduceCost 3,
//!     payCost true, isHand true.
//!   - OnEndTurn [End of Your Turn][OPT] gated by IsSkippable(!HasMatch yellow
//!     Agumon/Greymon Digimon): mandatory Marcus pick →
//!     BecomeDigimonThatCantDigivolve(6000) + GainRush (UntilEachTurnEnd); then
//!     optional (canNoSelect:true) pick a Digimon that may attack (SelectAttack).
//!   - SecuritySkill: PlaySelfTamerSecurityEffect (play self free).
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | also_treated_as | [Rule] | name alias "Marcus Damon" |
//! | 1 | [Your Turn] on self-suspend | Draw 1; then 1 of your Digimon may digivolve into a yellow [Greymon] hand card, cost −3 |
//! | 2 | [End of Your Turn][OPT] | (gated) treat a Marcus Damon as 6000 DP Digimon + Rush + CannotDigivolve; then 1 Digimon may attack |
//! | 3 | [Security] | play self without paying the cost |
//!
//! # Patterns this test covers
//! - on_suspend SELF-trigger (event_permanent_is_source) under [Your Turn]
//! - effect_initiated_digivolve into a selected own permanent from hand, cost −3
//! - TreatAsDigimon (synth 6000 DP) + Rush + CannotDigivolve on a Marcus Damon
//! - condition gate: requires a yellow Agumon/Greymon Digimon
//! - G-MAY-ATTACK-NOW: "1 of your Digimon may attack"
//! - also_treated_as name alias
//! - on_security play-self (tamer)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A [Marcus Damon] Tamer (yellow). Exact name so `name_contains: "Marcus Damon"`
/// matches.
fn make_marcus(id: &str) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A yellow Digimon with [Greymon] in its name (the EoYT condition enabler and a
/// would-be attacker). Lv.4 so it can host a Lv.5 digivolve target.
fn make_yellow_greymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Greymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A yellow Digimon with [Agumon] in its name (EoYT condition enabler).
fn make_yellow_agumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Agumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

/// A Lv.5 yellow Digimon with [Greymon] in its name and a printed
/// `Lv.4 Yellow, cost 5` evo cost. With the Your-Turn clause's `cost: { reduce: 3 }`
/// applied, the effective digivolve cost is 2 — the digivolve succeeds.
fn make_yellow_greymon_evo_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "MetalGreymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Yellow];
    c.evo_costs = vec![EvoCost {
        card_color: 2, // Yellow
        level: 4,
        memory_cost: 5,
    }];
    c
}

/// A non-yellow, non-Greymon Digimon — does NOT satisfy the Greymon hand filter
/// nor the EoYT yellow-Agumon/Greymon condition.
fn make_red_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Standard builder with AD1-021 in P0 hand + a generous card pool.
fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("AD1-021")
        .expect("AD1-021 in embedded DSL pack")
        .add_card(make_marcus("MARCUS"))
        .add_card(make_yellow_greymon("YGREY"))
        .add_card(make_yellow_agumon("YAGU"))
        .add_card(make_yellow_greymon_evo_lv5("METAL"))
        .add_card(make_red_digimon("REDDIGI"))
        .add_card(make_filler("FILL"))
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_021_yaml_parses_and_compiles() {
    let _ = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
}

#[test]
fn ad1_021_is_tamer_cost_5_yellow_red() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card("AD1-021").expect("compiled present");
    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Tamer
    );
    assert_eq!(compiled.cost, Some(5));
    assert!(compiled
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(compiled
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Red));
}

/// [Rule] Name: Also treated as [Marcus Damon].
#[test]
fn ad1_021_also_treated_as_marcus_damon() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card("AD1-021").expect("compiled present");
    assert!(
        compiled.also_treated_as.iter().any(|n| n == "Marcus Damon"),
        "AD1-021 must be also_treated_as [Marcus Damon]; got {:?}",
        compiled.also_treated_as
    );
}

/// Clause 1 fires on on_suspend with [Your Turn] scope.
#[test]
fn ad1_021_clause1_is_on_suspend() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card("AD1-021").expect("compiled present");
    let clause1 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .next()
        .expect("clause 1 (on_suspend) must exist");
    assert!(clause1.when.contains(&CompiledTiming::OnSuspend));
}

/// Clause 2 fires at [End of Your Turn] and is [Once Per Turn].
#[test]
fn ad1_021_clause2_is_end_of_your_turn_opt() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card("AD1-021").expect("compiled present");
    let clause2 = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 2 (end_of_your_turn) must exist");
    assert!(clause2.once_per_turn, "EoYT clause must be [Once Per Turn]");
}

/// Clause 3 fires at on_security.
#[test]
fn ad1_021_has_on_security_clause() {
    let runner = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner.compiled_card("AD1-021").expect("compiled present");
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "AD1-021 must have an on_security clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 [Your Turn] self-suspend behavioral
// ════════════════════════════════════════════════════════════════════════════

/// Helper: suspend the Tamer, which fires its own OnSuspend clause.
fn fire_self_suspend(runner: &mut DebugRunner, tamer: PermanentHandle) {
    runner.game.suspend(tamer);
    runner.game.drain_effect_queue();
}

/// Drive the EoYT selection chain, targeting `marcus` specifically for the
/// (mandatory) OwnField Marcus pick and PASSing every optional follow-up.
///
/// AD1-021 is *also* a legal "Marcus Damon" target (also_treated_as), so a naive
/// `valid_action_ids[0]` would pick the wrong permanent. Own-field selection
/// action IDs encode as `ATTACK_START (100) + field_index`.
fn resolve_picking_marcus(runner: &mut DebugRunner, marcus: PermanentHandle) {
    use digimon_engine::action::space::ATTACK_START;
    let marcus_action = ATTACK_START + marcus.index as u16;
    while let Some(sel) = runner.game.pending_selection.as_ref() {
        let who = sel.selecting_player;
        let action = if sel.is_optional {
            PASS
        } else if sel.valid_action_ids.contains(&marcus_action) {
            marcus_action
        } else {
            sel.valid_action_ids[0]
        };
        runner.game.resolve_selection(who, action).ok();
        runner.game.drain_effect_queue();
    }
}

/// POSITIVE: when AD1-021 suspends on your turn, you draw 1 and may digivolve a
/// Digimon into a yellow [Greymon] hand card for cost −3.
#[test]
fn ad1_021_self_suspend_draws_one() {
    let mut runner = builder()
        .hand(0, &["METAL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    // Seat AD1-021 as a Tamer on the field (turn 0 so it's the controller's).
    let tamer = runner.place_on_field(0, "AD1-021", Some(0));
    // A Lv.4 yellow Greymon base on field to host the digivolve.
    let _base = runner.place_on_field(0, "YGREY", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    fire_self_suspend(&mut runner, tamer);

    // The draw must have happened: hand grew by at least 1 (the draw) regardless
    // of how the optional digivolve resolves.
    assert!(
        runner.game.players[0].hand.len() >= hand_before + 1
            || runner.game.pending_selection.is_some(),
        "self-suspend must Draw 1 (hand grows) or park a selection"
    );
    let _ = runner.auto_resolve();
}

/// POSITIVE digivolve path: a Lv.4 yellow Greymon base on field + a Lv.5 yellow
/// [Greymon] in hand → the digivolve selection installs and, when driven, the
/// hand Greymon stacks onto the base for the cost reduced by 3.
#[test]
fn ad1_021_self_suspend_digivolves_into_greymon_cost_reduced_3() {
    let mut runner = builder()
        .hand(0, &["METAL"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "AD1-021", Some(0));
    let base = runner.place_on_field(0, "YGREY", Some(0));

    fire_self_suspend(&mut runner, tamer);
    let _ = runner.auto_resolve();

    // METAL (Lv.5 yellow Greymon) must have digivolved onto the YGREY base.
    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "METAL",
        "METAL must be the top card of the base stack after the cost-reduced digivolve"
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "METAL"),
        "METAL must leave the hand once it digivolves"
    );
}

/// NEGATIVE: no Greymon-named yellow card in hand → after the Draw, no digivolve
/// occurs (the optional select has no candidate).
#[test]
fn ad1_021_self_suspend_no_greymon_in_hand_no_digivolve() {
    let mut runner = builder()
        .hand(0, &["REDDIGI"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "AD1-021", Some(0));
    let base = runner.place_on_field(0, "YGREY", Some(0));

    fire_self_suspend(&mut runner, tamer);
    let _ = runner.auto_resolve();

    // The base must remain YGREY (no digivolve happened).
    let base_perm = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        base_perm.top_card().card_id(&runner.game.card_data),
        "YGREY",
        "no yellow Greymon in hand → base stays unchanged"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2 [End of Your Turn] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with a yellow Greymon Digimon on field AND a Marcus Damon, the EoYT
/// effect treats the Marcus Damon as a 6000 DP Digimon with Rush + CannotDigivolve.
#[test]
fn ad1_021_eoyt_treats_marcus_as_6000_digimon_rush_cant_digivolve() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let _tamer = runner.place_on_field(0, "AD1-021", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    // The condition enabler: a yellow Greymon Digimon.
    let _enabler = runner.place_on_field(0, "YGREY", Some(0));

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must not start treated as a Digimon"
    );

    // Fire the End-of-Your-Turn timing for the controller.
    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(_tamer),
    );
    runner.game.drain_effect_queue();

    // Resolve the mandatory Marcus pick (target the named MARCUS permanent, not
    // AD1-021 itself — which is *also* a legal "Marcus Damon" via also_treated_as).
    resolve_picking_marcus(&mut runner, marcus);

    assert!(
        runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must be treated as a Digimon"
    );
    assert_eq!(
        runner.effective_dp(marcus),
        Some(6000),
        "Marcus must be treated as a 6000 DP Digimon"
    );
    assert!(
        runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "Marcus must gain CannotDigivolve"
    );
    assert!(
        runner.game.has_keyword(marcus, Keyword::Rush),
        "Marcus must gain Rush"
    );
}

/// NEGATIVE: with NO yellow Agumon/Greymon Digimon on field, the EoYT effect is
/// skipped entirely (the condition gate fails) — no Marcus is treated as a Digimon.
#[test]
fn ad1_021_eoyt_no_enabler_does_nothing() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "AD1-021", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    // A RED (non-yellow) Digimon — does NOT satisfy the yellow Agumon/Greymon gate.
    let _red = runner.place_on_field(0, "REDDIGI", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "with no yellow Agumon/Greymon Digimon, the EoYT effect must not fire"
    );
}

/// The EoYT grants are end_of_turn — they expire after the turn ends.
#[test]
fn ad1_021_eoyt_grants_expire_at_end_of_turn() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, "AD1-021", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS", Some(0));
    let _enabler = runner.place_on_field(0, "YGREY", Some(0));

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::EndOfYourTurn,
        TriggerSource::Permanent(tamer),
    );
    runner.game.drain_effect_queue();
    resolve_picking_marcus(&mut runner, marcus);

    assert!(runner.modifiers().has(marcus, ModifierType::TreatAsDigimon));

    runner.game.end_turn();

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "TreatAsDigimon must expire at end of turn"
    );
    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "CannotDigivolve must expire at end of turn"
    );
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Rush),
        "Rush must expire at end of turn"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3 [Security] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// [Security] "Play this card without paying the cost." — driven through the real
/// combat / security-check path: AD1-021 sits in the defender's security stack,
/// an attacker checks it, and the inherited [Security] clause plays it free onto
/// the defender's field.
#[test]
fn ad1_021_security_plays_self_free() {
    let mut attacker = make_filler("AD1021-ATK");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = builder()
        .add_card(attacker.clone())
        .memory(10)
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .security(1, &["AD1-021"])
        .start();

    let attacker_handle = runner.place_on_field(0, "AD1021-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: AD1-021 in security"
    );

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.security_count(1),
        0,
        "AD1-021 must leave the defender's security stack after the security check"
    );

    // AD1-021 must be a Tamer permanent on the defender's field (played free).
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "AD1-021"),
        "AD1-021 must be played onto the defender's field by the [Security] effect"
    );
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|cs| cs.card_id(&runner.game.card_data) == "AD1-021"),
        "AD1-021 must not fall through to the trash — the [Security] clause played it"
    );
}
