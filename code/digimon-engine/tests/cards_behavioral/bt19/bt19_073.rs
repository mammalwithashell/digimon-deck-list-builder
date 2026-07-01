//! BT19-073 LordKnightmon (X Antibody) — Digimon, Lv.6, Purple/White, Cost 12, DP 12000.
//! Traits: Holy Warrior / X Antibody / Royal Knight. Attribute: Virus.
//!
//! # Card text (card image — verbatim)
//!
//! ```text
//! Digivolve: [LordKnightmon]: Cost 1
//! <Collision> <Piercing>
//! [When Digivolving] For each of your Digimon, <De-Digivolve 1> 1 of your
//!   opponent's Digimon. Then, until the end of their turn, 1 of their
//!   Digimon can't digivolve.
//! [All Turns] While [LordKnightmon] or [X Antibody] is in this Digimon's
//!   digivolution cards, all of your Digimon with [Knightmon] in their texts
//!   gain <Alliance> and get +3000 DP.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT19/Purple/BT19_073.cs — ONE mandatory
//! opponent-Digimon select, then `for(i < ownDigimonCount)
//! IDegeneration(permanent, 1)` (N separate <De-Digivolve 1> instances
//! against the SAME permanent), then a second mandatory select installing
//! "can't digivolve" until the end of the opponent's turn.
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt19/BT19-073.yaml

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT19-073";

fn lvl_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-073 in embedded DSL pack")
        .dsl_card("BT19-072")
        .expect("BT19-072 LordKnightmon loads")
        .add_card(lvl_digimon("ALLY-A", "AllyA", 4, 4000))
        .add_card(lvl_digimon("ALLY-B", "AllyB", 4, 4000))
        .add_card(lvl_digimon("OPP-L3", "OppThree", 3, 3000))
        .add_card(lvl_digimon("OPP-L4", "OppFour", 4, 5000))
        .add_card(lvl_digimon("OPP-L5", "OppFive", 5, 7000))
        .add_card(lvl_digimon("OPP-L6", "OppSix", 6, 11000))
        // A vanilla [Knightmon]-NAMED ally (text has no "Knightmon" mention)
        // and a Knightmon-IN-TEXT ally for the aura's two match arms.
        .add_card(lvl_digimon("KNIGHT-NAME", "Knightmon", 4, 6000))
        .add_card({
            let mut c = lvl_digimon("KNIGHT-TEXT", "PlainAlly", 4, 6000);
            c.effect_text = "[On Play] Reveal 1 [Knightmon] from your deck.".to_string();
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start()
}

fn fire_when_digivolving(r: &mut DebugRunner, handle: PermanentHandle) {
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_073_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    assert!(card.traits.iter().any(|t| t == "X Antibody"));
    assert!(card.traits.iter().any(|t| t == "Royal Knight"));
}

/// Alt-path: [Digivolve] [LordKnightmon]: Cost 1.
#[test]
fn bt19_073_has_lordknightmon_alt_path_cost1() {
    let card = compiled(CARD_ID);
    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(1))
        }),
        "must have the [LordKnightmon] cost-1 digivolve alt-path"
    );
}

/// <Collision> and <Piercing> are declared as keyword grants.
#[test]
fn bt19_073_grants_collision_and_piercing() {
    let card = compiled(CARD_ID);
    for kw in ["Collision", "Piercing"] {
        assert!(
            card.effects.iter().any(|c| matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                    if keyword == kw
            )),
            "BT19-073 must declare <{kw}> via grant_keyword; clauses: {:?}",
            card.effects
        );
    }
}

/// Exactly one [When Digivolving] clause and one +3000 aura.
#[test]
fn bt19_073_clause_shape() {
    let card = compiled(CARD_ID);
    let wd = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving)
            )
        })
        .count();
    assert_eq!(wd, 1, "exactly one [When Digivolving] clause");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dp_modifier: Some(3000),
                ..
            })
        )),
        "must carry the +3000 [All Turns] aura"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] N × <De-Digivolve 1> + can't-digivolve
// ════════════════════════════════════════════════════════════════════════════

/// With 3 of your Digimon, the selected opponent stack is de-digivolved 3
/// times (one card at a time); then 1 opponent Digimon can't digivolve until
/// the end of their turn.
#[test]
fn bt19_073_when_digivolving_de_digivolves_once_per_own_digimon() {
    let mut r = base_runner();
    // Your Digimon: LordKnightmon X (over BT19-072) + 2 allies = 3.
    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let _a = r.place_on_field(0, "ALLY-A", Some(0));
    let _b = r.place_on_field(0, "ALLY-B", Some(0));
    // Opponent: a 4-deep stack Lv3→Lv6.
    let opp = r.place_stack(1, &["OPP-L3", "OPP-L4", "OPP-L5", "OPP-L6"]);
    let trash_before = r.trash_size(1);

    fire_when_digivolving(&mut r, lkx);

    // First selection: the De-Digivolve target (mandatory; one candidate).
    let view = r
        .pending_selection_view()
        .expect("De-Digivolve target selection must install");
    assert_eq!(view.selecting_player, 0);
    assert!(!view.is_optional, "the De-Digivolve pick is mandatory");
    r.execute_action(0, view.valid_action_ids[0])
        .expect("select the opponent stack");

    // Second selection: the can't-digivolve target.
    let view = r
        .pending_selection_view()
        .expect("can't-digivolve selection must install");
    r.execute_action(0, view.valid_action_ids[0])
        .expect("select the can't-digivolve target");
    let _ = r.auto_resolve();

    // 3 pops: Lv6, Lv5, Lv4 trashed; Lv3 base remains.
    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(
        perm.stack_size(),
        1,
        "3 De-Digivolve 1 instances popped 3 cards"
    );
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "OPP-L3");
    assert_eq!(r.trash_size(1), trash_before + 3);
    // The chosen Digimon can't digivolve until the end of the opponent's turn.
    assert!(
        r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "the selected opponent Digimon must carry CannotDigivolve"
    );
}

/// "You can't trash past level 3 cards" — the standard De-Digivolve floor.
#[test]
fn bt19_073_de_digivolve_respects_level_3_floor() {
    let mut r = base_runner();
    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let _a = r.place_on_field(0, "ALLY-A", Some(0));
    let _b = r.place_on_field(0, "ALLY-B", Some(0));
    // Opponent stack of only 2 cards: Lv3 base + Lv4 — only ONE legal pop
    // even though 3 instances run.
    let opp = r.place_stack(1, &["OPP-L3", "OPP-L4"]);

    fire_when_digivolving(&mut r, lkx);
    let view = r.pending_selection_view().expect("De-Digivolve pick");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    if let Some(view) = r.pending_selection_view() {
        r.execute_action(0, view.valid_action_ids[0]).unwrap();
    }
    let _ = r.auto_resolve();

    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(
        perm.stack_size(),
        1,
        "the Lv3 base cannot be trashed — only the Lv4 pops"
    );
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "OPP-L3");
}

/// Judge-quiz Q15 mechanism (per-card view): when a pop exposes Gallantmon
/// (X Antibody) EX8-073 as the top card while its owner has 0 or less memory,
/// the REMAINING <De-Digivolve 1> instances are halted.
#[test]
fn bt19_073_remaining_de_digivolves_halted_by_exposed_immunity() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-073 loads")
        .dsl_card("BT19-072")
        .expect("BT19-072 loads")
        .dsl_card("EX8-073")
        .expect("EX8-073 Gallantmon (X Antibody) loads")
        .add_card({
            let mut c = make_test_card("OPP-TOP", "OppTop");
            c.card_kind = CardKind::Digimon;
            c.level = Some(7);
            c.dp = Some(16000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-L5X", "OppFive");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(7000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-L4X", "OppFour");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(5000);
            c
        })
        .add_card({
            let mut c = make_test_card("ALLY", "Ally");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    // Gauge at 0 on the turn player's side ⇒ the opponent has 0 memory
    // (own_memory_lte: 0 holds for player 1) ⇒ EX8-073's immunity is live
    // the moment it is exposed.
    r.game.set_memory(0);

    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let _a = r.place_on_field(0, "ALLY", Some(0));
    let _b = r.place_on_field(0, "ALLY", Some(0));
    // Opponent stack (bottom→top): Lv4 / Lv5 / EX8-073 / OPP-TOP.
    let opp = r.place_stack(1, &["OPP-L4X", "OPP-L5X", "EX8-073", "OPP-TOP"]);
    let trash_before = r.trash_size(1);

    fire_when_digivolving(&mut r, lkx);
    let view = r.pending_selection_view().expect("De-Digivolve pick");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    if let Some(view) = r.pending_selection_view() {
        r.execute_action(0, view.valid_action_ids[0]).unwrap();
    }
    let _ = r.auto_resolve();

    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "EX8-073",
        "after the first pop exposes Gallantmon (X Antibody), its immunity halts \
         the remaining <De-Digivolve 1> instances (judge-quiz Q15)"
    );
    assert_eq!(perm.stack_size(), 3, "only OPP-TOP was trashed");
    assert_eq!(r.trash_size(1), trash_before + 1);
}

/// Immunity-OFF contrast: with the gauge negative for the turn player, the
/// opponent has POSITIVE memory, EX8-073's gate is off, and all instances run.
#[test]
fn bt19_073_de_digivolves_through_ex8_073_when_opponent_has_memory() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-073 loads")
        .dsl_card("BT19-072")
        .expect("BT19-072 loads")
        .dsl_card("EX8-073")
        .expect("EX8-073 loads")
        .add_card({
            let mut c = make_test_card("OPP-TOP", "OppTop");
            c.card_kind = CardKind::Digimon;
            c.level = Some(7);
            c.dp = Some(16000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-L5X", "OppFive");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(7000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-L4X", "OppFour");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(5000);
            c
        })
        .add_card({
            let mut c = make_test_card("ALLY", "Ally");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    // Gauge at -3 for the turn player ⇒ the opponent has 3 memory ⇒
    // EX8-073's "while you have 0 or less memory" gate is OFF.
    r.game.set_memory(-3);

    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let _a = r.place_on_field(0, "ALLY", Some(0));
    let _b = r.place_on_field(0, "ALLY", Some(0));
    let opp = r.place_stack(1, &["OPP-L4X", "OPP-L5X", "EX8-073", "OPP-TOP"]);

    fire_when_digivolving(&mut r, lkx);
    let view = r.pending_selection_view().expect("De-Digivolve pick");
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    if let Some(view) = r.pending_selection_view() {
        r.execute_action(0, view.valid_action_ids[0]).unwrap();
    }
    let _ = r.auto_resolve();

    let perm = &r.game.player(1).battle_area[opp.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "OPP-L4X",
        "with the immunity gate off, all 3 instances resolve (Lv4 base remains)"
    );
    assert_eq!(perm.stack_size(), 1);
}

/// NEGATIVE: with no opponent Digimon the clause does not fire (no stuck
/// mandatory selection).
#[test]
fn bt19_073_when_digivolving_no_opponent_digimon_is_clean_noop() {
    let mut r = base_runner();
    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    fire_when_digivolving(&mut r, lkx);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "no opponent Digimon ⇒ the clause must resolve cleanly"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] Knightmon-text aura
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with [LordKnightmon] in the digivolution cards, your Digimon
/// with [Knightmon] in NAME or TEXT gain <Alliance> and +3000 DP.
#[test]
fn bt19_073_aura_buffs_knightmon_text_allies() {
    let mut r = base_runner();
    let _lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let by_name = r.place_on_field(0, "KNIGHT-NAME", Some(0));
    let by_text = r.place_on_field(0, "KNIGHT-TEXT", Some(0));
    let plain = r.place_on_field(0, "ALLY-A", Some(0));

    r.game.tick_declarative_effects();

    for (handle, label) in [(by_name, "name arm"), (by_text, "text arm")] {
        assert_eq!(
            r.game.modifiers.sum(handle, ModifierType::ChangeDp),
            3000,
            "{label}: +3000 DP while the gate holds"
        );
        assert!(
            r.game.has_keyword(handle, Keyword::Alliance),
            "{label}: gains <Alliance>"
        );
    }
    assert_eq!(
        r.game.modifiers.sum(plain, ModifierType::ChangeDp),
        0,
        "an ally without [Knightmon] in name/text is unaffected"
    );
    assert!(!r.game.has_keyword(plain, Keyword::Alliance));
}

/// The aura also reaches LordKnightmon (X Antibody) ITSELF — its own text
/// contains "Knightmon".
#[test]
fn bt19_073_aura_includes_self() {
    let mut r = base_runner();
    let lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.modifiers.sum(lkx, ModifierType::ChangeDp),
        3000,
        "LordKnightmon (X Antibody) has [Knightmon] in its own name/text"
    );
    assert!(r.game.has_keyword(lkx, Keyword::Alliance));
}

/// NEGATIVE: without [LordKnightmon]/[X Antibody] in the digivolution cards
/// (played directly — no sources), the aura stays off.
#[test]
fn bt19_073_aura_off_without_gate_sources() {
    let mut r = base_runner();
    let _lkx = r.place_on_field(0, CARD_ID, Some(0));
    let knight = r.place_on_field(0, "KNIGHT-NAME", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(
        r.game.modifiers.sum(knight, ModifierType::ChangeDp),
        0,
        "no [LordKnightmon]/[X Antibody] source ⇒ the aura gate is off"
    );
    assert!(!r.game.has_keyword(knight, Keyword::Alliance));
}

/// The aura never reaches the OPPONENT's Knightmon-text Digimon.
#[test]
fn bt19_073_aura_does_not_buff_opponent() {
    let mut r = base_runner();
    let _lkx = r.place_stack(0, &["BT19-072", CARD_ID]);
    let opp_knight = r.place_on_field(1, "KNIGHT-NAME", Some(0));
    r.game.tick_declarative_effects();
    assert_eq!(r.game.modifiers.sum(opp_knight, ModifierType::ChangeDp), 0);
    assert!(!r.game.has_keyword(opp_knight, Keyword::Alliance));
}
