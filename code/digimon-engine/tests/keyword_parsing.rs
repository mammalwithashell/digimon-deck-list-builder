//! Phase 3 native-keyword parser tests.

use digimon_engine::card_data::parse_printed_keywords;
use digimon_engine::enums::Keyword;

#[test]
fn parses_digixros_scoped_alias_without_global_name_alias() {
    let cards = CardData::load_from_str(
        r#"{
            "BT21-021": {
                "card_id": "BT21-021",
                "card_name_eng": "OmniShoutmon",
                "card_kind": 0,
                "play_cost": 8,
                "dp": 8000,
                "level": 5,
                "card_colors": [0, 2],
                "effect_description_eng": "This card is also treated as [Shoutmon] for DigiXros.",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("BT21-021").expect("fixture card exists");

    assert_eq!(data.digixros_aliases, vec!["Shoutmon"]);
    assert_eq!(data.card_name, "OmniShoutmon");
}

#[test]
fn parses_digixros_alias_with_article_from_printed_text() {
    let cards = CardData::load_from_str(
        r#"{
            "BT19-012": {
                "card_id": "BT19-012",
                "card_name_eng": "OmniShoutmon",
                "card_kind": 0,
                "play_cost": 7,
                "dp": 7000,
                "level": 5,
                "card_colors": [0, 2],
                "effect_description_eng": "This card is also treated as [Shoutmon] for a DigiXros.",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("BT19-012").expect("fixture card exists");

    assert_eq!(data.digixros_aliases, vec!["Shoutmon"]);
}

#[test]
fn parses_multiple_digixros_aliases_from_single_printed_phrase() {
    let cards = CardData::load_from_str(
        r#"{
            "BT21-027": {
                "card_id": "BT21-027",
                "card_name_eng": "Shoutmon DX",
                "card_kind": 0,
                "play_cost": 12,
                "dp": 12000,
                "level": 6,
                "card_colors": [0, 2],
                "effect_description_eng": "This card is also treated as [Shoutmon] or [ZeigGreymon] for DigiXros.",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("BT21-027").expect("fixture card exists");

    assert_eq!(data.digixros_aliases, vec!["Shoutmon", "ZeigGreymon"]);
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(data, "ZeigGreymon"),
        "DigiXros aliases must not leak into generic name matching"
    );
}

#[test]
fn parses_prefix_scoped_digixros_alias_from_printed_text() {
    let cards = CardData::load_from_str(
        r#"{
            "BT11-015": {
                "card_id": "BT11-015",
                "card_name_eng": "Star Sword Carrier",
                "card_kind": 0,
                "play_cost": 6,
                "dp": 5000,
                "level": 4,
                "card_colors": [0],
                "effect_description_eng": "When you would DigiXros, this card/Digimon is also treated as [Shoutmon].",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("BT11-015").expect("fixture card exists");

    assert_eq!(data.digixros_aliases, vec!["Shoutmon"]);
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(data, "Shoutmon"),
        "DigiXros aliases must not leak into generic name matching"
    );
}

#[test]
fn real_bt11_015_populates_prefix_scoped_digixros_alias() {
    let cards_json = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("code dir")
        .parent()
        .expect("repo root")
        .join("data/cards.json");
    let cards = CardData::load_from_file(&cards_json).expect("real cards.json loads");
    let data = cards.get("BT11-015").expect("BT11-015 exists");

    assert_eq!(data.digixros_aliases, vec!["Shoutmon"]);
}

#[test]
fn ignores_unscoped_generic_alias_text_for_digixros_aliases() {
    let cards = CardData::load_from_str(
        r#"{
            "ALIAS-001": {
                "card_id": "ALIAS-001",
                "card_name_eng": "Alias Carrier",
                "card_kind": 0,
                "play_cost": 3,
                "dp": 3000,
                "level": 3,
                "card_colors": [0],
                "effect_description_eng": "This card is also treated as [SomeName].",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("ALIAS-001").expect("fixture card exists");

    assert!(data.digixros_aliases.is_empty());
}

#[test]
fn ignores_generic_alias_before_separate_digixros_sentence() {
    let cards = CardData::load_from_str(
        r#"{
            "ALIAS-002": {
                "card_id": "ALIAS-002",
                "card_name_eng": "Alias Carrier",
                "card_kind": 0,
                "play_cost": 3,
                "dp": 3000,
                "level": 3,
                "card_colors": [0],
                "effect_description_eng": "This card is also treated as [Shoutmon]. When you would DigiXros, draw 1.",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("ALIAS-002").expect("fixture card exists");

    assert!(data.digixros_aliases.is_empty());
}

#[test]
fn ignores_generic_alias_before_separate_for_a_digixros_sentence() {
    let cards = CardData::load_from_str(
        r#"{
            "ALIAS-003": {
                "card_id": "ALIAS-003",
                "card_name_eng": "Alias Carrier",
                "card_kind": 0,
                "play_cost": 3,
                "dp": 3000,
                "level": 3,
                "card_colors": [0],
                "effect_description_eng": "This card is also treated as [Shoutmon]. You may place cards for a DigiXros.",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#,
    )
    .expect("fixture must parse");
    let data = cards.get("ALIAS-003").expect("fixture card exists");

    assert!(data.digixros_aliases.is_empty());
}

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
fn parses_group6_core_combat_keywords() {
    let keywords = parse_printed_keywords(
        "\u{ff1c}Collision\u{ff1e} \u{ff1c}Piercing\u{ff1e} \u{ff1c}Reboot\u{ff1e} \u{ff1c}Retaliation\u{ff1e}",
        "",
        "",
    );
    assert!(keywords.contains(&Keyword::Collision));
    assert!(keywords.contains(&Keyword::Piercing));
    assert!(keywords.contains(&Keyword::Reboot));
    assert!(keywords.contains(&Keyword::Retaliation));
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
fn parses_blast_digivolve_produces_blast_digivolve_variant() {
    // "<Blast Digivolve>" parses to Keyword::BlastDigivolve. The longest-
    // prefix match previously distinguished a (now-removed) standalone
    // "Blast" keyword; this test remains as a regression guard on the
    // printed-text → enum-variant mapping.
    use digimon_engine::enums::Keyword;
    let kw = parse_printed_keywords("\u{ff1c}Blast Digivolve\u{ff1e} (...)", "", "");
    assert_eq!(kw, vec![Keyword::BlastDigivolve]);
}

#[test]
fn parser_armor_purge_matches_correctly() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("[When Digivolving] ＜Armor Purge＞ effect text", "", "");
    assert!(kws.contains(&Keyword::ArmorPurge));
}

#[test]
fn parser_decode_before_decoy() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("＜Decode＞", "", "");
    assert!(kws.contains(&Keyword::Decode));
    assert!(!kws.contains(&Keyword::Decoy(0)));
}

#[test]
fn parser_evade_basic() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("＜Evade＞", "", "");
    assert_eq!(kws, vec![Keyword::Evade]);
}

#[test]
fn parser_fragment_paren_notation() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("＜Fragment (3)＞", "", "");
    assert_eq!(kws, vec![Keyword::Fragment(3)]);
}

#[test]
fn parser_fragment_bare_digit() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("＜Fragment 2＞", "", "");
    assert_eq!(kws, vec![Keyword::Fragment(2)]);
}

// ─── Game::has_keyword integration tests ───────────────────────────────────

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
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
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        keywords,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

#[test]
fn game_has_keyword_sees_native_printed() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text(
            "NATIVE_RUSH",
            "\u{ff1c}Rush\u{ff1e} (...)",
        ))
        .hand(0, &["NATIVE_RUSH"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };

    assert!(
        r.game_mut()
            .has_keyword(handle, digimon_engine::enums::Keyword::Rush),
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
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game_mut().modifiers.grant_keyword(
        handle,
        digimon_engine::enums::Keyword::Rush,
        Expiry::EndOfTurn,
        0,
    );

    assert!(r
        .game_mut()
        .has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

#[test]
fn game_has_keyword_false_when_neither() {
    let mut r = DebugRunner::builder()
        .add_card(digimon_with_text("NEITHER", ""))
        .hand(0, &["NEITHER"])
        .memory(5)
        .start();

    r.play(0, 0);
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(!r
        .game_mut()
        .has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

#[test]
fn game_has_keyword_bad_handle_returns_false() {
    let mut r = DebugRunner::builder().start();
    let handle = PermanentHandle {
        player: 0,
        index: 99,
    };
    assert!(!r
        .game_mut()
        .has_keyword(handle, digimon_engine::enums::Keyword::Rush));
}

// ─── Phase 3 Task 3 behavioral regression tests ────────────────────────────

#[test]
fn native_printed_rush_allows_same_turn_attack() {
    // A freshly-played Digimon with native printed Rush can attack
    // on the same turn (normally summoning-sickness blocks this).
    let mut atk = digimon_with_text(
        "R",
        "\u{ff1c}Rush\u{ff1e} (This Digimon can attack the turn it comes into play.)",
    );
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
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };
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
    let handle = PermanentHandle {
        player: 0,
        index: 0,
    };
    let _ = r.attack_player(handle, 1, true);

    assert!(
        r.battle_area_size(0) > 0,
        "Jamming should protect the losing attacker from deletion"
    );
}

#[test]
fn parser_material_save_parametric() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("\u{ff1c}Material Save 2\u{ff1e} (...)", "", "");
    assert!(kws.contains(&Keyword::MaterialSave(2)), "got {:?}", kws);
}

#[test]
fn parser_digi_burst_parametric() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("\u{ff1c}Digi-Burst 1\u{ff1e} (...)", "", "");
    assert_eq!(kws, vec![Keyword::DigiBurst(1)]);
}

#[test]
fn parser_material_save_no_alias_to_save() {
    // "<Material Save>" with no number should not alias back to Save —
    // it must either produce MaterialSave(1) or not parse at all.
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("\u{ff1c}Material Save\u{ff1e}", "", "");
    assert!(
        !kws.contains(&Keyword::Save),
        "must not alias MaterialSave -> Save"
    );
}

// ─── Decoy color-filter parsing (Track G close) ─────────────────────────────

/// Bare `<Decoy>` parses to `Decoy(0)` (no color filter — matches all
/// ally Digimon, identical to the prior un-parameterised behavior).
#[test]
fn parser_decoy_bare_form_is_zero_mask() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords("\u{ff1c}Decoy\u{ff1e}", "", "");
    assert_eq!(kws, vec![Keyword::Decoy(0)]);
}

/// `<Decoy (Black)>` parses to a single-bit mask. Black = CardColor::Black =
/// index 5 → bit 5 = 0b00100000 = 0x20.
#[test]
fn parser_decoy_single_color_filter() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords(
        "\u{ff1c}Decoy (Black)\u{ff1e} (When your other Black Digimon ...)",
        "",
        "",
    );
    assert_eq!(kws, vec![Keyword::Decoy(0b0010_0000)]);
}

/// `<Decoy (Red/Black)>` parses to a two-bit mask. Red=0 (bit 0), Black=5
/// (bit 5) → 0b00100001 = 0x21.
#[test]
fn parser_decoy_multi_color_filter() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords(
        "\u{ff1c}Decoy (Red/Black)\u{ff1e}",
        "",
        "",
    );
    assert_eq!(kws, vec![Keyword::Decoy(0b0010_0001)]);
}

/// `<Decoy (Black/White)>` — Black=5 (bit 5), White=4 (bit 4) → 0b00110000.
#[test]
fn parser_decoy_black_white_filter() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords(
        "\u{ff1c}Decoy (Black/White)\u{ff1e}",
        "",
        "",
    );
    assert_eq!(kws, vec![Keyword::Decoy(0b0011_0000)]);
}

/// Trait-form filter `<Decoy ([Bagra Army] trait)>` parses to `Decoy(0)` —
/// the parser drops the trait filter; cards using trait filters require a
/// hand-rolled `CardEffect` override (documented gap in
/// `RUST_ENGINE_GAPS.md`).
#[test]
fn parser_decoy_trait_filter_drops_to_zero_mask() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords(
        "\u{ff1c}Decoy ([Bagra Army] trait)\u{ff1e}",
        "",
        "",
    );
    assert_eq!(kws, vec![Keyword::Decoy(0)]);
}

/// `<Decoy ([Deva] or [Four Sovereigns] trait)>` — multi-trait OR form,
/// also drops to `Decoy(0)` per the parser's trait-filter handling.
#[test]
fn parser_decoy_multi_trait_filter_drops_to_zero_mask() {
    use digimon_engine::enums::Keyword;
    let kws = parse_printed_keywords(
        "\u{ff1c}Decoy ([Deva] or [Four Sovereigns] trait)\u{ff1e}",
        "",
        "",
    );
    assert_eq!(kws, vec![Keyword::Decoy(0)]);
}
