//! Phase 9 Task 6 — `<Piercing>` post-battle security check + PostBattle state.
//!
//! After a Digimon-vs-Digimon battle where the attacker survives and the
//! defender is wiped, a `<Piercing>` attacker continues into a security
//! check against the defending player. The check reuses the standard
//! security-resolution pipeline: honors Jamming on the attacker (no
//! security-skill-driven deletion), respects Security Attack +N modifiers
//! for consumption count, and fires `WhenWouldLoseSecurity` replacements
//! on the defender.
//!
//! Spec: docs/superpowers/specs/2026-04-21-combat-interrupt-completion-design.md
//! §4.3. Plan: Task 6 (lines 533-594).

use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};

fn big_digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 6,
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
        also_treated_as: Vec::new(),
    }
}

fn filler_option(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 2,
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
        also_treated_as: Vec::new(),
    }
}

/// Test 1: Attacker with <Piercing> wins the Digimon battle and triggers a
/// follow-up security check against the defending player.
#[test]
fn piercing_attacker_triggers_security_check_after_wiping_defender() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("DEF", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Grant Piercing to the attacker.
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);

    let sec_before = r.security_count(1);
    let trash_before = r.trash_size(1);

    let _ = r.attack_digimon(atk, def, false);

    // Defender must be wiped (DP 8000 vs 3000).
    assert_eq!(
        r.battle_area_size(1),
        0,
        "defender (DP 3000) must be wiped by attacker (DP 8000)"
    );

    // Piercing should have continued into a security check:
    // exactly one security card consumed.
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "Piercing follow-up must consume exactly one security card"
    );
    // P1's trash grows by +1 for the wiped defender (Digimon battle loss)
    // AND +1 for the consumed security card.
    assert_eq!(
        r.trash_size(1),
        trash_before + 2,
        "defender (wiped in battle) + consumed security card both land in P1 trash"
    );
}

/// Test 2: <Piercing> + <Jamming> — attacker wins a losing security battle
/// (or rather, Jamming prevents security-skill deletion when the revealed
/// card is a Digimon with higher DP). We use a non-Digimon security card
/// here so the interaction is straightforward: the security card is
/// consumed; Jamming is irrelevant for Option security, but this test
/// verifies Piercing honors the standard resolver, which would honor
/// Jamming if it mattered.
///
/// The load-bearing assertion is: no security-skill deletion of the
/// attacker even if the security card had been a Digimon that outclassed
/// it — we stage a Digimon security with higher DP to exercise the path.
#[test]
fn piercing_honors_jamming_on_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 4000))
        .add_card(big_digimon("DEF", 2000))
        .add_card(big_digimon("SEC", 9000)) // strong security Digimon
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Grant Piercing + Jamming to the attacker.
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Jamming, Expiry::Permanent, 0);

    let _ = r.attack_digimon(atk, def, false);

    // Defender must be wiped (4000 vs 2000).
    assert_eq!(r.battle_area_size(1), 0, "defender must be wiped");

    // Piercing fired a security check; the security card (9000 DP) would
    // normally delete the weaker attacker (4000), but Jamming protects.
    assert!(
        r.battle_area_size(0) > 0,
        "Jamming must protect attacker from security-skill deletion during \
         Piercing follow-up"
    );
    // Security card was still consumed.
    assert_eq!(
        r.security_count(1),
        0,
        "security card must still be consumed during the Piercing check"
    );
    // P1's trash: wiped defender (+1) + consumed security card (+1) = 2.
    assert_eq!(
        r.trash_size(1),
        2,
        "defender + consumed security card both land in P1 trash"
    );
}

/// Test 3: Piercing + Security Attack +1 — the Piercing-triggered check
/// consumes (1 + 1) = 2 security cards.
#[test]
fn piercing_stacks_with_security_attack_modifier() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("DEF", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game.modifiers.add(
        atk,
        digimon_engine::modifiers::ModifierEntry::simple(
            ModifierType::SecurityAttackChange,
            1,
            Expiry::Permanent,
            0,
        ),
    );

    let _ = r.attack_digimon(atk, def, false);

    // Defender wiped, then Piercing fires a 2-card security check.
    assert_eq!(
        r.battle_area_size(1),
        0,
        "defender wiped by DP 8000 vs 3000"
    );
    assert_eq!(
        r.security_count(1),
        1,
        "2 security cards consumed (1 base + 1 from Security Attack)"
    );
    // P1 trash: wiped defender (+1) + 2 consumed security cards (+2) = 3.
    assert_eq!(r.trash_size(1), 3);
}

/// Test 4: Mutual-KO — attacker is also wiped by the battle. Piercing
/// does NOT fire (the attacker no longer exists to continue into
/// security).
#[test]
fn piercing_does_nothing_when_attacker_wiped() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 5000))
        .add_card(big_digimon("DEF", 5000)) // equal DP → mutual KO
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);

    let sec_before = r.security_count(1);

    let _ = r.attack_digimon(atk, def, false);

    // Mutual destruction: both wiped.
    assert_eq!(r.battle_area_size(0), 0, "attacker wiped in mutual KO");
    assert_eq!(r.battle_area_size(1), 0, "defender wiped in mutual KO");

    // Piercing must NOT fire — no security card consumed. (DEF legitimately
    // hits P1's trash as part of the mutual KO, so we only assert the
    // security-stack delta here.)
    assert_eq!(
        r.security_count(1),
        sec_before,
        "Piercing must not fire when attacker is wiped"
    );
}

// ─── Blocked-player-attack path (glossary: "This effect also works if an
//     attack is blocked"; rules §16-6-1/-2) ─────────────────────────────
//
// The canonical real-game Piercing scenario: attack the PLAYER, the
// opponent blocks, the attacker deletes the blocker in battle and
// survives → the security check still happens (mandatory, §16-6-3).

/// Same CardData shape as `big_digimon` but with `<Piercing>` printed on
/// the card face (fullwidth brackets, as ingested card text uses).
fn piercing_digimon(id: &str, dp: i32) -> CardData {
    let mut c = big_digimon(id, dp);
    c.effect_text = "\u{ff1c}Piercing\u{ff1e} (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.)".to_string();
    c.keywords = vec![Keyword::Piercing];
    c
}

/// Source card whose inherited text grants `<Piercing>`.
fn inherited_piercing_source(id: &str) -> CardData {
    let mut c = big_digimon(id, 3000);
    c.level = Some(4);
    c.inherited_text = "\u{ff1c}Piercing\u{ff1e}".to_string();
    c
}

/// Drive: P0 attacks P1's player, P1 declares `blk` as blocker, battle
/// resolves. Returns nothing; callers assert on state.
fn attack_player_and_block(r: &mut DebugRunner, atk: digimon_engine::PermanentHandle, blk: digimon_engine::PermanentHandle) {
    let result = r.attack_player(atk, 1, false);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::InProgress,
        "block window must open (blocker candidate exists)"
    );
    let sel = r
        .pending_selection()
        .expect("BlockTiming selection installed");
    assert_eq!(sel.selecting_player, 1, "defender declares the blocker");
    r.game
        .resolve_selection(1, encode_attack(0, blk.index as u16))
        .expect("declaring the blocker must be legal");
}

/// Canonical: modifier-granted <Piercing>, player attack blocked, blocker
/// wiped, attacker survives → exactly one mandatory security check.
#[test]
fn piercing_fires_when_player_attack_is_blocked_modifier_grant() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("BLK", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    let sec_before = r.security_count(1);
    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(r.battle_area_size(1), 0, "blocker (3000) wiped by 8000");
    assert!(
        r.battle_area_size(0) > 0,
        "attacker survives the battle with the blocker"
    );
    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "Piercing must perform the security check after a blocked player \
         attack (glossary: 'This effect also works if an attack is blocked')"
    );
    // No prompt: the Piercing check is mandatory (§16-6-3).
    assert!(
        r.pending_selection().is_none(),
        "no player prompt — the Piercing security check is mandatory"
    );
    assert!(r.game.pending_attack.is_none(), "attack fully cleaned up");
    // Blocker (+1) and consumed security card (+1) both in P1 trash.
    assert_eq!(r.trash_size(1), 2);
}

/// Printed keyword on the card face (parsed from effect text) must be
/// honored on the blocked path — not just registry-granted keywords.
#[test]
fn piercing_fires_when_player_attack_is_blocked_printed_keyword() {
    let mut r = DebugRunner::builder()
        .add_card(piercing_digimon("PRC", 8000))
        .add_card(big_digimon("BLK", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "PRC", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(r.battle_area_size(1), 0, "blocker wiped");
    assert_eq!(
        r.security_count(1),
        1,
        "printed <Piercing> must trigger the security check when blocked"
    );
}

/// Inherited `<Piercing>` from a digivolution source must be honored on
/// the blocked path.
#[test]
fn piercing_fires_when_player_attack_is_blocked_inherited_keyword() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("TOP", 8000))
        .add_card(inherited_piercing_source("SRC"))
        .add_card(big_digimon("BLK", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    // SRC under TOP: inherited <Piercing> active.
    let atk = r.place_stack(0, &["SRC", "TOP"]);
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(r.battle_area_size(1), 0, "blocker wiped");
    assert_eq!(
        r.security_count(1),
        1,
        "inherited <Piercing> must trigger the security check when blocked"
    );
}

/// Negative: the blocker WINS the battle (attacker deleted) → no check.
#[test]
fn piercing_does_not_fire_when_blocker_wins_battle() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 3000))
        .add_card(big_digimon("BLK", 9000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(r.battle_area_size(0), 0, "attacker (3000) deleted by 9000");
    assert_eq!(
        r.security_count(1),
        2,
        "no Piercing check when the attacker loses the battle"
    );
}

/// Negative: blocker survives (attacker can't delete it — tie goes to
/// mutual KO, so use CannotBeDestroyedByBattle on the blocker) → no check.
#[test]
fn piercing_does_not_fire_when_blocker_survives() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("BLK", 3000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    // Blocker cannot be deleted in battle → it survives the losing battle.
    r.game.modifiers.add(
        blk,
        digimon_engine::modifiers::ModifierEntry::simple(
            ModifierType::CannotBeDestroyedByBattle,
            1,
            Expiry::Permanent,
            1,
        ),
    );

    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "blocker survives via CannotBeDestroyedByBattle"
    );
    assert_eq!(
        r.security_count(1),
        2,
        "no Piercing check when the battling Digimon was not deleted"
    );
}

/// Slot-shift regression (user report "Piercing not working"): the wiped
/// defender was NOT the highest-indexed permanent, so a bystander shifts
/// down into its battle-area slot after deletion. The piercing gate must
/// not mistake the shifted bystander for a surviving defender.
/// Direct-attack variant.
#[test]
fn piercing_fires_when_wiped_defender_has_higher_slot_bystander() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("DEF", 3000))
        .add_card(big_digimon("BYSTANDER", 12000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0)); // index 0
    let _bystander = r.place_on_field(1, "BYSTANDER", Some(0)); // index 1
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);

    let _ = r.attack_digimon(atk, def, false);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "DEF wiped; only the bystander remains"
    );
    assert_eq!(
        r.security_count(1),
        1,
        "Piercing must fire even though a bystander shifted into the \
         wiped defender's battle-area slot"
    );
}

/// Slot-shift regression, blocked-player-attack variant: blocker at slot 0
/// dies, bystander at slot 1 shifts down into slot 0.
#[test]
fn piercing_fires_when_blocked_with_higher_slot_bystander() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("BLK", 3000))
        .add_card(big_digimon("BYSTANDER", 12000))
        .add_card(filler_option("OPT"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0)); // index 0
    let _bystander = r.place_on_field(1, "BYSTANDER", Some(0)); // index 1
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(
        r.battle_area_size(1),
        1,
        "blocker wiped; only the bystander remains"
    );
    assert_eq!(
        r.security_count(1),
        1,
        "Piercing must fire after a blocked player attack even when a \
         bystander shifted into the wiped blocker's slot"
    );
}

/// §16-6-6: with 0 security cards the Piercing check can't be performed —
/// and, critically, it must NOT win the game (unlike an unblocked player
/// attack under §11-5-1-2). DCGO parity: `DetermineAttackOutcome` only
/// ends the game on the DefendingPermanent == null arm; the post-battle
/// security check is gated on `SecurityCards.Count >= 1`.
#[test]
fn piercing_with_zero_security_does_not_win_game() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", 8000))
        .add_card(big_digimon("BLK", 3000))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    assert_eq!(r.security_count(1), 0, "P1 staged with empty security");
    attack_player_and_block(&mut r, atk, blk);

    assert_eq!(r.battle_area_size(1), 0, "blocker wiped");
    assert!(
        !r.game_over(),
        "Piercing with 0 opposing security must not end the game \
         (§16-6-6: the check can't be performed at all)"
    );
    assert!(r.game.pending_attack.is_none(), "attack cleaned up normally");
}
