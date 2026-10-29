//! BT20-056 Alphamon
//!
//! Printed text (card image / cards.json — authoritative):
//!   <Barrier>
//!   [On Play][When Digivolving] <Recovery +1 (Deck)>. Then, if during an
//!     attack, 1 of your Digimon in the breeding area may digivolve into a
//!     level 6 or lower [Chronicle] trait Digimon card in the hand or trash
//!     without paying the cost.
//!   [All Turns][Once Per Turn] When security stacks are removed from, 1 of
//!     your opponent's Digimon gets -8000 DP for the turn.
//!   [Inherited][All Turns][Once Per Turn] When this Digimon would leave the
//!     battle area other than by your effects, if this Digimon is
//!     [Alphamon: Ouryuken], by trashing your top security card, it doesn't
//!     leave.
//!
//! DCGO reference (READ-ONLY): DCGO/Assets/Scripts/CardEffect/BT20/Black/BT20_056.cs

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{
    AttackState, AttackTarget, PendingAttack, SelectionKind, TriggerSource,
};

// ────────────────────────────── helpers ──────────────────────────────

fn make_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

fn security_ids(runner: &DebugRunner, player: usize) -> Vec<&str> {
    runner.game.players[player]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data))
        .collect()
}

/// Remove the top card of `player`'s security stack via an engine effect, which
/// is the trigger condition for the `on_own/opponent_security_removed`
/// observers. Uses a throwaway EffectContext sourced from the BT20-056
/// permanent (same pattern as BT24-101's security-loss test).
fn trash_top_security_via_effect(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
    owner: u8,
) {
    let source_card = runner.game.players[source.player as usize].battle_area
        [source.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, source_card, None, source.player);
    assert!(ctx.trash_top_security(owner));
}

// ───────────────────────────── slice loads ───────────────────────────

#[test]
fn bt20_056_loads_barrier_recovery_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 must load from embedded DSL pack")
        .start();
}

// ─────────────── [All Turns][OPT] security-removed -8000 DP ───────────────

#[test]
fn bt20_056_security_removed_debuffs_one_opponent_digimon_by_8000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OPP-A", "Opponent A", 6, 12000))
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    // Removing a card from *your own* security triggers the observer.
    trash_top_security_via_effect(&mut runner, alphamon, 0);

    let view = runner
        .pending_selection_view()
        .expect("security-removed observer should ask to pick an opponent Digimon");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().expect("finish -8000 DP debuff");

    assert_eq!(
        runner.dp_of(target),
        Some(4000),
        "the chosen opponent Digimon should be at 12000 - 8000 = 4000 DP"
    );
}

#[test]
fn bt20_056_security_removed_debuff_fires_once_across_both_stacks() {
    // The [All Turns][OPT] clause is keyed on BOTH own- and opponent-security
    // removal, sharing a single once-per-turn lockout. Removing from each stack
    // the same turn must produce exactly ONE -8000 debuff prompt.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OPP-A", "Opponent A", 6, 12000))
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .add_card(make_test_card("OPP-SEC-A", "Opp Security A"))
        .add_card(make_test_card("OPP-SEC-B", "Opp Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .security(1, &["OPP-SEC-A", "OPP-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));
    let target = runner.place_on_field(1, "OPP-A", Some(0));

    // First removal (own stack): observer fires, resolve the debuff.
    trash_top_security_via_effect(&mut runner, alphamon, 0);
    let view = runner
        .pending_selection_view()
        .expect("first security removal should fire the observer");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the opponent Digimon");
    runner.auto_resolve().expect("finish first debuff");
    assert_eq!(runner.dp_of(target), Some(4000));

    // Second removal the same turn (opponent stack) must NOT re-arm the OPT.
    trash_top_security_via_effect(&mut runner, alphamon, 1);
    assert!(
        runner.pending_selection_view().is_none(),
        "once-per-turn lockout is shared across both security-removed timings"
    );
    assert_eq!(
        runner.dp_of(target),
        Some(4000),
        "no second -8000 debuff should be applied the same turn"
    );
}

#[test]
fn bt20_056_security_removed_debuff_is_optional_when_no_opponent_digimon() {
    // With no opponent Digimon to target, the observer must not soft-lock; the
    // clause simply finds no legal target. (DCGO HasMatchConditionPermanent gate.)
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_test_card("MY-SEC-A", "My Security A"))
        .add_card(make_test_card("MY-SEC-B", "My Security B"))
        .security(0, &["MY-SEC-A", "MY-SEC-B"])
        .start();
    let alphamon = runner.place_on_field(0, "BT20-056", Some(0));

    trash_top_security_via_effect(&mut runner, alphamon, 0);
    // No opponent Digimon exists; resolving must not panic or strand a prompt.
    runner
        .auto_resolve()
        .expect("observer resolves with no target");
    assert_eq!(runner.security_count(0), 1);
}

// ───── [Inherited][All Turns][OPT] Ouryuken trash-top-security to stay ─────

#[test]
fn bt20_056_ouryuken_inherited_replacement_prevents_leave_for_top_security() {
    // BT20-056 is a digivolution source beneath a top card named
    // "Alphamon: Ouryuken"; an opponent-effect leave is prevented by trashing
    // the controller's top security card.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("inherited leave-prevention should ask to accept");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "replacement may be declined");
    runner
        .execute_action(0, REPLACEMENT_ACCEPT)
        .expect("accept Ouryuken replacement");
    runner.auto_resolve().expect("finish replacement");

    assert!(
        permanent_exists(&runner, 0, "OURYUKEN"),
        "Alphamon: Ouryuken stays after trashing top security"
    );
    assert_eq!(
        runner.security_count(0),
        1,
        "top security was trashed as the cost"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["SEC-A"],
        "the top (SEC-B) was trashed"
    );
}

#[test]
fn bt20_056_ouryuken_inherited_replacement_declined_lets_it_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    runner
        .pending_selection_view()
        .expect("inherited leave-prevention prompt");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline Ouryuken replacement");

    assert!(
        !permanent_exists(&runner, 0, "OURYUKEN"),
        "declining lets Alphamon: Ouryuken leave"
    );
    assert_eq!(
        security_ids(&runner, 0),
        vec!["SEC-A", "SEC-B"],
        "declining must not trash any security"
    );
}

#[test]
fn bt20_056_ouryuken_replacement_does_not_fire_for_non_ouryuken_top() {
    // If the top card is NOT named "Alphamon: Ouryuken", the inherited
    // replacement must not offer protection (the leave proceeds).
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("PLAIN-ALPHA", "Alphamon", 6, 11000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "PLAIN-ALPHA"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "no replacement prompt when the top card is not [Alphamon: Ouryuken]"
    );
    assert!(
        !permanent_exists(&runner, 0, "PLAIN-ALPHA"),
        "a non-Ouryuken top card is not protected and leaves"
    );
}

#[test]
fn bt20_056_ouryuken_replacement_does_not_fire_for_own_effect_leave() {
    // "other than by your effects": an own-effect leave must NOT be preventable.
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 YAML loads")
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    let carrier = runner.place_stack(0, &["BT20-056", "OURYUKEN"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect leave must not offer the Ouryuken protection"
    );
    assert!(
        !permanent_exists(&runner, 0, "OURYUKEN"),
        "own-effect leave proceeds"
    );
}

// Complete-card regression fixtures. Attack windows below isolate the printed
// conditional tail; these are simulator tests, not DCGO oracle scenarios.
// CR 4.3 3-4-7-4/6: breeding evolution does not trigger ordinary WD effects.

const ID: &str = "BT20-056";

fn colored(id: &str, level: u8, color: CardColor) -> CardData {
    let mut card = make_digimon(id, id, level, 12000);
    card.colors = vec![color];
    card
}

fn evolution(id: &str, level: u8, chronicle: bool, base: u8, color: CardColor) -> CardData {
    let mut card = colored(id, level, CardColor::Black);
    if chronicle {
        card.traits = vec!["Chronicle".into()];
    }
    card.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: base,
        memory_cost: 7,
    }];
    card
}

fn full_builder(owner: u8, hand: &[&str]) -> DebugRunnerBuilder {
    let mut tamer = evolution("TAMER", 6, true, 5, CardColor::Black);
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    DebugRunner::builder()
        .dsl_card(ID)
        .unwrap()
        .add_card(colored("FILL", 3, CardColor::Black))
        .add_card(colored("BASE-B", 5, CardColor::Black))
        .add_card(colored("BASE-Y", 5, CardColor::Yellow))
        .add_card(colored("BASE-P", 5, CardColor::Purple))
        .add_card(colored("MID", 4, CardColor::Black))
        .add_card(colored("OTHER", 6, CardColor::Blue))
        .add_card(evolution("EVO6", 6, true, 5, CardColor::Black))
        .add_card(evolution("EVO5", 5, true, 5, CardColor::Black))
        .add_card(evolution("EVO7", 7, true, 5, CardColor::Black))
        .add_card(evolution("NO-TRAIT", 6, false, 5, CardColor::Black))
        .add_card(evolution("WRONG-COLOR", 6, true, 5, CardColor::Purple))
        .add_card(evolution("WRONG-LEVEL", 6, true, 4, CardColor::Black))
        .add_card(tamer)
        .add_card(make_digimon("OURYUKEN", "Alphamon: Ouryuken", 7, 13000))
        .add_card(make_digimon(
            "NEAR-OURYUKEN",
            "Alphamon: Ouryuken (X Antibody)",
            7,
            13000,
        ))
        .hand(owner, hand)
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
}

fn full_ready(owner: u8, mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(owner);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn choices(r: &DebugRunner, owner: u8, count: usize, optional: bool) -> Vec<u16> {
    let p = r
        .pending_selection()
        .expect("printed choice must be exposed");
    assert_eq!(p.selecting_player, owner);
    assert_eq!(p.is_optional, optional, "{}", p.prompt);
    let ids: Vec<_> = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(ids.len(), count, "{}: {:?}", p.prompt, ids);
    for (action, actual) in build_action_mask(&r.game, owner).iter().enumerate() {
        let legal = ids.contains(&(action as u16)) || (optional && action == PASS as usize);
        assert_eq!(*actual, if legal { 1.0 } else { 0.0 }, "mask {action}");
    }
    assert!(build_action_mask(&r.game, 1 - owner)
        .iter()
        .all(|v| *v == 0.0));
    ids
}

fn choose(r: &mut DebugRunner, owner: u8, action: u16) {
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action).expect("legal choice");
}

fn zone_action(r: &DebugRunner, owner: u8, id: &str, trash: bool) -> u16 {
    let p = &r.game.players[owner as usize];
    let zone = if trash { &p.trash } else { &p.hand };
    let index = zone
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap();
    if r.game.pending_selection.as_ref().and_then(|p|p.card_choice.as_ref()).is_some() {
        return owned_union_fixture::action(&r.game, owner, id, trash).expect("printed eligible card has a legal physical row");
    }
    (if trash {
        space::TRASH_EFFECT_START
    } else {
        space::PLAY_HAND_START
    }) + index as u16
}

fn breeder_id(r: &DebugRunner, owner: u8) -> Option<&str> {
    r.game.players[owner as usize]
        .breeding_area
        .as_ref()
        .map(|p| p.top_card().card_id(&r.game.card_data))
}

fn fire_full(r: &mut DebugRunner, host: PermanentHandle, evo: bool) {
    if evo {
        r.game.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(host),
        );
        r.game.drain_effect_queue();
    } else {
        r.fire_on_play(host.player, host.index as usize);
    }
}

fn attack_window(r: &mut DebugRunner, attacker: PermanentHandle) {
    r.set_first_player(attacker.player);
    r.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: AttackTarget::Player(1 - attacker.player),
        effective_target: AttackTarget::Player(1 - attacker.player),
        effective_target_removed: false,
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        battle_defender_deleted: false,
        effect_battle_piercing_pending: false,
        security_check_started: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
}

#[test]
fn bt20_056_printed_routes_cost_three_and_recovery_occurs_on_evolution() {
    for (base, legal) in [
        ("BASE-B", true),
        ("BASE-Y", true),
        ("BASE-P", false),
        ("MID", false),
    ] {
        let mut r = full_ready(0, full_builder(0, &[ID]).start());
        let host = r.place_on_field(0, base, Some(0));
        let action = space::encode_digivolve(0, host.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            legal,
            "{base}"
        );
        if legal {
            r.game.decode_action(action, 0);
            assert_eq!(r.memory(), 7);
            assert_eq!(r.security_count(0), 6);
            assert_eq!(r.game.players[0].deck.len(), 28, "draw plus Recovery");
            assert!(r.game.has_keyword(host, Keyword::Barrier));
            assert_eq!(r.effective_dp(host), Some(11000));
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_056_recovery_is_mandatory_for_both_timings_and_controllers_without_attack() {
    for owner in [0, 1] {
        for evo in [false, true] {
            let mut r = full_ready(owner, full_builder(owner, &["EVO6"]).start());
            let host = r.place_on_field(owner, ID, Some(0));
            r.place_in_breeding(owner, "BASE-B");
            fire_full(&mut r, host, evo);
            assert_eq!(r.security_count(owner), 6);
            assert_eq!(r.game.players[owner as usize].deck.len(), 29);
            assert_eq!(r.hand_size(owner), 1);
            assert_eq!(breeder_id(&r, owner), Some("BASE-B"));
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_056_barrier_cost_may_be_accepted_or_declined_only_for_battle_deletion() {
    for owner in [0, 1] {
        for accept in [false, true] {
            let mut r = full_ready(owner, full_builder(owner, &[]).start());
            let host = r.place_on_field(owner, ID, Some(0));
            r.game
                .delete_permanent_with_cause(host, ReplacementCause::Battle);
            assert_eq!(
                r.pending_selection_view().unwrap().kind,
                SelectionKind::Replacement
            );
            assert_eq!(choices(&r, owner, 1, true), vec![REPLACEMENT_ACCEPT]);
            choose(
                &mut r,
                owner,
                if accept { REPLACEMENT_ACCEPT } else { PASS },
            );
            assert_eq!(permanent_exists(&r, owner as usize, ID), accept);
            assert_eq!(r.security_count(owner), if accept { 4 } else { 5 });
            assert!(r.pending_selection().is_none());
        }
    }
    let mut r = full_ready(0, full_builder(0, &[]).start());
    let host = r.place_on_field(0, ID, Some(0));
    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_none());
    assert!(!permanent_exists(&r, 0, ID));
    assert_eq!(r.security_count(0), 5);
}

#[test]
fn bt20_056_attack_tail_evolves_from_hand_or_trash_for_both_controllers_and_attackers() {
    for owner in [0, 1] {
        for evo in [false, true] {
            for trash in [false, true] {
                for own_attack in [false, true] {
                    let mut r = full_ready(owner, full_builder(owner, &[ID]).start());
                    if trash {
                        let card = r.game.players[owner as usize].hand.remove(0);
                        r.game.players[owner as usize].trash.push(card);
                    }
                    let host = r.place_on_field(owner, ID, Some(0));
                    let foe = r.place_on_field(1 - owner, "OTHER", Some(0));
                    r.place_in_breeding(owner, "BASE-B");
                    attack_window(&mut r, if own_attack { host } else { foe });
                    let memory = r.memory();
                    let hand = r.hand_size(owner);
                    fire_full(&mut r, host, evo);
                    assert_eq!(
                        r.security_count(owner),
                        6,
                        "Recovery precedes optional evolution"
                    );
                    let pick = choices(&r, owner, 1, true)[0];
                    choose(&mut r, owner, pick);
                    let action = zone_action(&r, owner, ID, trash);
                    assert!(choices(&r, owner, 1, true).contains(&action));
                    let mut clone = DebugRunner::wrap(r.game.clone());
                    for game in [&mut r, &mut clone] {
                        choose(game, owner, action);
                        assert_eq!(breeder_id(game, owner), Some(ID));
                        assert_eq!(game.battle_area_size(owner), 1);
                        assert_eq!(game.memory(), memory, "free evolution preserves memory");
                        assert_eq!(
                            game.game.players[owner as usize].deck.len(),
                            28,
                            "Recovery and exactly one draw"
                        );
                        assert_eq!(game.hand_size(owner), hand + usize::from(trash));
                        assert_eq!(
                            game.security_count(owner),
                            6,
                            "breeding Alphamon does not trigger Recovery"
                        );
                        assert!(game.pending_selection().is_none());
                    }
                }
            }
        }
    }
}

#[test]
fn bt20_056_attack_tail_union_masks_filter_trait_level_kind_and_evolution_requirements() {
    for owner in [0, 1] {
        let mut r = full_ready(
            owner,
            full_builder(
                owner,
                &[
                    ID,
                    "EVO6",
                    "EVO5",
                    "EVO7",
                    "NO-TRAIT",
                    "WRONG-COLOR",
                    "WRONG-LEVEL",
                    "TAMER",
                ],
            )
            .start(),
        );
        let card = r.game.players[owner as usize].hand.remove(1);
        r.game.players[owner as usize].trash.push(card);
        let host = r.place_on_field(owner, ID, Some(0));
        r.place_in_breeding(owner, "BASE-B");
        attack_window(&mut r, host);
        fire_full(&mut r, host, false);
        let pick = choices(&r, owner, 1, true)[0];
        choose(&mut r, owner, pick);
        let actions = choices(&r, owner, 3, true);
        for (id, trash) in [(ID, false), ("EVO5", false), ("EVO6", true)] {
            assert!(actions.contains(&zone_action(&r, owner, id, trash)), "{id}");
        }
        for id in ["EVO7", "NO-TRAIT", "WRONG-COLOR", "WRONG-LEVEL", "TAMER"] {
            assert!(owned_union_fixture::action(&r.game, owner, id, false).is_none(), "{id} must have no legal physical row");
        }
        let action = zone_action(&r, owner, "EVO5", false);
        choose(&mut r, owner, action);
        assert_eq!(breeder_id(&r, owner), Some("EVO5"));
    }
}

#[test]
fn bt20_056_attack_tail_decline_at_each_stage_preserves_recovery() {
    for owner in [0, 1] {
        for decline_card in [false, true] {
            let mut r = full_ready(owner, full_builder(owner, &["EVO6"]).start());
            let host = r.place_on_field(owner, ID, Some(0));
            r.place_in_breeding(owner, "BASE-B");
            attack_window(&mut r, host);
            fire_full(&mut r, host, false);
            let pick = choices(&r, owner, 1, true)[0];
            if decline_card {
                choose(&mut r, owner, pick);
                choices(&r, owner, 1, true);
            }
            choose(&mut r, owner, PASS);
            assert_eq!(r.security_count(owner), 6);
            assert_eq!(r.game.players[owner as usize].deck.len(), 29);
            assert_eq!(r.hand_size(owner), 1);
            assert_eq!(breeder_id(&r, owner), Some("BASE-B"));
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_056_attack_tail_requires_breeder_and_legal_candidate_but_recovery_still_runs() {
    for reason in ["no breeder", "no candidate", "wrong requirement"] {
        let mut r = full_ready(
            0,
            full_builder(
                0,
                &[if reason == "no candidate" {
                    "NO-TRAIT"
                } else {
                    "EVO6"
                }],
            )
            .start(),
        );
        let host = r.place_on_field(0, ID, Some(0));
        if reason != "no breeder" {
            r.place_in_breeding(
                0,
                if reason == "wrong requirement" {
                    "BASE-P"
                } else {
                    "BASE-B"
                },
            );
        }
        attack_window(&mut r, host);
        fire_full(&mut r, host, false);
        assert!(r.pending_selection().is_none(), "{reason}");
        assert_eq!(r.security_count(0), 6, "{reason}");
        assert_eq!(r.hand_size(0), 1);
    }
}

#[test]
fn bt20_056_attack_tail_is_independent_of_successful_recovery() {
    let mut r = full_ready(0, full_builder(0, &["EVO6"]).deck(0, &[]).start());
    let host = r.place_on_field(0, ID, Some(0));
    r.place_in_breeding(0, "BASE-B");
    attack_window(&mut r, host);
    fire_full(&mut r, host, false);
    choices(&r, 0, 1, true);
    assert_eq!(r.security_count(0), 5);
    choose(&mut r, 0, PASS);
    assert_eq!(breeder_id(&r, 0), Some("BASE-B"));
}

#[test]
fn bt20_056_security_debuff_choice_is_mandatory_excludes_tamers_and_expires_and_resets() {
    for owner in [0, 1] {
        let mut r = full_ready(owner, full_builder(owner, &[]).start());
        let host = r.place_on_field(owner, ID, Some(0));
        let first = r.place_on_field(1 - owner, "OTHER", Some(0));
        let second = r.place_on_field(1 - owner, "BASE-B", Some(0));
        r.place_on_field(1 - owner, "TAMER", Some(0));
        trash_top_security_via_effect(&mut r, host, 1 - owner);
        let actions = choices(&r, owner, 2, false);
        assert_eq!(r.pending_selection().unwrap().zone_owner, Some(1 - owner));
        assert_eq!(
            actions,
            vec![
                space::encode_attack(0, first.index as u16),
                space::encode_attack(0, second.index as u16),
            ]
        );
        // OppField carries the side; single-field selections use 100 + slot.
        let pick = space::encode_attack(0, second.index as u16);
        assert!(actions.contains(&pick));
        choose(&mut r, owner, pick);
        assert_eq!(r.dp_of(first), Some(12000));
        assert_eq!(r.dp_of(second), Some(4000));
        trash_top_security_via_effect(&mut r, host, owner);
        assert!(r.pending_selection().is_none());
        assert_eq!(build_action_mask(&r.game, owner)[PASS as usize], 1.0);
        r.game.decode_action(PASS, owner);
        assert_eq!(r.game.turn_player(), 1 - owner);
        assert_eq!(r.dp_of(second), Some(12000), "for-the-turn debuff expires");
        trash_top_security_via_effect(&mut r, host, owner);
        let pick = choices(&r, owner, 2, false)[0];
        choose(&mut r, owner, pick);
        assert_eq!(r.dp_of(first), Some(4000), "OPT resets on opponent's turn");
    }
}

#[test]
fn bt20_056_inherited_replacement_masks_and_opt_hold_for_both_controllers() {
    for owner in [0, 1] {
        let mut r = full_ready(owner, full_builder(owner, &[]).start());
        let host = r.place_stack(owner, &[ID, "OURYUKEN"]);
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        let actions = choices(&r, owner, 1, true);
        assert_eq!(actions, vec![REPLACEMENT_ACCEPT]);
        choose(&mut r, owner, REPLACEMENT_ACCEPT);
        assert!(permanent_exists(&r, owner as usize, "OURYUKEN"));
        assert_eq!(r.security_count(owner), 4);
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection().is_none());
        assert!(!permanent_exists(&r, owner as usize, "OURYUKEN"));
        assert_eq!(
            r.security_count(owner),
            4,
            "second leave cannot pay again this turn"
        );
    }
}

#[test]
fn bt20_056_inherited_replacement_requires_exact_name_and_available_security() {
    for (top, security) in [("NEAR-OURYUKEN", true), ("OURYUKEN", false)] {
        let mut r = full_ready(0, full_builder(0, &[]).start());
        if !security {
            r.game.players[0].security.clear();
        }
        let host = r.place_stack(0, &[ID, top]);
        r.game
            .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
        assert!(r.pending_selection().is_none());
        assert!(!permanent_exists(&r, 0, top));
        assert_eq!(r.security_count(0), if security { 5 } else { 0 });
    }
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
