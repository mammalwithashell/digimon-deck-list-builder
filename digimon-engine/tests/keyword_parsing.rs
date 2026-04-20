//! Phase 3 native-keyword parser tests.

use digimon_engine::card_data::parse_printed_keywords;
use digimon_engine::enums::Keyword;

#[test]
fn parses_rush() {
    let kw = parse_printed_keywords(
        "\u{ff1c}Rush\u{ff1e} (This Digimon can attack the turn it comes into play.)",
        "",
        "",
    );
    assert_eq!(kw, vec![Keyword::Rush]);
}

#[test]
fn parses_jamming_in_inherited() {
    let kw = parse_printed_keywords("", "\u{ff1c}Jamming\u{ff1e} (...)", "");
    assert_eq!(kw, vec![Keyword::Jamming]);
}

#[test]
fn parses_multiple_keywords_in_same_field() {
    let kw = parse_printed_keywords(
        "\u{ff1c}Raid\u{ff1e} (When this Digimon attacks, you may...)\r\n\u{ff1c}Piercing\u{ff1e} (...)",
        "",
        "",
    );
    assert!(kw.contains(&Keyword::Raid));
    assert!(kw.contains(&Keyword::Piercing));
    assert_eq!(kw.len(), 2);
}

#[test]
fn dedupes_same_keyword_in_multiple_fields() {
    let kw = parse_printed_keywords(
        "\u{ff1c}Rush\u{ff1e} (...)",
        "\u{ff1c}Rush\u{ff1e} (...)",
        "",
    );
    assert_eq!(kw, vec![Keyword::Rush]);
}

#[test]
fn parses_security_attack_plus() {
    let kw = parse_printed_keywords("\u{ff1c}Security A. +1\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::SecurityAttackPlus(1)]);
}

#[test]
fn parses_security_attack_minus() {
    let kw = parse_printed_keywords("\u{ff1c}Security A. -2\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::SecurityAttackMinus(2)]);
}

#[test]
fn parses_de_digivolve_with_arg() {
    let kw = parse_printed_keywords("\u{ff1c}De-Digivolve 2\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::DeDigivolve(2)]);
}

#[test]
fn parses_draw_with_arg() {
    let kw = parse_printed_keywords("\u{ff1c}Draw 2\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::DrawX(2)]);
}

#[test]
fn ignores_unrecognized_keywords() {
    let kw = parse_printed_keywords("\u{ff1c}MadeUpKeyword\u{ff1e} (...)", "", "");
    assert!(kw.is_empty());
}

#[test]
fn handles_empty_input() {
    assert!(parse_printed_keywords("", "", "").is_empty());
}

#[test]
fn parses_blocker_and_security_attack_together() {
    let kw = parse_printed_keywords(
        "\u{ff1c}Blocker\u{ff1e} (...)\r\n\u{ff1c}Security A. +1\u{ff1e} (...)",
        "",
        "",
    );
    assert!(kw.contains(&Keyword::Blocker));
    assert!(kw.contains(&Keyword::SecurityAttackPlus(1)));
}

#[test]
fn parses_blast_digivolve_not_confused_with_blast() {
    // "Blast Digivolve" is longer than any standalone "Blast" keyword; verify
    // the longest-prefix match works.
    let kw = parse_printed_keywords("\u{ff1c}Blast Digivolve\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::Blast]);
}

// ─── Game::has_keyword integration tests ───────────────────────────────────

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, Expiry};
use digimon_engine::permanent::PermanentHandle;

fn digimon_with_text(card_id: &str, effect_text: &str) -> CardData {
    let keywords = digimon_engine::card_data::parse_printed_keywords(effect_text, "", "");
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: effect_text.to_string(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        keywords,
    }
}

#[test]
fn game_has_keyword_sees_native_printed() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NATIVE_RUSH", "\u{ff1c}Rush\u{ff1e} (...)"))
        .hand(0, &["NATIVE_RUSH"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };

    assert!(
        r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush),
        "Game::has_keyword should see native printed Rush"
    );
}

#[test]
fn game_has_keyword_sees_modifier_granted() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NO_NATIVE", ""))
        .hand(0, &["NO_NATIVE"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    r.game_mut().modifiers.grant_keyword(
        handle,
        digimon_engine::enums::Keyword::Rush,
        Expiry::EndOfTurn,
        0,
    );

    assert!(r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

#[test]
fn game_has_keyword_false_when_neither() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NEITHER", ""))
        .hand(0, &["NEITHER"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    assert!(!r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

#[test]
fn game_has_keyword_bad_handle_returns_false() {
    let mut r = DebugRunner::builder().start();
    let handle = PermanentHandle { player: 0, index: 99 };
    assert!(!r.game_mut().has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

// ─── Phase 3 Task 3 behavioral regression tests ────────────────────────────

#[test]
fn native_printed_rush_allows_same_turn_attack() {
    // A freshly-played Digimon with native printed Rush can attack
    // on the same turn (normally summoning-sickness blocks this).
    let mut atk = digimon_with_text("R", "\u{ff1c}Rush\u{ff1e} (This Digimon can attack the turn it comes into play.)");
    atk.level = Some(5);
    atk.dp = Some(8000);

    let filler = digimon_with_text("F", "");
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(filler.clone())
        .hand(0, &["R"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    assert!(
        r.game_mut().can_attack(handle, false),
        "native printed Rush should allow fresh-turn attack"
    );
}

#[test]
fn native_printed_jamming_survives_losing_security_battle() {
    // Attacker has Jamming printed natively; loses DP comparison
    // against a security Digimon; Jamming keeps it alive.
    let mut atk = digimon_with_text("J", "\u{ff1c}Jamming\u{ff1e} (...)");
    atk.level = Some(5);
    atk.dp = Some(2000); // weak

    let mut sec = digimon_with_text("SEC", "");
    sec.level = Some(5);
    sec.dp = Some(9000); // strong security

    let filler = digimon_with_text("F", "");
    let mut r = DebugRunner::builder()
        .add_card(atk)
        .add_card(sec)
        .add_card(filler)
        .hand(0, &["J"])
        .deck(0, &["F"; 10])
        .deck(1, &["F"; 10])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle { player: 0, index: 0 };
    let _ = r.attack_player(handle, 1, true);

    assert!(
        r.battle_area_size(0) > 0,
        "Jamming should protect the losing attacker from deletion"
    );
}
