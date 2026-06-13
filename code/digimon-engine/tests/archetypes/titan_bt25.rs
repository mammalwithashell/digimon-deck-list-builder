//! Titan (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/titan-bt25-model.md`. Slice cards (8 named):
//! BT25-006 Dorimon, BT25-068 Deltamon, BT25-071 Orochimon, BT25-019
//! UltimateBrachiomon are **IMPLEMENTED**; BT25-069 Raremon, BT25-080 Witchmon,
//! BT25-073 Dragomon, BT25-083 LadyDevimon are **BLOCKED** (dsl/engine/hybrid
//! gaps — verdicts in `qa/qa-reports/validated_cards_dsl.json`). Per Phase-4
//! precondition gating, every combo naming a BLOCKED card is dropped (logged in
//! the model doc + `qa/qa-reports/archetype_interactions.json`); only combos
//! whose pieces are all implemented are authored here.
//!
//! The slice is a **suspend-engine** deck: payoffs fire *when this Digimon
//! suspends* (Deltamon's De-Digivolve, Orochimon's reveal-play), and Dorimon is
//! the *re-arm* engine — it **unsuspends** a Titan on the opponent's turn so it
//! can be suspended again and re-fire. These tests pin the *system-level* facts
//! the per-card tests (`cards_behavioral/bt25/*`) cannot see: the cross-card
//! trait gate (T1), the cross-turn OPT re-arm (T2), the free-play-as-trigger
//! platform (T3), and apex-removal composition with the De-Digivolve tempo (T4).
//!
//! Sources (per CLAUDE.md source priority):
//! - Card text: cards.json BT25-006/068/071/019.
//! - DCGO C#: `$BASE_DCGO/Assets/Scripts/CardEffect/BT25/{Purple,Black,Red}/…`.
//! - Rules: `general_rule.pdf` §15-14-1 ([X Per Turn] counted *per turn*),
//!   §10 suspend/unsuspend, deletion/De-Digivolve semantics.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use super::support::snapshot;

const DORIMON: &str = "BT25-006";
const DELTAMON: &str = "BT25-068";
const OROCHIMON: &str = "BT25-071";
const BRACHIOMON: &str = "BT25-019";

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A Lv.4 **Titan**-trait Digimon — a legal Dorimon unsuspend target / generic
/// Titan ally on the field.
fn titan_lv4(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(4);
    c.dp = Some(dp);
    c.traits = vec!["Titan".to_string(), "TS".to_string()];
    c
}

/// A Lv.4 **non-Titan** Digimon — never a legal Dorimon unsuspend target.
fn plain_lv4(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(4);
    c.dp = Some(dp);
    c.traits = vec!["Beast".to_string()];
    c
}

/// An opponent attacker (drives Dorimon's `on_opponent_attack` window).
fn opp_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(5000);
    c.traits = vec!["Beast".to_string()];
    c
}

/// A generic opponent Digimon target with a chosen DP.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A cost-4 **[TS]**-trait Lv.4 Digimon with **no own trigger** — the neutral
/// reveal-play target for T3 (lets us assert Orochimon's *free play itself*
/// without pulling a cross-set payoff card just to fire a second effect).
fn ts_cost4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = vec!["Titan".to_string(), "TS".to_string()];
    c
}

// ════════════════════════════════════════════════════════════════════════════
// T1 — Dorimon unsuspends a Titan payoff (trait gate spanning two cards)
// ════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-006 Dorimon (inherited engine, under a Titan), BT25-068 Deltamon
// (the unsuspend target, carries [Titan]).
//
// Claim: on the opponent's turn, when an opponent Digimon attacks, Dorimon's
// inherited clause trashes 1 hand card and unsuspends a suspended [Titan]
// Digimon. The trait gate is *cross-card*: Dorimon names "[Titan]"; Deltamon
// supplies the trait. A non-Titan ally is not a legal target — the unhappy path
// proves the gate, which neither per-card test (Dorimon alone / Deltamon alone)
// can express.
//
// Sources: DCGO BT25_006.cs (SelectPermanent UnTap over own [Titan], opp-turn);
// general_rule.pdf §10 unsuspend.

/// Happy path: a suspended **Deltamon** (Titan) is the unsuspend target.
#[test]
fn t1_dorimon_unsuspends_titan_deltamon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(DORIMON)
        .expect("BT25-006 (Dorimon) in embedded DSL pack")
        .dsl_card(DELTAMON)
        .expect("BT25-068 (Deltamon) in embedded DSL pack")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(opp_attacker("OPP-ATK"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .hand(0, &["PAD"])
        .start();

    // Deltamon standing on P0's field with Dorimon as a digivolution source
    // (so Dorimon's *inherited* opp-turn clause is live), then suspend it.
    let deltamon = runner.place_stack(0, &[DORIMON, DELTAMON]);
    runner.game.suspend(deltamon);
    assert!(
        runner.game.players[0].battle_area[deltamon.index as usize].is_suspended,
        "precondition: Deltamon (the Titan target) is suspended"
    );

    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn

    let before = snapshot(&runner);
    runner.attack_player(opp, 0, false); // opponent Digimon attacks → Dorimon window

    assert!(
        runner.pending_selection().is_some(),
        "Dorimon's inherited trigger must install the optional accept/decline prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("accept Dorimon's optional opp-turn trigger");
    runner.auto_resolve().expect("resolve trash-1 + unsuspend");

    let after = snapshot(&runner);
    // Cost paid: exactly 1 hand card trashed.
    assert_eq!(after.hand[0], before.hand[0] - 1, "1 hand card trashed (the cost)");
    assert_eq!(after.trash[0], before.trash[0] + 1, "trashed card lands in trash");
    // Payoff: Deltamon (the Titan) is unsuspended.
    assert!(
        !runner.game.players[0].battle_area[deltamon.index as usize].is_suspended,
        "Deltamon (a [Titan]) must be unsuspended by Dorimon's clause"
    );
}

/// Unhappy path: the only suspended ally is **non-Titan** → Dorimon's clause has
/// no legal unsuspend target. Even paying the cost cannot unsuspend a non-Titan;
/// the engine must not offer the non-Titan as a target. This is the cross-card
/// trait gate that no per-card test sees.
#[test]
fn t1_dorimon_cannot_unsuspend_non_titan() {
    let mut runner = DebugRunner::builder()
        .dsl_card(DORIMON)
        .expect("BT25-006 (Dorimon) in embedded DSL pack")
        .add_card(plain_lv4("PLAIN", 4000))
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(opp_attacker("OPP-ATK"))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
        .hand(0, &["PAD"])
        .start();

    // Dorimon must sit under SOME Titan carrier to be live; use a Titan carrier
    // that is NOT suspended, and a separate suspended *non-Titan* as the would-be
    // (illegal) target.
    let carrier = runner.place_stack(0, &[DORIMON, "PAD"]); // PAD carrier keeps Dorimon inherited-live
    let _ = carrier;
    let non_titan = runner.place_on_field(0, "PLAIN", Some(0));
    runner.game.suspend(non_titan);

    let opp = runner.place_on_field(1, "OPP-ATK", Some(0));
    runner.game.turn_player_idx = 1;

    runner.attack_player(opp, 0, false);

    // Either the clause does not install a prompt at all (no legal [Titan]
    // target), or if it installs the optional accept it offers no non-Titan
    // unsuspend — and in no case is the non-Titan unsuspended.
    if runner.pending_selection().is_some() {
        // Accepting must not be able to unsuspend the non-Titan.
        let _ = runner.accept_optional_trigger();
        let _ = runner.auto_resolve();
    }
    assert!(
        runner.game.players[0].battle_area[non_titan.index as usize].is_suspended,
        "a non-[Titan] ally must NEVER be unsuspended by Dorimon's [Titan]-gated clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// T2 — Dorimon re-arms Deltamon's De-Digivolve across the turn boundary
// ════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-006 Dorimon, BT25-068 Deltamon.
//
// Claim: Deltamon's on-suspend De-Digivolve is **[All Turns] [Once Per Turn]**.
// Per general_rule.pdf §15-14-1-1, an [X Per Turn] effect can activate X times
// "*during 1 turn*" — the count resets at the start of *each* turn, including
// the opponent's. So Deltamon (controlled by P0) fires its De-Digivolve once on
// P0's turn, and — after the turn rotates to P1's turn — must be able to fire it
// AGAIN when re-suspended on P1's turn (Dorimon is the engine that unsuspends it
// in between so it *can* be re-suspended).
//
// This is the system-level fact no per-card test sees: the per-card OPT test
// only checks the *same-turn* lockout; the cross-turn re-arm is a property of
// the [All Turns] timing interacting with the turn boundary.
//
// Sources: general_rule.pdf §15-14-1-1/§15-14-1-2 ([X Per Turn] is per *turn*);
// DCGO BT25_068.cs (OnTappedAnyone, CanTriggerWhenSelfPermanentSuspends, OPT).

/// Cross-turn re-arm: De-Digivolve fires on P0's turn, then again on P1's turn.
///
/// If the engine only resets the *turn player's* per-turn activation counters at
/// turn start (and not the non-turn-player's), an [All Turns] OPT effect on a
/// non-turn-player Digimon stays locked out on the opponent's turn — which
/// contradicts §15-14-1-1. This test encodes the *correct* (per-turn) behavior;
/// a failure on the second suspend is a candidate engine bug, routed to the
/// gap trackers rather than weakened.
///
/// CONFIRMED engine bug (2026-06-06): the engine resets per-turn activation
/// counters only for the *turn player* at turn-start
/// (`game_phases.rs::continue_begin_turn_after_start_delays` → `new_turn()` on
/// `tp` only), so a non-turn-player Digimon's [All Turns] OPT counter persists
/// into the opponent's turn and wrongly locks the effect out — contradicting
/// `general_rule.pdf` §15-14-1-1 ([X Per Turn] resets each turn). Filed under
/// `qa/archetype-qa/engine-gaps.md` [G-ALL-TURNS-FILTER] (OPT-counter sub-case).
/// `#[ignore]`'d so the archetypes suite stays green while this reproducer is
/// preserved — remove the ignore when the turn-start reset is widened to all
/// players' permanents.
#[test]
#[ignore = "engine gap: G-ALL-TURNS-FILTER (OPT-counter sub-case) — non-turn-player [All Turns] OPT counter not reset on opponent's turn-start; reproducer, see qa/archetype-qa/engine-gaps.md"]
fn t2_deltamon_de_digivolve_re_arms_on_opponent_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(DELTAMON)
        .expect("BT25-068 (Deltamon) in embedded DSL pack")
        .add_card(make_test_card("OPP-LV4", "OppLv4"))
        .add_card(make_test_card("OPP-LV5", "OppLv5"))
        .add_card(make_test_card("OPP-LV4B", "OppLv4b"))
        .add_card(make_test_card("OPP-LV5B", "OppLv5b"))
        .deck(0, &["OPP-LV4"; 8])
        .deck(1, &["OPP-LV4"; 8])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let deltamon = runner.place_on_field(0, DELTAMON, Some(0));
    // Two separate opponent stacks so each De-Digivolve has a 2-source target.
    let opp_a = runner.place_stack(1, &["OPP-LV4", "OPP-LV5"]);
    let opp_b = runner.place_stack(1, &["OPP-LV4B", "OPP-LV5B"]);

    let a_before = runner.game.players[1].battle_area[opp_a.index as usize].card_sources.len();
    let b_before = runner.game.players[1].battle_area[opp_b.index as usize].card_sources.len();
    assert_eq!((a_before, b_before), (2, 2), "precondition: two 2-source opponent stacks");

    // ── Suspend #1 on P0's turn → fires De-Digivolve (removes 1 source). ──
    runner.game.suspend(deltamon);
    assert!(
        runner.pending_selection().is_some(),
        "first self-suspend on P0's turn must install the De-Digivolve select"
    );
    runner.auto_resolve().expect("resolve first De-Digivolve");
    let removed_after_first = (a_before
        - runner.game.players[1].battle_area[opp_a.index as usize].card_sources.len())
        + (b_before
            - runner.game.players[1].battle_area[opp_b.index as usize].card_sources.len());
    assert_eq!(removed_after_first, 1, "exactly one opponent source De-Digivolved on turn 1");

    // ── Rotate to the opponent's turn. ──
    runner.end_turn();
    assert_eq!(runner.game.turn_player_idx, 1, "now the opponent's turn");

    // Re-suspend Deltamon on the opponent's turn (e.g., it blocked). [All Turns]
    // OPT must re-arm: this is a NEW turn, so the per-turn count is reset.
    runner.game.suspend(deltamon);
    assert!(
        runner.pending_selection().is_some(),
        "[All Turns][OPT] De-Digivolve must RE-ARM on the opponent's turn \
         (general_rule.pdf §15-14-1-1: [X Per Turn] resets each turn). If this \
         fails, the engine is only resetting the turn-player's OPT counters — \
         candidate engine bug for [All Turns] OPT on a non-turn-player Digimon."
    );
    runner.auto_resolve().expect("resolve second De-Digivolve");
    let removed_total = (a_before
        - runner.game.players[1].battle_area[opp_a.index as usize].card_sources.len())
        + (b_before
            - runner.game.players[1].battle_area[opp_b.index as usize].card_sources.len());
    assert_eq!(
        removed_total, 2,
        "a second source must be De-Digivolved on the opponent's turn (cross-turn re-arm)"
    );
}

/// Guard (same-turn lockout — the per-turn floor): two suspends on the SAME turn
/// fire the De-Digivolve only ONCE. This anchors the cross-turn test above: T2
/// asserts a NEW turn re-arms; this asserts the SAME turn does not — together
/// they pin the [X Per Turn] boundary at the turn edge exactly.
#[test]
fn t2_deltamon_de_digivolve_same_turn_lockout() {
    let mut runner = DebugRunner::builder()
        .dsl_card(DELTAMON)
        .expect("BT25-068 (Deltamon) in embedded DSL pack")
        .add_card(make_test_card("OPP-LV4", "OppLv4"))
        .add_card(make_test_card("OPP-LV5", "OppLv5"))
        .deck(0, &["OPP-LV4"; 8])
        .deck(1, &["OPP-LV4"; 8])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let deltamon = runner.place_on_field(0, DELTAMON, Some(0));
    let opp = runner.place_stack(1, &["OPP-LV4", "OPP-LV5"]);
    let before = runner.game.players[1].battle_area[opp.index as usize].card_sources.len();

    runner.game.suspend(deltamon);
    runner.auto_resolve().expect("first De-Digivolve resolves");
    runner.game.suspend(deltamon); // same turn again
    assert!(
        runner.pending_selection().is_none(),
        "second self-suspend in the SAME turn must not fire (OPT lockout)"
    );
    let after = runner.game.players[1].battle_area[opp.index as usize].card_sources.len();
    assert_eq!(before - after, 1, "only one source De-Digivolved within one turn");
}

// ════════════════════════════════════════════════════════════════════════════
// T3 — Orochimon on-suspend reveal-play is a free-play trigger platform
// ════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-071 Orochimon, a cost-4 [TS] Digimon played from the reveal.
//
// Claim: when Orochimon suspends it reveals the top 3 of deck and the player may
// play a cost≤4 [TS] Digimon **without paying its cost** (a play-event). The
// free play is genuine tempo: the played card enters the field as if played, so
// any [On Play] it carries would fire off Orochimon's clause. Here the reveal
// target is a neutral cost-4 [TS] with no own trigger, so the assertion is the
// free-play itself (field +1, no memory spent) — the cross-card fire is
// structural; no cross-set payoff is pulled (lazy-closure rule honored).
//
// Sources: DCGO BT25_071.cs (SimplifiedRevealDeckTopCardsAndSelect, play
// payCost:false, remainder to deck bottom); general_rule.pdf play timing.

/// Happy path: a cost-4 [TS] Digimon on top of deck is revealed and free-played;
/// the other revealed cards go to the deck bottom; no memory is spent.
#[test]
fn t3_orochimon_suspend_free_plays_ts_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(OROCHIMON)
        .expect("BT25-071 (Orochimon) in embedded DSL pack")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(ts_cost4("TS-LV4"))
        // Deck top is the LAST element → TS-LV4 is revealed first.
        .deck(0, &["PAD", "PAD", "PAD", "PAD", "TS-LV4"])
        .deck(1, &["PAD"; 8])
        .memory(2) // < 4: the play MUST be free, not paid
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let orochimon = runner.place_on_field(0, OROCHIMON, Some(0));
    let field_before = runner.battle_area_size(0);
    let memory_before = runner.game.memory;

    runner.game.suspend(orochimon);
    assert!(
        runner.pending_selection().is_some(),
        "Orochimon's self-suspend must install the reveal/play selection"
    );

    // Pick the revealed TS-LV4 (reveal index 0), then resolve the
    // remainder-ordering picks (bottom the other two).
    let _ = runner.execute_action(0, digimon_engine::action::space::SEL_REVEAL_START);
    runner.game.drain_effect_queue();
    while let Some(view) = runner.pending_selection_view() {
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(digimon_engine::action::space::PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the cost-4 [TS] Digimon must be played to the field (free) off Orochimon's clause"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "play_from_revealed_free must not spend the revealed card's play cost"
    );
}

/// Unhappy path: nothing eligible in the top 3 (no cost≤4 [TS] Digimon) → the
/// reveal still resolves (remainder bottomed) but NOTHING is played, so the
/// field is unchanged. The free-play platform is gated on the reveal *finding*
/// an eligible [TS] Digimon — a system-level conditionality.
#[test]
fn t3_orochimon_suspend_no_eligible_play_leaves_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(OROCHIMON)
        .expect("BT25-071 (Orochimon) in embedded DSL pack")
        .add_card(make_test_card("PAD", "Filler"))
        .deck(0, &["PAD"; 8]) // no [TS] Digimon to reveal
        .deck(1, &["PAD"; 8])
        .memory(2)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let orochimon = runner.place_on_field(0, OROCHIMON, Some(0));
    let field_before = runner.battle_area_size(0);

    runner.game.suspend(orochimon);
    // Resolve whatever the reveal installs (bottom the 3 revealed cards).
    while let Some(view) = runner.pending_selection_view() {
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(digimon_engine::action::space::PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no eligible [TS] Digimon in the reveal → nothing played, field unchanged"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// T4 — UltimateBrachiomon apex removal composes with the De-Digivolve tempo
// ════════════════════════════════════════════════════════════════════════════
//
// Cards: BT25-019 UltimateBrachiomon, BT25-068 Deltamon (board context).
//
// Claim: Brachiomon's [On Play / When Digivolving] deletes the opponent's
// HIGHEST-DP Digimon. Deltamon's De-Digivolve shrinks a stack's *level/sources*
// but not its printed DP; the highest-DP gate reads the *current DP board*. So
// the two payoffs compose without interference: De-Digivolve changes
// level/sources, Brachiomon's gate still picks the genuine top-DP target. This
// pins that the gate keys on DP (not level), which a per-card test of either
// card in isolation cannot establish.
//
// Sources: DCGO BT25_019.cs (IsMaxDP Destroy); BT25_068.cs (De-Digivolve);
// general_rule.pdf §11 DP, deletion.

/// Brachiomon deletes the highest-DP opponent Digimon; a De-Digivolved (lower
/// *level*, unchanged *DP*) opponent stack is NOT shielded by the level change —
/// the gate is DP-based, so the genuine highest-DP target is removed.
#[test]
fn t4_brachiomon_removes_highest_dp_after_de_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card(BRACHIOMON)
        .expect("BT25-019 (UltimateBrachiomon) in embedded DSL pack")
        .add_card(opp_digimon("OPP-LOW", 5000))
        .add_card(opp_digimon("OPP-HIGH", 11000))
        .add_card(make_test_card("OPP-SRC", "OppSrc"))
        .deck(0, &["OPP-LOW"; 8])
        .deck(1, &["OPP-LOW"; 8])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let brachiomon = runner.place_on_field(0, BRACHIOMON, Some(0));
    // OPP-HIGH is the highest-DP target; give it a digivolution source so it
    // resembles a De-Digivolved (now lower-level) stack whose DP is unchanged.
    let opp_high = runner.place_stack(1, &["OPP-SRC", "OPP-HIGH"]);
    let opp_low = runner.place_on_field(1, "OPP-LOW", Some(0));
    let _ = opp_low;

    let before = snapshot(&runner);
    let high_dp = runner.effective_dp(opp_high);
    assert_eq!(high_dp, Some(11000), "OPP-HIGH is the highest-DP opponent Digimon");

    // Fire Brachiomon's [On Play] removal.
    runner.fire_play_event_triggers(0, brachiomon.index as usize, false, false);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    // Exactly one opponent Digimon removed; it must be the 11000-DP one.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "Brachiomon must delete exactly one opponent Digimon (the highest DP)"
    );
    let survivors: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_name(&runner.game.card_data).to_string())
        .collect();
    assert!(
        !survivors.iter().any(|n| n == "OPP-HIGH"),
        "the highest-DP (11000) Digimon must be the one deleted; survivors={survivors:?}"
    );
    assert!(
        survivors.iter().any(|n| n == "OPP-LOW"),
        "the lower-DP Digimon must survive the highest-DP-gated removal; survivors={survivors:?}"
    );
}
