//! Phase 1 timing-dispatch integration tests.
//!
//! Sanity-checks that all EffectBuilder constructors added in Phase 1 Task 1
//! compile and produce an Effect with the correct timing. Actual dispatch
//! wiring is tested in subsequent Phase 1 tasks.

use digimon_engine::action::space::{BREEDING_TARGET, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect, EffectBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, CardSourceRef, CostDelta, DelayTrigger, EffectTiming, Expiry,
    ModifierType, PlaySource, StackPosition, Zone,
};
use digimon_engine::events::GameEvent;
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::replacement::{ReplacementCause, ReplacementSubject};
use digimon_engine::selection::{AttackTarget, SelectionKind, TriggerSource};
use digimon_engine::trigger_context::EventCause;
use digimon_engine::PlayerId;
use std::sync::{Arc, Mutex};

fn dummy() -> CardHandle {
    CardHandle(0)
}

#[test]
fn new_effect_timings_are_constructible() {
    // Verify each new builder constructor compiles and sets the right timing.
    let card = dummy();

    let e = Effect::start_of_your_turn(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfYourTurn);

    let e = Effect::start_of_opponents_turn(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfOpponentsTurn);

    let e = Effect::start_of_your_main_phase(card).build();
    assert_eq!(e.timing, EffectTiming::StartOfYourMainPhase);

    let e = Effect::end_of_opponents_turn(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfOpponentsTurn);

    let e = Effect::when_attacking(card).build();
    assert_eq!(e.timing, EffectTiming::WhenAttacking);

    let e = Effect::end_of_attack(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfAttack);

    let e = Effect::end_of_battle(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfBattle);

    let e = Effect::on_enter_field_anyone(card).build();
    assert_eq!(e.timing, EffectTiming::OnEnterFieldAnyone);

    let e = Effect::on_any_digimon_played(card).build();
    assert_eq!(e.timing, EffectTiming::OnEnterFieldAnyone);

    let e = Effect::on_ally_played(card).build();
    assert_eq!(e.timing, EffectTiming::OnAllyPlayed);

    let e = Effect::on_any_deletion(card).build();
    assert_eq!(e.timing, EffectTiming::OnAnyDeletion);

    let e = Effect::on_digivolve(card).build();
    assert_eq!(e.timing, EffectTiming::OnDigivolve);

    let e = Effect::on_suspend(card).build();
    assert_eq!(e.timing, EffectTiming::OnSuspend);

    let e = Effect::on_unsuspend(card).build();
    assert_eq!(e.timing, EffectTiming::OnUnsuspend);

    let e = Effect::on_attack_target_change(card).build();
    assert_eq!(e.timing, EffectTiming::OnAttackTargetChange);

    let e = Effect::on_hatch(card).build();
    assert_eq!(e.timing, EffectTiming::OnHatch);

    let e = Effect::on_move(card).build();
    assert_eq!(e.timing, EffectTiming::OnMove);

    let e = Effect::on_opponent_security_removed(card).build();
    assert_eq!(e.timing, EffectTiming::OnOpponentSecurityRemoved);

    let e = Effect::on_own_security_removed(card).build();
    assert_eq!(e.timing, EffectTiming::OnOwnSecurityRemoved);

    let e = Effect::on_digivolution_card_trashed(card).build();
    assert_eq!(e.timing, EffectTiming::OnDigivolutionCardTrashed);

    let e = Effect::on_option_placed(card).build();
    assert_eq!(e.timing, EffectTiming::OnOptionPlaced);

    let e = Effect::when_permanent_would_digivolve(card).build();
    assert_eq!(e.timing, EffectTiming::WhenPermanentWouldDigivolve);

    let e = Effect::when_permanent_would_play(card).build();
    assert_eq!(e.timing, EffectTiming::WhenPermanentWouldPlay);

    let e = Effect::when_would_link(card).build();
    assert_eq!(e.timing, EffectTiming::WhenWouldLink);

    let e = Effect::on_place_security(card).build();
    assert_eq!(e.timing, EffectTiming::OnPlaceSecurity);

    let e = Effect::on_added_to_security(card).build();
    assert_eq!(e.timing, EffectTiming::OnPlaceSecurity);

    let e = Effect::on_discard_security(card).build();
    assert_eq!(e.timing, EffectTiming::OnDiscardSecurity);
}

#[test]
fn existing_builders_still_correct() {
    // Regression check: pre-existing constructors must not have been disturbed.
    let card = dummy();

    let e = Effect::on_play(card).build();
    assert_eq!(e.timing, EffectTiming::OnPlay);
    assert!(e.on_play);

    let e = Effect::when_digivolving(card).build();
    assert_eq!(e.timing, EffectTiming::WhenDigivolving);
    assert!(e.when_digivolving);

    let e = Effect::on_attack(card).build();
    assert_eq!(e.timing, EffectTiming::OnAttack);
    assert!(e.on_attack);

    let e = Effect::on_deletion(card).build();
    assert_eq!(e.timing, EffectTiming::OnDeletion);
    assert!(e.on_deletion);

    let e = Effect::end_of_your_turn(card).build();
    assert_eq!(e.timing, EffectTiming::EndOfYourTurn);

    let e = Effect::declarative(card).build();
    assert_eq!(e.timing, EffectTiming::Declarative);
    assert!(e.declarative);
}

// ─── Helper ───────────────────────────────────────────────────────────────────

/// A Lv.3 Red Digimon with configurable play_cost and no inherent effects.
fn plain_digimon(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn add_breeding_source(r: &mut DebugRunner, player: PlayerId, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("add_breeding_source: unknown card_id {card_id}"));
    let next_idx = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    let handle = card.handle();
    let permanent = r.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    let top = permanent.card_sources.pop().expect("breeding top exists");
    permanent.card_sources.push(card);
    permanent.card_sources.push(top);
    handle
}

fn option_card_with_cost(card_id: &str, name: &str, play_cost: u16, traits: &[&str]) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost,
        colors: vec![CardColor::Red],
        traits: traits.iter().map(|t| t.to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn option_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    option_card_with_cost(card_id, name, 0, traits)
}

fn token_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = plain_digimon(card_id, name, 0);
    card.card_kind = CardKind::Token;
    card.level = None;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

// ─── TEST-P1-T2 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the start of the controller's turn.
struct StartTurnMemoryGain;
impl CardEffect for StartTurnMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::start_of_your_turn(card)
            .name("+1 at turn start")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn start_of_your_turn_fires_for_controller() {
    // Provide non-empty decks for both players so neither decks out on draw.
    let filler: Vec<&str> = vec!["FILLER"; 5];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("STM", "StartMem", 3))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["STM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();

    r.register_effect("STM", Arc::new(StartTurnMemoryGain));

    // Play STM on turn 1 (controller = player 0). StartOfYourTurn does NOT
    // fire on the same turn the card was played (begin_turn already ran).
    let played = r.play(0, 0);
    assert_eq!(played, Some(0));

    // Pass turn — now player 1's turn. STM's start_of_your_turn should NOT
    // fire (player 0 is not the turn player).
    r.pass_turn();
    // Pass again — back to player 0. Now STM's start_of_your_turn fires.
    r.pass_turn();

    // Build a control game without the STM effect to derive the baseline
    // memory value after the same sequence of two pass_turn calls.
    let mut control = DebugRunner::builder()
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["FILLER"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    let _ = control.play(0, 0);
    control.pass_turn();
    control.pass_turn();
    let expected_no_effect = control.memory();

    assert_eq!(
        r.memory(),
        expected_no_effect + 1,
        "StartOfYourTurn should have fired for player 0, granting +1 memory"
    );
}

// ─── TEST-P1-T3 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the start of the controller's main phase.
struct MainPhaseMemoryGain;
impl CardEffect for MainPhaseMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::start_of_your_main_phase(card)
            .name("+1 at main phase start")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct MainPhaseObserverRecorder {
    seen: Arc<Mutex<Vec<(String, Option<PermanentHandle>)>>>,
}

impl CardEffect for MainPhaseObserverRecorder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        vec![Effect::start_of_your_main_phase(card)
            .name("record main phase observer")
            .process(move |ctx| {
                let observer_id = ctx
                    .game
                    .card_data_for_handle(ctx.source_card)
                    .map(|data| data.card_id.clone())
                    .unwrap_or_else(|| "<missing>".to_string());
                seen.lock()
                    .unwrap()
                    .push((observer_id, ctx.source_permanent));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn start_of_your_main_phase_fires_for_controller() {
    let filler: Vec<&str> = vec!["F"; 10];
    // Both games use a cost-1 card so post-play memory is identical.
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("MPM", "MainPhase", 1))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["MPM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    r.register_effect("MPM", Arc::new(MainPhaseMemoryGain));

    // Play the card (works from Breeding phase — play_from_hand has no phase check).
    r.play(0, 0);

    // Advance to Main phase on player 0's turn — fires StartOfYourMainPhase.
    r.game_mut().enter_main_phase();
    let mem_after = r.memory();

    // Control: same sequence without the effect, using the same cost-1 card.
    let mut ctrl = DebugRunner::builder()
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["F"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    ctrl.play(0, 0);
    ctrl.game_mut().enter_main_phase();
    let ctrl_mem = ctrl.memory();

    assert_eq!(
        mem_after,
        ctrl_mem + 1,
        "StartOfYourMainPhase should have fired for player 0, granting +1 memory"
    );
}

#[test]
fn start_of_your_main_phase_fans_out_to_battle_and_breeding_once_each() {
    let filler: Vec<&str> = vec!["F"; 10];
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BATTLE-MAIN", "Battle Main", 1))
        .add_card(plain_digimon("BREED-MAIN", "Breed Main", 1))
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(0)
        .start();
    r.register_effect(
        "BATTLE-MAIN",
        Arc::new(MainPhaseObserverRecorder { seen: seen.clone() }),
    );
    r.register_effect(
        "BREED-MAIN",
        Arc::new(MainPhaseObserverRecorder { seen: seen.clone() }),
    );

    let battle = r.place_on_field(0, "BATTLE-MAIN", Some(0));
    r.place_in_breeding(0, "BREED-MAIN");

    let before = r.memory();
    r.game_mut().enter_main_phase();

    let (selecting_player, action_id) = {
        let selection = r
            .game
            .pending_selection
            .as_ref()
            .expect("battle + breeding phase triggers should ask for order");
        assert_eq!(selection.kind, SelectionKind::TriggerOrder);
        assert_eq!(
            selection.valid_action_ids.len(),
            2,
            "battle and breeding observers should be the only trigger-order choices"
        );
        (selection.selecting_player, selection.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve phase trigger order");

    assert_eq!(
        r.memory(),
        before + 2,
        "battle-area and breeding-area observers should each grant memory once"
    );
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            ("BATTLE-MAIN".to_string(), Some(battle)),
            (
                "BREED-MAIN".to_string(),
                Some(PermanentHandle {
                    player: 0,
                    index: BREEDING_TARGET as u8
                })
            )
        ],
        "phase fan-out should scan battle and breeding through distinct one-shot handles"
    );
}

// ─── TEST-P1-T4 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the end of the opponent's turn.
struct EndOfOppTurnMem;
impl CardEffect for EndOfOppTurnMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_opponents_turn(card)
            .name("+1 at end of opponent's turn")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_opponents_turn_fires_for_non_turn_player() {
    // Player 0 plays a card with EndOfOpponentsTurn; when player 1 ends
    // their turn, player 0's effect should fire.
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("EOM", "EndOppMem", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["EOM"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    r.register_effect("EOM", Arc::new(EndOfOppTurnMem));
    r.play(0, 0);

    // Control game without the effect.
    let mut ctrl = DebugRunner::builder()
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["F"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(3)
        .start();
    ctrl.play(0, 0);

    // Pass once: player 0 ends their turn → player 1's turn.
    // EndOfOpponentsTurn does NOT fire here (player 1 is the non-turn player,
    // but they have no permanents with the effect).
    r.pass_turn();
    ctrl.pass_turn();

    // Pass again: player 1 ends their turn → EndOfOpponentsTurn fires for
    // player 0's permanents (player 0 is the non-active player during player
    // 1's turn ending).
    r.pass_turn();
    ctrl.pass_turn();

    // gain_memory(1) is controller-aware. This observer belongs to player 0,
    // so after the turn handoff player 0 sees one more memory than the control
    // game — proving the effect fired.
    assert_eq!(
        r.memory(),
        ctrl.memory() + 1,
        "EndOfOpponentsTurn should have fired for player 0's permanent at end of player 1's turn"
    );
}

// ─── TEST-P1-T5 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when any Digimon in its battle area is
/// attacking (observer timing — fires for the whole attacker's side).
struct AttackingMemoryGain;
impl CardEffect for AttackingMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .name("+1 when attacking")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn when_attacking_fires_only_for_the_attacker() {
    // ATK is the attacker (Lv5, 8000 DP so it survives any security hit).
    // OBS is a bystander on the same side. BOTH carry a [When Attacking]
    // +1-memory clause. [When Attacking] is carrier-scoped ("when THIS Digimon
    // attacks"), so when ATK attacks ONLY ATK's clause fires — the bystander's
    // must NOT. (The retired battle-area-wide behavior fired both → +2.)
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("ATK", Arc::new(AttackingMemoryGain));
    r.register_effect("OBS", Arc::new(AttackingMemoryGain));

    // Play ATK then OBS. ATK lands at battle_area[0], OBS at battle_area[1].
    r.play(0, 0); // ATK (hand slot 0)
    r.play(0, 0); // OBS (now the only card left in hand)

    let attacker_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    // vortex=true bypasses summoning sickness (both were just played this turn).
    let _ = r.attack_player(attacker_handle, 1, /* vortex */ true);

    // Exactly +1: only the attacker's [When Attacking] fires; the bystander
    // OBS does not.
    assert_eq!(
        r.memory(),
        before + 1,
        "only the attacker's [When Attacking] should fire (+1); the non-attacking \
         bystander OBS must not (before={}, after={})",
        before,
        r.memory()
    );
}

#[test]
fn cannot_activate_when_attacking_effects_suppresses_when_attacking_queue() {
    // The attacker itself carries a [When Attacking] +1-memory clause, but is
    // gated by CannotActivateWhenAttackingEffects — so it must NOT fire even
    // though it is the one attacking. (Carrier-scoped: only the attacker's own
    // clause is in scope, and the gate suppresses it.)
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("ATK", Arc::new(AttackingMemoryGain));

    r.play(0, 0); // ATK -> battle_area[0]
    let attacker_handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        attacker_handle,
        ModifierEntry::simple(
            ModifierType::CannotActivateWhenAttackingEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let before = r.memory();
    let result = r.attack_player(attacker_handle, 1, /* vortex */ true);

    assert_ne!(
        result,
        AttackResult::Invalid,
        "test setup must perform a legal attack before checking trigger suppression"
    );

    assert_eq!(
        r.memory(),
        before,
        "the attacker's own gated [When Attacking] must not queue"
    );
}

#[test]
fn cannot_activate_when_digivolving_effects_suppresses_only_gated_permanent() {
    fn setup() -> DebugRunner {
        DebugRunner::builder()
            .add_card(zero_cost_evo_card("TEST-013", "Evolution", &[]))
            .add_card(plain_digimon("F", "F", 1))
            .deck(0, &["F", "F", "F"])
            .deck(1, &["F", "F", "F"])
            .memory(10)
            .start()
    }

    let mut control = setup();
    let control_handle = control.place_on_field(0, "TEST-013", Some(0));
    control.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(control_handle),
    );
    assert_eq!(
        control.game.effect_queue.len(),
        1,
        "control dispatch should queue TEST-013's WhenDigivolving memory effect"
    );

    let mut blocked = setup();
    let blocked_handle = blocked.place_on_field(0, "TEST-013", Some(0));
    blocked.game.modifiers.add(
        blocked_handle,
        ModifierEntry::simple(
            ModifierType::CannotActivateWhenDigivolvingEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );
    let blocked_before = blocked.memory();
    blocked.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(blocked_handle),
    );
    assert!(
        blocked.game.effect_queue.is_empty(),
        "gated permanent must not enqueue its WhenDigivolving effect"
    );
    assert_eq!(
        blocked.memory(),
        blocked_before,
        "WhenDigivolving effects on the gated Digimon must not queue"
    );
}

struct WhenDigivolvingMemoryGain;

impl CardEffect for WhenDigivolvingMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("+1 when digivolving")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct OnPlayMemoryGain;

impl CardEffect for OnPlayMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("+1 on play")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn cannot_activate_on_play_effects_suppresses_only_gated_permanent() {
    fn setup() -> DebugRunner {
        let mut runner = DebugRunner::builder()
            .add_card(plain_digimon("ONPLAY-GAIN", "On Play Gain", 1))
            .memory(10)
            .start();
        runner.register_effect("ONPLAY-GAIN", Arc::new(OnPlayMemoryGain));
        runner
    }

    let mut control = setup();
    let control_handle = control.place_on_field(0, "ONPLAY-GAIN", Some(0));
    control.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(control_handle),
    );
    assert_eq!(
        control.game.effect_queue.len(),
        1,
        "control dispatch should queue ONPLAY-GAIN's On Play memory effect"
    );

    let mut blocked = setup();
    let blocked_handle = blocked.place_on_field(0, "ONPLAY-GAIN", Some(0));
    blocked.game.modifiers.add(
        blocked_handle,
        ModifierEntry::simple(
            ModifierType::CannotActivateOnPlayEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );
    let blocked_before = blocked.memory();
    blocked.game.enqueue_triggered(
        EffectTiming::OnPlay,
        TriggerSource::Permanent(blocked_handle),
    );

    assert!(
        blocked.game.effect_queue.is_empty(),
        "gated permanent must not enqueue its On Play effect"
    );
    assert_eq!(
        blocked.memory(),
        blocked_before,
        "On Play effects on the gated Digimon must not queue"
    );
}

#[test]
fn cannot_activate_when_digivolving_effects_does_not_suppress_on_play() {
    let mut runner = DebugRunner::builder()
        .add_card(plain_digimon("TIMING-GAIN", "Timing Gain", 1))
        .memory(0)
        .start();
    runner.register_effect("TIMING-GAIN", Arc::new(OnPlayMemoryGain));
    let target = runner.place_on_field(0, "TIMING-GAIN", Some(0));
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(
            ModifierType::CannotActivateWhenDigivolvingEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let before = runner.memory();
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(target));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        before + 1,
        "WhenDigivolving lockout must not suppress unrelated On Play effects"
    );
}

#[test]
fn cannot_activate_on_play_effects_does_not_suppress_when_digivolving() {
    let mut runner = DebugRunner::builder()
        .add_card(zero_cost_evo_card("TIMING-GAIN", "Timing Gain", &[]))
        .memory(0)
        .start();
    runner.register_effect("TIMING-GAIN", Arc::new(WhenDigivolvingMemoryGain));
    let target = runner.place_on_field(0, "TIMING-GAIN", Some(0));
    runner.game.modifiers.add(
        target,
        ModifierEntry::simple(
            ModifierType::CannotActivateOnPlayEffects,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let before = runner.memory();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(target),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.memory(),
        before + 1,
        "On Play lockout must not suppress unrelated When Digivolving effects"
    );
}

// ─── TEST-P1-T6 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory at the end of any attack.
struct EoAttackMemoryGain;
impl CardEffect for EoAttackMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_attack(card)
            .name("+1 end of attack")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_attack_fires_for_all_players() {
    let mut attacker_data = plain_digimon("ATK", "Attacker", 5);
    attacker_data.level = Some(5);
    attacker_data.dp = Some(8000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(attacker_data)
        .add_card(plain_digimon("OBS", "EoAObserver", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EoAttackMemoryGain));

    r.play(0, 0); // ATK
    r.play(0, 0); // OBS

    let attacker_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    let _ = r.attack_player(attacker_handle, 1, true);

    assert!(
        r.memory() > before,
        "EndOfAttack should have fired, granting +1 from OBS (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T7 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when a Digimon-vs-Digimon battle resolves.
struct EoBattleMemoryGain;
impl CardEffect for EoBattleMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_battle(card)
            .name("+1 end of battle")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn end_of_battle_fires_on_digimon_vs_digimon() {
    // ATK (9000 DP) beats DEF (5000 DP). OBS on ATK's side has EndOfBattle.
    let mut atk = plain_digimon("ATK", "Attacker", 5);
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut def = plain_digimon("DEF", "Defender", 5);
    def.level = Some(5);
    def.dp = Some(5000); // will lose to ATK

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(def)
        .add_card(plain_digimon("OBS", "BattleObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EoBattleMemoryGain));

    r.play(0, 0); // ATK → battle_area[0]
    r.play(0, 0); // OBS → battle_area[1]

    // Place DEF on player 1's field directly (bypasses the memory seesaw).
    let defender_h = r.place_on_field(1, "DEF", Some(0));

    let attacker_h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    let _ = r.attack_digimon(attacker_h, defender_h, true);

    assert!(
        r.memory() > before,
        "EndOfBattle should have fired on the ATK vs DEF battle (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T8 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon enters the field.
struct EntryObserverMem;
impl CardEffect for EntryObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_enter_field_anyone(card)
            .name("+1 when any Digimon enters")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_enter_field_anyone_fires_for_all_players() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Observer", 3))
        .add_card(plain_digimon("P1", "Play1", 2))
        .hand(0, &["OBS", "P1"])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(EntryObserverMem));

    // Play OBS first (its own entry triggers OnEnterFieldAnyone — but
    // the observer is the card being played, so its effects fire on
    // itself too, yielding +1). Then play P1, which also triggers
    // OnEnterFieldAnyone for OBS (which is now on the field), yielding +1.
    r.play(0, 0); // OBS
    let after_obs = r.memory();
    r.play(0, 0); // P1
    let after_p1 = r.memory();

    // The delta between after_obs and after_p1 includes:
    //   -2 (play_cost of P1) + 1 (OBS observed P1's entry) = -1
    // So after_p1 = after_obs - 1.
    assert_eq!(
        after_p1,
        after_obs - 1,
        "OnEnterFieldAnyone should have fired when P1 entered, granting OBS +1"
    );
}

struct TrashResidentAllyPlayedObserver {
    seen: Arc<Mutex<Vec<(String, Option<PlayerId>, Option<PermanentHandle>, String)>>>,
}

impl CardEffect for TrashResidentAllyPlayedObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        vec![Effect::on_ally_played(card)
            .name("record allied play from trash")
            .process(move |ctx| {
                let event_id = ctx
                    .event_card()
                    .and_then(|handle| ctx.game.card_data_for_handle(handle))
                    .map(|data| data.card_id.clone())
                    .unwrap_or_else(|| "<missing>".to_string());
                let observer_id = ctx
                    .game
                    .card_data_for_handle(ctx.source_card)
                    .map(|data| data.card_id.clone())
                    .unwrap_or_else(|| "<missing>".to_string());
                seen.lock().unwrap().push((
                    event_id,
                    ctx.event_source_player(),
                    ctx.source_permanent,
                    observer_id,
                ));
            })
            .build()]
    }
}

fn add_registered_card_to_trash(
    runner: &mut DebugRunner,
    player: PlayerId,
    card_id: &str,
) -> CardHandle {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_index, player, card_index);
    let handle = source.handle();
    runner.game.players[player as usize].trash.push(source);
    handle
}

#[test]
fn trash_resident_on_ally_played_observer_sees_played_subject_once() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TRASH-OBS", "Trash Observer", 3))
        .add_card(plain_digimon("ALLY", "Ally Digimon", 2))
        .add_card(plain_digimon("OPP", "Opponent Digimon", 2))
        .hand(0, &["ALLY"])
        .hand(1, &["OPP"])
        .memory(10)
        .start();
    r.register_effect(
        "TRASH-OBS",
        Arc::new(TrashResidentAllyPlayedObserver { seen: seen.clone() }),
    );
    let observer = add_registered_card_to_trash(&mut r, 0, "TRASH-OBS");

    let _ = r.play(0, 0);
    let _ = r.play(1, 0);

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.as_slice(),
        &[("ALLY".to_string(), Some(0), None, "TRASH-OBS".to_string())],
        "trash observer {:?} should see only its controller's played Digimon once",
        observer
    );
}

// ─── TEST-P1-T9 ───────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon is deleted.
struct DeletionObserverMem;
impl CardEffect for DeletionObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_any_deletion(card)
            .name("+1 when any Digimon deleted")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// Inherited observer that must read the deleted object snapshot, not the
/// carrier holding this inherited effect.
struct InheritedDeletedSnapshotObserver;
impl CardEffect for InheritedDeletedSnapshotObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .timing(EffectTiming::OnAnyDeletion)
            .name("snapshot-gated inherited OnAnyDeletion")
            .condition(|ctx| {
                let Some(snapshot) = ctx.deleted_object_snapshot() else {
                    return false;
                };
                snapshot.former_controller == 0
                    && snapshot.card_kind == CardKind::Token
                    && snapshot
                        .traits
                        .iter()
                        .any(|trait_name| trait_name == "Puppet")
                    && snapshot.cause == EventCause::OwnEffect
            })
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_any_deletion_payload_carries_deleted_snapshot_to_inherited_observer_once() {
    let mut r = DebugRunner::builder()
        .add_card(token_card("PUP-TOKEN", "Puppet Token", &["Puppet"]))
        .add_card(plain_digimon("HOST", "Host Digimon", 3))
        .add_card(plain_digimon("INH-OBS", "Inherited Observer", 3))
        .start();
    r.register_effect("INH-OBS", Arc::new(InheritedDeletedSnapshotObserver));

    let victim = r.place_on_field(0, "PUP-TOKEN", Some(0));
    let _carrier = r.place_stack(0, &["INH-OBS", "HOST"]);
    let before = r.memory();

    r.game
        .delete_permanent_with_cause(victim, ReplacementCause::OwnEffect);

    assert_eq!(
        r.memory(),
        before + 1,
        "inherited observer should fire once from deleted-object snapshot"
    );
}

#[test]
fn on_any_deletion_fires_for_battle_deletion() {
    let mut atk = plain_digimon("ATK", "Atk", 5);
    atk.level = Some(5);
    atk.dp = Some(9000);
    let mut def = plain_digimon("DEF", "Def", 5);
    def.level = Some(5);
    def.dp = Some(3000); // weaker — will be deleted

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk.clone())
        .add_card(def.clone())
        .add_card(plain_digimon("OBS", "DelObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["ATK", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(15)
        .start();
    r.register_effect("OBS", Arc::new(DeletionObserverMem));

    r.play(0, 0); // ATK → battle_area[0]
    r.play(0, 0); // OBS → battle_area[1]

    // Seed DEF directly on player 1's field (bypasses memory seesaw).
    let defender_h = r.place_on_field(1, "DEF", Some(0));

    let attacker_h = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    let before = r.memory();
    let _ = r.attack_digimon(attacker_h, defender_h, true);

    // OBS fires once (DEF deleted). Only OBS is registered against
    // OnAnyDeletion, so any memory increase relative to `before` is
    // attributable to OBS. Other new timings (EndOfBattle/EndOfAttack)
    // have nothing registered against them in this test.
    assert!(
        r.memory() > before,
        "OnAnyDeletion should have fired when DEF was deleted, granting OBS +1 (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T10 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any permanent suspends.
struct SuspendObserverMem;
impl CardEffect for SuspendObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_suspend(card)
            .name("+1 on suspend")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A CardEffect that grants +1 memory whenever any permanent unsuspends.
struct UnsuspendObserverMem;
impl CardEffect for UnsuspendObserverMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_unsuspend(card)
            .name("+1 on unsuspend")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_suspend_fires_when_permanent_suspends() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SUS", "Suspended", 3))
        .add_card(plain_digimon("OBS", "SuspendObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(SuspendObserverMem));

    // Place OBS on field (it's in hand → play it, then place SUS directly).
    r.play(0, 0); // OBS → battle_area[0]

    // Seed SUS directly on field at index 1 (bypasses memory/summoning checks).
    let sus_h = r.place_on_field(0, "SUS", Some(0));

    let before = r.memory();
    // Suspend SUS — fires OnSuspend → OBS gains +1.
    r.game_mut().suspend(sus_h);

    assert!(
        r.memory() > before,
        "OnSuspend should have fired when SUS was suspended (before={}, after={})",
        before,
        r.memory()
    );
}

#[test]
fn on_unsuspend_fires_when_permanent_unsuspends() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("SUS", "Suspended", 3))
        .add_card(plain_digimon("OBS", "UnsuspendObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(UnsuspendObserverMem));

    // Play OBS onto field.
    r.play(0, 0); // OBS → battle_area[0]

    // Seed SUS directly on field and pre-suspend it by setting the flag
    // directly (bypassing Game::suspend so OnSuspend doesn't interfere).
    let sus_h = r.place_on_field(0, "SUS", Some(0));
    r.game_mut().players[sus_h.player as usize].battle_area[sus_h.index as usize].is_suspended =
        true;

    let before = r.memory();
    // Unsuspend SUS — fires OnUnsuspend → OBS gains +1.
    r.game_mut().unsuspend(sus_h);

    assert!(
        r.memory() > before,
        "OnUnsuspend should have fired when SUS was unsuspended (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T11 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when an egg hatches.
struct HatchObsMem;
impl CardEffect for HatchObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_hatch(card)
            .name("+1 on hatch")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_hatch_fires_when_egg_hatches() {
    let mut egg = plain_digimon("EGG", "Egg", 0);
    egg.level = Some(2); // Lv.2 eggs are standard digitama

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(egg)
        .add_card(plain_digimon("OBS", "HatchObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["OBS"])
        .digitama(0, &["EGG"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(HatchObsMem));

    // Play OBS onto field so it can observe.
    r.play(0, 0); // OBS → battle_area[0]

    // No breeding-area yet.
    assert!(r.game_mut().player(0).breeding_area.is_none());

    let before = r.memory();
    let ok = r.game_mut().hatch(0);
    assert!(ok, "hatch should succeed when digitama deck is non-empty");

    assert!(
        r.memory() > before,
        "OnHatch should have fired after the egg moved to breeding (before={}, after={})",
        before,
        r.memory()
    );
}

struct OnMoveObserver;
impl CardEffect for OnMoveObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_move(card)
            .name("OnMove observer gains memory")
            .condition(|ctx| ctx.event_permanent().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

#[test]
fn on_move_fires_after_breeding_permanent_moves_to_battle() {
    let filler: Vec<&str> = vec!["FILLER"; 5];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Move Observer", 3))
        .add_card(plain_digimon("ROOKIE", "Rookie", 1))
        .add_card(plain_digimon("FILLER", "Filler", 1))
        .hand(0, &["OBS"])
        .digitama(0, &["ROOKIE"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(OnMoveObserver));

    assert_eq!(r.play(0, 0), Some(0));
    assert!(r.game.hatch(0), "hatch ROOKIE into breeding");

    let before = r.memory();
    assert!(
        r.game.move_from_breeding(0),
        "breeding permanent should move"
    );

    assert_eq!(r.memory(), before + 1);
}

struct DelayOptionNoop;
impl CardEffect for DelayOptionNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delay noop")
            .delay(DelayTrigger::EndOfYourNextTurn)
            .process(|_ctx| {})
            .build()]
    }
}

struct StandardOptionNoop;
impl CardEffect for StandardOptionNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Standard option noop")
            .option_main()
            .process(|_ctx| {})
            .build()]
    }
}

struct OnOptionPlacedObserver;
impl CardEffect for OnOptionPlacedObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_option_placed(card)
            .name("Option placed observer")
            .condition(|ctx| ctx.event_card().is_some())
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    }
}

struct OnOptionPlacedNoopObserver;
impl CardEffect for OnOptionPlacedNoopObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_option_placed(card)
            .name("Option placed noop observer")
            .condition(|ctx| ctx.event_card().is_some())
            .process(|_ctx| {})
            .build()]
    }
}

#[test]
fn on_option_placed_fires_after_delay_option_enters_battle_area() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Option Observer", 3))
        .add_card(option_card("OPT-DELAY", "Delay Option", &["Royal Knight"]))
        .hand(0, &["OBS", "OPT-DELAY"])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(OnOptionPlacedObserver));
    r.register_effect("OPT-DELAY", Arc::new(DelayOptionNoop));

    assert_eq!(r.play(0, 0), Some(0));
    r.game.enter_main_phase();
    let before = r.memory();
    let battle_before = r.battle_area_size(0);
    let _ = r.game.play_option_from_hand(0, 0);

    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "Delay option should be placed as a battle-area option permanent"
    );

    assert_eq!(r.memory(), before + 1);
}

#[test]
fn on_option_placed_does_not_fire_for_standard_option_sent_to_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Option Observer", 3))
        .add_card(option_card("OPT-STANDARD", "Standard Option", &[]))
        .hand(0, &["OBS", "OPT-STANDARD"])
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(OnOptionPlacedObserver));
    r.register_effect("OPT-STANDARD", Arc::new(StandardOptionNoop));

    assert_eq!(r.play(0, 0), Some(0));
    r.game.enter_main_phase();
    let before_memory = r.memory();
    let before_battle_area = r.battle_area_size(0);

    let result = r.game.play_option_from_hand(0, 0);

    assert_eq!(result, digimon_engine::selection::OptionPlayResult::Trashed);
    assert_eq!(
        r.memory(),
        before_memory,
        "OnOptionPlaced must not fire for standard options"
    );
    assert_eq!(
        r.battle_area_size(0),
        before_battle_area,
        "standard options must not create battle-area option permanents"
    );
    assert!(
        r.game
            .player(0)
            .trash
            .iter()
            .any(|card| card.card_id(&r.game.card_data) == "OPT-STANDARD"),
        "standard option should resolve to trash"
    );
}

#[test]
fn delay_option_cost_crossing_turn_ends_after_on_option_placed_selection_resolves() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS-A", "Observer A", 0))
        .add_card(plain_digimon("OBS-B", "Observer B", 0))
        .add_card(option_card_with_cost(
            "OPT-DELAY-COST",
            "Delay Option Cost",
            3,
            &[],
        ))
        .hand(0, &["OBS-A", "OBS-B", "OPT-DELAY-COST"])
        .memory(1)
        .start();
    r.register_effect("OBS-A", Arc::new(OnOptionPlacedNoopObserver));
    r.register_effect("OBS-B", Arc::new(OnOptionPlacedNoopObserver));
    r.register_effect("OPT-DELAY-COST", Arc::new(DelayOptionNoop));

    assert_eq!(r.play(0, 0), Some(0));
    assert_eq!(r.play(0, 0), Some(1));
    r.game.enter_main_phase();

    let result = r.game.play_option_from_hand(0, 0);
    assert!(
        r.game.pending_selection.is_some(),
        "two OnOptionPlaced observers should ask for trigger order"
    );
    assert_eq!(
        r.game.turn_player(),
        0,
        "turn should not pass until the placed-option observer selection settles"
    );
    assert_eq!(
        result,
        digimon_engine::selection::OptionPlayResult::Pending,
        "option play should pause on the trigger-order selection"
    );

    let (selecting_player, action_id) = {
        let selection = r
            .game
            .pending_selection
            .as_ref()
            .expect("trigger-order selection should be pending");
        (selection.selecting_player, selection.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("resolving trigger order should succeed");

    assert!(
        r.game.pending_selection.is_none(),
        "all placed-option observers should settle after resolving trigger order"
    );
    assert_eq!(
        r.game.turn_player(),
        1,
        "memory crossed to the opponent while playing the Delay option, so the turn should pass after observers settle"
    );
}

struct SourceTrashObserver;
impl CardEffect for SourceTrashObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("Source trash observer")
            .condition(|ctx| ctx.event_card().is_some())
            .process(|ctx| {
                assert!(ctx.event_host_card().is_some());
                assert!(ctx.event_source_card().is_some());
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct SourceTrashHostAliasGuard;
impl CardEffect for SourceTrashHostAliasGuard {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("Source trash host alias guard")
            .condition(|ctx| ctx.event_source_card().is_some())
            .process(|ctx| {
                if let Some(host) = ctx.event_host_permanent() {
                    let aliased_id = ctx.game.player(host.player).battle_area[host.index as usize]
                        .top_card()
                        .card_id(ctx.card_data());
                    assert_ne!(
                        aliased_id, "OTHER",
                        "stale source-trash host handle must not alias a shifted permanent"
                    );
                }
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_digivolution_card_trashed_context_carries_host_and_trashed_source() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Source Trash Observer", 3))
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("UNDER", "Under", 3))
        .memory(5)
        .start();
    r.register_effect("OBS", Arc::new(SourceTrashObserver));

    r.place_on_field(0, "OBS", None);
    let host = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER")
            .unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    let before = r.memory();
    assert!(
        r.game_mut().return_to_hand(host).is_some(),
        "return_to_hand should move TOP to hand and trash UNDER"
    );

    assert_eq!(r.memory(), before + 1);
}

#[test]
fn on_digivolution_card_trashed_return_to_deck_carries_host_and_trashed_source() {
    use digimon_engine::enums::StackPosition;

    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS-DECK", "Source Trash Observer", 3))
        .add_card(plain_digimon("TOP-DECK", "Top", 4))
        .add_card(plain_digimon("UNDER-DECK", "Under", 3))
        .memory(5)
        .start();
    r.register_effect("OBS-DECK", Arc::new(SourceTrashObserver));

    r.place_on_field(0, "OBS-DECK", None);
    let host = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-DECK")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TOP-DECK")
            .unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    let before = r.memory();
    assert!(
        r.game_mut().return_to_deck(host, StackPosition::Bottom),
        "return_to_deck should move TOP-DECK to deck and trash UNDER-DECK"
    );

    assert_eq!(r.memory(), before + 1);
}

#[test]
fn on_digivolution_card_trashed_de_digivolve_carries_host_and_trashed_source() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS-DEDE", "Source Trash Observer", 3))
        .add_card(plain_digimon("TOP-DEDE", "Top", 4))
        .add_card(plain_digimon("UNDER-DEDE", "Under", 3))
        .memory(5)
        .start();
    r.register_effect("OBS-DEDE", Arc::new(SourceTrashObserver));

    r.place_on_field(0, "OBS-DEDE", None);
    let host = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER-DEDE")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TOP-DEDE")
            .unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    let before = r.memory();
    assert!(
        r.game_mut().de_digivolve_from_effect(host, 0, 1),
        "de_digivolve should trash TOP-DEDE and expose UNDER-DEDE"
    );

    assert_eq!(r.memory(), before + 1);
}

#[test]
fn source_trash_host_context_does_not_alias_shifted_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS", "Source Trash Observer", 3))
        .add_card(plain_digimon("TOP", "Top", 4))
        .add_card(plain_digimon("UNDER", "Under", 3))
        .add_card(plain_digimon("OTHER", "Other", 3))
        .memory(5)
        .start();
    r.register_effect("OBS", Arc::new(SourceTrashHostAliasGuard));

    let host = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER")
            .unwrap();
        let top_idx = g.card_data.iter().position(|c| c.card_id == "TOP").unwrap();
        let under = CardSource::new(under_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(under, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };
    r.place_on_field(0, "OTHER", None);
    r.place_on_field(0, "OBS", None);

    let before = r.memory();
    assert!(r.game_mut().return_to_hand(host).is_some());

    assert_eq!(r.memory(), before + 1);
}

struct AllyAttackObserver {
    seen: Arc<Mutex<Vec<(PermanentHandle, AttackTarget)>>>,
}

impl CardEffect for AllyAttackObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        vec![Effect::on_ally_attack(card)
            .name("Ally attack observer")
            .condition(|ctx| ctx.attack_attacker().is_some() && ctx.attack_target().is_some())
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.attack_attacker().expect("attacker context"),
                    ctx.attack_target().expect("target context"),
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct OpponentAttackObserver {
    seen: Arc<Mutex<Vec<(PermanentHandle, AttackTarget)>>>,
}

impl CardEffect for OpponentAttackObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = self.seen.clone();
        vec![Effect::on_opponent_attack(card)
            .name("Opponent attack observer")
            .condition(|ctx| ctx.attack_attacker().is_some() && ctx.attack_target().is_some())
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.attack_attacker().expect("attacker context"),
                    ctx.attack_target().expect("target context"),
                ));
                ctx.gain_memory(2);
            })
            .build()]
    }
}

struct SubstituteAttackTarget(PermanentHandle);
impl CardEffect for SubstituteAttackTarget {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let replacement = self.0;
        vec![
            EffectBuilder::new(card, EffectTiming::WhenWouldBeAttackTarget)
                .name("Substitute attack target")
                .replacement_process(move |ctx| {
                    ctx.substitute(ReplacementSubject::Permanent(replacement));
                })
                .build(),
        ]
    }
}

struct OptionalNoopWhenWouldAttack;
impl CardEffect for OptionalNoopWhenWouldAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(card, EffectTiming::WhenWouldAttack)
            .name("Optional noop when would attack")
            .optional()
            .replacement_process(|_ctx| {})
            .build()]
    }
}

struct OptionalCancelWhenWouldAttack;
impl CardEffect for OptionalCancelWhenWouldAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![EffectBuilder::new(card, EffectTiming::WhenWouldAttack)
            .name("Optional cancel when would attack")
            .optional()
            .replacement_process(|ctx| {
                ctx.cancel();
            })
            .build()]
    }
}

struct OptionalSubstituteAttackTarget(PermanentHandle);
impl CardEffect for OptionalSubstituteAttackTarget {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let replacement = self.0;
        vec![
            EffectBuilder::new(card, EffectTiming::WhenWouldBeAttackTarget)
                .name("Optional substitute attack target")
                .optional()
                .replacement_process(move |ctx| {
                    ctx.substitute(ReplacementSubject::Permanent(replacement));
                })
                .build(),
        ]
    }
}

struct ReturnSelfOnAttack;
impl CardEffect for ReturnSelfOnAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_attack(card)
            .name("Return self on attack")
            .process(|ctx| {
                if let Some(source) = ctx.source_permanent {
                    ctx.game.return_to_hand(source);
                }
            })
            .build()]
    }
}

struct ReturnSelfWithTriggerOrderOnAttack;
impl CardEffect for ReturnSelfWithTriggerOrderOnAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::on_attack(card)
                .name("Return self on ordered attack trigger")
                .process(|ctx| {
                    if let Some(source) = ctx.source_permanent {
                        ctx.game.return_to_hand(source);
                    }
                })
                .build(),
            Effect::on_attack(card)
                .name("Noop ordered attack trigger")
                .process(|_ctx| {})
                .build(),
        ]
    }
}

struct AddTopSourceOnAttack;
impl CardEffect for AddTopSourceOnAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_attack(card)
            .name("Add top source on attack")
            .process(|ctx| {
                if let Some(source) = ctx.source_permanent {
                    let evo_idx = ctx
                        .game
                        .card_data
                        .iter()
                        .position(|c| c.card_id == "EVO-TOP")
                        .expect("EVO-TOP fixture");
                    let evo = CardSource::new(evo_idx, source.player, ctx.game.next_card_index());
                    ctx.game.players[source.player as usize].battle_area[source.index as usize]
                        .card_sources
                        .push(evo);
                }
            })
            .build()]
    }
}

struct ReturnAttackerOnAllyAttack;
impl CardEffect for ReturnAttackerOnAllyAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_ally_attack(card)
            .name("Return attacker on ally attack")
            .process(|ctx| {
                if let Some(attacker) = ctx.attack_attacker() {
                    ctx.game.return_to_hand(attacker);
                }
            })
            .build()]
    }
}

#[test]
fn accepted_predeclare_cancel_replacement_cancels_before_observers() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATTACKER", Arc::new(OptionalCancelWhenWouldAttack));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    let before = r.memory();

    let result = r.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::InProgress,
        "optional pre-declare replacement should park the attack"
    );
    let selecting_player = r
        .game
        .pending_selection
        .as_ref()
        .expect("replacement selection should be pending")
        .selecting_player;
    r.game.decode_action(REPLACEMENT_ACCEPT, selecting_player);

    assert!(
        r.game.pending_attack.is_none(),
        "accepting the pre-declare cancel should resume and clear the attack"
    );
    assert_eq!(
        r.memory(),
        before,
        "cancelled pre-declare attack should not fire declared-attack observers"
    );
    assert!(
        ally_seen.lock().unwrap().is_empty(),
        "declared-attack observers must not fire after accepted pre-declare cancel"
    );
}

#[test]
fn declined_predeclare_replacement_resumes_attack_declaration() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATTACKER", Arc::new(OptionalNoopWhenWouldAttack));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    let before = r.memory();

    let result = r.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::InProgress,
        "optional pre-declare replacement should park the attack"
    );
    let selecting_player = r
        .game
        .pending_selection
        .as_ref()
        .expect("replacement selection should be pending")
        .selecting_player;
    r.game.decode_action(PASS, selecting_player);

    assert!(
        r.game.pending_attack.is_none(),
        "declining the replacement should resume and finish the attack"
    );
    assert_eq!(
        r.memory(),
        before + 1,
        "resumed declaration should still fire ally attack observers"
    );
    assert_eq!(
        *ally_seen.lock().unwrap(),
        vec![(attacker, AttackTarget::Player(1))]
    );
}

#[test]
fn accepted_predeclare_target_substitution_updates_attack_context() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .add_card(plain_digimon("DEF-A", "Original Defender", 3))
        .add_card(plain_digimon("DEF-B", "Replacement Defender", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    let original = r.place_on_field(1, "DEF-A", Some(0));
    let replacement = r.place_on_field(1, "DEF-B", Some(0));
    r.register_effect(
        "DEF-A",
        Arc::new(OptionalSubstituteAttackTarget(replacement)),
    );

    let result = r.attack_digimon(attacker, original, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::InProgress,
        "optional target substitution should park the attack"
    );
    let selecting_player = r
        .game
        .pending_selection
        .as_ref()
        .expect("replacement selection should be pending")
        .selecting_player;
    r.game.decode_action(REPLACEMENT_ACCEPT, selecting_player);

    assert_eq!(
        *ally_seen.lock().unwrap(),
        vec![(attacker, AttackTarget::Digimon(replacement))],
        "accepting the target substitution should update declared-attack context"
    );
    assert!(
        r.game.pending_attack.is_none(),
        "accepted target substitution should resume and finish the attack"
    );
}

#[test]
fn attack_resume_after_trigger_order_does_not_alias_removed_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY", "Shifted Ally", 3))
        .memory(5)
        .start();
    r.register_effect("ATTACKER", Arc::new(ReturnSelfWithTriggerOrderOnAttack));

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY", None);

    let result = r.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::InProgress,
        "multiple OnAttack effects should park on TriggerOrder"
    );
    let (selecting_player, action_id) = {
        let selection = r
            .game
            .pending_selection
            .as_ref()
            .expect("trigger-order selection should be pending");
        (selection.selecting_player, selection.valid_action_ids[0])
    };
    r.game
        .resolve_selection(selecting_player, action_id)
        .expect("return-self trigger should resolve");

    assert_eq!(
        r.game.advance_pending_attack(),
        digimon_engine::combat::AttackResult::Invalid,
        "resumed attack must not continue through the shifted ally in the old slot"
    );
}

#[test]
fn declared_attack_fires_ally_and_opponent_observers_with_attack_context() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .add_card(plain_digimon("OPP-OBS", "Opponent Observer", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    let opponent_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );
    r.register_effect(
        "OPP-OBS",
        Arc::new(OpponentAttackObserver {
            seen: opponent_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    r.place_on_field(1, "OPP-OBS", None);
    let expected_target = AttackTarget::Player(1);

    let before = r.memory();
    r.attack_player(attacker, 1, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        r.memory(),
        before - 1,
        "ally observer should gain 1 and opponent observer should gain 2 for its controller at attack declaration"
    );
    assert_eq!(
        *ally_seen.lock().unwrap(),
        vec![(attacker, expected_target)]
    );
    assert_eq!(
        *opponent_seen.lock().unwrap(),
        vec![(attacker, expected_target)]
    );
}

#[test]
fn attack_target_context_reports_effective_declared_target_after_substitution() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .add_card(plain_digimon("DEF-A", "Original Defender", 3))
        .add_card(plain_digimon("DEF-B", "Replacement Defender", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    let original = r.place_on_field(1, "DEF-A", Some(0));
    let replacement = r.place_on_field(1, "DEF-B", Some(0));
    r.register_effect("DEF-A", Arc::new(SubstituteAttackTarget(replacement)));

    r.attack_digimon(attacker, original, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        *ally_seen.lock().unwrap(),
        vec![(attacker, AttackTarget::Digimon(replacement))],
        "declared-attack observers should see the live effective target after replacement"
    );
}

#[test]
fn on_ally_attack_still_fires_if_attacker_stack_changes_during_on_attack() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("EVO-TOP", "Evo Top", 4))
        .add_card(plain_digimon("ALLY-OBS", "Ally Observer", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATTACKER", Arc::new(AddTopSourceOnAttack));
    r.register_effect(
        "ALLY-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-OBS", None);
    let before = r.memory();

    r.attack_player(attacker, 1, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        r.memory(),
        before + 1,
        "same-permanent stack changes must not suppress declared ally observers"
    );
    assert_eq!(
        *ally_seen.lock().unwrap(),
        vec![(attacker, AttackTarget::Player(1))]
    );
}

#[test]
fn on_opponent_attack_does_not_fire_if_ally_observer_removes_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-REMOVE", "Ally Remove", 3))
        .add_card(plain_digimon("OPP-OBS", "Opponent Observer", 3))
        .memory(5)
        .start();
    let opponent_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ALLY-REMOVE", Arc::new(ReturnAttackerOnAllyAttack));
    r.register_effect(
        "OPP-OBS",
        Arc::new(OpponentAttackObserver {
            seen: opponent_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-REMOVE", None);
    r.place_on_field(1, "OPP-OBS", None);
    let before = r.memory();

    let result = r.attack_player(attacker, 1, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::Invalid,
        "attack should stop after an ally observer removes the attacker"
    );
    assert_eq!(
        r.memory(),
        before,
        "opponent observers must not fire after the attacker leaves during ally observer fan-out"
    );
    assert!(
        opponent_seen.lock().unwrap().is_empty(),
        "opponent observers must not receive stale attack context"
    );
}

#[test]
fn on_ally_attack_does_not_fire_if_attacker_left_during_on_attack() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER", "Attacker", 3))
        .add_card(plain_digimon("ALLY-A", "Ally A", 3))
        .add_card(plain_digimon("ALLY-B", "Ally B", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect("ATTACKER", Arc::new(ReturnSelfOnAttack));
    r.register_effect(
        "ALLY-A",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );
    r.register_effect(
        "ALLY-B",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.place_on_field(0, "ALLY-A", None);
    r.place_on_field(0, "ALLY-B", None);
    let before = r.memory();

    let result = r.attack_player(attacker, 1, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::Invalid,
        "attack should stop after the attacker leaves during OnAttack"
    );
    assert_eq!(
        r.memory(),
        before,
        "OnAllyAttack must not fire after the original attacker leaves"
    );
    assert!(
        ally_seen.lock().unwrap().is_empty(),
        "shifted battle-area slots must not receive stale attacker context"
    );
}

#[test]
fn on_ally_attack_does_not_fire_on_the_attacker_itself() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ATTACKER-OBS", "Attacker Observer", 3))
        .memory(5)
        .start();
    let ally_seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "ATTACKER-OBS",
        Arc::new(AllyAttackObserver {
            seen: ally_seen.clone(),
        }),
    );

    let attacker = r.place_on_field(0, "ATTACKER-OBS", Some(0));
    let before = r.memory();

    r.attack_player(attacker, 1, false);
    r.auto_resolve()
        .expect("attack prompts should auto-resolve");

    assert_eq!(
        r.memory(),
        before,
        "OnAllyAttack observers exclude the attacking permanent itself; use OnAttack for self"
    );
    assert!(
        ally_seen.lock().unwrap().is_empty(),
        "attacking permanent must not receive an OnAllyAttack payload"
    );
}

// ─── TEST-P1-T12 ──────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory whenever any Digimon digivolves.
struct DigivolveObsMem;
impl CardEffect for DigivolveObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolve(card)
            .name("+1 on digivolve")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A Lv.4 Red Digimon that can digivolve onto a Lv.3 Red base for free.
fn lv4_digimon(card_id: &str, name: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0, // Red
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn zero_cost_evo_card(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = plain_digimon(card_id, name, 0);
    card.level = Some(4);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 0,
    }];
    card
}

#[test]
fn game_event_digivolve_is_emitted_with_new_top_card_and_field_index() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base", 3))
        .add_card(zero_cost_evo_card("EVO", "Evolution", &[]))
        .hand(0, &["EVO"])
        .memory(10)
        .start();
    let base = r.place_on_field(0, "BASE", None);
    r.game.enter_main_phase();

    let checkpoint = r.event_checkpoint();
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));

    let events = r.events_since(checkpoint);
    assert!(
        events.iter().any(|event| matches!(
            event,
            GameEvent::Digivolve {
                player: 0,
                top_card_id,
                field_index,
                from_stack_top,
                ..
            } if top_card_id == "EVO"
                && *field_index == base.index
                && from_stack_top == "BASE"
        )),
        "digivolve should emit a GameEvent::Digivolve containing new top card and previous stack top"
    );
}

#[test]
fn on_digivolve_fires_globally_when_any_digimon_digivolves() {
    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("B3", "Base3", 3))
        .add_card(lv4_digimon("E4", "Evo4"))
        .add_card(plain_digimon("OBS", "DigObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .hand(0, &["E4", "OBS"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(10)
        .start();
    r.register_effect("OBS", Arc::new(DigivolveObsMem));

    // Place B3 directly on player 0's field (index 0).
    let base_h = r.place_on_field(0, "B3", Some(0));
    assert_eq!(base_h.index, 0);

    // Play OBS from hand (hand[1]) → battle_area[1].
    r.play(0, 1); // OBS → field

    // E4 is still at hand[0]. Digivolve E4 onto B3 (hand_index=0, target=base_h).
    let before = r.memory();
    let ok = r.game_mut().effect_initiated_digivolve(
        0,
        0, // E4 is hand[0]
        base_h,
        CostDelta::Free,
        false,
        PlaySource::ByEffect,
    );
    assert!(ok, "effect_initiated_digivolve should succeed");

    assert!(
        r.memory() > before,
        "OnDigivolve should have fired globally after E4 digivolved onto B3 (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T13 ─────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when an attack's target is redirected.
struct AtkChangeObsMem;
impl CardEffect for AtkChangeObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_attack_target_change(card)
            .name("+1 on attack target change")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_attack_target_change_fires_on_block_redirect() {
    // ATK (9000 DP) attacks player 1 directly. BLK (5000 DP, Blocker keyword)
    // on player 1's field intercepts. OnAttackTargetChange fires for OBS on
    // player 0's side when the blocker redirect rewrites effective_target.
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::enums::{Expiry, Keyword};

    let mut atk_data = plain_digimon("ATK13", "Attacker13", 5);
    atk_data.level = Some(5);
    atk_data.dp = Some(9000);

    let mut blk_data = plain_digimon("BLK13", "Blocker13", 4);
    blk_data.level = Some(4);
    blk_data.dp = Some(5000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk_data)
        .add_card(blk_data)
        .add_card(plain_digimon("OBS13", "AtkChangeObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();
    r.register_effect("OBS13", Arc::new(AtkChangeObsMem));

    // Place ATK and OBS on player 0's field directly (bypasses summoning sickness).
    let atk_h = r.place_on_field(0, "ATK13", Some(0));
    let _obs_h = r.place_on_field(0, "OBS13", Some(0));

    // Place BLK on player 1's field and grant it Blocker.
    let blk_h = r.place_on_field(1, "BLK13", Some(0));
    r.game_mut()
        .modifiers
        .grant_keyword(blk_h, Keyword::Blocker, Expiry::Permanent, 1);

    let before = r.memory();
    // Attack player 1 directly — vortex=false so the Block interrupt window
    // opens. Permanents were placed with turn_played=Some(0) so they are not
    // fresh (game is at turn 1) and summoning sickness does not apply.
    let result = r.attack_player(atk_h, 1, false);
    // Blocker present → BlockTiming installed.
    assert!(
        matches!(result, digimon_engine::combat::AttackResult::InProgress),
        "BlockTiming should be installed when Blocker is present"
    );

    // Defender resolves selection: declare the blocker (index 0 on player 1).
    let block_action = encode_attack(0, 0);
    r.game_mut()
        .resolve_selection(1, block_action)
        .expect("declaring a valid blocker must succeed");

    // OnAttackTargetChange fired during the block callback → OBS gained +1.
    assert!(
        r.memory() > before,
        "OnAttackTargetChange should have fired when Block redirected the attack (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── TEST-P1-T14 ─────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when an opponent's security card is removed.
struct OppSecRemovedObsMem;
impl CardEffect for OppSecRemovedObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_opponent_security_removed(card)
            .name("+1 when opp security removed")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_opponent_security_removed_fires_for_attacker() {
    // ATK (9000 DP) attacks player 1 directly. Player 1 has 2 security cards.
    // OnOpponentSecurityRemoved fires in player 0's battle area (where OBS lives)
    // after the security card leaves the stack.
    let mut atk_data = plain_digimon("ATK14", "Attacker14", 5);
    atk_data.level = Some(5);
    atk_data.dp = Some(9000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk_data)
        .add_card(plain_digimon("OBS14", "OppSecObs", 3))
        .add_card(plain_digimon("SEC14", "SecurityCard14", 3))
        .add_card(plain_digimon("F", "F", 1))
        .security(1, &["SEC14", "SEC14"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();
    r.register_effect("OBS14", Arc::new(OppSecRemovedObsMem));

    // Place ATK and OBS on player 0's field.
    let atk_h = r.place_on_field(0, "ATK14", Some(0));
    let _obs_h = r.place_on_field(0, "OBS14", Some(0));

    let before = r.memory();
    // Attack player 1 directly (vortex=true to bypass summoning sickness).
    // Security check will reveal and trash one card from player 1's stack.
    let _ = r.attack_player(atk_h, 1, true);

    // OnOpponentSecurityRemoved should have fired → OBS gained +1 memory.
    assert!(
        r.memory() > before,
        "OnOpponentSecurityRemoved should fire when security card leaves the stack (before={}, after={})",
        before,
        r.memory()
    );
    // Confirm one security card was actually consumed.
    assert_eq!(
        r.security_count(1),
        1,
        "one security card should have been consumed"
    );
}

// ─── TEST-P1-T14a ────────────────────────────────────────────────────────────

/// An inherited CardEffect that records opponent-security-removal payload details.
struct BreedingInheritedOppSecRemovedPayloadObs {
    seen: Arc<
        Mutex<
            Vec<(
                Option<PlayerId>,
                Option<PlayerId>,
                Option<EventCause>,
                Option<CardHandle>,
                Option<PermanentHandle>,
            )>,
        >,
    >,
}
impl CardEffect for BreedingInheritedOppSecRemovedPayloadObs {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::inherited(card)
            .name("breeding inherited opponent security removal payload")
            .timing(EffectTiming::OnOpponentSecurityRemoved)
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.event_affected_player(),
                    ctx.event_source_player(),
                    ctx.event_cause(),
                    ctx.event_card(),
                    ctx.source_permanent,
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload() {
    let mut atk_data = plain_digimon("ATK14A", "Attacker14A", 5);
    atk_data.level = Some(5);
    atk_data.dp = Some(9000);

    let filler: Vec<&str> = vec!["F14A"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk_data)
        .add_card(plain_digimon("BREED14A", "BreedingTop14A", 3))
        .add_card(plain_digimon("OBS14A", "BreedingOppSecObs14A", 3))
        .add_card(plain_digimon("SEC14A", "SecurityCard14A", 3))
        .add_card(plain_digimon("F14A", "F14A", 1))
        .security(1, &["SEC14A", "SEC14A"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();

    let seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "OBS14A",
        Arc::new(BreedingInheritedOppSecRemovedPayloadObs {
            seen: Arc::clone(&seen),
        }),
    );

    let atk_h = r.place_on_field(0, "ATK14A", Some(0));
    r.place_in_breeding(0, "BREED14A");
    add_breeding_source(&mut r, 0, "OBS14A");

    let before = r.memory();
    let _ = r.attack_player(atk_h, 1, true);

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        1,
        "breeding inherited observer should be reachable through exactly one fan-out path"
    );
    assert_eq!(
        r.memory(),
        before + 1,
        "breeding inherited observer should gain memory for its controller"
    );
    assert_eq!(seen[0].0, Some(1), "affected player is the defender");
    assert_eq!(seen[0].1, Some(0), "source player is the attacker");
    assert_eq!(seen[0].2, Some(EventCause::SecurityRemoval));
    assert!(
        seen[0].3.is_some(),
        "event_card should carry the removed security card"
    );
    assert_eq!(
        seen[0].4,
        Some(PermanentHandle {
            player: 0,
            index: BREEDING_TARGET as u8,
        }),
        "source permanent should be the stable breeding sentinel"
    );
}

// ─── TEST-P1-T14a-SECURITY-PLACEMENT ────────────────────────────────────────

/// A CardEffect that records security-placement payload details.
struct PlaceSecurityPayloadObs {
    seen: Arc<
        Mutex<
            Vec<(
                Option<PlayerId>,
                Option<PlayerId>,
                Option<EventCause>,
                Option<CardHandle>,
                Option<CardHandle>,
                usize,
                Option<(Option<Zone>, Option<Zone>, usize)>,
            )>,
        >,
    >,
}
impl CardEffect for PlaceSecurityPayloadObs {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_place_security(card)
            .name("record security placement payload")
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.event_affected_player(),
                    ctx.event_source_player(),
                    ctx.event_cause(),
                    ctx.event_card(),
                    ctx.event_source_effect()
                        .and_then(|effect| effect.source_card),
                    ctx.event_selected_results().len(),
                    ctx.event_moved_card_sets()
                        .first()
                        .map(|set| (set.from, set.to, set.cards.len())),
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

struct PlaceSecurityFromTrashOnPlay;

impl CardEffect for PlaceSecurityFromTrashOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("place trash card into security")
            .process(|ctx| {
                ctx.place_on_security(0, CardSourceRef::Trash(0, 0), StackPosition::Top, false);
            })
            .build()]
    }
}

#[test]
fn on_place_security_fires_once_with_security_placement_payload() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OBS-PLACE-SEC", "Place Security Observer", 3))
        .add_card(plain_digimon("ACTOR-PLACE-SEC", "Place Security Actor", 0))
        .add_card(plain_digimon("PLACED-SEC", "Placed Security", 3))
        .hand(0, &["ACTOR-PLACE-SEC"])
        .memory(5)
        .start();
    r.register_effect(
        "OBS-PLACE-SEC",
        Arc::new(PlaceSecurityPayloadObs { seen: seen.clone() }),
    );
    r.register_effect("ACTOR-PLACE-SEC", Arc::new(PlaceSecurityFromTrashOnPlay));

    r.place_on_field(0, "OBS-PLACE-SEC", Some(0));
    let placed_card = add_registered_card_to_trash(&mut r, 0, "PLACED-SEC");
    let source_card = r.game.players[0].hand[0].handle();

    let before = r.memory();
    assert!(
        r.play(0, 0).is_some(),
        "actor should play and place trash card into security from its OnPlay effect"
    );

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        1,
        "OnPlaceSecurity observer should be reachable through exactly one fan-out path"
    );
    assert_eq!(
        r.memory(),
        before + 1,
        "observer should gain memory after seeing the placement"
    );
    assert_eq!(seen[0].0, Some(0), "affected player receives security");
    assert_eq!(seen[0].1, Some(0), "source player controls the effect");
    assert_eq!(seen[0].2, Some(EventCause::SecurityPlacement));
    assert_eq!(seen[0].3, Some(placed_card));
    assert_eq!(
        seen[0].4,
        Some(source_card),
        "payload source_effect should identify the effect card that caused the placement"
    );
    assert_eq!(
        seen[0].5, 0,
        "security-placement event did not come from a pending selection result"
    );
    assert_eq!(
        seen[0].6,
        Some((None, Some(Zone::Security), 1)),
        "moved-card payload should describe the card placed into security"
    );
}

// ─── TEST-P1-T14b ────────────────────────────────────────────────────────────

/// A CardEffect that records own-security-removal payload details.
struct OwnSecRemovedPayloadObs {
    seen: Arc<
        Mutex<
            Vec<(
                Option<PlayerId>,
                Option<PlayerId>,
                Option<EventCause>,
                Option<CardHandle>,
            )>,
        >,
    >,
}
impl CardEffect for OwnSecRemovedPayloadObs {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_own_security_removed(card)
            .name("record own security removal payload")
            .process(move |ctx| {
                seen.lock().unwrap().push((
                    ctx.event_affected_player(),
                    ctx.event_source_player(),
                    ctx.event_cause(),
                    ctx.event_card(),
                ));
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_own_security_removed_fires_for_defender_with_battle_payload() {
    let mut atk_data = plain_digimon("ATK14B", "Attacker14B", 5);
    atk_data.level = Some(5);
    atk_data.dp = Some(9000);

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(atk_data)
        .add_card(plain_digimon("OBS14B", "OwnSecObs", 3))
        .add_card(plain_digimon("SEC14B", "SecurityCard14B", 3))
        .add_card(plain_digimon("F", "F", 1))
        .security(1, &["SEC14B", "SEC14B"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();

    let seen = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "OBS14B",
        Arc::new(OwnSecRemovedPayloadObs {
            seen: Arc::clone(&seen),
        }),
    );

    let atk_h = r.place_on_field(0, "ATK14B", Some(0));
    let _obs_h = r.place_on_field(1, "OBS14B", Some(0));

    let before = r.memory();
    let _ = r.attack_player(atk_h, 1, true);

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.len(),
        1,
        "own-security observer should fire exactly once"
    );
    assert_eq!(
        r.memory(),
        before - 1,
        "defender's observer should gain memory for its controller"
    );
    assert_eq!(seen[0].0, Some(1), "affected player is the defender");
    assert_eq!(seen[0].1, Some(0), "source player is the attacker");
    assert_eq!(seen[0].2, Some(EventCause::SecurityRemoval));
    assert!(
        seen[0].3.is_some(),
        "event_card should carry the removed security card"
    );
}

// ─── TEST-P1-T15 ─────────────────────────────────────────────────────────────

/// A CardEffect that grants +1 memory when a digivolution source is trashed.
struct DigCardTrashedObsMem;
impl CardEffect for DigCardTrashedObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_digivolution_card_trashed(card)
            .name("+1 when source trashed")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn on_digivolution_card_trashed_fires_on_return_to_hand() {
    // Seed a permanent with a 2-card stack (UNDER on bottom, TOP on top).
    // OBS on the same side observes OnDigivolutionCardTrashed.
    // Calling return_to_hand: TOP goes to hand, UNDER goes to trash.
    // OnDigivolutionCardTrashed fires once (for UNDER).
    use digimon_engine::card_source::CardSource;
    use digimon_engine::permanent::Permanent;
    use digimon_engine::permanent::PermanentHandle;

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP15", "Top15", 4))
        .add_card(plain_digimon("UNDER15", "Under15", 3))
        .add_card(plain_digimon("OBS15", "DigTrashObs", 3))
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();
    r.register_effect("OBS15", Arc::new(DigCardTrashedObsMem));

    // Place OBS on player 0's field (index 0).
    let _obs_h = r.place_on_field(0, "OBS15", Some(0));

    // Manually seed a 2-card permanent: UNDER (bottom) + TOP (top).
    let target = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER15")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TOP15")
            .unwrap();
        let i_under = g.next_card_index();
        let i_top = g.next_card_index();
        let under_src = CardSource::new(under_idx, 0, i_under);
        let top_src = CardSource::new(top_idx, 0, i_top);
        // Permanent::new takes the bottom card as its sole source; push TOP on top.
        let mut perm = Permanent::new(under_src, turn);
        perm.card_sources.push(top_src);
        g.players[0].battle_area.push(perm);
        let idx = g.players[0].battle_area.len() - 1;
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };

    let before = r.memory();
    // return_to_hand: TOP → hand, UNDER → trash, fires OnDigivolutionCardTrashed.
    let result = r.game_mut().return_to_hand(target);
    assert!(
        result.is_some(),
        "return_to_hand should succeed on a 2-card stack"
    );

    // OBS observed UNDER being trashed.
    assert!(
        r.memory() > before,
        "OnDigivolutionCardTrashed should fire when UNDER was trashed from the stack (before={}, after={})",
        before,
        r.memory()
    );
}

#[test]
fn on_digivolution_card_trashed_fires_on_return_to_deck() {
    // Same setup as return_to_hand but calls return_to_deck.
    use digimon_engine::card_source::CardSource;
    use digimon_engine::enums::StackPosition;
    use digimon_engine::permanent::Permanent;
    use digimon_engine::permanent::PermanentHandle;

    let filler: Vec<&str> = vec!["F"; 10];
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("TOP15B", "Top15B", 4))
        .add_card(plain_digimon("UNDER15B", "Under15B", 3))
        .add_card(plain_digimon("OBS15B", "DigTrashObs2", 3))
        .add_card(plain_digimon("F", "F", 1))
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();
    r.register_effect("OBS15B", Arc::new(DigCardTrashedObsMem));

    let _obs_h = r.place_on_field(0, "OBS15B", Some(0));

    let target = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let under_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "UNDER15B")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TOP15B")
            .unwrap();
        let i_under = g.next_card_index();
        let i_top = g.next_card_index();
        let under_src = CardSource::new(under_idx, 0, i_under);
        let top_src = CardSource::new(top_idx, 0, i_top);
        let mut perm = Permanent::new(under_src, turn);
        perm.card_sources.push(top_src);
        g.players[0].battle_area.push(perm);
        let idx = g.players[0].battle_area.len() - 1;
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };

    let before = r.memory();
    let ok = r.game_mut().return_to_deck(target, StackPosition::Top);
    assert!(ok, "return_to_deck should succeed on a 2-card stack");

    assert!(
        r.memory() > before,
        "OnDigivolutionCardTrashed should fire when UNDER was trashed from the stack via return_to_deck (before={}, after={})",
        before,
        r.memory()
    );
}

// ─── Task A5.1 — Tamer-host OnDigivolutionCardTrashed dispatch coverage ───────
//
// The substrate (A1-A4) trashes a face-down card stashed under a *Tamer* via
// `EffectContext::trash_bottom_face_down_source`. The Rocks-refresh work closed
// Digimon-host coverage; these tests confirm the `SourceTrashedFromStack`
// dispatch fan-out is permanent-kind-agnostic — observers must fire, and the
// stored host handle must not alias a shifted permanent, when the host is a
// Tamer rather than a Digimon.

/// A plain Tamer card (no level / DP), used as a face-down-stash host.
fn plain_tamer(card_id: &str, name: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Stash a card as the BOTTOM (face-down) digivolution source of `target`.
/// Returns the stashed source's handle.
fn stash_face_down_bottom_source(
    r: &mut DebugRunner,
    target: PermanentHandle,
    card_id: &str,
) -> CardHandle {
    let g = r.game_mut();
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("stash_face_down_bottom_source: unknown card_id {card_id}"));
    let card_idx = g.next_card_index();
    let mut src = CardSource::new(data_idx, target.player, card_idx);
    src.face_down = true;
    let handle = src.handle();
    g.players[target.player as usize].battle_area[target.index as usize]
        .card_sources
        .insert(0, src);
    handle
}

/// Captured `OnDigivolutionCardTrashed` trigger context — recorded by an
/// observer permanent so the test can assert what the dispatch threaded.
#[derive(Default)]
struct CapturedTamerTrashContext {
    fired: bool,
    event_card: Option<CardHandle>,
    event_host_permanent: Option<PermanentHandle>,
    event_host_card: Option<CardHandle>,
}

/// An observer `CardEffect` that records the `OnDigivolutionCardTrashed`
/// trigger context into a shared slot when it fires.
struct TamerTrashContextObserver {
    captured: Arc<Mutex<CapturedTamerTrashContext>>,
}
impl CardEffect for TamerTrashContextObserver {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let captured = self.captured.clone();
        vec![Effect::on_digivolution_card_trashed(card)
            .name("Tamer-host trash context observer")
            .condition(|ctx| ctx.event_card().is_some())
            .process(move |ctx| {
                let mut slot = captured.lock().expect("capture mutex poisoned");
                slot.fired = true;
                slot.event_card = ctx.event_card();
                slot.event_host_permanent = ctx.event_host_permanent();
                slot.event_host_card = ctx.event_host_card();
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A5.1 #1 — A separate own permanent (the OBSERVER) installed with an
/// `OnDigivolutionCardTrashed` effect fires when a *Tamer*-host face-down
/// source is trashed, and the `TriggerContext` it sees carries the Tamer as
/// `event_host_permanent` and the trashed stash as `event_card`. This proves
/// the `SourceTrashedFromStack` observer fan-out reaches battle-area observers
/// regardless of whether the trashed source's host is a Digimon or a Tamer.
#[test]
fn tamer_host_source_trashed_dispatches_to_observer_with_correct_context() {
    let mut r = DebugRunner::builder()
        .add_card(plain_tamer("TAMER-A5", "Tamer A5", 3))
        .add_card(plain_digimon("STASH-A5", "Stash A5", 3))
        .add_card(plain_digimon("OBS-A5", "Trash Observer A5", 3))
        .memory(5)
        .start();

    let captured = Arc::new(Mutex::new(CapturedTamerTrashContext::default()));
    r.register_effect(
        "OBS-A5",
        Arc::new(TamerTrashContextObserver {
            captured: captured.clone(),
        }),
    );

    // OBSERVER: a separate own permanent (a Digimon) on player 0's field.
    let _obs = r.place_on_field(0, "OBS-A5", Some(0));

    // HOST: a Tamer on player 0's field, with a face-down card stashed at the
    // bottom of its stack.
    let tamer = r.place_on_field(0, "TAMER-A5", Some(0));
    let stash = stash_face_down_bottom_source(&mut r, tamer, "STASH-A5");
    let tamer_own_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();

    let before = r.memory();

    // Trash the Tamer's face-down bottom source via the A4.1 substrate.
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_own_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };
    assert!(
        result,
        "trash_bottom_face_down_source should trash the Tamer's face-down stash"
    );

    let slot = captured.lock().expect("capture mutex poisoned");

    // (a) The observer fired — the dispatch fan-out reached it.
    assert!(
        slot.fired,
        "OnDigivolutionCardTrashed observer must fire when a Tamer-host source is trashed"
    );
    assert!(
        r.memory() > before,
        "observer's gain_memory side-effect should have run (before={}, after={})",
        before,
        r.memory()
    );

    // (b) The trigger context carries the trashed stash as event_card.
    assert_eq!(
        slot.event_card,
        Some(stash),
        "event_card should be the trashed face-down stash"
    );

    // (c) The host permanent is the TAMER — not a Digimon, not the trashed
    //     source — proving the dispatch threads the Tamer host correctly.
    assert_eq!(
        slot.event_host_permanent,
        Some(tamer),
        "event_host_permanent should be the Tamer the stash sat under"
    );

    // (d) The host card is the Tamer's own (top) card.
    assert_eq!(
        slot.event_host_card,
        Some(tamer_own_card),
        "event_host_card should be the Tamer's own card"
    );
    assert_ne!(
        slot.event_host_card,
        Some(stash),
        "event_host_card must NOT be the trashed source handle"
    );
}

/// An observer that asserts, when it fires, that the stored
/// `event_host_permanent` resolves to a *Tamer* (the genuine host) — never to
/// some other battle-area permanent (e.g. a Digimon left of it). Mirrors
/// `SourceTrashHostAliasGuard` but for a Tamer host.
struct TamerSourceTrashAliasGuard {
    fired: Arc<Mutex<bool>>,
}
impl CardEffect for TamerSourceTrashAliasGuard {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fired = self.fired.clone();
        vec![Effect::on_digivolution_card_trashed(card)
            .name("Tamer-host source-trash alias guard")
            .condition(|ctx| ctx.event_source_card().is_some())
            .process(move |ctx| {
                if let Some(host) = ctx.event_host_permanent() {
                    let perm = &ctx.game.player(host.player).battle_area[host.index as usize];
                    // The stored host handle must resolve to the genuine
                    // Tamer host — never alias a shifted neighbour.
                    assert_eq!(
                        perm.top_card().card_kind(ctx.card_data()),
                        CardKind::Tamer,
                        "stale source-trash host handle must resolve to the Tamer host, \
                         not a shifted permanent"
                    );
                    assert_ne!(
                        perm.top_card().card_id(ctx.card_data()),
                        "OTHER-A5",
                        "host handle must not alias the unrelated 'OTHER' permanent"
                    );
                }
                *fired.lock().unwrap() = true;
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// A5.1 #2 — No-alias edge case for a Tamer host. The `SourceTrashedFromStack`
/// trigger context stores the host `PermanentHandle` by value; when other
/// permanents sit at lower indices, a wrongly re-resolved handle would alias a
/// neighbour. Set up `[OTHER, TAMER, OBS]` on the field so the Tamer is NOT at
/// index 0, trash the Tamer's face-down source, and confirm the observer's
/// `event_host_permanent` still resolves to the Tamer (never to `OTHER`).
#[test]
fn tamer_host_source_trash_context_does_not_alias_shifted_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OTHER-A5", "Other A5", 3))
        .add_card(plain_tamer("TAMER-A5B", "Tamer A5B", 3))
        .add_card(plain_digimon("STASH-A5B", "Stash A5B", 3))
        .add_card(plain_digimon("OBS-A5B", "Alias Guard A5B", 3))
        .memory(5)
        .start();

    let fired = Arc::new(Mutex::new(false));
    r.register_effect(
        "OBS-A5B",
        Arc::new(TamerSourceTrashAliasGuard {
            fired: fired.clone(),
        }),
    );

    // Field layout: index 0 = OTHER (Digimon), index 1 = TAMER, index 2 = OBS.
    // The Tamer host therefore sits at a non-zero index — a re-resolved or
    // mis-indexed host handle would alias OTHER at index 0.
    let _other = r.place_on_field(0, "OTHER-A5", Some(0));
    let tamer = r.place_on_field(0, "TAMER-A5B", Some(0));
    let _obs = r.place_on_field(0, "OBS-A5B", Some(0));
    assert_eq!(tamer.index, 1, "precondition: Tamer host sits at index 1");

    let stash = stash_face_down_bottom_source(&mut r, tamer, "STASH-A5B");

    let before = r.memory();
    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let result = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.trash_bottom_face_down_source(tamer)
    };
    assert!(
        result,
        "trash_bottom_face_down_source should trash the Tamer's face-down stash"
    );

    // The alias guard fired and its in-process assertions (host resolves to a
    // Tamer, not to OTHER) all held.
    assert!(
        *fired.lock().unwrap(),
        "the alias-guard observer must fire on the Tamer-host source trash"
    );
    assert!(
        r.memory() > before,
        "observer side-effect should have run (before={}, after={})",
        before,
        r.memory()
    );

    // The stashed source did land in the owner's trash.
    assert_eq!(
        r.game.player(0).trash.last().map(|c| c.handle()),
        Some(stash),
        "the trashed card is the Tamer's stashed face-down source"
    );
}

// ─── Phase 2 Track D regression: inherited dispatch from deep stack ─────────

/// An inherited CardEffect that gains 1 memory on OnOpponentSecurityRemoved.
/// Records each fire's `ctx.source_card` (the resolving QueuedEffect's
/// source-card handle) so the test can assert per-source identity.
struct InheritedOppSecRemovedMemoryGain {
    fires: Arc<Mutex<Vec<CardHandle>>>,
}
impl CardEffect for InheritedOppSecRemovedMemoryGain {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let fires = Arc::clone(&self.fires);
        vec![Effect::inherited(card)
            .name("inherited stack OnOpponentSecurityRemoved memory gain")
            .timing(EffectTiming::OnOpponentSecurityRemoved)
            .process(move |ctx| {
                fires.lock().unwrap().push(ctx.source_card);
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// Track D regression: `enqueue_from_permanent` walks every below-top
/// `card_sources` slot, not just the carrier's top card. Builds a 3-card
/// stack (BOTTOM-D / MID-D / TOP-D) where BOTTOM-D and MID-D each carry an
/// inherited OnOpponentSecurityRemoved observer; the TOP-D has no effect.
/// After P0 attacks P1's security, BOTH inherited observers must fire from
/// the single carrier permanent — confirming the dispatch walks the full
/// digivolution stack (skipping only the top, which is scanned separately).
#[test]
fn inherited_stack_dispatch_fires_each_below_top_source_once() {
    use digimon_engine::card_source::CardSource;
    use digimon_engine::permanent::Permanent;

    let filler: Vec<&str> = vec!["FD"; 10];
    let mut atk_data = plain_digimon("ATK-D", "AttackerD", 5);
    atk_data.level = Some(5);
    atk_data.dp = Some(9000);

    let mut r = DebugRunner::builder()
        .add_card(atk_data)
        .add_card(plain_digimon("BOTTOM-D", "BottomD", 3))
        .add_card(plain_digimon("MID-D", "MidD", 4))
        .add_card(plain_digimon("TOP-D", "TopD", 5))
        .add_card(plain_digimon("SEC-D", "SecurityCardD", 3))
        .add_card(plain_digimon("FD", "FD", 1))
        .security(1, &["SEC-D"])
        .deck(0, &filler)
        .deck(1, &filler)
        .memory(5)
        .start();

    let bottom_fires = Arc::new(Mutex::new(Vec::new()));
    let mid_fires = Arc::new(Mutex::new(Vec::new()));
    r.register_effect(
        "BOTTOM-D",
        Arc::new(InheritedOppSecRemovedMemoryGain {
            fires: Arc::clone(&bottom_fires),
        }),
    );
    r.register_effect(
        "MID-D",
        Arc::new(InheritedOppSecRemovedMemoryGain {
            fires: Arc::clone(&mid_fires),
        }),
    );

    // Place the attacker for P0.
    let atk_h = r.place_on_field(0, "ATK-D", Some(0));

    // Build the 3-card stack as a separate carrier on P0's battle area.
    // Order: card_sources = [BOTTOM-D, MID-D, TOP-D]; top is TOP-D.
    let stack_carrier = {
        let g = r.game_mut();
        let turn = g.turn_count;
        let bottom_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BOTTOM-D")
            .unwrap();
        let mid_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "MID-D")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "TOP-D")
            .unwrap();
        let i_bottom = g.next_card_index();
        let i_mid = g.next_card_index();
        let i_top = g.next_card_index();
        let bottom_src = CardSource::new(bottom_idx, 0, i_bottom);
        let mid_src = CardSource::new(mid_idx, 0, i_mid);
        let top_src = CardSource::new(top_idx, 0, i_top);
        let mut perm = Permanent::new(bottom_src, turn);
        perm.card_sources.push(mid_src);
        perm.card_sources.push(top_src);
        g.players[0].battle_area.push(perm);
        let idx = g.players[0].battle_area.len() - 1;
        PermanentHandle {
            player: 0,
            index: idx as u8,
        }
    };

    let before = r.memory();
    let _ = r.attack_player(atk_h, 1, true);
    // Two below-top inherited observers — the drainer installs a
    // TriggerOrder prompt for the controller to pick firing order.
    // Auto-resolve consumes the prompt and fires both in queue order.
    let _ = r.auto_resolve();

    // BOTTOM-D and MID-D are both below the top slot; the dispatch must
    // walk both sources once each. The fires are recorded with the
    // queued effect's source_card so we can confirm per-source identity.
    let bottom = bottom_fires.lock().unwrap();
    let mid = mid_fires.lock().unwrap();
    assert_eq!(
        bottom.len(),
        1,
        "BOTTOM-D inherited observer must fire exactly once \
         from the digivolution-stack walk (got {} fires)",
        bottom.len()
    );
    assert_eq!(
        mid.len(),
        1,
        "MID-D inherited observer must fire exactly once \
         from the digivolution-stack walk (got {} fires)",
        mid.len()
    );

    // Confirm the source_card on each queued effect matches the stacked
    // card identity, not the carrier top card. This is the keying
    // invariant Track C relies on for OPT-slot stability.
    let stack = &r.game.players[0].battle_area[stack_carrier.index as usize].card_sources;
    assert_eq!(stack.len(), 3, "stack must remain 3-deep after attack");
    assert_eq!(
        bottom[0],
        stack[0].handle(),
        "BOTTOM-D fire must carry source_card = bottom stacked card handle, not carrier top"
    );
    assert_eq!(
        mid[0],
        stack[1].handle(),
        "MID-D fire must carry source_card = middle stacked card handle, not carrier top"
    );
    assert_ne!(
        bottom[0],
        stack[2].handle(),
        "BOTTOM-D source_card must NOT be the carrier top — Track C OPT keying relies on per-source identity"
    );

    // Memory should have grown by 2 (one per below-top inherited observer).
    assert_eq!(
        r.memory(),
        before + 2,
        "memory should grow by 2 (one per below-top inherited observer); \
         before={before}, after={}",
        r.memory()
    );
}
