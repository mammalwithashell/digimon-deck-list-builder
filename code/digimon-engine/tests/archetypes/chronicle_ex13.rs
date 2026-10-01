//! EX13 "Chronicle" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/chronicle-ex13-model.md`. The Dorimon (EX13-006) →
//! Dorumon (EX13-049) → Raptordramon (EX13-055) → Grademon (EX13-057) line,
//! with Kota Domoto (EX13-072) and the out-of-slice payoff EX13-060 Alphamon.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{06,49,55,57,72}.rs`;
//! this file asserts only cross-card SYSTEM facts (combos C1..C8 of the model).
//!
//! Real DSL cards fill every role except one: C8 needs a [Chronicle] trait
//! Option, and no real one is implemented (BT9-109 [X Antibody], BT20-095 and
//! P-204 have no YAML), so a synthetic effectless Option stands in there.
//! Fillers / opponents are vanilla starter Digimon: ST1-02 Biyomon (Lv.3,
//! 3000, [Bird]), ST1-04 Dracomon (Lv.3, 4000, [Dragon]), ST1-05 Birdramon
//! (Lv.4, 5000, [Giant Bird]) — none matches any [X Antibody]/[Chronicle] filter.
//!
//! No DCGO C# exists for the five slice cards; printed text (official Bandai
//! DB bundles) governs, with DCGO BT20/Black/BT20_053.cs as the sibling for the
//! Grademon "if during an attack" rider.

#![allow(dead_code)]

use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, BREEDING_TARGET, HAND_EFFECT_START, PASS, PLAY_HAND_START,
    REPLACEMENT_ACCEPT, SEL_REVEAL_START, TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

use super::support::dsl_builder;

// ─── Card ids ────────────────────────────────────────────────────────────────

const DORIMON: &str = "EX13-006";
const DORUMON: &str = "EX13-049";
const RAPTORDRAMON: &str = "EX13-055";
const GRADEMON: &str = "EX13-057";
const KOTA: &str = "EX13-072";
const ALPHAMON: &str = "EX13-060";
/// Vanilla filler (deck / security), Lv.3 3000 [Bird].
const BIYOMON: &str = "ST1-02";
/// Vanilla non-[Chronicle] ally, Lv.3 4000 [Dragon].
const DRACOMON: &str = "ST1-04";
/// Vanilla opponent, Lv.4 5000 [Giant Bird].
const BIRDRAMON: &str = "ST1-05";

const REAL: &[&str] = &[
    DORIMON, DORUMON, RAPTORDRAMON, GRADEMON, KOTA, ALPHAMON, BIYOMON, DRACOMON, BIRDRAMON,
];

/// Synthetic [Chronicle] trait Option (C8 only). Role: "any Option card with
/// the [Chronicle] trait" for Kota's attack clause — no real implemented card
/// fits (BT9-109 / BT20-095 / P-204 unimplemented); effectless so it cannot
/// perturb the asserted outcome.
const CHRON_OPT: &str = "SYN-CHRON-OPT";

fn chron_option() -> CardData {
    let mut c = make_test_card(CHRON_OPT, "Chronicle Option");
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![CardColor::Black];
    c.play_cost = 3;
    c.traits = vec!["Chronicle".to_string()];
    c
}

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL)
        .deck(0, &[BIYOMON; 8])
        .deck(1, &[BIYOMON; 8])
        .security(0, &[BIYOMON; 3])
        .security(1, &[BIYOMON; 3])
}

fn start(b: DebugRunnerBuilder) -> DebugRunner {
    let mut r = b.start();
    r.skip_mulligan();
    r
}

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards.iter().map(|c| c.card_id(&r.game.card_data).to_string()).collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn trash_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].trash)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    ids(r, &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources)
}

fn find_perm(r: &DebugRunner, p: u8, id: &str) -> Option<PermanentHandle> {
    field_ids(r, p)
        .iter()
        .position(|f| f == id)
        .map(|i| PermanentHandle { player: p, index: i as u8 })
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = v;
}

/// Push `ids` on top of `p`'s deck (the LAST id ends on top).
fn stack_deck_top(r: &mut DebugRunner, p: u8, top: &[&str]) {
    for id in top {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("{id} registered"));
        let ci = r.game.next_card_index();
        r.game.players[p as usize]
            .deck
            .push(digimon_engine::card_source::CardSource::new(data_idx, p, ci));
    }
}

fn hand_action(r: &DebugRunner, view: &PendingSelectionView, p: u8, id: &str) -> Option<u16> {
    let slot = hand_ids(r, p).iter().position(|h| h == id)? as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn trash_action(r: &DebugRunner, view: &PendingSelectionView, p: u8, id: &str) -> Option<u16> {
    let i = trash_ids(r, p).iter().position(|t| t == id)? as u16;
    let a = TRASH_EFFECT_START + i;
    view.valid_action_ids.contains(&a).then_some(a)
}

fn field_action(view: &PendingSelectionView, h: PermanentHandle) -> Option<u16> {
    view.valid_action_ids.iter().copied().filter(|&a| a != PASS).find(|&a| {
        a.checked_sub(ATTACK_START)
            .is_some_and(|o| (o % TARGETS_PER_ATTACKER) as u8 == h.index)
    })
}

/// Pick the route whose label names `cost` at a digivolve-route EffectChoice.
fn route_action(view: &PendingSelectionView, cost: u16) -> Option<u16> {
    if view.kind != SelectionKind::EffectChoice {
        return None;
    }
    view.effect_choices
        .as_ref()?
        .iter()
        .find(|c| c.label.contains(&format!("{cost}")))
        .map(|c| c.action_id)
}

/// Drive pending prompts with `policy`; a `None` from the policy falls back to
/// PASS on optional prompts / the first legal action on mandatory ones
/// (TriggerOrder, top-or-bottom, ordering). Stops when nothing is pending.
fn drive(
    r: &mut DebugRunner,
    mut policy: impl FnMut(&DebugRunner, &PendingSelectionView) -> Option<u16>,
) {
    for _ in 0..64 {
        let Some(view) = r.pending_selection_view() else { return };
        let a = policy(r, &view).unwrap_or_else(|| {
            if view.is_optional || view.valid_action_ids.contains(&PASS) {
                PASS
            } else {
                *view.valid_action_ids.first().expect("legal action")
            }
        });
        r.execute_action(view.selecting_player, a)
            .unwrap_or_else(|e| panic!("action {a} on {view:?}: {e:?}"));
    }
    panic!("drive did not converge");
}

fn immune_to_opp_digimon(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.permanent_is_unaffected_by_effect(h, 1 - h.player, EffectSourceKind::Digimon)
}

// ─── C1 Breeding hatch-and-search ────────────────────────────────────────────

/// **C1 Breeding hatch-and-search** — EX13-006 Dorimon + EX13-049 Dorumon
/// (+ EX13-055 as the hit). Dorumon digivolves onto Dorimon IN THE BREEDING
/// AREA for 0 (printed "[Dorimon]: Cost 0"); moving it to the battle area fires
/// its [When Moving] reveal-3, which adds Raptordramon (both [X Antibody] and
/// [Chronicle]) and returns the vanilla rest to the deck.
#[test]
fn c1_dorimon_hatches_into_dorumon_for_free_and_moving_it_searches_raptordramon() {
    let mut r = start(builder().hand(0, &[DORUMON]).memory(3));
    r.place_in_breeding(0, DORIMON);
    // Deck top after the digivolve draw: BIYOMON, BIYOMON, RAPTORDRAMON.
    stack_deck_top(&mut r, 0, &[RAPTORDRAMON, BIYOMON, BIYOMON, BIYOMON]);

    r.game.decode_action(encode_digivolve(0, BREEDING_TARGET), 0);
    drive(&mut r, |_, v| route_action(v, 0));
    let breeding = r.game.players[0].breeding_area.as_ref().expect("stack in breeding");
    assert_eq!(
        breeding.top_card().card_id(&r.game.card_data),
        DORUMON,
        "Dorumon digivolved onto Dorimon in the breeding area"
    );
    assert_eq!(r.memory(), 3, "[Dorimon] route: Cost 0");
    let deck_before = r.deck_size(0);

    assert!(r.move_from_breeding(0));
    let view = r.pending_selection_view().expect("[When Moving] reveal fired");
    assert_eq!(view.kind, SelectionKind::Reveal);
    let pos = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == RAPTORDRAMON)
        .expect("Raptordramon revealed") as u16;
    let biyo = r
        .game
        .revealed_cards
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == BIYOMON)
        .expect("Biyomon revealed") as u16;
    assert!(
        !view.valid_action_ids.contains(&(SEL_REVEAL_START + biyo)),
        "vanilla [Bird] card is not an [X Antibody]/[Chronicle] pick"
    );
    r.execute_action(0, SEL_REVEAL_START + pos).expect("pick Raptordramon");
    drive(&mut r, |_, _| None); // top-or-bottom + ordering

    assert!(hand_ids(&r, 0).contains(&RAPTORDRAMON.to_string()), "next step of the line tutored");
    assert_eq!(r.deck_size(0), deck_before - 1, "only the pick left the deck");
    let dor = find_perm(&r, 0, DORUMON).expect("Dorumon in the battle area");
    assert_eq!(source_ids(&r, dor), vec![DORIMON, DORUMON]);
}

// ─── C2 Stacked DP shred ─────────────────────────────────────────────────────

/// **C2 Stacked DP shred** — EX13-049 Dorumon (inherited) + EX13-055
/// Raptordramon. Raptordramon digivolves onto Dorumon via "[Dorumon]: Cost 2";
/// its [When Digivolving] -3000 takes Birdramon 5000 → 2000; attacking then
/// fires Dorumon's inherited [When Attacking] -2000 → 0 DP → deleted.
#[test]
fn c2_raptordramon_minus_3000_plus_dorumon_inherited_minus_2000_deletes_a_5000_digimon() {
    let mut r = start(builder().hand(0, &[RAPTORDRAMON]).memory(5));
    let base = r.place_stack(0, &[DORIMON, DORUMON]);
    let bird = r.place_on_field(1, BIRDRAMON, Some(0));

    r.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    drive(&mut r, |_, v| route_action(v, 2).or_else(|| field_action(v, bird)));
    assert_eq!(top_id(&r, base), RAPTORDRAMON);
    assert_eq!(r.memory(), 3, "[Dorumon] route: Cost 2");
    assert_eq!(r.effective_dp(bird), Some(2000), "[When Digivolving] -3000");

    r.attack_player(base, 1, false);
    drive(&mut r, |r, v| {
        if v.kind == SelectionKind::OppField {
            find_perm(r, 1, BIRDRAMON).and_then(|h| field_action(v, h))
        } else {
            None
        }
    });
    assert!(find_perm(&r, 1, BIRDRAMON).is_none(), "-3000 then -2000 → 0 DP → deleted");
    assert!(trash_ids(&r, 1).contains(&BIRDRAMON.to_string()));
}

// ─── C3 Mid-attack climb into Grademon from trash ────────────────────────────

/// **C3 Mid-attack climb** — EX13-049 (source) + EX13-055 + EX13-057.
/// Raptordramon's [When Attacking] digivolves into Grademon FROM THE TRASH,
/// paying the [Raptordramon] route (3). Grademon's [When Digivolving] then
/// resolves during the attack: the chosen Digimon (Grademon itself) gains
/// <Reboot> + <Blocker>, immunity to opponent Digimon effects and +5000
/// (7000 → 12000) — DCGO BT20_053.cs `IsAttacking` gate. The stack keeps
/// Raptordramon's inherited <Barrier>, and Dorumon's inherited -2000 still
/// resolves from the same attack.
#[test]
fn c3_raptordramon_attack_climbs_into_grademon_from_trash_with_the_attack_rider() {
    let mut r = start(builder().memory(6));
    r.inject_trash(0, GRADEMON);
    let atk = r.place_stack(0, &[DORUMON, RAPTORDRAMON]);
    let bird = r.place_on_field(1, BIRDRAMON, Some(0));

    r.attack_player(atk, 1, false);
    let mut saw_union = false;
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::UnionZone { .. } => {
            saw_union = true;
            trash_action(r, v, 0, GRADEMON)
        }
        SelectionKind::OwnField => field_action(v, atk),
        SelectionKind::OppField => field_action(v, bird),
        _ => route_action(v, 3),
    });
    assert!(saw_union, "Raptordramon's [When Attacking] digivolve was offered");
    assert_eq!(top_id(&r, atk), GRADEMON, "digivolved from the trash");
    assert!(!trash_ids(&r, 0).contains(&GRADEMON.to_string()));
    assert_eq!(r.memory(), 3, "6 - 3 ([Raptordramon] route)");
    assert_eq!(source_ids(&r, atk), vec![DORUMON, RAPTORDRAMON, GRADEMON]);

    assert_eq!(r.effective_dp(atk), Some(12000), "7000 + 5000 (during an attack)");
    assert!(r.game.has_keyword(atk, Keyword::Reboot));
    assert!(r.game.has_keyword(atk, Keyword::Blocker));
    assert!(immune_to_opp_digimon(&r, atk), "isn't affected by their Digimon effects");
    assert!(r.game.has_keyword(atk, Keyword::Barrier), "inherited <Barrier> from Raptordramon");
    assert_eq!(r.effective_dp(bird), Some(3000), "Dorumon's inherited -2000 resolved too");
    assert_eq!(r.security_count(1), 2, "the attack still checked a security card");
}

/// C3 unhappy path: with no [Chronicle] card in hand or trash the climb is not
/// offered, the rider never applies, and Raptordramon attacks at its own 5000.
#[test]
fn c3_without_a_chronicle_card_raptordramon_does_not_climb() {
    let mut r = start(builder().hand(0, &[BIRDRAMON]).memory(6));
    let atk = r.place_stack(0, &[DORUMON, RAPTORDRAMON]);
    r.place_on_field(1, BIRDRAMON, Some(0));
    r.attack_player(atk, 1, false);
    let mut saw_union = false;
    drive(&mut r, |_, v| {
        if matches!(v.kind, SelectionKind::UnionZone { .. }) {
            saw_union = true;
        }
        None
    });
    assert!(!saw_union, "nothing to digivolve into → no prompt");
    assert_eq!(top_id(&r, atk), RAPTORDRAMON);
    assert_eq!(r.memory(), 6);
    assert_eq!(r.effective_dp(atk), Some(5000));
}

// ─── C4 Kota discard fuels the trash climb ───────────────────────────────────

/// **C4 Kota discard fuels the trash climb** — EX13-072 + EX13-055 + EX13-057.
/// Kota's [Start of Your Main Phase] trashes Grademon (cost) → Draw 1 + 1
/// memory; Raptordramon's [When Attacking] then climbs into that same Grademon
/// from the trash. Kota's attack clause stays silent (no eligible Option).
#[test]
fn c4_kota_discarded_grademon_is_reclaimed_by_raptordramon_from_the_trash() {
    let mut r = start(builder().hand(0, &[GRADEMON]).memory(3));
    let kota = r.place_on_field(0, KOTA, Some(0));
    let rap = r.place_on_field(0, RAPTORDRAMON, Some(0));

    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(kota));
    r.game.drain_effect_queue();
    drive(&mut r, |r, v| {
        if v.kind == SelectionKind::Hand {
            hand_action(r, v, 0, GRADEMON)
        } else if v.is_optional && v.kind == SelectionKind::EffectChoice {
            v.valid_action_ids.iter().copied().find(|&a| a != PASS)
        } else {
            None
        }
    });
    assert!(trash_ids(&r, 0).contains(&GRADEMON.to_string()), "Grademon paid Kota's cost");
    assert_eq!(hand_ids(&r, 0).len(), 1, "Draw 1");
    assert_eq!(r.memory(), 4, "gain 1 memory");

    r.attack_player(rap, 1, false);
    let mut saw_kota_offer = false;
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::UnionZone { .. } => trash_action(r, v, 0, GRADEMON),
        SelectionKind::OwnField => field_action(v, rap),
        _ => {
            if v.source_card == r.game.players[0].battle_area[kota.index as usize].top_card().handle() {
                saw_kota_offer = true;
            }
            route_action(v, 3)
        }
    });
    assert!(!saw_kota_offer, "Kota's attack clause needs an eligible Option in hand");
    assert!(!suspended(&r, kota));
    assert_eq!(top_id(&r, rap), GRADEMON, "the discarded Grademon came back from the trash");
    assert_eq!(r.memory(), 1, "4 - 3 ([Raptordramon] route)");
    assert_eq!(r.effective_dp(rap), Some(12000), "rider applied during the attack");
}

// ─── C5 Grademon End-of-Attack climb into Alphamon ───────────────────────────

/// **C5 End-of-Attack climb into Alphamon** — EX13-057 + EX13-060. Grademon's
/// [End of Attack][OPT] digivolves into Alphamon from hand via the [Grademon]
/// route (4, cheaper than the 5 circle); Alphamon's [When Digivolving] -8000
/// deletes Birdramon (5000).
#[test]
fn c5_grademon_end_of_attack_climbs_into_alphamon_whose_wd_deletes_a_digimon() {
    let mut r = start(builder().hand(0, &[ALPHAMON]).memory(6));
    let grade = r.place_on_field(0, GRADEMON, Some(0));
    let bird = r.place_on_field(1, BIRDRAMON, Some(0));

    r.attack_player(grade, 1, false);
    let mut saw_union = false;
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::UnionZone { .. } => {
            saw_union = true;
            hand_action(r, v, 0, ALPHAMON)
        }
        SelectionKind::OppField => field_action(v, bird),
        _ => route_action(v, 4),
    });
    assert!(saw_union, "[End of Attack] digivolve offered");
    assert_eq!(top_id(&r, grade), ALPHAMON);
    assert_eq!(source_ids(&r, grade), vec![GRADEMON, ALPHAMON]);
    assert_eq!(r.memory(), 2, "6 - 4 ([Grademon] route)");
    assert!(find_perm(&r, 1, BIRDRAMON).is_none(), "Alphamon [WD] -8000 deleted Birdramon");
    assert_eq!(r.security_count(1), 2, "the attack checked one security card first");
}

// ─── C6 Grademon's inherited shield under Alphamon ───────────────────────────

/// **C6 Inherited shield** — EX13-057 (source) under EX13-060 Alphamon protects
/// ANOTHER [Chronicle] Digimon (Dorumon) from deletion by trashing the top
/// security card; a non-[Chronicle] ally (Dracomon) is not covered.
#[test]
fn c6_grademon_under_alphamon_shields_a_chronicle_ally_but_not_a_vanilla_one() {
    let mut r = start(builder().memory(3));
    r.place_stack(0, &[GRADEMON, ALPHAMON]);
    let dor = r.place_on_field(0, DORUMON, Some(0));
    r.place_on_field(0, DRACOMON, Some(0));

    r.game.delete_permanents_batch(vec![dor], ReplacementCause::OpponentEffect);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.execute_action(0, REPLACEMENT_ACCEPT).expect("pay security");
    let _ = r.auto_resolve();
    assert!(find_perm(&r, 0, DORUMON).is_some(), "Dorumon doesn't leave");
    assert_eq!(r.security_count(0), 2, "top security card trashed");

    let draco = find_perm(&r, 0, DRACOMON).expect("Dracomon on field");
    r.game.delete_permanents_batch(vec![draco], ReplacementCause::OpponentEffect);
    assert_ne!(r.pending_kind(), Some(SelectionKind::Replacement), "[Dragon] ally not covered");
    let _ = r.auto_resolve();
    assert!(find_perm(&r, 0, DRACOMON).is_none());
    assert_eq!(r.security_count(0), 2);
}

// ─── C7 Dorimon re-arms the attacker ─────────────────────────────────────────

/// **C7 Dorimon re-arms the attacker** — EX13-006 at the bottom of a
/// Dorimon → Dorumon → Raptordramon stack. Raptordramon attacks (suspends); at
/// end of turn the inherited Dorimon clause pays 1 and unsuspends it
/// (Raptordramon carries both traits), leaving it ready on the opponent's turn.
#[test]
fn c7_dorimon_inherited_unsuspends_the_raptordramon_that_attacked() {
    let mut r = start(builder().memory(3));
    let rap = r.place_stack(0, &[DORIMON, DORUMON, RAPTORDRAMON]);
    r.attack_player(rap, 1, false);
    drive(&mut r, |_, _| None);
    assert!(suspended(&r, rap), "attacking suspended Raptordramon");

    r.game.memory = -3; // the opponent would start with 3
    r.end_turn();
    let view = r.pending_selection_view().expect("Dorimon's optional activation");
    assert!(view.is_optional);
    let before = r.memory();
    let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).expect("accept");
    r.execute_action(0, a).expect("accept");
    assert_eq!(r.memory(), before - 1, "1 cost paid");
    let view = r.pending_selection_view().expect("unsuspend pick");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let a = field_action(&view, rap).expect("Raptordramon selectable");
    r.execute_action(0, a).expect("pick");
    drive(&mut r, |_, _| None);
    assert!(!suspended(&r, rap), "the attacker is ready again");
}

// ─── C8 Kota attack trigger + Raptordramon climb in one attack ───────────────

/// **C8 Two triggers, one attack** — EX13-072 + EX13-055 + EX13-057. One
/// Raptordramon attack opens Kota's [Your Turn] clause AND Raptordramon's
/// [When Attacking] climb; both resolve: Kota suspends to use a [Chronicle]
/// Option at -1 (3 → 2), and Raptordramon digivolves into Grademon from hand
/// (3) whose rider then applies.
///
/// FAILS today — engine gap `G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS`
/// (`docs/RUST_ENGINE_GAPS.md`): `Game::fire_on_attack` drains the attacker's
/// own `[When Attacking]` batch first and returns as soon as it parks a prompt,
/// so the OnAllyAttack fan-out (Kota) is never enqueued. Kota is never offered,
/// stays unsuspended, and the Option stays in hand. DCGO stacks both in ONE
/// `EffectTiming.OnAllyAttack` batch (`Script/AttackProcess.cs` ~L197).
#[test]
#[ignore = "engine gap G-ENGINE-WHEN-ATTACKING-PARK-DROPS-ALLY-ATTACK-OBSERVERS — un-ignore when fixed"]
fn c8_kota_option_use_and_raptordramon_climb_both_resolve_from_one_attack() {
    let mut r = start(
        builder()
            .add_card(chron_option())
            .hand(0, &[CHRON_OPT, GRADEMON])
            .memory(6),
    );
    let kota = r.place_on_field(0, KOTA, Some(0));
    let rap = r.place_on_field(0, RAPTORDRAMON, Some(0));
    let kota_card = r.game.players[0].battle_area[kota.index as usize].top_card().handle();

    r.attack_player(rap, 1, false);
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::UnionZone { .. } => hand_action(r, v, 0, GRADEMON),
        SelectionKind::Hand => hand_action(r, v, 0, CHRON_OPT),
        SelectionKind::OwnField => field_action(v, rap),
        SelectionKind::TriggerOrder => None,
        _ if v.is_optional && v.source_card == kota_card => {
            v.valid_action_ids.iter().copied().find(|&a| a != PASS)
        }
        _ => route_action(v, 3),
    });
    assert!(suspended(&r, kota), "Kota suspended as the cost");
    assert!(trash_ids(&r, 0).contains(&CHRON_OPT.to_string()), "the Option was used");
    assert_eq!(top_id(&r, rap), GRADEMON, "Raptordramon climbed in the same attack");
    assert_eq!(r.memory(), 1, "6 - 2 (Option 3 reduced by 1) - 3 ([Raptordramon] route)");
    assert_eq!(r.effective_dp(rap), Some(12000));
}

