//! EX3-008 Flamedramon — Lv.4 Red, DP 5000, Cost 5. Armor Form / Free.
//! Its inherited [End of Your Turn] DNA-digivolve is the judge-quiz Q30
//! driver (pinned end-to-end in `judge_quiz/c_declare_then_pay.rs`).

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex3_008_compiles_with_wd_choice_and_inherited_eot_dna() {
    let r = DebugRunner::builder()
        .dsl_card("EX3-008")
        .expect("EX3-008 YAML parses and compiles")
        .build();
    let card = r.compiled_card("EX3-008").expect("present in pack");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "[When Digivolving] two-effect choice clause compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
                    && t.optional
        )),
        "inherited [End of Your Turn] DNA-digivolve clause compiles"
    );
}

// ─── DNA picks: only a result this Digimon can DNA into, then only partners ─
// DCGO picks the RESULT first: EX3_008.cs:72-94 (WD) and :209-232 (inherited)
// offer a hand card only when `IsDigimon && CanPlayJogress(true) &&
// CanJogressFromTargetPermanent(this, true)` — a printed DNA requirement that
// THIS Digimon and some other own Digimon satisfy, with the DNA cost payable
// (`true` = pay cost). The roots are then chosen with this Digimon fixed
// (`permanentConditions: permanent == this`, :190 / :274;
// DNADigivolveEffects.cs:456-600). CR 4.3 1-3-11-1: a DNA digivolution whose
// cost can't be paid can't be declared; 1-3-2: an impossible one is not
// carried out, so offering its card (or a partner no hand card pairs with) is
// a dead choice.

use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn lv(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(level);
    c.dp = Some(1000 * level as i32);
    c.play_cost = 3;
    c
}

fn dna(id: &str, a: u8, b: u8, cost: i16) -> CardData {
    let mut c = make_test_dna_card(id, id, a, b, cost);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(7);
    c.dp = Some(15000);
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX3-008")
        .expect("EX3-008 YAML parses and compiles")
        .add_card(lv("B3", 3))
        .add_card(lv("C6", 6))
        .add_card(lv("P5", 5))
        .add_card(lv("P4", 4))
        .add_card(dna("R65", 6, 5, 3))
        .add_card(dna("R64", 6, 4, 3))
        .add_card(dna("R54", 5, 4, 3))
        .add_card(dna("R65X", 6, 5, 20))
        .add_card(dna("R45", 4, 5, 3))
        .add_card(dna("R44", 4, 4, 3))
        .add_card(dna("R55", 5, 5, 3))
}

fn hand_offered(r: &DebugRunner) -> Vec<String> {
    let v = r.pending_selection_view().expect("hand pick");
    assert_eq!(v.kind, SelectionKind::Hand, "{v:?}");
    let mut ids: Vec<String> = v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            let slot = if a >= HAND_EFFECT_START { a - HAND_EFFECT_START } else { a - PLAY_HAND_START };
            r.game.players[0].hand.get(slot as usize)
        })
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    ids.sort();
    ids
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let v = r.pending_selection_view().expect("hand pick");
    let slot = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in hand") as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| v.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not offered: {v:?}"));
    r.execute_action(0, a).expect("pick hand card");
}

fn partners_offered(r: &DebugRunner) -> Vec<u16> {
    let v = r.pending_selection_view().expect("partner pick");
    let mut ids: Vec<u16> = v.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    ids.sort();
    ids
}

/// Accept the optional trigger (and any trigger-order prompt) until the first
/// card or permanent pick of the effect is open.
fn accept(r: &mut DebugRunner) {
    for _ in 0..4 {
        let v = r.pending_selection_view().expect("optional trigger");
        if matches!(v.kind, SelectionKind::Hand | SelectionKind::OwnField) || v.effect_choices.is_some() && v.kind != SelectionKind::TriggerOrder {
            return;
        }
        let a = v.valid_action_ids.iter().copied().find(|&a| a != PASS).expect("accept");
        r.execute_action(v.selecting_player, a).expect("accept");
    }
}

fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Field: C6 (Lv.6, Flamedramon under it), P5, P4. Hand: R65 (Lv.6 + Lv.5),
/// R64 (Lv.6 + Lv.4), R54 (Lv.5 + Lv.4: excludes this Digimon), R65X (cost
/// 20: more than memory 3 can pay). End the turn and accept the inherited
/// [End of Your Turn].
fn eot_scene() -> (DebugRunner, [PermanentHandle; 3]) {
    let mut r = builder()
        .deck(0, &["P4"; 10])
        .deck(1, &["P4"; 10])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    for id in ["R65", "R64", "R54", "R65X"] {
        r.add_to_hand(0, id);
    }
    let c6 = r.place_on_field(0, "C6", Some(0));
    r.push_source(c6, "EX3-008");
    let p5 = r.place_on_field(0, "P5", Some(0));
    let p4 = r.place_on_field(0, "P4", Some(0));
    r.end_turn();
    accept(&mut r);
    (r, [c6, p5, p4])
}

#[test]
fn ex3_008_inherited_result_pick_offers_only_cards_this_digimon_can_dna_into() {
    let (r, _) = eot_scene();
    assert_eq!(
        hand_offered(&r),
        vec!["R64".to_string(), "R65".to_string()],
        "R54's pair excludes this Digimon; R65X is unpayable"
    );
}

#[test]
fn ex3_008_inherited_partner_pick_offers_only_partners_for_the_chosen_result() {
    let (mut r, [c6, p5, _p4]) = eot_scene();
    pick_hand(&mut r, "R65");
    assert_eq!(partners_offered(&r), vec![encode_attack(0, p5.index as u16)], "only the Lv.5");
    r.execute_action(0, encode_attack(0, p5.index as u16)).expect("pick P5");
    let _ = r.auto_resolve();
    assert_eq!(top(&r, c6), "R65", "DNA digivolved");
    assert_eq!(r.memory(), 0, "printed DNA cost 3 paid");
}

/// [When Digivolving] DNA branch on a freshly digivolved Flamedramon (Lv.4):
/// field P5, P4. Hand R45 (Lv.4 + Lv.5), R44 (Lv.4 + Lv.4), R55 (Lv.5 +
/// Lv.5: this Lv.4 fits neither side).
#[test]
fn ex3_008_when_digivolving_dna_picks_follow_this_digimon() {
    let mut r = builder()
        .deck(0, &["P4"; 10])
        .deck(1, &["P4"; 10])
        .memory(8)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    for id in ["EX3-008", "R45", "R44", "R55"] {
        r.add_to_hand(0, id);
    }
    let base = r.place_on_field(0, "B3", Some(0));
    let p5 = r.place_on_field(0, "P5", Some(0));
    let _p4 = r.place_on_field(0, "P4", Some(0));
    let slot = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "EX3-008")
        .unwrap() as u16;
    r.game.decode_action(encode_digivolve(slot, base.index as u16), 0);
    accept(&mut r);
    r.execute_branch(1).expect("DNA digivolve branch");
    assert_eq!(hand_offered(&r), vec!["R44".to_string(), "R45".to_string()]);
    pick_hand(&mut r, "R45");
    assert_eq!(partners_offered(&r), vec![encode_attack(0, p5.index as u16)]);
}
