use std::sync::{Arc, Mutex};

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, CostDelta};
use digimon_engine::permanent::PermanentHandle;

struct SecurityLossAndDigivolveOrder {
    seen: Arc<Mutex<Vec<&'static str>>>,
}

impl CardEffect for SecurityLossAndDigivolveOrder {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let lose_seen = self.seen.clone();
        let digivolve_seen = self.seen.clone();
        vec![
            Effect::on_lose_security(card)
                .name("record lose security")
                .process(move |_ctx| {
                    lose_seen.lock().unwrap().push("lose_security");
                })
                .build(),
            Effect::when_digivolving(card)
                .name("record when digivolving")
                .process(move |_ctx| {
                    digivolve_seen.lock().unwrap().push("when_digivolving");
                })
                .build(),
        ]
    }
}

fn evo_lv4(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 0,
        }],
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn add_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    runner.game.players[player as usize].trash.push(card);
    handle
}

fn push_source(runner: &mut DebugRunner, target: PermanentHandle, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    let card = CardSource::new(data_idx, target.player, card_index);
    let handle = card.handle();
    runner.game.players[target.player as usize].battle_area[target.index as usize]
        .card_sources
        .push(card);
    handle
}

#[test]
fn effect_digivolve_from_trash_moves_exact_card_to_target_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("OTHER", "Other"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = add_to_trash(&mut runner, 0, "EVO4");
    let other_handle = add_to_trash(&mut runner, 0, "OTHER");
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Trash(0, 0),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    let target_perm = &runner.game.players[0].battle_area[target.index as usize];
    assert_eq!(target_perm.top_card().handle(), evo_handle);
    assert_eq!(target_perm.card_sources.len(), 2);
    assert_eq!(runner.game.players[0].trash.len(), 1);
    assert_eq!(runner.game.players[0].trash[0].handle(), other_handle);
}

#[test]
fn effect_digivolve_from_security_moves_selected_card_and_preserves_neighbors() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("SEC-C", "Security C"))
        .security(0, &["SEC-A", "EVO4", "SEC-C"])
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let low = runner.game.players[0].security[0].handle();
    let evo_handle = runner.game.players[0].security[1].handle();
    let high = runner.game.players[0].security[2].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 1),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    let security_handles: Vec<_> = runner.game.players[0]
        .security
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(security_handles, vec![low, high]);
}

#[test]
fn effect_digivolve_from_security_fires_security_loss_before_digivolve_triggers() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .security(0, &["EVO4"])
        .memory(5)
        .start();
    runner.register_effect(
        "EVO4",
        Arc::new(SecurityLossAndDigivolveOrder { seen: seen.clone() }),
    );

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = runner.game.players[0].security[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Security(0, 0),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(runner.game.players[0].security.len(), 0);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["lose_security", "when_digivolving"],
        "security-loss observer must finish before the final digivolve trigger dispatch"
    );
}

#[test]
fn effect_digivolve_from_material_moves_exact_source_out_of_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET3", "Target"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("CARRIER-TOP", "Carrier Top"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "TARGET3", None);
    let carrier = runner.place_on_field(0, "CARRIER", None);
    let evo_handle = push_source(&mut runner, carrier, "EVO4");
    let top_handle = push_source(&mut runner, carrier, "CARRIER-TOP");
    let carrier_bottom =
        runner.game.players[0].battle_area[carrier.index as usize].card_sources[0].handle();
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Material(carrier, 1),
            target,
            CostDelta::Free,
            false,
        )
    };

    assert!(ok);
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .handle(),
        evo_handle
    );
    let carrier_handles: Vec<_> = runner.game.players[0].battle_area[carrier.index as usize]
        .card_sources
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(carrier_handles, vec![carrier_bottom, top_handle]);
}

#[test]
fn failed_effect_digivolve_after_take_restores_source_zone() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("OTHER", "Other"))
        .memory(-9)
        .start();

    let target = runner.place_on_field(0, "BASE3", None);
    let evo_handle = add_to_trash(&mut runner, 0, "EVO4");
    let other_handle = add_to_trash(&mut runner, 0, "OTHER");
    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source(
            0,
            CardSourceRef::Trash(0, 0),
            target,
            CostDelta::Fixed(2),
            false,
        )
    };

    assert!(!ok);
    assert_eq!(
        runner.game.memory, -9,
        "failed payment must not move memory"
    );
    assert_eq!(
        runner.game.players[0].battle_area[target.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "BASE3",
        "target should not digivolve after the post-take failure"
    );
    let trash_handles: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(
        trash_handles,
        vec![evo_handle, other_handle],
        "taken source must be restored at its original trash index"
    );
}

/// Regression test for the panic family tracked in
/// `qa/archetype-qa/engine-gaps.md` as `G-PERMANENT-EMPTY-DURING-BATCH-DELETION`.
///
/// The gap entry hypothesizes the bug lives in the post-#525 batched-deletion
/// flow (`combat.rs::trash_single_for_batch`'s inline `drain_effect_queue`
/// firing OnDeletion handlers against a mid-trash kill_list). Replaying the
/// v4 crash recording
/// `models/generalist_v4/pilot_ppo_20260523_145133/recordings/train_env_003_game_000017_draw_crash.json`
/// against `target/debug/digimon-engine-cli replay --step 96` instead reveals
/// the real frame stack:
///
///   permanent.rs:134                Permanent::top_card                (panic site)
///   effect_queue.rs:1220            Game::top_card_handle
///   effect_queue.rs:1003            trigger_context_for_source (Digivolved arm)
///   effect_queue.rs:317             Game::enqueue_triggered
///   game_actions.rs:6438            effect_initiated_digivolve_from_source_inner  ← real bug site
///   game_actions.rs:6266            …_ignore_requirements
///   dsl_cards/step/play_digivolve.rs:480  CompiledStep::EffectInitiatedDigivolve
///
/// Root cause: `effect_initiated_digivolve_from_source_inner` calls
/// `take_card_source_ref(Material(src, i))` which uses `card_sources.remove(i)`
/// (`game_actions.rs:4249`). When the source permanent has only one card and
/// `i == 0`, the take leaves `src.card_sources` empty but the slot remains
/// in `battle_area`. The subsequent OnDigivolve fan-out iterates every
/// battle-area permanent in both players (`effect_queue.rs:307-321`) and
/// calls `top_card_handle(observer)` per observer, panicking on the now-empty
/// carrier with `Permanent must have at least one card`.
///
/// This is independent of PR #525's deletion-batching work — the panic
/// surfaces on any code path that empties a permanent's `card_sources`
/// without also removing the slot. The deletion-batching refactor merely
/// silenced the noisier `G-DELETION-RESUME-NESTED` panic; this latent bug
/// is now the dominant residual.
///
/// 41 cards use the DSL `effect_initiated_digivolve` step (BT24-016, P-103,
/// LM-027/029/030/031/032/054/055, BT21-001/013/093, EX9-012/019/032, etc.);
/// any of them will hit this path when their source binding resolves to a
/// single-card Material ref.
///
/// Expected behavior after the fix: the carrier whose last card was consumed
/// is cleared from `battle_area` (or otherwise made invisible to trigger
/// observers) BEFORE the OnDigivolve fan-out runs.
#[test]
fn effect_digivolve_from_material_emptying_source_does_not_leave_zombie_permanent() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET3", "Target"))
        .add_card(evo_lv4("EVO4"))
        .memory(5)
        .start();

    // Target: a base lv3 Digimon to digivolve onto.
    let target = runner.place_on_field(0, "TARGET3", None);
    // Source carrier: a permanent whose ONLY card is the EVO4 to move.
    // After the take, its `card_sources` is empty.
    let carrier = runner.place_on_field(0, "EVO4", None);

    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    // `ignore_requirements` mirrors the DSL `effect_initiated_digivolve` path
    // that surfaced this crash in the v4 backtrace (frames 27-29).
    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source_ignore_requirements(
            0,
            CardSourceRef::Material(carrier, 0),
            target,
            CostDelta::Free,
        )
    };

    // On current main: the call above panics inside the OnDigivolve fan-out
    // at `permanent.rs:134` before this assertion is reached. Asserting the
    // post-condition rather than `#[should_panic]` so the test continues to
    // be meaningful after the fix lands.
    assert!(ok, "digivolve should succeed");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| !p.card_sources.is_empty()),
        "no battle_area permanent should have empty card_sources after a \
         digivolve consumed a source's only card (zombie carrier visible to \
         trigger fan-out)",
    );
}

/// Variant of [`effect_digivolve_from_material_emptying_source_does_not_leave_zombie_permanent`]
/// where the source slot is at a LOWER index than the target. After the
/// soft-remove, the target's index shifts down by 1; the cleanup code
/// adjusts `target` before the OnDigivolve fan-out reads it.
#[test]
fn effect_digivolve_from_material_emptying_lower_indexed_source_shifts_target_index() {
    let mut runner = DebugRunner::builder()
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("TARGET3", "Target"))
        .memory(5)
        .start();

    // Carrier at index 0 (its only card is EVO4) — will be soft-removed.
    let carrier = runner.place_on_field(0, "EVO4", None);
    // Target at index 1 — after carrier is removed, its index becomes 0.
    let target = runner.place_on_field(0, "TARGET3", None);
    assert_eq!(carrier.index, 0);
    assert_eq!(target.index, 1);

    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    let target_top_pre = source_card;

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source_ignore_requirements(
            0,
            CardSourceRef::Material(carrier, 0),
            target,
            CostDelta::Free,
        )
    };

    assert!(ok, "digivolve should succeed");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "carrier slot removed; only target remains",
    );
    // Target should now have 2 cards: original top + the digivolved EVO4.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources.len(),
        2,
        "target should have base + digivolved card after shift",
    );
    // Verify the original target body is still under the new top.
    assert_eq!(
        runner.game.players[0].battle_area[0].card_sources[0].handle(),
        target_top_pre,
        "original target body preserved at bottom of stack",
    );
}

/// Variant where the source carrier has linked cards. They should flow to
/// trash + fire `OnLinkedCardTrashed` observers as part of the soft-remove,
/// matching `trash_single_for_batch`'s linked-card handling.
#[test]
fn effect_digivolve_from_material_emptying_source_with_linked_cards_trashes_them() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET3", "Target"))
        .add_card(evo_lv4("EVO4"))
        .add_card(make_test_card("LINK-A", "Link A"))
        .add_card(make_test_card("LINK-B", "Link B"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "TARGET3", None);
    let carrier = runner.place_on_field(0, "EVO4", None);

    // Attach two linked cards to the carrier.
    let link_a = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "LINK-A")
            .unwrap();
        let card_index = runner.game.next_card_index();
        let card = CardSource::new(data_idx, 0, card_index);
        let handle = card.handle();
        runner.game.players[0].battle_area[carrier.index as usize]
            .linked_cards
            .push(card);
        handle
    };
    let link_b = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "LINK-B")
            .unwrap();
        let card_index = runner.game.next_card_index();
        let card = CardSource::new(data_idx, 0, card_index);
        let handle = card.handle();
        runner.game.players[0].battle_area[carrier.index as usize]
            .linked_cards
            .push(card);
        handle
    };

    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source_ignore_requirements(
            0,
            CardSourceRef::Material(carrier, 0),
            target,
            CostDelta::Free,
        )
    };

    assert!(ok, "digivolve should succeed");
    let trash_handles: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.handle())
        .collect();
    assert!(
        trash_handles.contains(&link_a) && trash_handles.contains(&link_b),
        "both linked cards must flow to trash after the host's soft-remove (got trash={:?})",
        trash_handles,
    );
    // The host carrier slot is gone; no orphan linked cards on the surviving
    // target.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.linked_cards.is_empty()),
        "no battle_area permanent should still hold the soft-removed carrier's linked cards",
    );
}

/// Negative case: the existing
/// [`effect_digivolve_from_material_moves_exact_source_out_of_stack`] test
/// already covers the case where the carrier survives with sources left.
/// This variant pins the boundary: taking the BOTTOM source (index 0) when
/// the stack still has 2+ cards should leave the carrier alive — no
/// soft-remove triggered.
#[test]
fn effect_digivolve_from_material_taking_bottom_of_multi_card_stack_keeps_carrier_alive() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TARGET3", "Target"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(evo_lv4("EVO4"))
        .memory(5)
        .start();

    let target = runner.place_on_field(0, "TARGET3", None);
    let carrier = runner.place_on_field(0, "CARRIER", None);
    let evo_handle = push_source(&mut runner, carrier, "EVO4");
    // Stack: [CARRIER (bottom, idx 0), EVO4 (top, idx 1)]
    // We take index 0 (bottom), leaving the carrier alive with just EVO4.
    let carrier_top_pre =
        runner.game.players[0].battle_area[carrier.index as usize].card_sources[1].handle();
    assert_eq!(carrier_top_pre, evo_handle);

    let source_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(target), 0);
        ctx.effect_initiated_digivolve_from_source_ignore_requirements(
            0,
            CardSourceRef::Material(carrier, 0),
            target,
            CostDelta::Free,
        )
    };

    assert!(ok, "digivolve should succeed");
    // Carrier survives with 1 card (the EVO4 top, which is now also its bottom).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| !p.card_sources.is_empty()
                && p.card_sources.len() == 1
                && p.card_sources[0].handle() == evo_handle),
        "carrier should survive with just its (now-only) top card",
    );
}

/// Layer-2 regression: synthetic test that constructs a battle-area state
/// containing an artificially-emptied permanent and fires the OnDigivolve
/// fan-out manually. Before Layer 2's `top_card_handle` defensive fix, this
/// test would panic at `permanent.rs:134`. After: the zombie slot is
/// silently skipped (no `target_card` resolved for it) and the fan-out
/// completes without crashing.
#[test]
fn ondigivolve_fanout_tolerates_pre_existing_empty_permanent_in_battle_area() {
    use digimon_engine::enums::EffectTiming;
    use digimon_engine::selection::TriggerSource;

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BASE3", "Base"))
        .add_card(make_test_card("OTHER", "Other"))
        .memory(5)
        .start();

    // Place two permanents; manually empty the second one to simulate a
    // zombie left by some hypothetical buggy code path.
    let base = runner.place_on_field(0, "BASE3", None);
    let other = runner.place_on_field(0, "OTHER", None);
    runner.game.players[0].battle_area[other.index as usize]
        .card_sources
        .clear();

    let base_top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .handle();

    // Fire OnDigivolve as if `base` just digivolved. The fan-out iterates
    // every battle-area permanent on both players, including the zombie at
    // `other`. Pre-fix this panicked at the `top_card_handle` lookup.
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: base,
            card: base_top,
            effect_initiated: true,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
    // Reaching here without panic is the success criterion.
}
