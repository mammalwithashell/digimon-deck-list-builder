//! Cluster D — trigger activation site (where must a card be for its trigger to
//! live: trash for `[On Deletion]`, battle area for `[All Turns]`).
//!
//! Questions (see `card-resolution.md`):
//!   Q9  Mastemon (BT23-102) trashes Gatomon (BT15-037) from security; Gatomon
//!       plays out only after Mastemon resolves and is NOT in the battle area
//!       during security removal — judge: after both trashed, NO memory.
//!   Q19 Eyesmon: Scatter Mode (BT7-069) returned to hand by Calling From the
//!       Darkness (BT7-107) ⇒ no `[On Deletion]` — judge: 0 draws.
//!   Q20 Eyesmon stays in trash (+Pumpkinmon BT2-076) ⇒ all `[On Deletion]`
//!       fire — judge: 8 draws.
//!   Q21 Eyesmon played from trash by Back for Revenge! (BT3-109) ⇒ remaining
//!       `[On Deletion]` can't fire — judge: 0 draws.
//!   Q23 Proganomon (EX8-051)+Tumblemon (EX8-005) — only the Tumblemon
//!       remaining in trash gives its inherited `[On Deletion]` — judge: 1
//!       memory.  [READY: EX8-051/EX8-005/BT24-017 all impl]
//!
//! Scenarios authored under tasks §6.

#![allow(unused_imports)]

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
use std::sync::Arc;

/// Medusamon (BT24-017) analog driver: in ONE resolving effect body, trash 3
/// digivolution sources (Proganomon's Fragment <3>) then return 2 of the trashed
/// cards to the deck (Medusamon's "return up to 2"). Used by the Q23 test to
/// exercise the multi-source trash → TriggerOrder-parking → remain-in-trash
/// re-evaluation path without driving the full Medusamon multi-selection UI chain.
struct TrashThreeReturnTwo {
    host: PermanentHandle,
    return_cards: Vec<CardHandle>,
}

impl CardEffect for TrashThreeReturnTwo {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let host = self.host;
        let return_cards = self.return_cards.clone();
        vec![Effect::end_of_your_turn(card)
            .name("trash 3 sources then return 2 (Medusamon analog)")
            .process(move |ctx| {
                ctx.trash_bottom_sources(host, 3);
                ctx.return_trash_cards_to_deck_bottom(0, &return_cards);
            })
            .build()]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cluster-D/F ROOT-RULE PROBE — "remain-in-trash to resolve" (Q23)
// ─────────────────────────────────────────────────────────────────────────────
//
// FINDING (2026-05-30, run to completion — supersedes the 2026-05-29 note):
// `on_digivolution_card_trashed` behavior splits by trigger count.
//   • SINGLE source trashed: a lone mandatory observer fires SYNCHRONOUSLY at
//     trash-time (this probe; +1 immediately). `fire_digivolution_card_trashed`
//     enqueues then drains — kept synchronous for EX10-036's sibling already-queued
//     clauses (see fix-judge-quiz-engine-gaps §3.1).
//   • ≥2 sources trashed (the real Q23 shape): the mandatory observers form a
//     multi-trigger bundle → the drainer installs a `TriggerOrder` selection, which
//     PARKS them past the trashing effect. When the selection is resolved, each
//     observer's clause condition is RE-EVALUATED, and the cards returned in the
//     meantime fail (no longer in trash) → dropped. So Q23 already resolves to the
//     judge-correct +1 (see `q23_inherited_trash_memory_gated_on_remaining_in_trash`).
//
// The earlier "+3 over-count" (G-ON-TRASH-OBSERVER-SYNCHRONOUS) was a
// MISCHARACTERIZATION of the multi-source case — it only ran THIS single-source
// probe and reasoned about deferral abstractly, never running the 3-source-then-
// return scenario to completion. The engine's TriggerOrder parking + condition
// re-evaluation already implements the remain-in-trash gating for Q23. Residual
// narrow open question: a SINGLE source trashed then returned WITHIN the same
// effect would still fire synchronously (no deferral) — but no known card exercises
// that, so it is not a blocker.

/// Characterizes the single-source synchronous firing. EX8-051 Proganomon is a
/// `[Mineral]` host, so trashing one EX8-005 Tumblemon source meets its inherited
/// "gain 1 memory" condition and memory changes immediately on the trash call (a
/// lone mandatory trigger does not park). The multi-source case behaves differently
/// (parks as TriggerOrder, gates on remain-in-trash) — see the Q23 test.
#[test]
fn cluster_d_on_trash_observer_fires_synchronously_not_deferred() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX8-051")
        .expect("EX8-051 Proganomon (Mineral host) loads")
        .dsl_card("EX8-005")
        .expect("EX8-005 Tumblemon loads")
        .memory(0)
        .start();

    let host = r.place_on_field(0, "EX8-051", Some(0));
    r.push_source(host, "EX8-005"); // add a Tumblemon digivolution source
    let mem_before = r.memory();

    // Trash the Tumblemon source → fires on_digivolution_card_trashed.
    r.trash_one_source(host);

    // Synchronous: the +1 memory has ALREADY resolved (no deferral window).
    assert_eq!(
        (r.memory() as i32 - mem_before as i32).abs(),
        1,
        "Tumblemon's inherited gain-memory must resolve synchronously at \
         trash-time (the root of the Q21/Q23 remain-in-trash gap)"
    );
}

/// Q23 PASS — dispatch-level pin (synthetic Medusamon driver over real EX8-051 /
/// EX8-005). A Proganomon (`EX8-051`, [Mineral]) holds 3 Tumblemon (`EX8-005`)
/// sources, each carrying the inherited "gain 1 memory when trashed from a
/// Mineral/Rock Digimon" clause. One resolving effect trashes all 3 (Fragment <3>)
/// then returns 2 to the deck (Medusamon's return-2). Judge ruling: only the 1 card
/// STILL in trash at resolution gains memory → +1.
///
/// VERIFIED ENGINE BEHAVIOR (2026-05-30) == judge answer: when ≥2 sources are
/// trashed mid-effect, their mandatory `OnDigivolutionCardTrashed` observers form a
/// multi-trigger bundle and the engine installs a `TriggerOrder` selection — which
/// PARKS them past the trashing effect (the return-2 runs first). On resolution
/// each observer's clause condition is re-evaluated and the 2 returned cards'
/// observers fail (no longer in trash) → dropped; only the 1 remaining fires (+1).
/// The previously-logged `G-ON-TRASH-OBSERVER-SYNCHRONOUS` "+3 over-count" was a
/// mischaracterization for this multi-source case (it only ran the single-source
/// probe + reasoned about deferral abstractly; nobody ran the 3-source scenario to
/// completion). Driven as a top-level queued effect so the trashes run nested.
#[test]
fn q23_inherited_trash_memory_gated_on_remaining_in_trash() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX8-051")
        .expect("EX8-051 Proganomon (Mineral host) loads")
        .dsl_card("EX8-005")
        .expect("EX8-005 Tumblemon loads")
        .add_card(make_test_card("TEST-MEDUSA", "Medusamon analog"))
        .memory(0)
        .start();

    let host = r.place_on_field(0, "EX8-051", Some(0));
    let t1 = r.push_source(host, "EX8-005");
    let t2 = r.push_source(host, "EX8-005");
    let _t3 = r.push_source(host, "EX8-005");

    // Driver permanent: trashes all 3 sources then returns 2 — in one effect body.
    let _drv = r.place_on_field(0, "TEST-MEDUSA", Some(1));
    r.register_effect(
        "TEST-MEDUSA",
        Arc::new(TrashThreeReturnTwo {
            host,
            return_cards: vec![t1, t2],
        }),
    );

    let mem_before = r.memory() as i32;
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::PlayerBattleArea(0));
    r.game.drain_effect_queue();

    // The 3 mandatory observers form a multi-trigger bundle → the engine installs
    // a TriggerOrder selection (this IS the deferral window). Resolve it the way a
    // real game / RL agent must. As each parked observer resolves, its clause
    // condition is RE-EVALUATED: the 2 returned cards' observers now fail (their
    // source is no longer in trash) and are dropped; only the 1 card still in trash
    // fires. This is the engine's built-in remain-in-trash gating.
    let mut guard = 0;
    while let Some(sel) = r.game.pending_selection.as_ref() {
        let player = sel.selecting_player;
        let action = sel.valid_action_ids[0];
        r.game
            .resolve_selection(player, action)
            .expect("resolve TriggerOrder");
        guard += 1;
        assert!(guard <= 10, "selection resolution did not terminate");
    }

    // 1 of the 3 Tumblemon remains in trash; only its inherited gain-memory
    // resolves. Verified engine behavior == judge answer.
    assert_eq!(
        r.memory() as i32 - mem_before,
        1,
        "only the 1 Tumblemon still in trash should gain memory (judge Q23: +1, not +3)"
    );
    assert_eq!(r.trash_size(0), 1, "exactly 1 Tumblemon remains in trash");
}

/// Q9 — Mastemon (BT23-102) trashes Gatomon (BT15-037) from security; Gatomon
/// plays out only after Mastemon resolves and is NOT in the battle area during
/// security removal. Judge: after both trashed, NO memory from Gatomon [All Turns].
#[test]
#[ignore = "BLOCKED-CARD: needs BT23-102 (Mastemon), BT15-037 (Gatomon)."]
fn q9_gatomon_not_in_battle_area_during_removal_no_memory() {}

/// Q19 — Eyesmon: Scatter Mode (BT7-069) returned to hand by Calling From the
/// Darkness (BT7-107) ⇒ no [On Deletion]. Judge: 0 draws.
#[test]
#[ignore = "BLOCKED-CARD: needs BT7-069 (Eyesmon: Scatter Mode), BT2-069 (Gabumon), BT3-006 (DemiMeramon). BT7-107 implemented."]
fn q19_on_deletion_suppressed_when_returned_to_hand() {}

/// Q20 — Eyesmon stays in trash (+Pumpkinmon BT2-076) ⇒ all [On Deletion] fire.
/// Judge: 8 draws.
#[test]
#[ignore = "BLOCKED-CARD: needs BT7-069, BT2-069, BT3-006, BT2-076 (Pumpkinmon). BT7-107 implemented."]
fn q20_all_on_deletion_fire_when_eyesmon_stays_in_trash() {}

/// Q21 — Eyesmon played from trash by Back for Revenge! (BT3-109) ⇒ remaining
/// [On Deletion] can't fire. Judge: 0 draws.
#[test]
#[ignore = "BLOCKED-CARD: needs BT7-069, BT2-069, BT3-006, BT2-076, BT3-109 (Back for Revenge!). BT7-107 implemented."]
fn q21_remaining_on_deletion_suppressed_when_played_from_trash() {}

