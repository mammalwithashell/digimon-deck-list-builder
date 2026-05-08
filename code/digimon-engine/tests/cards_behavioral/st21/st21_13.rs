//! ST21-13 Matt Ishida & T.K. Takaishi — Tamer, Purple+Yellow, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (cards.json)
//!
//! **[Your Turn]** When any Digimon cards with the [ADVENTURE] trait would
//! be played from your hand, by suspending this Tamer, reduce the play cost
//! by 1.
//!
//! **[Your Turn]** All of your level 5 or higher Digimon with the [ADVENTURE]
//! trait gain `<Rush>` (This Digimon can attack the turn it comes into play.).
//!
//! **Inherited (Security):** [Security] Play this card without paying the
//! cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Multi/ST21_13.cs`
//!
//! # Patterns this test covers
//!
//! - **[Your Turn] cost reduction with `pay_cost: [suspend self]`** — Clause 1.
//!   Mirrors EX8-074 (cost-reduction + pay_cost) and BT22-094 (cost-reduction
//!   + `optional` + `when_any_ally_played` + ally-trait predicate +
//!   `pay_cost: [return_to_deck]` on `target: source`). ST21-13 differs only
//!   in the cost step (`suspend: { target: source }` instead of return-to-
//!   deck) and the ally predicate (`{kind: digimon, trait_has: ADVENTURE}`
//!   instead of `{any_of:[kind:digimon, kind:tamer], trait_has: CS}`).
//! - **[Your Turn] filtered aura with `grant_keyword: { keyword: Rush }`** —
//!   Clause 2. Mirrors BT22-084 / BT5-093 filtered aura shape (`target.owner:
//!   you`), exercising the G-DECLARATIVE-KEYWORD-resolved (2026-05-02)
//!   keyword-grant aura runtime via `Game::tick_declarative_effects` (Group 6
//!   in `tests/dsl/group6_auras.rs`).
//! - **[Security] play self free** — Clause 3 (structural; behavioral
//!   replay coverage lives in BT21-015 / BT9-092 / BT22-094 sister tests for
//!   the `play_from_security` substrate).
//!
//! # Faithfulness audit (per clause)
//!
//! 1. **[Your Turn] -1 cost on ally ADVENTURE Digimon play, by suspending
//!    self** — `kind: cost_reduction` declarative with `reduction_timing:
//!    before_pay_cost`, `active_when: { your_turn: true }` (mirrors DCGO
//!    `IsOwnerTurn(card)`), `optional: true` (mirrors DCGO `isOptional:
//!    true`; printed "by suspending ..." marks the cost as opt-in),
//!    `condition: any_permanent self-on-field & is_unsuspended` (mirrors
//!    DCGO `IsExistOnBattleArea(card) && CanActivateSuspendCostEffect(card)`),
//!    `when_any_ally_played: { all_of: [kind: digimon, trait_has:
//!    ADVENTURE] }` (mirrors DCGO `IsDigimon && HasAdventureTraits && Owner
//!    == card.Owner`), `amount: 1`, and `pay_cost: [suspend { target:
//!    source }]`.
//!
//!    Owner-scoping on `when_any_ally_played` is implicit — the
//!    `cost_target_card` predicate-subject is set during the controller's
//!    own pay-cost flow (see `lower_cost_reduction.rs`).
//!
//!    Once-per-turn is NOT marked: the `is_unsuspended: true` guard self-
//!    limits re-entry — after the first fire the tamer is suspended and the
//!    `condition` gate fails on subsequent BeforePayCost scans this turn.
//!    Matches DCGO behavior (no `maxCount` on the outer ActivateClass —
//!    the suspend cost is the natural limiter).
//!
//! 2. **[Your Turn] aura — own Lv5+ ADVENTURE Digimon gain Rush** — `kind:
//!    aura` declarative with `active_when: { your_turn: true }` (mirrors
//!    DCGO `RushStaticEffect.condition: IsOwnerTurn(card)`; on-battle-area
//!    is implicit per `Game::tick_declarative_effects`), `target: { owner:
//!    you, kind: digimon, level_gte: 5, trait_has: ADVENTURE }` (mirrors
//!    DCGO `IsPermanentExistsOnOwnerBattleAreaDigimon && permanent.Level >=
//!    5 && permanent.TopCard.HasAdventureTraits`), and `grant_keyword:
//!    { keyword: Rush }` (DCGO `RushStaticEffect`).
//!
//!    The G-DECLARATIVE-KEYWORD gap (RESOLVED 2026-05-02 by Group 6) means
//!    `tick_declarative_effects` materialises the Rush keyword on each
//!    matching permanent every refresh and clears it when the source leaves
//!    or the active_when gate flips false.
//!
//! 3. **[Security] play this card without paying the cost** — `when:
//!    on_security` + `process: [play_from_security: {}]`. Standard pattern
//!    (BT17-093, BT18-087, BT22-084, BT21-015, BT9-092, EX11-054, BT22-094).
//!    Mandatory (no [Security] effect is opt-out per RULES_CONTEXT.md §16).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-13";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A Lv-N Digimon with the ADVENTURE trait. Used as ally play targets and
/// as aura targets.
fn make_adventure_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// A Lv-N Digimon WITHOUT the ADVENTURE trait — for negative branch tests
/// of the cost-reduction `when_any_ally_played` predicate AND the aura
/// `target.trait_has` predicate.
fn make_non_adventure_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec![]; // no ADVENTURE
    c
}

/// A Tamer with the ADVENTURE trait — for negative branch test of the
/// cost-reduction `kind: digimon` filter (Tamer must NOT trigger the
/// reduction even with ADVENTURE trait).
fn make_adventure_tamer(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c.colors = vec![CardColor::Yellow];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// Helper: walk pending selections, accepting the first non-PASS legal
/// action, until the engine settles. Mirrors the bt22_094 helper.
fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST21-13 must compile from the embedded DSL pack as a Tamer with cost 3
/// and the ADVENTURE trait.
#[test]
fn st21_13_compiles_as_tamer_cost_3_with_adventure_trait() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "ST21-13 prints Cost 3");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST21-13 must carry the ADVENTURE trait (printed type_eng = ADVENTURE)"
    );
}

/// Three clauses total: 1 triggered (on_security) + 2 declaratives
/// (cost_reduction, aura).
#[test]
fn st21_13_has_one_triggered_and_two_declaratives() {
    let card = compiled(CARD_ID);

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        triggered.len(),
        1,
        "ST21-13 must have exactly 1 triggered clause (on_security)"
    );

    let cr_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(
        cr_count, 1,
        "ST21-13 must have exactly 1 cost_reduction declarative"
    );

    let aura_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    assert_eq!(
        aura_count, 1,
        "ST21-13 must have exactly 1 aura declarative (Rush grant)"
    );
}

/// Clause 1 (cost_reduction): optional + active_when:your_turn + amount=1 +
/// when_any_ally_played present + non-empty pay_cost (the suspend cost).
#[test]
fn st21_13_clause1_cost_reduction_optional_your_turn_amount_1() {
    let card = compiled(CARD_ID);
    let cr = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                active_when,
                optional,
                once_per_turn,
                amount,
                when_any_ally_played,
                pay_cost,
                ..
            }) => Some((
                *scope,
                active_when.clone(),
                *optional,
                *once_per_turn,
                *amount,
                when_any_ally_played.clone(),
                pay_cost.clone(),
            )),
            _ => None,
        })
        .expect("ST21-13 must have a cost_reduction clause");

    let (scope, active_when, optional, once_per_turn, amount, when_any_ally, pay_cost) = cr;

    assert_eq!(
        scope,
        CompiledScope::FaceUp,
        "cost_reduction is FaceUp scope"
    );
    assert!(
        optional,
        "cost_reduction must be optional — DCGO isOptional:true; printed \
         'by suspending ...' is opt-in"
    );
    assert!(
        !once_per_turn,
        "cost_reduction is NOT marked OPT — printed text has no [Once Per \
         Turn]; the suspend-self cost self-limits re-entry via the \
         is_unsuspended condition guard"
    );
    assert_eq!(amount, Some(1), "amount must be 1 (printed -1 cost)");
    assert!(
        when_any_ally.is_some(),
        "when_any_ally_played predicate must be present (kind: digimon + \
         trait_has: ADVENTURE)"
    );
    assert!(
        active_when.is_some_and(|p| p.your_turn == Some(true)),
        "active_when must gate to your_turn=true"
    );
    assert!(
        !pay_cost.is_empty(),
        "pay_cost must be non-empty (suspend self)"
    );
}

/// Clause 2 (aura): FaceUp scope, active_when:your_turn, target.owner=you,
/// target.kind=digimon, target.level_gte=5, target.trait_has=ADVENTURE,
/// grant_keyword.keyword=Rush.
#[test]
fn st21_13_clause2_aura_grants_rush_to_own_lv5_adventure_digimon() {
    use digimon_dsl::compiled::CompiledPlayerRef;

    let card = compiled(CARD_ID);
    let aura = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                target,
                grant_keyword,
                ..
            }) => Some((
                *scope,
                active_when.clone(),
                target.clone(),
                grant_keyword.clone(),
            )),
            _ => None,
        })
        .expect("ST21-13 must have an aura clause");

    let (scope, active_when, target, grant_keyword) = aura;

    assert_eq!(scope, CompiledScope::FaceUp, "aura is FaceUp scope");
    assert!(
        active_when.is_some_and(|p| p.your_turn == Some(true)),
        "aura active_when must gate to your_turn=true"
    );

    // target predicate shape
    assert_eq!(
        target.owner,
        Some(CompiledPlayerRef::You),
        "aura target.owner must be You (printed 'All of YOUR Lv5+ ...')"
    );
    assert_eq!(
        target.kind,
        Some(CompiledCardKind::Digimon),
        "aura target.kind must be Digimon (printed 'all of your ... Digimon')"
    );
    assert_eq!(
        target.level_gte,
        Some(5),
        "aura target.level_gte must be 5 (printed 'level 5 or higher')"
    );
    assert!(
        target
            .trait_has
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "aura target.trait_has must be ADVENTURE (printed '[ADVENTURE] trait'); \
         got {:?}",
        target.trait_has
    );

    // grant_keyword shape
    let gk = grant_keyword.expect("aura must declare grant_keyword");
    assert!(
        gk.keyword.eq_ignore_ascii_case("Rush"),
        "aura must grant Rush; got {:?}",
        gk.keyword
    );
}

/// Clause 3 (on_security): triggered, FaceUp, mandatory.
#[test]
fn st21_13_clause3_on_security_face_up_mandatory() {
    let card = compiled(CARD_ID);
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("ST21-13 must have an on_security clause");

    assert_eq!(security.scope, CompiledScope::FaceUp);
    assert!(
        !security.optional,
        "Security clause must be mandatory ([Security] effects are not \
         opt-out per RULES_CONTEXT.md §16; DCGO PlaySelfTamerSecurityEffect \
         has no canNoSelect parameter)"
    );
    assert!(!security.once_per_turn, "Security clause has no OPT");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Behavioral: Clause 1 [Your Turn] -1 cost on ally ADVENTURE
//                          Digimon play, by suspending self
// ═══════════════════════════════════════════════════════════════════════════════
//
// Strategy: place ST21-13 on P0's field, put an ADVENTURE Digimon in P0's
// hand, and `play(0, hand_index)`. The engine should detect the cost-
// reduction declarative effect during BeforePayCost scan; if the player
// accepts the optional reduction (HAND_EFFECT_START prompt), the pay_cost
// step (suspend ST21-13) runs and the ally's effective cost is reduced by 1.
//
// Memory deduction is the cleanest assertion: with cost-5 ally + cost-1
// reduction, memory should drop by 4 instead of 5. We verify alongside the
// invariant that ST21-13 becomes suspended after pay_cost.

/// Positive: ST21-13 on field, ADVENTURE Digimon (cost 5) played from hand
/// → cost reduction fires, ST21-13 becomes suspended, ally cost is 5-1=4.
///
/// Note on flow: when an `optional: true` cost_reduction matches, the engine
/// returns `PlayFromHandCostResult::Pending` (→ `play_from_hand` returns
/// `None`) and installs a `pending_selection` of kind EffectChoice with
/// `is_optional: true`. Accepting (HAND_EFFECT_START) chains into the
/// pay_cost steps; PASS-ing declines.
#[test]
fn st21_13_cost_reduction_fires_on_ally_adventure_digimon_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();

    // Place ST21-13 on field (bypass on_play). Cost reduction is registered
    // as a BeforePayCost effect on the carrier permanent.
    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));

    // Sanity: ST21-13 is on field and unsuspended.
    let matt_present_before = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID);
    assert!(
        matt_present_before,
        "ST21-13 must be on P0's field before the play"
    );
    assert!(
        !runner.game.players[0].battle_area[matt_handle.index as usize].is_suspended,
        "ST21-13 must start unsuspended"
    );

    let mem_before = runner.memory();

    // Play ADV-DIGI from P0's hand. With an optional cost-reduction in
    // scope, play_from_hand returns Pending → None; we accept the prompt
    // next.
    let _ = runner.play(0, 0);

    // A pending selection prompt for the optional cost reduction must
    // install.
    assert!(
        runner.game.pending_selection.is_some(),
        "an optional cost-reduction prompt must install for the ADV-DIGI play"
    );

    // Accept and resolve the suspend cost.
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let mem_after = runner.memory();
    let cost_paid = mem_before - mem_after;
    assert_eq!(
        cost_paid, 4,
        "ADV-DIGI's effective cost must be 5 - 1 = 4 with ST21-13's reduction; got {cost_paid}"
    );

    // ST21-13 must now be suspended (the cost was paid).
    let matt_after = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST21-13 must remain on field after pay_cost (suspend, not delete)");
    assert!(
        matt_after.is_suspended,
        "ST21-13 must be suspended after pay_cost (suspend-self cost paid)"
    );

    // The ally must be on field (it was successfully played at the
    // reduced cost).
    let ally_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-DIGI");
    assert!(
        ally_on_field,
        "ADV-DIGI must be on field after the reduced-cost play"
    );
}

/// Negative: ally Digimon WITHOUT ADVENTURE trait → cost reduction must
/// NOT fire. Full printed cost is paid and ST21-13 stays unsuspended.
#[test]
fn st21_13_cost_reduction_does_not_fire_on_non_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_non_adventure_digimon(
            "NON-ADV",
            "NonAdv Digimon",
            4,
            5,
        ))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["NON-ADV"])
        .memory(10)
        .start();

    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    // For the no-fire case, the cost-reduction predicate (trait_has:
    // ADVENTURE) on when_any_ally_played rejects NON-ADV, so no Pending
    // prompt installs and play_from_hand returns Some(field_index) on
    // success.
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "NON-ADV Digimon play should succeed without any optional \
         cost-reduction prompt"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 5,
        "NON-ADV Digimon must pay full printed cost 5 — predicate \
         trait_has:ADVENTURE must reject it; got {cost_paid}"
    );

    // ST21-13 must still be unsuspended — pay_cost was never run.
    assert!(
        !runner.game.players[0].battle_area[matt_handle.index as usize].is_suspended,
        "ST21-13 must remain unsuspended — cost reduction did not fire"
    );
}

/// Negative: ally Tamer WITH ADVENTURE trait → cost reduction must NOT
/// fire (predicate filters by `kind: digimon`). The printed text says
/// "Digimon cards", so the ADVENTURE Tamer must NOT trigger the reduction
/// even though its trait matches.
#[test]
fn st21_13_cost_reduction_does_not_fire_on_adventure_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_tamer("ADV-TAMER", "Adv Tamer", 3))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-TAMER"])
        .memory(10)
        .start();

    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "ADV-TAMER play should succeed without any optional cost-reduction \
         prompt (Tamer is not Digimon)"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 3,
        "ADV-TAMER must pay full printed cost 3 — predicate kind:digimon \
         must reject Tamers; got {cost_paid}"
    );

    assert!(
        !runner.game.players[0].battle_area[matt_handle.index as usize].is_suspended,
        "ST21-13 must remain unsuspended — cost reduction did not fire \
         (Tamer ally rejected by kind: digimon predicate)"
    );
}

/// Negative: opponent's turn — `active_when: { your_turn: true }` gate
/// must suppress the cost reduction. Mirrors BT22-094's opp-turn negative
/// test pattern.
#[test]
fn st21_13_cost_reduction_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon(
            "OPP-ADV-DIGI",
            "Opp Adv Digimon",
            4,
            5,
        ))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(1, &["OPP-ADV-DIGI"])
        .memory(0) // 0 memory means P0 → end_turn flips to P1's turn
        .start();

    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));

    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let mem_before = runner.memory();
    let played = runner.play(1, 0);
    assert!(
        played.is_some(),
        "P1's ADV-DIGI play should succeed with no cost-reduction prompt"
    );
    drive_accept(&mut runner, 1);
    let _ = runner.auto_resolve();

    // Cost paid by P1 reflected in memory delta (absolute).
    let mem_swing = (runner.memory() - mem_before).abs();
    assert_eq!(
        mem_swing, 5,
        "P1's ADV-Digimon must pay full cost 5 — P0's ST21-13 cost \
         reduction must not fire on opponent's turn; mem swing was {mem_swing}"
    );

    assert!(
        !runner.game.players[0].battle_area[matt_handle.index as usize].is_suspended,
        "ST21-13 must remain unsuspended — cost reduction did not fire on \
         opponent's turn"
    );
}

/// Optional decline: cost reduction is `optional: true`. PASS the prompt →
/// pay full cost; ST21-13 stays unsuspended.
#[test]
fn st21_13_cost_reduction_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();

    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "cost-reduction prompt must install (we'll decline below)"
    );

    // Decline every optional prompt by passing.
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap_or(view.valid_action_ids[0])
        };
        runner
            .execute_action(0, action)
            .expect("execute pending selection (declining where optional)");
    }
    let _ = runner.auto_resolve();

    let cost_paid = mem_before - runner.memory();
    assert_eq!(
        cost_paid, 5,
        "with cost reduction declined, full 5-cost is paid; got {cost_paid}"
    );

    assert!(
        !runner.game.players[0].battle_area[matt_handle.index as usize].is_suspended,
        "ST21-13 must remain unsuspended when the player declines the \
         cost reduction"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral: Clause 2 [Your Turn] aura — own Lv5+ ADVENTURE
//                          Digimon gain Rush
// ═══════════════════════════════════════════════════════════════════════════════
//
// G-DECLARATIVE-KEYWORD was RESOLVED 2026-05-02 by Group 6 — filtered
// `kind: aura` with `grant_keyword` lowers to a process-backed declarative
// effect that calls `ctx.grant_declarative_keyword(handle, Keyword::Rush,
// Expiry::Permanent)` for each matching permanent during
// `Game::tick_declarative_effects`. The aura's `active_when` closure is
// evaluated each tick.
//
// Test pattern mirrors `tests/dsl/group6_auras.rs::
// declarative_grant_keyword_refresh_removes_stale_keyword_after_source_leaves`
// and `tests/cards_behavioral/bt22/bt22_084.rs` aura tests:
//   1. Place ST21-13 on P0's field.
//   2. Place an ally Digimon on P0's field (Lv5+ ADVENTURE for positive,
//      otherwise for negative).
//   3. Call `runner.game.tick_declarative_effects()` to materialise the
//      Rush modifier.
//   4. Assert `runner.game.has_keyword(handle, Keyword::Rush)` matches
//      the expected branch.

/// Positive: own Lv6 ADVENTURE Digimon gains Rush from ST21-13's aura.
#[test]
fn st21_13_aura_grants_rush_to_own_lv6_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-LV6", "Adv Lv6", 6, 8))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ADV-LV6", Some(0));

    // Sanity: the Lv6 Digimon does not have Rush printed natively.
    assert!(
        !runner.game.has_keyword(ally, Keyword::Rush),
        "precondition: ADV-LV6 must not have Rush before the aura ticks"
    );

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(ally, Keyword::Rush),
        "ST21-13's aura must grant Rush to own Lv6+ [ADVENTURE] Digimon"
    );
}

/// Positive: own Lv5 ADVENTURE Digimon also gains Rush (boundary case for
/// `level_gte: 5`).
#[test]
fn st21_13_aura_grants_rush_to_own_lv5_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-LV5", "Adv Lv5", 5, 6))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ADV-LV5", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(ally, Keyword::Rush),
        "ST21-13's aura must grant Rush at the level_gte: 5 boundary (Lv5)"
    );
}

/// Negative: own Lv4 ADVENTURE Digimon does NOT gain Rush (below the
/// `level_gte: 5` threshold).
#[test]
fn st21_13_aura_does_not_grant_rush_to_own_lv4_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-LV4", "Adv Lv4", 4, 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ADV-LV4", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(ally, Keyword::Rush),
        "ST21-13's aura must NOT grant Rush below Lv5 threshold"
    );
}

/// Negative: own Lv6 Digimon WITHOUT the ADVENTURE trait does NOT gain
/// Rush (predicate filters by `trait_has: ADVENTURE`).
#[test]
fn st21_13_aura_does_not_grant_rush_to_own_non_adventure_lv6_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_non_adventure_digimon(
            "NON-ADV-LV6",
            "NonAdv Lv6",
            6,
            8,
        ))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "NON-ADV-LV6", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(ally, Keyword::Rush),
        "ST21-13's aura must NOT grant Rush to non-ADVENTURE Digimon"
    );
}

/// Negative: opponent's Lv6 ADVENTURE Digimon does NOT gain Rush
/// (predicate's `target.owner: you` excludes opponent permanents).
#[test]
fn st21_13_aura_does_not_grant_rush_to_opponent_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("OPP-ADV-LV6", "Opp Adv Lv6", 6, 8))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-ADV-LV6", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(opp, Keyword::Rush),
        "ST21-13's aura must NOT grant Rush to opponent's Digimon \
         (target.owner: you)"
    );
}

/// Negative: on opponent's turn, the aura is gated off by `active_when:
/// { your_turn: true }`. Even own Lv5+ ADVENTURE Digimon must not have
/// the aura-granted Rush during the opponent's turn.
///
/// Per Group 6's refresh-on-tick semantics, the materialised Rush modifier
/// is cleared when `active_when` evaluates false on the next tick.
#[test]
fn st21_13_aura_does_not_grant_rush_during_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-LV6", "Adv Lv6", 6, 8))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0) // forces end_turn to flip to P1
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ADV-LV6", Some(0));

    // Tick once on P0's turn — aura should fire.
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(ally, Keyword::Rush),
        "precondition: aura must grant Rush on P0's own turn"
    );

    // Hand off to P1's turn and re-tick.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(ally, Keyword::Rush),
        "ST21-13's aura must be inactive on opponent's turn — \
         active_when: your_turn=true gates off, materialised Rush should \
         clear on refresh"
    );
}

/// Refresh: when ST21-13 leaves the field, the aura-granted Rush must
/// clear from previously-buffed allies.
///
/// Mirrors `tests/dsl/group6_auras.rs::
/// declarative_grant_keyword_refresh_removes_stale_keyword_after_source_leaves`.
#[test]
fn st21_13_aura_clears_rush_when_source_leaves_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-13 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-LV6", "Adv Lv6", 6, 8))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(3)
        .start();

    let matt_handle = runner.place_on_field(0, CARD_ID, Some(0));
    let ally = runner.place_on_field(0, "ADV-LV6", Some(0));

    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(ally, Keyword::Rush),
        "precondition: ally has Rush while source is on field"
    );

    // Remove ST21-13 from the field directly (simulates trash/return).
    runner.game.players[0]
        .battle_area
        .remove(matt_handle.index as usize);
    runner.game.tick_declarative_effects();

    // After the source leaves, the ally's index shifts to 0; recompute.
    let ally_after = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-LV6")
        .map(|i| digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("ally still on field after source removed");

    assert!(
        !runner.game.has_keyword(ally_after, Keyword::Rush),
        "process-backed declarative keyword grants must be cleared on \
         refresh when the source leaves"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Anti-helpers (silence dead-code warnings across the test fork)
// ═══════════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_adventure_tamer("X", "X", 0);
    let _ = make_filler("X");
}
