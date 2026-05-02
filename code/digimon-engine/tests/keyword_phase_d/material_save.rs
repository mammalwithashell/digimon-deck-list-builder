//! Phase D Task 10 — `Keyword::MaterialSave(N)` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::MaterialSave(N)]` (no
//! hand-rolled `CardEffect`) installs an `EffectTiming::MainOnField` active
//! skill that mirrors DCGO `MaterialSave.cs`:
//!
//!   - `[Main]` active skill on the carrier (NOT a replacement, NOT a
//!     deletion trigger). Cost: zero (no memory paid, no suspension).
//!   - Activation gate (`CanActivateMaterialSave`): self has ≥1 selectable
//!     digivolution source AND controller has ≥1 own Tamer.
//!   - On activation: player picks 1 own Tamer (mandatory), then picks
//!     up to N own card_sources from self (optional zero — player may
//!     pick fewer than N), and the picked sources are placed at the bottom
//!     of the chosen Tamer's stack.
//!
//! ## DCGO filter scope (Phase D auto-install)
//!
//! Per the Phase D plan note, MaterialSave on non-DigiXros: auto-install
//! offers "any source" filter. Per-card-text restrictions (e.g., DigiXros
//! source filter) are a hand-rolled override on top of the auto-install —
//! out of Phase D scope.
//!
//! Tamer-only target filter — `is_tamer(&card_data)` excludes Digimon.
//! Owner filter — opponent's Tamers are excluded.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn material_save_card(id: &str, n: u8) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(6),
        dp: Some(7000),
        play_cost: 7,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only MaterialSave: the auto-install MUST be the sole
        // source of behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::MaterialSave(n)],
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn tamer_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack so it
/// becomes the new visible top.
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

// ─── Test 1: happy path — N=2, 3 sources, 1 Tamer ──────────────────────────

/// Stack: [SRC-A, SRC-B, MAT-SAVE] (index 0 = bottom, 2 = top).
/// Activate MaterialSave(2): pick the Tamer, pick 2 of 3 sources.
/// Sources move to bottom of the Tamer's stack; carrier loses those sources.
#[test]
fn material_save_picks_tamer_and_two_sources_moves_under_tamer() {
    let mut r = DebugRunner::builder()
        .add_card(material_save_card("MAT-SAVE", 2))
        .add_card(tamer_card("TAMER"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .start();

    // Build the carrier with 2 sources under MAT-SAVE.
    //   index 0 = SRC-A, 1 = SRC-B, 2 = MAT-SAVE (top).
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-B");
    push_source_card(&mut r, 0, carrier.index as usize, "MAT-SAVE");
    let _tamer = r.place_on_field(0, "TAMER", None);

    // Carrier is field index 0; Tamer is field index 1.
    let carrier_field_index = carrier.index as usize;
    assert_eq!(carrier_field_index, 0);
    let tamer_field_index = 1usize;

    {
        let perm = &r.game.players[0].battle_area[carrier_field_index];
        assert_eq!(perm.card_sources.len(), 3, "preconditions: 3-card stack");
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "MAT-SAVE",
            "MAT-SAVE is the visible top"
        );
    }
    let tamer_stack_before = r.game.players[0].battle_area[tamer_field_index]
        .card_sources
        .len();

    // Enter main phase as the carrier's controller.
    r.game.enter_main_phase();
    r.game.set_memory(0);

    // Action mask must offer the [Main] activation for the carrier.
    {
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[field_main_bit(carrier_field_index)],
            1.0,
            "MaterialSave [Main] active skill must be in the action mask"
        );
    }

    // Activate the [Main] skill on the carrier.
    let fired = r.game.activate_field_main(0, carrier_field_index);
    assert!(fired, "MaterialSave activation must succeed");

    // First parked selection: pick a Tamer (mandatory).
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Tamer-pick selection must be parked first");
        assert_eq!(pending.selecting_player, 0);
        assert!(
            !pending.is_optional,
            "Tamer-pick is mandatory once activation gate passes"
        );
    }

    let tamer_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, tamer_action)
        .expect("pick Tamer");

    // Second parked selection: pick up to N sources from the carrier's stack.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("source-pick selection must be parked second");
        assert!(
            pending.is_optional,
            "source-pick is_optional_zero=true — player may pick 0..N sources"
        );
        // 2 candidate sources (SRC-A, SRC-B) — top is excluded.
        assert_eq!(pending.valid_action_ids.len(), 2);
    }

    // First source pick: action_ids[0] (SRC-A, the bottom source).
    let src_action_1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, src_action_1)
        .expect("first source pick");

    // After the first pick, count_capped chains to a second pick (max=2).
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("second source-pick parked");
        assert_eq!(pending.valid_action_ids.len(), 1);
    }
    let src_action_2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, src_action_2)
        .expect("second source pick");

    // Selection chain done.
    assert!(
        r.game.pending_selection.is_none(),
        "no pending selection after MaterialSave resolves"
    );

    // The 2 picked sources have moved to the bottom of the Tamer's stack.
    // Carrier now has only MAT-SAVE on top with no sources.
    let carrier_perm = &r.game.players[0].battle_area[carrier_field_index];
    assert_eq!(
        carrier_perm.card_sources.len(),
        1,
        "carrier stack reduced to top only (MAT-SAVE) after sources moved out"
    );
    assert_eq!(
        carrier_perm.top_card().card_id(&r.game.card_data),
        "MAT-SAVE",
        "MAT-SAVE remains the top of the carrier"
    );

    let tamer_perm = &r.game.players[0].battle_area[tamer_field_index];
    assert_eq!(
        tamer_perm.card_sources.len(),
        tamer_stack_before + 2,
        "Tamer gained exactly 2 source cards (the picked sources)"
    );
    // Both picked sources are now under the Tamer (at the bottom of its stack).
    // Phase D Task 3 ordering property: each picked source is placed at the
    // bottom of the Tamer's stack one at a time. SRC-A was picked first, so
    // it landed at index 0; the SRC-B pick then bottom-inserted, pushing
    // SRC-A to index 1. The original TAMER source remains the visible top.
    assert_eq!(
        tamer_perm.card_sources[0].card_id(&r.game.card_data),
        "SRC-B",
        "second-picked source (SRC-B) sits at the very bottom (index 0) after bottom-insert pushed SRC-A up"
    );
    assert_eq!(
        tamer_perm.card_sources[1].card_id(&r.game.card_data),
        "SRC-A",
        "first-picked source (SRC-A) is at index 1 — pushed up one slot by the second pick"
    );
    assert_eq!(
        tamer_perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "TAMER remains the visible top after receiving sources at the bottom"
    );

    // Sources must NOT be in trash — they were moved, not trashed.
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| matches!(c.card_id(&r.game.card_data), "SRC-A" | "SRC-B")),
        "moved sources must not be in trash"
    );
}

// ─── Test 2: no own Tamer → activation NOT in action mask ──────────────────

#[test]
fn material_save_with_no_own_tamer_does_not_appear_in_action_mask() {
    let mut r = DebugRunner::builder()
        .add_card(material_save_card("MAT-SAVE", 2))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .start();

    // Carrier with 2 sources but NO Tamer on field.
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-B");
    push_source_card(&mut r, 0, carrier.index as usize, "MAT-SAVE");
    let carrier_field_index = carrier.index as usize;

    r.game.enter_main_phase();
    r.game.set_memory(0);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(carrier_field_index)],
        0.0,
        "MaterialSave activation must NOT appear in the action mask when no own Tamer is on field"
    );

    // And direct activation must fail (the condition gate suppresses fire).
    let fired = r.game.activate_field_main(0, carrier_field_index);
    assert!(
        !fired,
        "MaterialSave activation must fail when no own Tamer is on field"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no selection parked when activation gate fails"
    );
}

// ─── Test 3: no card_sources under top → activation NOT in action mask ─────

#[test]
fn material_save_with_no_sources_does_not_appear_in_action_mask() {
    let mut r = DebugRunner::builder()
        .add_card(material_save_card("MAT-SAVE", 2))
        .add_card(tamer_card("TAMER"))
        .start();

    // Carrier with NO sources under top.
    let carrier = r.place_on_field(0, "MAT-SAVE", None);
    let _tamer = r.place_on_field(0, "TAMER", None);
    let carrier_field_index = carrier.index as usize;

    r.game.enter_main_phase();
    r.game.set_memory(0);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(carrier_field_index)],
        0.0,
        "MaterialSave activation must NOT appear in the mask when carrier has no sources under top"
    );

    let fired = r.game.activate_field_main(0, carrier_field_index);
    assert!(
        !fired,
        "MaterialSave activation must fail when no sources are present"
    );
}

// ─── Test 4: opponent's Tamers must NOT count toward the activation gate ───

/// Player 0 has the MaterialSave carrier with sources, but only player 1 has
/// a Tamer on field. The activation gate must NOT pass — DCGO restricts the
/// Tamer pool to OWN Tamers (`HasMatchConditionPermanent` against own
/// permanents in the auto-install body).
#[test]
fn material_save_does_not_count_opponent_tamer_for_gate() {
    let mut r = DebugRunner::builder()
        .add_card(material_save_card("MAT-SAVE", 2))
        .add_card(tamer_card("OPP-TAMER"))
        .add_card(plain_digimon("SRC-A"))
        .start();

    // Player 0: carrier with 1 source.
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "MAT-SAVE");
    let carrier_field_index = carrier.index as usize;

    // Player 1: Tamer (opponent).
    let _opp_tamer = r.place_on_field(1, "OPP-TAMER", None);

    r.game.enter_main_phase();
    r.game.set_memory(0);

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[field_main_bit(carrier_field_index)],
        0.0,
        "opponent-controlled Tamer must not satisfy the MaterialSave own-Tamer gate"
    );

    let fired = r.game.activate_field_main(0, carrier_field_index);
    assert!(
        !fired,
        "MaterialSave activation must fail when only an opponent Tamer is on field"
    );
}
