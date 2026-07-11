//! ST23-09 Atratusmon / "Eclipse Impact" — DUAL card (Digimon + Option).
//! Lv.6 Green/Black, Mutant / Glowing Dawn / BEATBREAK, DP 12000.
//!
//! # Card text (card image + DCGO ST23_09.cs — authoritative)
//! DIGIMON FACE
//!   <Digivolve> Lv.5 w/ [Glowing Dawn] trait: Cost 3.
//!   <Security A. +1> <Reboot> <Blocker>
//!   [When Digivolving] [When Attacking] [Once Per Turn] Your opponent's Digimon
//!     effects don't affect this Digimon until their turn ends. Then, delete 1
//!     of your opponent's Digimon with the lowest DP.
//! OPTION FACE — "Eclipse Impact", Use 5
//!   Use Req. [BEATBREAK] trait.
//!   [Main] Suspend 1 of your opponent's Digimon. Then, return 1 of your
//!     opponent's suspended Digimon with the highest DP to the bottom of the deck.
//!   <Arts Digivolve>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_09.cs
//!
//! # Patterns covered
//! - G-DSL-DUAL-PER-FACE-EFFECTS (both faces), G-DSL-ARTS-DIGIVOLVE.
//! - Static keywords on the Digimon face (Security A.+1 / Reboot / Blocker).
//! - Digimon [WD/WA][OPT]: self effect-immunity + delete lowest-DP opp Digimon.
//! - Option [Main]: suspend opp Digimon → bounce highest-DP suspended opp Digimon.
//!
//! # Verdict — IMPLEMENTED (per-face effects + Arts digivolve)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "ST23-09";

fn beatbreak_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(9000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["BEATBREAK".to_string(), "Glowing Dawn".to_string()];
    c
}

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![CardColor::Blue];
    c
}

/// Green Lv.5 [Glowing Dawn] base for digivolve / arts.
fn green_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-09 in embedded DSL pack")
        .add_card(beatbreak_digimon("BB-DIGI"))
        .add_card(green_base("GREEN-BASE"))
        .add_card(opp_digimon("OPP-LOW", 3000))
        .add_card(opp_digimon("OPP-HIGH", 9000))
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn st23_09_is_dual_with_both_faces_populated() {
    let runner = builder().memory(8).start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled ST23-09");
    assert_eq!(compiled.kind, CompiledCardKind::Dual);
    let dual = compiled.dual.as_ref().unwrap();
    assert_eq!(dual.digimon.level, 6);
    assert_eq!(dual.option.use_cost, 5);
    // Digimon face: 3 static keyword grants + 1 WD/WA clause.
    assert_eq!(dual.digimon.effects.len(), 4);
    // Option face: 1 [Main] clause.
    assert_eq!(dual.option.effects.len(), 1);
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "arts_digivolve folds into the ArtsDigivolve keyword"
    );
}

#[test]
fn st23_09_digimon_face_declares_static_keywords() {
    let runner = builder().memory(8).start();
    let dual = runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .dual
        .clone()
        .unwrap();
    let kw_grants: Vec<&str> = dual
        .digimon
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(kw_grants.contains(&"SecurityAttackPlus"));
    assert!(kw_grants.contains(&"Reboot"));
    assert!(kw_grants.contains(&"Blocker"));
}

#[test]
fn st23_09_static_keywords_live_on_field() {
    // The Digimon-face static keywords must materialize when the card is in
    // play as a Digimon (they were authored on `dual.digimon.effects`).
    let mut r = builder().memory(8).start();
    let me = r.place_stack(0, &["GREEN-BASE", CARD_ID]);
    assert!(
        r.game.has_keyword(me, Keyword::Reboot),
        "Reboot lives on the Digimon face on field"
    );
    assert!(
        r.game.has_keyword(me, Keyword::Blocker),
        "Blocker lives on the Digimon face on field"
    );
    assert!(
        r.game.has_keyword(me, Keyword::SecurityAttackPlus(1)),
        "Security A. +1 lives on the Digimon face on field"
    );
}

// ─── Digimon [WD/WA][OPT]: immunity + delete lowest-DP ───────────────────────

fn fire_wd(r: &mut DebugRunner, handle: PermanentHandle) {
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
}

#[test]
fn st23_09_wd_deletes_lowest_dp_opponent_digimon() {
    let mut r = builder().memory(8).start();
    let me = r.place_stack(0, &["GREEN-BASE", CARD_ID]);
    let low = r.place_on_field(1, "OPP-LOW", Some(0)); // 3000 DP
    let _high = r.place_on_field(1, "OPP-HIGH", Some(0)); // 9000 DP
    r.game.enter_main_phase();

    fire_wd(&mut r, me);

    // The delete clause installs an OppField selection restricted to the
    // lowest-DP Digimon. The lowest-DP one (OPP-LOW) is the only legal target.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("delete-lowest-DP selection installs");
    assert_eq!(sel.kind, SelectionKind::OppField);
    let low_action = encode_attack(0, low.index as u16);
    assert!(
        sel.valid_action_ids.contains(&low_action),
        "lowest-DP opponent Digimon is a legal delete target"
    );
    let battle_before = r.battle_area_size(1);
    r.execute_action(0, low_action).expect("delete lowest");
    assert_eq!(
        r.battle_area_size(1),
        battle_before - 1,
        "lowest-DP opponent Digimon was deleted"
    );
}

// ─── Option [Main]: suspend + bounce highest-DP suspended ────────────────────

#[test]
fn st23_09_option_main_suspends_then_bounces_highest_dp_suspended() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(8).start();
    // BEATBREAK Digimon satisfies the Use Req.
    r.place_on_field(0, "BB-DIGI", Some(0));
    // Two opponent Digimon — a low and a high DP.
    let low = r.place_on_field(1, "OPP-LOW", Some(0)); // 3000
    let high = r.place_on_field(1, "OPP-HIGH", Some(0)); // 9000
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    // Step 1: suspend selection (any opponent Digimon). Suspend the HIGH one so
    // it becomes the highest-DP *suspended* Digimon for the bounce.
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    r.execute_action(0, encode_attack(0, high.index as u16))
        .expect("suspend the high-DP opp Digimon");
    assert!(
        r.game.player(1).battle_area[high.index as usize].is_suspended,
        "the chosen opponent Digimon is suspended"
    );

    // Step 2: bounce the highest-DP *suspended* opponent Digimon. Only the
    // suspended high-DP one qualifies (low is unsuspended).
    let bounce_sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("bounce selection installs");
    assert_eq!(bounce_sel.kind, SelectionKind::OppField);
    assert!(
        bounce_sel
            .valid_action_ids
            .contains(&encode_attack(0, high.index as u16)),
        "the suspended high-DP Digimon is the bounce target"
    );
    assert!(
        !bounce_sel
            .valid_action_ids
            .contains(&encode_attack(0, low.index as u16)),
        "the UNsuspended low-DP Digimon is NOT a legal bounce target"
    );

    let deck_before = r.deck_size(1);
    let battle_before = r.battle_area_size(1);
    r.execute_action(0, encode_attack(0, high.index as u16))
        .expect("bounce highest-DP suspended");
    assert_eq!(r.battle_area_size(1), battle_before - 1);
    assert_eq!(
        r.deck_size(1),
        deck_before + 1,
        "bounced to the deck bottom"
    );
}

// ─── Arts Digivolve wiring ───────────────────────────────────────────────────

fn drive_to_arts(r: &mut DebugRunner, base: PermanentHandle) {
    loop {
        let sel = r.game.pending_selection.as_ref().expect("selection parked");
        let is_arts = sel.kind == SelectionKind::OwnField
            && sel.is_optional
            && sel
                .valid_action_ids
                .contains(&encode_attack(0, base.index as u16));
        if is_arts {
            return;
        }
        let action = if sel.is_optional {
            PASS
        } else {
            sel.valid_action_ids[0]
        };
        r.execute_action(0, action).expect("drive follow-up");
    }
}

#[test]
fn st23_09_arts_digivolve_arms_and_stacks_onto_base() {
    let mut r = builder()
        .deck(0, &["BB-DIGI"])
        .hand(0, &[CARD_ID])
        .memory(8)
        .start();
    // A green Lv.5 [Glowing Dawn] base satisfies both the Arts can_digivolve
    // gate AND (being Glowing Dawn, not BEATBREAK) does NOT satisfy Use Req —
    // so add a BEATBREAK Digimon for the Use Req.
    let base = r.place_on_field(0, "GREEN-BASE", Some(0));
    r.place_on_field(0, "BB-DIGI", Some(0));
    // No opponent Digimon → the [Main] suspend/bounce steps no-op and the
    // resolution proceeds straight to the Arts prompt.
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    drive_to_arts(&mut r, base);
    let arts = r.game.pending_selection.as_ref().unwrap();
    assert!(arts.is_optional, "Arts is declinable");
    assert!(
        arts.valid_action_ids
            .contains(&encode_attack(0, base.index as u16)),
        "the green Lv.5 Glowing Dawn Digimon is a legal Arts target"
    );

    let trash_before = r.trash_size(0);
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept Arts");
    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.stack_size(), 2);
    assert_eq!(perm.top_card().card_id(&r.game.card_data), CARD_ID);
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "Arts prevented the normal Option trash"
    );
}

// ─── Printed digivolution routes (cheaper-than-printed guard, per card) ───────
// Official Bandai DB (data/card_bundles/ST23-09.md): standard circles are
// Green Lv.5 / cost 4 and Black Lv.5 / cost 4; the ONLY cost-3 route is the
// special condition "Lv.5 w/[Glowing Dawn] trait: Cost 3" (DCGO ST23_09.cs
// AddSelfDigivolutionRequirementStaticEffect(HasGlowingDawnTraits, 3, level 5)).

/// A green Lv.5 WITHOUT the [Glowing Dawn] trait — only the printed Green
/// Lv.5 circle (cost 4) may apply to it.
fn plain_green_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Insectoid".to_string()];
    c
}

/// Stage `base` on the field with ST23-09 in hand, Main phase, memory 15.
fn digivolve_runner(base: CardData) -> (DebugRunner, usize) {
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-09 in embedded DSL pack")
        .add_card(base)
        .hand(0, &[CARD_ID])
        .memory(15)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// A green Lv.5 WITHOUT [Glowing Dawn] must pay the PRINTED circle cost (4).
/// A cost-3 route for it would be cheaper than any printed requirement — the
/// BT21-015/017 free-digivolve bug class.
#[test]
fn st23_09_green_lv5_without_glowing_dawn_pays_printed_cost_4() {
    let (mut r, slot) = digivolve_runner(plain_green_lv5("PLAIN-G5"));
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        proceeded,
        "exactly one distinct-cost route (the printed Green Lv.5 circle) applies to a \
         non-[Glowing Dawn] green Lv.5 → the digivolve must commit without a cost choice"
    );
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID,
        "ST23-09 stacked onto the base"
    );
    assert_eq!(
        mem_before - r.game.memory,
        4,
        "the printed Green Lv.5 circle costs 4 — the cost-3 route is [Glowing Dawn]-gated \
         and must NOT apply to a base without the trait"
    );
}

/// A green Lv.5 WITH [Glowing Dawn] satisfies BOTH the printed circle (4) and
/// the trait special condition (3) → the engine must surface the cost CHOICE
/// (rule 17, no auto-selection) and honor a cost-3 pick.
#[test]
fn st23_09_glowing_dawn_green_lv5_offers_cost_3_vs_4_choice() {
    let (mut r, slot) = digivolve_runner(green_base("GD-CHOICE"));
    let mem_before = r.game.memory;
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !proceeded,
        "a [Glowing Dawn] green Lv.5 satisfies two distinct costs (3 and 4) — the \
         digivolve must PAUSE for a cost choice, not auto-pay"
    );
    let view = r
        .pending_selection_view()
        .expect("a cost-choice selection must be pending");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let choices = view
        .effect_choices
        .clone()
        .expect("the cost choice carries effect_choices");
    let labels: Vec<String> = choices.iter().map(|c| c.label.to_lowercase()).collect();
    assert_eq!(
        choices.len(),
        2,
        "exactly two distinct costs (3 trait / 4 circle) must be offered; got {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l.contains("cost 3")),
        "the [Glowing Dawn] cost-3 special condition must be offered (labels {labels:?})"
    );
    assert!(
        labels.iter().any(|l| l.contains("cost 4")),
        "the printed cost-4 circle must be offered (labels {labels:?})"
    );
    let cost3 = choices
        .iter()
        .find(|c| c.label.to_lowercase().contains("cost 3"))
        .unwrap();
    r.execute_action(0, cost3.action_id)
        .expect("resolve the cost-3 option");
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID,
        "ST23-09 stacked onto the [Glowing Dawn] base"
    );
    assert_eq!(
        mem_before - r.game.memory,
        3,
        "the trait route pays exactly 3"
    );
}
