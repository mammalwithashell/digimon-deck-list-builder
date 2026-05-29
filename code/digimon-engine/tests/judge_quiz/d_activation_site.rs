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

use digimon_engine::debug_runner::DebugRunner;

// ─────────────────────────────────────────────────────────────────────────────
// Cluster-D/F ROOT-RULE PROBE — "remain-in-trash to resolve" (Q21, Q23)
// ─────────────────────────────────────────────────────────────────────────────
//
// Q23 (and Q21) require that an inherited on-trash / [On Deletion] effect, once
// triggered, is DEFERRED (queued) and resolves only if its carrier still remains
// in the trash at resolution — so when a later effect (Medusamon's "return 2", or
// Back for Revenge!'s replay) removes the card before resolution, that card's
// effect does NOT fire. Q23: 3 Tumblemon trashed → 2 returned → only 1 of the 3
// gain-memory effects resolves (+1, not +3).
//
// FINDING (2026-05-29, code-confirmed + probed): `on_digivolution_card_trashed`
// resolves SYNCHRONOUSLY at trash-time. `fire_digivolution_card_trashed`
// (game_actions.rs:3291-3308) enqueues the trigger then immediately
// `drain_effect_queue()` — an intentional choice (comment cites EX10-036, whose
// secondary clauses must see just-trashed cards mid-resolution). Because the
// inherited gain-memory fires the instant the source is trashed, there is NO
// deferral window in which a subsequent return could remove the card first → the
// engine cannot honor the Q21/Q23 remain-in-trash gating and would over-count
// (+1 per trashed source). Logged G-ON-TRASH-OBSERVER-SYNCHRONOUS.

/// Characterizes the synchronous firing (the gap root). EX8-051 Proganomon is a
/// `[Mineral]` host, so trashing an EX8-005 Tumblemon source meets its inherited
/// "gain 1 memory" condition. Memory changes immediately on the trash call —
/// proving the effect is not deferred. Passes as a characterization; the doc
/// above explains why this conflicts with the judge-faithful Q21/Q23 timing.
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

/// Q23 — Proganomon (EX8-051, Rock) with 3 Tumblemon (EX8-005) sources; Medusamon
/// (BT24-017) [When Digivolving] targets Proganomon → Fragment <3> trashes all 3
/// Tumblemon → each fires its inherited `on_digivolution_card_trashed` "gain 1
/// memory"; Medusamon then returns 2 to deck bottom. JUDGE ANSWER: Player A gains
/// 1 — like [On Deletion], the gain-memory only resolves for the cards still in
/// trash, and 2 of the 3 were returned. So only the 1 remaining resolves.
///
/// CANDIDATE — all three cards are implemented (the only such remaining
/// scenario), but a faithful test needs the full multi-selection chain
/// (Medusamon delete → Proganomon Fragment pick-3 → Medusamon return-2) AND
/// turns on whether `on_digivolution_card_trashed` gain-memory is gated on the
/// card remaining in trash at RESOLUTION (the queued triggers must resolve
/// AFTER Medusamon's [WD] returns the 2 cards). It is also downstream of the
/// Q22 routing bug (G-RETURN-TRASH-DIGI-EGG-ROUTING) — the same return verb.
/// Write as a real end-to-end test immediately after Q22 is fixed; expected to
/// either pin "+1 memory" or surface a second gap (gain-memory not gated on
/// remain-in-trash → would over-count to +3).
#[test]
#[ignore = "CANDIDATE (all cards implemented): needs the full Medusamon→Fragment→return-2 chain + verification that on_digivolution_card_trashed gain-memory is gated on remain-in-trash; downstream of Q22 (G-RETURN-TRASH-DIGI-EGG-ROUTING). Write after the Q22 fix."]
fn q23_inherited_trash_memory_gated_on_remaining_in_trash() {}
