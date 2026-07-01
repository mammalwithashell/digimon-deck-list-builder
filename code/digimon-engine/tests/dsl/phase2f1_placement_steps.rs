//! Phase 2f1 Task 5 — DSL behavioral tests for the placement-step variants
//! wired in Task 4: `PlaceOnSecurity`, `PlaceAsBottomSource`, `TrashTopSource`.
//!
//! Each test drives the variant through `run_step` (synchronous family) and
//! asserts the observable mutation matches the engine primitive.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledPlayerRef, CompiledStackPosition, CompiledStep,
};
use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_step, run_steps};
use digimon_engine::effect_context::EffectContext;

// ─── PlaceOnSecurity ─────────────────────────────────────────────────────────

#[test]
fn place_on_security_step_moves_hand_card_to_security_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-POS", "PlaceOnSecurityTest"))
        .hand(0, &["TST-POS"])
        .memory(5)
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();
    let security_before = runner.game.players[0].security.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);

    let step = CompiledStep::PlaceOnSecurity {
        of: CompiledPlayerRef::You,
        source: CompiledBindingRef::Named("src".into()),
        position: CompiledStackPosition::Top,
        face_up: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Hand consumed; security gained 1 card.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        security_before + 1,
        "security should gain 1 card"
    );
    // The card on top of security is the one we placed.
    // `place_on_security(StackPosition::Top, ...)` puts the card at the
    // top — by Digimon convention, the top of the security stack is the
    // last-drawn card. The engine pushes in stack-top order; verify that
    // the placed card's handle is now somewhere in security.
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .any(|c| c.handle() == src_handle),
        "the placed card must now be in the security stack"
    );
}

// ─── PlaceAsBottomSource ─────────────────────────────────────────────────────

#[test]
fn place_as_bottom_source_step_tucks_hand_card_under_target_permanent() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-PERM", "TargetPerm"))
        .add_card(make_test_card("TST-PABS", "PlaceAsBottomSourceTest"))
        .hand(0, &["TST-PABS"])
        .memory(5)
        .start();

    // Build a 1-source permanent on P0's battle area.
    let target = runner.place_on_field(0, "T-PERM", None);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);
    let original_top_h = runner.game.players[0].battle_area[0].top_card().handle();

    let src_handle = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::PlaceAsBottomSource {
        source: CompiledBindingRef::Named("src".into()),
        target: CompiledBindingRef::Named("tgt".into()),
        face_down: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Hand consumed.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1"
    );
    // The target permanent's stack grew by 1, with the hand card at the
    // bottom (card_sources[0]).
    let perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        2,
        "target permanent's stack should have grown by 1"
    );
    assert_eq!(
        perm.card_sources[0].handle(),
        src_handle,
        "the placed card must be at the BOTTOM of the stack (card_sources[0])"
    );
    assert_eq!(
        perm.top_card().handle(),
        original_top_h,
        "the original top card must still be on top"
    );
}

#[test]
fn place_as_bottom_source_uses_bound_hand_owner() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-PERM", "TargetPerm"))
        .add_card(make_test_card("P0-HAND", "P0Hand"))
        .add_card(make_test_card("P1-HAND", "P1Hand"))
        .hand(0, &["P0-HAND"])
        .hand(1, &["P1-HAND"])
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "T-PERM", None);
    let p0_hand_handle = runner.game.players[0].hand[0].handle();
    let p1_hand_handle = runner.game.players[1].hand[0].handle();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index_for("src", 1, 0);
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::PlaceAsBottomSource {
        source: CompiledBindingRef::Named("src".into()),
        target: CompiledBindingRef::Named("tgt".into()),
        face_down: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, p0_hand_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.players[0].hand.len(), 1, "P0 hand is untouched");
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|cs| cs.handle() == p0_hand_handle),
        "P0 hand card remains in hand"
    );
    assert!(
        runner.game.players[1].hand.is_empty(),
        "P1 hand source is consumed"
    );
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle(),
        p1_hand_handle,
        "P1 hand card is tucked under the P0 target"
    );
}

/// Royal Knights (BT13-093, BT23-072, EX11-053) need to place a hand card
/// as the bottom digivolution source of the breeding-area permanent (e.g.
/// [King Drasil_7D6]). The breeding permanent binding must resolve as a
/// sentinel `PermanentHandle` (`index == BREEDING_TARGET`) and the
/// `place_as_bottom_source` lowering must take that path. Proves the
/// substrate close of "hand-Main bottom-source placement".
#[test]
fn place_as_bottom_source_accepts_breeding_permanent_target_from_hand_source() {
    use digimon_engine::selection::BreedingPermanentSelectionRef;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("KING-DRASIL", "King Drasil_7D6"))
        .add_card(make_test_card("RK-CARD", "Some Royal Knight"))
        .hand(0, &["RK-CARD"])
        .memory(5)
        .start();

    let breeding = runner.place_in_breeding(0, "KING-DRASIL");
    let src_handle = runner.game.players[0].hand[0].handle();
    let hand_before = runner.game.players[0].hand.len();
    let stack_before = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .unwrap()
        .stack_size();

    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);
    bindings.insert_breeding_permanent_ref(
        "kd",
        BreedingPermanentSelectionRef {
            player: breeding.player,
            card: breeding.card,
        },
    );

    let step = CompiledStep::PlaceAsBottomSource {
        source: CompiledBindingRef::Named("src".into()),
        target: CompiledBindingRef::Named("kd".into()),
        face_down: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand card consumed"
    );
    let breeding_after = runner.game.player(0).breeding_area.as_ref().unwrap();
    assert_eq!(
        breeding_after.stack_size(),
        stack_before + 1,
        "breeding stack grew by 1"
    );
    assert_eq!(
        breeding_after.card_sources[0].handle(),
        src_handle,
        "hand card landed at the bottom of the breeding stack"
    );
}

#[test]
fn place_on_security_uses_bound_trash_owner() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("P0-TRASH", "P0Trash"))
        .add_card(make_test_card("P1-TRASH", "P1Trash"))
        .hand(0, &["SRC"])
        .memory(5)
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();
    for (player, card_id) in [(0, "P0-TRASH"), (1, "P1-TRASH")] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[player]
            .trash
            .push(CardSource::new(data_idx, player as u8, card_index));
    }
    let p0_trash_handle = runner.game.players[0].trash[0].handle();
    let p1_trash_handle = runner.game.players[1].trash[0].handle();

    let mut bindings = Bindings::new();
    bindings.insert_trash_index_for("src", 1, 0);

    let step = CompiledStep::PlaceOnSecurity {
        of: CompiledPlayerRef::You,
        source: CompiledBindingRef::Named("src".into()),
        position: CompiledStackPosition::Top,
        face_up: false,
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.players[0].trash.len(),
        1,
        "P0 trash is untouched"
    );
    assert_eq!(runner.game.players[0].trash[0].handle(), p0_trash_handle);
    assert!(
        runner.game.players[1].trash.is_empty(),
        "P1 trash source is consumed"
    );
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .any(|cs| cs.handle() == p1_trash_handle),
        "P1 trash card is placed into P0 security"
    );
}

#[test]
fn place_on_security_choice_prompt_clones_faithfully() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .memory(5)
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();
    let mut bindings = Bindings::new();
    bindings.insert_hand_index("src", 0, 0);

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        run_steps(
            &[CompiledStep::PlaceOnSecurity {
                of: CompiledPlayerRef::You,
                source: CompiledBindingRef::Named("src".into()),
                position: CompiledStackPosition::Choice,
                face_up: false,
            }],
            &mut ctx,
            &mut bindings,
        );
    }

    let top_choice = runner
        .game
        .pending_selection
        .as_ref()
        .expect("top/bottom security placement prompt should be parked")
        .valid_action_ids
        .iter()
        .copied()
        .find(|id| *id == HAND_EFFECT_START)
        .expect("top choice should be the first effect-choice action");
    assert!(
        runner.game.pending_selection_resume.is_some(),
        "top/bottom security placement prompt must be resume-driven before cloning"
    );

    let mut clone = runner.game.clone();
    clone
        .resolve_selection(0, top_choice)
        .expect("clone resolves top placement choice");

    assert!(
        runner.game.pending_selection.is_some(),
        "resolving the clone must not clear the original prompt"
    );
    assert!(
        runner.game.players[0].security.is_empty(),
        "resolving the clone must not mutate the original security"
    );

    runner
        .game
        .resolve_selection(0, top_choice)
        .expect("original resolves the same top placement choice");

    assert!(runner.game.pending_selection.is_none());
    assert!(clone.pending_selection.is_none());
    assert_eq!(
        clone.players[0].security[0].handle(),
        runner.game.players[0].security[0].handle(),
        "clone and original should place the same card after the same choice"
    );
}

// ─── PlaceTopSourceAsBottom (Phase 2 Track F) ────────────────────────────────

#[test]
fn place_top_source_as_bottom_rotates_top_stacked_card_to_bottom() {
    // Closes G-DSL-PLACE-TOP-SOURCE-AS-BOTTOM. Build a 3-source stack
    // [BASE, MID, TOP] and verify the verb rotates MID (the top stacked
    // card — the one immediately beneath TOP) to position 0 → [MID, BASE, TOP].
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "Base"))
        .add_card(make_test_card("T-MID", "Mid"))
        .add_card(make_test_card("T-TOP", "Top"))
        .start();

    let target = runner.place_on_field(0, "T-BASE", None);
    let base_h = runner.game.players[0].battle_area[0].card_sources[0].handle();
    for card_id in ["T-MID", "T-TOP"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap();
        let card_index = runner.game.next_card_index();
        runner.game.players[0].battle_area[0]
            .card_sources
            .push(CardSource::new(data_idx, 0, card_index));
    }
    let mid_h = runner.game.players[0].battle_area[0].card_sources[1].handle();
    let top_h = runner.game.players[0].battle_area[0].card_sources[2].handle();
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 3);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::PlaceTopSourceAsBottom {
        target: CompiledBindingRef::Named("tgt".into()),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    let perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(perm.stack_size(), 3, "stack size is unchanged");
    assert_eq!(
        perm.card_sources[0].handle(),
        mid_h,
        "the rotated top stacked card lives at the bottom of the stack"
    );
    assert_eq!(
        perm.card_sources[1].handle(),
        base_h,
        "the original base shifts up one slot"
    );
    assert_eq!(
        perm.card_sources[2].handle(),
        top_h,
        "the active top card stays on top"
    );
}

#[test]
fn place_top_source_as_bottom_noops_when_no_stacked_cards() {
    // Single-source permanents have no "top stacked card" beneath the
    // active top — the engine helper returns false and the stack is
    // unchanged. Callers gate the activating step with `materials_count_gte: 1`.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-SOLO", "Solo"))
        .start();

    let target = runner.place_on_field(0, "T-SOLO", None);
    let solo_h = runner.game.players[0].battle_area[0].top_card().handle();
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 1);

    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::PlaceTopSourceAsBottom {
        target: CompiledBindingRef::Named("tgt".into()),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, solo_h, Some(target), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    let perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(perm.stack_size(), 1);
    assert_eq!(perm.top_card().handle(), solo_h);
}

// ─── TrashTopSource ──────────────────────────────────────────────────────────

#[test]
fn trash_top_source_step_pops_top_source_to_controller_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-BASE", "Base"))
        .add_card(make_test_card("T-TOP", "Top"))
        .start();

    // Build a 2-source permanent on P0's battle area.
    let target = runner.place_on_field(0, "T-BASE", None);
    let base_h = runner.game.players[0].battle_area[0].card_sources[0].handle();
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "T-TOP")
        .unwrap();
    let card_index = runner.game.next_card_index();
    let top_src = CardSource::new(data_idx, 0, card_index);
    let top_h = top_src.handle();
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src);
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);

    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let trash_before = runner.game.players[0].trash.len();

    let mut bindings = Bindings::new();
    bindings.insert_permanent("tgt", target);

    let step = CompiledStep::TrashTopSource {
        target: CompiledBindingRef::Named("tgt".into()),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Stack shrank by 1 (top source removed).
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .card_sources
            .len(),
        1,
        "permanent stack should shrink by 1"
    );
    // Remaining source is the original base.
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize].card_sources[0].handle(),
        base_h,
        "remaining source must be the original base (top was removed, not bottom)"
    );
    // Trash gained the top source.
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before + 1,
        "controller's trash should gain 1 card"
    );
    assert_eq!(
        runner.game.players[0].trash[0].handle(),
        top_h,
        "trashed card must be the original top source"
    );
}
