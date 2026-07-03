//! G-DSL-TRASH-OPTION-FROM-SOURCES-AS-COST — DSL verb
//! `trash_option_from_own_stacks`.
//!
//! The trash-Option-from-{digivolution|link}-cards ACTIVATION cost that gates
//! BT25-085 BeelStarmon's [WD][WA][Counter] unsuspend clause: "By trashing 1
//! Option card from any of your Digimon's digivolution cards or link cards,
//! this Digimon unsuspends." (DCGO `BT25_085.cs` "Shared WD / WA / Counter":
//! `PermanentWithTrashableCard` (own battle-area Digimon whose
//! `DigivolutionOrLinkCards.Any(IsOption)`) → `SelectPermanentEffect` → a
//! `SelectCardEffect` Mode.Discard over `permanent.DigivolutionOrLinkCards`
//! filtered to `IsOption`.)
//!
//! The verb compiles to `CompiledStep::TrashOptionFromOwnStacks`, which in
//! `try_install`:
//!   * pre-filters the controller's Digimon whose digivolution cards (sources
//!     BELOW the top) OR link cards contain ≥1 Option. 0 candidates → the cost
//!     is unpayable, `cost_unpayable = true` + `TailAlreadyRan` (the dispatcher
//!     stops, the tail never runs).
//!   * ≥1 candidate → install the FIRST selection (which own Digimon). On pick,
//!     the SECOND selection (which Option among its digivolution + link cards)
//!     installs. On that pick the Option is trashed — a digivolution-source
//!     Option via `trash_specific_source_card` (fires
//!     `OnDigivolutionCardTrashed`), a link-card Option via
//!     `trash_specific_link_card` (fires `OnLinkedCardTrashed`) — and the tail
//!     runs ONLY if a card was trashed (no-approximations cost gate).
//!
//! This proves the GATE SHAPE (two selections → union-universe trash → cost-
//! gated tail) plus the per-zone observer routing; it does NOT author BT25-085
//! (whose unsuspend body is `unsuspend_self`, expressible on its own). Both
//! selections are clone-safe (resumable-VM frames: the first parks a
//! `FieldPermanent{post: SelectAndTrashStackOption}`, the second a
//! `TrashOptionFromStackSelection`).

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, EffectTiming};

/// A Digimon `CardData` — the cost source and the hosts.
fn digimon(id: &str) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(4);
    d.dp = Some(4000);
    d
}

/// A plain Option — attached as a digivolution source or link card.
fn option_card(id: &str) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Option;
    d.level = None;
    d.dp = None;
    d
}

/// Card YAML: an `[On Play]` clause that pays the stack-Option-trash cost, then
/// gains 1 memory (the observable cost-gated tail). Faithful BT25-085 would use
/// `unsuspend_self` for the tail; a memory bump is a simpler observable here.
const YAML: &str = r#"
card: TST-TOFOS
name: TrashOptionFromOwnStacks
kind: digimon
level: 4
color: [purple]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - trash_option_from_own_stacks:
          of: you
      - gain_memory: 1
"#;

/// Same, but with declinable picks (`optional: true`).
const YAML_OPTIONAL: &str = r#"
card: TST-TOFOS-OPT
name: TrashOptionFromOwnStacksOptional
kind: digimon
level: 4
color: [purple]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - trash_option_from_own_stacks:
          of: you
          optional: true
      - gain_memory: 1
"#;

fn compiled(yaml: &str) -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    Arc::new(digimon_dsl::compile::compile(&spec).expect("compiles cleanly"))
}

/// Observer that bumps a counter for `OnDigivolutionCardTrashed`.
struct OnSourceTrashedWitness(Arc<Mutex<u32>>);
impl CardEffect for OnSourceTrashedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnDigivolutionCardTrashed witness")
            .timing(EffectTiming::OnDigivolutionCardTrashed)
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

/// Observer that bumps a counter for `OnLinkedCardTrashed`.
struct OnLinkedTrashedWitness(Arc<Mutex<u32>>);
impl CardEffect for OnLinkedTrashedWitness {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let slot = self.0.clone();
        vec![Effect::on_play(card)
            .name("OnLinkedCardTrashed witness")
            .timing(EffectTiming::OnLinkedCardTrashed)
            .process(move |_ctx| {
                *slot.lock().unwrap() += 1;
            })
            .build()]
    }
}

fn run_on_play(runner: &mut DebugRunner, yaml: &str, src_card: CardHandle) {
    let dsl_effect = DslCardEffect::new(compiled(yaml));
    let effects = dsl_effect.effects(src_card);
    let process = effects[0].process.as_ref().expect("OnPlay has a process");
    let mut ctx =
        digimon_engine::effect_context::EffectContext::new(&mut runner.game, src_card, None, 0);
    process(&mut ctx);
}

// ══════════════════════════════════════════════════════════════════════════════
// Payable — trash a DIGIVOLUTION-SOURCE Option (fires OnDigivolutionCardTrashed)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn payable_trash_digivolution_source_option_fires_onsource_and_runs_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        .add_card(digimon("BASE-3"))
        .add_card(digimon("TOP-4"))
        .add_card(digimon("PLAIN-HOST"))
        .add_card(digimon("WITNESS"))
        .add_card(option_card("STACK-OPT"))
        .hand(0, &["TST-TOFOS"])
        .start();
    let witness = Arc::new(Mutex::new(0u32));
    runner.register_effect("WITNESS", Arc::new(OnSourceTrashedWitness(witness.clone())));

    // HOST carries a [TS-less] Option in its digivolution stack (below the top).
    // stack: bottom = STACK-OPT (Option), then BASE-3, top = TOP-4.
    let host = runner.place_stack(0, &["STACK-OPT", "BASE-3", "TOP-4"]);
    // A second own Digimon with NO Option in its stack/links — not a candidate.
    runner.place_on_field(0, "PLAIN-HOST", Some(0));
    runner.place_on_field(0, "WITNESS", Some(0));

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    // FIRST selection: pick which own Digimon (only HOST qualifies).
    let (first_ids, first_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first selection (which own Digimon) installs");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    assert_eq!(
        first_ids.len(),
        1,
        "only the own Digimon carrying a stack/link Option is offered"
    );
    assert_eq!(runner.game.memory, memory_before, "tail deferred until cost paid");

    let pick_host = digimon_engine::action::space::encode_attack(0, host.index as u16);
    assert!(first_ids.contains(&pick_host), "HOST is the valid first pick");
    runner
        .game
        .resolve_selection(first_player, pick_host)
        .expect("resolve first selection");

    // SECOND selection: which Option among HOST's digivolution + link cards.
    // HOST has exactly one Option (the digivolution source).
    let (second_ids, second_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second selection (which Option) installs");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    assert_eq!(
        second_ids.len(),
        1,
        "HOST has exactly 1 Option among its digivolution + link cards"
    );
    assert_eq!(runner.game.memory, memory_before, "tail still deferred");

    let trash_id = digimon_engine::action::space::HAND_EFFECT_START;
    assert!(second_ids.contains(&trash_id), "the stack Option is valid");
    runner
        .game
        .resolve_selection(second_player, trash_id)
        .expect("resolve second selection");

    assert!(runner.game.pending_selection.is_none(), "both selections resolved");

    // The Option left the host's digivolution stack and is in the controller's
    // trash. The host and its remaining stack survive.
    let host_perm = &runner.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        host_perm.card_sources.len(),
        2,
        "HOST lost the digivolution-source Option (3 → 2)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "STACK-OPT"),
        "the trashed Option is in the controller's trash"
    );
    // OnDigivolutionCardTrashed fired (a digivolution-source trash, NOT a link).
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "trashing a digivolution-source Option fires OnDigivolutionCardTrashed once"
    );
    // The cost-gated tail ran.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "the tail (gain_memory: 1) runs after the cost is paid"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Payable — trash a LINK-CARD Option (fires OnLinkedCardTrashed)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn payable_trash_link_card_option_fires_onlinked_and_runs_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        .add_card(digimon("HOST-A"))
        .add_card(digimon("WITNESS"))
        .add_card(option_card("LINK-OPT"))
        .hand(0, &["TST-TOFOS"])
        .start();
    let witness = Arc::new(Mutex::new(0u32));
    runner.register_effect("WITNESS", Arc::new(OnLinkedTrashedWitness(witness.clone())));

    let host_a = runner.place_on_field(0, "HOST-A", Some(0));
    runner.place_on_field(0, "WITNESS", Some(0));
    // A linked Option on HOST-A — the link-card leg of the union.
    let link_opt = runner.push_linked_owned(host_a, "LINK-OPT", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    // FIRST selection → pick HOST-A.
    let (first_ids, first_player) = {
        let p = runner.game.pending_selection.as_ref().expect("first selection");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    assert_eq!(first_ids.len(), 1, "only HOST-A carries a link Option");
    let pick_a = digimon_engine::action::space::encode_attack(0, host_a.index as u16);
    runner
        .game
        .resolve_selection(first_player, pick_a)
        .expect("resolve first");

    // SECOND selection → trash the link Option.
    let (second_ids, second_player) = {
        let p = runner.game.pending_selection.as_ref().expect("second selection");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    assert_eq!(second_ids.len(), 1, "HOST-A has 1 link Option");
    let trash_id = digimon_engine::action::space::HAND_EFFECT_START;
    runner
        .game
        .resolve_selection(second_player, trash_id)
        .expect("resolve second");

    assert!(runner.game.pending_selection.is_none(), "resolved");

    // The link Option left HOST-A and is in the controller's trash.
    let host_perm = &runner.game.players[0].battle_area[host_a.index as usize];
    assert_eq!(host_perm.linked_cards.len(), 0, "HOST-A lost its link Option");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == link_opt),
        "the trashed link Option is in the controller's trash"
    );
    // OnLinkedCardTrashed fired (a link-card trash, NOT a digivolution source).
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "trashing a link-card Option fires OnLinkedCardTrashed once"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "the tail runs after the cost is paid"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Union universe — a host carrying BOTH a digivolution Option and a link Option
// offers 2 second-pick options.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn second_selection_spans_union_of_digivolution_and_link_options() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        .add_card(digimon("BASE-3"))
        .add_card(digimon("TOP-4"))
        .add_card(option_card("STACK-OPT"))
        .add_card(option_card("LINK-OPT"))
        .hand(0, &["TST-TOFOS"])
        .start();

    // HOST carries a digivolution-source Option AND a link Option.
    let host = runner.place_stack(0, &["STACK-OPT", "BASE-3", "TOP-4"]);
    let _link = runner.push_linked_owned(host, "LINK-OPT", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    run_on_play(&mut runner, YAML, src_card);

    let first_player = runner.game.pending_selection.as_ref().unwrap().selecting_player;
    let pick_host = digimon_engine::action::space::encode_attack(0, host.index as u16);
    runner
        .game
        .resolve_selection(first_player, pick_host)
        .expect("resolve first");

    let second_ids = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second selection")
        .valid_action_ids
        .clone();
    assert_eq!(
        second_ids.len(),
        2,
        "the union of digivolution Options (1) + link Options (1) is offered"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Non-Option sources are excluded (a non-Option digivolution source is NOT a
// candidate, and a stack with only non-Option sources is not a first-pick).
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn non_option_sources_are_excluded() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        // A Digimon carrying only a (non-Option) Digimon digivolution source.
        .add_card(digimon("BASE-3"))
        .add_card(digimon("TOP-4"))
        .hand(0, &["TST-TOFOS"])
        .start();
    // stack: bottom = BASE-3 (Digimon source, NOT an Option), top = TOP-4.
    runner.place_stack(0, &["BASE-3", "TOP-4"]);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    // No own Digimon has an Option in its stack/links → unpayable → no selection,
    // tail skipped.
    assert!(
        runner.game.pending_selection.is_none(),
        "unpayable cost installs no selection (no Option sources)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the cost is unpayable"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Unpayable — no own Digimon carries any stack/link Option → abort clause.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn unpayable_when_no_own_digimon_has_option_sources_aborts_clause() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        .add_card(digimon("PLAIN-HOST"))
        .hand(0, &["TST-TOFOS"])
        .start();
    // An own Digimon with no digivolution/link Options — not a candidate.
    runner.place_on_field(0, "PLAIN-HOST", Some(0));

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    assert!(
        runner.game.pending_selection.is_none(),
        "unpayable cost installs no selection"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the cost is unpayable"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Optional decline on the first pick skips the trash and the tail.
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn optional_decline_on_first_pick_skips_trash_and_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS-OPT"))
        .add_card(digimon("BASE-3"))
        .add_card(digimon("TOP-4"))
        .add_card(option_card("STACK-OPT"))
        .hand(0, &["TST-TOFOS-OPT"])
        .start();
    let host = runner.place_stack(0, &["STACK-OPT", "BASE-3", "TOP-4"]);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML_OPTIONAL, src_card);

    let (is_optional, selecting_player) = {
        let p = runner.game.pending_selection.as_ref().expect("first selection");
        (p.is_optional, p.selecting_player)
    };
    assert!(is_optional, "optional first pick is declinable");
    runner
        .game
        .resolve_selection(selecting_player, digimon_engine::action::space::PASS)
        .expect("decline first pick");

    assert!(runner.game.pending_selection.is_none(), "declined; resolved");
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        3,
        "declining trashes no Option (stack intact)"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the cost is declined"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Clone-safety across both selections (resumable VM, no closure-only park panic).
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn clone_safe_across_both_selections() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TOFOS"))
        .add_card(digimon("BASE-3"))
        .add_card(digimon("TOP-4"))
        .add_card(option_card("STACK-OPT"))
        .hand(0, &["TST-TOFOS"])
        .start();
    let host = runner.place_stack(0, &["STACK-OPT", "BASE-3", "TOP-4"]);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    run_on_play(&mut runner, YAML, src_card);

    // Clone AFTER the first selection is parked.
    let mut cloned = runner.game.clone();

    let first_player = cloned.pending_selection.as_ref().unwrap().selecting_player;
    let pick_host = digimon_engine::action::space::encode_attack(0, host.index as u16);
    cloned
        .resolve_selection(first_player, pick_host)
        .expect("clone resolves first selection");
    let second_player = cloned.pending_selection.as_ref().unwrap().selecting_player;
    let trash_id = digimon_engine::action::space::HAND_EFFECT_START;
    cloned
        .resolve_selection(second_player, trash_id)
        .expect("clone resolves second selection");

    assert!(cloned.pending_selection.is_none(), "clone fully resolved");
    assert!(
        cloned.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&cloned.card_data) == "STACK-OPT"),
        "clone trashed the Option via the resumable VM"
    );
    assert_eq!(cloned.memory, memory_before + 1, "clone ran the cost-gated tail");
}
