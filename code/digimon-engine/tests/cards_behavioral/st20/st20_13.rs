//! ST20-13 Tai Kamiya & Izzy Izumi — Tamer, Black/Green, Cost 3, traits: [ADVENTURE].
//!
//! # Card text (official Bandai DB, `data/card_bundles/ST20-13.md`)
//!
//! **[Your Turn]** When any Digimon cards with the [ADVENTURE] trait would be
//! played from your hand, by suspending this Tamer, reduce the play cost by 1.
//!
//! **[Opponent's Turn]** All of your Digimon with the [ADVENTURE] trait gain
//! `<Blocker>` (This Digimon can block in the blocker timing.).
//!
//! **Security Effect:** [Security] Play this card without paying the cost.
//!
//! Official Q&A: with two such Tamers the play cost is reduced by a total of 2
//! (see `st20_12.rs::st20_12_and_st20_13_reductions_stack_for_a_total_of_2`).
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_13.cs`
//! - `EffectTiming.BeforePayCost` ActivateClass (optional): PlayCardCondition =
//!   card in HAND && IsDigimon && HasAdventureTraits && Owner == card.Owner;
//!   activation needs owner's turn + on battle area + `CanActivateSuspendCostEffect`;
//!   suspends this Tamer then installs a -1 `ChangeCostClass`.
//! - `EffectTiming.None` `BlockerStaticEffect` (not inherited): PermanentCondition =
//!   owner's battle-area Digimon whose TopCard HasAdventureTraits, gated on
//!   `IsExistOnBattleArea(card) && IsOpponentTurn(card)`.
//! - `EffectTiming.SecuritySkill`: `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md section 4.3)
//! - [Your Turn] `kind: cost_reduction` with `pay_cost: [suspend self]` (sibling
//!   ST21-13), positive / negative per condition, decline, event-log cost proof.
//! - [Opponent's Turn] filtered `kind: aura` granting `<Blocker>`; integrated
//!   through a real attack so the Block window actually offers the ADVENTURE
//!   Digimon (and only it).
//! - `[Security] play self free` driven through the real security-check path.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledPlayerRef,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::events::GameEvent;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST20-13";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, name: &str, level: u8, cost: u16, adventure: bool) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Green];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn adventure_tamer(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST20-13 in embedded DSL pack")
        .add_card(filler("PAD"))
        .add_card(digimon("ADV-DIGI", "Adv Digimon", 4, 5, true))
        .add_card(digimon("ADV-OWN", "Adv Own", 4, 4, true))
        .add_card(digimon("PLAIN-DIGI", "Plain Digimon", 4, 5, false))
        .add_card(digimon("OPP-ATK", "Opp Attacker", 4, 4, false))
        .add_card(digimon("OPP-ADV", "Opp Adventure", 4, 4, true))
        .add_card(adventure_tamer("ADV-TAMER", 3))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

/// Walk pending selections accepting the first non-PASS action until settled.
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

fn is_suspended(runner: &DebugRunner, player: u8, index: u8) -> bool {
    runner.game.players[player as usize].battle_area[index as usize].is_suspended
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_13_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Tai Kamiya & Izzy Izumi");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "ST20-13 prints Cost 3");
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST20-13 carries the ADVENTURE trait, got {:?}",
        card.traits
    );
    assert!(card.color.contains(&CompiledColor::Black));
    assert!(card.color.contains(&CompiledColor::Green));
    assert_eq!(card.color.len(), 2, "Black/Green only");
}

#[test]
fn st20_13_has_cost_reduction_aura_and_security_clauses() {
    let card = compiled(CARD_ID);
    let cost_reductions = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    let auras = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })
            )
        })
        .count();
    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity] => Some(t),
            _ => None,
        })
        .count();
    assert_eq!(cost_reductions, 1, "exactly one cost_reduction clause");
    assert_eq!(auras, 1, "exactly one aura clause (Blocker)");
    assert_eq!(security, 1, "exactly one on_security clause");
    assert_eq!(
        card.effects.len(),
        3,
        "no extra clauses beyond the printed three"
    );
}

#[test]
fn st20_13_cost_reduction_is_optional_your_turn_amount_1_with_a_suspend_cost() {
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
        .expect("cost_reduction clause");
    let (scope, active_when, optional, once_per_turn, amount, when_any, pay_cost) = cr;
    assert_eq!(scope, CompiledScope::FaceUp);
    assert!(
        optional,
        "'by suspending this Tamer' is an opt-in cost (DCGO isOptional)"
    );
    assert!(
        !once_per_turn,
        "no printed [Once Per Turn]; the suspend self-limits"
    );
    assert_eq!(amount, Some(1));
    assert!(
        when_any.is_some(),
        "ADVENTURE Digimon play predicate present"
    );
    assert!(active_when.is_some_and(|p| p.your_turn == Some(true)));
    assert!(!pay_cost.is_empty(), "suspend-self pay_cost present");
}

#[test]
fn st20_13_aura_is_opponents_turn_blocker_for_own_adventure_digimon() {
    let card = compiled(CARD_ID);
    let (active_when, target, grant) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                active_when,
                target,
                grant_keyword,
                ..
            }) => Some((active_when.clone(), target.clone(), grant_keyword.clone())),
            _ => None,
        })
        .expect("aura clause");
    assert!(
        active_when.is_some_and(|p| p.opponents_turn == Some(true)),
        "aura active only on the opponent's turn"
    );
    assert_eq!(target.owner, Some(CompiledPlayerRef::You));
    assert_eq!(target.kind, Some(CompiledCardKind::Digimon));
    assert!(target
        .trait_has
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("ADVENTURE")));
    assert!(grant
        .expect("grant_keyword")
        .keyword
        .eq_ignore_ascii_case("Blocker"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [Your Turn] by suspending this Tamer, reduce the play cost by 1
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: ADVENTURE Digimon (cost 5) from hand, accept the optional
/// reduction: cost paid is 4 and the Tamer is suspended.
#[test]
fn st20_13_cost_reduction_fires_on_adventure_digimon_play() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !is_suspended(&runner, 0, tamer.index),
        "Tamer starts unsuspended"
    );

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "an optional cost-reduction prompt must install"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 4, "5 - 1 = 4");
    assert!(
        is_suspended(&runner, 0, tamer.index),
        "the 'by suspending this Tamer' cost was paid"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-DIGI"),
        "the Digimon was played"
    );
}

/// Event-log proof of the cost: the Play event records printed 5 / paid 4.
#[test]
fn st20_13_cost_reduction_event_log_records_reduced_cost() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));

    let cp = runner.event_checkpoint();
    let _ = runner.play(0, 0);
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    let play = runner
        .events_since(cp)
        .iter()
        .find_map(|e| match e {
            GameEvent::Play {
                card_id,
                cost_paid,
                cost_printed,
                ..
            } if card_id == "ADV-DIGI" => Some((*cost_paid, *cost_printed)),
            _ => None,
        })
        .expect("a Play event for ADV-DIGI");
    assert_eq!(play, (4, 5), "Play event: paid 4 of printed 5");
}

/// Negative: a Digimon without the ADVENTURE trait is not reduced.
#[test]
fn st20_13_cost_reduction_does_not_fire_on_non_adventure_digimon() {
    let mut runner = builder().hand(0, &["PLAIN-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "plain Digimon plays with no reduction prompt"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full printed cost");
    assert!(
        !is_suspended(&runner, 0, tamer.index),
        "Tamer not suspended"
    );
}

/// Negative: the printed text says "Digimon cards" — an ADVENTURE Tamer card is
/// not reduced.
#[test]
fn st20_13_cost_reduction_does_not_fire_on_adventure_tamer_card() {
    let mut runner = builder().hand(0, &["ADV-TAMER"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(played.is_some(), "Tamer plays with no reduction prompt");
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 3, "full printed cost 3");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

/// Negative: [Your Turn] — on the opponent's turn the reduction is off.
#[test]
fn st20_13_cost_reduction_does_not_fire_on_opponents_turn() {
    let mut runner = builder().hand(1, &["ADV-DIGI"]).memory(0).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: player 1's turn");

    let mem_before = runner.memory();
    let played = runner.play(1, 0);
    assert!(played.is_some(), "player 1's play has no reduction prompt");
    drive_accept(&mut runner, 1);
    let _ = runner.auto_resolve();

    assert_eq!((runner.memory() - mem_before).abs(), 5, "full cost 5");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

/// Negative: the Tamer is already suspended, so the suspend cost cannot be
/// paid and no reduction is available.
#[test]
fn st20_13_cost_reduction_unavailable_while_tamer_is_suspended() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "no reduction prompt when the cost is unpayable"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full cost 5");
}

/// Decline: PASS the optional prompt, pay full cost, Tamer stays unsuspended.
#[test]
fn st20_13_cost_reduction_can_be_declined() {
    let mut runner = builder().hand(0, &["ADV-DIGI"]).memory(10).start();
    runner.skip_mulligan();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    assert!(runner.game.pending_selection.is_some(), "prompt installs");
    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids[0]
        };
        runner.execute_action(0, action).expect("decline");
    }
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "declined: full cost");
    assert!(!is_suspended(&runner, 0, tamer.index));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [Opponent's Turn] your ADVENTURE Digimon gain <Blocker>
// ═══════════════════════════════════════════════════════════════════════════════

/// Own turn: no Blocker (the aura is "[Opponent's Turn]").
#[test]
fn st20_13_no_blocker_on_own_turn() {
    let mut runner = builder().memory(3).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let adv = runner.place_on_field(0, "ADV-OWN", Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(adv, Keyword::Blocker),
        "no Blocker on the Tamer controller's own turn"
    );
}

/// Opponent's turn: own ADVENTURE Digimon have Blocker; a non-ADVENTURE Digimon
/// does not; the opponent's own ADVENTURE Digimon does not.
#[test]
fn st20_13_adventure_digimon_gain_blocker_on_opponents_turn() {
    let mut runner = builder().memory(0).start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let adv = runner.place_on_field(0, "ADV-OWN", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    let opp_adv = runner.place_on_field(1, "OPP-ADV", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: opponent's turn");
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(adv, Keyword::Blocker),
        "own ADVENTURE Digimon gains Blocker on the opponent's turn"
    );
    assert!(
        !runner.game.has_keyword(plain, Keyword::Blocker),
        "non-ADVENTURE Digimon does not"
    );
    assert!(
        !runner.game.has_keyword(opp_adv, Keyword::Blocker),
        "the opponent's ADVENTURE Digimon is not 'your' Digimon"
    );
}

/// The Blocker clears when the Tamer leaves the battle area.
#[test]
fn st20_13_blocker_clears_when_tamer_leaves() {
    let mut runner = builder().memory(0).start();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-OWN", Some(0));
    runner.end_turn();
    runner.game.tick_declarative_effects();
    let adv_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-OWN")
        .unwrap();
    let adv = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: adv_idx as u8,
    };
    assert!(
        runner.game.has_keyword(adv, Keyword::Blocker),
        "precondition"
    );

    runner.game.players[0]
        .battle_area
        .remove(tamer.index as usize);
    runner.game.tick_declarative_effects();
    let adv_idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-OWN")
        .unwrap();
    let adv = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: adv_idx as u8,
    };
    assert!(
        !runner.game.has_keyword(adv, Keyword::Blocker),
        "Blocker must clear once the source Tamer is gone"
    );
}

/// Integrated: the opponent attacks; the Block window offers exactly the
/// ADVENTURE Digimon (not the plain one).
#[test]
fn st20_13_block_window_offers_the_adventure_digimon() {
    let mut runner = builder()
        .security(0, &["PAD"; 3])
        .security(1, &["PAD"; 3])
        .memory(0)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let adv = runner.place_on_field(0, "ADV-OWN", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-DIGI", Some(0));
    let attacker = runner.place_on_field(1, "OPP-ATK", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    runner.game.tick_declarative_effects();

    let result = runner.attack_player(attacker, 0, false);
    assert_eq!(result, AttackResult::InProgress, "block window opens");
    assert_eq!(runner.current_phase(), GamePhase::BlockTiming);
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("block selection installed");
    assert_eq!(sel.selecting_player, 0, "the defender declares blockers");
    assert!(sel.is_optional, "blocking is a may");
    assert!(
        sel.valid_action_ids
            .contains(&encode_attack(0, adv.index as u16)),
        "the ADVENTURE Digimon can block"
    );
    assert!(
        !sel.valid_action_ids
            .contains(&encode_attack(0, plain.index as u16)),
        "the non-ADVENTURE Digimon cannot block"
    );
}

/// Negative control: without the Tamer there is no Block window at all.
#[test]
fn st20_13_no_block_window_without_the_tamer() {
    let mut runner = builder().security(0, &["PAD"; 3]).memory(0).start();
    runner.place_on_field(0, "ADV-OWN", Some(0));
    let attacker = runner.place_on_field(1, "OPP-ATK", Some(0));
    runner.end_turn();
    runner.game.tick_declarative_effects();

    let _ = runner.attack_player(attacker, 0, false);
    assert_ne!(
        runner.current_phase(),
        GamePhase::BlockTiming,
        "an ADVENTURE Digimon alone has no <Blocker>"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] Play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st20_13_security_plays_self_free() {
    let mut attacker = filler("SEC-ATK");
    attacker.level = Some(4);
    attacker.dp = Some(6000);

    let mut runner = builder()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(10)
        .start();
    runner.skip_mulligan();
    let atk = runner.place_on_field(0, "SEC-ATK", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: Tamer in security"
    );
    let mem_before = runner.memory();

    let _ = runner.attack_player(atk, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "Tamer left the security stack");
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "ST20-13 is played onto the defender's field"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "played without paying the cost"
    );
}
