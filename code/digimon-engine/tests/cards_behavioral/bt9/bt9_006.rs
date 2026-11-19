//! BT9-006 Pagumon (Digi-Egg, Lv.2, Purple): inherited
//! [When Attacking] You may trash 1 card in your hand to have this Digimon get
//! +1000 DP for the turn.
//!
//! DCGO BT9/Purple/BT9_006.cs: OnAllyAttack inherited, isOptional TRUE (the
//! always-shown Yes/No), CanActivate needs >= 1 card in hand, then
//! SelectHandEffect(canNoSelect: true, Discard) and +1000 DP (UntilEachTurnEnd)
//! only if a card was discarded.
//!
//! Synthetic boards only; no recorded-game data.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const ID: &str = "BT9-006";

fn host_card() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("HOST", "Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

fn base(hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT9-006 loads from the embedded pack")
        .add_card(host_card())
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

fn attack_trigger(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

#[test]
fn bt9_006_metadata_and_single_inherited_clause() {
    let r = base(&[]);
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Pagumon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert!(d.traits.contains(&"Lesser".to_string()));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}

#[test]
fn bt9_006_accept_trashes_one_card_and_gives_plus_1000_dp() {
    let mut r = base(&["FILLER", "FILLER"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    assert_eq!(r.effective_dp(h), Some(5000));
    attack_trigger(&mut r, h);

    // Optional activation (DCGO Yes/No) first.
    let v = r.pending_selection_view().expect("optional activation prompt");
    assert!(v.is_optional, "'You may' is declinable");
    r.accept_optional_trigger().expect("accept");

    let v = r.pending_selection_view().expect("hand pick");
    assert_eq!(v.kind, SelectionKind::Hand);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(r.hand_size(0), 1, "exactly 1 card trashed from hand");
    assert_eq!(r.trash_size(0), 1);
    assert_eq!(r.effective_dp(h), Some(6000), "+1000 DP for the carrier");
}

#[test]
fn bt9_006_decline_trashes_nothing_and_no_dp() {
    let mut r = base(&["FILLER", "FILLER"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack_trigger(&mut r, h);
    assert!(r.pending_is_optional());
    r.decline_optional_trigger().expect("decline");
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.trash_size(0), 0);
    assert_eq!(r.effective_dp(h), Some(5000));
}

#[test]
fn bt9_006_accept_then_pick_nothing_gives_no_dp() {
    // DCGO SelectHandEffect canNoSelect:true — "if (discarded)" gates the DP.
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack_trigger(&mut r, h);
    r.accept_optional_trigger().expect("accept");
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional, "the hand pick may select nothing");
    r.execute_action(0, digimon_engine::action::space::PASS)
        .unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.effective_dp(h), Some(5000));
}

#[test]
fn bt9_006_empty_hand_offers_nothing() {
    // DCGO CanActivateCondition requires HandCards.Count >= 1.
    let mut r = base(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack_trigger(&mut r, h);
    assert!(
        r.pending_selection().is_none(),
        "no prompt with an empty hand"
    );
    assert_eq!(r.effective_dp(h), Some(5000));
}

#[test]
fn bt9_006_only_the_carrier_under_the_egg_benefits() {
    let mut r = base(&["FILLER"]);
    // Another Digimon's attack never fires it.
    let other = r.place_on_field(0, "HOST", Some(0));
    attack_trigger(&mut r, other);
    assert!(
        r.pending_selection().is_none(),
        "other Digimon's attack does not trigger it"
    );
    let carrier = r.place_stack(0, &[ID, "HOST"]);
    attack_trigger(&mut r, carrier);
    assert!(
        r.pending_selection().is_some(),
        "the carrier's own attack does"
    );
    assert_eq!(r.effective_dp(other), Some(5000));
}

#[test]
fn bt9_006_dp_bonus_expires_at_end_of_turn() {
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack_trigger(&mut r, h);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(h), Some(6000));
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.effective_dp(h), Some(5000), "'for the turn'");
}
