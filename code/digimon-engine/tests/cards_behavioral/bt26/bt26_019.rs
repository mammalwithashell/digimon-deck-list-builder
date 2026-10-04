//! BT26-019 Mailmon — Digimon, Lv.3, Blue, DP 4000, Cost 4.
//! Form Stnd./Appmon, Type Mail/Seven Code, Attribute Social.
//!
//! Printed (data/card_bundles/BT26-019.md):
//!   <Detach ([Seven Code] trait)> [When Attacking] If your hand has 7 or
//!   fewer cards, <Draw 1>. Link DP+3000. <Link> [Appmon] trait: Cost 3.
//!   [When Linking] 1 of your opponent's Digimon or Tamers can't suspend until
//!   their turn ends.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Blue/BT26_019.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
};
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-019";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-019 compiles")
        .add_card(digimon(
            "APP-HOST",
            "AppHost",
            CardColor::White,
            4,
            5,
            &["Appmon"],
        ))
        .add_card(digimon(
            "SC-LINK",
            "SevenLink",
            CardColor::Red,
            3,
            4,
            &["Seven Code"],
        ))
        .add_card(digimon(
            "PLAIN-LINK",
            "PlainLink",
            CardColor::Red,
            3,
            4,
            &["Tool"],
        ))
        .add_card(digimon("OPP-D", "OppD", CardColor::Red, 4, 4, &[]))
        .add_card(tamer("OPP-T", "OppT", CardColor::Red, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_019_structure() {
    let r = base().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!((c.level, c.dp, c.cost), (Some(3), Some(4000), Some(4)));
    for t in ["Stnd.", "Appmon", "Social", "Mail", "Seven Code"] {
        assert!(c.traits.iter().any(|x| x == t), "trait {t}");
    }
    let appmon_alt = c.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && p.cost == Some(CompiledCost::Literal(0))
            && p.from
                .as_ref()
                .is_some_and(|f| f.level_eq == Some(2) && f.trait_has.as_deref() == Some("Appmon"))
    });
    assert!(appmon_alt, "[Digivolve] Lv.2 w/[Appmon]: Cost 0");
    assert!(c.effects.iter().any(|e| matches!(
        e,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost: 3, .. })
    )));
    assert!(c.effects.iter().any(|e| matches!(
        e,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Linked,
            dp_modifier: Some(3000),
            ..
        })
    )));
}

// ── Detach ───────────────────────────────────────────────────────────────────

#[test]
fn bt26_019_detach_trashes_seven_code_link_and_stays() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::OpponentEffect);
    assert!(r.pending_selection().is_some(), "Detach accept prompt");
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()], "Mailmon stays");
    assert!(r.game.players[0].battle_area[0].linked_cards.is_empty());
    assert!(trash_ids(&r, 0).contains(&"SC-LINK".to_string()));
}

#[test]
fn bt26_019_detach_by_battle_also_applies() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::Battle);
    assert!(
        r.pending_selection().is_some(),
        "battle deletion is not 'your effect'"
    );
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0).len(), 1);
}

#[test]
fn bt26_019_detach_only_offers_seven_code_link_cards() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "PLAIN-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::OpponentEffect);
    assert!(
        r.pending_selection().is_none(),
        "no [Seven Code] link card: Detach is not payable"
    );
    assert!(field_ids(&r, 0).is_empty(), "Mailmon left");
}

#[test]
fn bt26_019_detach_choice_excludes_non_seven_code() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "PLAIN-LINK", 0);
    r.push_linked_owned(m, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::OpponentEffect);
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    assert_eq!(non_pass(&r).len(), 1, "only the [Seven Code] link card");
    pick_first(&mut r, 0);
    let linked: Vec<String> = r.game.players[0].battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(linked, vec!["PLAIN-LINK".to_string()]);
}

#[test]
fn bt26_019_detach_not_offered_for_own_effect() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::OwnEffect);
    assert!(r.pending_selection().is_none(), "own effects bypass Detach");
    assert!(field_ids(&r, 0).is_empty());
}

#[test]
fn bt26_019_detach_declined_leaves() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(m, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(m, ReplacementCause::OpponentEffect);
    r.execute_action(0, PASS).unwrap();
    assert!(field_ids(&r, 0).is_empty());
}

// ── [When Attacking] ─────────────────────────────────────────────────────────

#[test]
fn bt26_019_when_attacking_draws_with_7_cards() {
    let mut r = base().hand(0, &["FILLER"; 7]).start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 8);
}

#[test]
fn bt26_019_when_attacking_no_draw_with_8_cards() {
    let mut r = base().hand(0, &["FILLER"; 8]).start();
    r.game.enter_main_phase();
    let m = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, m);
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 8);
}

// ── Link ─────────────────────────────────────────────────────────────────────

#[test]
fn bt26_019_link_from_hand_pays_3_buffs_and_locks_suspend() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "APP-HOST", Some(0));
    let opp = r.place_on_field(1, "OPP-D", Some(0));
    let dp0 = r.effective_dp(host).unwrap();
    let mem0 = r.memory();
    link_from_hand(&mut r, 0, ID);
    drain_first(&mut r);
    assert_eq!(r.game.players[0].battle_area[0].linked_cards.len(), 1);
    assert_eq!(r.memory(), mem0 - 3, "link cost 3");
    r.game.tick_declarative_effects();
    assert_eq!(r.effective_dp(host), Some(dp0 + 3000), "Link DP+3000");
    assert!(
        r.modifiers().has(opp, ModifierType::CannotSuspend),
        "[When Linking] opp Digimon can't suspend"
    );
}

#[test]
fn bt26_019_when_linking_can_target_tamer() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    let t = r.place_on_field(1, "OPP-T", Some(0));
    link_from_hand(&mut r, 0, ID);
    drain_first(&mut r);
    assert!(r.modifiers().has(t, ModifierType::CannotSuspend));
}

#[test]
fn bt26_019_link_needs_appmon_host() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "OPP-D", Some(0));
    assert!(
        !r.game.hand_link_available(0, 0),
        "no [Appmon] host: Mailmon can't link"
    );
}
