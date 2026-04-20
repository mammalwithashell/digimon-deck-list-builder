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
