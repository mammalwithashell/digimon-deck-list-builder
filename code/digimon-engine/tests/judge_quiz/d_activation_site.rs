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
///
/// # Board (read off the PDF, "ZXavier's Digi Rulings", PDF pp.39–41)
/// Player A has a Digimon in the battle area:
///   top  Eyesmon: Scatter Mode (BT7-069) — own [On Deletion] <Draw 3>+trash 2
///   src  Gabumon (BT2-069) — inherited [On Deletion] <Draw 2>+trash 1
///   src  DemiMeramon (BT3-006) — inherited [On Deletion] <Draw 1>+trash 1
/// Trash: 0 cards.
///
/// # Action
/// Player A uses Calling From the Darkness (BT7-107) targeting Eyesmon: Scatter
/// Mode. CFtD deletes the Digimon (the whole stack goes to trash), THEN returns
/// up to 2 purple Digimon cards from trash to hand — here Eyesmon: Scatter Mode
/// & Gabumon.
///
/// # Judge rationale (PDF p.41)
/// "[On Deletion] effects are tied to the top-most card of the Digimon, not tied
/// to the digivolution cards. Additionally, [On Deletion] effects can only
/// activate if they're in the trash. Since Calling From the Darkness returned
/// Eyesmon: Scatter Mode to the hand, no [On Deletion] effects are able to
/// activate." ⇒ the WHOLE [On Deletion] bundle (the topmost card's own + EVERY
/// inherited source's) is gated on the TOP-MOST card (Eyesmon) still being in
/// trash at resolution. Eyesmon returned ⇒ all suppressed ⇒ 0 draws.
///
/// # BLOCKED — G-ON-DELETION-RESOLVES-MID-EFFECT (timing) + color data bug
/// Run un-ignored 2026-06-03 to capture the real failure: driving the REAL
/// BT7-107 option (`activate_hand_main`) over the real stack draws **6**
/// (Eyesmon 3 + Gabumon 2 + DemiMeramon 1), not 0:
///
///   assertion `left == right` failed: Q19 (judge: 0 draws)
///     left: 6   right: 0
///
/// Two stacked blockers:
///  1. **G-ON-DELETION-RESOLVES-MID-EFFECT** — when CFtD's `delete_permanent`
///     step runs, the 3 [On Deletion] triggers park as a `TriggerOrder` and then
///     resolve *nested inside the delete step*, BEFORE CFtD's later
///     return-to-hand step runs. So a same-effect return of the carrier cannot
///     suppress them — they have already drawn. The judge's model defers the
///     whole [On Deletion] bundle until the causing effect (CFtD, incl. its
///     return) fully resolves, then gates each on the TOP-MOST card remaining in
///     trash. The engine has no "gate the inherited+own bundle on the topmost
///     card's trash residency" check. (Contrast Q20, where Eyesmon stays in
///     trash and the engine's 8 happens to match.)
///  2. **Color data bug** — BT7-069/BT2-069/BT3-006 are declared `color:
///     [black]` in their YAML specs (the in-file comments even say "Black"), but
///     the printed cards are PURPLE (cards.json `card_colors:[6]`; the PDF images
///     are purple-bordered). BT7-107's return filter is `color_is: purple`, so it
///     can't even target these mis-coloured cards from trash — the return step
///     surfaces no selection. (Independent of blocker 1: fixing the colour would
///     still leave the timing gap → still 6 draws.) Editing the card YAML is out
///     of scope here.
#[test]
#[ignore = "BLOCKED: G-ON-DELETION-RESOLVES-MID-EFFECT — [On Deletion] resolves nested in the delete step, before CFtD's return-to-hand, so the carrier-returns-to-hand suppression the judge requires never happens (engine draws 6, judge: 0). See engine-gaps.md. (The earlier color data bug — the 4 Eyesmon-stack cards declared black, actually purple [6] — was fixed 2026-06-03; only the timing gap remains.)"]
fn q19_on_deletion_suppressed_when_returned_to_hand() {
    let pad: Vec<&str> = vec!["PAD"; 30];
    let mut r = DebugRunner::builder()
        .dsl_card("BT7-069")
        .expect("BT7-069 Eyesmon: Scatter Mode loads")
        .dsl_card("BT2-069")
        .expect("BT2-069 Gabumon loads")
        .dsl_card("BT3-006")
        .expect("BT3-006 DemiMeramon loads")
        .dsl_card("BT7-107")
        .expect("BT7-107 Calling From the Darkness loads")
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &pad)
        .hand(0, &["BT7-107"])
        .memory(5)
        .start();
    // Stack bottom→top: DemiMeramon (egg), Gabumon, Eyesmon (top). 0 in trash.
    let _host = r.place_stack(0, &["BT3-006", "BT2-069", "BT7-069"]);
    let deck_before = r.deck_size(0);

    // Drive the REAL Calling From the Darkness: it deletes the Eyesmon stack and
    // (per the PDF) returns Eyesmon + Gabumon from trash to hand.
    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT7-107 main effect must fire");
    let _ = r.auto_resolve();

    // Judge: Eyesmon (top-most) returned to hand ⇒ NO [On Deletion] resolves.
    assert_eq!(
        deck_before - r.deck_size(0),
        0,
        "Q19: Eyesmon returned to hand ⇒ no [On Deletion] resolves (judge: 0 draws)"
    );
}

/// Q20 — Eyesmon stays in trash (+Pumpkinmon BT2-076) ⇒ all [On Deletion] fire.
/// Judge: 8 draws.  **PASS (2026-06-03).**
///
/// # Board (read off the PDF, "ZXavier's Digi Rulings", PDF pp.42–44)
/// Player A has a Digimon in the battle area:
///   top  Eyesmon: Scatter Mode (BT7-069) — own [On Deletion] <Draw 3>+trash 2
///   src  Gabumon (BT2-069)      — inherited [On Deletion] <Draw 2>+trash 1
///   src  DemiMeramon (BT3-006)  — inherited [On Deletion] <Draw 1>+trash 1
///   src  Pumpkinmon (BT2-076)   — inherited [On Deletion] <Draw 2>+trash 1
/// Trash: 0 cards.
///
/// # Action
/// Player A uses Calling From the Darkness (BT7-107) targeting Eyesmon: Scatter
/// Mode. CFtD deletes the Digimon (the whole 4-card stack goes to trash), THEN
/// returns up to 2 purple Digimon cards from trash to hand — here Gabumon &
/// Pumpkinmon (NOT Eyesmon).
///
/// # Judge rationale (PDF p.44)
/// "[On Deletion] effects are tied to the top-most card of the Digimon ... Since
/// Calling From the Darkness didn't return Eyesmon: Scatter Mode to the hand, all
/// [On Deletion] effects are able to activate." ⇒ the top-most card stays in
/// trash, so the WHOLE bundle fires:
///   Eyesmon 3 + Gabumon 2 + DemiMeramon 1 + Pumpkinmon 2 = **8 draws**.
///
/// # Load-bearing assertion
/// Net cards drawn (deck-size delta) == 8. The scenario is genuinely staged: the
/// real BT7-107 option deletes the real 4-card Eyesmon stack via
/// `activate_hand_main` (battle area empties), and every [On Deletion] in the
/// stack resolves. The deck must be deep enough that all 8 draws land (30 PADs).
/// Note: the colour data bug (sources declared black, are purple — see Q19) means
/// CFtD's purple-only return surfaces no selection, but that is NOT load-bearing
/// here: the judge keeps Eyesmon in trash either way, so the answer is 8
/// regardless of whether Gabumon/Pumpkinmon get returned. What this test pins is
/// the activation-site rule: the deleted carrier remaining in trash ⇒ all
/// [On Deletion] (own + every inherited) fire.
#[test]
fn q20_all_on_deletion_fire_when_eyesmon_stays_in_trash() {
    let pad: Vec<&str> = vec!["PAD"; 30];
    let mut r = DebugRunner::builder()
        .dsl_card("BT7-069")
        .expect("BT7-069 Eyesmon: Scatter Mode loads")
        .dsl_card("BT2-069")
        .expect("BT2-069 Gabumon loads")
        .dsl_card("BT3-006")
        .expect("BT3-006 DemiMeramon loads")
        .dsl_card("BT2-076")
        .expect("BT2-076 Pumpkinmon loads")
        .dsl_card("BT7-107")
        .expect("BT7-107 Calling From the Darkness loads")
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &pad)
        .hand(0, &["BT7-107"])
        .memory(5)
        .start();
    // Stack bottom→top: DemiMeramon (egg), Gabumon, Pumpkinmon, Eyesmon (top).
    let host = r.place_stack(0, &["BT3-006", "BT2-069", "BT2-076", "BT7-069"]);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "precondition: the 4-card Eyesmon stack is staged on the field"
    );
    let deck_before = r.deck_size(0);

    // Drive the REAL Calling From the Darkness: it deletes the Eyesmon stack.
    let fired = r.game.activate_hand_main(0, 0);
    assert!(fired, "BT7-107 main effect must fire");
    let _ = r.auto_resolve();

    // Precondition the scenario was genuinely staged: Eyesmon really got deleted.
    assert!(
        r.game
            .player(0)
            .battle_area
            .get(host.index as usize)
            .is_none(),
        "precondition: Eyesmon: Scatter Mode was deleted by Calling From the Darkness"
    );

    // Judge: Eyesmon stayed in trash ⇒ every [On Deletion] (own 3 + Gabumon 2 +
    // DemiMeramon 1 + Pumpkinmon 2) resolves ⇒ 8 cards drawn (deck shrinks by 8).
    assert_eq!(
        deck_before - r.deck_size(0),
        8,
        "Q20: all [On Deletion] fire when Eyesmon stays in trash \
         (3+2+1+2 = 8 draws)"
    );
}

/// Q21 — Eyesmon played from trash by Back for Revenge! (BT3-109) ⇒ remaining
/// [On Deletion] can't fire. Judge: 0 draws.
///
/// # Board (read off the PDF, "ZXavier's Digi Rulings"; same family as Q19/Q20)
/// (Q20 board) + Back for Revenge! (BT3-109) — Eyesmon: Scatter Mode is *played
/// from trash* mid-resolution, so it leaves the trash before the remaining
/// [On Deletion] can resolve ⇒ Judge: 0 draws.
///
/// # BLOCKED-CARD — BT3-109 not implemented (G-DSL-DELETED-SELF-TRASH-BINDING)
/// Back for Revenge! cannot be authored: the DSL has no binding for "this card" =
/// the just-deleted carrier sitting in trash (the card needs to reference and
/// play back the very Digimon whose deletion triggered it). Until that binding
/// exists, this scenario cannot be staged faithfully (no auto-selecting which
/// trashed card to play). Kept ignored as BLOCKED-CARD on BT3-109; this would
/// ALSO hit the same G-ON-DELETION-RESOLVES-MID-EFFECT timing gap as Q19 even if
/// BT3-109 existed. See engine-gaps.md.
#[test]
#[ignore = "BLOCKED-CARD: BT3-109 (Back for Revenge!) not implemented — G-DSL-DELETED-SELF-TRASH-BINDING (no DSL binding for 'this card' = the just-deleted carrier in trash). Would also hit G-ON-DELETION-RESOLVES-MID-EFFECT. BT7-069/BT2-069/BT3-006/BT2-076/BT7-107 implemented."]
fn q21_remaining_on_deletion_suppressed_when_played_from_trash() {}

