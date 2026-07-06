//! G-DSL-LINK-TRASH-AS-COST — DSL verb `trash_link_card_of_own_digimon`.
//!
//! The link-card-trash ACTIVATION cost driving BT25-073 Dragomon's Main clause:
//! "By trashing 1 of your Digimon's link cards, you may play or use 1 [TS] cost
//! ≤5 card from hand free." (DCGO `BT25_073.cs`: `SelectPermanentEffect`
//! (`!HasNoLinkCards`) → `SelectCardEffect Root.LinkedCards` →
//! `TrashLinkCardsAndProcessAccordingToResult` → `successProcess`.)
//!
//! The verb compiles to `CompiledStep::TrashLinkCardOfOwnDigimon`, which in
//! `try_install`:
//!   * pre-filters the controller's Digimon carrying ≥1 link card. 0 candidates
//!     → the cost is unpayable, `cost_unpayable = true` + `TailAlreadyRan` (the
//!     dispatcher stops, the tail never runs).
//!   * ≥1 candidate → install the FIRST selection (which own Digimon). On pick,
//!     the SECOND selection (which of ITS link cards) installs. On that pick the
//!     link card is trashed (`trash_specific_link_card` fires
//!     `OnLinkedCardTrashed`) and the tail runs ONLY if a card was trashed
//!     (no-approximations cost gate).
//!
//! This proves the GATE SHAPE (two selections → trash → cost-gated tail); it
//! does NOT author BT25-073 (whose body is a play/use-free — expressible on its
//! own). Both selections are clone-safe (resumable-VM frames: the first parks a
//! `FieldPermanent{post: SelectAndTrashLinkCard}`, the second a
//! `TrashLinkCardOfDigimonSelection`).

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

/// A Digimon `CardData` (the default kind) — the cost source and the hosts.
fn digimon(id: &str) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(4);
    d.dp = Some(4000);
    d
}

/// A plain Option — the link card attached onto a host via `push_linked_owned`.
fn link_option(id: &str) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Option;
    d.level = None;
    d.dp = None;
    d
}

/// Card YAML: an `[On Play]` clause that pays the link-card-trash cost, then
/// gains 1 memory (the observable cost-gated tail). Clause-level `optional` so
/// the no-candidate test can decline cleanly if it ever installs (it does not).
const YAML: &str = r#"
card: TST-TLCOD
name: TrashLinkCardOfOwnDigimon
kind: digimon
level: 4
color: [black]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - trash_link_card_of_own_digimon:
          of: you
      - gain_memory: 1
"#;

/// Same, but with declinable picks (`optional: true`).
const YAML_OPTIONAL: &str = r#"
card: TST-TLCOD-OPT
name: TrashLinkCardOfOwnDigimonOptional
kind: digimon
level: 4
color: [black]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - trash_link_card_of_own_digimon:
          of: you
          optional: true
      - gain_memory: 1
"#;

fn compiled(yaml: &str) -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("YAML parses");
    Arc::new(digimon_dsl::compile::compile(&spec).expect("compiles cleanly"))
}

/// Observer that bumps a counter when `OnLinkedCardTrashed` fires.
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

#[test]
fn payable_two_selections_trash_fires_onlinked_and_runs_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TLCOD"))
        .add_card(digimon("HOST-A"))
        .add_card(digimon("HOST-B"))
        .add_card(digimon("WITNESS"))
        .add_card(link_option("LINK-A1"))
        .add_card(link_option("LINK-A2"))
        .add_card(link_option("LINK-B1"))
        .hand(0, &["TST-TLCOD"])
        .start();
    // Register the OnLinkedCardTrashed witness before placing the bystander.
    let witness = Arc::new(Mutex::new(0u32));
    runner.register_effect("WITNESS", Arc::new(OnLinkedTrashedWitness(witness.clone())));

    // Two own Digimon each carry link cards; a bystander witnesses OnLinkedCardTrashed.
    let host_a = runner.place_on_field(0, "HOST-A", Some(0));
    let host_b = runner.place_on_field(0, "HOST-B", Some(0));
    runner.place_on_field(0, "WITNESS", Some(0));

    let link_a1 = runner.push_linked_owned(host_a, "LINK-A1", 0);
    let _link_a2 = runner.push_linked_owned(host_a, "LINK-A2", 0);
    let _link_b1 = runner.push_linked_owned(host_b, "LINK-B1", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    // FIRST selection: pick which own Digimon (both HOST-A and HOST-B qualify).
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
        2,
        "both own Digimon carrying link cards are offered"
    );
    // Tail has NOT run yet.
    assert_eq!(runner.game.memory, memory_before, "tail deferred until cost paid");

    // Pick HOST-A (encode_attack(0, host_a.index)). The valid_action_ids are the
    // two field targets; choose the one decoding to host_a.
    let pick_a = digimon_engine::action::space::encode_attack(0, host_a.index as u16);
    assert!(first_ids.contains(&pick_a), "HOST-A is a valid first pick");
    runner
        .game
        .resolve_selection(first_player, pick_a)
        .expect("resolve first selection");

    // SECOND selection: which of HOST-A's TWO link cards to trash.
    let (second_ids, second_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second selection (which link card) installs");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    assert_eq!(
        second_ids.len(),
        2,
        "HOST-A has 2 link cards → 2 options for the link-card trash"
    );
    assert_eq!(runner.game.memory, memory_before, "tail still deferred");

    // Trash the FIRST link card (index 0 → HAND_EFFECT_START).
    let trash_id = digimon_engine::action::space::HAND_EFFECT_START;
    assert!(second_ids.contains(&trash_id), "link-card option 0 valid");
    runner
        .game
        .resolve_selection(second_player, trash_id)
        .expect("resolve second selection");

    assert!(
        runner.game.pending_selection.is_none(),
        "both selections resolved"
    );

    // The chosen link card left HOST-A and is in the controller's trash.
    let host_a_perm = &runner.game.players[0].battle_area[host_a.index as usize];
    assert_eq!(
        host_a_perm.linked_cards.len(),
        1,
        "HOST-A lost exactly one link card (2 → 1)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == link_a1),
        "the trashed link card is in the controller's trash"
    );
    // HOST-B untouched.
    assert_eq!(
        runner.game.players[0].battle_area[host_b.index as usize]
            .linked_cards
            .len(),
        1,
        "the un-chosen host keeps its link card"
    );
    // OnLinkedCardTrashed fired.
    assert_eq!(
        *witness.lock().unwrap(),
        1,
        "trashing a link card fires OnLinkedCardTrashed once"
    );
    // The cost-gated tail ran.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "the tail (gain_memory: 1) runs after the cost is paid"
    );
}

#[test]
fn unpayable_when_no_own_digimon_has_link_cards_aborts_clause() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TLCOD"))
        .add_card(digimon("PLAIN-HOST"))
        .hand(0, &["TST-TLCOD"])
        .start();
    // An own Digimon with NO link cards — not a candidate.
    runner.place_on_field(0, "PLAIN-HOST", Some(0));

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML, src_card);

    // No candidate → no selection installs, cost is unpayable, tail is skipped.
    assert!(
        runner.game.pending_selection.is_none(),
        "unpayable cost installs no selection"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the cost is unpayable"
    );
}

#[test]
fn optional_decline_on_first_pick_skips_trash_and_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TLCOD-OPT"))
        .add_card(digimon("HOST-A"))
        .add_card(link_option("LINK-A1"))
        .hand(0, &["TST-TLCOD-OPT"])
        .start();
    let host_a = runner.place_on_field(0, "HOST-A", Some(0));
    let _link = runner.push_linked_owned(host_a, "LINK-A1", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML_OPTIONAL, src_card);

    // Optional first pick is declinable (PASS resolves an `is_optional`
    // selection). Decline → no trash, no tail.
    let (is_optional, selecting_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first selection installs");
        (p.is_optional, p.selecting_player)
    };
    assert!(is_optional, "optional first pick is declinable");
    runner
        .game
        .resolve_selection(selecting_player, digimon_engine::action::space::PASS)
        .expect("decline first pick");

    assert!(
        runner.game.pending_selection.is_none(),
        "declining resolves the clause with no further selection"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        1,
        "declining trashes no link card"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the cost is declined"
    );
}

#[test]
fn optional_decline_on_second_pick_skips_trash_and_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TLCOD-OPT"))
        .add_card(digimon("HOST-A"))
        .add_card(link_option("LINK-A1"))
        .hand(0, &["TST-TLCOD-OPT"])
        .start();
    let host_a = runner.place_on_field(0, "HOST-A", Some(0));
    let _link = runner.push_linked_owned(host_a, "LINK-A1", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    run_on_play(&mut runner, YAML_OPTIONAL, src_card);

    // ACCEPT the first pick (which own Digimon).
    let (first_ids, first_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("first selection installs");
        (p.valid_action_ids.clone(), p.selecting_player)
    };
    let pick_a = digimon_engine::action::space::encode_attack(0, host_a.index as u16);
    assert!(first_ids.contains(&pick_a), "HOST-A is a valid first pick");
    runner
        .game
        .resolve_selection(first_player, pick_a)
        .expect("accept first pick");

    // The SECOND pick (which link card) must ALSO be declinable — DCGO
    // BT25_073.cs:99 sets `canNoSelect: () => true` on the link-card
    // SelectCardEffect. Regression: the installer wired `on_decline` but
    // never flipped `pending.is_optional`, so PASS was rejected and the
    // pick was silently mandatory (fixed 2026-07-04).
    let (is_optional, second_player) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("second selection installs");
        (p.is_optional, p.selecting_player)
    };
    assert!(is_optional, "optional second pick is declinable");
    runner
        .game
        .resolve_selection(second_player, digimon_engine::action::space::PASS)
        .expect("decline second pick");

    assert!(
        runner.game.pending_selection.is_none(),
        "declining the second pick resolves the clause with no further selection"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host_a.index as usize]
            .linked_cards
            .len(),
        1,
        "declining trashes no link card"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail must NOT run when the link-card trash is declined"
    );
}

/// Clone-safety: after the FIRST selection installs, the game is `Clone`-able and
/// the clone resolves both selections through the resumable VM (no closure-only
/// park panics). Pins the make-engine-cloneable constraint for this cost step.
#[test]
fn clone_safe_across_both_selections() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("TST-TLCOD"))
        .add_card(digimon("HOST-A"))
        .add_card(link_option("LINK-A1"))
        .hand(0, &["TST-TLCOD"])
        .start();
    let host_a = runner.place_on_field(0, "HOST-A", Some(0));
    let link_a1 = runner.push_linked_owned(host_a, "LINK-A1", 0);

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;
    run_on_play(&mut runner, YAML, src_card);

    // Clone AFTER the first selection is parked — exercises PendingSelection::clone
    // + the resume stack (a closure-only park would clone to a panic-stub).
    let mut cloned = runner.game.clone();

    // Drive the clone: first pick HOST-A, second pick its link card.
    let first_player = cloned.pending_selection.as_ref().unwrap().selecting_player;
    let pick_a = digimon_engine::action::space::encode_attack(0, host_a.index as u16);
    cloned
        .resolve_selection(first_player, pick_a)
        .expect("clone resolves first selection");
    let second_player = cloned.pending_selection.as_ref().unwrap().selecting_player;
    let trash_id = digimon_engine::action::space::HAND_EFFECT_START;
    cloned
        .resolve_selection(second_player, trash_id)
        .expect("clone resolves second selection");

    assert!(cloned.pending_selection.is_none(), "clone fully resolved");
    assert!(
        cloned.players[0].trash.iter().any(|c| c.handle() == link_a1),
        "clone trashed the link card via the resumable VM"
    );
    assert_eq!(
        cloned.memory,
        memory_before + 1,
        "clone ran the cost-gated tail"
    );
}
