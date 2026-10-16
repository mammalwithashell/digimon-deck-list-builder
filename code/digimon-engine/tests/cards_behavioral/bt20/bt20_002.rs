//! BT20-002 verified official observation 6eaef7e67a6025d49b130da627b2cbef3204b9b289ec95ac6106be9770940cba,
//! printed image, Q4281/Q4282 and DCGO BT20_002.cs.
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;
const ID: &str = "BT20-002";
fn base() -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(include_str!("../../../cards/bt20/BT20-002.yaml"))
        .expect("BT20-002 must exist");
    for (id, name, effect, inherited) in [
        ("NAME", "Dracomon", "", ""),
        ("EFFECT", "Plain", "[Examon] gains DP", ""),
        ("INHERITED", "Plain", "", "[Dracomon] gains DP"),
        ("PLAIN", "Plain", "", ""),
    ] {
        let mut c = make_test_card(id, name);
        c.effect_text = effect.into();
        c.inherited_text = inherited.into();
        b = b.add_card(c);
    }
    let mut r = b
        .deck(0, &["PLAIN"; 20])
        .deck(1, &["PLAIN"; 20])
        .memory(5)
        .start();
    r.skip_mulligan();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_none(),
        "mandatory draw has no optional choice"
    );
}
#[test]
fn bt20_002_printed_metadata() {
    let r = base();
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Bebydomon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert_eq!(d.colors, vec![CardColor::Blue]);
    assert!(d.traits.contains(&"Baby Dragon".into()));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}
#[test]
fn bt20_002_top_name_effect_and_inherited_text_qualify() {
    for top in ["NAME", "EFFECT", "INHERITED"] {
        let mut r = base();
        let h = r.place_stack(0, &[ID, top]);
        let before = r.hand_size(0);
        let deck = r.deck_size(0);
        fire(&mut r, h);
        assert_eq!(r.hand_size(0), before + 1, "{top}");
        assert_eq!(r.deck_size(0), deck - 1);
    }
}
#[test]
fn bt20_002_own_source_text_or_other_source_does_not_qualify_carrier() {
    let mut r = base();
    let h = r.place_stack(0, &[ID, "EFFECT", "PLAIN"]);
    let before = r.hand_size(0);
    fire(&mut r, h);
    assert_eq!(r.hand_size(0), before);
}
#[test]
fn bt20_002_inherited_only_and_only_attacking_carrier() {
    let mut r = base();
    let h = r.place_stack(0, &[ID, "NAME"]);
    let ally = r.place_on_field(0, "EFFECT", Some(0));
    let face = r.place_on_field(0, ID, Some(0));
    let before = r.hand_size(0);
    fire(&mut r, ally);
    fire(&mut r, face);
    assert_eq!(r.hand_size(0), before);
    fire(&mut r, h);
    assert_eq!(r.hand_size(0), before + 1);
}
#[test]
fn bt20_002_once_per_turn_resets_and_is_per_copy() {
    let mut r = base();
    let a = r.place_stack(0, &[ID, "NAME"]);
    let b = r.place_stack(0, &[ID, "EFFECT"]);
    let before = r.hand_size(0);
    fire(&mut r, a);
    fire(&mut r, a);
    fire(&mut r, b);
    assert_eq!(r.hand_size(0), before + 2);
    r.end_turn();
    r.auto_resolve().unwrap();
    let before = r.hand_size(0);
    fire(&mut r, a);
    fire(&mut r, a);
    assert_eq!(r.hand_size(0), before + 1);
}

#[test]
fn bt20_002_legal_attack_mask_reaches_mandatory_draw() {
    use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
    let mut r = base();
    let h = r.place_stack(0, &[ID, "EFFECT"]);
    // Keep security nonempty so the attack does not immediately end the game.
    let card = r.game.players[1].deck.pop().unwrap();
    r.game.players[1].security.push(card);
    let action = encode_attack(h.index as u16, SECURITY_TARGET);
    assert_eq!(
        digimon_engine::action::build_action_mask(&r.game, 0)[action as usize],
        1.0
    );
    let before = r.hand_size(0);
    r.game.decode_action(action, 0);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), before + 1);
    assert!(r.pending_selection().is_none());
}
