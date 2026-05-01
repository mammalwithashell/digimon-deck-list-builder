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
