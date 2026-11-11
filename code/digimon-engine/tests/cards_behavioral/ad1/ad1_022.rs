//! AD1-022 Izzy Izumi & Tai Kamiya — Tamer, Green/Red, Cost 3. ADVENTURE / Hero.
//!
//! # Card text (card image + official Bandai DB; data/card_bundles/AD1-022.md)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [Your Turn] When any of your other [ADVENTURE] trait Digimon or Tamers are
//!   played, by suspending this Tamer, 1 of your Digimon may digivolve into an
//!   [ADVENTURE] trait Digimon card in the hand. For every 2 of your Tamers'
//!   colors, reduce this effect's digivolution cost by 1.
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Green/AD1_022.cs
//!   - OnStartMainPhase → Gain1MemoryTamerOpponentDigimonEffect.
//!   - OnEnterFieldAnyone (isOptional TRUE): on the battle area, IsOwnerTurn,
//!     CanTriggerOnPermanentPlay for a permanent other than this Tamer that is
//!     the owner's Digimon or Tamer with the ADVENTURE trait; the suspend is the
//!     cost; then optional own-Digimon pick → DigivolveIntoHandOrTrashCard(
//!     IsAdventureDigimon, payCost true, reduceCostTuple = floor(distinct Tamer
//!     colors / 2)).
//!   - SecuritySkill → PlaySelfTamerSecurityEffect.
//!
//! # Official Q&A
//!   - Q6112: two copies' effects can't be combined into one digivolution with a
//!     doubled reduction — each activation is its own effect with its own
//!     suspend cost and its own reduction.
//!
//! # Patterns this test file covers (RUST_DSL_TEST_API.md §4.3)
//! - B1  start-of-main tamer (memory gain gated on an opponent Digimon)
//! - B3  trigger-on-event tamer (on ADVENTURE play)
//! - D2-adjacent: formula-reduced effect digivolution cost
//! - Cost: "by suspending this Tamer" (suspend is the activation cost)
//! - E1  optional decline at each stage (outer, host, card)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    card_data::{CardData, EvoCost},
    debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder},
    enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource},
    permanent::PermanentHandle,
    TriggerSource,
};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const ID: &str = "AD1-022";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Lv.3 red Digimon: the digivolution base.
fn base_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// Lv.4 red Digimon (optionally ADVENTURE) with a printed Lv.3 red cost-4
/// digivolution circle: the hand card the effect digivolves into.
fn evo_digimon(id: &str, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c.evo_costs = vec![EvoCost {
        card_color: 0, // red, matches make_test_card's default colour
        level: 3,
        memory_cost: 4,
    }];
    c
}

/// A Tamer fixture with explicit colours and optional ADVENTURE trait.
fn tamer(id: &str, colors: Vec<CardColor>, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = colors;
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut played_digimon = base_digimon("ADV-D");
    played_digimon.traits = vec!["ADVENTURE".to_string()];
    let mut plain_digimon = base_digimon("PLAIN-D");
    plain_digimon.traits = vec![];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("AD1-022 YAML")
        .add_card(base_digimon("BASE"))
        .add_card(base_digimon("BASE2"))
        .add_card(played_digimon)
        .add_card(plain_digimon)
        .add_card(evo_digimon("EVO", true))
        .add_card(evo_digimon("EVO2", true))
        .add_card(evo_digimon("PLAIN-EVO", false))
        .add_card(tamer("ADV-T", vec![CardColor::Yellow], true))
        .add_card(tamer("PLAIN-T", vec![CardColor::Yellow], false))
        .add_card(tamer(
            "BLUE-YELLOW-T",
            vec![CardColor::Blue, CardColor::Yellow],
            false,
        ))
        .add_card(tamer(
            "GREEN-RED-T",
            vec![CardColor::Green, CardColor::Red],
            false,
        ))
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(10)
}

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn pick(r: &mut DebugRunner, a: u16) {
    let p = r
        .pending_selection()
        .expect("a pending choice")
        .selecting_player;
    assert!(
        build_action_mask(&r.game, p)[a as usize] > 0.5,
        "action {a} must be legal in the mask"
    );
    r.execute_action(p, a).unwrap();
}

/// Pick the first non-PASS legal action of the pending selection.
fn pick_first(r: &mut DebugRunner) {
    let a = *r
        .pending_selection()
        .expect("a pending choice")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a legal non-PASS action");
    pick(r, a);
}

fn on_field(r: &DebugRunner, slot: usize) -> String {
    r.game.players[0].battle_area[slot]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[0].battle_area[h.index as usize].is_suspended
}

/// Place an already-built permanent for `id` and fire its play-event bundle
/// (OnPlay + OnEnterFieldAnyone + OnAllyPlayed), as the real play flow does.
fn play_event(r: &mut DebugRunner, id: &str) -> PermanentHandle {
    let h = r.place_on_field(0, id, Some(0));
    r.fire_play_event_triggers(0, h.index as usize, false, false);
    h
}

/// Standard board: AD1-022 + a Lv.3 base on the field, EVO in hand, our turn.
fn board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder().hand(0, &["EVO"]).start();
    let tamer = r.place_on_field(0, ID, Some(0));
    let base = r.place_on_field(0, "BASE", Some(0));
    (r, tamer, base)
}

fn triggered(id: &str) -> Vec<CompiledTriggeredClause> {
    compiled(id)
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structure and printed identity
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_022_identity_matches_print() {
    let card = compiled(ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.name, "Izzy Izumi & Tai Kamiya");
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.level, None);
    assert_eq!(card.dp, None);
    let colors: Vec<CompiledColor> = card.color.iter().copied().collect();
    assert_eq!(colors.len(), 2, "green / red");
    assert!(colors.contains(&CompiledColor::Green));
    assert!(colors.contains(&CompiledColor::Red));
    let traits: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(traits.contains(&"adventure".to_string()), "{traits:?}");
    assert!(traits.contains(&"hero".to_string()), "{traits:?}");
}

#[test]
fn ad1_022_has_three_triggered_clauses_with_printed_shape() {
    let clauses = triggered(ID);
    assert_eq!(clauses.len(), 3);

    let memory = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("[Start of Your Main Phase] clause");
    assert!(!memory.optional, "gaining the memory is not a 'may'");
    assert!(!memory.once_per_turn);

    let digivolve = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnAllyPlayed])
        .expect("[Your Turn] on-play observer clause");
    assert!(digivolve.optional, "DCGO isOptional TRUE");
    assert!(!digivolve.once_per_turn, "no [Once Per Turn] is printed");
    assert!(digivolve.active_when.is_some(), "[Your Turn] gate");

    let security = clauses
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSecurity])
        .expect("[Security] clause");
    assert!(!security.optional);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [Start of Your Main Phase]: opponent has a Digimon → gain 1
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_022_start_of_main_gains_memory_when_opponent_has_a_digimon() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "BASE", Some(0));
    r.game.memory = 0;
    trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
    assert_eq!(r.memory(), 1);
}

#[test]
fn ad1_022_start_of_main_no_memory_without_an_opponent_digimon() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.memory = 0;
    trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
    assert_eq!(r.memory(), 0);
}

#[test]
fn ad1_022_start_of_main_an_opponent_tamer_is_not_a_digimon() {
    let mut r = builder().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "PLAIN-T", Some(0));
    r.game.memory = 0;
    trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
    assert_eq!(r.memory(), 0, "the gate reads 'has a Digimon'");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [Your Turn] another ADVENTURE Digimon / Tamer is played:
//             by suspending this Tamer, 1 Digimon may digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Full positive path for an ADVENTURE Digimon being played: the optional
/// activation, the suspend cost, the host pick, the hand-card pick, and the
/// digivolution at the printed cost minus floor(2 colours / 2) = 1.
#[test]
fn ad1_022_adventure_digimon_played_digivolves_at_reduced_cost() {
    let (mut r, tamer_h, base) = board();
    let mem = r.memory();
    play_event(&mut r, "ADV-D");

    assert!(r.pending_is_optional(), "the activation is a 'may'");
    assert!(
        !suspended(&r, tamer_h),
        "the cost is not paid before the player accepts"
    );
    pick_first(&mut r); // accept the activation
    assert!(
        suspended(&r, tamer_h),
        "by suspending this Tamer: the cost is paid on activation"
    );

    // 1 of your Digimon: BASE is a legal host (the played ADV-D is a Lv.3 too).
    assert!(
        r.pending_is_optional(),
        "'may digivolve': host pick can pass"
    );
    let host_actions = r.pending_selection().unwrap().valid_action_ids.clone();
    assert!(
        host_actions.iter().any(|&a| a != PASS),
        "at least one legal Digimon host"
    );
    // Choose the BASE specifically (first own-field action maps to slot 0 = the
    // Tamer is not a Digimon; BASE is slot 1).
    let base_action = host_actions
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("host action");
    pick(&mut r, base_action);

    assert!(r.pending_is_optional(), "the hand-card pick can pass");
    pick_first(&mut r); // EVO from hand
    r.auto_resolve().unwrap();

    let digivolved = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EVO");
    assert!(digivolved, "a Digimon digivolved into [ADVENTURE] EVO");
    assert_eq!(
        mem - r.memory(),
        3,
        "printed digivolution cost 4 reduced by floor(2 Tamer colours / 2) = 1"
    );
    let _ = base;
}

/// An ADVENTURE Tamer being played triggers the effect as well.
#[test]
fn ad1_022_adventure_tamer_played_also_triggers() {
    let (mut r, tamer_h, _base) = board();
    play_event(&mut r, "ADV-T");
    assert!(r.pending_selection().is_some(), "activation prompt");
    pick_first(&mut r);
    assert!(suspended(&r, tamer_h));
}

/// The reduction counts distinct colours across ALL of the controller's
/// Tamers: AD1-022 (green/red) + a blue/yellow Tamer = 4 colours → reduce 2.
#[test]
fn ad1_022_reduction_scales_with_distinct_tamer_colors() {
    let mut r = builder().hand(0, &["EVO"]).start();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BLUE-YELLOW-T", Some(0));
    r.place_on_field(0, "BASE", Some(0));
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    pick_first(&mut r); // accept (the first prompt belongs to AD1-022)
    pick_first(&mut r); // host
    pick_first(&mut r); // card
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EVO"));
    assert_eq!(
        mem - r.memory(),
        2,
        "4 distinct Tamer colours → reduction floor(4 / 2) = 2; cost 4 - 2"
    );
}

/// Duplicate colours across Tamers collapse: AD1-022 (green/red) + another
/// green/red Tamer is still just 2 colours → reduction 1.
#[test]
fn ad1_022_duplicate_tamer_colors_do_not_stack() {
    let mut r = builder().hand(0, &["EVO"]).start();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "GREEN-RED-T", Some(0));
    r.place_on_field(0, "BASE", Some(0));
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    pick_first(&mut r);
    pick_first(&mut r);
    pick_first(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EVO"));
    assert_eq!(
        mem - r.memory(),
        3,
        "same 2 colours → reduction 1; cost 4 - 1"
    );
}

// ─── Trigger gating ──────────────────────────────────────────────────────────

/// A played Digimon without the ADVENTURE trait does not trigger.
#[test]
fn ad1_022_non_adventure_digimon_played_does_not_trigger() {
    let (mut r, tamer_h, _base) = board();
    play_event(&mut r, "PLAIN-D");
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, tamer_h));
}

/// A played Tamer without the ADVENTURE trait does not trigger.
#[test]
fn ad1_022_non_adventure_tamer_played_does_not_trigger() {
    let (mut r, tamer_h, _base) = board();
    play_event(&mut r, "PLAIN-T");
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, tamer_h));
}

/// "Any of YOUR other [ADVENTURE] ...": the opponent's ADVENTURE Digimon does
/// not trigger it.
#[test]
fn ad1_022_opponents_adventure_play_does_not_trigger() {
    let (mut r, tamer_h, _base) = board();
    let opp = r.place_on_field(1, "ADV-D", Some(0));
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, tamer_h));
}

/// "Other": this Tamer's own play (it has the ADVENTURE trait) must not
/// trigger itself.
#[test]
fn ad1_022_its_own_play_does_not_trigger_itself() {
    let mut r = builder().hand(0, &["EVO"]).start();
    r.place_on_field(0, "BASE", Some(0));
    let tamer_h = play_event(&mut r, ID);
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, tamer_h));
}

/// Another copy of AD1-022 (an ADVENTURE Tamer) being played DOES trigger the
/// first copy, but never itself.
#[test]
fn ad1_022_a_second_copy_triggers_the_first_but_not_itself() {
    let (mut r, first, _base) = board();
    let second = play_event(&mut r, ID);
    assert!(r.pending_selection().is_some(), "the first copy triggers");
    pick_first(&mut r);
    assert!(suspended(&r, first), "first copy paid its cost");
    assert!(
        !suspended(&r, second),
        "the played copy did not trigger itself"
    );
}

/// [Your Turn]: nothing happens on the opponent's turn.
#[test]
fn ad1_022_opponents_turn_does_not_trigger() {
    let (mut r, tamer_h, _base) = board();
    r.game.turn_player_idx = 1;
    play_event(&mut r, "ADV-D");
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, tamer_h));
}

/// "By suspending this Tamer": an already-suspended Tamer cannot pay the cost,
/// so the effect isn't offered.
#[test]
fn ad1_022_a_suspended_tamer_cannot_activate() {
    let (mut r, tamer_h, _base) = board();
    r.game.players[0].battle_area[tamer_h.index as usize].is_suspended = true;
    play_event(&mut r, "ADV-D");
    assert!(r.pending_selection().is_none());
}

// ─── Optional declines ───────────────────────────────────────────────────────

/// Declining the activation pays nothing: the Tamer stays unsuspended and no
/// digivolution happens.
#[test]
fn ad1_022_declining_the_activation_keeps_the_tamer_unsuspended() {
    let (mut r, tamer_h, _base) = board();
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    pick(&mut r, PASS);
    r.auto_resolve().unwrap();
    assert!(!suspended(&r, tamer_h));
    assert_eq!(r.memory(), mem);
    assert!(!r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EVO"));
}

/// After the cost is paid the Digimon pick is still a "may": passing it leaves
/// the Tamer suspended and digivolves nothing.
#[test]
fn ad1_022_declining_the_host_pick_still_pays_the_suspend_cost() {
    let (mut r, tamer_h, _base) = board();
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    pick_first(&mut r); // accept
    pick(&mut r, PASS); // decline the Digimon pick
    r.auto_resolve().unwrap();
    assert!(suspended(&r, tamer_h), "the suspend cost was already paid");
    assert_eq!(r.memory(), mem, "no digivolution cost paid");
}

/// Declining the hand-card pick also leaves the Tamer suspended.
#[test]
fn ad1_022_declining_the_card_pick_still_pays_the_suspend_cost() {
    let (mut r, tamer_h, _base) = board();
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    pick_first(&mut r); // accept
    pick_first(&mut r); // host
    pick(&mut r, PASS); // decline the card
    r.auto_resolve().unwrap();
    assert!(suspended(&r, tamer_h));
    assert_eq!(r.memory(), mem);
}

// ─── Candidate filtering ─────────────────────────────────────────────────────

/// Only [ADVENTURE] trait Digimon cards in hand are candidates: with only a
/// non-ADVENTURE Digimon in hand there is nothing to digivolve into, so after
/// the cost no host/card prompt appears.
#[test]
fn ad1_022_non_adventure_hand_card_is_not_a_candidate() {
    let mut r = builder().hand(0, &["PLAIN-EVO"]).start();
    let tamer_h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BASE", Some(0));
    play_event(&mut r, "ADV-D");
    pick_first(&mut r); // accept
    assert!(
        suspended(&r, tamer_h),
        "the cost is paid even when nothing can be digivolved"
    );
    r.auto_resolve().unwrap();
    assert!(!r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "PLAIN-EVO"));
}

/// The hand card must be able to digivolve onto the chosen host: a Lv.4
/// ADVENTURE card cannot digivolve onto a Lv.4 host.
#[test]
fn ad1_022_a_host_without_a_legal_route_is_not_offered() {
    let mut r = builder().hand(0, &["EVO"]).start();
    r.place_on_field(0, ID, Some(0));
    // Only Lv.4 Digimon on the field besides the Tamer: EVO (Lv.4, evolves from
    // Lv.3) has no route onto it.
    r.place_on_field(0, "EVO2", Some(0));
    play_event(&mut r, "ADV-D"); // the played Lv.3 ADV-D IS a legal host
    pick_first(&mut r); // accept
    let view = r.pending_selection_view().expect("host prompt");
    let hosts: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        hosts.len(),
        1,
        "only the Lv.3 ADV-D can be digivolved into EVO; the Lv.4 EVO2 cannot"
    );
}

/// The digivolution cost must be payable ("payCost: true"): at a memory that
/// cannot pay the reduced cost no host is offered.
#[test]
fn ad1_022_unaffordable_digivolution_offers_no_host() {
    let mut r = builder().hand(0, &["EVO"]).start();
    r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BASE", Some(0));
    r.game.memory = -9; // paying 3 more would cross the -10 floor
    play_event(&mut r, "ADV-D");
    pick_first(&mut r); // accept
    let offered = r
        .pending_selection()
        .map(|p| p.valid_action_ids.iter().any(|&a| a != PASS))
        .unwrap_or(false);
    assert!(!offered, "no payable digivolution route exists");
}

// ─── Q6112: each activation is its own effect ───────────────────────────────

/// Two copies on the field: each copy's activation pays its own suspend cost
/// and applies only its own reduction (2 colours → 1) to its own digivolution —
/// they never combine into one doubled reduction.
#[test]
fn ad1_022_q6112_two_copies_do_not_combine_their_reductions() {
    let mut r = builder().hand(0, &["EVO", "EVO2"]).start();
    let first = r.place_on_field(0, ID, Some(0));
    let second = r.place_on_field(0, ID, Some(0));
    r.place_on_field(0, "BASE", Some(0));
    r.place_on_field(0, "BASE2", Some(0));
    let mem = r.memory();
    play_event(&mut r, "ADV-D");
    // Resolve both copies' effects: accept, host, card each time.
    let mut guard = 0;
    while r.pending_selection().is_some() {
        guard += 1;
        assert!(guard < 12, "selection loop runaway");
        pick_first(&mut r);
    }
    assert!(suspended(&r, first) && suspended(&r, second), "both paid");
    let evolved = r.game.players[0]
        .battle_area
        .iter()
        .filter(|p| {
            let id = p.top_card().card_id(&r.game.card_data);
            id == "EVO" || id == "EVO2"
        })
        .count();
    assert_eq!(evolved, 2, "one digivolution per activation");
    assert_eq!(
        mem - r.memory(),
        6,
        "each digivolution cost 4 - 1; never a doubled reduction on one card"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ad1_022_security_dispatch_plays_self_free() {
    let mut r = builder().security(0, &[ID]).start();
    let attacker = r.place_on_field(1, "BASE", Some(0));
    r.game.turn_player_idx = 1;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    let memory = r.memory();
    let _ = r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(on_field(&r, 0), ID);
    assert_eq!(r.memory(), memory, "played without paying the cost");
}
