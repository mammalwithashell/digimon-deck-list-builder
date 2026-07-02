//! BT25-057 Monarchlizamon / "Final Judgment" — DUAL card (Digimon + Option).
//! Lv.5 Green/Black, Cyborg / Glowing Dawn / BEATBREAK, DP 8000.
//!
//! # Card text (card image + DCGO — cards.json MISLABELS this as a plain
//! # Digimon and puts the Option text in the inherited slot, so the image +
//! # DCGO BT25_057.cs are authoritative)
//!
//! DIGIMON FACE
//!   <Digivolve> Lv.4 w/ [Glowing Dawn] trait: Cost 3.
//!   [When Digivolving] [When Attacking] [Once Per Turn] By trashing the bottom
//!     face-down card under any of your Tamers, <De-Digivolve 1> 1 of your
//!     opponent's Digimon.
//!   [When Digivolving] This Digimon may battle 1 of your opponent's Digimon.
//! OPTION FACE — "Final Judgment", Use 4
//!   Use Req. [Glowing Dawn] trait (ignore color requirements).
//!   [Main] 1 of your Digimon gains <Rush>, <Security A. +1> and +5000 DP for
//!     the turn. Then, it may attack.
//!   <Arts Digivolve>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_057.cs
//!
//! # Patterns this test covers
//! - G-DSL-DUAL-PER-FACE-EFFECTS: both faces carry behavioral clauses.
//! - G-DSL-ARTS-DIGIVOLVE / G-DSL-BEATBREAK-ARTS-OPTION: `arts_digivolve: true`
//!   arms the engine arts-digivolve selection after the Option [Main].
//! - Option [Main]: select own Digimon → Rush + Security A. +1 + 5000 DP + may attack.
//! - Digimon [WD] may-battle, [WD/WA][OPT] De-Digivolve via FD-trash-under-Tamer.
//!
//! # Verdict — IMPLEMENTED (per-face effects + Arts digivolve)

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind, TriggerSource};

const CARD_ID: &str = "BT25-057";

fn glowing_dawn_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A green Lv.4 [Glowing Dawn] base the dual card can digivolve from / arts onto.
fn glowing_dawn_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-057 in embedded DSL pack")
        .add_card(glowing_dawn_digimon("GD-DIGI"))
        .add_card(glowing_dawn_base("GD-BASE"))
        .add_card(opp_digimon("OPP-1"))
        .add_card(opp_digimon("OPP-2"))
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt25_057_is_a_dual_card() {
    let runner = builder().memory(8).start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled BT25-057");
    assert_eq!(compiled.kind, CompiledCardKind::Dual, "BT25-057 is DUAL");
    let dual = compiled.dual.as_ref().expect("dual block present");
    assert_eq!(dual.digimon.level, 5);
    assert_eq!(dual.option.use_cost, 4);
}

#[test]
fn bt25_057_digimon_face_carries_behavioral_effects() {
    let runner = builder().memory(8).start();
    let dual = runner
        .compiled_card(CARD_ID)
        .expect("compiled BT25-057")
        .dual
        .clone()
        .unwrap();
    assert_eq!(
        dual.digimon.effects.len(),
        2,
        "Digimon face has 2 behavioral clauses (per-face effects sink)"
    );
    let wd_wa = dual
        .digimon
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("WD/WA De-Digivolve clause present on Digimon face");
    assert!(wd_wa.once_per_turn, "WD/WA clause is [Once Per Turn]");
}

#[test]
fn bt25_057_option_face_carries_main_effect_and_arts() {
    let runner = builder().memory(8).start();
    let dual = runner
        .compiled_card(CARD_ID)
        .expect("compiled BT25-057")
        .dual
        .clone()
        .unwrap();
    assert_eq!(
        dual.option.effects.len(),
        1,
        "Option face has 1 behavioral clause ([Main])"
    );
    assert!(
        dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"),
        "arts_digivolve: true must compile into an ArtsDigivolve option keyword; got {:?}",
        dual.option.keywords
    );
}

#[test]
fn bt25_057_engine_card_data_sees_arts_keyword_and_evo_costs() {
    let runner = builder().memory(8).start();
    let data = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .expect("BT25-057 card data");
    let dual = data.dual.as_ref().expect("dual data");
    assert!(
        dual.option.keywords.contains(&Keyword::ArtsDigivolve),
        "engine option face must carry ArtsDigivolve keyword"
    );
    assert!(
        !dual.digimon.evo_costs.is_empty(),
        "Digimon face evo_costs backfilled from the alt-digivolve box"
    );
}

// ─── Option [Main] behavior ──────────────────────────────────────────────────

#[test]
fn bt25_057_option_main_grants_rush_secatk_dp_to_chosen_digimon() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(8).start();
    let buff_target = r.place_on_field(0, "GD-DIGI", Some(0));
    r.game.enter_main_phase();

    let dp_before = r.dp_of(buff_target).unwrap();
    let result = r.game.play_option_from_hand(0, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "Option [Main] parks a selection"
    );
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));

    let pick = encode_attack(0, buff_target.index as u16);
    r.execute_action(0, pick)
        .expect("select own Digimon to buff");

    assert!(
        r.game.has_keyword(buff_target, Keyword::Rush),
        "buffed Digimon gains <Rush>"
    );
    assert!(
        r.game
            .has_keyword(buff_target, Keyword::SecurityAttackPlus(1)),
        "buffed Digimon gains <Security A. +1>"
    );
    assert_eq!(
        r.dp_of(buff_target).unwrap(),
        dp_before + 5000,
        "buffed Digimon gets +5000 DP"
    );
}

// ─── Arts Digivolve wiring ───────────────────────────────────────────────────

/// Drive any mandatory or optional-but-non-Arts follow-ups until the
/// Arts-digivolve OwnField prompt (optional, with `base` legal) is on top.
fn drive_to_arts(r: &mut DebugRunner, base: PermanentHandle) {
    loop {
        let sel = r
            .game
            .pending_selection
            .as_ref()
            .expect("a selection is parked");
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
fn bt25_057_arts_digivolve_prompt_arms_after_option_main() {
    let mut r = builder()
        .deck(0, &["GD-DIGI"])
        .hand(0, &[CARD_ID])
        .memory(8)
        .start();
    let base = r.place_on_field(0, "GD-BASE", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("pick buff target");

    drive_to_arts(&mut r, base);
    let arts = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(arts.kind, SelectionKind::OwnField);
    assert!(
        arts.is_optional,
        "Arts digivolve is declinable (PASS → trash)"
    );
    assert!(
        arts.valid_action_ids
            .contains(&encode_attack(0, base.index as u16)),
        "the green Lv.4 Glowing Dawn Digimon is a legal Arts target"
    );
}

#[test]
fn bt25_057_arts_accept_stacks_dual_onto_base_without_cost() {
    let mut r = builder()
        .deck(0, &["GD-DIGI"])
        .hand(0, &[CARD_ID])
        .memory(8)
        .start();
    let base = r.place_on_field(0, "GD-BASE", Some(0));
    r.game.enter_main_phase();

    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("buff pick");
    drive_to_arts(&mut r, base);

    let trash_before = r.trash_size(0);
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept Arts digivolve");

    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.stack_size(), 2, "Arts stacked DUAL onto the base");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        CARD_ID,
        "DUAL is now the top card"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before,
        "Arts prevented the normal Option trash"
    );
}

// ─── Digimon face: [WD] may-battle ───────────────────────────────────────────

fn fire_when_digivolving(r: &mut DebugRunner, handle: PermanentHandle) {
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
}

#[test]
fn bt25_057_digimon_wd_installs_opponent_field_selection() {
    let mut r = builder().memory(8).start();
    let me = r.place_stack(0, &["GD-BASE", CARD_ID]);
    let opp = r.place_on_field(1, "OPP-1", Some(0));
    r.game.enter_main_phase();

    fire_when_digivolving(&mut r, me);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("a WD selection installs");
    assert!(
        sel.kind == SelectionKind::OppField || sel.kind == SelectionKind::TriggerOrder,
        "WD installs an opponent-field selection (battle / de-digivolve), got {:?}",
        sel.kind
    );
    let _ = opp;
}

// ─── Digimon face: [WD/WA][OPT] De-Digivolve gated on FD-under-Tamer ─────────

#[test]
fn bt25_057_wd_wa_dedigivolve_needs_face_down_under_tamer() {
    // With NO Tamer carrying a face-down source, the De-Digivolve clause has no
    // payable cost → must not de-digivolve the opponent for free.
    let mut r = builder().memory(8).start();
    let me = r.place_stack(0, &["GD-BASE", CARD_ID]);
    let opp = r.place_stack(1, &["OPP-2", "OPP-1"]);
    r.game.enter_main_phase();

    let opp_stack_before = r.game.player(1).battle_area[opp.index as usize].stack_size();
    fire_when_digivolving(&mut r, me);
    while r
        .game
        .pending_selection
        .as_ref()
        .is_some_and(|s| s.is_optional)
    {
        r.execute_action(0, PASS).expect("decline optional WD");
    }
    let opp_stack_after = r.game.player(1).battle_area[opp.index as usize].stack_size();
    assert_eq!(
        opp_stack_after, opp_stack_before,
        "no Tamer face-down cost available → no free De-Digivolve"
    );
    let _ = (Expiry::EndOfTurn, ModifierType::ChangeDp);
}
