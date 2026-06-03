//! ST-6 Venomous Violet — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st-6-venomous-violet-model.md`.
//!
//! ST-6 is a Purple **trash engine**: fill the trash (draw-then-trash attackers,
//! On-Deletion mill) and recur out of it (free plays via Nail Bone / CresGarurumon
//! Digi-Burst), with a sacrifice/deletion sub-theme (Death Claw + Matt-Ishida
//! memory-on-deletion + Pagumon On-Deletion mill) and a Retaliation trade top end.
//!
//! These are the FIRST tests to actually EXECUTE these recursion/sacrifice chains —
//! the per-card tests in `cards_behavioral/st6/st6_cards.rs` are mostly STRUCTURAL
//! (compiled clause SHAPES). Where an assertion below could plausibly surface a real
//! engine bug rather than a test to weaken, it is flagged UNCERTAIN in the doc-comment.
//!
//! Sources per test: printed card text (see model doc), `general_rule.pdf` (On-Deletion
//! lifecycle = CLAUDE.md rule 25; §16 Retaliation / Digi-Burst / Security Attack), and
//! DCGO C# at `$BASE_DCGO/Assets/Scripts/CardEffect/ST6/Purple/ST6_XX.cs`.

#![allow(dead_code)]

use digimon_engine::action::space::{encode_attack, encode_source_select, PASS};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::support::snapshot;

// ─── Combo-piece fixtures (all bodies are REAL vanilla ST-6 DSL cards) ──────────
//
// Neutral targets / fillers / digivolution sources below are REAL vanilla purple
// ST-6 DSL cards (zero effect clauses), so a placed/sourced/recurred body never
// fires a stray effect onto the combo:
//   ST6-02 DemiDevimon (Lv3, 4000)  ST6-05 Elecmon (Lv3, 5000)
//   ST6-07 Youkomon   (Lv4, 6000)  ST6-09 Kyukimon (Lv5, 9000)
// A purple Lv5 (ST6-09) doubles as the "non-Lv3/Lv4" trash filler that the
// Nail Bone / CresGarurumon purple-Lv3/Lv4 recursion filter must NOT pick up.

/// Push a card by id into player `p`'s trash, owned by `p`.
/// (Mirrors the proven `push_trash` helper in `cards_behavioral/lm/lm_030.rs`.)
fn push_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_trash: unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].trash.push(card);
}

/// Fire a permanent's `[When Digivolving]` timing (`place_stack` builds the stack
/// without firing the trigger, so we fire it explicitly to model the digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 1 — Death Claw sacrifice → Matt memory + Pagumon On-Deletion mill
// ═══════════════════════════════════════════════════════════════════════════════

/// Signature sacrifice engine. On YOUR turn, with Matt (ST6-14) on the field and a
/// Pagumon-carrying Digimon (ST6-01 as a digivolution source under a Lv3 carrier),
/// play Death Claw (ST6-15) [Main]: delete YOUR Pagumon-carrying Digimon to delete
/// the opp Lv4-or-lower Digimon. Assert the chain:
///   - your field −1 (sacrificed carrier), opp field −1 (removed target),
///   - Pagumon INH [On Deletion] trashes top 2 of your deck (your deck −2, trash grows),
///   - Matt's optional "your Digimon deleted → suspend to gain 1 memory" → +1 memory,
///     Matt suspended.
///
/// On-Deletion fires post-trash (CLAUDE.md rule 25 / `general_rule.pdf` deletion lifecycle).
///
/// UNCERTAIN: (a) the *ordering* of Pagumon's On-Deletion mill vs. Matt's deletion
/// observer is not asserted here — only that both happen. (b) Whether Matt's optional
/// trigger surfaces as an explicit pending selection or is consumed by `auto_resolve`
/// is also not assumed; this test drives `auto_resolve` then, if Matt's gain did not
/// land, explicitly accepts a remaining optional trigger. If Matt's +1 never lands at
/// all, that is a candidate engine bug (deletion-observer wiring), not a test to weaken.
#[test]
fn death_claw_sacrifice_chains_pagumon_mill_and_matt_memory() {
    // Sacrifice carrier: vanilla ST6-02 DemiDevimon (purple Lv3). Opp target:
    // vanilla ST6-07 Youkomon (purple Lv4, ≤4 → deletable). Deck/mill filler:
    // vanilla ST6-09 Kyukimon (purple Lv5).
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-15") // Death Claw
        .expect("ST6-15 in embedded DSL pack")
        .dsl_card("ST6-14") // Matt Ishida
        .expect("ST6-14 in embedded DSL pack")
        .dsl_card("ST6-01") // Pagumon (Digi-Egg, inherited On Deletion)
        .expect("ST6-01 in embedded DSL pack")
        .dsl_card("ST6-02") // DemiDevimon (vanilla sacrifice carrier)
        .expect("ST6-02 in embedded DSL pack")
        .dsl_card("ST6-07") // Youkomon (vanilla opp Lv4 target)
        .expect("ST6-07 in embedded DSL pack")
        .dsl_card("ST6-09") // Kyukimon (vanilla deck/mill filler)
        .expect("ST6-09 in embedded DSL pack")
        .memory(3)
        .hand(0, &["ST6-15"])
        // Deck cards for Pagumon's "trash top 2 of your deck" to consume.
        .deck(0, &["ST6-09"; 6])
        .deck(1, &["ST6-09"; 6])
        .start();
    // Your turn.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Matt on your field (unsuspended), an opp Lv4 target, and your sacrifice carrier
    // with Pagumon as its digivolution source.
    runner.place_on_field(0, "ST6-14", None);
    runner.place_on_field(1, "ST6-07", None);
    let _stack = runner.place_stack(0, &["ST6-01", "ST6-02"]);

    let before = snapshot(&runner);

    // Play Death Claw [Main] from hand (hand index 0).
    assert!(
        runner.game.activate_hand_main(0, 0),
        "Death Claw [Main] must activate from hand"
    );

    // Step 1: optional select_own_permanent (the sacrifice). Accept it (the carrier).
    let sac_view = runner
        .pending_selection_view()
        .expect("Death Claw must prompt for YOUR Digimon to sacrifice");
    assert!(
        runner.pending_is_optional(),
        "the sacrifice is optional ('you may delete 1 of your Digimon')"
    );
    // Pick the first eligible own-Digimon (the Pagumon-carrying carrier; Matt is a
    // Tamer, not a Digimon, so it is not offered).
    let sac_action = sac_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a sacrifice target should be offered");
    runner
        .execute_action(sac_view.selecting_player, sac_action)
        .expect("accept the sacrifice");

    // Step 2: select the opp Lv4-or-lower target (mandatory once a sacrifice was made).
    let tgt_view = runner
        .pending_selection_view()
        .expect("Death Claw must prompt for the opp Lv4-or-lower target");
    runner
        .execute_action(tgt_view.selecting_player, tgt_view.valid_action_ids[0])
        .expect("delete the opp target");

    // Resolve any follow-up triggers (Pagumon mill / Matt observer).
    let _ = runner.auto_resolve();
    // If Matt's optional trigger is still parked, accept it explicitly.
    if runner.game.pending_selection.is_some() {
        let _ = runner.accept_optional_trigger();
        let _ = runner.auto_resolve();
    }

    let after = snapshot(&runner);

    // Your field: the sacrificed carrier is gone. (Matt + Death Claw landing as an
    // Option permanent may net out; assert the carrier specifically left.)
    let carrier_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-02");
    assert!(
        !carrier_present,
        "the sacrificed Pagumon-carrying Digimon must be deleted"
    );

    // Opp field −1: the Lv4 target was deleted.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the opponent Lv4-or-lower Digimon must be deleted"
    );

    // Pagumon INH [On Deletion]: trashed top 2 of YOUR deck.
    assert_eq!(
        after.deck[0],
        before.deck[0] - 2,
        "Pagumon's [On Deletion] must trash the top 2 cards of YOUR deck"
    );
    assert!(
        after.trash[0] >= before.trash[0] + 2,
        "Pagumon's milled cards (and the trashed carrier+Pagumon) must enter your trash; \
         before={} after={}",
        before.trash[0],
        after.trash[0]
    );

    // Matt's [Your Turn] deletion observer: +1 memory and Matt suspended.
    assert_eq!(
        after.memory,
        before.memory + 1,
        "Matt Ishida must gain 1 memory when one of your Digimon is deleted on your turn \
         (if this fails, suspect the deletion-observer wiring, not the test)"
    );
    let matt_suspended = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-14")
        .map(|p| p.is_suspended)
        .unwrap_or(false);
    assert!(
        matt_suspended,
        "Matt Ishida must be suspended as the activation cost for the memory gain"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 2 — Death Claw is gated to opp Lv4-or-lower (unhappy / gate)
// ═══════════════════════════════════════════════════════════════════════════════

/// Death Claw's opponent-removal is restricted to Lv4-or-lower. With an opp Lv4 and
/// an opp Lv5 on the board, the target selection must offer ONLY the Lv4 — the
/// system-level fact that the removal cannot touch the Lv5.
#[test]
fn death_claw_only_targets_opponent_level_4_or_lower() {
    // Vanilla bodies: ST6-02 (your Lv3 carrier), ST6-07 (opp Lv4, deletable),
    // ST6-09 (opp Lv5, above the gate), ST6-05 (deck filler).
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-15")
        .expect("ST6-15 in embedded DSL pack")
        .dsl_card("ST6-02")
        .expect("ST6-02 in embedded DSL pack")
        .dsl_card("ST6-07")
        .expect("ST6-07 in embedded DSL pack")
        .dsl_card("ST6-09")
        .expect("ST6-09 in embedded DSL pack")
        .dsl_card("ST6-05")
        .expect("ST6-05 in embedded DSL pack")
        .memory(3)
        .hand(0, &["ST6-15"])
        .deck(0, &["ST6-05"; 4])
        .deck(1, &["ST6-05"; 4])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    runner.place_on_field(0, "ST6-02", None);
    let lv4_h = runner.place_on_field(1, "ST6-07", None);
    runner.place_on_field(1, "ST6-09", None);

    assert!(runner.game.activate_hand_main(0, 0), "Death Claw must activate");

    // Accept the sacrifice (own carrier).
    let sac_view = runner
        .pending_selection_view()
        .expect("sacrifice prompt");
    let sac_action = sac_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a sacrifice target");
    runner
        .execute_action(sac_view.selecting_player, sac_action)
        .expect("accept sacrifice");

    // Target prompt: exactly ONE valid target (the Lv4); the Lv5 is filtered out.
    let tgt_view = runner
        .pending_selection_view()
        .expect("opponent-target prompt");
    let non_pass: Vec<u16> = tgt_view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "Death Claw must offer exactly one target (the opp Lv4), never the Lv5; offered={:?}",
        tgt_view.valid_action_ids
    );

    // Resolve and confirm the Lv4 is the one deleted and the Lv5 survives.
    runner
        .execute_action(tgt_view.selecting_player, non_pass[0])
        .expect("delete the Lv4");
    let _ = runner.auto_resolve();

    let lv4_gone = !runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-07");
    let lv5_alive = runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-09");
    assert!(lv4_gone, "the Lv4 target must be deleted");
    assert!(lv5_alive, "the Lv5 must survive Death Claw's level gate");
    let _ = lv4_h;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 3 — Fill trash → Nail Bone double recursion
// ═══════════════════════════════════════════════════════════════════════════════

/// Nail Bone (ST6-16) [Main]: play 1 purple Lv3 AND 1 purple Lv4 Digimon from trash
/// for free, with their [On Play] effects suppressed. Seed trash with a purple Lv3 +
/// a purple Lv4 (plus filler) and assert your field +2 and your trash drops by the
/// two recurred bodies.
#[test]
fn nail_bone_double_recursion_plays_lv3_and_lv4_free() {
    // Recursion targets: vanilla ST6-05 Elecmon (purple Lv3) + ST6-07 Youkomon
    // (purple Lv4). Trash filler: vanilla ST6-09 Kyukimon (purple Lv5) — NOT a
    // purple Lv3/Lv4, so Nail Bone's filter must never pick it up.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-16") // Nail Bone
        .expect("ST6-16 in embedded DSL pack")
        .dsl_card("ST6-05")
        .expect("ST6-05 in embedded DSL pack")
        .dsl_card("ST6-07")
        .expect("ST6-07 in embedded DSL pack")
        .dsl_card("ST6-09")
        .expect("ST6-09 in embedded DSL pack")
        .memory(10)
        .hand(0, &["ST6-16"])
        .deck(0, &["ST6-09"; 4])
        .deck(1, &["ST6-09"; 4])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Seed trash: one Lv3, one Lv4 (the recursion targets), plus a Lv5 filler
    // (ineligible) to prove the purple-Lv3/Lv4 filter ignores it.
    push_trash(&mut runner, 0, "ST6-05");
    push_trash(&mut runner, 0, "ST6-07");
    push_trash(&mut runner, 0, "ST6-09");

    let before = snapshot(&runner);
    assert_eq!(before.field[0], 0, "precondition: empty field");

    assert!(
        runner.game.activate_hand_main(0, 0),
        "Nail Bone [Main] must activate"
    );

    // Step 1: select the Lv3 (optional select_trash). Pick the only eligible Lv3.
    let lv3_view = runner
        .pending_selection_view()
        .expect("Nail Bone must prompt for the purple Lv3 trash target");
    let lv3_action = lv3_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an eligible Lv3 in trash");
    runner
        .execute_action(lv3_view.selecting_player, lv3_action)
        .expect("select Lv3 from trash");
    // NOTE: do NOT auto_resolve here — the Lv4 select_trash is OPTIONAL, and
    // auto_resolve would PASS it before we can drive it. Drive the Lv4 pick
    // explicitly; the engine advances Lv3-play → Lv4-select on its own.

    // Step 2: select the Lv4 (optional select_trash). Pick the only eligible Lv4.
    let lv4_view = runner
        .pending_selection_view()
        .expect("Nail Bone must prompt for the purple Lv4 trash target");
    let lv4_action = lv4_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an eligible Lv4 in trash");
    runner
        .execute_action(lv4_view.selecting_player, lv4_action)
        .expect("select Lv4 from trash");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    assert_eq!(
        after.field[0],
        before.field[0] + 2,
        "Nail Bone must play BOTH the Lv3 and Lv4 from trash to your field"
    );
    assert_eq!(
        after.trash[0],
        before.trash[0] - 2,
        "both recurred Digimon must leave the trash"
    );
    // The exact recurred bodies are on the field.
    let on_field: Vec<String> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        on_field.iter().any(|id| id == "ST6-05"),
        "the purple Lv3 (ST6-05) must be recurred; field={on_field:?}"
    );
    assert!(
        on_field.iter().any(|id| id == "ST6-07"),
        "the purple Lv4 (ST6-07) must be recurred; field={on_field:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 4 — Trash threshold → WereGarurumon inherited +2000
// ═══════════════════════════════════════════════════════════════════════════════

/// WereGarurumon (ST6-11) INHERITED aura: while it is your turn AND you have 5+
/// cards in your trash, the CARRIER (the Digimon WereGarurumon is a digivolution
/// SOURCE under) gets +2000 DP. ST6-11's effect is `scope: inherited`, so it only
/// applies from under a carrier — we place ST6-11 as a source beneath a vanilla
/// purple carrier (ST6-09 Kyukimon, base 9000) and read the CARRIER's effective DP.
/// With 4 cards in trash → no buff (carrier == base 9000). Raise to 5 and re-tick →
/// +2000 (== 11000). The deck pays you for filling the trash.
#[test]
fn weregarurumon_aura_flips_at_five_card_trash_threshold() {
    // Carrier: vanilla ST6-09 Kyukimon (base 9000). Trash filler: vanilla ST6-05.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-11") // WereGarurumon (Lv5, 7000 DP) — inherited +2000 source
        .expect("ST6-11 in embedded DSL pack")
        .dsl_card("ST6-09")
        .expect("ST6-09 in embedded DSL pack")
        .dsl_card("ST6-05")
        .expect("ST6-05 in embedded DSL pack")
        .memory(10)
        .deck(0, &["ST6-05"; 8])
        .deck(1, &["ST6-05"; 4])
        .start();
    // Your turn (the aura's `your_turn` gate).
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // ST6-11 as a digivolution SOURCE under the carrier: [WereGarurumon, Kyukimon].
    let carrier = runner.place_stack(0, &["ST6-11", "ST6-09"]);
    let carrier_base = 9000;

    // 4 cards in trash → below threshold → no buff on the carrier.
    for _ in 0..4 {
        push_trash(&mut runner, 0, "ST6-05");
    }
    assert_eq!(runner.trash_size(0), 4, "precondition: 4 cards in trash");
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(carrier_base),
        "with only 4 cards in trash the carrier stays at base 9000 DP (WereGarurumon's inherited aura is off)"
    );

    // Cross the threshold: 5 cards in trash → carrier +2000.
    push_trash(&mut runner, 0, "ST6-05");
    assert_eq!(runner.trash_size(0), 5, "precondition: 5 cards in trash");
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(carrier_base + 2000),
        "with 5+ cards in trash on your turn WereGarurumon's inherited aura gives the carrier +2000 DP (-> 11000)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 5 — CresGarurumon Digi-Burst → play a Lv3 purple from trash
// ═══════════════════════════════════════════════════════════════════════════════

/// CresGarurumon (ST6-13) [Main] <Digi-Burst 2>: trash 2 digivolution sources, then
/// play 1 purple Lv3 Digimon from trash for free. Build a CresGarurumon stack with 2
/// sources, seed trash with a purple Lv3, activate the field [Main], pick both sources,
/// then play the Lv3 → assert 2 sources trashed (stack sources −2, trash +2 from the
/// burst) AND your field +1 (the recurred Lv3 leaves trash). `general_rule.pdf` §16
/// Digi-Burst.
#[test]
fn cresgarurumon_digiburst_plays_lv3_purple_from_trash() {
    // Digivolution sources are vanilla and deliberately NOT purple Lv3 (ST6-07
    // Lv4 + ST6-09 Lv5), so after Digi-Burst trashes them the ONLY purple Lv3 in
    // trash is the seeded recursion target ST6-02 DemiDevimon (unambiguous pick).
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-13") // CresGarurumon
        .expect("ST6-13 in embedded DSL pack")
        .dsl_card("ST6-07")
        .expect("ST6-07 in embedded DSL pack")
        .dsl_card("ST6-09")
        .expect("ST6-09 in embedded DSL pack")
        .dsl_card("ST6-02")
        .expect("ST6-02 in embedded DSL pack")
        .dsl_card("ST6-05")
        .expect("ST6-05 in embedded DSL pack")
        .memory(10)
        .deck(0, &["ST6-05"; 4])
        .deck(1, &["ST6-05"; 4])
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Stack: [SrcA(Lv4), SrcB(Lv5), CresGarurumon] — 2 digivolution sources under top.
    let cres = runner.place_stack(0, &["ST6-07", "ST6-09", "ST6-13"]);
    // Seed a recurable purple Lv3 in trash.
    push_trash(&mut runner, 0, "ST6-02");

    let before = snapshot(&runner);
    let sources_before = runner.game.player(0).battle_area[cres.index as usize]
        .card_sources
        .len();

    assert!(
        runner.game.activate_field_main(0, cres.index as usize),
        "CresGarurumon [Main] Digi-Burst must activate"
    );

    // Pick the two Digi-Burst sources (positions 0 and 1 of the stack).
    runner
        .execute_action(
            0,
            encode_source_select(cres.index as u16, 0).expect("first source pick"),
        )
        .expect("pick first Digi-Burst source");
    runner
        .execute_action(
            0,
            encode_source_select(cres.index as u16, 1).expect("second source pick"),
        )
        .expect("pick second Digi-Burst source");

    // Then: select the purple Lv3 from trash to play free.
    let lv3_view = runner
        .pending_selection_view()
        .expect("Digi-Burst must prompt for the purple Lv3 trash target");
    let lv3_action = lv3_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("an eligible Lv3 in trash");
    runner
        .execute_action(lv3_view.selecting_player, lv3_action)
        .expect("play Lv3 from trash");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // Digi-Burst 2 trashed both sources.
    let sources_after = runner.game.player(0).battle_area[cres.index as usize]
        .card_sources
        .len();
    assert_eq!(
        sources_after,
        sources_before - 2,
        "Digi-Burst 2 must trash 2 digivolution sources"
    );
    // Field +1: the recurred Lv3 (CresGarurumon's own stack stays one permanent).
    assert_eq!(
        after.field[0],
        before.field[0] + 1,
        "the purple Lv3 must be played from trash onto your field"
    );
    // Net trash: +2 (burst sources) −1 (recurred Lv3) = +1.
    assert_eq!(
        after.trash[0],
        before.trash[0] + 1,
        "trash nets +2 burst sources minus the 1 recurred Lv3"
    );
    let lv3_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-02");
    assert!(lv3_on_field, "the recurred purple Lv3 (ST6-02) must be on your field");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Combo 6 — VenomMyotismon Retaliation trade
// ═══════════════════════════════════════════════════════════════════════════════

/// VenomMyotismon (ST6-12) [When Digivolving]: up to 2 of your Digimon gain
/// <Retaliation> until the end of your opponent's next turn. Grant it to one of your
/// Digimon (assert `has_keyword(Retaliation)`), then attack a bigger opp Digimon and
/// lose the battle — Retaliation deletes the attacker in return (a 1-for-1 trade).
/// `general_rule.pdf` §16 Retaliation (mandatory; fires on Battle-cause deletion).
///
/// UNCERTAIN: the grant targets up to 2 Digimon via `select_count_capped_multi`
/// (optional_zero). This test grants to exactly one of the two own Digimon; if the
/// selection UI offers VenomMyotismon itself as a candidate that is fine — we grant to
/// a separate small Digimon and drive its losing attack, so the trade is unambiguous.
///
// CANDIDATE ENGINE BUG (CONFIRMED): a DSL step `grant_keyword: Retaliation`
// silently fails to attach. `code/digimon-engine/src/dsl_cards/modifier_map.rs::
// lookup_keyword` has NO `"Retaliation"` arm, so it returns `None`; the runtime
// `CompiledStep::GrantKeyword` handler
// (`src/dsl_cards/step/modifiers.rs:234`) then early-returns WITHOUT granting.
// `Keyword::Retaliation` exists in `enums.rs` and is referenced by the test, so
// the gap is purely the missing keyword-map vocabulary. Confirmed: after the
// selection completes (verified picks reach the `per_selected` callback),
// `modifiers.has_keyword(trader, Retaliation)` is false. Per ST6-12 card text
// "[When Digivolving] Up to 2 of your Digimon gain <Retaliation>…", the chosen
// Digimon must carry the granted keyword.
#[test]
fn venommyotismon_grants_retaliation_and_trades_in_battle() {
    // Trader: vanilla ST6-07 Youkomon (purple Lv4, 6000) — receives Retaliation
    // and loses its attack. Bigger opponent: vanilla ST6-09 Kyukimon (purple Lv5,
    // 9000) — wins the battle, so the trader is deleted in battle.
    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-12") // VenomMyotismon
        .expect("ST6-12 in embedded DSL pack")
        .dsl_card("ST6-07")
        .expect("ST6-07 in embedded DSL pack")
        .dsl_card("ST6-09")
        .expect("ST6-09 in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // VenomMyotismon and the trader on your field; the big opponent on theirs.
    let venom = runner.place_on_field(0, "ST6-12", None);
    // Pass Some(0) so the trader has no summoning sickness and can attack.
    let trader_h = runner.place_on_field(0, "ST6-07", Some(0));
    let opp_h = runner.place_on_field(1, "ST6-09", Some(0));

    // Fire VenomMyotismon's [When Digivolving] grant.
    fire_when_digivolving(&mut runner, venom);

    // The grant prompt: up to 2 own Digimon. Explicitly grant to the TRADER
    // (own field slot = trader_h.index) so the trade below is unambiguous; then
    // PASS to stop at one pick.
    let grant_view = runner
        .pending_selection_view()
        .expect("VenomMyotismon must prompt to choose up to 2 Digimon for Retaliation");
    let trader_pick = encode_attack(0, trader_h.index as u16);
    assert!(
        grant_view.valid_action_ids.contains(&trader_pick),
        "the trader must be an offered Retaliation candidate; offered={:?}",
        grant_view.valid_action_ids
    );
    runner
        .execute_action(grant_view.selecting_player, trader_pick)
        .expect("grant Retaliation to the trader");
    // "Up to 2" — PASS to stop after the single pick.
    if runner.game.pending_selection.is_some() {
        let _ = runner.execute_action(0, PASS);
    }
    let _ = runner.auto_resolve();

    // The trader now carries the GRANTED <Retaliation>. A step-`grant_keyword`
    // installs into the keyword-modifier registry, so the proper reader is
    // `modifiers.has_keyword` (the source-walk `game.has_keyword` only sees
    // FACE / INHERITED keywords, not effect-granted ones).
    let trader_has_retaliation = runner
        .game
        .modifiers
        .has_keyword(trader_h, Keyword::Retaliation);
    assert!(
        trader_has_retaliation,
        "VenomMyotismon's [When Digivolving] must grant <Retaliation> to the chosen Digimon"
    );

    // Drive a losing attack with the Retaliation carrier (the trader) into the
    // bigger opponent. The trader (3000 DP) loses to the 9000 opp and is deleted
    // by Battle → its <Retaliation> deletes the opp it battled. Net: a 1-for-1
    // trade — both combatants gone.
    let _ = venom;
    let attacker = trader_h;
    let before = snapshot(&runner);
    runner.attack_digimon(attacker, opp_h, false);
    let after = snapshot(&runner);

    // The opponent it battled must be deleted by Retaliation even though it won.
    let opp_gone = !runner.game.players[1]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-09");
    assert!(
        opp_gone,
        "Retaliation must delete the Digimon our attacker battled when our attacker \
         is deleted after losing the battle"
    );
    // Sanity: the opponent's field shrank by the traded body.
    assert!(
        after.field[1] < before.field[1],
        "the opponent's board must lose the traded Digimon to Retaliation"
    );
}
