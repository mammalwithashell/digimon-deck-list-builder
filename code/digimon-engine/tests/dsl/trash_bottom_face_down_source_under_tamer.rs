//! Task A4.2 — DSL verb `trash_bottom_face_down_source_under_tamer`.
//!
//! End-to-end coverage: a YAML `[On Play]` effect that bundles "pick one of
//! your Tamers that has a face-down stash → trash its bottom face-down
//! source". BEATBREAK / DATA SQUAD cards use this as an activation cost.
//!
//! The verb compiles to `CompiledStep::TrashBottomFaceDownSourceUnderTamer`,
//! which in `try_install` pre-filters the controller's permanents with the
//! hardcoded predicate `{ kind: tamer, has_face_down_source: true }`:
//!
//!   * ≥1 eligible Tamer → install a `select_own_permanent` selection (even a
//!     single candidate is exposed as a 1-option selection — the
//!     no-approximations policy never auto-resolves). The selection's
//!     callback trashes the picked Tamer's bottom face-down source, then runs
//!     the captured tail.
//!   * 0 eligible Tamers → the cost is unpayable; the arm returns
//!     `TailAlreadyRan` so the dispatcher stops and the tail (the rest of the
//!     clause) never runs.
//!
//! Pattern mirrors `place_as_bottom_source_deck_top.rs` (invoke a YAML card's
//! `[On Play]` process directly via `EffectContext`) and `phase2f1_end_to_end`
//! (resolve a parked selection with `resolve_selection`).

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;

/// Build a `CardData` like `make_test_card` but with the given `CardKind` so a
/// carrier's TOP card can be a Tamer rather than the default Digimon.
fn make_test_card_kind(card_id: &str, card_name: &str, kind: CardKind) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.card_kind = kind;
    d
}

/// Card YAML: an `[On Play]` clause whose process trashes a face-down source
/// from under one of the controller's Tamers, then gains 1 memory (the
/// observable tail). The clause is `optional` so test 3 — the no-eligible-Tamer
/// case — can decline cleanly at the clause level.
const YAML: &str = r#"
card: TST-TBFDSUT
name: TrashBottomFaceDownUnderTamer
kind: digimon
level: 3
color: [red]
cost: 4
dp: 3000
effects:
  - when: on_play
    optional: true
    process:
      - trash_bottom_face_down_source_under_tamer:
          of: you
      - gain_memory: 1
"#;

/// Compile the shared YAML card and return its single `[On Play]` process.
fn compiled() -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("YAML parses");
    Arc::new(digimon_dsl::compile::compile(&spec).expect("compiles cleanly"))
}

#[test]
fn trash_bottom_face_down_source_under_tamer_single_tamer_installs_one_option_selection() {
    // Seed: the effect card in P0's hand (the OnPlay source), plus exactly ONE
    // own Tamer carrying a face-down bottom source (a stash).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(
            "TST-TBFDSUT",
            "TrashBottomFaceDownUnderTamer",
        ))
        .add_card(make_test_card("STASH-SRC", "Stash Source"))
        .add_card(make_test_card_kind(
            "TAMER-TOP",
            "Hosting Tamer",
            CardKind::Tamer,
        ))
        .hand(0, &["TST-TBFDSUT"])
        .start();

    // Tamer carrier: card_sources = [STASH-SRC (bottom), TAMER-TOP (top)].
    let tamer = runner.place_stack(0, &["STASH-SRC", "TAMER-TOP"]);
    let stash_handle: CardHandle = {
        let perm = &mut runner.game.players[0].battle_area[tamer.index as usize];
        perm.card_sources[0].face_down = true;
        perm.card_sources[0].handle()
    };

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let dsl_effect = DslCardEffect::new(compiled());
    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    let memory_before = runner.game.memory;
    assert_eq!(
        runner.game.players[0].trash.len(),
        0,
        "precondition: trash empty"
    );

    // Invoke the process — installs a 1-option permanent selection and parks.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // A pending selection IS installed even with a single candidate — the
    // no-approximations policy exposes the choice to the RL action space.
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a 1-option permanent selection must be installed");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "exactly one eligible Tamer → one valid action ID"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    // The tail (gain_memory) has NOT run yet — it runs in the selection callback.
    assert_eq!(
        runner.game.memory, memory_before,
        "tail must not run before the selection is resolved"
    );

    // Resolve the selection — callback trashes the stash and runs the tail.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection succeeds");

    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection cleared after resolution"
    );

    // The face-down source was trashed: it left the Tamer's stack and is now in
    // its owner's trash.
    let perm = &runner.game.players[0].battle_area[tamer.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "Tamer stack shrank by 1 — the face-down source was popped"
    );
    assert_eq!(
        perm.card_sources[0].handle(),
        perm.top_card().handle(),
        "the Tamer retains its own card as the only remaining source"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == stash_handle),
        "the trashed face-down source must be in the controller's trash"
    );

    // The tail ran: +1 memory.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail (gain_memory: 1) must run after the selection resolves"
    );
}

#[test]
fn trash_bottom_face_down_source_under_tamer_multi_tamer_offers_both_and_trashes_only_picked() {
    // Seed: TWO own Tamers, each carrying a face-down bottom source.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(
            "TST-TBFDSUT",
            "TrashBottomFaceDownUnderTamer",
        ))
        .add_card(make_test_card("STASH-A", "Stash A"))
        .add_card(make_test_card_kind("TAMER-A", "Tamer A", CardKind::Tamer))
        .add_card(make_test_card("STASH-B", "Stash B"))
        .add_card(make_test_card_kind("TAMER-B", "Tamer B", CardKind::Tamer))
        .hand(0, &["TST-TBFDSUT"])
        .start();

    let tamer_a = runner.place_stack(0, &["STASH-A", "TAMER-A"]);
    let tamer_b = runner.place_stack(0, &["STASH-B", "TAMER-B"]);
    let stash_a: CardHandle = {
        let perm = &mut runner.game.players[0].battle_area[tamer_a.index as usize];
        perm.card_sources[0].face_down = true;
        perm.card_sources[0].handle()
    };
    let stash_b: CardHandle = {
        let perm = &mut runner.game.players[0].battle_area[tamer_b.index as usize];
        perm.card_sources[0].face_down = true;
        perm.card_sources[0].handle()
    };

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let dsl_effect = DslCardEffect::new(compiled());
    let effects = dsl_effect.effects(src_card);
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    let memory_before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // Both Tamers are offered as candidates.
    let (valid_ids, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a permanent selection must be installed");
        (pending.valid_action_ids.clone(), pending.selecting_player)
    };
    assert_eq!(
        valid_ids.len(),
        2,
        "both Tamers with a face-down stash must be selection candidates"
    );

    let action_a = digimon_engine::action::space::encode_attack(0, tamer_a.index as u16);
    let action_b = digimon_engine::action::space::encode_attack(0, tamer_b.index as u16);
    assert!(valid_ids.contains(&action_a), "Tamer A must be a candidate");
    assert!(valid_ids.contains(&action_b), "Tamer B must be a candidate");

    // Pick Tamer A.
    runner
        .game
        .resolve_selection(selecting_player, action_a)
        .expect("resolve_selection succeeds");

    assert!(
        runner.game.pending_selection.is_none(),
        "pending selection cleared after resolution"
    );

    // Tamer A's stash was trashed; Tamer B's was NOT.
    assert_eq!(
        runner.game.players[0].battle_area[tamer_a.index as usize]
            .card_sources
            .len(),
        1,
        "Tamer A's stack shrank — its face-down source was popped"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer_b.index as usize]
            .card_sources
            .len(),
        2,
        "Tamer B's stack is untouched — only the picked Tamer pays the cost"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == stash_a),
        "Tamer A's face-down source must be in trash"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == stash_b),
        "Tamer B's face-down source must NOT be in trash"
    );

    // The tail ran.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail (gain_memory: 1) must run after the selection resolves"
    );
}

#[test]
fn trash_bottom_face_down_source_under_tamer_no_eligible_tamer_aborts_clause() {
    // Seed: an own Tamer with NO face-down source (its only source is its own
    // face-up card). The cost is unpayable: nothing is trashed AND the tail
    // (gain_memory) must NOT run — the `TailAlreadyRan` behavior.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(
            "TST-TBFDSUT",
            "TrashBottomFaceDownUnderTamer",
        ))
        .add_card(make_test_card_kind(
            "TAMER-FU",
            "Tamer Without Stash",
            CardKind::Tamer,
        ))
        .hand(0, &["TST-TBFDSUT"])
        .start();

    // A Tamer placed plain: its only card_source is its own (face-up) card.
    let tamer = runner.place_on_field(0, "TAMER-FU", Some(0));
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down,
        "precondition: the Tamer's only source is face-up — no stash"
    );

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let dsl_effect = DslCardEffect::new(compiled());
    let effects = dsl_effect.effects(src_card);
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    let memory_before = runner.game.memory;
    let trash_before = runner.game.players[0].trash.len();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // No eligible Tamer → no selection installed.
    assert!(
        runner.game.pending_selection.is_none(),
        "no selection should be installed when no Tamer has a face-down stash"
    );

    // Nothing trashed.
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "an unpayable cost must trash nothing"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "the Tamer's stack is untouched"
    );

    // The tail must NOT run — the dispatcher stopped on `TailAlreadyRan`.
    assert_eq!(
        runner.game.memory, memory_before,
        "the tail (gain_memory: 1) must NOT run when the cost is unpayable"
    );
}

/// Regression test for the Issue-1 bool-guard: a filter/action desync must not
/// silently run the tail.
///
/// The eligibility filter `has_face_down_source` matches a Tamer if ANY source
/// is face-down, but `trash_bottom_face_down_source` only succeeds when
/// `card_sources[0]` (the BOTTOM) is face-down. The ST-23/ST-24 card pool cannot
/// naturally produce a Tamer whose bottom source is face-UP while a higher
/// source is face-DOWN (stashes always insert at index 0), so this test
/// FABRICATES that stack state directly: `card_sources[0]` (bottom) is face-up
/// and `card_sources[1]` is face-down. The filter therefore matches and offers
/// the Tamer, but `trash_bottom_face_down_source` returns `false`.
///
/// The install function's callback guards the tail on that `bool` AND carries a
/// `debug_assert!` that fires on the desync. The engine's tests run in debug
/// mode (no `[profile.test]` opt-level override, CI runs `cargo test` without
/// `--release`), so the `debug_assert!` is live — which is exactly the honest
/// thing to assert here: `#[should_panic]` proves the desync is caught loudly
/// rather than silently swallowed. The production `if trashed` guard (verified
/// by reading the install function) covers the graceful-degradation half: in a
/// release build the tail would be skipped, nothing trashed.
#[test]
#[should_panic(expected = "filter and action have desynced")]
fn trash_bottom_face_down_source_under_tamer_face_up_bottom_desync_does_not_run_tail() {
    // Seed: an own Tamer carrying a 3-card stack. We fabricate the desync stack
    // state below: bottom source face-UP, middle source face-DOWN.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card(
            "TST-TBFDSUT",
            "TrashBottomFaceDownUnderTamer",
        ))
        .add_card(make_test_card("BOTTOM-SRC", "Face-Up Bottom Source"))
        .add_card(make_test_card("MIDDLE-SRC", "Face-Down Middle Source"))
        .add_card(make_test_card_kind(
            "TAMER-DESYNC",
            "Desync Tamer",
            CardKind::Tamer,
        ))
        .hand(0, &["TST-TBFDSUT"])
        .start();

    // Stack: card_sources = [BOTTOM-SRC (idx 0), MIDDLE-SRC (idx 1), TAMER (top)].
    let tamer = runner.place_stack(0, &["BOTTOM-SRC", "MIDDLE-SRC", "TAMER-DESYNC"]);
    {
        let perm = &mut runner.game.players[0].battle_area[tamer.index as usize];
        // Fabricate the desync: the BOTTOM source is face-UP, a HIGHER source is
        // face-DOWN. `has_face_down_source` is true (idx 1 is face-down) so the
        // filter matches, but `card_sources[0]` is face-up so the trash fails.
        perm.card_sources[0].face_down = false;
        perm.card_sources[1].face_down = true;
    }

    let src_card: CardHandle = runner.game.players[0].hand[0].handle();
    let dsl_effect = DslCardEffect::new(compiled());
    let effects = dsl_effect.effects(src_card);
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    let memory_before = runner.game.memory;
    let trash_before = runner.game.players[0].trash.len();

    // Invoke the process — the filter matches the fabricated Tamer (it has a
    // face-down source), so a 1-option selection installs and parks.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("the filter matches the fabricated Tamer — a selection installs");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "the fabricated Tamer is the sole candidate"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    // Snapshot for the (unreachable-in-debug) graceful-degradation assertions:
    // in a release build the panic would not fire and these would hold —
    // nothing trashed, tail skipped. In debug the `debug_assert!` panics here.
    let _ = (memory_before, trash_before);

    // Resolving the selection runs the callback: `trash_bottom_face_down_source`
    // returns `false` (bottom is face-up), the `debug_assert!` fires, this
    // panics with "...filter and action have desynced" — caught by
    // `#[should_panic(expected = ...)]`. The tail (gain_memory) never runs.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection dispatches the callback");
}
