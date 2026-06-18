//! `CannotSuspend` / `CannotUnsuspend` enforcement tests.
//!
//! Rules authority (`Digimon TCG resources/general_rule.pdf`):
//!   - 11-2-5 — "An attack declaration can't be made using a Digimon that
//!     can't suspend."
//!   - 12-1-4 — "A block can't be performed using a Digimon that can't
//!     suspend."
//!   - 15-1-3 — "A prohibiting effect takes precedence over an enabling
//!     [effect]" — a "can't suspend" prohibition also stops suspension by
//!     effects and cost payments (and "can't unsuspend" stops unsuspension
//!     by effects).
//!
//! DCGO consult chokepoints mirrored here:
//!   - `Permanent.CanAttackTargetDigimon` (`Permanent.cs:2230`) — attack
//!     declaration legality consults `!CanSuspend`.
//!   - `Permanent.CanBlock` (`Permanent.cs:2140`) — block legality consults
//!     `!CanSuspend`.
//!   - `CardController.SuspendPermanentsClass.Tap()` (`CardController.cs:5633`)
//!     — the universal suspend executor filters out `!CanSuspend` permanents
//!     (covers effect suspends AND cost payments).
//!   - `CardController.IUnsuspendPermanents.Unsuspend()`
//!     (`CardController.cs:5716`) — the universal unsuspend executor filters
//!     out `!CanUnsuspend` permanents (effect unsuspends no-op).
//!   - `Alliance.cs:18` / `Evade.cs:28` — suspend-as-cost candidacy requires
//!     `CanActivate(Permanent)SuspendCostEffect` → `CanSuspend`.
//!
//! Cards shipping the modifier today: BT25-026, BT25-028, EX9-019, BT20-084,
//! EX8-023, EX7-023, EX11-017 (CannotSuspend); BT13-060, BT20-037, BT25-050/
//! 053/058/061, BT24-095 (CannotUnsuspend).

use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::action::{build_action_mask, ATTACK_END, ATTACK_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn digimon(card_id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn lock(r: &mut DebugRunner, handle: PermanentHandle, modifier: ModifierType, expiry: Expiry) {
    let source_player = 1 - handle.player;
    r.game
        .modifiers
        .add(handle, ModifierEntry::simple(modifier, 0, expiry, source_player));
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(digimon("ATK", 5000, Vec::new()))
        .add_card(digimon("ATK2", 5000, Vec::new()))
        .add_card(digimon("DEF", 3000, Vec::new()))
        .start()
}

// ─── Attack declaration — mask (11-2-5) ─────────────────────────────────────

/// A CannotSuspend-locked attacker must lose ALL of its attack bits in the
/// Main-phase mask; an unlocked sibling on the same field keeps its bits.
#[test]
fn cannot_suspend_zeroes_attack_bits_for_locked_attacker_only() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(tp, "ATK", Some(0));
    let free = r.place_on_field(tp, "ATK2", Some(0));
    // A suspended defender so Digimon-target attack bits exist too.
    let def = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[def.index as usize].is_suspended = true;

    r.game.enter_main_phase();

    let before = build_action_mask(&r.game, tp);
    assert_eq!(
        before[encode_attack(locked.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "sanity: security-attack bit lit before the lock"
    );
    assert_eq!(
        before[encode_attack(locked.index as u16, def.index as u16) as usize],
        1.0,
        "sanity: digimon-attack bit lit before the lock"
    );

    lock(&mut r, locked, ModifierType::CannotSuspend, Expiry::Permanent);

    let after = build_action_mask(&r.game, tp);
    for target in 0..=SECURITY_TARGET {
        let bit = encode_attack(locked.index as u16, target);
        if (bit as usize) < ATTACK_END as usize && bit >= ATTACK_START {
            assert_eq!(
                after[bit as usize], 0.0,
                "locked attacker must expose NO attack bit (target {})",
                target
            );
        }
    }
    assert_eq!(
        after[encode_attack(free.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "the unlocked sibling keeps its security-attack bit"
    );
}

/// A permanent-scoped `CannotAttack`-locked attacker must lose ALL of its
/// attack bits in the Main-phase mask; an unlocked sibling keeps its bits.
///
/// Regression: `can_basic_attack` (the Main-phase mask gate) once checked
/// only the player-scoped CannotAttack short-circuit and `CannotSuspend`, so
/// a permanent-scoped `CannotAttack` (e.g. a Tamer locking one opposing
/// Digimon) still emitted attack bits. The engine's `can_attack` consult then
/// refused the declaration, leaving a no-op the mask still offered — a
/// deterministic policy re-picked it forever (the no-op attack loop the
/// training tripwire caught: `Grizzlymon+CannotAttack` offered action 114).
#[test]
fn cannot_attack_zeroes_attack_bits_for_locked_attacker_only() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(tp, "ATK", Some(0));
    let free = r.place_on_field(tp, "ATK2", Some(0));
    // A suspended defender so Digimon-target attack bits exist too.
    let def = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[def.index as usize].is_suspended = true;

    r.game.enter_main_phase();

    let before = build_action_mask(&r.game, tp);
    assert_eq!(
        before[encode_attack(locked.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "sanity: security-attack bit lit before the lock"
    );
    assert_eq!(
        before[encode_attack(locked.index as u16, def.index as u16) as usize],
        1.0,
        "sanity: digimon-attack bit lit before the lock"
    );

    lock(&mut r, locked, ModifierType::CannotAttack, Expiry::Permanent);

    let after = build_action_mask(&r.game, tp);
    for target in 0..=SECURITY_TARGET {
        let bit = encode_attack(locked.index as u16, target);
        if (bit as usize) < ATTACK_END as usize && bit >= ATTACK_START {
            assert_eq!(
                after[bit as usize], 0.0,
                "CannotAttack-locked attacker must expose NO attack bit (target {})",
                target
            );
        }
    }
    assert_eq!(
        after[encode_attack(free.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "the unlocked sibling keeps its security-attack bit"
    );
}

// ─── Attack declaration — engine API (11-2-5) ───────────────────────────────

/// The combat API itself must reject the declaration (mask and engine must
/// AGREE): `attack_player` / `attack_digimon` return `Invalid`, no security
/// is checked, and the locked Digimon stays unsuspended.
#[test]
fn cannot_suspend_rejects_attack_declaration_at_api() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(tp, "ATK", Some(0));
    let def = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[def.index as usize].is_suspended = true;

    lock(&mut r, locked, ModifierType::CannotSuspend, Expiry::Permanent);

    let security_before = r.security_count(opp);
    let result = r.attack_player(locked, opp, false);
    assert_eq!(
        result,
        AttackResult::Invalid,
        "11-2-5: an attack declaration can't be made using a Digimon that can't suspend"
    );
    assert_eq!(
        r.security_count(opp),
        security_before,
        "no security check may resolve from the rejected declaration"
    );
    assert!(
        !r.game.players[tp as usize].battle_area[locked.index as usize].is_suspended,
        "the locked attacker must remain unsuspended"
    );

    let result = r.attack_digimon(locked, def, false);
    assert_eq!(
        result,
        AttackResult::Invalid,
        "Digimon-target declarations are equally rejected"
    );
}

/// Expiry restores the ability: once the modifier expires the same Digimon
/// attacks normally.
#[test]
fn cannot_suspend_expiry_restores_attack() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(tp, "ATK", Some(0));
    let _def = r.place_on_field(opp, "DEF", Some(0));

    lock(&mut r, locked, ModifierType::CannotSuspend, Expiry::EndOfTurn);
    assert_eq!(
        r.attack_player(locked, opp, false),
        AttackResult::Invalid,
        "locked while the modifier is live"
    );

    r.game.modifiers.expire_end_of_turn(tp);
    assert!(
        !r.game.modifiers.has(locked, ModifierType::CannotSuspend),
        "sanity: the modifier expired"
    );

    let result = r.attack_player(locked, opp, false);
    let _ = r.auto_resolve();
    assert_ne!(
        result,
        AttackResult::Invalid,
        "after expiry the attack declaration is legal again"
    );
}

// ─── Block declaration (12-1-4) ─────────────────────────────────────────────

/// A Blocker under CannotSuspend is not a block candidate: with it as the
/// only Blocker, no BlockTiming window opens and the attack resolves
/// directly against the original target.
#[test]
fn cannot_suspend_excludes_blocker_from_block_window() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5000, Vec::new()))
        .add_card(digimon("DEF", 3000, Vec::new()))
        .add_card(digimon("BLK", 9000, Vec::new()))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    lock(&mut r, blk, ModifierType::CannotSuspend, Expiry::Permanent);

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "12-1-4: a block can't be performed using a Digimon that can't suspend — no window"
    );
    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "the attack resolves synchronously against the original target"
    );
    // DEF was deleted by the battle, so re-locate BLK by card id.
    let blk_perm = r.game.players[1]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BLK")
        .expect("the locked Blocker survived (it never blocked)");
    assert!(
        !blk_perm.is_suspended,
        "the locked Blocker never suspends"
    );
}

/// With one locked and one free Blocker, the BlockTiming candidate list
/// contains ONLY the free one.
#[test]
fn cannot_suspend_filters_block_candidate_list() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5000, Vec::new()))
        .add_card(digimon("DEF", 3000, Vec::new()))
        .add_card(digimon("BLK", 9000, Vec::new()))
        .add_card(digimon("BLK2", 9000, Vec::new()))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let locked_blk = r.place_on_field(1, "BLK", Some(0));
    let free_blk = r.place_on_field(1, "BLK2", Some(0));
    r.game
        .modifiers
        .grant_keyword(locked_blk, Keyword::Blocker, Expiry::Permanent, 1);
    r.game
        .modifiers
        .grant_keyword(free_blk, Keyword::Blocker, Expiry::Permanent, 1);

    lock(&mut r, locked_blk, ModifierType::CannotSuspend, Expiry::Permanent);

    let result = r.attack_digimon(atk, def, false);
    assert_eq!(result, AttackResult::InProgress, "free Blocker opens the window");
    assert_eq!(r.current_phase(), GamePhase::BlockTiming);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("BlockTiming selection parked");
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_attack(0, free_blk.index as u16)],
        "only the unlocked Blocker is a legal block candidate"
    );
}

// ─── Effect suspension no-ops (15-1-3) ──────────────────────────────────────

/// An effect that suspends a CannotSuspend permanent no-ops (DCGO Tap()
/// filter); an unlocked control target still suspends.
#[test]
fn cannot_suspend_makes_effect_suspend_noop() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(opp, "DEF", Some(0));
    let free = r.place_on_field(opp, "ATK2", Some(0));

    lock(&mut r, locked, ModifierType::CannotSuspend, Expiry::Permanent);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.suspend(locked);
        ctx.suspend(free);
    }

    assert!(
        !r.game.players[opp as usize].battle_area[locked.index as usize].is_suspended,
        "15-1-3: the prohibiting CannotSuspend wins — effect suspension no-ops"
    );
    assert!(
        r.game.players[opp as usize].battle_area[free.index as usize].is_suspended,
        "control: the unlocked target still suspends"
    );
}

/// CannotUnsuspend (the sibling modifier) symmetrically stops EFFECT
/// unsuspension at the same chokepoint (DCGO IUnsuspendPermanents filter).
/// The unsuspend-phase gate already exists (game_phases.rs); this covers the
/// effect path.
#[test]
fn cannot_unsuspend_makes_effect_unsuspend_noop() {
    let mut r = base_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(opp, "DEF", Some(0));
    let free = r.place_on_field(opp, "ATK2", Some(0));
    r.game.players[opp as usize].battle_area[locked.index as usize].is_suspended = true;
    r.game.players[opp as usize].battle_area[free.index as usize].is_suspended = true;

    lock(&mut r, locked, ModifierType::CannotUnsuspend, Expiry::Permanent);

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, tp);
        ctx.unsuspend(locked);
        ctx.unsuspend(free);
    }

    assert!(
        r.game.players[opp as usize].battle_area[locked.index as usize].is_suspended,
        "the CannotUnsuspend permanent stays suspended through effect unsuspension"
    );
    assert!(
        !r.game.players[opp as usize].battle_area[free.index as usize].is_suspended,
        "control: the unlocked target still unsuspends"
    );
}

// ─── Suspend-as-cost (15-1-3; DCGO CanActivateSuspendCostEffect) ────────────

/// `suspend_self_as_cost` (Tamer "by suspending this Tamer" family) must
/// report the cost as unpayable for a locked carrier.
#[test]
fn cannot_suspend_blocks_suspend_self_as_cost() {
    let mut r = base_runner();
    let tp = r.game.turn_player();

    let locked = r.place_on_field(tp, "ATK", Some(0));
    lock(&mut r, locked, ModifierType::CannotSuspend, Expiry::Permanent);

    let paid = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(locked), tp);
        ctx.suspend_self_as_cost()
    };
    assert!(
        !paid,
        "a CannotSuspend carrier cannot pay a suspend-self activation cost"
    );
    assert!(
        !r.game.players[tp as usize].battle_area[locked.index as usize].is_suspended,
        "the carrier stays unsuspended"
    );

    // Control: without the lock the cost is payable.
    let free = r.place_on_field(tp, "ATK2", Some(0));
    let paid = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(free), tp);
        ctx.suspend_self_as_cost()
    };
    assert!(paid, "control: an unlocked carrier pays the cost normally");
}

/// Alliance's ally suspension is a cost (DCGO `Alliance.cs:18`
/// `CanActivateSuspendCostEffect`): a locked ally is not a candidate. With
/// one locked and one free ally, the AllianceTiming candidate list contains
/// only the free one.
#[test]
fn cannot_suspend_filters_alliance_ally_candidates() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5000, vec![Keyword::Alliance]))
        .add_card(digimon("ALLY", 4000, Vec::new()))
        .add_card(digimon("ALLY2", 4000, Vec::new()))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let locked_ally = r.place_on_field(0, "ALLY", Some(0));
    let free_ally = r.place_on_field(0, "ALLY2", Some(0));

    lock(&mut r, locked_ally, ModifierType::CannotSuspend, Expiry::Permanent);

    let result = r.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::InProgress,
        "the free ally still opens the Alliance window"
    );
    assert_eq!(r.current_phase(), GamePhase::AllianceTiming);
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Alliance selection parked");
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_attack(0, free_ally.index as u16)],
        "only the unlocked ally can pay the Alliance suspend cost"
    );
}

/// Evade's cost is suspending the carrier (DCGO `Evade.cs:28`
/// `CanActivatePermanentSuspendCostEffect` → `CanSuspend`): a locked carrier
/// cannot pay it — no accept dialog parks and the deletion proceeds.
#[test]
fn cannot_suspend_blocks_evade_cost() {
    let mut r = DebugRunner::builder()
        .add_card(digimon("EVADE", 4000, vec![Keyword::Evade]))
        .start();
    let evade = r.place_on_field(0, "EVADE", None);

    lock(&mut r, evade, ModifierType::CannotSuspend, Expiry::Permanent);

    r.game.delete_permanent_with_effects(evade);

    assert!(
        r.game.pending_selection.is_none(),
        "Evade cost unpayable under CannotSuspend — no accept dialog"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "the deletion proceeded normally"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EVADE"),
        "EVADE landed in trash"
    );
}
