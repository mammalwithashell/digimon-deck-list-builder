//! EX11-001 Koromon (Digi-Egg, Lv.2, Red): inherited
//! [When Attacking] [Once Per Turn] This Digimon may digivolve into a Digimon
//! card with [Tyrannomon] in its name or the [Dinosaur] trait in the hand.
//!
//! DCGO EX11/Red/EX11_001.cs: OnAllyAttack inherited, maxCount 1,
//! isOptional TRUE; CanActivate = hand >= 1; the digivolve pick is offered only
//! over Digimon cards (trait EQUALS Dinosaur or name contains Tyrannomon) that
//! can legally digivolve onto this Digimon with the printed cost paid.
//!
//! Synthetic boards only; no recorded-game data.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "EX11-001";

fn rookie(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    // Digivolves from a red Lv.2 for 2 memory (explicit: the harness leaves
    // evo_costs empty, which would make every card undigivolvable).
    c.evo_costs = vec![digimon_engine::card_data::EvoCost {
        level: 2,
        memory_cost: 2,
        card_color: 0,
    }];
    c
}

fn builder(hand: &[&str]) -> DebugRunner {
    let mut host = rookie("HOST", "Host", &[]);
    host.level = Some(2);
    host.evo_costs.clear();
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX11-001 loads from the embedded pack")
        .add_card(host)
        .add_card(rookie("DINO", "Greymon", &["Dinosaur"]))
        .add_card(rookie("TYRANNO", "Tyrannomon", &["Reptile"]))
        .add_card(rookie("OTHER", "Garurumon", &["Beast"]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .hand(0, hand)
        .memory(10)
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

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

#[test]
fn ex11_001_metadata() {
    let r = builder(&[]);
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Koromon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}

#[test]
fn ex11_001_digivolves_into_dinosaur_or_tyrannomon_named_card() {
    for want in ["DINO", "TYRANNO"] {
        let mut r = builder(&["DINO", "TYRANNO", "OTHER"]);
        let h = r.place_stack(0, &[ID, "HOST"]);
        let mem = r.memory();
        attack(&mut r, h);
        r.accept_optional_trigger().unwrap();
        let v = r.pending_selection_view().expect("digivolve pick");
        let picks: Vec<_> = v
            .valid_action_ids
            .iter()
            .copied()
            .filter(|a| *a != digimon_engine::action::space::PASS)
            .collect();
        assert_eq!(picks.len(), 2, "Garurumon (neither trait nor name) is not offered");
        // The two offered picks are DINO and TYRANNO in hand order.
        let idx = if want == "DINO" { 0 } else { 1 };
        r.execute_action(0, picks[idx]).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(top_id(&r, h), want, "digivolved into {want}");
        assert!(r.memory() < mem, "the printed digivolution cost is paid");
    }
}

#[test]
fn ex11_001_decline_digivolves_nothing() {
    let mut r = builder(&["DINO"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack(&mut r, h);
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(top_id(&r, h), "HOST");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn ex11_001_empty_hand_offers_nothing() {
    let mut r = builder(&[]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack(&mut r, h);
    assert!(r.pending_selection().is_none());
}

#[test]
fn ex11_001_once_per_turn() {
    let mut r = builder(&["DINO", "DINO"]);
    let h = r.place_stack(0, &[ID, "HOST"]);
    attack(&mut r, h);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(top_id(&r, h), "DINO");
    attack(&mut r, h);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]");
}
