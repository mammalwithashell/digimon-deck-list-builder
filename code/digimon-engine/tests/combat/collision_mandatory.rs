//! Phase 9 Task 5 — Collision MUST-block enforcement.
//!
//! Today Collision expands the Blocker candidate pool to every
//! unsuspended opponent Digimon (see `combat.rs::try_enter_block`), but
//! the installed `PendingSelection` still carries `is_optional = true`
//! so the defender can PASS. Spec §8 flips that: when the attacker has
//! `<Collision>` AND the candidate list is non-empty, the selection is
//! MANDATORY (`is_optional = false`) and the action-mask drops PASS.
//!
//! When the candidate list is empty (every opponent Digimon is gated by
//! `CannotBlock`, is suspended, or simply isn't there), no selection is
//! installed and the attack proceeds normally — no special MUST-block
//! fallback.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn strong_digimon(card_id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 0,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(strong_digimon("ATK", 5000))
        .add_card(strong_digimon("DEF", 3000))
        .start()
}

/// Attacker has Collision and defender has ≥1 legal blocker candidate
/// (Collision grants Blocker to every unsuspended opponent Digimon).
/// The installed PendingSelection is MANDATORY — `is_optional = false`
/// — and the defender's action mask drops the PASS bit.
#[test]
fn collision_with_legal_blockers_mandates_selection() {
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF", Some(0));
    let _def_b = r.place_on_field(1, "DEF", Some(0));
    // No native Blocker keyword — Collision alone expands the pool.
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    let result = r.attack_digimon(atk, def_a, false);
    assert_eq!(result, AttackResult::InProgress);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("Collision installs a BlockTiming selection");
    assert!(
        !sel.is_optional,
        "Collision + non-empty candidate list must install a MANDATORY selection"
    );
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
        "both defender Digimon are Blocker candidates"
    );

    // Defender's mask: blocker bits set, PASS bit dropped.
    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_attack(0, 0) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 1) as usize], 1.0);
    assert_eq!(
        mask[PASS as usize], 0.0,
        "MUST-block selection drops the PASS bit"
    );
}

/// Attacker has Collision but every opponent Digimon is gated by
/// `ModifierType::CannotBlock`. Candidate list is empty → no selection
/// installed → attack proceeds synchronously against the original
/// target (no MUST-block, no soft lock).
#[test]
fn collision_without_legal_blockers_falls_back_to_optional() {
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);
    // Gate the only defender with CannotBlock — Collision cannot override
    // a `CannotBlock` restriction.
    r.game.modifiers.add(
        def,
        ModifierEntry::simple(ModifierType::CannotBlock, 1, Expiry::Permanent, 1),
    );

    let result = r.attack_digimon(atk, def, false);
    // No blocker candidates → attack resolves synchronously. ATK (5000)
    // beats DEF (3000).
    assert_eq!(result, AttackResult::AttackerWins);
    assert!(
        r.game.pending_selection.is_none(),
        "no selection installed when the candidate list is empty"
    );
    assert!(r.game.pending_attack.is_none(), "attack cleaned up");
}

/// Attacker has Collision. Defender has three Digimon; one carries a
/// `CannotBlock` restriction. Only the two unrestricted Digimon appear
/// as candidates; PASS bit is still dropped (MUST-block applies because
/// the candidate list is non-empty).
#[test]
fn collision_combined_with_cannot_block_excludes_restricted_blockers() {
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def_a = r.place_on_field(1, "DEF", Some(0));
    let _def_b = r.place_on_field(1, "DEF", Some(0));
    let def_c = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);
    // Gate the third defender with CannotBlock — Collision does NOT
    // override CannotBlock (they compose: Collision promotes Blocker,
    // CannotBlock forbids blocking outright).
    r.game.modifiers.add(
        def_c,
        ModifierEntry::simple(ModifierType::CannotBlock, 1, Expiry::Permanent, 1),
    );

    let result = r.attack_digimon(atk, def_a, false);
    assert_eq!(result, AttackResult::InProgress);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("non-empty candidate pool installs a selection");
    assert!(!sel.is_optional, "non-empty pool + Collision = MUST-block");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
        "only the two unrestricted defenders are candidates"
    );

    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_attack(0, 0) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 1) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 2) as usize], 0.0, "CannotBlock gated");
    assert_eq!(mask[PASS as usize], 0.0, "MUST-block drops PASS");
}
