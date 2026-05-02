//! Phase F Task 5 — `Keyword::MindLink` auto-install behavioral tests.
//!
//! `<Mind Link>` is a `[Main]` active skill on Tamers
//! (RULES_CONTEXT 16-27, DCGO `MindLink.cs`):
//!   - Place this Tamer at the bottom of one of your Digimon's
//!     digivolution stack.
//!   - Target Digimon must have NO non-face-down Tamer cards in its
//!     digivolution stack (DCGO line 25:
//!     `cardSource.IsTamer && !cardSource.IsFlipped`).
//!   - Optional pick (DCGO `canNoSelect: true`, line 60).
//!
//! ## Auto-install shape (Phase F §F3)
//!
//! `EffectTiming::MainOnField` declarative skill on the Tamer carrier:
//!   - Activation gate: self is a Tamer on battle area AND controller has
//!     ≥1 own non-Tamer permanent with no non-face-down Tamer source.
//!   - Body: optional pick (`is_optional=true`); on accept, the Tamer's
//!     top card is moved to the bottom of the chosen Digimon's stack
//!     and the Tamer permanent is removed from battle area.
//!
//! Mirrors DCGO line 71-79: `IPlacePermanentToDigivolutionCards(...)`
//! grabs the Tamer's TopCard and adds it to the bottom of the target
//! Digimon's stack. The face-down flag is NOT set (MindLink places
//! face-up; only `<Training>` produces face-down sources).
//!
//! Cost: zero (no memory, no suspension) — `EffectTiming::MainOnField`
//! exposes the activation in the action mask without cost gating,
//! mirroring MaterialSave's parity treatment.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::{attach_face_down_source, plain_digimon, plain_tamer, tamer_with_keywords};

fn mind_link_tamer(id: &str) -> CardData {
    tamer_with_keywords(id, vec![Keyword::MindLink])
}

/// Append `card_id` on top of a field permanent's digivolution stack so it
/// becomes the new visible top. Used to plant a non-face-down Tamer source
/// under a Digimon (the negative case for the candidate filter).
fn push_source_card(r: &mut DebugRunner, player: u8, field_index: usize, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_card: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].battle_area[field_index]
        .card_sources
        .push(card);
}

fn field_main_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize
}

// ─── Test 1: happy path — Tamer attaches to Digimon with no Tamer source ───

/// P0 has a Tamer-with-MindLink and a plain Digimon with no Tamer in its
/// stack. Activate the [Main] skill, pick the Digimon. Expected:
///   - Tamer permanent is gone from battle area.
///   - Digimon's stack now has the Tamer source at the bottom (index 0).
///   - Digimon's top is unchanged.
#[test]
fn mind_link_attaches_tamer_to_digimon_with_no_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(mind_link_tamer("ML-TAMER"))
        .add_card(plain_digimon("DIGI"))
        .start();

    let _digi = r.place_on_field(0, "DIGI", None);
    let tamer = r.place_on_field(0, "ML-TAMER", None);
    let digi_field_index = 0usize;
    let tamer_field_index = tamer.index as usize;
    assert_eq!(tamer_field_index, 1);

    let digi_stack_before = r.game.players[0].battle_area[digi_field_index]
        .card_sources
        .len();
    assert_eq!(digi_stack_before, 1, "DIGI starts with just its top card");

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Action mask must offer the [Main] activation for the Tamer.
    {
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[field_main_bit(tamer_field_index)],
            1.0,
            "MindLink [Main] activation must appear in the action mask when a valid \
             target Digimon exists"
        );
    }

    // Activate.
    let fired = r.game.activate_field_main(0, tamer_field_index);
    assert!(fired, "MindLink activation must succeed");

    // Parked optional own-permanent pick.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("MindLink target pick must be parked");
        assert_eq!(pending.selecting_player, 0);
        assert!(
            pending.is_optional,
            "MindLink target pick is optional (DCGO canNoSelect: true)"
        );
        // Only one valid target: the Digimon.
        assert_eq!(pending.valid_action_ids.len(), 1);
    }

    let digi_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, digi_action)
        .expect("pick Digimon");

    assert!(
        r.game.pending_selection.is_none(),
        "no pending selection after MindLink resolves"
    );

    // Tamer is gone.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Tamer permanent must have been removed from battle area"
    );
    assert!(
        !r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "ML-TAMER"),
        "no permanent should still have ML-TAMER as top"
    );

    // Digimon's stack now has the Tamer at the bottom (index 0); top is
    // still DIGI.
    let digi_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        digi_perm.card_sources.len(),
        digi_stack_before + 1,
        "DIGI gained exactly 1 source — the Tamer card"
    );
    assert_eq!(
        digi_perm.card_sources[0].card_id(&r.game.card_data),
        "ML-TAMER",
        "Tamer source sits at the very bottom (index 0) of DIGI's stack"
    );
    assert_eq!(
        digi_perm.top_card().card_id(&r.game.card_data),
        "DIGI",
        "DIGI remains the visible top after receiving the Tamer at the bottom"
    );
    // Tamer source must be face-up (MindLink places face-up; only
    // <Training> produces face-down sources).
    assert!(
        !digi_perm.card_sources[0].face_down,
        "MindLink places the Tamer source face-up"
    );
}

// ─── Test 2: Digimon with non-face-down Tamer source is filtered out ───────

/// P0 has a Tamer-with-MindLink and a Digimon that already has a
/// non-face-down Tamer source in its stack. The activation gate must
/// fail — `[Main]` activation should NOT surface in the action mask.
#[test]
fn mind_link_skips_digimon_with_existing_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(mind_link_tamer("ML-TAMER"))
        .add_card(plain_digimon("DIGI"))
        .add_card(plain_tamer("OTHER-TAMER"))
        .start();

    // Stack: [OTHER-TAMER, DIGI]. DIGI is the top, OTHER-TAMER is a
    // non-face-down Tamer source under it.
    let digi = r.place_on_field(0, "OTHER-TAMER", None);
    push_source_card(&mut r, 0, digi.index as usize, "DIGI");
    let tamer = r.place_on_field(0, "ML-TAMER", None);
    let digi_field_index = digi.index as usize;
    let tamer_field_index = tamer.index as usize;

    // Sanity: DIGI is a non-Tamer top, with a non-face-down Tamer source.
    let digi_perm = &r.game.players[0].battle_area[digi_field_index];
    assert!(
        digi_perm.is_digimon(&r.game.card_data),
        "top must be a Digimon"
    );
    assert!(
        digi_perm
            .card_sources
            .iter()
            .any(|s| s.is_tamer(&r.game.card_data) && !s.face_down),
        "DIGI must carry at least one non-face-down Tamer source for this test"
    );

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Action mask must NOT offer the [Main] activation — no valid target.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(tamer_field_index)],
        0.0,
        "MindLink activation must NOT appear in the mask when no Digimon is \
         a valid target (DIGI carries a non-face-down Tamer source)"
    );

    // Direct activation must also fail (the condition gate suppresses fire).
    let fired = r.game.activate_field_main(0, tamer_field_index);
    assert!(
        !fired,
        "MindLink activation must fail when no valid Digimon target exists"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no selection parked when activation gate fails"
    );
}

// ─── Test 3: Digimon with only face-down Tamer sources is a valid target ───

/// P0 has a Tamer-with-MindLink and a Digimon whose Tamer source is
/// `face_down=true` (the `<Training>`-installed shape). DCGO line 25:
/// `cardSource.IsTamer && !cardSource.IsFlipped` — face-down Tamer sources
/// do NOT count, so MindLink should still treat the Digimon as a valid
/// target.
#[test]
fn mind_link_targets_digimon_with_only_facedown_tamer_source() {
    let mut r = DebugRunner::builder()
        .add_card(mind_link_tamer("ML-TAMER"))
        .add_card(plain_digimon("DIGI"))
        .add_card(plain_tamer("FACEDOWN-TAMER"))
        .start();

    let digi = r.place_on_field(0, "DIGI", None);
    // Plant a face-down Tamer source under DIGI.
    attach_face_down_source(&mut r, 0, digi.index as usize, "FACEDOWN-TAMER");
    let tamer = r.place_on_field(0, "ML-TAMER", None);
    let digi_field_index = digi.index as usize;
    let tamer_field_index = tamer.index as usize;

    // Sanity: face-down source planted, top is still DIGI.
    let digi_perm = &r.game.players[0].battle_area[digi_field_index];
    assert!(
        digi_perm
            .card_sources
            .iter()
            .any(|s| s.is_tamer(&r.game.card_data) && s.face_down),
        "DIGI must carry a face-down Tamer source for this test"
    );
    assert!(
        !digi_perm
            .card_sources
            .iter()
            .any(|s| s.is_tamer(&r.game.card_data) && !s.face_down),
        "DIGI must NOT carry any non-face-down Tamer source for this test"
    );
    assert_eq!(
        digi_perm.top_card().card_id(&r.game.card_data),
        "DIGI",
        "DIGI must still be the visible top after attach_face_down_source"
    );

    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Action mask SHOULD offer the [Main] activation — the face-down
    // Tamer source doesn't disqualify DIGI.
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(tamer_field_index)],
        1.0,
        "MindLink activation MUST appear in the mask — face-down Tamer sources \
         do not block the candidate filter (DCGO `!cardSource.IsFlipped`)"
    );

    // Activate and resolve the pick.
    let fired = r.game.activate_field_main(0, tamer_field_index);
    assert!(fired);

    let pending_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, pending_action)
        .expect("pick DIGI");

    // The Tamer was attached at the bottom — i.e., one slot below the
    // pre-existing face-down source.
    let digi_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        digi_perm.card_sources[0].card_id(&r.game.card_data),
        "ML-TAMER",
        "ML-TAMER must be at the very bottom (index 0) — pushed below the \
         pre-existing face-down Tamer source"
    );
    assert!(
        !digi_perm.card_sources[0].face_down,
        "the newly placed Tamer source must be face-up"
    );
}

// ─── Test 4: optional pick can be declined → no state change ───────────────

/// Activation surfaces, but the player declines (PASS). Expected:
///   - Tamer remains on field.
///   - Digimon's stack is unchanged.
#[test]
fn mind_link_optional_pick_can_be_declined() {
    use digimon_engine::action::space::PASS;

    let mut r = DebugRunner::builder()
        .add_card(mind_link_tamer("ML-TAMER"))
        .add_card(plain_digimon("DIGI"))
        .start();

    let digi = r.place_on_field(0, "DIGI", None);
    let tamer = r.place_on_field(0, "ML-TAMER", None);
    let digi_field_index = digi.index as usize;
    let tamer_field_index = tamer.index as usize;
    let digi_stack_before = r.game.players[0].battle_area[digi_field_index]
        .card_sources
        .len();

    r.game.enter_main_phase();
    r.game.set_memory(0);

    let fired = r.game.activate_field_main(0, tamer_field_index);
    assert!(fired);
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .expect("pick parked")
            .is_optional,
        "MindLink target pick must be optional"
    );

    // Decline via PASS.
    r.game.resolve_selection(0, PASS).expect("PASS to decline");

    assert!(r.game.pending_selection.is_none());

    // Tamer still on field, DIGI's stack unchanged.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "Tamer permanent must remain on field after decline"
    );
    assert_eq!(
        r.game.players[0].battle_area[tamer_field_index]
            .top_card()
            .card_id(&r.game.card_data),
        "ML-TAMER",
        "Tamer must still be at its original field slot"
    );
    let digi_perm = &r.game.players[0].battle_area[digi_field_index];
    assert_eq!(
        digi_perm.card_sources.len(),
        digi_stack_before,
        "DIGI's stack must be unchanged after decline"
    );
    assert_eq!(
        digi_perm.top_card().card_id(&r.game.card_data),
        "DIGI",
        "DIGI's top must be unchanged"
    );
}
