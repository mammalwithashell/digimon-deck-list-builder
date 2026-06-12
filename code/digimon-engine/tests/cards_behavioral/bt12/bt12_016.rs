//! BT12-016 WarGrowlmon — Digimon, Lv.5, Red, Cost 8, DP 8000.
//! Traits: Cyborg. Attribute: Virus.
//!
//! # Card text (card image — verbatim)
//!
//! ```text
//! [When Digivolving] Delete 1 of your opponent's Digimon with 4000 DP or
//!   less. If no opponent's Digimon was deleted by this effect, you may
//!   digivolve this Digimon into a level 6 Digimon card with [Gallantmon] in
//!   its name in your hand for the digivolution cost. When this Digimon would
//!   digivolve by this effect, reduce the digivolution cost by 1.
//! Inherited [End of Attack] [Once Per Turn] If your opponent has no Digimon
//!   in play, and this Digimon has [Growlmon] or [Gallantmon] in its name,
//!   gain 2 memory.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Red/BT12_016.cs — select <=4000 →
//! delete; FailureProcess: DigivolveIntoHandOrTrashCard(name contains
//! "Gallantmon", Level 6, payCost: true, reduceCost 1, isHand). Inherited
//! OnEndAttack OPT: opponent battle-area Digimon count == 0 AND carrier top
//! name contains Growlmon/Gallantmon → AddMemory(2).
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt12/BT12-016.yaml

#![allow(dead_code, unused_imports)]

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT12-016";

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT12-016 in embedded DSL pack")
        .dsl_card("BT17-016")
        .expect("BT17-016 Gallantmon loads (the digivolve-into target)")
        .add_card({
            let mut c = make_test_card("OPP-SMALL", "OppSmall");
            c.card_kind = CardKind::Digimon;
            c.level = Some(3);
            c.dp = Some(4000);
            c
        })
        .add_card({
            let mut c = make_test_card("OPP-BIG", "OppBig");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(9000);
            c
        })
        // A non-[Gallantmon] Lv.6 and a [Gallantmon]-named non-Lv.6 to pin
        // the hand filter.
        .add_card({
            let mut c = make_test_card("LV6-OTHER", "Examon");
            c.card_kind = CardKind::Digimon;
            c.level = Some(6);
            c.dp = Some(12000);
            c.colors = vec![CardColor::Red];
            c
        })
        .add_card({
            let mut c = make_test_card("GALL-LV4", "Gallantmon Jr");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(4000);
            c.colors = vec![CardColor::Red];
            c
        })
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt12_016_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(8));
    assert_eq!(card.dp, Some(8000));
    assert!(card.traits.iter().any(|t| t == "Cyborg"));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] delete <=4000 or may digivolve into Gallantmon
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE delete branch: a 4000-DP opponent Digimon is deleted and NO
/// digivolve prompt follows.
#[test]
fn bt12_016_when_digivolving_deletes_4000_or_less() {
    let mut r = base_runner();
    let war = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    let _hand = r.game.players[0].hand.len();

    fire(&mut r, EffectTiming::WhenDigivolving, war);
    let view = r
        .pending_selection_view()
        .expect("mandatory delete pick when a <=4000 DP target exists");
    assert!(!view.is_optional);
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();

    assert_eq!(r.battle_area_size(1), 0, "the 4000-DP Digimon is deleted");
    // Still WarGrowlmon on top — no digivolve happened.
    assert_eq!(
        r.game.player(0).battle_area[war.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID,
        "the effect deleted ⇒ no digivolve fallback"
    );
}

/// FALLBACK branch: nothing deletable ⇒ optional hand pick (Lv.6 [Gallantmon]
/// only) ⇒ digivolves for the digivolution cost REDUCED BY 1.
#[test]
fn bt12_016_may_digivolve_into_gallantmon_with_cost_reduced_1() {
    let mut r = base_runner();
    r.set_first_player(0);
    let war = r.place_on_field(0, CARD_ID, Some(0));
    let _opp = r.place_on_field(1, "OPP-BIG", Some(0)); // not a legal delete target
    // Hand: the real Gallantmon (legal), a non-Gallantmon Lv.6 (illegal), a
    // Lv.4 Gallantmon-named card (illegal).
    push_to_hand(&mut r, 0, "BT17-016");
    push_to_hand(&mut r, 0, "LV6-OTHER");
    push_to_hand(&mut r, 0, "GALL-LV4");
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenDigivolving, war);
    let view = r
        .pending_selection_view()
        .expect("the 'you may digivolve' hand pick must surface");
    assert!(
        view.is_optional,
        "printed 'you may digivolve' ⇒ the pick is declinable"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1, // only the Gallantmon (PASS is implicit for optional selections)
        "only the Lv.6 [Gallantmon] is a legal pick — not the non-Gallantmon \
         Lv.6 nor the Lv.4 Gallantmon-named card; got {:?}",
        view.valid_action_ids
    );
    let pick = view.valid_action_ids[0];
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();

    let perm = &r.game.player(0).battle_area[war.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "BT17-016",
        "WarGrowlmon digivolves into the hand Gallantmon"
    );
    assert_eq!(perm.stack_size(), 2, "BT12-016 stays as a digivolution source");
    // BT17-016's digivolution route from a Lv.5 red is cost 3 ⇒ pays 2.
    assert_eq!(
        r.memory(),
        mem_before - 2,
        "the digivolution cost is reduced by 1 (3 − 1 = 2)"
    );
}

/// The fallback digivolve is DECLINABLE (PASS leaves the board unchanged).
#[test]
fn bt12_016_digivolve_fallback_is_declinable() {
    let mut r = base_runner();
    let war = r.place_on_field(0, CARD_ID, Some(0));
    push_to_hand(&mut r, 0, "BT17-016");
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::WhenDigivolving, war);
    let view = r
        .pending_selection_view()
        .expect("the optional hand pick surfaces (empty opponent board ⇒ nothing deleted)");
    assert!(view.is_optional);
    r.execute_action(0, digimon_engine::action::space::PASS)
        .expect("declining the 'you may digivolve' is legal");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.player(0).battle_area[war.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID,
        "declined ⇒ still WarGrowlmon"
    );
    assert_eq!(r.memory(), mem_before, "declined ⇒ no cost paid");
}

/// NEGATIVE: nothing deleted and no legal [Gallantmon] in hand ⇒ clean no-op.
#[test]
fn bt12_016_no_gallantmon_in_hand_clean_noop() {
    let mut r = base_runner();
    let war = r.place_on_field(0, CARD_ID, Some(0));
    push_to_hand(&mut r, 0, "LV6-OTHER");
    push_to_hand(&mut r, 0, "GALL-LV4");

    fire(&mut r, EffectTiming::WhenDigivolving, war);
    let _ = r.auto_resolve();
    assert!(
        r.game.pending_selection.is_none(),
        "no legal pick ⇒ no stuck selection"
    );
    assert_eq!(
        r.game.player(0).battle_area[war.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        CARD_ID
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — inherited [End of Attack][OPT] gain 2 memory
// ════════════════════════════════════════════════════════════════════════════

/// Build a carrier named [Gallantmon] with BT12-016 as an inherited source.
fn gallant_over_wargrowlmon(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &[CARD_ID, "BT17-016"])
}

/// POSITIVE: no opponent Digimon + [Gallantmon] carrier name ⇒ +2 memory.
#[test]
fn bt12_016_inherited_end_of_attack_gains_2_memory() {
    let mut r = base_runner();
    r.set_first_player(0);
    r.game.set_memory(0);
    let carrier = gallant_over_wargrowlmon(&mut r);
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::EndOfAttack, carrier);
    let _ = r.auto_resolve();
    assert_eq!(
        r.memory(),
        mem_before + 2,
        "no opp Digimon + [Gallantmon] name ⇒ gain 2 memory"
    );
}

/// OPT lockout: a second [End of Attack] in the same turn gains nothing.
#[test]
fn bt12_016_inherited_gain_is_once_per_turn() {
    let mut r = base_runner();
    r.set_first_player(0);
    r.game.set_memory(0);
    let carrier = gallant_over_wargrowlmon(&mut r);
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::EndOfAttack, carrier);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::EndOfAttack, carrier);
    let _ = r.auto_resolve();
    assert_eq!(
        r.memory(),
        mem_before + 2,
        "[Once Per Turn] ⇒ the second End of Attack must not gain again"
    );
}

/// NEGATIVE: an opponent Digimon in play blocks the gain.
#[test]
fn bt12_016_no_gain_when_opponent_has_digimon() {
    let mut r = base_runner();
    r.set_first_player(0);
    r.game.set_memory(0);
    let carrier = gallant_over_wargrowlmon(&mut r);
    let _opp = r.place_on_field(1, "OPP-SMALL", Some(0));
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::EndOfAttack, carrier);
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem_before, "opponent has a Digimon ⇒ no gain");
}

/// NEGATIVE: a carrier without [Growlmon]/[Gallantmon] in its name gains
/// nothing.
#[test]
fn bt12_016_no_gain_without_growlmon_or_gallantmon_name() {
    let mut r = base_runner();
    r.set_first_player(0);
    r.game.set_memory(0);
    // BT12-016 buried under a non-matching Lv.6 name ("Examon").
    let carrier = r.place_stack(0, &[CARD_ID, "LV6-OTHER"]);
    let mem_before = r.memory();

    fire(&mut r, EffectTiming::EndOfAttack, carrier);
    let _ = r.auto_resolve();
    assert_eq!(
        r.memory(),
        mem_before,
        "the carrier's name lacks [Growlmon]/[Gallantmon] ⇒ no gain"
    );
}
