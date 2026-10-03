//! EX13 "Guilmon / reptile" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/guilmon-reptile-ex13-model.md`. Two sub-engines:
//! red EX13-068 Takato Matsuki (memory floor + self-refresh + [Guilmon]
//! revival, feeding EX13-001 Gigimon's red-Tamer-played digivolve) and black
//! EX13-048 Kotemon (Knightmon tutor + inherited sacrifice-shield whose
//! sacrificed EX13-058 Knightmon refills the board via [On Deletion]).
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{48,68}.rs`;
//! this file asserts only cross-card SYSTEM facts.
//!
//! EX13-007 Guilmon is BLOCKED (`validated_cards_dsl.json`,
//! G-ENGINE-DP-DELETION-MAX-MODIFIER), so no combo here names it; the
//! [Guilmon] revival target is the implemented EX4-006 Guilmon.
//!
//! Real DSL cards fill every role — no synthetic cards. Vanilla starter
//! Digimon: ST1-02 Biyomon (3000 DP filler / deletable opponent), ST1-04
//! Dracomon (4000 DP opponent, outside every ≤3000 filter).
//!
//! No DCGO C# exists for EX13-048 / EX13-068; printed text (official Bandai DB
//! bundles `data/card_bundles/EX13-0xx.md`) + `general_rule.pdf` govern.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{
    ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, SEL_REVEAL_START,
    TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const TAKATO: &str = "EX13-068";
const KOTEMON: &str = "EX13-048";
/// Red Gigimon egg — inherited "when your red Tamer is played" digivolve.
const GIGIMON: &str = "EX13-001";
/// Implemented [Guilmon] (the EX13-007 printing is BLOCKED).
const GUILMON: &str = "EX4-006";
/// [Growlmon] that digivolves from a [Guilmon]-named Digimon (cost 2).
const GROWLMON: &str = "EX3-057";
const KNIGHTMON: &str = "EX13-058";
const LORDKNIGHTMON: &str = "EX13-064";
/// Vanilla Lv.3, 3000 DP.
const BIYOMON: &str = "ST1-02";
/// Vanilla Lv.3, 4000 DP.
const DRACOMON: &str = "ST1-04";

const PRINTED: &[&str] = &[KOTEMON, KNIGHTMON, LORDKNIGHTMON, TAKATO, GIGIMON];
const REAL: &[&str] = &[
    TAKATO, KOTEMON, GIGIMON, GUILMON, GROWLMON, KNIGHTMON, LORDKNIGHTMON, BIYOMON, DRACOMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// Production `CardData` for `id`, parsed from its committed per-card JSON.
fn printed(id: &str) -> CardData {
    let set = id.split('-').next().unwrap().to_lowercase();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join(set)
        .join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let wrapped = format!("{{\"{id}\": {raw}}}");
    let mut map: HashMap<String, CardData> =
        CardData::load_from_str(&wrapped).unwrap_or_else(|e| panic!("{id}: {e}"));
    map.remove(id).unwrap_or_else(|| panic!("{id} parsed"))
}

/// DSL-loaded cards carry empty printed text; "[Knightmon] in its text"
/// filters scan it in production, so copy the printed text in.
fn with_printed_text(runner: &mut DebugRunner) {
    for id in PRINTED {
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
        .deck(1, &[BIYOMON; 8])
        .security(0, &[BIYOMON; 3])
        .security(1, &[BIYOMON; 3])
}

fn start(b: DebugRunnerBuilder) -> DebugRunner {
    let mut runner = b.start();
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

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

fn strs(v: &[&str]) -> Vec<String> {
    sorted(v.iter().map(|s| s.to_string()).collect())
}

fn field_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].hand)
}

fn trash_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].trash)
}

fn count(v: &[String], id: &str) -> usize {
    v.iter().filter(|x| *x == id).count()
}

fn source_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    ids_of(
        runner,
        &runner.game.players[h.player as usize].battle_area[h.index as usize].card_sources,
    )
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

fn push_to_trash(runner: &mut DebugRunner, p: u8, id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == id)
        .expect("card registered");
    let card_index = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_index, p, card_index));
}

fn view_kind(runner: &DebugRunner) -> Option<SelectionKind> {
    runner.pending_selection_view().map(|v| v.kind)
}

/// Resolve any TriggerOrder / OrderedPermutation prompt by taking its first
/// non-PASS option.
fn settle_ordering(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        match view.kind {
            SelectionKind::TriggerOrder | SelectionKind::OrderedPermutation { .. }
                if !view.is_optional =>
            {
                runner
                    .execute_action(view.selecting_player, view.valid_action_ids[0])
                    .expect("order");
            }
            _ => return,
        }
    }
}

/// Hand-card ids offered by a pending Hand prompt.
fn offered_hand(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand, "{view:?}");
    let hand = hand_ids(runner, view.selecting_player);
    sorted(
        view.valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .filter_map(|&a| {
                let slot = if a >= HAND_EFFECT_START {
                    a - HAND_EFFECT_START
                } else {
                    a.checked_sub(PLAY_HAND_START)?
                };
                hand.get(slot as usize).cloned()
            })
            .collect(),
    )
}

fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand, "{view:?}");
    let slot = hand_ids(runner, view.selecting_player)
        .iter()
        .position(|h| h == id)
        .unwrap_or_else(|| panic!("{id} in hand")) as u16;
    let a = [HAND_EFFECT_START + slot, PLAY_HAND_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    runner.execute_action(view.selecting_player, a).expect("pick hand");
}

fn offered_trash(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("trash prompt");
    assert_eq!(view.kind, SelectionKind::Trash, "{view:?}");
    let trash = trash_ids(runner, view.selecting_player);
    sorted(
        view.valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .filter_map(|&a| a.checked_sub(TRASH_EFFECT_START))
            .filter_map(|i| trash.get(i as usize).cloned())
            .collect(),
    )
}

fn pick_trash(runner: &mut DebugRunner, id: &str) {
    let idx = trash_ids(runner, 0)
        .iter()
        .position(|t| t == id)
        .unwrap_or_else(|| panic!("{id} in trash")) as u16;
    runner
        .execute_action(0, TRASH_EFFECT_START + idx)
        .expect("pick trash");
}

/// Field-pick options (by top-card id) of the pending OwnField/OppField prompt.
fn offered_field(runner: &DebugRunner, kind: SelectionKind, p: u8) -> Vec<String> {
    let view = runner.pending_selection_view().expect("field prompt pending");
    assert_eq!(view.kind, kind, "{view:?}");
    let field = field_ids(runner, p);
    sorted(
        view.valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .filter_map(|&a| a.checked_sub(ATTACK_START))
            .filter_map(|i| field.get((i % TARGETS_PER_ATTACKER) as usize).cloned())
            .collect(),
    )
}

fn pick_handle(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(ATTACK_START)
                .is_some_and(|o| (o % TARGETS_PER_ATTACKER) as u8 == h.index)
        })
        .unwrap_or_else(|| panic!("{h:?} not offered: {view:?}"));
    runner.execute_action(view.selecting_player, a).expect("pick field");
}

fn offered_reveal(runner: &DebugRunner) -> Vec<String> {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    sorted(
        view.valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
            .filter_map(|i| runner.game.revealed_cards.get(i as usize))
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect(),
    )
}

fn pick_revealed(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("reveal prompt pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| runner.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&runner.game.card_data) == id)
        })
        .unwrap_or_else(|| panic!("{id} must be a legal reveal pick: {view:?}"));
    runner.execute_action(view.selecting_player, a).expect("pick revealed");
}

/// Deck stacked bottom-first: `top` (index 0 = first revealed/drawn) sits on
/// top of `filler_below` copies of Biyomon.
fn deck_with_top(top: &[&'static str], filler_below: usize) -> Vec<&'static str> {
    let mut deck: Vec<&'static str> = vec![BIYOMON; filler_below];
    deck.extend(top.iter().rev().copied());
    deck
}

// ═══════════════════════════════════════════════════════════════════════════
// C1 — Takato reload: memory floor + Tamer refresh + Guilmon revival
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-068 Takato Matsuki ×2 (one on field, one in hand), EX4-006
/// Guilmon (trash).
///
/// Expected mechanical outcome (driven through the REAL turn flow): we start
/// our turn with 1 memory → Takato's [Start of Your Turn] sets it to 3. At
/// [Start of Your Main Phase] we pay the cost (Takato → deck BOTTOM), play the
/// hand Takato free, and — with no Digimon, checked after the Tamer play —
/// play Guilmon from the trash free. Field: [Takato, Guilmon], memory still 3,
/// trash empty, the hand Takato gone. EX4-006's own [On Play] fires but its
/// "both trashes ≥20" gate is unmet, so no <Rush>.
///
/// Sources: `data/card_bundles/EX13-068.md` (+ Q&A), `EX4-006.yaml`.
#[test]
fn c1_takato_reload_sets_memory_refreshes_tamer_and_revives_guilmon() {
    let mut runner = start(builder().deck(0, &[BIYOMON; 8]).hand(0, &[TAKATO]).memory(3));
    runner.place_on_field(0, TAKATO, Some(0));
    push_to_trash(&mut runner, 0, GUILMON);

    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.turn_player(), 1);
    // The opponent passes leaving us 1 memory (≤ 2) at the start of our turn.
    runner.game.memory = 1;
    runner.end_turn();

    // Our turn: the SoT memory set resolves (mandatory); the SoMP clause parks
    // its optional accept.
    let view = runner.pending_selection_view().expect("SoMP optional offer");
    assert_eq!(runner.game.turn_player(), 0, "back on our turn");
    assert!(view.is_optional, "\"By returning…, you may\": {view:?}");
    assert_eq!(runner.memory(), 3, "[Start of Your Turn] set 1 → 3 before the main phase");

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    runner.accept_optional_trigger().expect("pay the return cost");
    assert!(field_ids(&runner, 0).is_empty(), "the cost moved Takato off the field");
    assert_eq!(runner.deck_size(0), deck_before + 1);
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        TAKATO,
        "returned to the BOTTOM of the deck"
    );

    assert_eq!(offered_hand(&runner), strs(&[TAKATO]));
    pick_hand(&mut runner, TAKATO);
    assert_eq!(offered_trash(&runner), strs(&[GUILMON]), "no Digimon → Guilmon offered");
    pick_trash(&mut runner, GUILMON);
    settle_ordering(&mut runner);
    let _ = runner.auto_resolve();

    assert_eq!(sorted(field_ids(&runner, 0)), strs(&[TAKATO, GUILMON]));
    assert_eq!(runner.hand_size(0), hand_before - 1, "the hand Takato was played");
    assert!(trash_ids(&runner, 0).is_empty(), "Guilmon left the trash");
    assert_eq!(runner.memory(), 3, "both plays were free");
    let guilmon = find_perm(&runner, 0, GUILMON);
    assert!(
        !runner.game.has_keyword(guilmon, Keyword::Rush),
        "EX4-006 [On Play] gate (≥20 trash) is unmet"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// C2 — Takato refresh feeds Gigimon: red Tamer played → cost −2 digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-068 ×2, EX13-001 Gigimon (source under EX4-006 Guilmon),
/// EX3-057 Growlmon (hand), EX4-006 Guilmon (trash, revival target).
/// Opponent: ST1-02 Biyomon (3000 DP), ST1-04 Dracomon (4000 DP).
///
/// Expected mechanical outcome: the refreshed Takato is a red Tamer played on
/// our turn, so Gigimon's inherited [Your Turn][OPT] trigger offers Growlmon
/// from hand; the [Guilmon] digivolve costs 2 − 2 = 0 (memory unchanged).
/// Growlmon's [When Digivolving] offers only the ≤3000 DP Biyomon and deletes
/// it (so the "else both mill 2" branch is skipped); the digivolve draws 1. The Guilmon-revival
/// branch never opens — we control a Digimon.
///
/// Sources: `EX13-001.yaml`, `data/card_bundles/EX13-068.md`, `EX3-057.yaml`.
#[test]
fn c2_takato_refresh_triggers_gigimon_cost_reduced_growlmon_digivolve() {
    let mut runner = start(
        builder()
            .deck(0, &[BIYOMON; 8])
            .hand(0, &[TAKATO, GROWLMON])
            .memory(3),
    );
    let takato = runner.place_on_field(0, TAKATO, Some(0));
    runner.place_stack(0, &[GIGIMON, GUILMON]);
    push_to_trash(&mut runner, 0, GUILMON);
    runner.place_on_field(1, BIYOMON, Some(1));
    runner.place_on_field(1, DRACOMON, Some(1));
    let before = snapshot(&runner);

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(takato));
    runner.game.drain_effect_queue();
    runner.accept_optional_trigger().expect("pay the return cost");
    pick_hand(&mut runner, TAKATO);
    settle_ordering(&mut runner);

    // Gigimon's trigger (optional digivolve) — not the Guilmon trash prompt.
    assert_ne!(view_kind(&runner), Some(SelectionKind::Trash), "we have a Digimon → no revival");
    if runner
        .pending_selection_view()
        .is_some_and(|v| v.kind != SelectionKind::Hand && v.is_optional)
    {
        runner.accept_optional_trigger().expect("accept Gigimon trigger");
    }
    assert_eq!(offered_hand(&runner), strs(&[GROWLMON]), "only the [Growlmon] card");
    pick_hand(&mut runner, GROWLMON);
    settle_ordering(&mut runner);

    // Growlmon [When Digivolving]: delete ≤3000 DP.
    if view_kind(&runner) == Some(SelectionKind::OppField) {
        assert_eq!(
            offered_field(&runner, SelectionKind::OppField, 1),
            strs(&[BIYOMON]),
            "Dracomon (4000) is outside the ≤3000 filter"
        );
        let biyo = find_perm(&runner, 1, BIYOMON);
        pick_handle(&mut runner, biyo);
    }
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    let growl = find_perm(&runner, 0, GROWLMON);
    // (Takato left the field, so battle-area indices shifted — compare sources.)
    assert_eq!(
        source_ids(&runner, growl),
        vec![GIGIMON, GUILMON, GROWLMON],
        "Growlmon sits on the Gigimon→Guilmon stack"
    );
    assert_eq!(sorted(field_ids(&runner, 0)), strs(&[TAKATO, GROWLMON]));
    assert_eq!(field_ids(&runner, 1), vec![DRACOMON.to_string()], "Biyomon deleted");
    assert_eq!(runner.memory(), 3, "Takato free + digivolve 2 − 2 = 0");
    // Takato + Growlmon left the hand; the digivolution bonus drew 1 (the
    // deck is all Biyomon) — the draw applies to effect digivolves too.
    assert_eq!(after.hand[0], before.hand[0] - 2 + 1);
    assert_eq!(hand_ids(&runner, 0), vec![BIYOMON.to_string()], "digivolve draw");
    assert_eq!(trash_ids(&runner, 0), vec![GUILMON.to_string()], "no revival, no mill");
    assert_eq!(after.deck[1], before.deck[1], "deletion happened → no 'both mill 2'");
}

// ═══════════════════════════════════════════════════════════════════════════
// C3 — Kotemon tutors the real Knightmon package
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-048 Kotemon, EX13-064 LordKnightmon, EX13-058 Knightmon,
/// ST1-04 Dracomon (non-match).
///
/// Expected mechanical outcome: revealing [LordKnightmon, Knightmon, Dracomon]
/// — the "[Knightmon] in its text" bucket offers BOTH knights (a name is part
/// of the text, Q&A); after taking LordKnightmon the name bucket offers only
/// Knightmon. Hand +2 knights, Dracomon to the deck bottom, memory −3.
///
/// Sources: `data/card_bundles/EX13-048.md` (+ Q&A).
#[test]
fn c3_kotemon_tutors_lordknightmon_and_knightmon() {
    let deck = deck_with_top(&[LORDKNIGHTMON, KNIGHTMON, DRACOMON], 4);
    let mut runner = start(builder().deck(0, &deck).hand(0, &[KOTEMON]).memory(5));
    let before = snapshot(&runner);

    runner.play(0, 0).expect("Kotemon played");
    settle_ordering(&mut runner);
    assert_eq!(offered_reveal(&runner), strs(&[LORDKNIGHTMON, KNIGHTMON]));
    pick_revealed(&mut runner, LORDKNIGHTMON);
    assert_eq!(offered_reveal(&runner), strs(&[KNIGHTMON]), "name bucket, de-duplicated");
    pick_revealed(&mut runner, KNIGHTMON);
    let _ = runner.auto_resolve();

    assert_eq!(sorted(hand_ids(&runner, 0)), strs(&[LORDKNIGHTMON, KNIGHTMON]));
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        DRACOMON,
        "the rest to the bottom"
    );
    assert_eq!(runner.deck_size(0), before.deck[0] - 2);
    assert_eq!(runner.memory(), 2, "Kotemon cost 3");
}

// ═══════════════════════════════════════════════════════════════════════════
// C4 — Kotemon shield: sacrificed Knightmon's [On Deletion] replays Kotemon
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-058 Knightmon atop EX13-048 Kotemon (protected), EX13-058
/// Knightmon #2 (sacrifice), EX13-048 Kotemon #2 (hand), EX13-064
/// LordKnightmon (deck top, Kotemon #2's reveal hit).
///
/// Expected mechanical outcome: an opponent's effect would delete the
/// Knightmon-on-Kotemon stack → Kotemon's inherited replacement is offered;
/// its cost offers ONLY Knightmon #2 (never the carrier). Paying keeps the
/// stack. Knightmon #2's [On Deletion] offers Kotemon #2 (cost 3 ≤ 4,
/// [Knightmon] in text) from hand and plays it free; Kotemon #2's [On Play]
/// tutors LordKnightmon. Net: field count unchanged (−Knightmon #2,
/// +Kotemon #2), memory unchanged, LordKnightmon in hand.
///
/// Sources: `data/card_bundles/EX13-048.md`, `EX13-058.yaml`.
#[test]
fn c4_kotemon_shield_sacrificed_knightmon_replays_kotemon_which_tutors() {
    let deck = deck_with_top(&[LORDKNIGHTMON, BIYOMON, BIYOMON], 4);
    let mut runner = start(builder().deck(0, &deck).hand(0, &[KOTEMON]).memory(3));
    let shielded = runner.place_stack(0, &[KOTEMON, KNIGHTMON]);
    let sacrifice = runner.place_on_field(0, KNIGHTMON, Some(0));
    let before = snapshot(&runner);

    runner
        .game
        .delete_permanent_with_cause(shielded, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("replacement offer");
    assert_eq!(view.kind, SelectionKind::Replacement, "{view:?}");
    runner.accept_optional_trigger().expect("accept leave-prevention");
    assert_eq!(
        offered_field(&runner, SelectionKind::OwnField, 0),
        strs(&[KNIGHTMON]),
        "only the OTHER Knightmon is a legal cost"
    );
    pick_handle(&mut runner, sacrifice);
    settle_ordering(&mut runner);

    // Knightmon #2 [On Deletion]: may play a [Knightmon]-text cost ≤4 card.
    assert_eq!(offered_hand(&runner), strs(&[KOTEMON]));
    assert!(runner.pending_selection_view().unwrap().is_optional, "\"you may play\"");
    pick_hand(&mut runner, KOTEMON);
    settle_ordering(&mut runner);

    // Kotemon #2 [On Play] reveal.
    assert_eq!(offered_reveal(&runner), strs(&[LORDKNIGHTMON]));
    pick_revealed(&mut runner, LORDKNIGHTMON);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    assert_eq!(sorted(field_ids(&runner, 0)), strs(&[KNIGHTMON, KOTEMON]));
    let kept = find_perm(&runner, 0, KNIGHTMON);
    assert_eq!(source_ids(&runner, kept), vec![KOTEMON, KNIGHTMON], "the shielded stack stayed");
    assert_eq!(after.field[0], before.field[0], "−sacrifice, +Kotemon #2");
    assert_eq!(hand_ids(&runner, 0), vec![LORDKNIGHTMON.to_string()]);
    assert_eq!(count(&trash_ids(&runner, 0), KNIGHTMON), 1, "the sacrifice");
    assert_eq!(runner.memory(), 3, "Kotemon #2 was played free");
}

// ═══════════════════════════════════════════════════════════════════════════
// C5 — Lone Knightmon falls; its own [On Deletion] still replays Kotemon
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-058 Knightmon atop EX13-048 Kotemon, EX13-048 Kotemon #2 (hand),
/// ST1-02 Biyomon (non-[Knightmon] ally).
///
/// Expected mechanical outcome: with no OTHER [Knightmon]-text Digimon (the
/// Biyomon ally doesn't qualify) the shield isn't offered; the stack is
/// deleted (Knightmon + Kotemon to trash). Knightmon's [On Deletion] still
/// plays Kotemon #2 from hand free — the line keeps a body when the shield
/// can't be paid.
///
/// Sources: `data/card_bundles/EX13-048.md`, `EX13-058.yaml`.
#[test]
fn c5_lone_knightmon_unshielded_but_on_deletion_replays_kotemon() {
    let deck = deck_with_top(&[BIYOMON, BIYOMON, BIYOMON], 4);
    let mut runner = start(builder().deck(0, &deck).hand(0, &[KOTEMON]).memory(3));
    let lone = runner.place_stack(0, &[KOTEMON, KNIGHTMON]);
    runner.place_on_field(0, BIYOMON, Some(0));

    runner
        .game
        .delete_permanent_with_cause(lone, ReplacementCause::OpponentEffect);
    settle_ordering(&mut runner);
    assert_ne!(
        view_kind(&runner),
        Some(SelectionKind::Replacement),
        "no other [Knightmon]-text Digimon → no shield offer"
    );
    assert_eq!(offered_hand(&runner), strs(&[KOTEMON]), "Knightmon [On Deletion]");
    pick_hand(&mut runner, KOTEMON);
    let _ = runner.auto_resolve();

    assert_eq!(sorted(field_ids(&runner, 0)), strs(&[BIYOMON, KOTEMON]));
    let trash = trash_ids(&runner, 0);
    assert_eq!(count(&trash, KNIGHTMON), 1, "{trash:?}");
    assert_eq!(count(&trash, KOTEMON), 1, "the source Kotemon went with it: {trash:?}");
    assert!(hand_ids(&runner, 0).is_empty());
    assert_eq!(runner.memory(), 3, "free play");
}
