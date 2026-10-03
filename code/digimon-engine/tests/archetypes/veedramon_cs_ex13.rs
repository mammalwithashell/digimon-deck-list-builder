//! EX13 "Veedramon / CS" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/veedramon-cs-ex13-model.md`. A blue CS tempo shell:
//! [Veemon] (EX13-017) searches, the cheap CS curve climbs Veedramon (EX13-019,
//! cost 2) → AeroVeedramon (EX13-022, cost 3), both of which drop the CS Tamer
//! Rina Shinomiya (EX13-069) cheaply/free; Aero's inherited untap feeds Rina's
//! "when your Digimon unsuspend" draw + discounted digivolve, and Veemon's
//! inherited protects the stack from removal. Nokia Shiramine (EX13-067) is the
//! slice's other CS Tamer. EX13-074 Rie Kishibe is BLOCKED (no YAML) and is not
//! exercised here.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{17,19,22,67,69}.rs`;
//! this file asserts only cross-card SYSTEM facts.
//!
//! Real DSL cards fill every role (no synthetic cards). Cross-set pieces:
//! BT13-030 UlforceVeedramon (a [Veedramon]-name Lv.6 top with no
//! [When Attacking]), ST1-03 Agumon, BT23-008 Greymon, ST2-03 Gabumon, and the
//! vanilla ST1-02 Biyomon as filler / opponent body.
//!
//! **Printed-text fidelity.** DSL-loaded `CardData` carries empty printed text,
//! but "[Veedramon] in its text" scans it in production (Rina's name does not
//! contain "Veedramon" — only her text does). `with_printed_text` copies every
//! real card's printed text from its committed per-card JSON (same idiom as
//! `dracomon_ex13.rs`).

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START, SEL_REVEAL_START,
    TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const VEEMON: &str = "EX13-017";
const VEEDRAMON: &str = "EX13-019";
const AERO: &str = "EX13-022";
const NOKIA: &str = "EX13-067";
const RINA: &str = "EX13-069";
/// [Veedramon]-name Lv.6 top with no [When Attacking] / passive effect in play.
const ULFORCE: &str = "BT13-030";
const AGUMON: &str = "ST1-03";
const GREYMON: &str = "BT23-008";
const GABUMON: &str = "ST2-03";
/// Vanilla 3000-DP Rookie (no effect) — filler, opponent bodies, security.
const BIYOMON: &str = "ST1-02";

const REAL: &[&str] = &[
    VEEMON, VEEDRAMON, AERO, NOKIA, RINA, ULFORCE, AGUMON, GREYMON, GABUMON, BIYOMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn set_dir(id: &str) -> String {
    id.split('-').next().unwrap().to_lowercase()
}

fn printed(id: &str) -> CardData {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join(set_dir(id))
        .join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let wrapped = format!("{{\"{id}\": {raw}}}");
    let mut map: HashMap<String, CardData> =
        CardData::load_from_str(&wrapped).unwrap_or_else(|e| panic!("{id}: {e}"));
    map.remove(id).unwrap_or_else(|| panic!("{id} parsed"))
}

fn with_printed_text(runner: &mut DebugRunner) {
    for id in REAL {
        let p = printed(id);
        for c in runner.game.card_data.iter_mut().filter(|c| c.card_id == *id) {
            c.effect_text = p.effect_text.clone();
            c.inherited_text = p.inherited_text.clone();
            c.security_text = p.security_text.clone();
        }
    }
}

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL)
}

fn start(b: DebugRunnerBuilder) -> DebugRunner {
    let mut runner = b
        .deck(0, &[BIYOMON; 8])
        .deck(1, &[BIYOMON; 8])
        .security(1, &[BIYOMON; 5])
        .start();
    runner.skip_mulligan();
    with_printed_text(&mut runner);
    runner
}

fn ids_of(runner: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].hand)
}

fn field_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    ids_of(
        runner,
        &runner.game.players[h.player as usize].battle_area[h.index as usize].card_sources,
    )
}

fn hand_index(runner: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(runner, p)
        .iter()
        .position(|h| h == id)
        .unwrap_or_else(|| panic!("{id} must be in player {p}'s hand: {:?}", hand_ids(runner, p)))
}

fn find_perm(runner: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = field_ids(runner, p)
        .iter()
        .position(|f| f == id)
        .unwrap_or_else(|| panic!("{id} must be on player {p}'s field: {:?}", field_ids(runner, p)));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn locked(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.modifiers.has(h, ModifierType::CannotSuspend)
}

/// Push `ids` onto the top of player 0's deck (the LAST id ends on top).
fn stack_deck_top(runner: &mut DebugRunner, ids: &[&str]) {
    for id in ids {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == *id)
            .unwrap_or_else(|| panic!("card {id} registered"));
        let card_index = runner.game.next_card_index();
        runner.game.players[0]
            .deck
            .push(CardSource::new(data_idx, 0, card_index));
    }
}

fn revealed_ids(runner: &DebugRunner) -> Vec<String> {
    ids_of(runner, &runner.game.revealed_cards)
}

fn view(runner: &DebugRunner) -> PendingSelectionView {
    runner.pending_selection_view().expect("a prompt must be pending")
}

fn view_dbg(runner: &DebugRunner) -> String {
    format!("{:?}", runner.pending_selection_view())
}

fn act(runner: &mut DebugRunner, action: u16) {
    let v = view(runner);
    assert!(v.valid_action_ids.contains(&action), "action {action} must be legal; view={v:?}");
    runner.execute_action(v.selecting_player, action).expect("action accepted");
}

fn pass(runner: &mut DebugRunner) {
    let v = view(runner);
    assert!(v.is_optional, "PASS must be legal; view={v:?}");
    runner.execute_action(v.selecting_player, PASS).expect("pass");
}

/// Take the first non-PASS option of the pending prompt.
fn accept(runner: &mut DebugRunner) {
    let v = view(runner);
    let a = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("non-PASS option; view={v:?}"));
    runner.execute_action(v.selecting_player, a).expect("accept");
}

/// Accept yes/no confirms, trigger ordering, and cost-route choices until a
/// prompt of `kind` is pending (or nothing is). Returns whether it was reached.
fn advance_to(runner: &mut DebugRunner, kind: SelectionKind) -> bool {
    for _ in 0..6 {
        let Some(v) = runner.pending_selection_view() else {
            return false;
        };
        if v.kind == kind {
            return true;
        }
        match v.kind {
            SelectionKind::EffectChoice | SelectionKind::TriggerOrder | SelectionKind::Replacement => {
                accept(runner)
            }
            _ => return false,
        }
    }
    false
}

fn hand_offered(runner: &DebugRunner, v: &PendingSelectionView, id: &str) -> bool {
    let slot = hand_index(runner, 0, id) as u16;
    v.valid_action_ids.contains(&(PLAY_HAND_START + slot))
        || v.valid_action_ids.contains(&(HAND_EFFECT_START + slot))
}

/// Pick `id` from a hand prompt (PLAY_HAND or HAND_EFFECT encoding).
fn pick_hand(runner: &mut DebugRunner, id: &str) {
    assert!(advance_to(runner, SelectionKind::Hand), "hand prompt; view={}", view_dbg(runner));
    let slot = hand_index(runner, 0, id) as u16;
    let v = view(runner);
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| v.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable; view={v:?}"));
    runner.execute_action(v.selecting_player, a).expect("pick hand");
}

fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let pos = revealed_ids(runner)
        .iter()
        .position(|r| r == id)
        .unwrap_or_else(|| panic!("{id} must be revealed: {:?}", revealed_ids(runner)));
    act(runner, SEL_REVEAL_START + pos as u16);
    order_remainder(runner);
}

fn order_remainder(runner: &mut DebugRunner) {
    while let Some(v) = runner.pending_selection_view() {
        if !matches!(v.kind, SelectionKind::OrderedPermutation { .. }) {
            return;
        }
        runner
            .execute_action(v.selecting_player, v.valid_action_ids[0])
            .expect("order remainder");
    }
}

/// Single-side field pick (own-only / opponent-only prompts).
fn one_side(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

/// Digivolve hand card `id` onto `base` via the action space; if the engine
/// asks which digivolution route to pay (both the colour circle and the
/// "w/[CS] trait" route apply and cost the same), take the first.
fn digivolve(runner: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = hand_index(runner, 0, id) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
    if let Some(v) = runner.pending_selection_view() {
        if v.effect_choices
            .as_ref()
            .is_some_and(|cs| cs.iter().any(|c| c.label.contains("cost")))
        {
            runner.execute_branch(0).expect("choose digivolution route");
        }
    }
}

fn fire_start_of_main(runner: &mut DebugRunner, tamers: &[PermanentHandle]) {
    for t in tamers {
        runner
            .game
            .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(*t));
    }
    runner.game.drain_effect_queue();
    for _ in 0..4 {
        match runner.pending_selection_view() {
            Some(v) if v.kind == SelectionKind::TriggerOrder => accept(runner),
            _ => break,
        }
    }
    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// C1 — Veemon search → Veedramon on curve (CS route, cost 2)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-017 Veemon, EX13-019 Veedramon, EX13-069 Rina (alternative
/// legal hit), ST1-02 Biyomon (illegal hit).
///
/// Expected mechanical outcome: Veemon's [On Play] reveals 3; Veedramon (name)
/// and Rina ([Veedramon] only in her TEXT) are both offered, Biyomon is not.
/// Veedramon is added, the rest go to the bottom; Veedramon then digivolves onto
/// the same Veemon for 2. Net memory −3 −2, a Veemon→Veedramon stack.
///
/// Sources: `cards/ex13/EX13-017.json`, `EX13-019.json`, `EX13-069.json`;
/// DCGO `EX13/Blue/EX13_017.cs` (SimplifiedRevealDeckTopCardsAndSelect,
/// HasText("Veedramon") || HasRoyalKnightTraits), `EX13/Blue/EX13_019.cs`
/// (alt digivolve Lv.3 w/[CS] cost 2).
#[test]
fn c1_veemon_search_finds_veedramon_and_rina_then_veedramon_digivolves_for_2() {
    let mut runner = start(builder().hand(0, &[VEEMON]).memory(10));
    stack_deck_top(&mut runner, &[BIYOMON, RINA, VEEDRAMON]);
    let before = snapshot(&runner);

    let field = runner.play(0, hand_index(&runner, 0, VEEMON)).expect("Veemon plays");
    let vee = PermanentHandle {
        player: 0,
        index: field as u8,
    };
    let v = view(&runner);
    let offered: Vec<String> = revealed_ids(&runner)
        .iter()
        .enumerate()
        .filter(|(i, _)| v.valid_action_ids.contains(&(SEL_REVEAL_START + *i as u16)))
        .map(|(_, id)| id.clone())
        .collect();
    assert!(offered.contains(&VEEDRAMON.to_string()), "Veedramon offered: {offered:?}");
    assert!(offered.contains(&RINA.to_string()), "Rina ([Veedramon] in text) offered: {offered:?}");
    assert!(!offered.contains(&BIYOMON.to_string()), "Biyomon not offered: {offered:?}");
    pick_revealed(&mut runner, VEEDRAMON);
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner, 0), vec![VEEDRAMON]);

    let mem = runner.memory();
    digivolve(&mut runner, VEEDRAMON, vee);
    let _ = runner.auto_resolve();
    assert_eq!(mem - runner.memory(), 2, "Lv.3 → Veedramon: Cost 2");
    assert_eq!(stack_ids(&runner, vee), vec![VEEMON, VEEDRAMON]);
    let after = snapshot(&runner);
    assert_eq!(before.memory - after.memory, 3 + 2);
    // Deck: −3 revealed, +2 to bottom, −1 digivolution draw.
    assert_eq!(after.deck[0], before.deck[0] - 1 - 1);
    assert_eq!(after.hand[0], 1, "only the digivolution draw remains in hand");
}

// ═══════════════════════════════════════════════════════════════════════════
// C2 — Veedramon attacks → Rina for 1 (Nokia not eligible)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-019 Veedramon over EX13-017 Veemon, EX13-069 Rina, EX13-067
/// Nokia (a CS Tamer WITHOUT [Veedramon] in its text).
///
/// Expected mechanical outcome: Veedramon's [When Attacking][OPT] offers Rina but
/// not Nokia; Rina enters for 3 − 2 = 1 memory; the attack then checks a security
/// card.
///
/// Sources: `cards/ex13/EX13-019.json`; DCGO `EX13/Blue/EX13_019.cs`
/// (OnAllyAttack SelectHandEffect IsTamer && HasText("Veedramon"), cost −2).
#[test]
fn c2_veedramon_attack_plays_rina_for_one_but_not_nokia() {
    let mut runner = start(builder().hand(0, &[RINA, NOKIA]).memory(3));
    let vd = runner.place_stack(0, &[VEEMON, VEEDRAMON]);
    let mem0 = runner.memory();

    runner.attack_player(vd, 1, false);
    assert!(advance_to(&mut runner, SelectionKind::Hand), "WA hand prompt; {}", view_dbg(&runner));
    let v = view(&runner);
    assert!(v.is_optional, "\"You may play\"");
    assert!(hand_offered(&runner, &v, RINA), "Rina offered");
    assert!(!hand_offered(&runner, &v, NOKIA), "Nokia has no [Veedramon] text");
    pick_hand(&mut runner, RINA);
    for _ in 0..8 {
        if runner.pending_selection_view().is_none() {
            break;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert!(field_ids(&runner, 0).contains(&RINA.to_string()), "Rina in play");
    assert_eq!(mem0 - runner.memory(), 1, "Rina cost 3 reduced by 2");
    assert_eq!(hand_ids(&runner, 0), vec![NOKIA]);
    assert_eq!(runner.security_count(1), 4, "the attack checked one security card");
}

// ═══════════════════════════════════════════════════════════════════════════
// C3 — AeroVeedramon WD → free Rina → Aero's own Tamer-played lock
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-022 AeroVeedramon, EX13-019 / EX13-017 (stack), EX13-069 Rina,
/// EX13-067 Nokia (second Tamer), ST1-02 Biyomon (opponent).
///
/// Expected mechanical outcome: AeroVeedramon digivolves onto Veedramon for 3;
/// its [When Digivolving] plays Rina without paying; that Tamer entering play
/// fires Aero's OWN [All Turns][OPT] lock → the opponent's Biyomon gets
/// CannotSuspend. Memory −3 only. Paying for Nokia afterwards the same turn
/// does NOT lock again (OPT spent), and the shared [OP]/[WD]/[WA] OPT is spent
/// too, so attacking with Aero offers no further free Tamer.
///
/// Sources: `cards/ex13/EX13-022.json`; DCGO `EX13/Blue/EX13_022.cs` (shared
/// hash "EX13_022_OP_WD_WA"; OnEnterFieldAnyone → GainCantSuspendUntilOpponentTurnEnd).
#[test]
fn c3_aero_wd_plays_rina_free_and_the_tamer_play_fires_its_own_lock() {
    let mut runner = start(builder().hand(0, &[AERO, RINA, NOKIA]).memory(10));
    let vd = runner.place_stack(0, &[VEEMON, VEEDRAMON]);
    let opp = runner.place_on_field(1, BIYOMON, Some(0));
    let mem0 = runner.memory();

    digivolve(&mut runner, AERO, vd);
    pick_hand(&mut runner, RINA);
    let v = view(&runner);
    assert_eq!(v.kind, SelectionKind::OppField, "Aero's lock target prompt: {v:?}");
    act(&mut runner, one_side(opp));
    let _ = runner.auto_resolve();

    assert_eq!(stack_ids(&runner, vd), vec![VEEMON, VEEDRAMON, AERO]);
    assert!(field_ids(&runner, 0).contains(&RINA.to_string()));
    assert_eq!(mem0 - runner.memory(), 3, "digivolve 3; Rina free");
    assert!(locked(&runner, opp), "opponent Biyomon can't suspend");

    // Second Tamer (paid) the same turn: lock OPT spent.
    runner.play(0, hand_index(&runner, 0, NOKIA)).expect("Nokia plays");
    assert!(runner.pending_selection().is_none(), "lock is [Once Per Turn]: {}", view_dbg(&runner));
    assert_eq!(mem0 - runner.memory(), 3 + 4);
}

// ═══════════════════════════════════════════════════════════════════════════
// C4 — Unsuspend engine: attack → Aero inherited untap → Rina draw + 0-cost evo
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: BT13-030 UlforceVeedramon (top) over EX13-022 / EX13-019 / EX13-017,
/// EX13-069 Rina, a second EX13-017 Veemon on the field, EX13-019 in hand.
///
/// Expected mechanical outcome: attacking suspends Ulforce → AeroVeedramon's
/// INHERITED [OPT] "this [Veedramon]-name Digimon suspends → may unsuspend" →
/// that unsuspend is "any of your Digimon unsuspend" → Rina suspends: <Draw 1>,
/// then the second Veemon digivolves into the hand Veedramon for 2 − 2 = 0.
/// Net: attacker unsuspended, Rina suspended, memory unchanged, hand 1 → 2.
///
/// Sources: `cards/ex13/EX13-022.json` (inherited), `EX13-069.json`; DCGO
/// `EX13/Blue/EX13_022.cs` (OnTappedAnyone → IUnsuspendPermanents),
/// `EX13/Blue/EX13_069.cs` (OnUnTappedAnyone → suspend cost → DrawClass(1) →
/// DigivolveIntoHandOrTrashCard reduce 2).
#[test]
fn c4_attack_untaps_via_aero_inherited_and_rina_draws_and_digivolves_for_0() {
    let mut runner = start(builder().hand(0, &[VEEDRAMON]).memory(3));
    let top = runner.place_stack(0, &[VEEMON, VEEDRAMON, AERO, ULFORCE]);
    let rina = runner.place_on_field(0, RINA, Some(0));
    let vee2 = runner.place_on_field(0, VEEMON, Some(0));
    let mem0 = runner.memory();
    let sec0 = runner.security_count(1);

    runner.attack_player(top, 1, false);
    // Aero inherited: "it may unsuspend".
    assert!(advance_to(&mut runner, SelectionKind::OwnField), "Rina's own-Digimon pick; {}", view_dbg(&runner));
    assert!(is_suspended(&runner, rina), "Rina paid her suspend cost");
    assert_eq!(hand_ids(&runner, 0).len(), 2, "Rina's <Draw 1>");
    act(&mut runner, one_side(vee2));
    pick_hand(&mut runner, VEEDRAMON);
    let _ = runner.auto_resolve();
    for _ in 0..6 {
        if runner.pending_selection_view().is_none() {
            break;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert!(!is_suspended(&runner, top), "Ulforce untapped by Aero's inherited effect");
    assert_eq!(stack_ids(&runner, vee2), vec![VEEMON, VEEDRAMON], "second Veemon digivolved");
    assert_eq!(runner.memory(), mem0, "Veedramon cost 2 reduced by 2 → 0");
    assert_eq!(hand_ids(&runner, 0).len(), 2, "1 + Draw 1 − Veedramon + digivolution draw");
    assert_eq!(runner.security_count(1), sec0 - 1, "the attack still resolved");
}

// ═══════════════════════════════════════════════════════════════════════════
// C5 — Resilience chain: opponent removal → Veemon suspend-instead →
//      Aero inherited untap → Rina draw
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-017 Veemon (inherited replacement), EX13-022 AeroVeedramon
/// (inherited untap), EX13-019, BT13-030 UlforceVeedramon (top), EX13-069 Rina.
///
/// Expected mechanical outcome: an opponent's effect deleting the Ulforce stack
/// is replaced by suspending it (Veemon inherited, [Veedramon]-name carrier);
/// that suspension triggers Aero's inherited "may unsuspend", whose unsuspend
/// triggers Rina's <Draw 1>. Ulforce survives untapped, Rina is suspended, hand
/// +1, trash untouched. A second opponent deletion the same turn is NOT
/// prevented (Veemon's [Once Per Turn] is spent) — the stack goes to trash.
///
/// The opponent's deleting effect is driven through the engine's batched
/// deletion entry point with `ReplacementCause::OpponentEffect` (the per-card
/// EX13-017 idiom): the combo is about the defensive chain, not the remover.
///
/// Sources: `cards/ex13/EX13-017.json` (inherited); DCGO `EX13/Blue/EX13_017.cs`
/// (WhenRemoveField IsByEffect(opponent), CanActivateSuspendCostEffect),
/// `EX13_022.cs`, `EX13_069.cs`.
#[test]
fn c5_opponent_removal_becomes_suspend_then_untap_then_rina_draw() {
    let mut runner = start(builder().memory(3));
    let top = runner.place_stack(0, &[VEEMON, VEEDRAMON, AERO, ULFORCE]);
    let rina = runner.place_on_field(0, RINA, Some(0));
    let before = snapshot(&runner);

    runner
        .game
        .delete_permanents_batch(vec![top], ReplacementCause::OpponentEffect);
    for _ in 0..8 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        // Rina's follow-up "1 of your Digimon may digivolve": nothing to digivolve into.
        if v.kind == SelectionKind::OwnField {
            pass(&mut runner);
            continue;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    let ul = find_perm(&runner, 0, ULFORCE);
    assert_eq!(stack_ids(&runner, ul), vec![VEEMON, VEEDRAMON, AERO, ULFORCE], "it doesn't leave");
    assert!(!is_suspended(&runner, ul), "suspended by Veemon's cost, then untapped by Aero's inherited");
    assert!(is_suspended(&runner, rina), "Rina paid her suspend cost");
    assert_eq!(after.hand[0], before.hand[0] + 1, "Rina's <Draw 1>");
    assert_eq!(after.trash[0], 0);

    // Second opponent deletion this turn: Veemon's OPT is spent.
    runner
        .game
        .delete_permanents_batch(vec![ul], ReplacementCause::OpponentEffect);
    assert!(
        runner
            .pending_selection_view()
            .map_or(true, |v| v.kind != SelectionKind::Replacement),
        "no second leave-prevention: {}",
        view_dbg(&runner)
    );
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&ULFORCE.to_string()), "deleted this time");
    assert_eq!(runner.trash_size(0), 4, "the whole stack went to trash");
}

// ═══════════════════════════════════════════════════════════════════════════
// C6 — Security Rina → AeroVeedramon lock on the opponent's turn
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-069 Rina (in security), EX13-022 AeroVeedramon (field), ST1-02
/// Biyomon ×2 (opponent attackers).
///
/// Expected mechanical outcome: the opponent attacks and checks Rina; her
/// [Security] plays her; that is "any of your Tamers are played", and Aero's
/// clause is [All Turns], so it fires on the OPPONENT's turn → their other
/// (still unsuspended) Biyomon can't suspend until the end of that turn.
///
/// Sources: `cards/ex13/EX13-069.json` ([Security] play), `EX13-022.json`;
/// DCGO `EX13_069.cs` (PlaySelfTamerSecurityEffect → PlayPermanentCards
/// `activateETB: true`, `Script/CardEffectFactory.cs`), `EX13_022.cs`
/// (OnEnterFieldAnyone, no turn gate).
///
/// **Ignored reproducer (CONFIRMED engine bug).** Rina reaches the battle area
/// but Aero's observer never fires: `EffectContext::play_pending_security`
/// (`src/effect_context/selections.rs`) calls `Game::fire_on_play` only, not
/// `Game::fire_play_event_triggers`, so `OnEnterFieldAnyone` / `OnAllyPlayed`
/// observers miss every "[Security] Play this card". Routed to
/// `docs/RUST_ENGINE_GAPS.md` §G-ENGINE-SECURITY-PLAY-SKIPS-ENTER-FIELD-OBSERVERS.
#[test]
#[ignore = "G-ENGINE-SECURITY-PLAY-SKIPS-ENTER-FIELD-OBSERVERS: play_pending_security fires only OnPlay, not OnEnterFieldAnyone/OnAllyPlayed (docs/RUST_ENGINE_GAPS.md)"]
fn c6_security_rina_played_on_opponents_turn_fires_aero_lock() {
    let mut runner = start(builder().security(0, &[RINA]).memory(3));
    runner.place_on_field(0, AERO, Some(0));
    let atk = runner.place_on_field(1, BIYOMON, Some(0));
    let other = runner.place_on_field(1, BIYOMON, Some(0));

    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1, "opponent's turn");
    // The debug end_turn hands the opponent an empty gauge; give them room so
    // their turn does not auto-pass after the attack resolves.
    runner.game.memory = 3;
    runner.attack_player(atk, 0, false);
    // Decline any block / ordering prompts until Aero's lock target prompt.
    let mut lock_prompt = false;
    for _ in 0..8 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.kind == SelectionKind::OppField && v.selecting_player == 0 {
            lock_prompt = true;
            break;
        }
        if v.is_optional {
            pass(&mut runner);
        } else {
            accept(&mut runner);
        }
    }
    assert!(lock_prompt, "Aero's lock fired on the opponent's turn; {}", view_dbg(&runner));
    act(&mut runner, one_side(other));
    let _ = runner.auto_resolve();

    assert!(field_ids(&runner, 0).contains(&RINA.to_string()), "Rina played from security");
    assert_eq!(runner.security_count(0), 0);
    assert!(locked(&runner, other), "the other opponent Biyomon can't suspend");
}

// ═══════════════════════════════════════════════════════════════════════════
// C7 — Nokia: Agumon → Greymon with a lone Digimon → free Gabumon from trash
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-067 Nokia, ST1-03 Agumon, BT23-008 Greymon (CS), ST2-03 Gabumon
/// (trash), ST1-02 Biyomon (unhappy-path second Digimon).
///
/// Expected mechanical outcome: Greymon digivolves onto Agumon for 2; you have 1
/// Digimon, so Nokia may suspend to play [Gabumon] (the digivolved Digimon has
/// [Greymon] in its name) from the trash without paying. Memory −2 only.
///
/// Sources: `cards/ex13/EX13-067.json` (official Bandai DB text + Q&A; no DCGO
/// script), `cards/bt23/BT23-008.json` (alt digivolve Lv.3 w/[Agumon] name /
/// [CS] cost 2).
#[test]
fn c7_nokia_plays_gabumon_free_when_lone_agumon_becomes_greymon() {
    let mut runner = start(builder().hand(0, &[GREYMON]).memory(5));
    let nokia = runner.place_on_field(0, NOKIA, Some(0));
    let agu = runner.place_on_field(0, AGUMON, Some(0));
    runner.inject_trash(0, GABUMON);
    let mem0 = runner.memory();

    digivolve(&mut runner, GREYMON, agu);
    let v = view(&runner);
    assert!(v.is_optional, "Nokia's effect is optional: {v:?}");
    accept(&mut runner);
    // Union [hand, trash] pick: the trashed Gabumon.
    let v = view(&runner);
    let a = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a >= TRASH_EFFECT_START && a != PASS)
        .unwrap_or_else(|| panic!("a trash pick; view={v:?}"));
    runner.execute_action(v.selecting_player, a).expect("pick Gabumon");
    let _ = runner.auto_resolve();

    assert!(is_suspended(&runner, nokia), "Nokia paid her suspend cost");
    let fields = field_ids(&runner, 0);
    assert!(fields.contains(&GREYMON.to_string()) && fields.contains(&GABUMON.to_string()), "{fields:?}");
    assert_eq!(runner.trash_size(0), 0);
    assert_eq!(mem0 - runner.memory(), 2, "digivolve 2; Gabumon free");
}

/// Unhappy path of C7: with a second Digimon on the field ("1 or fewer" fails)
/// the digivolution offers nothing.
#[test]
fn c7_nokia_does_not_trigger_with_two_digimon() {
    let mut runner = start(builder().hand(0, &[GREYMON]).memory(5));
    let nokia = runner.place_on_field(0, NOKIA, Some(0));
    let agu = runner.place_on_field(0, AGUMON, Some(0));
    runner.place_on_field(0, BIYOMON, Some(0));
    runner.inject_trash(0, GABUMON);

    digivolve(&mut runner, GREYMON, agu);
    assert!(runner.pending_selection().is_none(), "no Nokia prompt: {}", view_dbg(&runner));
    assert!(!is_suspended(&runner, nokia));
    assert_eq!(runner.trash_size(0), 1, "Gabumon stays in trash");
}

// ═══════════════════════════════════════════════════════════════════════════
// C8 — CS Tamer ramp at Start of Your Main Phase
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-069 Rina, EX13-067 Nokia, EX13-017 Veemon, ST1-02 Biyomon.
///
/// Expected mechanical outcome: with a Veemon on your field and a Digimon on the
/// opponent's, both Tamers' [Start of Your Main Phase] fire: +2 memory. With
/// only a Biyomon (no [Veemon]/[Veedramon] name) Rina's condition fails and only
/// Nokia gains: +1.
///
/// Sources: `cards/ex13/EX13-069.json`, `EX13-067.json`; DCGO `EX13_069.cs`
/// (StartOfYourMainPhaseClass, ContainsCardName Veemon|Veedramon).
#[test]
fn c8_rina_and_nokia_ramp_with_veemon_on_field() {
    let mut runner = start(builder().memory(0));
    let rina = runner.place_on_field(0, RINA, Some(0));
    let nokia = runner.place_on_field(0, NOKIA, Some(0));
    runner.place_on_field(0, VEEMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    fire_start_of_main(&mut runner, &[rina, nokia]);
    assert_eq!(runner.memory(), 2, "Rina +1 and Nokia +1");
}

#[test]
fn c8_rina_needs_a_veemon_or_veedramon_name() {
    let mut runner = start(builder().memory(0));
    let rina = runner.place_on_field(0, RINA, Some(0));
    let nokia = runner.place_on_field(0, NOKIA, Some(0));
    runner.place_on_field(0, BIYOMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    fire_start_of_main(&mut runner, &[rina, nokia]);
    assert_eq!(runner.memory(), 1, "only Nokia gains");
}
