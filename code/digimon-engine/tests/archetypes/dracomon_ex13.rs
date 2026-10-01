//! EX13 "Dracomon / dragon" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/dracomon-ex13-model.md`. A red/green/blue Dragon
//! midrange-combo shell that searches with [Dracomon] (EX13-008), climbs through
//! the two [Coredramon] (EX13-018 draw / EX13-039 recursion) on their cheap
//! "Lv.3 w/[Dracomon] in name: Cost 2" route, assembles [Breakdramon] (EX13-044)
//! out of the trash the Coredramons fill, and closes with [Examon] (EX13-045),
//! DNA-digivolved at end of turn via Dracomon's inherited effect.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{08,18,39,44}.rs`;
//! this file asserts only the cross-card SYSTEM facts no per-card test can see.
//!
//! Real DSL cards fill every role. The single synthetic card is the Lv.5
//! Assembly material in Combos 1–6, authored while EX13-021 Wingdramon /
//! EX13-041 Groundramon were BLOCKED (both are implemented as of 2026-10-01 —
//! Combo 7 uses them for [Examon]'s DNA). Its own printed effect is never fired.
//!
//! **Printed-text fidelity.** DSL-loaded `CardData` carries empty printed text
//! (`card_data_from_compiled`), but every "[Dracomon] or [Examon] in its text"
//! filter (DCGO `CardSource.HasText`) scans that text in production. So
//! `with_printed_text` copies each real card's effect / inherited / security
//! text from its committed per-card JSON (`cards/<set>/<ID>.json`, the same
//! `RawCard` shape `CardData::load_from_str` ingests for `cards.json`). Without
//! it, a Coredramon / Breakdramon would not count as a [Dracomon]-text card.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{
    encode_attack, encode_digivolve, encode_source_select, PASS, SECURITY_TARGET, PLAY_HAND_START, HAND_EFFECT_START, SEL_REVEAL_START,
    TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const DRACOMON: &str = "EX13-008";
const COREDRAMON_BLUE: &str = "EX13-018";
const COREDRAMON_GREEN: &str = "EX13-039";
const BREAKDRAMON: &str = "EX13-044";
const EXAMON: &str = "EX13-045";
/// Vanilla 3000-DP Rookie (no effect) — opponent targets + deck filler.
const BIYOMON: &str = "ST1-02";
/// Vanilla blue Lv.6 (no effect) — the Blue Lv.6 half of Examon's DNA recipe.
const PLESIOMON: &str = "ST2-10";
/// Synthetic Lv.5 Assembly material (see module doc for why no real card fits).
const MAT_L5: &str = "SYN-L5-EXAMON-TEXT";

const REAL: &[&str] = &[
    DRACOMON,
    COREDRAMON_BLUE,
    COREDRAMON_GREEN,
    BREAKDRAMON,
    EXAMON,
    BIYOMON,
    PLESIOMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn set_dir(id: &str) -> String {
    id.split('-').next().unwrap().to_lowercase()
}

/// Production `CardData` for `id`, parsed from its committed per-card JSON.
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

/// Copy the printed text of every real card into the runner's card database
/// (see the module doc: DSL-loaded cards carry empty text).
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
    // Role: Lv.5 [Examon]-text Assembly material (synthetic — see module doc).
    let mut l5 = make_test_card(MAT_L5, "Dragon Five");
    l5.level = Some(5);
    l5.dp = Some(7000);
    l5.play_cost = 7;
    l5.colors = vec![CardColor::Green];
    l5.effect_text = "Treated as [Examon] DNA material.".to_string();
    dsl_builder(REAL).add_card(l5)
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

fn trash_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].trash)
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

fn view_dbg(runner: &DebugRunner) -> String {
    format!("{:?}", runner.pending_selection_view())
}

fn act(runner: &mut DebugRunner, action: u16) {
    let view = runner
        .pending_selection_view()
        .unwrap_or_else(|| panic!("a prompt must be pending to take action {action}"));
    assert!(
        view.valid_action_ids.contains(&action),
        "action {action} must be legal; view={view:?}"
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("action accepted");
}

fn pass(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("a prompt must be pending");
    assert!(view.is_optional, "PASS must be legal; view={view:?}");
    runner.execute_action(view.selecting_player, PASS).expect("pass");
}

/// Take the first non-PASS option of the pending prompt.
fn accept(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("a prompt must be pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("non-PASS option; view={view:?}"));
    runner.execute_action(view.selecting_player, a).expect("accept");
}

/// Pick `id` from a hand prompt (PLAY_HAND or HAND_EFFECT encoding).
fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let slot = hand_index(runner, 0, id) as u16;
    let view = runner.pending_selection_view().expect("hand prompt");
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable; view={view:?}"));
    runner.execute_action(view.selecting_player, a).expect("pick hand");
}

/// Pick `id` out of the reveal pool, then order the remainder (any order).
fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let pos = revealed_ids(runner)
        .iter()
        .position(|r| r == id)
        .unwrap_or_else(|| panic!("{id} must be revealed: {:?}", revealed_ids(runner)));
    act(runner, SEL_REVEAL_START + pos as u16);
    order_remainder(runner);
}

fn order_remainder(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        if !matches!(view.kind, SelectionKind::OrderedPermutation { .. }) {
            return;
        }
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("order remainder");
    }
}

/// Play Breakdramon from hand through Assembly -5, picking the three trash
/// materials (the caller seeds exactly one legal card per level).
fn assemble_breakdramon(runner: &mut DebugRunner) {
    let slot = hand_index(runner, 0, BREAKDRAMON) as u16;
    runner.game.decode_action(PLAY_HAND_START + slot, 0);
    for _ in 0..3 {
        let view = runner
            .pending_selection_view()
            .expect("an Assembly material prompt");
        let a = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a >= TRASH_EFFECT_START && a != PASS)
            .unwrap_or_else(|| panic!("a trash material pick; view={view:?}"));
        runner.execute_action(view.selecting_player, a).expect("material");
    }
    assert!(
        field_ids(runner, 0).contains(&BREAKDRAMON.to_string()),
        "Breakdramon is in play"
    );
}

/// Digivolve hand card `id` onto `base`, resolving the "which digivolution
/// cost" choice to the `cost` route. A real (red) Dracomon base satisfies BOTH
/// Coredramon routes — the Red Lv.3 circle (3) and "Lv.3 w/[Dracomon] in name"
/// (2) — so the engine asks; the assertion pins that both are offered.
fn digivolve_paying(runner: &mut DebugRunner, id: &str, base: PermanentHandle, cost: u8) {
    let slot = hand_index(runner, 0, id) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
    let view = runner.pending_selection_view().expect("digivolution-cost choice");
    let choices = view.effect_choices.clone().unwrap_or_else(|| panic!("cost choice; view={view:?}"));
    let labels: Vec<String> = choices.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels.iter().any(|l| l.ends_with("cost 2")) && labels.iter().any(|l| l.ends_with("cost 3")),
        "both the [Dracomon]-name (2) and Red Lv.3 (3) routes are offered: {labels:?}"
    );
    let idx = labels
        .iter()
        .position(|l| l.ends_with(&format!("cost {cost}")))
        .unwrap_or_else(|| panic!("a cost-{cost} route: {labels:?}"));
    runner.execute_branch(idx).expect("choose digivolution cost");
}

/// Action id for a pick over BOTH players' fields (`select_any_permanent`).
fn any_side(h: PermanentHandle) -> u16 {
    encode_attack(h.player as u16, h.index as u16)
}

/// Action id for a single-side field pick (own-only / opponent-only prompts).
fn one_side(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Dracomon search → Coredramon (name route, cost 2) → trash-to-draw
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-008 Dracomon, EX13-018 Coredramon (blue), EX13-039 Coredramon
/// (green, as the trashed [Dracomon]-text cost).
///
/// Expected mechanical outcome: Dracomon's [On Play] reveal finds Coredramon
/// (its text names [Dracomon]); Coredramon then digivolves onto that same
/// Dracomon for **2** (the "Lv.3 w/[Dracomon] in name" route, not the Red Lv.3
/// circle's 3); its [When Digivolving] trashes a second [Dracomon]-text card
/// for <Draw 2>. Net from the 2-card hand: memory −5, hand 2 → 3, one card in
/// trash, a 2-card Dracomon→Coredramon stack.
///
/// Sources: printed text `cards/ex13/EX13-008.json`, `EX13-018.json`;
/// DCGO `EX13/Red/EX13_008.cs` (SimplifiedRevealDeckTopCardsAndSelect,
/// HasText), `EX13/Blue/EX13_018.cs` (ContainsCardName("Dracomon") cost 2;
/// SelectHandEffect discard → DrawClass(2)); `general_rule.pdf` digivolution
/// draw (rule 5-3 / §6 digivolve procedure).
#[test]
fn dracomon_search_finds_coredramon_which_digivolves_for_2_and_draws() {
    let mut runner = start(builder().hand(0, &[DRACOMON, COREDRAMON_GREEN]).memory(10));
    stack_deck_top(&mut runner, &[BIYOMON, BIYOMON, COREDRAMON_BLUE]);
    let before = snapshot(&runner);

    let slot = hand_index(&runner, 0, DRACOMON);
    let field = runner.play(0, slot).expect("Dracomon plays");
    let draco = PermanentHandle {
        player: 0,
        index: field as u8,
    };
    pick_revealed(&mut runner, COREDRAMON_BLUE);
    assert!(hand_ids(&runner, 0).contains(&COREDRAMON_BLUE.to_string()), "search hit");

    let mem_before_evo = runner.memory();
    digivolve_paying(&mut runner, COREDRAMON_BLUE, draco, 2);
    assert_eq!(mem_before_evo - runner.memory(), 2, "[Dracomon]-name route: Cost 2");

    // [When Digivolving]: accept any yes/no gate, then trash the green Coredramon.
    for _ in 0..3 {
        let view = runner.pending_selection_view().expect("WD trash-to-draw prompt");
        if view.kind == SelectionKind::Hand {
            break;
        }
        accept(&mut runner);
    }
    pick_hand(&mut runner, COREDRAMON_GREEN);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    assert_eq!(before.memory - after.memory, 3 + 2, "play 3 + digivolve 2");
    assert_eq!(stack_ids(&runner, draco), vec![DRACOMON, COREDRAMON_BLUE]);
    assert_eq!(trash_ids(&runner, 0), vec![COREDRAMON_GREEN], "the [Dracomon]-text cost");
    // 2 → play (1) → +search (2) → digivolve (1) → +evo draw (2) → cost (1) → +2 (3)
    assert_eq!(after.hand[0], 3, "net card advantage: {:?}", hand_ids(&runner, 0));
    assert_eq!(after.deck[0], before.deck[0] - 1 - 1 - 2, "search + evo draw + Draw 2");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — Green Coredramon recursion → replayed Dracomon searches again
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-008 Dracomon (×2: one on field, one in trash), EX13-039
/// Coredramon (green), EX13-018 Coredramon (the second search hit).
///
/// Expected mechanical outcome: digivolving EX13-039 onto Dracomon (cost 2)
/// returns the trashed Dracomon to hand ([When Digivolving], non-Digi-Egg
/// [Dracomon]-text card); replaying that Dracomon fires its [On Play] reveal a
/// second time, adding the next Coredramon. Replaying it ALSO fires EX13-039's
/// own [Your Turn] "other [Dracomon]-text Digimon played" observer (the system
/// fact); with no [Examon]-text Digimon in hand there is nothing to digivolve
/// into, so declining it leaves Coredramon in place. Memory −2 −3.
///
/// Sources: printed text `cards/ex13/EX13-039.json`, `EX13-008.json`;
/// DCGO `EX13/Green/EX13_039.cs` (SelectCardEffect Root.Trash AddHand,
/// canNoSelect; OnEnterFieldAnyone CanTriggerOnPermanentPlay),
/// `EX13/Red/EX13_008.cs`.
#[test]
fn green_coredramon_recurs_dracomon_whose_replay_searches_again() {
    let mut runner = start(builder().hand(0, &[COREDRAMON_GREEN]).memory(10));
    let draco = runner.place_on_field(0, DRACOMON, Some(0));
    runner.inject_trash(0, DRACOMON);
    stack_deck_top(&mut runner, &[BIYOMON, BIYOMON, COREDRAMON_BLUE]);
    // The digivolution draw takes the top card, so put the search hit one
    // deeper: [.., COREDRAMON_BLUE, BIYOMON(top)].
    stack_deck_top(&mut runner, &[BIYOMON]);
    let mem0 = runner.memory();

    digivolve_paying(&mut runner, COREDRAMON_GREEN, draco, 2);
    assert_eq!(mem0 - runner.memory(), 2, "[Dracomon]-name route: Cost 2");
    // [When Digivolving] "may return": pick the trashed Dracomon.
    let view = runner.pending_selection_view().expect("WD trash-recursion prompt");
    assert!(view.is_optional, "\"You may return\"");
    accept(&mut runner);
    let _ = runner.auto_resolve();
    assert!(hand_ids(&runner, 0).contains(&DRACOMON.to_string()), "Dracomon recurred");
    assert!(trash_ids(&runner, 0).is_empty());

    // Replay it: two triggers — Dracomon's reveal and Coredramon's observer.
    let slot = hand_index(&runner, 0, DRACOMON);
    runner.play(0, slot).expect("recurred Dracomon replays");
    let mut saw_observer = false;
    let mut searched = false;
    for _ in 0..12 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if !runner.game.revealed_cards.is_empty()
            && view.valid_action_ids.iter().any(|a| (SEL_REVEAL_START..SEL_REVEAL_START + 10).contains(a))
            && view.kind != SelectionKind::Hand
            && !searched
        {
            pick_revealed(&mut runner, COREDRAMON_BLUE);
            searched = true;
        } else if view.kind == SelectionKind::Hand {
            // Coredramon's "may digivolve into an [Examon]-text Digimon": none in hand.
            saw_observer = true;
            pass(&mut runner);
        } else if view.is_optional && view.effect_choices.is_none() && searched {
            pass(&mut runner);
        } else {
            accept(&mut runner);
        }
    }
    let _ = runner.auto_resolve();

    assert!(searched, "the replayed Dracomon's [On Play] reveal fired");
    assert!(
        hand_ids(&runner, 0).contains(&COREDRAMON_BLUE.to_string()),
        "second search hit: {:?}",
        hand_ids(&runner, 0)
    );
    let core = find_perm(&runner, 0, COREDRAMON_GREEN);
    assert_eq!(stack_ids(&runner, core), vec![DRACOMON, COREDRAMON_GREEN]);
    assert!(field_ids(&runner, 0).contains(&DRACOMON.to_string()), "Dracomon back in play");
    assert_eq!(mem0 - runner.memory(), 2 + 3, "digivolve 2 + replay 3");
    assert!(
        saw_observer,
        "replaying the recurred Dracomon fired EX13-039's own [Your Turn] observer"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — Coredramon's discard fuels Breakdramon's Assembly -5
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-008 Dracomon (×2), EX13-018 Coredramon (blue), EX13-039
/// Coredramon (green, already in trash), EX13-044 Breakdramon.
///
/// Expected mechanical outcome: the [Dracomon] Coredramon trashes as its <Draw
/// 2> cost is exactly the Lv.3 [Dracomon]-text piece Breakdramon's Assembly
/// needs. With a Lv.4 (green Coredramon) and Lv.5 [Examon]-text card already in
/// trash, Breakdramon is played for 12 − 5 = **7** with all three under it and
/// the trash emptied. Unhappy path: before the discard (no Lv.3 in trash) the
/// recipe is incomplete.
///
/// Sources: printed text `cards/ex13/EX13-018.json`, `EX13-044.json`
/// ("Assembly -5 Lv.5 × Lv.4 × Lv.3, all w/[Dracomon]/[Examon] in text");
/// DCGO `EX13/Green/EX13_044.cs` (AddAssemblyConditionClass, reduceCost 5);
/// `general_rule.pdf` Assembly (§16 keyword list).
#[test]
fn coredramon_discard_completes_breakdramons_assembly() {
    let mut runner = start(
        builder()
            .hand(0, &[COREDRAMON_BLUE, DRACOMON, BREAKDRAMON])
            .memory(10),
    );
    let draco = runner.place_on_field(0, DRACOMON, Some(0));
    runner.inject_trash(0, MAT_L5);
    runner.inject_trash(0, COREDRAMON_GREEN);

    // Digivolve (cost 2) and pay the <Draw 2> cost with the spare Dracomon.
    let mem_evo = runner.memory();
    digivolve_paying(&mut runner, COREDRAMON_BLUE, draco, 2);
    assert_eq!(mem_evo - runner.memory(), 2, "[Dracomon]-name route: Cost 2");
    for _ in 0..3 {
        let view = runner.pending_selection_view().expect("WD trash-to-draw prompt");
        if view.kind == SelectionKind::Hand {
            break;
        }
        accept(&mut runner);
    }
    pick_hand(&mut runner, DRACOMON);
    let _ = runner.auto_resolve();
    let mut trash = trash_ids(&runner, 0);
    trash.sort();
    let mut want = vec![COREDRAMON_GREEN.to_string(), DRACOMON.to_string(), MAT_L5.to_string()];
    want.sort();
    assert_eq!(trash, want, "Lv.5 + Lv.4 + Lv.3 [Dracomon]/[Examon]-text pieces in trash");

    let mem = runner.memory();
    assemble_breakdramon(&mut runner);
    assert_eq!(mem - runner.memory(), 7, "Assembly -5: 12 − 5");
    // [On Play]: decline the optional suspend; nothing to lock on an empty board.
    if runner
        .pending_selection_view()
        .is_some_and(|v| v.effect_choices.is_some())
    {
        runner.execute_branch(1).expect("don't suspend");
    }
    let _ = runner.auto_resolve();

    let bd = find_perm(&runner, 0, BREAKDRAMON);
    let mut under = stack_ids(&runner, bd);
    under.retain(|c| c != BREAKDRAMON);
    under.sort();
    assert_eq!(under, want, "the three trashed pieces went under Breakdramon");
    assert!(runner.game.players[0].trash.is_empty(), "trash emptied");
}

/// Unhappy path of Combo 3: without the Coredramon discard the Lv.3 piece is
/// missing, so Breakdramon is NOT reduced — it plays at its full cost of 12
/// (memory 10 → −2, passing the turn) with nothing placed under it.
#[test]
fn breakdramon_assembly_without_the_discarded_lv3_is_incomplete() {
    let mut runner = start(builder().hand(0, &[BREAKDRAMON]).memory(10));
    runner.inject_trash(0, MAT_L5);
    runner.inject_trash(0, COREDRAMON_GREEN);
    let slot = hand_index(&runner, 0, BREAKDRAMON) as u16;
    runner.game.decode_action(PLAY_HAND_START + slot, 0);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].trash.len(), 2, "no material left the trash");
    let bd = find_perm(&runner, 0, BREAKDRAMON);
    assert_eq!(stack_ids(&runner, bd), vec![BREAKDRAMON], "nothing placed under it");
    assert_ne!(runner.turn_player(), 0, "full cost 12 from 10 memory passed the turn");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — Breakdramon suspends its own Coredramon → OPT battle removal
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-044 Breakdramon (Assembly from EX13-008 / EX13-039 / Lv.5),
/// EX13-018 Coredramon (the suspended battler), ST1-02 Biyomon ×2 (opponent).
///
/// Expected mechanical outcome: Breakdramon's [On Play] "suspend up to 2
/// Digimon or Tamers" may pick YOUR OWN Coredramon; that suspension is "any of
/// your Digimon suspend", so Breakdramon's own [All Turns][Once Per Turn]
/// clause lets a [Dracomon]-text Digimon (Coredramon: its text names
/// [Dracomon]) battle an opponent Digimon. The 5000-DP Coredramon deletes a
/// 3000 Biyomon and survives; the 2-target "can't unsuspend" lock still lands
/// on the opponent. A second own-suspend the same turn does not re-trigger.
///
/// Sources: printed text `cards/ex13/EX13-044.json`; DCGO
/// `EX13/Green/EX13_044.cs` (SelectPermanentEffect Tap over either side;
/// OnTappedAnyone CanTriggerWhenPermanentSuspends(own Digimon) → IBattle);
/// `general_rule.pdf` battle resolution (§ battle, higher DP wins; a battle
/// caused by an effect is not an attack).
#[test]
fn breakdramon_suspends_own_coredramon_to_battle_and_delete() {
    let mut runner = start(builder().hand(0, &[BREAKDRAMON]).memory(10));
    runner.place_on_field(0, COREDRAMON_BLUE, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.inject_trash(0, MAT_L5);
    runner.inject_trash(0, COREDRAMON_GREEN);
    runner.inject_trash(0, DRACOMON);
    assemble_breakdramon(&mut runner);

    let core = find_perm(&runner, 0, COREDRAMON_BLUE);
    // Suspend #1 = own Coredramon; then stop ("up to 2").
    runner.execute_branch(0).expect("suspend");
    act(&mut runner, any_side(core));
    assert!(is_suspended(&runner, core), "own Coredramon suspended by Breakdramon");
    if runner
        .pending_selection_view()
        .is_some_and(|v| v.effect_choices.is_some())
    {
        runner.execute_branch(1).expect("stop after one");
    }

    // Lock 2 opponent Digimon, then the queued [All Turns] battle trigger.
    let mut battled = false;
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if view.prompt.contains("battle") && view.prompt.contains("your Digimon with") {
            let c = find_perm(&runner, 0, COREDRAMON_BLUE);
            act(&mut runner, one_side(c));
            let opp = PermanentHandle { player: 1, index: 0 };
            act(&mut runner, one_side(opp));
            battled = true;
            let _ = runner.auto_resolve();
            break;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();
    assert!(battled, "the own-suspend opened the battle; last view={}", view_dbg(&runner));

    assert_eq!(field_ids(&runner, 1), vec![BIYOMON], "5000 Coredramon deleted a 3000 Biyomon");
    assert!(field_ids(&runner, 0).contains(&COREDRAMON_BLUE.to_string()), "battler survives");
    let left = PermanentHandle { player: 1, index: 0 };
    assert!(
        runner.modifiers().has(left, ModifierType::CannotUnsuspend),
        "the surviving opponent Digimon is locked"
    );

    // [Once Per Turn]: suspending Breakdramon itself now does not re-open it.
    let bd = find_perm(&runner, 0, BREAKDRAMON);
    runner.game.suspend(bd);
    assert!(runner.pending_selection().is_none(), "OPT spent: {}", view_dbg(&runner));
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — Examon over Breakdramon: attack → inherited battle → win → free
//           Breakdramon from its own digivolution cards
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-045 Examon (top), EX13-044 Breakdramon (source, inherited
/// clause), EX13-018 / EX13-008 (lower sources), ST1-02 Biyomon ×2 (opponent).
///
/// Expected mechanical outcome: Examon attacking suspends it → Breakdramon's
/// INHERITED [All Turns][OPT] "when any of your Digimon suspend" lets Examon
/// (a [Examon]-text Digimon) battle an opponent Digimon; 15000 beats 3000, so
/// Examon "wins a battle" on your turn → its [OPT] plays the cost-12
/// [Dracomon]-text Breakdramon out of Examon's OWN digivolution cards without
/// paying the cost. Breakdramon lands as a second Digimon, memory unchanged
/// by that play.
///
/// Sources: printed text `cards/ex13/EX13-044.json` (inherited),
/// `EX13-045.json`; DCGO `EX13/Green/EX13_044.cs` (inherited OnTappedAnyone),
/// `EX13/Green/EX13_045.cs` (OnEndBattle CanTriggerWhenWinBattle →
/// PlayPermanentCards from digivolution cards, payCost: false).
#[test]
fn examon_attack_triggers_inherited_battle_whose_win_plays_breakdramon_free() {
    let mut runner = start(builder().memory(3));
    let examon = runner.place_stack(0, &[DRACOMON, COREDRAMON_BLUE, BREAKDRAMON, EXAMON]);
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    let mem0 = runner.memory();

    let target = PermanentHandle { player: 1, index: 0 };
    let _ = runner.attack_digimon(examon, target, false);

    let mut battled = false;
    let mut played_free = false;
    for _ in 0..24 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        if !battled && view.prompt.contains("your Digimon with") {
            let e = find_perm(&runner, 0, EXAMON);
            act(&mut runner, one_side(e));
            // Battle the OTHER Biyomon (index 1), not the attack target.
            act(&mut runner, one_side(PermanentHandle { player: 1, index: 1 }));
            battled = true;
            continue;
        }
        if !played_free && view.prompt.contains("Play or use 1 play or use cost 12") {
            // Union pick over hand + Examon's digivolution cards: Breakdramon.
            choose_breakdramon_from_union(&mut runner);
            played_free = true;
            continue;
        }
        if view.effect_choices.is_some() && played_free {
            // Freshly played Breakdramon's [On Play]: don't suspend.
            runner.execute_branch(1).expect("don't suspend");
            continue;
        }
        if view.is_optional && played_free {
            pass(&mut runner);
            continue;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert!(battled, "inherited Breakdramon battle opened; view={}", view_dbg(&runner));
    assert!(played_free, "Examon's win-battle play opened; view={}", view_dbg(&runner));
    let fields = field_ids(&runner, 0);
    assert!(fields.contains(&EXAMON.to_string()));
    assert!(fields.contains(&BREAKDRAMON.to_string()), "Breakdramon played from sources: {fields:?}");
    let ex = find_perm(&runner, 0, EXAMON);
    assert!(!stack_ids(&runner, ex).contains(&BREAKDRAMON.to_string()), "left the stack");
    assert_eq!(runner.memory(), mem0, "played without paying the cost");
}

/// Pick Breakdramon out of the [hand, Examon's digivolution cards] union
/// prompt (source slots encode as `encode_source_select(field, source)`).
fn choose_breakdramon_from_union(runner: &mut DebugRunner) {
    let ex = find_perm(runner, 0, EXAMON);
    let src = stack_ids(runner, ex)
        .iter()
        .position(|c| c == BREAKDRAMON)
        .expect("Breakdramon is one of Examon's digivolution cards");
    let a = encode_source_select(ex.index as u16, src as u16).expect("encodable");
    act(runner, a);
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 6 — Dracomon's inherited end-of-turn DNA: Breakdramon + Blue Lv.6 →
//           Examon, whose DNA [When Digivolving] attacks at end of turn
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-008 Dracomon (digivolution card under Breakdramon — the
/// inherited [End of Your Turn] DNA enabler), EX13-044 Breakdramon (green
/// Lv.6 half), ST2-10 Plesiomon (vanilla blue Lv.6 half), EX13-045 Examon (DNA
/// result in hand), ST1-02 Biyomon (opponent).
///
/// Expected mechanical outcome: at the end of your turn, Dracomon's inherited
/// effect lets its carrier (Breakdramon) and another of your Digimon DNA
/// digivolve into Examon from hand, paying Examon's printed DNA cost (Green
/// Lv.6 + Blue Lv.6: Cost 0). Examon lands with BOTH stacks under it, and
/// because it DNA digivolved its [When Digivolving] gives all your Digimon
/// +10000 DP and makes it attack — an end-of-turn attack the deck uses to
/// close. Memory is unchanged by the DNA.
///
/// Sources: printed text `cards/ex13/EX13-008.json` (inherited),
/// `EX13-045.json` (DNA row + [When Digivolving]); DCGO `EX13/Red/EX13_008.cs`
/// (OnEndTurn DNADigivolvePermanentsIntoHandOrTrashCard, payCost: true),
/// `EX13/Green/EX13_045.cs` (AddJogressConditionClass green6 + blue6 cost 0;
/// IsJogress → SelectAttackEffect + ChangeDigimonDPPlayerEffect);
/// `general_rule.pdf` DNA digivolution (§16 DNA Digivolve).
#[test]
fn dracomon_inherited_eot_dna_turns_breakdramon_and_blue_six_into_examon() {
    let mut runner = start(builder().hand(0, &[EXAMON]).memory(3));
    runner.place_stack(0, &[DRACOMON, COREDRAMON_BLUE, BREAKDRAMON]);
    let plesio = runner.place_on_field(0, PLESIOMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    let mem0 = runner.memory();
    assert_eq!(runner.security_count(1), 5);

    runner.end_turn();
    // Dracomon's inherited [End of Your Turn] "may" → partner → DNA result.
    let view = runner.pending_selection_view().expect("Dracomon's inherited EoT trigger");
    assert!(view.is_optional, "\"may DNA digivolve\"");
    accept(&mut runner);
    act(&mut runner, one_side(plesio));
    pick_hand(&mut runner, EXAMON);

    let ex = find_perm(&runner, 0, EXAMON);
    let mut stack = stack_ids(&runner, ex);
    stack.sort();
    let mut want: Vec<String> = [DRACOMON, COREDRAMON_BLUE, BREAKDRAMON, PLESIOMON, EXAMON]
        .iter()
        .map(|s| s.to_string())
        .collect();
    want.sort();
    assert_eq!(stack, want, "both stacks merged under Examon");
    assert_eq!(field_ids(&runner, 0), vec![EXAMON], "two Digimon became one");
    assert_eq!(runner.memory(), mem0, "Green Lv.6 + Blue Lv.6 DNA: Cost 0");
    // 15000 base + 10000 (DNA [When Digivolving]) + 2000 (EX13-018 Coredramon's
    // inherited [Your Turn] +2000 — still your turn during End of Turn).
    assert_eq!(
        runner.effective_dp(ex),
        Some(15_000 + 10_000 + 2_000),
        "DNA buff stacks with the inherited Coredramon bonus"
    );

    // DNA [When Digivolving]: "this Digimon attacks" (mandatory) — at security.
    let view = runner.pending_selection_view().expect("forced-attack target prompt");
    assert!(!view.is_optional, "the attack is mandatory");
    act(&mut runner, encode_attack(ex.index as u16, SECURITY_TARGET));
    // Examon suspending re-arms Breakdramon's INHERITED battle clause (Combo 5
    // covers accepting it); decline here so the attack reaches security.
    for _ in 0..8 {
        let Some(view) = runner.pending_selection_view() else {
            break;
        };
        assert!(view.is_optional, "only optional prompts remain; view={view:?}");
        pass(&mut runner);
    }
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.security_count(1),
        5 - 2,
        "end-of-turn attack checked 2 (＜Security A. +1＞)"
    );
    assert_eq!(field_ids(&runner, 1), vec![BIYOMON], "declined battle left the Biyomon");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 7 — Wingdramon / Groundramon as Lv.6 [Slayerdramon] / [Breakdramon]
//           for [Examon]'s DNA digivolution
//           (G-DNA-MATERIAL-TREATED-AS-FOR-TARGET)
// ═══════════════════════════════════════════════════════════════════════════

const WINGDRAMON: &str = "EX13-021";
const GROUNDRAMON: &str = "EX13-041";
/// BT20-045 Examon — <Blast DNA Digivolve ([Breakdramon] + [Slayerdramon])>.
const EXAMON_BT20: &str = "BT20-045";

/// The slice cards plus the two Lv.5 bridges and the Blast-DNA Examon, with
/// printed text copied for every real card (see the module doc).
fn dna_start(b: impl FnOnce(DebugRunnerBuilder) -> DebugRunnerBuilder) -> DebugRunner {
    let extra = [WINGDRAMON, GROUNDRAMON, EXAMON_BT20];
    let mut ids: Vec<&str> = REAL.to_vec();
    ids.extend(extra);
    let mut runner = b(dsl_builder(&ids)
        .deck(0, &[BIYOMON; 8])
        .deck(1, &[BIYOMON; 8])
        .security(1, &[BIYOMON; 5]))
    .start();
    runner.skip_mulligan();
    with_printed_text(&mut runner);
    for id in extra {
        let p = printed(id);
        for c in runner.game.card_data.iter_mut().filter(|c| c.card_id == id) {
            c.effect_text = p.effect_text.clone();
            c.inherited_text = p.inherited_text.clone();
            c.security_text = p.security_text.clone();
        }
    }
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    runner
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// Cards: EX13-021 Wingdramon, EX13-044 Breakdramon, EX13-045 Examon.
///
/// Expected: Wingdramon (printed Blue/Red **Lv.5**) is "also treated as Lv.6
/// [Slayerdramon] for [Examon]'s DNA digivolution", so it is the Blue Lv.6 half
/// of Examon's "Green Lv.6 + Blue Lv.6: Cost 0" recipe; Breakdramon (Green
/// Lv.6) is the other half. Main-phase DNA merges both stacks under Examon for
/// 0 memory and Examon's DNA [When Digivolving] fires (+10000 → 25000 DP).
///
/// Sources: `cards/ex13/EX13-021.json`, `EX13-045.json`; DCGO
/// `EX13/Blue/EX13_021.cs` (`AddJogressLevelsClass`, `EqualsCardName("Examon")`),
/// `EX13/Green/EX13_045.cs` (`AddJogressConditionClass` green Lv.6 + blue Lv.6).
#[test]
fn wingdramon_as_lv6_slayerdramon_dna_digivolves_with_breakdramon_into_examon() {
    let mut runner = dna_start(|b| b.hand(0, &[EXAMON]).memory(3));
    runner.place_on_field(0, WINGDRAMON, Some(0));
    runner.place_on_field(0, BREAKDRAMON, Some(0));
    runner.game.tick_declarative_effects();
    let mem0 = runner.memory();

    assert!(runner.game.initiate_dna_digivolve(0, 0), "Examon's DNA is legal");
    runner.game.resolve_selection(0, 0).expect("first material");
    runner.game.resolve_selection(0, 1).expect("second material");

    assert_eq!(field_ids(&runner, 0), vec![EXAMON], "two Digimon became one");
    let ex = find_perm(&runner, 0, EXAMON);
    assert_eq!(
        sorted(stack_ids(&runner, ex)),
        sorted(vec![WINGDRAMON.into(), BREAKDRAMON.into(), EXAMON.into()])
    );
    assert_eq!(runner.memory(), mem0, "DNA cost 0");
    assert_eq!(runner.effective_dp(ex), Some(25_000), "DNA [When Digivolving] +10000");
}

/// Cards: EX13-021 Wingdramon + EX13-041 Groundramon → EX13-045 Examon.
///
/// Expected: TWO printed Lv.5s satisfy the Lv.6 recipe — Groundramon (Green,
/// treated as Lv.6 [Breakdramon]) + Wingdramon (Blue, treated as Lv.6
/// [Slayerdramon]). Neither is Lv.6 for any other DNA result (per-card tests).
#[test]
fn wingdramon_and_groundramon_both_lv5_dna_digivolve_into_examon() {
    let mut runner = dna_start(|b| b.hand(0, &[EXAMON]).memory(3));
    runner.place_on_field(0, GROUNDRAMON, Some(0));
    runner.place_on_field(0, WINGDRAMON, Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.initiate_dna_digivolve(0, 0), "Lv.5 + Lv.5 → Examon");
    runner.game.resolve_selection(0, 0).expect("first material");
    runner.game.resolve_selection(0, 1).expect("second material");
    let ex = find_perm(&runner, 0, EXAMON);
    assert_eq!(
        sorted(stack_ids(&runner, ex)),
        sorted(vec![GROUNDRAMON.into(), WINGDRAMON.into(), EXAMON.into()])
    );
}

/// Cards: EX13-008 Dracomon (inherited [End of Your Turn] may-DNA), EX13-018
/// Coredramon, EX13-021 Wingdramon, EX13-044 Breakdramon, EX13-045 Examon.
///
/// Expected: the effect-initiated DNA (Dracomon's inherited clause enforces the
/// printed recipe) honors the treatment too: the Dracomon→Coredramon→Wingdramon
/// stack + Breakdramon DNA into Examon.
#[test]
fn dracomon_inherited_eot_dna_uses_wingdramon_as_the_blue_lv6() {
    let mut runner = dna_start(|b| b.hand(0, &[EXAMON]).memory(3));
    runner.place_stack(0, &[DRACOMON, COREDRAMON_BLUE, WINGDRAMON]);
    let brk = runner.place_on_field(0, BREAKDRAMON, Some(0));
    runner.game.tick_declarative_effects();

    runner.end_turn();
    let view = runner.pending_selection_view().expect("Dracomon's inherited EoT trigger");
    assert!(view.is_optional, "\"may DNA digivolve\"");
    accept(&mut runner);
    act(&mut runner, one_side(brk));
    pick_hand(&mut runner, EXAMON);

    let ex = find_perm(&runner, 0, EXAMON);
    assert_eq!(
        sorted(stack_ids(&runner, ex)),
        sorted(vec![
            DRACOMON.into(),
            COREDRAMON_BLUE.into(),
            WINGDRAMON.into(),
            BREAKDRAMON.into(),
            EXAMON.into()
        ]),
        "Wingdramon's stack + Breakdramon merged under Examon"
    );
}

/// Cards: EX13-021 Wingdramon (field), EX13-044 Breakdramon (hand), BT20-045
/// Examon (hand, <Blast DNA Digivolve ([Breakdramon] + [Slayerdramon])>).
///
/// Expected: when the opponent attacks Wingdramon, the Counter window offers
/// Examon's Blast DNA — Wingdramon is [Slayerdramon] for [Examon]'s DNA
/// digivolution — and resolving it stacks Wingdramon + Breakdramon under
/// BT20-045.
#[test]
fn wingdramon_as_slayerdramon_enables_bt20_examon_blast_dna_with_breakdramon() {
    let mut runner = dna_start(|b| b.hand(1, &[EXAMON_BT20, BREAKDRAMON]));
    let attacker = runner.place_on_field(0, BIYOMON, Some(0));
    let wing = runner.place_on_field(1, WINGDRAMON, Some(0));
    runner.game.tick_declarative_effects();

    let result = runner.attack_digimon(attacker, wing, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    let view = runner.pending_selection_view().expect("Counter window");
    assert!(
        view.valid_action_ids
            .contains(&digimon_engine::action::space::DNA_DIGIVOLVE_START),
        "BT20-045 Blast DNA is offered; view={view:?}"
    );
    runner
        .execute_action(1, digimon_engine::action::space::DNA_DIGIVOLVE_START)
        .expect("Blast DNA");
    runner.execute_action(1, 0).expect("Wingdramon as the field material");
    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("Breakdramon as the hand material");
    let ex = find_perm(&runner, 1, EXAMON_BT20);
    assert_eq!(
        sorted(stack_ids(&runner, ex)),
        sorted(vec![WINGDRAMON.into(), BREAKDRAMON.into(), EXAMON_BT20.into()])
    );
}
