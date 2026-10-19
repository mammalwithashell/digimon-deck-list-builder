//! End-of-turn mask parity tests.
//!
//! <Vortex> and <Overclock> (Comprehensive Rules 4.3 §16-33-2 / §16-34-2) are
//! trigger-type effects at the end of your turn: since 2026-10-18
//! (G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS) their decision is
//! offered INSIDE the `[End of Your Turn]` pool, as a pending selection, so
//! these tests end the turn (`end_turn`) and read the mask of that decision:
//!   - `PASS` (the decline, §16-33-3)
//!   - Digimon-target attack bits for the <Vortex> Digimon whose
//!     `can_attack(vortex=true)` returns true. Vortex bypasses summoning
//!     sickness (via §2.1 plumbing) but NOT the target rule: only a suspended
//!     enemy Digimon, or one the attacker may attack unsuspended
//!     (CAN_ATTACK_UNSUSPENDED), is a valid target (§11-2-7-1, §16-33-1);
//!     <Raid> switches the target after the declaration (§16-23-1) and adds
//!     none. Player/security targets require a separate
//!     `VortexCanAttackPlayer` extension.
//!
//! The `EndOfTurnAction` window itself remains for `MAY_ATTACK` /
//! `FORCE_ATTACK` grants; those tests still set the phase directly.
//!

use digimon_engine::action::{
    build_action_mask, encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK,
    FIELD_EFFECT_START, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn make_digimon(id: &str, color: CardColor, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![color],
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Vortex grants a bit for a suspended enemy Digimon when the phase is
/// active, but base Vortex does not grant player/security attacks.
#[test]
fn mask_vortex_emits_attacks_in_end_of_turn_phase() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let def_idx = r.place_on_field(opp, "DEF", Some(0)).index;
    // Opponent starts suspended so the Main-phase pathway would also allow
    // this attack; we want to confirm it flows through the EndOfTurnAction
    // arm instead.
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS declines the <Vortex> decision"
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "base Vortex does not permit attacking security",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, def_idx as u16) as usize],
        1.0,
        "Vortex permits attacking enemy Digimon",
    );
}

#[test]
fn base_vortex_does_not_emit_security_target_without_player_extension() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "base Vortex should not emit a security/player target bit",
    );
}

#[test]
fn vortex_player_extension_emits_security_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::VortexCanAttackPlayer,
            1,
            Expiry::EndOfTurn,
            tp,
        ),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "VortexCanAttackPlayer should extend Vortex to player/security targets",
    );
}

#[test]
fn vortex_player_extension_respects_cannot_attack_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::VortexCanAttackPlayer,
            1,
            Expiry::EndOfTurn,
            tp,
        ),
    );
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::CannotAttackPlayer, 1, Expiry::EndOfTurn, tp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "CannotAttackPlayer should suppress the Vortex player/security extension",
    );
}

#[test]
fn fresh_vortex_may_attack_without_player_extension_does_not_emit_security_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "MayAttack must not borrow Vortex's summoning-sickness bypass for security targets",
    );
}

#[test]
fn fresh_vortex_may_attack_without_player_extension_does_not_decode_security_attack() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    r.game
        .decode_action(encode_attack(attacker.index as u16, SECURITY_TARGET), tp);

    assert!(
        !r.game.players[tp as usize].battle_area[attacker.index as usize].is_suspended,
        "decode should not execute a fresh MayAttack security attack via Vortex bypass",
    );
}

#[test]
fn fresh_vortex_player_extension_emits_and_decodes_security_as_vortex() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(
            ModifierType::VortexCanAttackPlayer,
            1,
            Expiry::EndOfTurn,
            tp,
        ),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let security_attack = encode_attack(attacker.index as u16, SECURITY_TARGET);
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[security_attack as usize], 1.0,
        "VortexCanAttackPlayer should allow a fresh Vortex security attack",
    );

    r.game.decode_action(security_attack, tp);

    assert!(
        r.game.players[tp as usize].battle_area[attacker.index as usize].is_suspended,
        "decoded fresh security attack should execute via Vortex and suspend the attacker",
    );
}

#[test]
fn fresh_vortex_may_attack_emits_and_decodes_digimon_target_as_vortex() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", None);
    let defender = r.place_on_field(opp, "DEF", Some(0));
    // Only a suspended Digimon is a legal <Vortex> target (§11-2-7-1,
    // §16-33-1).
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let digimon_attack = encode_attack(attacker.index as u16, defender.index as u16);
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[digimon_attack as usize], 1.0,
        "fresh Vortex + MayAttack should still expose Digimon targets via Vortex",
    );

    r.game.decode_action(digimon_attack, tp);

    assert_eq!(
        r.game.players[opp as usize].battle_area.len(),
        0,
        "decoded fresh Digimon attack should execute via Vortex and delete DEF",
    );
}

/// Without a Vortex grant, `EndOfTurnAction` only emits PASS — no attack
/// bits.
#[test]
fn mask_vortex_without_keyword_only_emits_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let def_idx = r.place_on_field(opp, "DEF", Some(0)).index;

    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(mask[PASS as usize], 1.0);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "no Vortex keyword → no security attack",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, def_idx as u16) as usize],
        0.0,
        "no Vortex keyword → no digimon attack",
    );
}

/// Vortex bypasses summoning sickness — a freshly-played permanent with
/// Vortex granted can attack. Regression guard for the §2.1 `vortex=true`
/// plumbing in `can_attack`.
#[test]
fn mask_vortex_bypasses_summoning_sickness() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    // place_on_field with None uses current turn_count → summoning-sick.
    let attacker = r.place_on_field(tp, "ATK", None);
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        1.0,
        "Vortex must bypass summoning sickness (relies on can_attack(vortex=true))",
    );
}

#[test]
fn decode_vortex_attack_uses_vortex_flag_for_fresh_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", None);
    let defender = r.place_on_field(opp, "DEF", Some(0));
    // Only a suspended Digimon is a legal <Vortex> target (§11-2-7-1,
    // §16-33-1).
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let attack = encode_attack(attacker.index as u16, defender.index as u16);
    r.game.decode_action(attack, tp);

    assert_eq!(
        r.game.players[opp as usize].battle_area.len(),
        0,
        "decoded EndOfTurnAction Vortex attack should bypass summoning sickness and delete DEF",
    );
}

/// §11-2-7-1 "they choose either the opponent or 1 of the opponent's
/// suspended Digimon as the attack target"; §16-33-1 <Vortex> grants the
/// attack, not a target permission (contrast <Execute> §16-38-1; W2 reading
/// (a), DCGO `CanAttackTargetDigimon` requires `IsSuspended || isExecute`).
/// Inverted from `mask_vortex_targets_unsuspended_digimon_too`, which pinned
/// "any enemy Digimon is valid" (G-ENGINE-VORTEX-ATTACKS-UNSUSPENDED-DIGIMON):
/// with both enemy Digimon unsuspended no Vortex target bit is lit, and
/// suspending one lights exactly that one.
#[test]
fn mask_vortex_does_not_target_unsuspended_digimon() {
    // `suspend_weak`: whether WEAK is suspended before the turn ends.
    let board = |suspend_weak: bool| {
        let mut r = DebugRunner::builder()
            .add_card(make_digimon("ATK", CardColor::Red, 5000))
            .add_card(make_digimon_without_dp("WEAK", CardColor::Blue))
            .add_card(make_digimon_without_dp("STRONG", CardColor::Blue))
            .start();

        let tp = r.game.turn_player();
        let opp = 1 - tp;
        let attacker = r.place_on_field(tp, "ATK", Some(0));
        let weak_idx = r.place_on_field(opp, "WEAK", Some(0)).index;
        let strong_idx = r.place_on_field(opp, "STRONG", Some(0)).index;
        // Both unsuspended.
        for p in &mut r.game.players[opp as usize].battle_area {
            p.is_suspended = false;
        }
        r.game.players[opp as usize].battle_area[weak_idx as usize].is_suspended = suspend_weak;

        r.game
            .modifiers
            .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
        // §16-33-2: the decision is offered inside the `[End of Your Turn]`
        // pool.
        r.game.end_turn();
        let mask = build_action_mask(&r.game, tp);
        (
            mask[encode_attack(attacker.index as u16, weak_idx as u16) as usize],
            mask[encode_attack(attacker.index as u16, strong_idx as u16) as usize],
            r.game.pending_selection.is_some(),
        )
    };

    // Both unsuspended: no legal <Vortex> declaration, so it does not activate
    // (DCGO `CanActivateVortex`) and nothing is offered.
    let (weak, strong, offered) = board(false);
    assert_eq!(weak, 0.0, "an unsuspended enemy Digimon is not a <Vortex> target (weak)");
    assert_eq!(strong, 0.0, "an unsuspended enemy Digimon is not a <Vortex> target (strong)");
    assert!(!offered, "no legal declaration: <Vortex> does not activate");

    // WEAK suspended: exactly WEAK is offered.
    let (weak, strong, offered) = board(true);
    assert!(offered, "<Vortex> offers its attack");
    assert_eq!(weak, 1.0, "the suspended enemy Digimon is a <Vortex> target");
    assert_eq!(
        strong, 0.0,
        "the unsuspended enemy Digimon is still not a <Vortex> target"
    );
}

/// §11-2-7-1 leaves room for an effect that lets a Digimon attack an
/// unsuspended Digimon (§16-38-1 <Execute>; DCGO
/// `Permanent.CanAttackTargetDigimon` accepts such a grant). With a
/// `CanAttackUnsuspended` grant a summoning-sick <Vortex> Digimon may attack
/// an UNSUSPENDED enemy Digimon at the end of turn: the mask lights the bit,
/// and the decoder agrees and executes it with the Vortex flag (the
/// played-this-turn exemption, §16-33-1).
#[test]
fn vortex_with_can_attack_unsuspended_targets_an_unsuspended_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", None);
    let defender = r.place_on_field(opp, "DEF", Some(0));
    assert!(!r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::CanAttackUnsuspended, 1, Expiry::EndOfTurn, tp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let attack = encode_attack(attacker.index as u16, defender.index as u16);
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[attack as usize], 1.0,
        "CanAttackUnsuspended makes the unsuspended Digimon a <Vortex> target",
    );

    r.game.decode_action(attack, tp);
    assert!(r.game.pending_attack.is_none(), "the attack resolved");
    assert_eq!(
        r.game.players[opp as usize].battle_area.len(),
        0,
        "the decoder accepted the target (Vortex flag, fresh attacker) and DEF lost",
    );
}

/// §11-2-7-1: the attack declaration targets the opponent or a SUSPENDED
/// Digimon; §16-23-1 / §16-23-2: <Raid> SWITCHES the target after the
/// declaration (a [When Attacking] trigger in the attack's pool), it adds no
/// declaration target. A <Vortex> + <Raid> Digimon at the end of turn: the
/// mask and the decoder offer only the suspended `SUS`, not the unsuspended
/// `LOW` / `HIGH` (the highest-DP one included); declared on `SUS`, <Raid>
/// then offers its optional switch to `HIGH` (the highest-DP unsuspended
/// Digimon), and the switched attack battles `HIGH`.
///
/// Re-pinned from `vortex_with_raid_declares_on_highest_dp_unsuspended_today`
/// (which pinned the declaration-time <Raid> arm;
/// G-ENGINE-RAID-DECLARATION-TARGET-UNSUSPENDED, resolved). DCGO:
/// `Permanent.CanAttackTargetDigimon` has no <Raid> arm; `Raid.cs` switches on
/// `OnAllyAttack`.
#[test]
fn vortex_with_raid_declares_on_a_legal_target_then_may_switch() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("LOW", CardColor::Blue, 3000))
        .add_card(make_digimon("HIGH", CardColor::Blue, 4000))
        .add_card(make_digimon("SUS", CardColor::Blue, 1000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", None);
    let low = r.place_on_field(opp, "LOW", Some(0));
    let high = r.place_on_field(opp, "HIGH", Some(0));
    let sus = r.place_on_field(opp, "SUS", Some(0));
    r.game.players[opp as usize].battle_area[sus.index as usize].is_suspended = true;
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Raid, Expiry::EndOfTurn, tp);
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let low_attack = encode_attack(attacker.index as u16, low.index as u16);
    let high_attack = encode_attack(attacker.index as u16, high.index as u16);
    let sus_attack = encode_attack(attacker.index as u16, sus.index as u16);
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[low_attack as usize], 0.0, "not the lower-DP unsuspended Digimon");
    assert_eq!(
        mask[high_attack as usize], 0.0,
        "not the highest-DP unsuspended Digimon either: <Raid> adds no declaration target \
         (§11-2-7-1, §16-23-1)",
    );
    assert_eq!(mask[sus_attack as usize], 1.0, "the suspended Digimon");

    // The decoder agrees: a declaration on HIGH is refused (nothing happens;
    // the <Vortex> decision is still the pending one).
    r.game.decode_action(high_attack, tp);
    assert!(r.game.pending_attack.is_none(), "the decoder refused HIGH");
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|sel| sel.valid_action_ids.contains(&sus_attack)),
        "no <Raid> prompt: still the <Vortex> decision"
    );
    assert_eq!(r.game.players[opp as usize].battle_area.len(), 3);

    r.game.decode_action(sus_attack, tp);
    // <Raid> triggers (§16-23-2) and offers its optional switch to the
    // highest-DP unsuspended Digimon (HIGH).
    let sel = r.game.pending_selection.as_ref().expect("the <Raid> switch prompt");
    assert_eq!(sel.selecting_player, tp);
    assert!(sel.is_optional);
    assert_eq!(sel.valid_action_ids, vec![encode_attack(0, high.index as u16)]);
    r.game
        .resolve_selection(tp, encode_attack(0, high.index as u16))
        .expect("switch to HIGH");
    assert!(r.game.pending_attack.is_none(), "the attack resolved");
    let left: Vec<String> = r.game.players[opp as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        left,
        vec!["LOW".to_string(), "SUS".to_string()],
        "the switched attack battled HIGH; SUS was not battled"
    );
}

// Helper with distinct DP so we can verify Vortex target legality ignores
// DP (§11-2-7-1: a suspended Digimon or a CanAttackUnsuspended grant; DP
// matters only to the <Raid> switch trigger after the declaration, §16-23-1).
fn make_digimon_without_dp(id: &str, color: CardColor) -> CardData {
    make_digimon(id, color, 3000)
}

/// Vortex attack bits (EndOfTurnAction arm) must also honor
/// CannotAttackTarget on the enemy permanent — parity with Python's
/// action_mask.py:348-351.
#[test]
fn mask_vortex_respects_cannot_attack_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    // Suspended, so only CannotAttackTarget can suppress the bit (an
    // unsuspended Digimon is no <Vortex> target at all: §11-2-7-1).
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        defender,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 1, Expiry::EndOfTurn, opp),
    );
    // §16-33-2 / §16-34-2: the decision is offered inside the
    // `[End of Your Turn]` pool.
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "Vortex must also honor CannotAttackTarget",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "base Vortex still does not emit security when CannotAttackTarget is present",
    );
}

// ─── §16-34 Overclock ────────────────────────────────────────────────

/// The former `EndOfTurnAction` field-effect bit for <Overclock>; never legal
/// since <Overclock> offers its condition inside the end-of-turn pool.
fn overclock_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START
        + field_index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_OVERCLOCK) as usize
}

/// §16-34-2 / §16-34-3: with a legal candidate, ending the turn activates
/// <Overclock> and offers its optional condition's pick (the candidate's
/// own-field bit plus PASS) as a pending selection; no activation bit.
#[test]
fn overclock_offers_its_condition_with_a_sacrifice_available() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red, 5000))
        .add_card(make_digimon("SAC", CardColor::Red, 1000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[overclock_bit(oc.index as usize)], 0.0, "no activation bit");
    assert_eq!(
        mask[encode_attack(0, sac.index as u16) as usize],
        1.0,
        "the sacrifice candidate is offered",
    );
    assert_eq!(mask[PASS as usize], 1.0, "the condition is optional");
}

/// §16-34-2: with no legal candidate <Overclock> does not activate (DCGO
/// `CanActivateOverclock`): nothing is offered and the turn passes.
#[test]
fn overclock_does_not_activate_without_sacrifice() {
    // Only the Overclock Digimon on the field — nothing else to sacrifice.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    assert!(r.game.pending_selection.is_none(), "nothing offered");
    assert_ne!(r.game.turn_player(), tp, "the turn passed");
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(mask[overclock_bit(oc.index as usize)], 0.0);
}

// ─── §4.6c MayAttack ──────────────────────────────────────────────────

#[test]
fn mask_may_attack_emits_attack_bits_against_suspended_digimon_and_security() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "MayAttack permits attacking security",
    );
    // Normal MayAttack uses Main-phase target legality (37152674): an
    // unsuspended enemy Digimon is not a declaration target without a
    // CanAttackUnsuspended grant (the same rule as for Vortex, §11-2-7-1;
    // <Raid> only switches after the declaration, §16-23-1) ...
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "MayAttack does not permit attacking an unsuspended enemy Digimon",
    );
    // ... while a suspended one is.
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        1.0,
        "MayAttack permits attacking a suspended enemy Digimon",
    );
}

#[test]
fn mask_may_attack_respects_cannot_attack_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.modifiers.add(
        defender,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 1, Expiry::EndOfTurn, opp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "MayAttack must honor CannotAttackTarget like Vortex does",
    );
}

// ─── §4.6c ForceAttack (EOT branch) ───────────────────────────────────

#[test]
fn mask_force_attack_emits_attack_bits_in_eot() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::ForceAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "ForceAttack permits attacking security in EOT",
    );
    // Normal ForceAttack uses Main-phase target legality (37152674): an
    // unsuspended enemy Digimon is not a declaration target without a
    // CanAttackUnsuspended grant (the same rule as for Vortex, §11-2-7-1;
    // <Raid> only switches after the declaration, §16-23-1) ...
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "ForceAttack does not permit attacking an unsuspended enemy Digimon",
    );
    // ... while a suspended one is.
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;
    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        1.0,
        "ForceAttack permits attacking a suspended enemy Digimon",
    );
    // PASS is still emitted — execution-side enforcement of the "mandatory"
    // part is out of scope for mask-level parity (matches Python).
    assert_eq!(mask[PASS as usize], 1.0);
}
