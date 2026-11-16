//! BT20-036 BanchoLeomon — Lv.6 Yellow/Black, DP 12000, Cost 12.
//! Full clause inventory in the YAML header. The Q30 judge-quiz pin
//! (`judge_quiz/c_declare_then_pay.rs`) exercises this card as a Partition
//! source played mid-deletion.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_attack, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn bt20_036_compiles_with_redirect_and_eot_dna_clauses() {
    let r = DebugRunner::builder()
        .dsl_card("BT20-036")
        .expect("BT20-036 YAML parses and compiles")
        .build();
    let card = r.compiled_card("BT20-036").expect("present in pack");
    assert_eq!(card.alt_paths.len(), 2, "Lv5 yellow/4 + Lv5 ACCEL/3");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOpponentAttack)
        )),
        "inherited [Opp Turn][OPT] attack redirect compiles"
    );
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::FaceUp
                    && t.when.contains(&CompiledTiming::EndOfYourTurn)
        )),
        "[End of Your Turn] DNA-into-[Chaosmon] clause compiles"
    );
}

/// [On Play]: De-Digivolve 2 on one opponent Digimon, then −5000 DP on one.
#[test]
fn bt20_036_on_play_de_digivolves_2_then_minus_5000() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-036")
        .expect("BT20-036 loads")
        .add_card({
            let mut c = make_test_card("BL-OPP-T", "Opp Top");
            c.card_kind = CardKind::Digimon;
            c.level = Some(5);
            c.dp = Some(7000);
            c
        })
        .add_card({
            let mut c = make_test_card("BL-OPP-S", "Opp Src");
            c.card_kind = CardKind::Digimon;
            c.level = Some(4);
            c.dp = Some(8000);
            c
        })
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);

    let opp = r.place_stack(1, &["BL-OPP-S", "BL-OPP-S", "BL-OPP-T"]);
    let bancho = r.place_on_field(0, "BT20-036", Some(0));
    r.fire_on_play(0, bancho.index as usize);

    // De-Digivolve target pick, then the −5000 target pick.
    // §16-12-1: the controller declares how many cards <De-Digivolve> trashes.
    r.auto_resolve_declaring_max()
        .expect("both opponent picks resolve, declaring 2");

    assert_eq!(
        r.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "De-Digivolve 2 trashed 2 cards off the stack"
    );
    // The De-Digivolve exposed the bottom BL-OPP-S (base 8000); the second
    // pick applied -5000 until the end of their turn → 3000.
    assert_eq!(
        r.dp_of(opp),
        Some(3000),
        "the -5000 DP modifier applies to the surviving Digimon"
    );
}

// ─── [End of Your Turn] DNA candidates ──────────────────────────────────────
// DCGO BT20_036.cs:220-238 `CanSelectCardCondition`: IsDigimon &&
// ContainsCardName("Chaosmon") && CanPlayJogress(true) &&
// CanJogressFromTargetPermanent(card.PermanentOfThisCard(), true) — the hand
// pick lists only a [Chaosmon] whose printed DNA requirement THIS BanchoLeomon
// can fill. :269-275 `DNADigivolvePermanentsIntoHandOrTrashCard(..., payCost:
// true, isHand: true, permanentConditions: [this permanent])` picks the hand
// card first, then the roots (one of them this Digimon), and pays the printed
// DNA cost (the text has no "without paying the cost"). :278-289 the attack
// runs only on DNA success. CR 4.3 1-3-11-1: a card can't be declared for use
// when its cost can't be paid; 1-3-2: an impossible action isn't carried out.

/// A Lv.`level` Digimon material.
fn material(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(9000);
    c
}

/// A [Chaosmon]-named Lv.7 DNA result with a level-only recipe.
fn chaosmon(id: &str, a: u8, b: u8, cost: i16) -> CardData {
    let mut c = make_test_dna_card(id, &format!("Chaosmon {id}"), a, b, cost);
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(15000);
    c
}

/// BanchoLeomon (Lv.6) plus `others` on P0's field, `hand` in P0's hand, P0's
/// main phase at 10 memory. Returns (BanchoLeomon, other handles).
fn eot_runner(others: &[&str], hand: &[&str]) -> (DebugRunner, PermanentHandle, Vec<PermanentHandle>) {
    let mut r = DebugRunner::builder()
        .dsl_card("BT20-036")
        .expect("BT20-036 loads")
        .add_card(material("M6", 6))
        .add_card(material("M5", 5))
        // Lv.6 + Lv.6 at cost 2: BanchoLeomon + M6 fill it.
        .add_card(chaosmon("C-OK", 6, 6, 2))
        // Lv.5 + Lv.5: BanchoLeomon (Lv.6) can never be a material.
        .add_card(chaosmon("C-FAR", 5, 5, 2))
        // Lv.6 + Lv.6 at cost 25: 10 - 25 = -15 is past the -10 floor.
        .add_card(chaosmon("C-PRICY", 6, 6, 25))
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["M5"; 10])
        .deck(1, &["M5"; 10])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    for id in hand {
        r.add_to_hand(0, id);
    }
    let bancho = r.place_on_field(0, "BT20-036", None);
    let handles = others.iter().map(|id| r.place_on_field(0, id, None)).collect();
    (r, bancho, handles)
}

fn fire_eot(r: &mut DebugRunner, bancho: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(bancho));
    r.game.drain_effect_queue();
    // An outer "may" prompt, if the clause installs one, is accepted.
    while r.pending_kind() == Some(SelectionKind::Replacement) {
        r.accept_optional_trigger().unwrap();
    }
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

/// The non-PASS candidates of the pending hand pick, as card ids.
fn offered_hand(r: &DebugRunner) -> Vec<String> {
    let hand = hand_ids(r);
    let mut v: Vec<String> = r
        .pending_selection_view()
        .expect("a hand pick is pending")
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| hand[(id - PLAY_HAND_START) as usize].clone())
        .collect();
    v.sort();
    v
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let index = hand_ids(r).iter().position(|h| h == id).expect("card in hand");
    r.execute_action(0, PLAY_HAND_START + index as u16).unwrap();
}

/// Drive the EoT clause: whichever of the two picks is pending, answer it
/// with `result` / `partner`, until neither is pending.
fn drive(r: &mut DebugRunner, result: &str, partner: PermanentHandle) {
    for _ in 0..6 {
        match r.pending_kind() {
            Some(SelectionKind::Replacement) => r.accept_optional_trigger().unwrap(),
            Some(SelectionKind::Hand) => pick_hand(r, result),
            Some(SelectionKind::OwnField) => r
                .execute_action(0, encode_attack(0, partner.index as u16))
                .unwrap(),
            _ => return,
        }
    }
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[0].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

/// Only a [Chaosmon] BanchoLeomon can be a material of, at a payable DNA cost,
/// is offered: C-FAR (Lv.5 + Lv.5) and C-PRICY (cost 25) are not.
#[test]
fn bt20_036_eot_result_offers_only_chaosmon_this_digimon_can_pair_into_and_afford() {
    let (mut r, _bancho, others) = eot_runner(&["M6"], &["C-OK", "C-FAR", "C-PRICY"]);
    let m6 = others[0];
    fire_eot(&mut r, _bancho);
    // Answer a partner-first pick (old shape) so the hand pick is reached.
    if r.pending_kind() == Some(SelectionKind::OwnField) {
        r.execute_action(0, encode_attack(0, m6.index as u16)).unwrap();
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert_eq!(offered_hand(&r), vec!["C-OK".to_string()]);
}

/// The other material must complete the chosen card's recipe with this
/// Digimon: the Lv.5 is not offered for the Lv.6 + Lv.6 C-OK.
#[test]
fn bt20_036_eot_partner_pick_offers_only_digimon_completing_the_recipe() {
    let (mut r, bancho, others) = eot_runner(&["M6", "M5"], &["C-OK"]);
    let (m6, _m5) = (others[0], others[1]);
    fire_eot(&mut r, bancho);
    if r.pending_kind() == Some(SelectionKind::Hand) {
        pick_hand(&mut r, "C-OK");
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::OwnField));
    let offered: Vec<u16> = r
        .pending_selection_view()
        .unwrap()
        .valid_action_ids
        .into_iter()
        .filter(|&id| id != PASS)
        .collect();
    assert_eq!(offered, vec![encode_attack(0, m6.index as u16)], "only M6 (Lv.6)");
}

/// "may DNA digivolve" with no "without paying the cost": the printed DNA
/// cost (2) is paid (DCGO `payCost: true`).
#[test]
fn bt20_036_eot_dna_pays_the_printed_dna_cost() {
    let (mut r, bancho, others) = eot_runner(&["M6"], &["C-OK"]);
    fire_eot(&mut r, bancho);
    drive(&mut r, "C-OK", others[0]);
    let tops: Vec<String> = (0..r.battle_area_size(0))
        .map(|i| top_id(&r, PermanentHandle { player: 0, index: i as u8 }))
        .collect();
    assert!(tops.contains(&"C-OK".to_string()), "DNA digivolved: {tops:?}");
    assert_eq!(r.memory(), 8, "printed DNA cost 2 paid");
}

/// A successful DNA grants the DNA-digivolved Digimon its attack ("Then, the
/// DNA digivolved Digimon may attack"; DCGO :278-289 `OnDNASuccess` attacks
/// with `cardSource.PermanentOfThisCard()`). The grant is the windowed
/// `MayAttack` (the end-of-turn attack idiom, as AD1-004).
#[test]
fn bt20_036_eot_successful_dna_grants_the_merged_digimon_its_attack() {
    use digimon_engine::enums::ModifierType;
    let (mut r, bancho, others) = eot_runner(&["M6"], &["C-OK"]);
    fire_eot(&mut r, bancho);
    drive(&mut r, "C-OK", others[0]);
    let merged = (0..r.battle_area_size(0))
        .map(|i| PermanentHandle { player: 0, index: i as u8 })
        .find(|&h| top_id(&r, h) == "C-OK")
        .expect("the DNA digivolved Digimon is on the field");
    assert!(
        r.game.modifiers.has(merged, ModifierType::MayAttack),
        "the DNA-digivolved Digimon may attack"
    );
}

/// Declining the DNA grants no attack: "the DNA digivolved Digimon may
/// attack" has no subject when nothing DNA digivolved (DCGO `OnDNASuccess`).
#[test]
fn bt20_036_eot_declined_dna_grants_no_attack() {
    let (mut r, bancho, _others) = eot_runner(&["M6"], &["C-OK"]);
    fire_eot(&mut r, bancho);
    assert!(r.pending_selection().is_some(), "the DNA pick is offered");
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.pending_selection().is_none(),
        "nothing more is offered after the decline: {:?}",
        r.pending_selection_view().map(|v| (v.kind, v.prompt))
    );
    assert_eq!(top_id(&r, bancho), "BT20-036");
    assert!(!r.game.players[0].battle_area[bancho.index as usize].is_suspended);
}

/// No [Chaosmon] in hand that this Digimon can DNA into: nothing is offered
/// (DCGO `CanActivateCondition` needs `HasMatchConditionOwnersHand`).
#[test]
fn bt20_036_eot_without_a_feasible_chaosmon_offers_nothing() {
    let (mut r, bancho, _others) = eot_runner(&["M6"], &["C-FAR"]);
    fire_eot(&mut r, bancho);
    assert!(
        r.pending_selection().is_none(),
        "no DNA pick: {:?}",
        r.pending_selection_view().map(|v| (v.kind, v.prompt))
    );
}
