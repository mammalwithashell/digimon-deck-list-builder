//! `to_ui_json` permanent-inspector fields — keywords + innate/gained
//! breakdown, security-attack modifier, DP breakdown, per-source effect text,
//! and the active-modifier list. Covers the `permanent-runtime-state-
//! serialization` capability (change `add-permanent-stack-inspector`).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::serialization::to_ui_json;

/// Build a runner with a two-card stack on player 1: bottom source "SRC"
/// (with inherited text) under top "TOP" (printed Blocker, 4000 DP, main
/// effect text). Returns the runner positioned for serialization.
fn runner_with_stack() -> DebugRunner {
    let mut top = make_test_card("TOP", "TopMon");
    top.dp = Some(4000);
    top.level = Some(4);
    // Keywords are parsed from printed text when text is present (see
    // `face_keywords`), so encode the printed `<Blocker>` token in the effect
    // text rather than the raw `keywords` field.
    top.effect_text = "＜Blocker＞\nTop main effect.".to_string();

    let mut src = make_test_card("SRC", "SrcMon");
    src.inherited_text = "Inherited: gain 1 memory.".to_string();

    let mut r = DebugRunner::builder().add_card(top).add_card(src).start();
    // bottom-to-top: SRC under TOP
    r.place_stack(0, &["SRC", "TOP"]);
    r
}

fn perm0(value: &serde_json::Value) -> serde_json::Value {
    value["player1"]["battleArea"][0].clone()
}

#[test]
fn innate_printed_keyword_breakdown() {
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    let kws = p["keywords"].as_array().unwrap();
    assert!(
        kws.iter().any(|k| k == "blocker"),
        "expected blocker in keywords, got {:?}",
        kws
    );
    let innate = p["keywordBreakdown"]["innate"].as_array().unwrap();
    let gained = p["keywordBreakdown"]["gained"].as_array().unwrap();
    assert!(
        innate.iter().any(|k| k == "blocker"),
        "blocker should be innate"
    );
    assert!(
        !gained.iter().any(|k| k == "blocker"),
        "blocker should not be gained"
    );
}

#[test]
fn modifier_granted_keyword_is_gained() {
    let mut r = runner_with_stack();
    let handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    // Grant Rush via the keyword store (how engine grants actually register).
    r.game
        .modifiers
        .grant_keyword(handle, Keyword::Rush, Expiry::Permanent, 0);

    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    let gained = p["keywordBreakdown"]["gained"].as_array().unwrap();
    let innate = p["keywordBreakdown"]["innate"].as_array().unwrap();
    assert!(
        gained.iter().any(|k| k == "rush"),
        "rush should be gained, got {:?}",
        gained
    );
    assert!(
        !innate.iter().any(|k| k == "rush"),
        "rush should not be innate"
    );
}

#[test]
fn dp_breakdown_reflects_modifier() {
    let mut r = runner_with_stack();
    let handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        handle,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 0),
    );

    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    assert_eq!(p["dpBreakdown"]["base"], serde_json::json!(4000));
    assert_eq!(p["dpBreakdown"]["total"], serde_json::json!(7000));
    assert_eq!(p["dpBreakdown"]["temporary"], serde_json::json!(3000));
}

#[test]
fn dp_breakdown_neutral_without_modifier() {
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    assert_eq!(p["dpBreakdown"]["base"], p["dpBreakdown"]["total"]);
    assert_eq!(p["dpBreakdown"]["temporary"], serde_json::json!(0));
}

#[test]
fn source_count_excludes_the_top_card() {
    // "Digivolution cards" are the cards UNDER the top card — the top card
    // itself is not one (the UI badge showed the total stack size, so a
    // Digimon with 1 source rendered as "x2").
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    // 2-card stack (SRC under TOP) → exactly 1 digivolution card.
    assert_eq!(p["sourceCount"], serde_json::json!(1));
    // The full stack (top + sources) is still serialized in `sources`.
    assert_eq!(p["sources"].as_array().unwrap().len(), 2);
}

#[test]
fn security_attack_modifier_default_zero() {
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    assert_eq!(p["securityAttackModifier"], serde_json::json!(0));
}

#[test]
fn source_and_inherited_effect_text_populated() {
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);

    // Top card's main effect text on the permanent and on its top source.
    assert_eq!(
        p["mainEffectText"],
        serde_json::json!("＜Blocker＞\nTop main effect.")
    );

    let sources = p["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2, "expected a two-card stack");
    // The bottom source (non-top) carries inherited text.
    let bottom = sources.iter().find(|s| s["cardId"] == "SRC").unwrap();
    assert_eq!(
        bottom["inheritedEffectText"],
        serde_json::json!("Inherited: gain 1 memory.")
    );

    // Permanent-level inheritedEffects lists the non-top source.
    let inh = p["inheritedEffects"].as_array().unwrap();
    assert_eq!(
        inh.len(),
        1,
        "exactly one inherited effect from the buried source"
    );
    assert_eq!(inh[0]["cardId"], serde_json::json!("SRC"));
    assert_eq!(
        inh[0]["text"],
        serde_json::json!("Inherited: gain 1 memory.")
    );
}

/// A link card's link effect is listed for the Digimon it is linked to
/// (general_rule.pdf 4-2-6), never as an inherited effect: a link card among
/// the digivolution cards confers nothing (4-2-4, 4-7-2).
#[test]
fn link_effect_text_listed_for_link_cards_only() {
    let mut linked = make_test_card("LNK", "LinkMon");
    linked.link_text = "[When Linking] Gain 1 memory.".to_string();
    let mut buried = make_test_card("LSRC", "BuriedLinkMon");
    buried.link_text = "[Your Turn] This Digimon gets +1000 DP.".to_string();
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TOP", "TopMon"))
        .add_card(linked)
        .add_card(buried)
        .start();
    let handle = r.place_stack(0, &["LSRC", "TOP"]);
    r.push_linked_owned(handle, "LNK", 0);

    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    let links = p["linkEffects"].as_array().expect("linkEffects array");
    assert_eq!(links.len(), 1, "one link effect, from the link card: {links:?}");
    assert_eq!(links[0]["linkIndex"], serde_json::json!(0));
    assert_eq!(links[0]["cardId"], serde_json::json!("LNK"));
    assert_eq!(links[0]["cardName"], serde_json::json!("LinkMon"));
    assert_eq!(links[0]["text"], serde_json::json!("[When Linking] Gain 1 memory."));
    assert_eq!(
        p["inheritedEffects"].as_array().unwrap().len(),
        0,
        "a link card among the digivolution cards confers no inherited effect"
    );
}

#[test]
fn single_card_permanent_has_no_inherited_effects() {
    let mut top = make_test_card("SOLO", "Solo");
    top.effect_text = "Solo effect.".to_string();
    let mut r = DebugRunner::builder().add_card(top).start();
    r.place_on_field(0, "SOLO", Some(0));

    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    assert_eq!(p["inheritedEffects"].as_array().unwrap().len(), 0);
    assert_eq!(p["modifiers"].as_array().unwrap().len(), 0);
}

#[test]
fn active_modifier_list_emits_structured_entries() {
    let mut r = runner_with_stack();
    let handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    r.game.modifiers.add(
        handle,
        ModifierEntry::simple(ModifierType::CannotBeDestroyed, 0, Expiry::Permanent, 0),
    );
    r.game.modifiers.add(
        handle,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::EndOfTurn, 0),
    );
    r.game.modifiers.add(
        handle,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );

    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    let mods = p["modifiers"].as_array().unwrap();

    let immunity = mods
        .iter()
        .find(|m| m["type"] == "CannotBeDestroyed")
        .unwrap();
    assert_eq!(immunity["expiry"], serde_json::json!("Permanent"));

    let dp = mods.iter().find(|m| m["type"] == "ChangeDp").unwrap();
    assert_eq!(dp["value"], serde_json::json!(3000));
    assert_eq!(dp["expiry"], serde_json::json!("EndOfTurn"));

    assert!(
        mods.iter().any(|m| m["type"] == "CannotSuspend"),
        "expected CannotSuspend in modifiers, got {:?}",
        mods
    );
}

#[test]
fn permanent_has_documented_keys() {
    let r = runner_with_stack();
    let v = to_ui_json(&r.game);
    let p = perm0(&v);
    let obj = p.as_object().unwrap();
    for key in [
        "topCardId",
        "topCardName",
        "dp",
        "level",
        "isSuspended",
        "sourceCount",
        "keywords",
        "keywordBreakdown",
        "securityAttackModifier",
        "linkedCardIds",
        "sources",
        "mainEffectText",
        "inheritedEffects",
        "linkEffects",
        "modifiers",
        "dpBreakdown",
        "turnPlayed",
        "colors",
    ] {
        assert!(obj.contains_key(key), "permanent missing key {:?}", key);
    }
    // modifiers is always an array, even when empty.
    assert!(p["modifiers"].is_array());
}
