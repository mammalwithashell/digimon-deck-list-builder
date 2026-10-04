//! BT26-086 Dantemon — Digimon Lv.7 White, Unknown/Appmon, Open.
//! <Rush> <Reboot> <Blocker> <Link +6>; [On Play][When Digivolving] may link
//! up to 7 [Appmon] cards with different names from its digivolution cards
//! to it free, then it may attack without suspending; [All Turns][OPT] when
//! it gets linked, may delete 1 opponent Digimon, then if it has 7 link cards
//! return the opponent's top security card to the bottom of the deck.
//! Assembly -7 (7 [Seven Code] Digimon cards with different names).
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/White/BT26_086.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const ID: &str = "BT26-086";

const NAMES: [(&str, &str); 8] = [
    ("APP-1", "Alpha"),
    ("APP-2", "Beta"),
    ("APP-3", "Gamma"),
    ("APP-4", "Delta"),
    ("APP-5", "Epsilon"),
    ("APP-6", "Zeta"),
    ("APP-7", "Eta"),
    ("APP-8", "Theta"),
];

fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-086 compiles")
        .add_card(digimon(
            "APP-1B",
            "Alpha",
            CardColor::Red,
            3,
            4,
            &["Appmon"],
        ))
        .add_card(digimon(
            "NON-APP",
            "NonApp",
            CardColor::Red,
            3,
            4,
            &["Beast"],
        ))
        .add_card(digimon("OPP-D", "OppD", CardColor::Red, 5, 6, &[]))
        .add_card(filler("FILLER"));
    for (id, name) in NAMES {
        b = b.add_card(digimon(id, name, CardColor::Red, 3, 4, &["Appmon"]));
    }
    b.deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 3])
}

#[test]
fn bt26_086_structure() {
    let mut r = base().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!((c.level, c.dp, c.cost), (Some(7), Some(14000), Some(14)));
    assert!(c.alt_paths.iter().any(
        |p| p.kind == CompiledAltPathKind::Assembly && p.cost == Some(CompiledCost::Literal(7))
    ));
    assert!(c.effects.iter().any(|e| matches!(
        e,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            modifier_value: Some(6),
            ..
        })
    )));
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    for k in [Keyword::Rush, Keyword::Reboot, Keyword::Blocker] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

#[test]
fn bt26_086_on_play_links_different_names_only() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let h = r.place_stack(0, &["NON-APP", "APP-1B", "APP-2", "APP-1", ID]);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert_eq!(non_pass(&r).len(), 3, "the three [Appmon] sources");
    pick_first(&mut r, 0); // APP-1B (Alpha)
    let mut log = Vec::new();
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        log.push(format!("{:?} {:?}", v.prompt, v.valid_action_ids));
        if v.prompt.contains("different name") {
            assert_eq!(
                non_pass(&r).len(),
                1,
                "the other [Alpha] is excluded: {log:?}"
            );
            pick_first(&mut r, 0);
        } else if v.valid_action_ids.contains(&PASS) {
            pass(&mut r, 0);
        } else {
            pick_first(&mut r, 0);
        }
    }
    assert!(r.pending_selection().is_none(), "{log:?}");
    let mut linked = linked_ids(&r, h);
    linked.sort();
    assert_eq!(linked, vec!["APP-1B".to_string(), "APP-2".to_string()]);
    assert_eq!(r.memory(), 3, "free");
}

#[test]
fn bt26_086_then_may_attack_without_suspending() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let h = r.place_stack(0, &["NON-APP", ID]);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert!(r.pending_selection().is_some(), "the may-attack prompt");
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert!(
        !r.game.players[0].battle_area[h.index as usize].is_suspended,
        "attacked without suspending"
    );
    assert!(r.security_count(1) < 3, "the attack checked security");
}

fn place_with_links(r: &mut DebugRunner, n: usize) -> digimon_engine::permanent::PermanentHandle {
    let h = r.place_on_field(0, ID, Some(0));
    for (id, _) in NAMES.iter().take(n) {
        r.push_linked_owned(h, id, 0);
    }
    h
}

#[test]
fn bt26_086_when_linked_delete_and_seventh_link_bounces_security() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = place_with_links(&mut r, 6);
    r.place_on_field(1, "OPP-D", Some(0));
    let sec0 = r.security_count(1);
    let deck0 = r.deck_size(1);
    push_link_and_fire(&mut r, h, "APP-7");
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert!(field_ids(&r, 1).is_empty(), "deleted");
    assert_eq!(r.security_count(1), sec0 - 1);
    assert_eq!(r.deck_size(1), deck0 + 1, "to the bottom of the deck");
}

#[test]
fn bt26_086_seven_links_bounce_even_when_delete_declined() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = place_with_links(&mut r, 6);
    r.place_on_field(1, "OPP-D", Some(0));
    let sec0 = r.security_count(1);
    push_link_and_fire(&mut r, h, "APP-7");
    pass(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPP-D".to_string()]);
    assert_eq!(r.security_count(1), sec0 - 1);
}

#[test]
fn bt26_086_eight_links_no_bounce() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = place_with_links(&mut r, 7);
    let sec0 = r.security_count(1);
    push_link_and_fire(&mut r, h, "APP-8");
    drain_first(&mut r);
    assert_eq!(r.security_count(1), sec0, "exactly 7 required");
}

#[test]
fn bt26_086_when_linked_decline_refunds_opt() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = place_with_links(&mut r, 1);
    r.place_on_field(1, "OPP-D", Some(0));
    push_link_and_fire(&mut r, h, "APP-2");
    pass(&mut r, 0);
    assert_eq!(field_ids(&r, 1), vec!["OPP-D".to_string()]);
    push_link_and_fire(&mut r, h, "APP-3");
    assert!(
        r.pending_selection().is_some(),
        "not used → available again"
    );
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert!(field_ids(&r, 1).is_empty());
    push_link_and_fire(&mut r, h, "APP-4");
    assert!(r.pending_selection().is_none(), "used → once per turn");
}

#[test]
fn bt26_086_when_linked_fires_on_opponents_turn() {
    let mut r = base().start();
    r.end_turn();
    let h = place_with_links(&mut r, 1);
    r.place_on_field(1, "OPP-D", Some(0));
    push_link_and_fire(&mut r, h, "APP-2");
    assert!(r.pending_selection().is_some(), "[All Turns]");
}
