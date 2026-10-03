//! BT26-028 Medicmon — Digimon Lv.4 Yellow, Sup./Appmon, Medical/Seven Code,
//! Life. <Barrier> <Detach ([Seven Code])>; [On Play][When Digivolving] may
//! link 1 Lv.3 [Life]/[System]/[Seven Code] Digimon card from its
//! digivolution cards to it free; Assembly -2; App Fusion [Aidmon] &
//! [Supplemon] & [Spamon]; Link [Appmon] cost 3, DP+3000; [When Linking] 1
//! opponent Digimon can't activate [When Digivolving] and gets -3000 DP until
//! their turn ends.
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/BT26/Yellow/BT26_028.cs

#![allow(dead_code, unused_imports)]

use super::support::*;
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost};
use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, EffectTiming, Keyword, ModifierType};
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT26-028";
const MAIL: &str = "BT26-019";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT26-028 compiles")
        .dsl_card(MAIL)
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
            "LIFE-L3",
            "LifeL3",
            CardColor::Red,
            3,
            4,
            &["Life"],
        ))
        .add_card(digimon(
            "LIFE-L2",
            "LifeL2",
            CardColor::Red,
            2,
            0,
            &["Life"],
        ))
        .add_card(digimon(
            "SC-LINK",
            "SevenLink",
            CardColor::Red,
            3,
            4,
            &["Seven Code"],
        ))
        .add_card(digimon("OPP-D", "OppD", CardColor::Red, 5, 6, &[]))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
}

#[test]
fn bt26_028_structure() {
    let r = base().start();
    let c = r.compiled_card(ID).unwrap();
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(5000), Some(5)));
    let has = |k: CompiledAltPathKind, cost: i32| {
        c.alt_paths
            .iter()
            .any(|p| p.kind == k && p.cost == Some(CompiledCost::Literal(cost)))
    };
    assert!(has(CompiledAltPathKind::AppFusion, 0));
    assert!(has(CompiledAltPathKind::Assembly, 2));
    assert!(c
        .alt_paths
        .iter()
        .any(|p| p.kind == CompiledAltPathKind::Digivolve
            && p.from
                .as_ref()
                .is_some_and(|f| f.trait_has.as_deref() == Some("Stnd."))));
}

#[test]
fn bt26_028_has_barrier() {
    let mut r = base().start();
    let h = r.place_on_field(0, ID, Some(0));
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(h, Keyword::Barrier));
}

#[test]
fn bt26_028_on_play_links_lv3_source() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let h = r.place_stack(0, &["LIFE-L2", MAIL, ID]);
    fire(&mut r, EffectTiming::OnPlay, h);
    assert_eq!(non_pass(&r).len(), 1, "only the Lv.3 [Seven Code] source");
    pick_first(&mut r, 0);
    drain_first(&mut r);
    assert_eq!(linked_ids(&r, h), vec![MAIL.to_string()]);
    assert_eq!(source_ids(&r, h), vec!["LIFE-L2".to_string()]);
    assert_eq!(r.memory(), 3, "without paying the cost");
}

#[test]
fn bt26_028_when_digivolving_link_is_optional() {
    let mut r = base().memory(3).start();
    r.game.enter_main_phase();
    let h = r.place_stack(0, &["LIFE-L3", ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    pass(&mut r, 0);
    assert!(linked_ids(&r, h).is_empty());
}

#[test]
fn bt26_028_when_linking_locks_wd_and_minus_3000() {
    let mut r = base().hand(0, &[ID]).memory(5).start();
    r.game.enter_main_phase();
    r.place_on_field(0, "APP-HOST", Some(0));
    let opp = r.place_on_field(1, "OPP-D", Some(0));
    let dp0 = r.effective_dp(opp).unwrap();
    link_from_hand(&mut r, 0, ID);
    drain_first(&mut r);
    assert!(r
        .modifiers()
        .has(opp, ModifierType::CannotActivateWhenDigivolvingEffects));
    assert_eq!(r.effective_dp(opp), Some(dp0 - 3000));
    r.end_turn();
    assert_eq!(
        r.effective_dp(opp),
        Some(dp0 - 3000),
        "lasts through their turn"
    );
}

#[test]
fn bt26_028_detach() {
    let mut r = base().start();
    r.game.enter_main_phase();
    let h = r.place_on_field(0, ID, Some(0));
    r.push_linked_owned(h, "SC-LINK", 0);
    r.game
        .delete_permanent_with_cause(h, ReplacementCause::OpponentEffect);
    // <Barrier> is battle-only; an effect deletion offers Detach.
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
    pick_first(&mut r, 0);
    assert_eq!(field_ids(&r, 0), vec![ID.to_string()]);
}
