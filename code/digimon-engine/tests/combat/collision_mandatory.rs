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
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
        vec![encode_attack(0, 1)],
        "only the distinct defender is a Blocker candidate"
    );

    // Defender's mask: blocker bits set, PASS bit dropped.
    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_attack(0, 0) as usize], 0.0, "current target cannot block");
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
        vec![encode_attack(0, 1)],
        "only the unrestricted non-target defender is a candidate"
    );

    let mask = build_action_mask(&r.game, 1);
    assert_eq!(mask[encode_attack(0, 0) as usize], 0.0, "current target cannot block");
    assert_eq!(mask[encode_attack(0, 1) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, 2) as usize], 0.0, "CannotBlock gated");
    assert_eq!(mask[PASS as usize], 0.0, "MUST-block drops PASS");
}

// ─── §16-30 declarative <Blocker> grant (G-ENGINE-COLLISION-BLOCKER-NOT-GRANTED) ──

fn printed_blocker(card_id: &str) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        keywords: vec![Keyword::Blocker],
        ..strong_digimon(card_id, 3000)
    }
}

fn collision_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(strong_digimon("ATK", 5000))
        .add_card(strong_digimon("DEF", 3000))
        .add_card(printed_blocker("PBLK"))
        .start()
}

fn unaffected_by_everything(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotBeAffected, 0, Expiry::Permanent, h.player),
    );
}

/// §16-30-1: "all of your OPPONENT's Digimon gain <Blocker>" -- the attacker's
/// own other Digimon do not, and both keyword lookups agree.
#[test]
fn collision_grant_is_for_opponent_digimon_only() {
    let mut r = collision_runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let own = r.place_on_field(0, "DEF", Some(0));
    let opp = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    r.attack_player(atk, 1, false);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    assert!(r.game.has_keyword(opp, Keyword::Blocker));
    assert!(r.game.has_keyword_type(opp, &Keyword::Blocker));
    assert!(!r.game.has_keyword(own, Keyword::Blocker), "own Digimon");
    assert!(!r.game.has_keyword_type(own, &Keyword::Blocker));
    assert!(!r.game.has_keyword(atk, Keyword::Blocker), "the attacker");
}

/// §16-30-4: the grant affects Digimon (an unaffected one gains nothing), the
/// forced block affects the PLAYER. A Digimon unaffected by the attacker's
/// effects that already has a PRINTED <Blocker> is still a forced candidate;
/// the unaffected one without it is not offered.
#[test]
fn collision_unaffected_printed_blocker_is_still_a_forced_candidate() {
    let mut r = collision_runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let plain = r.place_on_field(1, "DEF", Some(0));
    let printed = r.place_on_field(1, "PBLK", Some(0));
    unaffected_by_everything(&mut r, plain);
    unaffected_by_everything(&mut r, printed);
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);

    r.attack_player(atk, 1, false);
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert!(r.game.has_keyword(printed, Keyword::Blocker), "printed");
    let sel = r.game.pending_selection.as_ref().expect("block window");
    assert!(!sel.is_optional, "the player is forced to block");
    assert_eq!(sel.valid_action_ids, vec![encode_attack(0, 1)]);
}

/// §16-30-1 "While a Digimon with this effect is attacking": once the attack
/// is over the opponent's Digimon no longer have <Blocker>.
#[test]
fn collision_grant_is_gone_after_the_attack() {
    let mut r = collision_runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let opp = r.place_on_field(1, "DEF", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Collision, Expiry::Permanent, 0);
    assert!(!r.game.has_keyword(opp, Keyword::Blocker), "before");

    r.attack_player(atk, 1, false);
    assert!(r.game.has_keyword(opp, Keyword::Blocker), "during");
    r.game
        .resolve_selection(1, encode_attack(0, 0))
        .expect("the forced block");
    assert!(r.game.pending_attack.is_none(), "attack over");
    assert!(!r.game.has_keyword(opp, Keyword::Blocker), "after");
    assert!(!r.game.has_keyword_type(opp, &Keyword::Blocker), "after");
}

/// The grant follows the Collision attacker's IDENTITY, not its battle-area
/// slot (`Game::live_attack_attacker`): removing an earlier-slot Digimon of
/// the attacker's side mid-attack (compaction) leaves the grant on, and
/// removing the Collision attacker itself takes the grant away with it.
/// (`trash_permanent_stack` is the raw removal: a deletion cannot be staged
/// while the block prompt is the pending selection.)
#[test]
fn collision_grant_tracks_attacker_identity_through_compaction_and_deletion() {
    // <Collision> is PRINTED here: the raw slot removal below does not re-key
    // handle-keyed modifiers the way a real deletion does.
    let mut r = DebugRunner::builder()
        .add_card(strong_digimon("DEF", 3000))
        .add_card(digimon_engine::card_data::CardData {
            keywords: vec![Keyword::Collision],
            ..strong_digimon("COLL", 5000)
        })
        .start();
    let bystander = r.place_on_field(0, "DEF", Some(0));
    let atk = r.place_on_field(0, "COLL", Some(0));
    let opp = r.place_on_field(1, "DEF", Some(0));
    assert_eq!((bystander.index, atk.index), (0, 1));

    r.attack_player(atk, 1, false);
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    assert!(r.game.has_keyword(opp, Keyword::Blocker));

    // Earlier-slot bystander leaves: the attacker slides to slot 0.
    r.game.trash_permanent_stack(0, bystander.index as usize);
    let moved = r.perm_handle(0, 0);
    assert_eq!(r.battle_area_size(0), 1, "the attacker compacted into slot 0");
    assert!(r.game.has_keyword(moved, Keyword::Collision));
    assert!(
        r.game.has_keyword(opp, Keyword::Blocker),
        "the Collision attacker is still attacking"
    );

    // The attacker itself is deleted: no attacker, no grant.
    r.game.trash_permanent_stack(0, moved.index as usize);
    assert_eq!(r.battle_area_size(0), 0, "the attacker is gone");
    assert!(!r.game.has_keyword(opp, Keyword::Blocker));
    assert!(!r.game.has_keyword_type(opp, &Keyword::Blocker));
}
