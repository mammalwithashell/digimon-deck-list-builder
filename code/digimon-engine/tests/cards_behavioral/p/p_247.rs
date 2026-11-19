//! P-247 Nyaromon (Digi-Egg, Lv.2, Purple): inherited
//! [When Attacking] [Once Per Turn] By trashing 1 [Dark Animal], [Shaman],
//! [Undead] or [TS] trait card from your hand, delete 1 of your opponent's
//! unsuspended level 4 or lower Digimon.
//!
//! DCGO P/Purple/P_247.cs: OnAllyAttack inherited, maxCount 1, mandatory
//! activation (isOptional false); the hand pick is declinable
//! (canNoSelect: true) and declining refunds the once-per-turn use
//! (`RemoveUse`); the delete target pick is mandatory.
//!
//! Synthetic boards only; no recorded-game data.
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "P-247";

fn mon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base(hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("P-247 loads from the embedded pack")
        .add_card(mon("HOST", 4, &[]))
        .add_card(mon("H-DARK", 3, &["Dark Animal"]))
        .add_card(mon("H-SHAMAN", 3, &["Shaman"]))
        .add_card(mon("H-UNDEAD", 3, &["Undead"]))
        .add_card(mon("H-TS", 3, &["TS"]))
        .add_card(mon("H-OTHER", 3, &["Dragon"]))
        .add_card(mon("OPP-4", 4, &[]))
        .add_card(mon("OPP-5", 5, &[]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .hand(0, hand)
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

fn attack(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn on_field(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .any(|x| x.top_card().card_id(&r.game.card_data) == id)
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn pick(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("a prompt");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
}

#[test]
fn p_247_metadata_and_single_inherited_clause() {
    let r = base(&[]);
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Nyaromon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}

#[test]
fn p_247_trashes_matching_trait_card_and_deletes_unsuspended_lv4() {
    for trait_card in ["H-DARK", "H-SHAMAN", "H-UNDEAD", "H-TS"] {
        let mut r = base(&[trait_card]);
        let h = r.place_stack(0, &[ID, "HOST"]);
        r.place_on_field(1, "OPP-4", Some(0));
        attack(&mut r, h);
        let v = r.pending_selection_view().expect("hand pick");
        assert!(v.is_optional, "the 'by trashing' pick is declinable");
        pick(&mut r);
        pick(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.hand_size(0), 0, "{trait_card} trashed");
        assert_eq!(r.trash_size(0), 1);
        assert!(!on_field(&r, 1, "OPP-4"), "{trait_card}: Lv.4 deleted");
    }
}

#[test]
fn p_247_target_must_be_unsuspended_and_level_4_or_lower() {
    let mut r = base(&["H-DARK"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    let lv5 = r.place_on_field(1, "OPP-5", Some(0));
    let sus = r.place_on_field(1, "OPP-4", Some(0));
    suspend(&mut r, sus);
    let ok = r.place_on_field(1, "OPP-4", Some(0));
    let _ = (lv5, ok);
    attack(&mut r, h);
    pick(&mut r); // hand card
    let v = r.pending_selection_view().expect("target pick");
    assert_eq!(
        v.valid_action_ids.len(),
        1,
        "only the unsuspended Lv.4 is a legal target"
    );
    assert!(!v.is_optional, "the delete is mandatory");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert!(on_field(&r, 1, "OPP-5"));
    assert_eq!(
        r.game.players[1].battle_area.len(),
        2,
        "exactly one Digimon deleted"
    );
    assert!(r.game.players[1].battle_area.iter().any(|p| p.is_suspended));
}

#[test]
fn p_247_non_matching_hand_offers_nothing() {
    let mut r = base(&["H-OTHER"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, h);
    assert!(r.pending_selection().is_none(), "no eligible hand card");
    assert!(on_field(&r, 1, "OPP-4"));
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn p_247_only_matching_cards_are_offered_for_the_trash() {
    let mut r = base(&["H-OTHER", "H-DARK"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, h);
    let v = r.pending_selection_view().expect("hand pick");
    let picks: Vec<_> = v.valid_action_ids.iter().filter(|a| **a != PASS).collect();
    assert_eq!(picks.len(), 1, "only the Dark Animal card is selectable");
}

#[test]
fn p_247_decline_refunds_the_once_per_turn_use() {
    let mut r = base(&["H-DARK"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, h);
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert!(on_field(&r, 1, "OPP-4"));
    // DCGO RemoveUse: the declined use is refunded, so it can fire again.
    attack(&mut r, h);
    assert!(
        r.pending_selection().is_some(),
        "declining must not spend the once-per-turn use"
    );
}

#[test]
fn p_247_once_per_turn_after_resolving() {
    let mut r = base(&["H-DARK", "H-TS"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.place_on_field(1, "OPP-4", Some(0));
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, h);
    pick(&mut r);
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    attack(&mut r, h);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]");
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn p_247_discard_with_no_legal_target_still_trashes_nothing_else() {
    // DCGO: the delete only runs if a legal target exists; the discard has
    // already happened by then.
    let mut r = base(&["H-DARK"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    r.place_on_field(1, "OPP-5", Some(0));
    attack(&mut r, h);
    pick(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.pending_selection().is_none());
    assert!(on_field(&r, 1, "OPP-5"));
}

#[test]
fn p_247_only_the_carrier_attack_triggers_it() {
    let mut r = base(&["H-DARK"]);
    let other = r.place_on_field(0, "HOST", Some(0));
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, other);
    assert!(r.pending_selection().is_none());
}
