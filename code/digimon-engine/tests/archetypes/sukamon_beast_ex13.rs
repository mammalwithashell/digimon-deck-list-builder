//! EX13 "Sukamon / beast" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/sukamon-beast-ex13-model.md`. Two lines:
//! yellow Chuumon (EX13-027) → Sukamon (EX13-028) — a recursive <Blocker> wall
//! whose inherited leave-prevention is paid by deleting ANOTHER Sukamon, whose
//! [On Deletion] replays Chuumon, whose [On Play] re-tutors Sukamon — and green
//! Salamon (EX13-038) → Mikemon (EX13-040), a suspend-lock tempo line with an
//! inherited "+1000 DP to your suspended Digimon".
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{27,28,38,40}.rs`;
//! this file asserts only the cross-card SYSTEM facts.
//!
//! EX13-031 KingSukamon is BLOCKED (`validated_cards_dsl.json`), so no combo
//! here names it.
//!
//! Real DSL cards fill every role — no synthetic cards. Vanilla starter
//! Digimon are fillers / opponents: ST1-02 Biyomon (filler, not [Beast]),
//! ST1-04 Dracomon (vanilla Lv.3), ST1-05 Birdramon (vanilla Lv.4, 5000 DP).
//! EX13-043 Leopardmon appears only as a Salamon reveal hit (never played).
//!
//! No DCGO C# exists for any of the four cards; printed text (official Bandai
//! DB bundles `data/card_bundles/EX13-0xx.md`) + `general_rule.pdf` govern.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, SEL_REVEAL_START, TARGETS_PER_ATTACKER,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const CHUUMON: &str = "EX13-027";
const SUKAMON: &str = "EX13-028";
const SALAMON: &str = "EX13-038";
const MIKEMON: &str = "EX13-040";
/// Salamon's [Leopardmon]-text reveal hit (never played here).
const LEOPARDMON: &str = "EX13-043";
/// Vanilla filler (Bird trait — matches no filter in this slice).
const BIYOMON: &str = "ST1-02";
/// Vanilla Lv.3 opponent.
const DRACOMON: &str = "ST1-04";
/// Vanilla Lv.4, 5000 DP opponent attacker / lock target.
const BIRDRAMON: &str = "ST1-05";

const PRINTED: &[&str] = &[CHUUMON, SUKAMON, SALAMON, MIKEMON, LEOPARDMON];
const REAL: &[&str] = &[
    CHUUMON, SUKAMON, SALAMON, MIKEMON, LEOPARDMON, BIYOMON, DRACOMON, BIRDRAMON,
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

/// DSL-loaded cards carry empty printed text; Salamon's "[Leopardmon] in its
/// text" filter scans it in production, so copy the printed text in.
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

fn field_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

fn strs(v: &[&str]) -> Vec<String> {
    sorted(v.iter().map(|s| s.to_string()).collect())
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

fn hand_index(runner: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(runner, p)
        .iter()
        .position(|h| h == id)
        .unwrap_or_else(|| panic!("{id} must be in player {p}'s hand: {:?}", hand_ids(runner, p)))
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn view_kind(runner: &DebugRunner) -> Option<SelectionKind> {
    runner.pending_selection_view().map(|v| v.kind)
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

/// Pick the permanent `h` from the pending field prompt (target / cost / block).
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

/// Ids currently offered by a pending reveal prompt.
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

/// Pick `id` out of the reveal pool.
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

fn is_reveal_prompt(runner: &DebugRunner) -> bool {
    runner.pending_selection_view().is_some_and(|v| {
        v.valid_action_ids
            .iter()
            .any(|&a| a != PASS && a >= SEL_REVEAL_START && a < SEL_REVEAL_START + 16)
    }) && !runner.game.revealed_cards.is_empty()
}

/// Resolve any TriggerOrder / OrderedPermutation prompt by taking its first option.
fn settle_ordering(runner: &mut DebugRunner) {
    while let Some(view) = runner.pending_selection_view() {
        match view.kind {
            SelectionKind::TriggerOrder | SelectionKind::OrderedPermutation { .. } => {
                runner
                    .execute_action(view.selecting_player, view.valid_action_ids[0])
                    .expect("order");
            }
            _ => return,
        }
    }
}

/// Deck for the yellow chain (bottom-first; last = top). The first reveal
/// (Sukamon's [On Deletion]) sees CHUUMON, BIYOMON, BIYOMON; the second
/// (Chuumon's [On Play]) sees BIYOMON, SUKAMON, SUKAMON.
const YELLOW_CHAIN_DECK: &[&str] = &[
    BIYOMON, BIYOMON, BIYOMON, SUKAMON, SUKAMON, BIYOMON, BIYOMON, BIYOMON, CHUUMON,
];

/// Drive the Sukamon [On Deletion] → free Chuumon → Chuumon [On Play] tail
/// that C1 and C2 share. Asserts every prompt's offered set along the way.
fn drive_on_deletion_replays_chuumon_which_retutors_sukamon(runner: &mut DebugRunner) {
    settle_ordering(runner);
    assert!(is_reveal_prompt(runner), "Sukamon [On Deletion] reveal prompt");
    assert!(
        runner.pending_selection_view().unwrap().is_optional,
        "\"you may play\""
    );
    assert_eq!(offered_reveal(runner), strs(&[CHUUMON]), "only the cost-3 Chuumon qualifies");
    pick_revealed(runner, CHUUMON);
    settle_ordering(runner);

    // Chuumon's [On Play]: hand bucket then trash bucket, both Sukamon.
    assert!(is_reveal_prompt(runner), "Chuumon [On Play] hand-bucket prompt");
    assert_eq!(offered_reveal(runner), strs(&[SUKAMON, SUKAMON]));
    pick_revealed(runner, SUKAMON);
    if is_reveal_prompt(runner) {
        assert_eq!(offered_reveal(runner), strs(&[SUKAMON]), "trash bucket: the other Sukamon");
        pick_revealed(runner, SUKAMON);
    }
    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// C1 — Blocker wall: protected Sukamon survives, the sacrificed Sukamon refills
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-028 Sukamon ×2 (one atop EX13-027 Chuumon), EX13-027 Chuumon
/// (deck), opponent ST1-05 Birdramon (5000 DP).
///
/// Expected mechanical outcome: on the opponent's turn Birdramon attacks;
/// Sukamon (<Blocker>, 16-4) blocks and loses the battle 2000 vs 5000. A battle
/// deletion is "other than by your effects", so Chuumon's inherited [All
/// Turns][OPT] replacement is offered; its cost offers ONLY the other Sukamon
/// (never the carrier itself). Paying it keeps the blocker on the field. The
/// sacrificed Sukamon's [On Deletion] reveals Chuumon and plays it free, whose
/// [On Play] adds one Sukamon to the hand and trashes the other.
/// Net: own field count unchanged (−Sukamon#2, +Chuumon), hand +1, security
/// untouched (the attack was blocked), Birdramon survives.
///
/// Sources: `data/card_bundles/EX13-027.md`, `EX13-028.md`; `general_rule.pdf`
/// 16-4 (<Blocker>).
#[test]
fn c1_blocker_wall_protected_sukamon_survives_and_sacrifice_refills_board() {
    let mut runner = start(builder().deck(0, YELLOW_CHAIN_DECK));
    let wall = runner.place_stack(0, &[CHUUMON, SUKAMON]);
    runner.place_on_field(0, SUKAMON, Some(0));
    let bird = runner.place_on_field(1, BIRDRAMON, Some(0));
    assert!(runner.game.has_keyword(wall, Keyword::Blocker));

    runner.end_turn();
    let _ = runner.auto_resolve();
    let before = snapshot(&runner);
    let mem_before = runner.memory();

    let _ = runner.attack_player(bird, 0, false);
    assert_eq!(
        offered_field(&runner, SelectionKind::OwnField, 0),
        strs(&[SUKAMON, SUKAMON]),
        "both unsuspended Sukamon are <Blocker> candidates"
    );
    pick_handle(&mut runner, wall);
    settle_ordering(&mut runner);

    // Battle loss → Chuumon's inherited replacement is offered.
    let view = runner.pending_selection_view().expect("replacement offer");
    assert_eq!(view.kind, SelectionKind::Replacement, "{view:?}");
    assert_eq!(view.selecting_player, 0);
    runner.accept_optional_trigger().expect("accept leave-prevention");
    assert_eq!(
        offered_field(&runner, SelectionKind::OwnField, 0),
        strs(&[SUKAMON]),
        "cost offers only the OTHER Sukamon"
    );
    let other = PermanentHandle {
        player: 0,
        index: if wall.index == 0 { 1 } else { 0 },
    };
    pick_handle(&mut runner, other);

    drive_on_deletion_replays_chuumon_which_retutors_sukamon(&mut runner);

    let after = snapshot(&runner);
    assert_eq!(field_ids(&runner, 0), vec![SUKAMON.to_string(), CHUUMON.to_string()]);
    let wall = find_perm(&runner, 0, SUKAMON);
    assert!(
        source_ids(&runner, wall).contains(&CHUUMON.to_string()),
        "the protected blocker is the original Sukamon-on-Chuumon stack"
    );
    assert!(suspended(&runner, wall), "it stayed on the field (suspended by the block)");
    assert_eq!(field_ids(&runner, 1), vec![BIRDRAMON.to_string()], "Birdramon won the battle");
    assert_eq!(after.security[0], before.security[0], "blocked: no security check");
    assert_eq!(after.hand[0], before.hand[0] + 1, "Chuumon tutored a Sukamon");
    assert_eq!(count(&hand_ids(&runner, 0), SUKAMON), 1);
    // Trash: sacrificed Sukamon#2, 2 Biyomon from the first reveal, the trashed Sukamon.
    let trash = trash_ids(&runner, 0);
    assert_eq!(count(&trash, SUKAMON), 2, "{trash:?}");
    assert_eq!(count(&trash, BIYOMON), 2, "{trash:?}");
    assert_eq!(runner.memory(), mem_before, "the replayed Chuumon was free");
}

// ═══════════════════════════════════════════════════════════════════════════
// C2 — Lone Sukamon falls: no replacement, [On Deletion] still refills (C1's unhappy path)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-028 Sukamon atop EX13-027 Chuumon, EX13-027 Chuumon (deck).
///
/// Expected mechanical outcome: with NO other [Sukamon]-named Digimon the
/// inherited replacement is not offered (its cost can't be paid). The blocker
/// is deleted with its Chuumon source; Sukamon's [On Deletion] then plays
/// Chuumon free and Chuumon re-tutors a Sukamon — the line keeps a body and
/// gains a card even when the wall breaks.
///
/// Sources: `EX13-027.md`, `EX13-028.md`; `general_rule.pdf` 16-4.
#[test]
fn c2_lone_sukamon_is_not_protected_but_on_deletion_replays_chuumon() {
    let mut runner = start(builder().deck(0, YELLOW_CHAIN_DECK));
    let wall = runner.place_stack(0, &[CHUUMON, SUKAMON]);
    let bird = runner.place_on_field(1, BIRDRAMON, Some(0));

    runner.end_turn();
    let _ = runner.auto_resolve();
    let before = snapshot(&runner);

    let _ = runner.attack_player(bird, 0, false);
    pick_handle(&mut runner, wall);
    settle_ordering(&mut runner);
    assert_ne!(
        view_kind(&runner),
        Some(SelectionKind::Replacement),
        "no other Sukamon → no leave-prevention offer"
    );

    drive_on_deletion_replays_chuumon_which_retutors_sukamon(&mut runner);

    let after = snapshot(&runner);
    assert_eq!(field_ids(&runner, 0), vec![CHUUMON.to_string()], "Chuumon replaced the wall");
    assert_eq!(after.hand[0], before.hand[0] + 1);
    let trash = trash_ids(&runner, 0);
    // Deleted stack (Sukamon + Chuumon source) + 2 Biyomon + trashed Sukamon.
    assert_eq!(count(&trash, SUKAMON), 2, "{trash:?}");
    assert_eq!(count(&trash, CHUUMON), 1, "{trash:?}");
    assert_eq!(after.security[0], before.security[0], "still blocked");
}

// ═══════════════════════════════════════════════════════════════════════════
// C3 — Chuumon tutors its own digivolution; the stack inherits the protection
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-027 Chuumon, EX13-028 Sukamon ×3 (2 revealed, 1 ally).
///
/// Expected mechanical outcome: playing Chuumon (cost 3) reveals 3 — one
/// Sukamon to hand, the other to trash, the filler to the bottom. Digivolving
/// that Sukamon onto Chuumon (Yellow Lv.3 circle, cost 2) draws 1. The new
/// stack carries Chuumon's inherited leave-prevention: an opponent-effect
/// deletion is offered the "delete 1 other Sukamon" replacement, and paying it
/// keeps the stack. Memory spent: 3 + 2.
///
/// Sources: `EX13-027.md`, `EX13-028.md`; digivolution draw per
/// `general_rule.pdf` digivolve procedure.
#[test]
fn c3_chuumon_tutors_sukamon_digivolves_and_stack_inherits_protection() {
    let mut runner = start(
        builder()
            .hand(0, &[CHUUMON])
            .deck(0, &[BIYOMON, BIYOMON, BIYOMON, BIYOMON, BIYOMON, SUKAMON, SUKAMON, BIYOMON])
            .memory(10),
    );
    let mem0 = runner.memory();

    runner.play(0, hand_index(&runner, 0, CHUUMON)).expect("Chuumon plays");
    assert!(is_reveal_prompt(&runner), "Chuumon [On Play] reveal");
    pick_revealed(&mut runner, SUKAMON);
    if is_reveal_prompt(&runner) {
        pick_revealed(&mut runner, SUKAMON);
    }
    let _ = runner.auto_resolve();
    assert_eq!(count(&hand_ids(&runner, 0), SUKAMON), 1);
    assert_eq!(count(&trash_ids(&runner, 0), SUKAMON), 1);
    let hand_before_digi = runner.hand_size(0);

    let chuu = find_perm(&runner, 0, CHUUMON);
    let slot = hand_index(&runner, 0, SUKAMON) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, chuu.index as u16), 0);
    let _ = runner.auto_resolve();
    assert_eq!(mem0 - runner.memory(), 3 + 2, "play 3 + Yellow Lv.3 digivolve 2");
    assert_eq!(runner.hand_size(0), hand_before_digi, "−Sukamon +1 digivolution draw");
    let stack = find_perm(&runner, 0, SUKAMON);
    assert_eq!(source_ids(&runner, stack), vec![CHUUMON.to_string(), SUKAMON.to_string()]);

    // The stack now carries Chuumon's inherited leave-prevention.
    runner.place_on_field(0, SUKAMON, Some(0));
    runner
        .game
        .delete_permanent_with_cause(stack, ReplacementCause::OpponentEffect);
    let view = runner.pending_selection_view().expect("replacement offer");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner.accept_optional_trigger().expect("accept");
    let ally = PermanentHandle { player: 0, index: 1 };
    assert_eq!(offered_field(&runner, SelectionKind::OwnField, 0), strs(&[SUKAMON]));
    pick_handle(&mut runner, ally);
    // The ally Sukamon's [On Deletion] reveal (no qualifying card left) resolves.
    let _ = runner.auto_resolve();

    let stack = find_perm(&runner, 0, SUKAMON);
    assert!(
        source_ids(&runner, stack).contains(&CHUUMON.to_string()),
        "the digivolved stack survived the opponent's removal"
    );
    assert_eq!(field_ids(&runner, 0).len(), 1, "the ally Sukamon paid the cost");
}

// ═══════════════════════════════════════════════════════════════════════════
// C4 — Salamon tutors Leopardmon + Mikemon; Mikemon digivolves and locks
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-038 Salamon, EX13-040 Mikemon, EX13-043 Leopardmon (hit only).
///
/// Expected mechanical outcome: Salamon's [On Play] reveal adds Leopardmon
/// (the [Leopardmon]-in-text bucket) AND Mikemon (the [Beast] Digimon bucket),
/// bottoming the Bird-trait Biyomon. Digivolving Mikemon onto Salamon (Green
/// Lv.3 circle, cost 2) draws 1 and its [When Digivolving] makes the chosen
/// opponent Digimon unable to unsuspend; the other opponent Digimon is
/// untouched. Memory spent: 3 + 2.
///
/// Sources: `EX13-038.md` (incl. Official Q&A on "in its text"), `EX13-040.md`.
#[test]
fn c4_salamon_tutors_mikemon_which_digivolves_and_locks() {
    let mut runner = start(
        builder()
            .hand(0, &[SALAMON])
            .deck(0, &[BIYOMON, BIYOMON, BIYOMON, BIYOMON, BIYOMON, LEOPARDMON, MIKEMON])
            .memory(10),
    );
    let bird = runner.place_on_field(1, BIRDRAMON, Some(0));
    let draco = runner.place_on_field(1, DRACOMON, Some(0));
    let mem0 = runner.memory();
    let deck0 = runner.deck_size(0);

    runner.play(0, hand_index(&runner, 0, SALAMON)).expect("Salamon plays");
    assert!(is_reveal_prompt(&runner), "Salamon [On Play] reveal");
    assert_eq!(offered_reveal(&runner), strs(&[LEOPARDMON]), "[Leopardmon]-text bucket");
    pick_revealed(&mut runner, LEOPARDMON);
    if is_reveal_prompt(&runner) {
        assert_eq!(offered_reveal(&runner), strs(&[MIKEMON]), "[Beast] Digimon bucket");
        pick_revealed(&mut runner, MIKEMON);
    }
    let _ = runner.auto_resolve();
    let hand = hand_ids(&runner, 0);
    assert!(hand.contains(&LEOPARDMON.to_string()) && hand.contains(&MIKEMON.to_string()), "{hand:?}");
    assert_eq!(runner.deck_size(0), deck0 - 2, "Biyomon went to the bottom");

    let sala = find_perm(&runner, 0, SALAMON);
    let slot = hand_index(&runner, 0, MIKEMON) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, sala.index as u16), 0);
    settle_ordering(&mut runner);
    assert_eq!(
        offered_field(&runner, SelectionKind::OppField, 1),
        strs(&[BIRDRAMON, DRACOMON]),
        "[When Digivolving] lock target"
    );
    pick_handle(&mut runner, bird);
    let _ = runner.auto_resolve();

    assert_eq!(mem0 - runner.memory(), 3 + 2);
    assert_eq!(runner.deck_size(0), deck0 - 3, "digivolution draw");
    assert!(runner.modifiers().has(bird, ModifierType::CannotUnsuspend));
    assert!(!runner.modifiers().has(draco, ModifierType::CannotUnsuspend));
    let mike = find_perm(&runner, 0, MIKEMON);
    assert_eq!(source_ids(&runner, mike), vec![SALAMON.to_string(), MIKEMON.to_string()]);
}

// ═══════════════════════════════════════════════════════════════════════════
// C5 — Mikemon attacks: suspends the locked target; Salamon's inherited +1000
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-040 Mikemon digivolved onto EX13-038 Salamon.
///
/// Expected mechanical outcome: [When Digivolving] locks Birdramon. Mikemon
/// attacks the player; suspending it fires "[All Turns] when this Digimon
/// suspends, suspend 1 opp Digimon/Tamer" → Birdramon. While suspended Mikemon
/// is 5000 + 1000 (Salamon's inherited aura; Mikemon's own inherited is not
/// active face-up) = 6000. The security check resolves (3 → 2). In the
/// opponent's unsuspend phase (6-2-1) Birdramon stays suspended under the
/// lock; the untouched Dracomon is unsuspended normally.
///
/// Sources: `EX13-038.md`, `EX13-040.md`; `general_rule.pdf` 6-2-1.
#[test]
fn c5_mikemon_attack_suspends_locked_target_with_salamon_dp_buff() {
    let mut runner = start(builder().hand(0, &[MIKEMON]).deck(0, &[BIYOMON; 6]).memory(10));
    let sala = runner.place_on_field(0, SALAMON, Some(0));
    let bird = runner.place_on_field(1, BIRDRAMON, Some(0));
    let draco = runner.place_on_field(1, DRACOMON, Some(0));
    runner.game.suspend(draco);

    runner
        .game
        .decode_action(encode_digivolve(hand_index(&runner, 0, MIKEMON) as u16, sala.index as u16), 0);
    settle_ordering(&mut runner);
    pick_handle(&mut runner, bird);
    let _ = runner.auto_resolve();
    assert!(runner.modifiers().has(bird, ModifierType::CannotUnsuspend));

    let mike = find_perm(&runner, 0, MIKEMON);
    assert_eq!(runner.effective_dp(mike), Some(5000), "unsuspended: no aura");
    let sec_before = snapshot(&runner).security[1];

    let _ = runner.attack_player(mike, 1, false);
    settle_ordering(&mut runner);
    assert!(suspended(&runner, mike), "attacking suspends Mikemon");
    assert_eq!(
        offered_field(&runner, SelectionKind::OppField, 1),
        strs(&[BIRDRAMON, DRACOMON]),
        "any opponent Digimon or Tamer"
    );
    pick_handle(&mut runner, bird);
    assert!(suspended(&runner, bird), "Mikemon's suspend trigger suspended Birdramon");
    assert_eq!(runner.effective_dp(mike), Some(6000), "Salamon's inherited +1000 while suspended");
    let _ = runner.auto_resolve();
    assert_eq!(snapshot(&runner).security[1], sec_before - 1, "security check resolved");

    runner.end_turn(); // → opponent's unsuspend phase
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, bird), "locked Birdramon stays suspended");
    assert!(!suspended(&runner, draco), "untouched Dracomon unsuspends");
}
