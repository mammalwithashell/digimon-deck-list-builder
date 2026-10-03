//! EX13 "Mutant / Mamemon" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/mutant-ex13-model.md`. A black removal shell:
//! Bokomon (EX13-050) → Thundermon (EX13-053) / Nanimon (EX13-054) →
//! BigMamemon (EX13-059). Thundermon recurs [Mamemon]-text cards to the deck
//! top and removes on [On Play]/[On Deletion]; BigMamemon reveals free Mutants
//! and sacrifices its own [Mamemon]-named Digimon at end of turn for removal.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{50,53,54,59}.rs`;
//! this file asserts only the cross-card SYSTEM facts.
//!
//! EX13-031 KingSukamon and EX13-063 PrinceMamemon are BLOCKED
//! (`validated_cards_dsl.json`), so no combo here names them.
//!
//! Real DSL cards fill every role — no synthetic cards. Vanilla starter
//! Digimon are the opponents / fillers: ST1-02 Biyomon (cost 2), ST1-04
//! Dracomon (cost 3, Lv.3), ST1-05 Birdramon (cost 4, Lv.4, 5000 DP),
//! ST4-07 Kuwagamon (cost 5), ST5-02 Jazamon (vanilla black Lv.3).
//!
//! No DCGO C# exists for any EX13 Mutant card; printed text (official Bandai
//! DB bundles `data/card_bundles/EX13-0xx.md`) + `general_rule.pdf` govern.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, SEL_REVEAL_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const BOKOMON: &str = "EX13-050";
const THUNDERMON: &str = "EX13-053";
const NANIMON: &str = "EX13-054";
const BIGMAMEMON: &str = "EX13-059";
/// Vanilla cost-2 Rookie — filler + cheapest opponent target.
const BIYOMON: &str = "ST1-02";
/// Vanilla cost-3 Lv.3.
const DRACOMON: &str = "ST1-04";
/// Vanilla cost-4 Lv.4, 5000 DP.
const BIRDRAMON: &str = "ST1-05";
/// Vanilla cost-5 Lv.4.
const KUWAGAMON: &str = "ST4-07";
/// Vanilla black Lv.3 (a Thundermon base with no <Blocker> to inherit).
const JAZAMON: &str = "ST5-02";

const EX13: &[&str] = &[BOKOMON, THUNDERMON, NANIMON, BIGMAMEMON];
const REAL: &[&str] = &[
    BOKOMON, THUNDERMON, NANIMON, BIGMAMEMON, BIYOMON, DRACOMON, BIRDRAMON, KUWAGAMON, JAZAMON,
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

/// DSL-loaded cards carry empty printed text; Thundermon's "[Mamemon] in their
/// texts" filter scans it in production, so copy the EX13 printed text in.
fn with_printed_text(runner: &mut DebugRunner) {
    for id in EX13 {
        let p = printed(id);
        for c in runner.game.card_data.iter_mut().filter(|c| c.card_id == *id) {
            c.effect_text = p.effect_text.clone();
            c.inherited_text = p.inherited_text.clone();
            c.security_text = p.security_text.clone();
        }
    }
}

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL).deck(1, &[BIYOMON; 8])
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

fn trash_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].trash)
}

fn deck_top(runner: &DebugRunner, p: u8) -> String {
    ids_of(runner, &runner.game.players[p as usize].deck)
        .last()
        .cloned()
        .expect("non-empty deck")
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
    ids_of(runner, &runner.game.players[p as usize].hand)
        .iter()
        .position(|h| h == id)
        .unwrap_or_else(|| panic!("{id} must be in player {p}'s hand"))
}

fn view_kind(runner: &DebugRunner) -> Option<SelectionKind> {
    runner.pending_selection_view().map(|v| v.kind)
}

/// Field-pick options (by top-card id) of the pending OwnField/OppField prompt.
fn offered_field(runner: &DebugRunner, kind: SelectionKind, p: u8) -> Vec<String> {
    let view = runner.pending_selection_view().expect("field prompt pending");
    assert_eq!(view.kind, kind, "{view:?}");
    let field = field_ids(runner, p);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(ATTACK_START))
        .filter_map(|i| field.get(i as usize).cloned())
        .collect();
    out.sort();
    out
}

fn pick_field(runner: &mut DebugRunner, p: u8, id: &str) {
    let h = find_perm(runner, p, id);
    let view = runner.pending_selection_view().expect("field prompt pending");
    let a = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&a), "{id} not offered: {view:?}");
    runner.execute_action(view.selecting_player, a).expect("pick field");
}

/// Pick `id` from the pending trash prompt (Thundermon's return).
fn pick_trash(runner: &mut DebugRunner, id: &str) {
    let idx = trash_ids(runner, 0)
        .iter()
        .position(|t| t == id)
        .unwrap_or_else(|| panic!("{id} not in trash: {:?}", trash_ids(runner, 0)));
    let view = runner.pending_selection_view().expect("trash prompt pending");
    let a = TRASH_EFFECT_START + idx as u16;
    assert!(view.valid_action_ids.contains(&a), "{id} not offered: {view:?}");
    runner.execute_action(view.selecting_player, a).expect("pick trash");
}

fn is_trash_prompt(runner: &DebugRunner) -> bool {
    matches!(
        view_kind(runner),
        Some(SelectionKind::UnionZone { .. } | SelectionKind::Trash)
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

/// Digimon-sourced memory gain for `player`, sourced from permanent `src`.
fn gain_from(runner: &mut DebugRunner, src: PermanentHandle, player: u8, amount: i16) {
    let card = runner.game.players[src.player as usize].battle_area[src.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, card, Some(src), player);
    ctx.gain_memory(amount);
}

// ═══════════════════════════════════════════════════════════════════════════
// C1 — Thundermon's [Mamemon] alias pays BigMamemon's end-of-turn cost
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-059 BigMamemon, EX13-053 Thundermon (+ a 2nd BigMamemon in trash).
///
/// Expected mechanical outcome: BigMamemon's [End of Your Turn][OPT] "By
/// deleting 1 of your Digimon with [Mamemon] in its name" offers Thundermon
/// (its (Rule) name is treated as including [Mamemon]) alongside BigMamemon.
/// Deleting Thundermon: BigMamemon deletes the opponent's lowest play cost
/// Digimon (Biyomon, cost 2) FIRST — Thundermon's [On Deletion] waits as a
/// pending trigger (15-8-3-2) — then Thundermon returns the trashed BigMamemon
/// to the deck top (cap 3+1 = 4) and deletes Birdramon (cost 4). Net: own −1,
/// opponent −2, own deck +1 with BigMamemon on top.
///
/// Sources: printed text `data/card_bundles/EX13-053.md`, `EX13-059.md`;
/// `general_rule.pdf` 15-7 (processing conditions), 15-8-3-2 (pending triggers).
#[test]
fn c1_thundermon_alias_pays_bigmamemon_eot_cost_for_double_removal() {
    let mut runner = start(builder().deck(0, &[BIYOMON; 6]).memory(2));
    runner.place_on_field(0, BIGMAMEMON, Some(0));
    runner.place_on_field(0, THUNDERMON, Some(0));
    runner.inject_trash(0, BIGMAMEMON);
    for id in [BIYOMON, DRACOMON, BIRDRAMON, KUWAGAMON] {
        runner.place_on_field(1, id, Some(0));
    }
    let before = snapshot(&runner);

    runner.pass_turn();
    runner.accept_optional_trigger().expect("accept BigMamemon's EoYT cost");
    assert_eq!(
        offered_field(&runner, SelectionKind::OwnField, 0),
        {
            let mut v = vec![BIGMAMEMON.to_string(), THUNDERMON.to_string()];
            v.sort();
            v
        },
        "Thundermon's (Rule) [Mamemon] name pays the cost"
    );
    pick_field(&mut runner, 0, THUNDERMON);

    // BigMamemon's effect finishes first: lowest play cost = Biyomon (2).
    assert_eq!(offered_field(&runner, SelectionKind::OppField, 1), vec![BIYOMON.to_string()]);
    pick_field(&mut runner, 1, BIYOMON);
    settle_ordering(&mut runner);

    // Then Thundermon's pending [On Deletion]: return BigMamemon, stop.
    assert!(is_trash_prompt(&runner), "Thundermon [On Deletion] return prompt");
    pick_trash(&mut runner, BIGMAMEMON);
    while is_trash_prompt(&runner) {
        runner.execute_action(0, PASS).expect("stop returning");
    }
    // 1 returned → cap 4: Dracomon (3) and Birdramon (4), not Kuwagamon (5).
    let mut want = vec![DRACOMON.to_string(), BIRDRAMON.to_string()];
    want.sort();
    assert_eq!(offered_field(&runner, SelectionKind::OppField, 1), want);
    pick_field(&mut runner, 1, BIRDRAMON);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    assert_eq!(field_ids(&runner, 0), vec![BIGMAMEMON.to_string()]);
    assert_eq!(field_ids(&runner, 1), vec![DRACOMON.to_string(), KUWAGAMON.to_string()]);
    assert_eq!(after.field[1], before.field[1] - 2, "two opponent Digimon removed");
    assert_eq!(deck_top(&runner, 0), BIGMAMEMON, "returned card sits on top");
    assert_eq!(after.deck[0], before.deck[0] + 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// C2 — Thundermon stacks the deck → BigMamemon [When Digivolving] replays it
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-053 Thundermon ×2, EX13-059 BigMamemon ×2.
///
/// Expected mechanical outcome: Thundermon's [On Play] returns Thundermon#2,
/// then BigMamemon#2, to the deck top (pick order — last pick on top), raising
/// the delete cap to 3+2 = 5 (Kuwagamon deleted). Digivolving BigMamemon onto
/// Thundermon (Black Lv.4, cost 3) draws BigMamemon#2; the [When Digivolving]
/// reveal then finds Thundermon#2, which is played free and fires its own
/// [On Play] again — nothing [Mamemon]-text left in the trash, so cap 3 →
/// Dracomon deleted. Total memory spent: 4 + 3.
///
/// Sources: printed text `EX13-053.md`, `EX13-059.md`; digivolution draw
/// (`general_rule.pdf` digivolve procedure).
#[test]
fn c2_thundermon_stacks_the_top_and_bigmamemon_reveal_replays_it() {
    let mut runner = start(
        builder()
            .hand(0, &[THUNDERMON, BIGMAMEMON])
            .deck(0, &[BIYOMON; 6])
            .memory(10),
    );
    runner.inject_trash(0, THUNDERMON);
    runner.inject_trash(0, BIGMAMEMON);
    runner.place_on_field(1, DRACOMON, Some(0));
    runner.place_on_field(1, KUWAGAMON, Some(0));
    let mem0 = runner.memory();

    let slot = hand_index(&runner, 0, THUNDERMON);
    runner.play(0, slot).expect("Thundermon plays");
    pick_trash(&mut runner, THUNDERMON);
    pick_trash(&mut runner, BIGMAMEMON);
    while is_trash_prompt(&runner) {
        runner.execute_action(0, PASS).expect("stop returning");
    }
    let mut want = vec![DRACOMON.to_string(), KUWAGAMON.to_string()];
    want.sort();
    assert_eq!(offered_field(&runner, SelectionKind::OppField, 1), want, "cap 3+2 = 5");
    pick_field(&mut runner, 1, KUWAGAMON);
    let _ = runner.auto_resolve();
    assert_eq!(deck_top(&runner, 0), BIGMAMEMON);

    let thunder = find_perm(&runner, 0, THUNDERMON);
    let slot = hand_index(&runner, 0, BIGMAMEMON) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, thunder.index as u16), 0);
    // The digivolution draw took BigMamemon#2; the reveal holds Thundermon#2.
    assert!(is_reveal_prompt(&runner), "BigMamemon [When Digivolving] reveal");
    pick_revealed(&mut runner, THUNDERMON);
    settle_ordering(&mut runner);

    // Thundermon#2's [On Play]: no [Mamemon]-text card left → cap 3.
    while is_trash_prompt(&runner) {
        runner.execute_action(0, PASS).expect("nothing to return");
    }
    assert_eq!(offered_field(&runner, SelectionKind::OppField, 1), vec![DRACOMON.to_string()]);
    pick_field(&mut runner, 1, DRACOMON);
    let _ = runner.auto_resolve();

    assert!(field_ids(&runner, 1).is_empty(), "both opponent Digimon removed");
    let mut mine = field_ids(&runner, 0);
    mine.sort();
    let mut want = vec![BIGMAMEMON.to_string(), THUNDERMON.to_string()];
    want.sort();
    assert_eq!(mine, want, "BigMamemon (on Thundermon) + the free Thundermon#2");
    assert!(
        ids_of(&runner, &runner.game.players[0].hand).contains(&BIGMAMEMON.to_string()),
        "digivolution draw took the returned BigMamemon#2"
    );
    assert_eq!(mem0 - runner.memory(), 4 + 3, "the replayed Thundermon was free");
}

// ═══════════════════════════════════════════════════════════════════════════
// C3 — BigMamemon sacrifices itself, carrying Thundermon's inherited De-Digivolve
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-053 Thundermon (digivolution source), EX13-059 BigMamemon,
/// EX13-054 Nanimon (reveal hit).
///
/// Expected mechanical outcome: at end of turn BigMamemon pays its own cost
/// (it has [Mamemon] in its name) and deletes the opponent's lowest play cost
/// Digimon (Biyomon). Its deletion then triggers BOTH BigMamemon's [On
/// Deletion] reveal and Thundermon's inherited [On Deletion] <De-Digivolve 1>.
/// The reveal plays Nanimon free (its Rule-granted [Mutant] trait qualifies)
/// and Nanimon's [On Play] makes the opponent's remaining Digimon unable to
/// attack players; the De-Digivolve strips Birdramon off Dracomon.
///
/// Sources: printed text `EX13-053.md`, `EX13-054.md` ((Rule) Trait: Has
/// [Mutant]), `EX13-059.md`; `general_rule.pdf` 15-8-3 (simultaneous triggers),
/// §16 <De-Digivolve>.
#[test]
fn c3_bigmamemon_self_sacrifice_chains_reveal_nanimon_and_inherited_de_digivolve() {
    let mut runner = start(
        builder()
            .deck(0, &[BIYOMON, BIYOMON, BIYOMON, BIYOMON, BIYOMON, NANIMON])
            .memory(2),
    );
    runner.place_stack(0, &[THUNDERMON, BIGMAMEMON]);
    runner.place_stack(1, &[DRACOMON, BIRDRAMON]);
    runner.place_on_field(1, BIYOMON, Some(0));

    runner.pass_turn();
    runner.accept_optional_trigger().expect("accept BigMamemon's EoYT cost");
    assert_eq!(
        offered_field(&runner, SelectionKind::OwnField, 0),
        vec![BIGMAMEMON.to_string()],
        "BigMamemon is its own [Mamemon]-named cost (Thundermon is only a source)"
    );
    pick_field(&mut runner, 0, BIGMAMEMON);
    assert_eq!(
        offered_field(&runner, SelectionKind::OppField, 1),
        vec![BIYOMON.to_string()],
        "lowest play cost: Biyomon (2) over the Birdramon stack (4)"
    );
    pick_field(&mut runner, 1, BIYOMON);

    // Both [On Deletion]s: order them, then resolve whichever surfaces.
    let mut revealed_nanimon = false;
    for _ in 0..12 {
        settle_ordering(&mut runner);
        let Some(view) = runner.pending_selection_view() else { break };
        if is_reveal_prompt(&runner) {
            pick_revealed(&mut runner, NANIMON);
            revealed_nanimon = true;
        } else if view.kind == SelectionKind::OppField {
            // Only one opponent permanent remains (the Dracomon/Birdramon stack):
            // it is both the De-Digivolve target and Nanimon's lock target.
            runner
                .execute_action(view.selecting_player, ATTACK_START)
                .expect("opponent pick");
        } else if matches!(view.kind, SelectionKind::Replacement) {
            runner.accept_optional_trigger().expect("accept");
        } else {
            panic!("unexpected prompt: {view:?}");
        }
    }
    let _ = runner.auto_resolve();

    assert!(revealed_nanimon, "BigMamemon's [On Deletion] reveal offered Nanimon");
    assert!(field_ids(&runner, 0).contains(&NANIMON.to_string()), "Nanimon played free");
    assert!(!field_ids(&runner, 0).contains(&BIGMAMEMON.to_string()));
    assert_eq!(
        field_ids(&runner, 1),
        vec![DRACOMON.to_string()],
        "Thundermon's inherited <De-Digivolve 1> stripped Birdramon; Biyomon deleted"
    );
    let opp = find_perm(&runner, 1, DRACOMON);
    assert!(
        runner.game.modifiers.has(opp, ModifierType::CannotAttackPlayer),
        "Nanimon's [On Play] locked the opponent's Digimon from attacking players"
    );
    let trash = trash_ids(&runner, 0);
    assert!(trash.contains(&BIGMAMEMON.to_string()) && trash.contains(&THUNDERMON.to_string()));
    let opp_trash = trash_ids(&runner, 1);
    assert!(opp_trash.contains(&BIYOMON.to_string()) && opp_trash.contains(&BIRDRAMON.to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// C4 — Bokomon → Thundermon: inherited <Blocker> trade kills the attacker
// ═══════════════════════════════════════════════════════════════════════════

/// Digivolve Thundermon from hand onto `base`, then hand the turn to player 1
/// with a ready Birdramon attacker. Returns (Thundermon, attacker).
fn blocker_board(base_id: &str) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = start(builder().hand(0, &[THUNDERMON]).deck(0, &[BIYOMON; 6]).memory(5));
    runner.inject_trash(0, BIGMAMEMON);
    let base = runner.place_on_field(0, base_id, Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert!(
        runner.pending_selection().is_none(),
        "Thundermon has no [When Digivolving] — its [On Play] doesn't fire on digivolve"
    );
    assert_eq!(field_ids(&runner, 0), vec![THUNDERMON.to_string()]);
    let attacker = runner.place_on_field(1, BIRDRAMON, Some(0));
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;
    (runner, base, attacker)
}

/// Cards: EX13-050 Bokomon (digivolution source), EX13-053 Thundermon,
/// EX13-059 BigMamemon (the [Mamemon]-text card in trash).
///
/// Expected mechanical outcome: Thundermon digivolved on Bokomon inherits
/// <Blocker>. Blocking a 5000-DP Birdramon (cost 4) loses the battle (4000),
/// and Thundermon's [On Deletion] returns BigMamemon to the deck top → cap
/// 3+1 = 4 → deletes the attacker. Both Digimon end in trash.
///
/// Sources: printed text `EX13-050.md` (inherited <Blocker>), `EX13-053.md`;
/// `general_rule.pdf` §16 <Blocker>.
#[test]
fn c4_bokomon_blocker_lets_thundermon_trade_and_delete_the_attacker() {
    let (mut runner, thunder, attacker) = blocker_board(BOKOMON);
    assert!(runner.game.has_keyword(thunder, Keyword::Blocker), "inherited <Blocker>");

    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField), "block prompt");
    let view = runner.pending_selection_view().unwrap();
    let block = ATTACK_START + thunder.index as u16;
    assert!(view.valid_action_ids.contains(&block), "{view:?}");
    runner.execute_action(0, block).expect("block with Thundermon");
    settle_ordering(&mut runner);

    assert!(is_trash_prompt(&runner), "Thundermon [On Deletion] return prompt");
    pick_trash(&mut runner, BIGMAMEMON);
    while is_trash_prompt(&runner) {
        runner.execute_action(0, PASS).expect("stop returning");
    }
    assert_eq!(
        offered_field(&runner, SelectionKind::OppField, 1),
        vec![BIRDRAMON.to_string()],
        "cap 4 reaches the cost-4 attacker"
    );
    pick_field(&mut runner, 1, BIRDRAMON);
    let _ = runner.auto_resolve();

    assert!(field_ids(&runner, 0).is_empty(), "Thundermon lost the battle");
    assert!(field_ids(&runner, 1).is_empty(), "the attacker was deleted by [On Deletion]");
    assert_eq!(deck_top(&runner, 0), BIGMAMEMON);
}

/// Unhappy path: on a vanilla black Lv.3 (Jazamon) Thundermon has no <Blocker>
/// — the Bokomon source is what enables the blocker trade.
#[test]
fn c4_without_bokomon_thundermon_cannot_block() {
    let (mut runner, thunder, attacker) = blocker_board(JAZAMON);
    assert!(!runner.game.has_keyword(thunder, Keyword::Blocker));
    let _ = runner.attack_player(attacker, 0, false);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField),
        "no blocker → no block-declaration prompt"
    );
    assert!(field_ids(&runner, 0).contains(&THUNDERMON.to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// C5 — Nanimon → BigMamemon: +1000 inherited, reveal lands Bokomon's lock
// ═══════════════════════════════════════════════════════════════════════════

fn nanimon_to_bigmamemon() -> (DebugRunner, PermanentHandle) {
    // Top (drawn by the digivolve) Biyomon; then the reveal holds Bokomon + 2 Biyomon.
    let mut runner = start(
        builder()
            .hand(0, &[BIGMAMEMON])
            .deck(0, &[BIYOMON, BIYOMON, BIYOMON, BIYOMON, BIYOMON, BOKOMON, BIYOMON])
            .memory(5),
    );
    let base = runner.place_on_field(0, NANIMON, Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert!(is_reveal_prompt(&runner), "BigMamemon [When Digivolving] reveal");
    (runner, base)
}

/// Cards: EX13-054 Nanimon (digivolution source), EX13-059 BigMamemon,
/// EX13-050 Bokomon (reveal hit).
///
/// Expected mechanical outcome: BigMamemon on Nanimon is 8000 + 1000 = 9000 DP
/// (Nanimon's inherited [All Turns] +1000). The reveal plays Bokomon
/// ([Mutant], cost 3) for free, and from then on a Digimon-sourced memory gain
/// is refused (Bokomon: "Players can't gain memory other than by Tamer
/// effects"). Control: declining the play, the same gain goes through.
///
/// Sources: printed text + official Q&A `EX13-050.md`, `EX13-054.md`, `EX13-059.md`.
#[test]
fn c5_nanimon_bigmamemon_reveal_lands_bokomon_memory_lock() {
    let (mut runner, big) = nanimon_to_bigmamemon();
    pick_revealed(&mut runner, BOKOMON);
    let _ = runner.auto_resolve();
    // `DebugRunner::execute_action` calls `resolve_selection` directly; the
    // production `Game::decode_action` path re-ticks declarative effects after
    // every action, which is what materialises Bokomon's [All Turns] floodgate.
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(big), Some(9000), "Nanimon's inherited +1000");
    assert!(field_ids(&runner, 0).contains(&BOKOMON.to_string()), "Bokomon played free");
    assert_eq!(runner.memory(), 5 - 3, "only the digivolve cost was paid");

    let mem = runner.memory();
    gain_from(&mut runner, big, 0, 2);
    assert_eq!(runner.memory(), mem, "Bokomon blocks BigMamemon-sourced memory gain");

    // Control: same line, play declined → no Bokomon → the gain resolves.
    let (mut control, big2) = nanimon_to_bigmamemon();
    control.execute_action(0, PASS).expect("decline the free play");
    let _ = control.auto_resolve();
    control.game.tick_declarative_effects();
    assert!(!field_ids(&control, 0).contains(&BOKOMON.to_string()));
    let mem = control.memory();
    gain_from(&mut control, big2, 0, 2);
    assert_eq!(control.memory(), mem + 2, "without Bokomon the gain goes through");
}
