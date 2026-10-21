//! Chapter 16 (Comprehensive Rules Ver. 4.3), the processing keywords:
//! `<Recovery>` (§16-6), `<Draw>` (§16-8), `<Digisorption>` (§16-10),
//! `<Reboot>` (§16-11), `<De-Digivolve>` (§16-12), `<Digi-Burst>` (§16-14),
//! `<Delay>` (§16-17), `<Save>` (§16-20), `<Material Save>` (§16-21),
//! `<Blast Digivolve>` (§16-26), `<Fortitude>` (§16-27), `<Mind Link>`
//! (§16-28), `<Blast DNA Digivolve>` (§16-31), `<Link +>` (§16-40) and
//! `<Training>` (§16-41).
//!
//! Every test asserts the RULE, never the engine's current output. A test the
//! engine fails is `#[ignore]`d with the gap id that explains it (discover then
//! pin); see `docs/RUST_ENGINE_GAPS.md` and `qa/rules/cr_obligations_status.yaml`.
//!
//! Per keyword: `<kw>_kind_matches_rule` checks the observable consequence of
//! its effect type (a keyword that "executes processing" is processed inside
//! the effect that contains it, with no activation or decision of its own; a
//! trigger-type effect pends with the other effects triggered at the same
//! time; an immediate-type effect is offered BEFORE its cause completes; an
//! activation-type effect is declared by the player; a persistent effect
//! applies from the state of the game, with no decision),
//! `<kw>_optionality_matches_rule` the optional / mandatory split,
//! `<kw>_timing_matches_rule` the "when ..." / "during ..." condition, plus one
//! test per behavioural sub-rule. A keyword whose rule has no such sentence
//! gets no such test.
//!
//! Engine reality: `<Reboot>`, `<Save>`, `<Material Save>`, `<Blast Digivolve>`
//! and `<Fortitude>` are `Keyword` variants granted with `kind: grant_keyword`;
//! `<Recovery>`, `<Draw>`, `<De-Digivolve>` and `<Digi-Burst>` are DSL steps
//! (`recover`, `draw`, `de_digivolve`, `digi_burst`); `<Delay>` is a
//! `kind: delay` clause beside `place_self_as_delay_option`; `<Blast DNA
//! Digivolve>` is a `blast_dna_digivolve` alt path; `<Link +>` is a
//! `ChangeLinkMax` aura; `<Mind Link>` and `<Training>` ride on the
//! printed-keyword parse (`mind_link_card`, `train_card`), as does
//! `<Digisorption>` (`digisorption_card` loads its printed face).
//! Each fixture card's header names the idiom it uses.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch16_proc/*.yaml` (plus
//! `LV4_PLAIN` / `MAIN_DEL1` / `PLAIN_TAMER` / `INH_OD_MEM` / `TRAIN` from
//! `ch15_08`, `MAIN_BOUNCE1` from `ch16_new`, `LINK_A` / `LINK_B` /
//! `LINK_HOST_LV4` / `TAMER_PLAIN` from `ch17`), one synthetic card per file,
//! authored to printed-style text (each clause's `summary:` is that text).
//!
//! Decks, security stacks and stacks (`place_stack`) are listed bottom first:
//! the LAST card is the top card.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, BREEDING_TARGET, DNA_DIGIVOLVE_START, EFFECTS_PER_PERMANENT,
    FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, HAND_EFFECT_START, PASS, SOURCES_PER_FIELD,
    SOURCE_SELECT_END, SOURCE_SELECT_START, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use digimon_engine::card_data::{parse_printed_keywords, CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, GamePhase, Keyword, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn ch16_proc() -> DebugRunnerBuilder {
    fixtures::builder(&[
        fixtures::ch15_08_cards(),
        fixtures::ch16_new_cards(),
        fixtures::ch17_cards(),
        fixtures::ch16_proc_cards(),
    ])
}

/// P0 (memory 5, hand `hand0`) to move in its main phase; P1 holds `hand1`;
/// both players have 6 deck cards and 3 security cards.
fn board_on(builder: DebugRunnerBuilder, hand0: &[&str], hand1: &[&str]) -> DebugRunner {
    builder
        .hand(0, hand0)
        .hand(1, hand1)
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(5)
        .start()
}

fn board(hand0: &[&str], hand1: &[&str]) -> DebugRunner {
    board_on(ch16_proc(), hand0, hand1)
}

/// Put the red `TAMER_PLAIN` (no text) on P0's field: it meets the color
/// requirement of P0's red Options without being a Digimon any effect here
/// can choose. Placed LAST, so it never shifts a staged slot.
fn red_anchor(r: &mut DebugRunner) -> PermanentHandle {
    r.place_on_field(0, "TAMER_PLAIN", Some(0))
}

/// A decision the engine offered.
#[derive(Debug, Clone, PartialEq)]
struct Prompt {
    player: PlayerId,
    kind: SelectionKind,
    optional: bool,
    labels: Vec<String>,
    actions: Vec<u16>,
}

fn current_prompt(r: &DebugRunner) -> Option<Prompt> {
    r.pending_selection().map(|sel| Prompt {
        player: sel.selecting_player,
        kind: sel.kind,
        optional: sel.is_optional,
        labels: sel
            .effect_choices
            .iter()
            .flatten()
            .map(|c| c.label.clone())
            .collect(),
        actions: sel.valid_action_ids.clone(),
    })
}

/// Settle every pending decision: DECLINE every optional prompt, and take the
/// only option of every other prompt. Returns the decisions offered, in order.
fn settle_declining(r: &mut DebugRunner) -> Vec<Prompt> {
    let mut seen = Vec::new();
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return seen;
        };
        seen.push(p.clone());
        if p.optional {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline {p:?}: {e:?}"));
        } else {
            step_prompt(r);
        }
    }
    panic!("prompts did not settle after 24 decisions");
}

/// Settle decisions (as `settle_declining`) until one matching `want` is
/// pending. Returns whether it is.
fn advance_to(r: &mut DebugRunner, want: impl Fn(&Prompt) -> bool) -> bool {
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return false;
        };
        if want(&p) {
            return true;
        }
        if p.optional {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline {p:?}: {e:?}"));
        } else {
            step_prompt(r);
        }
    }
    panic!("no matching prompt after 24 decisions");
}

/// An optional prompt owned by `player`.
fn optional_for(player: PlayerId) -> impl Fn(&Prompt) -> bool {
    move |p: &Prompt| p.optional && p.player == player
}

fn offered_to(prompts: &[Prompt], player: PlayerId) -> bool {
    prompts.iter().any(|p| optional_for(player)(p))
}

/// Every decision offered. (§9-1-5: the used Option's trash is never one,
/// so nothing is filtered out.)
fn decisions(prompts: &[Prompt]) -> Vec<Prompt> {
    prompts.to_vec()
}

/// Whether `player` may take `action` now (its mask bit).
fn legal(r: &DebugRunner, player: PlayerId, action: u16) -> bool {
    build_action_mask(&r.game, player)[action as usize] == 1.0
}

/// Use the Option `option` from P0's hand and pick P1's battle-area `slot`
/// as its target.
fn use_on_opponent_slot(r: &mut DebugRunner, option: &str, slot: u16) {
    use_option(r, 0, option);
    pick_field(r, slot);
}

/// Card ids in `player`'s security stack, bottom first (the last is the top).
fn security_ids(r: &DebugRunner, player: PlayerId) -> Vec<String> {
    card_ids(r, &r.game.player(player).security)
}

/// The cards of the stack at `h`, bottom first (the last is the top card).
fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    card_ids(
        r,
        &r.game.player(h.player).battle_area[h.index as usize].card_sources,
    )
}

/// `player`'s trash, in the order the cards arrived (the last is the newest).
/// (A digivolution card trashed from a stack emits no `GameEvent::Trash`, so
/// the zone itself is read.)
fn trash_in_order(r: &DebugRunner, player: PlayerId) -> Vec<String> {
    card_ids(r, &r.game.player(player).trash)
}

/// The cards that arrived in `player`'s trash after the first `before`.
fn trashed_after(r: &DebugRunner, player: PlayerId, before: usize) -> Vec<String> {
    trash_in_order(r, player)[before..].to_vec()
}

/// How many effect-sourced memory changes `card_id` made since `cp`.
fn memory_events_from(r: &DebugRunner, cp: usize, card_id: &str) -> usize {
    memory_sources(r, cp)
        .iter()
        .filter(|(_, id)| id == card_id)
        .count()
}

/// Indices (since `cp`) of the events matching `pred`.
fn event_indices(r: &DebugRunner, cp: usize, pred: impl Fn(&GameEvent) -> bool) -> Vec<usize> {
    r.events_since(cp)
        .iter()
        .enumerate()
        .filter(|(_, e)| pred(e))
        .map(|(i, _)| i)
        .collect()
}

/// Index (since `cp`) of `player`'s phase change into `phase`.
fn phase_change_index(r: &DebugRunner, cp: usize, player: PlayerId, phase: GamePhase) -> usize {
    event_indices(r, cp, |e| {
        matches!(e, GameEvent::PhaseChange { player: p, phase: ph, .. } if *p == player && *ph == phase)
    })
    .first()
    .copied()
    .unwrap_or_else(|| panic!("no PhaseChange into {phase:?} for P{player}"))
}

/// The battle-area handle of `player`'s permanent whose top card is `card_id`.
fn find_top(r: &DebugRunner, player: PlayerId, card_id: &str) -> PermanentHandle {
    let index = field_top_ids(r, player)
        .iter()
        .position(|c| c == card_id)
        .unwrap_or_else(|| panic!("{card_id} is not on P{player}'s field"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

/// End the current turn and settle the start of the next one (declining any
/// optional prompt). Returns the decisions offered.
fn next_turn(r: &mut DebugRunner) -> Vec<Prompt> {
    r.end_turn();
    settle_declining(r)
}

/// A printed card face (`effect_text`) as production loads it from
/// cards.json: the printed keywords come from `parse_printed_keywords`.
fn printed(base: CardData, effect_text: &str) -> CardData {
    let mut card = base;
    card.effect_text = effect_text.to_string();
    card.keywords = parse_printed_keywords(&card.effect_text, "", "");
    card
}

// ─── §16-6 <Recovery> ───────────────────────────────────────────────────────

/// §16-6-1 "<Recovery> is a keyword effect where the specified number of cards
/// from the specified area are placed face down on top of the security stack.
/// This is shown on cards using text such as <Recovery +1>."
///
/// P0's deck is `[FILL x3, LV4_PLAIN, OD_MEM]` (`OD_MEM` on top). P0 uses
/// `REC_OPT` ("[Main] <Recovery +2 (Deck)>"): the top 2 deck cards
/// (`OD_MEM`, `LV4_PLAIN`) are now the top 2 security cards, nothing was
/// revealed (face down), the deck lost exactly those 2 and the hand is
/// unchanged.
#[test]
fn recovery_places_top_deck_cards_face_down_on_security() {
    cite!(
        "16-6-1",
        "631570b83072defcd716950c57d03d519473369c18b7669b1fe8de88a3c229b4"
    );
    let mut r = ch16_proc()
        .hand(0, &["REC_OPT"])
        .deck(0, &[FILL, FILL, FILL, "LV4_PLAIN", "OD_MEM"])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(5)
        .start();
    red_anchor(&mut r);
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "REC_OPT");
    settle_declining(&mut r);

    let sec = security_ids(&r, 0);
    assert_eq!(sec.len(), 5, "2 cards were added: {sec:?}");
    assert_eq!(
        &sec[..3],
        &owned(&[FILL, FILL, FILL])[..],
        "under the old stack"
    );
    let mut top2 = sec[3..].to_vec();
    top2.sort();
    assert_eq!(
        top2,
        owned(&["LV4_PLAIN", "OD_MEM"]),
        "the top 2 deck cards, on top"
    );
    assert_eq!(r.deck_size(0), 3);
    assert!(hand_ids(&r, 0).is_empty(), "the hand is unchanged");
    assert!(
        !r.events_since(cp).iter().any(|e| matches!(
            e,
            GameEvent::Reveal { .. } | GameEvent::SecurityReveal { .. }
        )),
        "placed face down: nothing was revealed"
    );
}

/// §16-6-2 "<Recovery> effects execute processing."
///
/// <Recovery> is processed inside the effect that contains it: using
/// `REC_OPT` adds the security cards with no activation or decision of its
/// own, before the used Option itself is trashed.
#[test]
fn recovery_kind_matches_rule() {
    cite!(
        "16-6-2",
        "2421fed1cb6d5535ef438e04f597eaffa27b049a29247d96fc06f34216d4983b"
    );
    let mut r = board(&["REC_OPT"], &[]);
    red_anchor(&mut r);
    use_option(&mut r, 0, "REC_OPT");
    assert_eq!(
        r.security_count(0),
        5,
        "placed while the [Main] effect resolves"
    );
    let prompts = settle_declining(&mut r);
    assert!(
        decisions(&prompts).is_empty(),
        "no decision of its own: {prompts:?}"
    );
    assert_eq!(r.security_count(0), 5);
}

// ─── §16-8 <Draw> ───────────────────────────────────────────────────────────

/// §16-8-1 "<Draw> is a keyword effect where a player draws the cards from
/// their deck according to the number specified in the effect. ..."
/// §16-8-2 "<Draw> effects execute processing."
///
/// P0's deck is `[FILL x4, OD_MEM, LV4_PLAIN]`. `DRAW_OPT` ("[Main] <Draw
/// 2>") puts the top 2 deck cards in P0's hand, inside the Option's effect,
/// with no decision of its own.
#[test]
fn draw_kind_matches_rule() {
    cite!(
        "16-8-1",
        "ca5c31ba6ed71de5056da55d1424e3a95523e9e540915a5a297b47a426e45176"
    );
    cite!(
        "16-8-2",
        "3249c4acb1c6db2cf03d4422eab990a91f25530b50a50a74a534ef03636d1878"
    );
    let mut r = ch16_proc()
        .hand(0, &["DRAW_OPT"])
        .deck(0, &[FILL, FILL, FILL, FILL, "OD_MEM", "LV4_PLAIN"])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(5)
        .start();
    red_anchor(&mut r);
    use_option(&mut r, 0, "DRAW_OPT");
    let prompts = settle_declining(&mut r);
    assert!(
        decisions(&prompts).is_empty(),
        "no decision of its own: {prompts:?}"
    );
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(
        hand,
        owned(&["LV4_PLAIN", "OD_MEM"]),
        "the top 2 deck cards"
    );
    assert_eq!(r.deck_size(0), 4);
}

/// §16-8-3 "A draw from <Draw> is mandatory."
///
/// - `DRAW_OPT` ("[Main] <Draw 2>"): nothing lets P0 draw fewer; the deck
///   loses 2 and the hand gains 2.
/// - As a triggered effect: P0's `WA_DRAW` ("[When Attacking] <Draw 1>")
///   attacks P1; no optional prompt is offered for it and P0 draws 1.
#[test]
fn draw_is_mandatory() {
    cite!(
        "16-8-3",
        "1b8c94588f1fdc5ff69663d256a20fd51d26f3f77a163e702e410ad1b26aff79"
    );
    let mut r = board(&["DRAW_OPT"], &[]);
    red_anchor(&mut r);
    use_option(&mut r, 0, "DRAW_OPT");
    let prompts = settle_declining(&mut r);
    assert!(
        decisions(&prompts).is_empty(),
        "nothing to decline: {prompts:?}"
    );
    assert_eq!(r.deck_size(0), 4);
    assert_eq!(r.hand_size(0), 2);

    let mut r = board(&[], &[]);
    let attacker = r.place_on_field(0, "WA_DRAW", Some(0));
    r.attack_player(attacker, 1, false);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 0), "nothing to decline: {prompts:?}");
    assert_eq!(r.hand_size(0), 1, "the [When Attacking] draw happened");
    assert_eq!(r.deck_size(0), 5);
}

// ─── §16-10 <Digisorption> ──────────────────────────────────────────────────

/// `DIGISORB`'s printed face as production would load it from cards.json
/// (effect text + its red Lv.3 digivolve cost 2): `parse_printed_keywords`
/// reads `Keyword::Digisorption` off the printed text.
fn digisorption_card(id: &str, text: &str) -> CardData {
    let mut card = make_test_card(id, "Absorbmon");
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.evo_costs = vec![EvoCost {
        card_color: 0, // red
        level: 3,
        memory_cost: 2,
    }];
    printed(card, text)
}

const DIGISORB_TEXT: &str = concat!(
    "＜Digisorption -3＞ (When one of your Digimon would digivolve into this card from ",
    "your hand, you may suspend 1 of your Digimon to reduce the memory cost of the ",
    "digivolution by 3.)"
);

const DIGISORB_X2_TEXT: &str = concat!(
    "＜Digisorption -1＞ (When one of your Digimon would digivolve into this card from ",
    "your hand, you may suspend 1 of your Digimon to reduce the memory cost of the ",
    "digivolution by 1.) ＜Digisorption -1＞ (When one of your Digimon would digivolve ",
    "into this card from your hand, you may suspend 1 of your Digimon to reduce the ",
    "memory cost of the digivolution by 1.)"
);

/// P0 (memory 5) holds `DIGISORB` (printed `text`); P0's field is the red
/// Lv.3 `FILL` (slot 0, the digivolution base) and `LV4_PLAIN` (slot 1).
fn digisorption_board(text: &str) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = board_on(
        ch16_proc().add_card(digisorption_card("DIGISORB", text)),
        &["DIGISORB"],
        &[],
    );
    let base = r.place_on_field(0, FILL, Some(0));
    let other = r.place_on_field(0, "LV4_PLAIN", Some(0));
    (r, base, other)
}

/// Digivolve P0's `base` into `DIGISORB` (hand index 0) from the hand.
fn digivolve_into_digisorb(r: &mut DebugRunner, base: PermanentHandle) {
    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
}

/// §16-10-1 "<Digisorption> is a keyword effect. When a digivolution into a
/// Digimon card with this effect in the hand would occur, by suspending 1 of
/// your Digimon, the digivolution cost is reduced according to the value
/// specified in the effect. ..."
/// §16-10-3 ""By suspending 1 of your Digimon" in <Digisorption> is an
/// optional processing condition. If the content of that condition is
/// executed, the processing for "the digivolution cost is reduced" will be
/// mandatory."
/// §16-10-4 "If 1 Digimon card has multiple instances of <Digisorption>, they
/// overlap when they trigger. Multiple instances of <Digisorption> can be
/// activated for a single digivolution."
/// §16-10-5 "<Digisorption> can be used to suspend a Digimon that will become
/// a digivolution card."
///
/// - P0 digivolves `FILL` into `DIGISORB` (<Digisorption -3>, cost 2): P0 is
///   offered <Digisorption>; the Digimon P0 may suspend include `FILL` itself
///   (it will become a digivolution card) and `LV4_PLAIN`. Suspending `FILL`
///   reduces the cost 2 by 3, to 0 (a cost is never below 0): P0's memory
///   stays 5, and the new `DIGISORB` Digimon is suspended.
/// - `DIGISORB` printed with two <Digisorption -1>: both are offered for the
///   one digivolution; suspending `FILL` and `LV4_PLAIN` reduces the cost 2
///   to 0.
#[test]
fn digisorption_optional_suspend_reduces_cost_floors_at_zero_and_stacks() {
    cite!(
        "16-10-1",
        "46ffa6072e70675fb7aad16dc7a671b4dfe3bf31402d69494bb9807a3c7af35a"
    );
    cite!(
        "16-10-3",
        "096c5b9d397f4219597aea82e472b233ba6032fe1b474ef40de8077ab0639fad"
    );
    cite!(
        "16-10-4",
        "9eaef97b28a20933b73e2847d5d3ba0498fe1f822579d7e51b26efeb9e15fa3b"
    );
    cite!(
        "16-10-5",
        "bb23bb4ee3301d9acd8de6afd8ca7546e8313f5bebf9271e1a74759ecc1956fb"
    );
    let (mut r, base, other) = digisorption_board(DIGISORB_TEXT);
    digivolve_into_digisorb(&mut r, base);
    let p = current_prompt(&r)
        .expect("P0 is offered <Digisorption> before paying the digivolution cost");
    assert!(p.optional && p.player == 0, "{p:?}");
    accept(&mut r, 0);
    let p = current_prompt(&r).expect("P0 chooses the Digimon to suspend");
    let mut slots: Vec<u16> = p.actions.iter().map(|&a| a - ATTACK_START).collect();
    slots.sort();
    assert_eq!(
        slots,
        vec![base.index as u16, other.index as u16],
        "the Digimon that will become a digivolution card can be suspended"
    );
    pick_field(&mut r, base.index as u16);
    settle_declining(&mut r);
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert!(is_suspended(&r, base), "the suspended Digimon digivolved");
    assert_eq!(r.memory(), 5, "cost 2 - 3 is 0, not negative");

    // Two instances for a single digivolution.
    let (mut r, base, other) = digisorption_board(DIGISORB_X2_TEXT);
    digivolve_into_digisorb(&mut r, base);
    assert!(
        advance_to(&mut r, optional_for(0)),
        "the first <Digisorption -1>"
    );
    accept(&mut r, 0);
    pick_field(&mut r, base.index as u16);
    assert!(
        advance_to(&mut r, optional_for(0)),
        "the second <Digisorption -1>"
    );
    accept(&mut r, 0);
    pick_field(&mut r, other.index as u16);
    settle_declining(&mut r);
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert_eq!(r.memory(), 5, "2 - 1 - 1 = 0");
}

/// §16-10-2 "<Digisorption> is an immediate-type effect that triggers when a
/// digivolution into a card with this effect in the hand WOULD occur. ..."
///
/// When P0 is offered <Digisorption>, the digivolution has not happened:
/// `DIGISORB` is still in P0's hand, `FILL` is still the top card and no
/// memory has been paid.
#[test]
fn digisorption_kind_matches_rule() {
    cite!(
        "16-10-2",
        "a6a10bed2ea37996c30e662df98118930a6e6f7fdba2f0c7efae3f1a8615b815"
    );
    let (mut r, base, _) = digisorption_board(DIGISORB_TEXT);
    digivolve_into_digisorb(&mut r, base);
    let p = current_prompt(&r).expect("P0 is offered <Digisorption>");
    assert!(p.optional && p.player == 0, "{p:?}");
    assert_eq!(
        hand_ids(&r, 0),
        owned(&["DIGISORB"]),
        "offered before the digivolution"
    );
    assert_eq!(stack_ids(&r, base), owned(&[FILL]));
    assert_eq!(r.memory(), 5, "nothing paid yet");
}

/// §16-10-3 ""By suspending 1 of your Digimon" in <Digisorption> is an
/// OPTIONAL processing condition. If the content of that condition is
/// executed, the processing for "the digivolution cost is reduced" will be
/// MANDATORY."
///
/// - Declined: the full cost 2 is paid (memory 3) and no Digimon is suspended.
/// - Accepted: once `LV4_PLAIN` is suspended, the cost is reduced with no
///   further decision (memory 5). (Matrix row `Digisorption`:
///   `optional_cost_then_mandatory`, agrees.)
#[test]
fn digisorption_optionality_matches_rule() {
    cite!(
        "16-10-3",
        "096c5b9d397f4219597aea82e472b233ba6032fe1b474ef40de8077ab0639fad"
    );
    let (mut r, base, other) = digisorption_board(DIGISORB_TEXT);
    digivolve_into_digisorb(&mut r, base);
    assert!(
        current_prompt(&r).is_some_and(|p| optional_for(0)(&p)),
        "P0 may decline <Digisorption>: {:?}",
        current_prompt(&r)
    );
    r.decline_optional_trigger()
        .expect("decline <Digisorption>");
    settle_declining(&mut r);
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert_eq!(r.memory(), 3, "declined: the full cost");
    assert!(!is_suspended(&r, base) && !is_suspended(&r, other));

    let (mut r, base, other) = digisorption_board(DIGISORB_TEXT);
    digivolve_into_digisorb(&mut r, base);
    accept(&mut r, 0);
    pick_field(&mut r, other.index as u16);
    assert!(
        current_prompt(&r).is_none(),
        "the reduction is mandatory once suspended: {:?}",
        current_prompt(&r)
    );
    assert!(is_suspended(&r, other));
    assert_eq!(r.memory(), 5);
}

/// §16-10-2 "<Digisorption> ... triggers when a DIGIVOLUTION into a card with
/// this effect IN THE HAND would occur."
///
/// - Playing `DIGISORB` from the hand (not a digivolution) offers nothing and
///   costs its play cost 5.
/// - Digivolving into it from the hand offers <Digisorption>.
#[test]
fn digisorption_timing_matches_rule() {
    cite!(
        "16-10-2",
        "a6a10bed2ea37996c30e662df98118930a6e6f7fdba2f0c7efae3f1a8615b815"
    );
    let (mut r, _, _) = digisorption_board(DIGISORB_TEXT);
    r.play(0, 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 0),
        "a play is not a digivolution: {prompts:?}"
    );
    assert_eq!(r.memory(), 0);

    let (mut r, base, _) = digisorption_board(DIGISORB_TEXT);
    digivolve_into_digisorb(&mut r, base);
    assert!(
        current_prompt(&r).is_some_and(|p| optional_for(0)(&p)),
        "a digivolution from the hand offers <Digisorption>: {:?}",
        current_prompt(&r)
    );
}

/// §3-4-7-6 "Trigger conditions can't be met by cards in breeding areas,
/// except for effects that explicitly specify or reference breeding areas."
/// §16-10-2 "<Digisorption> is an immediate-type effect that triggers when a
/// digivolution into a card with this effect in the hand would occur. ..."
///
/// P0 digivolves the `FILL` in its BREEDING area into `DIGISORB` from the
/// hand while `LV4_PLAIN` (unsuspended, could pay the condition) is in its
/// battle area: the digivolving Digimon is in the breeding area and
/// <Digisorption> does not reference breeding areas, so it is not offered
/// (DCGO `BT2_047` likewise requires the digivolving permanent to be in the
/// battle area). The full cost 2 is paid and `LV4_PLAIN` stays unsuspended.
#[test]
fn digisorption_not_offered_for_a_breeding_area_digivolution() {
    cite!(
        "3-4-7-6",
        "28cf3f5f581997bd3dd8df2fdac718cd968e073c9431706ec612efceda45cd52"
    );
    cite!(
        "16-10-2",
        "a6a10bed2ea37996c30e662df98118930a6e6f7fdba2f0c7efae3f1a8615b815"
    );
    let mut r = board_on(
        ch16_proc().add_card(digisorption_card("DIGISORB", DIGISORB_TEXT)),
        &["DIGISORB"],
        &[],
    );
    r.place_in_breeding(0, FILL);
    let other = r.place_on_field(0, "LV4_PLAIN", Some(0));
    r.game.decode_action(encode_digivolve(0, BREEDING_TARGET), 0);
    assert!(
        current_prompt(&r).is_none(),
        "a breeding-area digivolution offers nothing: {:?}",
        current_prompt(&r)
    );
    let breeding_top = r.game.players[0]
        .breeding_area
        .as_ref()
        .map(|b| b.top_card().card_id(&r.game.card_data).to_string());
    assert_eq!(breeding_top.as_deref(), Some("DIGISORB"), "it digivolved");
    assert_eq!(r.memory(), 3, "the full cost 2");
    assert!(!is_suspended(&r, other));
}

/// §16-10-3 ""By suspending 1 of your Digimon" in <Digisorption> is an
/// optional processing condition. ..."
///
/// With no Digimon of P0's that can be suspended (the only one, the base
/// `FILL`, is already suspended), the condition can't be performed:
/// <Digisorption> is not offered and the full cost 2 is paid.
#[test]
fn digisorption_not_offered_without_a_digimon_to_suspend() {
    cite!(
        "16-10-3",
        "096c5b9d397f4219597aea82e472b233ba6032fe1b474ef40de8077ab0639fad"
    );
    let mut r = board_on(
        ch16_proc().add_card(digisorption_card("DIGISORB", DIGISORB_TEXT)),
        &["DIGISORB"],
        &[],
    );
    let base = r.place_on_field(0, FILL, Some(0));
    set_suspended(&mut r, base, true);
    digivolve_into_digisorb(&mut r, base);
    assert!(
        current_prompt(&r).is_none(),
        "nothing to suspend, nothing offered: {:?}",
        current_prompt(&r)
    );
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert_eq!(r.memory(), 3);
}

/// §16-10-1 "... by suspending 1 of your Digimon, the digivolution cost is
/// reduced according to the value specified in the effect. ..."
///
/// While P0 can't reduce digivolution costs (`CannotReduceDigivolveCost`),
/// <Digisorption> is not offered: nothing is suspended and the full cost 2 is
/// paid.
#[test]
fn digisorption_not_offered_while_digivolution_cost_cannot_be_reduced() {
    cite!(
        "16-10-1",
        "46ffa6072e70675fb7aad16dc7a671b4dfe3bf31402d69494bb9807a3c7af35a"
    );
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut r, base, other) = digisorption_board(DIGISORB_TEXT);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotReduceDigivolveCost,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
    digivolve_into_digisorb(&mut r, base);
    assert!(
        current_prompt(&r).is_none(),
        "no reduction possible, nothing offered: {:?}",
        current_prompt(&r)
    );
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert_eq!(r.memory(), 3);
    assert!(!is_suspended(&r, base) && !is_suspended(&r, other));
}

/// §16-10-4 "... Multiple instances of <Digisorption> can be activated for a
/// single digivolution."
/// §16-10-3 the condition is "by suspending 1 of your Digimon".
///
/// `DIGISORB` printed with two <Digisorption -1>, and the base `FILL` is P0's
/// only Digimon: after the first instance suspends it, no Digimon can pay the
/// second, so the second is not offered. The cost is 2 - 1 = 1.
#[test]
fn digisorption_second_instance_not_offered_once_nothing_can_suspend() {
    cite!(
        "16-10-4",
        "9eaef97b28a20933b73e2847d5d3ba0498fe1f822579d7e51b26efeb9e15fa3b"
    );
    cite!(
        "16-10-3",
        "096c5b9d397f4219597aea82e472b233ba6032fe1b474ef40de8077ab0639fad"
    );
    let mut r = board_on(
        ch16_proc().add_card(digisorption_card("DIGISORB", DIGISORB_X2_TEXT)),
        &["DIGISORB"],
        &[],
    );
    let base = r.place_on_field(0, FILL, Some(0));
    digivolve_into_digisorb(&mut r, base);
    accept(&mut r, 0);
    pick_field(&mut r, base.index as u16);
    assert!(
        current_prompt(&r).is_none(),
        "the second <Digisorption -1> has no Digimon to suspend: {:?}",
        current_prompt(&r)
    );
    assert_eq!(stack_ids(&r, base), owned(&[FILL, "DIGISORB"]));
    assert_eq!(r.memory(), 4, "2 - 1 = 1 paid");
}

/// §16-10-5 "<Digisorption> can be used to suspend a Digimon that will become
/// a digivolution card." -- when that Digimon CAN be suspended.
///
/// The base `NOSUSP3` ("This Digimon can't suspend") is not among the Digimon
/// P0 may suspend; `LV4_PLAIN` is the only candidate, and suspending it
/// reduces the cost 2 to 0.
#[test]
fn digisorption_base_that_cannot_suspend_is_not_a_candidate() {
    cite!(
        "16-10-5",
        "bb23bb4ee3301d9acd8de6afd8ca7546e8313f5bebf9271e1a74759ecc1956fb"
    );
    let mut r = board_on(
        ch16_proc().add_card(digisorption_card("DIGISORB", DIGISORB_TEXT)),
        &["DIGISORB"],
        &[],
    );
    let base = r.place_on_field(0, "NOSUSP3", Some(0));
    let other = r.place_on_field(0, "LV4_PLAIN", Some(0));
    digivolve_into_digisorb(&mut r, base);
    accept(&mut r, 0);
    let p = current_prompt(&r).expect("P0 chooses the Digimon to suspend");
    let slots: Vec<u16> = p.actions.iter().map(|&a| a - ATTACK_START).collect();
    assert_eq!(slots, vec![other.index as u16], "only a Digimon that can suspend");
    pick_field(&mut r, other.index as u16);
    settle_declining(&mut r);
    assert_eq!(stack_ids(&r, base), owned(&["NOSUSP3", "DIGISORB"]));
    assert!(is_suspended(&r, other));
    assert_eq!(r.memory(), 5);
}

// ─── §16-11 <Reboot> ────────────────────────────────────────────────────────

/// P0's suspended stack `p0_stack` and P1's suspended `[UNSUSP_OBS_INH,
/// LV4_PLAIN]` (P1's own Digimon, unsuspended in P1's unsuspend phase).
/// `UNSUSP_OBS_INH`: "[All Turns] When this Digimon unsuspends, gain 1
/// memory" (inherited).
fn reboot_board(p0_stack: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = board(&[], &[]);
    let mine = r.place_stack(0, p0_stack);
    set_suspended(&mut r, mine, true);
    let theirs = r.place_stack(1, &["UNSUSP_OBS_INH", "LV4_PLAIN"]);
    set_suspended(&mut r, theirs, true);
    (r, mine, theirs)
}

/// §16-11-1 "<Reboot> is a keyword effect where a Digimon with this effect is
/// unsuspended during the opponent's unsuspend phase."
/// §16-11-3 "Even if 1 Digimon has multiple instances of <Reboot>, it is only
/// unsuspended once during the opponent's unsuspend phase."
/// §16-11-5 "The unsuspending from <Reboot> is performed at the same time as
/// the unsuspending of the turn player's Digimon. ..."
///
/// P0's suspended `[UNSUSP_OBS_INH, REBOOT_INH, REBOOT]` has two instances of
/// <Reboot> (its own and the inherited one of `REBOOT_INH`). When P1's turn
/// starts, it is unsuspended together with P1's own suspended Digimon: both
/// "when this Digimon unsuspends" effects fire in P1's unsuspend phase
/// (before P1's draw phase), P0's exactly once.
#[test]
fn reboot_unsuspends_in_opponents_unsuspend_phase_once_with_two_instances() {
    cite!(
        "16-11-1",
        "d4ecc245c3a047cd338ac1d15f759305d3e15b654257608006975dc1fd4aa55a"
    );
    cite!(
        "16-11-3",
        "4a7a6ec035b959cddf24d9ac1755ff4f42bb8714e9fa0d0ae7e91f5deeb37758"
    );
    cite!(
        "16-11-5",
        "47b7e4effee427ae7371d56e0b57b554f0e4374ce4bec6f0ace534082c759681"
    );
    let (mut r, mine, theirs) = reboot_board(&["UNSUSP_OBS_INH", "REBOOT_INH", "REBOOT"]);
    let cp = r.event_checkpoint();
    next_turn(&mut r);
    assert_eq!(r.turn_player(), 1);
    assert!(!is_suspended(&r, mine), "<Reboot> unsuspended P0's Digimon");
    assert!(!is_suspended(&r, theirs), "P1's own Digimon unsuspended");
    let sources = memory_sources(&r, cp);
    let from = |p: PlayerId| {
        sources
            .iter()
            .filter(|(q, id)| *q == p && id == "UNSUSP_OBS_INH")
            .count()
    };
    assert_eq!(
        from(0),
        1,
        "unsuspended once, not once per <Reboot>: {sources:?}"
    );
    assert_eq!(from(1), 1, "{sources:?}");
    let draw = phase_change_index(&r, cp, 1, GamePhase::Draw);
    let wakes = event_indices(
        &r,
        cp,
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "UNSUSP_OBS_INH"),
    );
    assert!(
        wakes.iter().all(|&i| i < draw),
        "both unsuspends happen in P1's unsuspend phase, before its draw phase: {wakes:?} vs {draw}"
    );
}

/// §16-11-2 "<Reboot> is a persistent effect. ..."
///
/// A persistent effect is applied from the game state, with no activation or
/// decision: P0's suspended `[REBOOT_INH, LV4_PLAIN]` (an inherited <Reboot>)
/// unsuspends at P1's turn with nothing offered; the same Digimon whose
/// `REBOOT_INH` was trashed during P0's turn no longer has <Reboot> and stays
/// suspended.
#[test]
fn reboot_kind_matches_rule() {
    cite!(
        "16-11-2",
        "db72d62942650468d79a90fc342411c2921e1e7b7479b688be68816257554633"
    );
    let (mut r, mine, _) = reboot_board(&["REBOOT_INH", "LV4_PLAIN"]);
    let prompts = next_turn(&mut r);
    assert!(prompts.is_empty(), "no decision: {prompts:?}");
    assert!(!is_suspended(&r, mine));

    let (mut r, mine, _) = reboot_board(&["REBOOT_INH", "LV4_PLAIN"]);
    r.game.players[0].battle_area[mine.index as usize]
        .card_sources
        .remove(0);
    assert_eq!(stack_ids(&r, mine), owned(&["LV4_PLAIN"]));
    next_turn(&mut r);
    assert!(is_suspended(&r, mine), "no <Reboot> left: stays suspended");
}

/// §16-11-4 "The unsuspending from <Reboot> is mandatory."
///
/// P0's suspended `REBOOT`: at P1's turn nothing lets P0 keep it suspended;
/// it is unsuspended.
#[test]
fn reboot_optionality_matches_rule() {
    cite!(
        "16-11-4",
        "4d7716ffc3a142be787b4e5b42721b56e592d4d68ddf5f0f1c5ca53bb156fd0e"
    );
    let (mut r, mine, _) = reboot_board(&["REBOOT"]);
    let prompts = next_turn(&mut r);
    assert!(!offered_to(&prompts, 0), "nothing to decline: {prompts:?}");
    assert!(!is_suspended(&r, mine));
}

/// §16-11-1 "... unsuspended DURING THE OPPONENT'S UNSUSPEND PHASE."
///
/// P0's suspended `[UNSUSP_OBS_INH, REBOOT]` unsuspends after P1's turn has
/// started and before P1's draw phase; at the end of P0's own turn (before
/// the turn passes) it is still suspended.
#[test]
fn reboot_timing_matches_rule() {
    cite!(
        "16-11-1",
        "d4ecc245c3a047cd338ac1d15f759305d3e15b654257608006975dc1fd4aa55a"
    );
    let (mut r, mine, _) = reboot_board(&["UNSUSP_OBS_INH", "REBOOT"]);
    assert!(is_suspended(&r, mine), "suspended in P0's own turn");
    let cp = r.event_checkpoint();
    next_turn(&mut r);
    let start = event_indices(&r, cp, |e| {
        matches!(e, GameEvent::TurnStart { player: 1, .. })
    })[0];
    let draw = phase_change_index(&r, cp, 1, GamePhase::Draw);
    let wake = event_indices(
        &r,
        cp,
        |e| matches!(e, GameEvent::MemoryChange { player: 0, source_card_id: Some(id), .. } if id == "UNSUSP_OBS_INH"),
    );
    assert_eq!(wake.len(), 1, "unsuspended once");
    assert!(
        start < wake[0] && wake[0] < draw,
        "in P1's unsuspend phase: {start} < {} < {draw}",
        wake[0]
    );
}

// ─── §16-12 <De-Digivolve> ──────────────────────────────────────────────────

/// P0 holds `option`; P1's stack `p1_stack` is at slot 0; P1 also has the
/// Tamer `observer` when given: `OBS_TOP_TRASH_MEM` ("[All Turns] When the
/// top stacked card of one of your Digimon is trashed, gain 1 memory") or
/// `OBS_SRC_TRASH_MEM` ("[All Turns] When cards are trashed from the
/// digivolution cards of one of your Digimon, gain 1 memory").
fn dedigi_board(
    option: &str,
    p1_stack: &[&str],
    observer: Option<&str>,
) -> (DebugRunner, PermanentHandle) {
    let mut r = board(&[option], &[]);
    let target = r.place_stack(1, p1_stack);
    if let Some(observer) = observer {
        r.place_on_field(1, observer, Some(0));
    }
    red_anchor(&mut r);
    (r, target)
}

/// Settle every decision; a non-optional, non-ordering decision with several
/// options (the <De-Digivolve> count declaration) is answered with the
/// option that trashes the most cards from P1's stacks (the maximum).
fn settle_declaring_max(r: &mut DebugRunner) {
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return;
        };
        if p.optional {
            r.decline_optional_trigger()
                .unwrap_or_else(|e| panic!("decline {p:?}: {e:?}"));
        } else if p.actions.len() == 1 || p.kind == SelectionKind::TriggerOrder {
            let a = p.actions[0];
            r.execute_action(p.player, a)
                .unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        } else {
            let trashed = |a: u16| {
                let mut g = r.game.clone();
                g.resolve_selection(p.player, a)
                    .unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
                g.player(1).trash.len()
            };
            let best = *p.actions.iter().max_by_key(|&&a| trashed(a)).unwrap();
            r.execute_action(p.player, best)
                .unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        }
    }
    panic!("prompts did not settle after 24 decisions");
}

/// §16-12-1 "<De-Digivolve> is a keyword effect where the player declares a
/// number from 1 up to the number specified in the effect, then the declared
/// number of cards are trashed from the chosen stack(s) of cards starting with
/// the top card. ..."
/// §16-12-3 "The trashing from <De-Digivolve> is mandatory. Once activated, a
/// player can't choose to trash 0 cards."
/// §16-12-4 "<De-Digivolve> can't trash cards from level 3 cards or lower."
///
/// - P0's `DEDIGI2_OPT` (<De-Digivolve 2>) on P1's `[FILL, LV4_PLAIN,
///   LV5_PLAIN]`: P0 declares the number, a decision with exactly the options
///   1 and 2 (not 0, not declinable). Declaring 1 trashes `LV5_PLAIN` only,
///   declaring 2 trashes `LV5_PLAIN` and `LV4_PLAIN`.
/// - On `[FILL, LV4_PLAIN]`, declaring the maximum trashes `LV4_PLAIN` and
///   stops at the level 3 `FILL`.
#[test]
fn de_digivolve_player_declares_count_mandatory_at_least_one_stops_at_level_3() {
    cite!(
        "16-12-1",
        "29204991edef21032de826718e2bd74edb37cbc6f870483b871c14735d057d0a"
    );
    cite!(
        "16-12-3",
        "2febcfe712be6454f972591ed0cfeaf5cf014cf8e0fca737180fd668f8edd37c"
    );
    cite!(
        "16-12-4",
        "b205c610d0d6b7a30e8002e5585a7cae99d9a1e5300f403b8d0803ee15675fdd"
    );
    let (mut r, target) = dedigi_board("DEDIGI2_OPT", &[FILL, "LV4_PLAIN", "LV5_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    let p = current_prompt(&r)
        .filter(|p| p.player == 0)
        .expect("P0 declares how many cards to trash");
    assert!(!p.optional, "once activated, 0 can't be chosen: {p:?}");
    assert_eq!(p.actions.len(), 2, "the numbers 1 and 2: {p:?}");
    let mut left: Vec<Vec<String>> = p
        .actions
        .iter()
        .map(|&a| {
            let mut g = r.game.clone();
            g.resolve_selection(0, a).expect("declaration");
            card_ids(
                &r,
                &g.player(1).battle_area[target.index as usize].card_sources,
            )
        })
        .collect();
    left.sort();
    assert_eq!(
        left,
        vec![owned(&[FILL]), owned(&[FILL, "LV4_PLAIN"])],
        "declaring 2 trashes 2, declaring 1 trashes 1"
    );

    let (mut r, target) = dedigi_board("DEDIGI2_OPT", &[FILL, "LV4_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(
        stack_ids(&r, target),
        owned(&[FILL]),
        "stops at the level 3 card"
    );
}

/// §16-12-1 "<De-Digivolve> is a keyword effect where the player declares a
/// number from 1 up to the number specified in the effect, then the declared
/// number of cards are trashed from the chosen stack(s) of cards STARTING
/// WITH THE TOP CARD. ..."
/// §4-8-1 "A digivolution card refers to a card placed under a Digimon. ..."
///
/// Every card <De-Digivolve> trashes is the top card at that moment, never a
/// card under the Digimon, so it trashes no digivolution card: P0's
/// `DEDIGI2_OPT` on P1's `[FILL, LV4_PLAIN, LV5_PLAIN]` trashes 2 cards and
/// P1's `OBS_SRC_TRASH_MEM` ("When cards are trashed from the digivolution
/// cards of one of your Digimon, gain 1 memory") does not trigger. DCGO
/// agrees: `IDegeneration` stacks `WhenTopCardTrashed`, never
/// `OnDigivolutionCardDiscarded` (`CardController.cs` ~L5384 vs ~L5685).
#[test]
fn de_digivolve_trashes_top_cards_not_digivolution_cards() {
    cite!(
        "16-12-1",
        "29204991edef21032de826718e2bd74edb37cbc6f870483b871c14735d057d0a"
    );
    let (mut r, target) = dedigi_board(
        "DEDIGI2_OPT",
        &[FILL, "LV4_PLAIN", "LV5_PLAIN"],
        Some("OBS_SRC_TRASH_MEM"),
    );
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]), "2 cards trashed");
    assert_eq!(
        memory_events_from(&r, cp, "OBS_SRC_TRASH_MEM"),
        0,
        "no digivolution card was trashed"
    );
}

/// §16-12-4 "<De-Digivolve> can't trash cards from level 3 cards or lower."
///
/// - `DEDIGI2_OPT` on P1's `[FILL, LV4_PLAIN]`: `LV4_PLAIN` is trashed, the
///   level 3 `FILL` is not.
/// - `DEDIGI1_OPT` (<De-Digivolve 1>) on P1's `[FILL, OD_MEM]` (a level 3
///   Digimon on a level 3 card): the top card is level 3, so nothing is
///   trashed. (DCGO `IDegeneration` stops when the permanent's level is 3.)
#[test]
fn de_digivolve_cannot_trash_level_3_or_lower() {
    cite!(
        "16-12-4",
        "b205c610d0d6b7a30e8002e5585a7cae99d9a1e5300f403b8d0803ee15675fdd"
    );
    let (mut r, target) = dedigi_board("DEDIGI2_OPT", &[FILL, "LV4_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));

    let (mut r, target) = dedigi_board("DEDIGI1_OPT", &[FILL, "OD_MEM"], None);
    use_on_opponent_slot(&mut r, "DEDIGI1_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(
        stack_ids(&r, target),
        owned(&[FILL, "OD_MEM"]),
        "a level 3 top card can't be trashed"
    );
}

/// §16-12-2 "<De-Digivolve> effects execute processing."
///
/// `DEDIGI1_OPT` on P1's `[FILL, LV4_PLAIN]`: as soon as P0 has chosen the
/// target, `LV4_PLAIN` is in P1's trash -- processed by the Option's [Main]
/// effect itself, with no activation or decision of its own.
#[test]
fn de_digivolve_kind_matches_rule() {
    cite!(
        "16-12-2",
        "0a924be0962e3422e2509510527568f1d74ab36fad96c48209231f390116c8bb"
    );
    let (mut r, target) = dedigi_board("DEDIGI1_OPT", &[FILL, "LV4_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI1_OPT", target.index as u16);
    assert_eq!(
        trash_in_order(&r, 1),
        owned(&["LV4_PLAIN"]),
        "inside the [Main] effect"
    );
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));
    let prompts = settle_declining(&mut r);
    assert!(
        decisions(&prompts).is_empty(),
        "no decision of its own: {prompts:?}"
    );
}

/// §16-12-3 "The trashing from <De-Digivolve> is MANDATORY. ..."
///
/// `DEDIGI1_OPT` on P1's `[FILL, LV4_PLAIN]`: nothing lets P0 decline the
/// trashing; `LV4_PLAIN` is trashed. (Matrix row `DeDigivolve`: mandatory,
/// agrees.)
#[test]
fn de_digivolve_optionality_matches_rule() {
    cite!(
        "16-12-3",
        "2febcfe712be6454f972591ed0cfeaf5cf014cf8e0fca737180fd668f8edd37c"
    );
    let (mut r, target) = dedigi_board("DEDIGI1_OPT", &[FILL, "LV4_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI1_OPT", target.index as u16);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 0), "nothing to decline: {prompts:?}");
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));
}

/// §16-12-5 "If multiple cards are trashed using 1 instance of <De-
/// Digivolve>, all of the cards are considered to be trashed at the same time,
/// but the procedure for trashing them is 1 card from the stack at a time."
/// §16-12-6 "When using <De-Digivolve> to trash multiple cards, even if the
/// category of the top card changes while the trashing is still being
/// performed, the trashing will continue until the declared number is
/// trashed."
///
/// - Category change: `DEDIGI2_OPT` (declaring 2) on P1's `[FILL,
///   PLAIN_TAMER, LV5_PLAIN]`: after `LV5_PLAIN`, the top card is a Tamer;
///   the trashing continues with `PLAIN_TAMER` (top first, 1 at a time).
/// - At the same time: `DEDIGI2_OPT` (declaring 2) on P1's `[FILL, LV4_PLAIN,
///   LV5_PLAIN]` with P1's `OBS_TOP_TRASH_MEM` ("When the top stacked card of
///   one of your Digimon is trashed"): each card is the top card when it is
///   trashed, and the 2 are trashed at the same time, so it triggers once
///   (§15-5-2). DCGO agrees: `IDegeneration` (`CardController.cs`
///   ~L5313-5385) trashes 1 card at a time, then stacks
///   `EffectTiming.WhenTopCardTrashed` ONCE with every trashed card.
///   This observer half is PROVISIONAL until an `on_top_card_trashed` DSL
///   timing exists (`G-DSL-TOP-CARD-TRASHED-TIMING`): the witness is authored
///   on the stand-in `on_digivolution_card_trashed`.
#[test]
fn de_digivolve_multi_card_trash_is_simultaneous_but_sequential() {
    cite!(
        "16-12-5",
        "576b21150d3237de9979204cc9c15708a3317bd32ab042e36a2b2d3981109b47"
    );
    cite!(
        "16-12-6",
        "d9c38010c2f484614592dc2e98179f9a66c8cc0895695e972c99eb059750d863"
    );
    let (mut r, target) = dedigi_board("DEDIGI2_OPT", &[FILL, "PLAIN_TAMER", "LV5_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));
    assert_eq!(
        trash_in_order(&r, 1),
        owned(&["LV5_PLAIN", "PLAIN_TAMER"]),
        "1 card at a time from the top, past the category change"
    );

    let (mut r, target) = dedigi_board(
        "DEDIGI2_OPT",
        &[FILL, "LV4_PLAIN", "LV5_PLAIN"],
        Some("OBS_TOP_TRASH_MEM"),
    );
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "DEDIGI2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));
    assert_eq!(
        memory_events_from(&r, cp, "OBS_TOP_TRASH_MEM"),
        1,
        "both cards were trashed at the same time: one trigger"
    );
}

/// §16-12-7 "When separate instances of <De-Digivolve> are executed multiple
/// times in a single effect, the processing is performed separately for each
/// instance."
/// §16-12-8 "... the state of the chosen card is confirmed after each single
/// process for <De-Digivolve> ends. ... (Example: When 3 instances of
/// <De-Digivolve 1> are executed in a single effect, if the chosen card
/// becomes an "unaffected by effects" card as a result of the 1st instance of
/// <De-Digivolve 1>, that card is unaffected by the following instances of
/// <De-Digivolve 1>."
///
/// `DEDIGI1X2_OPT` ("<De-Digivolve 1> it, then <De-Digivolve 1> it again") on
/// P1's `[FILL, LV4_PLAIN, LV5_PLAIN]` with P1's `OBS_TOP_TRASH_MEM` ("When
/// the top stacked card of one of your Digimon is trashed"): both cards are
/// trashed, by 2 separate trashings (DCGO: 2 `IDegeneration` runs, each
/// stacking its own `WhenTopCardTrashed`). Unlike the 1 instance of
/// <De-Digivolve 2> in `de_digivolve_multi_card_trash_is_simultaneous_but_sequential`
/// (2 cards at the same time: 1 trigger, §15-5-2), the 2nd trashing meets
/// "when cards are trashed" again, so the observer triggers again (§15-4-2-4
/// "If an already triggered effect's trigger conditions are met again before
/// it activates, that effect will be triggered again"; owner ruling
/// 2026-10-10 #5): 2 triggers.
#[test]
fn de_digivolve_separate_instances_processed_separately() {
    cite!(
        "16-12-7",
        "706566bf4559d2891c74d862aaf32604927b34d4ade8ad425abca709c5584da5"
    );
    let (mut r, target) = dedigi_board(
        "DEDIGI1X2_OPT",
        &[FILL, "LV4_PLAIN", "LV5_PLAIN"],
        Some("OBS_TOP_TRASH_MEM"),
    );
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "DEDIGI1X2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]));
    assert_eq!(
        memory_events_from(&r, cp, "OBS_TOP_TRASH_MEM"),
        2,
        "each instance trashed separately: the condition is met twice"
    );
}

/// §16-12-8 "When separate instances of <De-Digivolve> are executed multiple
/// times in a single effect, the state of the chosen card is confirmed after
/// each single process for <De-Digivolve> ends. ... (Example: When 3
/// instances of <De-Digivolve 1> are executed in a single effect, if the
/// chosen card becomes an "unaffected by effects" card as a result of the 1st
/// instance of <De-Digivolve 1>, that card is unaffected by the following
/// instances of <De-Digivolve 1>."
///
/// `DEDIGI1X2_OPT` on P1's `[FILL, IMMUNE_LV4, LV5_PLAIN]`: the 1st instance
/// trashes `LV5_PLAIN`; `IMMUNE_LV4` ("isn't affected by your opponent's
/// effects") is then the top card, so the 2nd instance doesn't affect it.
/// Control: on `[FILL, LV4_PLAIN, LV5_PLAIN]` the 2nd instance trashes
/// `LV4_PLAIN`.
#[test]
fn de_digivolve_state_confirmed_after_each_instance() {
    cite!(
        "16-12-8",
        "f578bdda9bb979befe6962db42892245adde778aa5198bb3dffa6b38f338dfed"
    );
    let (mut r, target) = dedigi_board("DEDIGI1X2_OPT", &[FILL, "LV4_PLAIN", "LV5_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI1X2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(stack_ids(&r, target), owned(&[FILL]), "control");

    let (mut r, target) = dedigi_board("DEDIGI1X2_OPT", &[FILL, "IMMUNE_LV4", "LV5_PLAIN"], None);
    use_on_opponent_slot(&mut r, "DEDIGI1X2_OPT", target.index as u16);
    settle_declaring_max(&mut r);
    assert_eq!(
        stack_ids(&r, target),
        owned(&[FILL, "IMMUNE_LV4"]),
        "the 2nd instance doesn't affect the now-unaffected card"
    );
}

// ─── §16-14 <Digi-Burst> ────────────────────────────────────────────────────

/// Settle every decision, ACCEPTING every optional prompt and taking the
/// only option of every other prompt.
fn settle_accepting(r: &mut DebugRunner) -> Vec<Prompt> {
    let mut seen = Vec::new();
    for _ in 0..24 {
        let Some(p) = current_prompt(r) else {
            return seen;
        };
        seen.push(p.clone());
        if p.optional && p.actions.len() != 1 {
            r.accept_optional_trigger()
                .unwrap_or_else(|e| panic!("accept {p:?}: {e:?}"));
        } else {
            let a = p.actions[0];
            r.execute_action(p.player, a)
                .unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        }
    }
    panic!("prompts did not settle after 24 decisions");
}

/// §16-14-1 "<Digi-Burst> is a keyword effect where another effect specified
/// by this effect will activate by trashing the specified number of
/// digivolution cards from the Digimon with this effect. ..."
/// §16-14-2 "The processing from <Digi-Burst> is optional. ..."
///
/// P0's `[FILL, BURST]` ("[Main] <Digi-Burst 1> ・Gain 2 memory"), P0's
/// `[FILL, LV4_PLAIN]`, and a second `BURST` with no digivolution cards:
/// - The [Main] is a choice P0 may make or not (its bit and PASS are both
///   legal); the bare `BURST` can't activate it (nothing to trash).
/// - Activated: `FILL` is trashed from `BURST` itself (not from the other
///   Digimon), then P0 gains 2 memory.
#[test]
fn digi_burst_optional_trash_sources_then_linked_effect() {
    cite!(
        "16-14-1",
        "c5ba8a86a13a9fb45c7d0630684e3c3cc139c3f872bf48ac74f07694facc6c8e"
    );
    cite!(
        "16-14-2",
        "3ff1fd941479059f79cca27c58479ff7bc850018f57df0bab7957de89902c21f"
    );
    let mut r = board(&[], &[]);
    let burst = r.place_stack(0, &[FILL, "BURST"]);
    let other = r.place_stack(0, &[FILL, "LV4_PLAIN"]);
    let bare = r.place_on_field(0, "BURST", Some(0));
    assert!(
        legal(&r, 0, field_main_action(burst)),
        "<Digi-Burst> can be activated"
    );
    assert!(legal(&r, 0, PASS), "or not");
    assert!(
        !legal(&r, 0, field_main_action(bare)),
        "no digivolution card to trash: can't be activated"
    );
    r.game.decode_action(field_main_action(burst), 0);
    settle_accepting(&mut r);
    assert_eq!(stack_ids(&r, burst), owned(&["BURST"]));
    assert_eq!(stack_ids(&r, other), owned(&[FILL, "LV4_PLAIN"]));
    assert_eq!(trash_in_order(&r, 0), owned(&[FILL]));
    assert_eq!(r.memory(), 7, "then the specified effect");
}

/// §16-14-2 "The processing from <Digi-Burst> is OPTIONAL. ..."
///
/// P0 doesn't activate `BURST`'s [Main] <Digi-Burst 1>: P0's turn ends with
/// nothing pending and `BURST` keeps its digivolution card. (Matrix row
/// `DigiBurst`: optional, agrees.)
#[test]
fn digi_burst_optionality_matches_rule() {
    cite!(
        "16-14-2",
        "3ff1fd941479059f79cca27c58479ff7bc850018f57df0bab7957de89902c21f"
    );
    let mut r = board(&[], &[]);
    let burst = r.place_stack(0, &[FILL, "BURST"]);
    assert!(r.pending_selection().is_none(), "nothing forces it");
    next_turn(&mut r);
    assert_eq!(stack_ids(&r, burst), owned(&[FILL, "BURST"]));
}

// ─── §16-17 <Delay> ─────────────────────────────────────────────────────────

/// P0 uses `option` (red anchor present), then the anchor is removed, so P0
/// has no red Digimon or Tamer left. Returns the runner; the Option is in
/// P0's battle area.
fn delay_placed(option: &str) -> DebugRunner {
    let mut r = board(&[option], &[]);
    let anchor = red_anchor(&mut r);
    use_option(&mut r, 0, option);
    settle_declining(&mut r);
    find_top(&r, 0, option);
    r.game.players[0].battle_area.remove(anchor.index as usize);
    r
}

/// §16-17-1 "<Delay> is a keyword effect. While a card with this effect is in
/// the battle area, by trashing that card, the effect specified in <Delay>
/// will activate."
/// §16-17-3 "<Delay> can't be activated the same turn the card with the
/// effect is placed in the battle area."
///
/// P0 uses `DELAY_MEM` (cost 3, "[Main] Place this card in the battle area.
/// <Delay> ・Gain 2 memory"):
/// - The turn it is placed, its <Delay> can't be activated.
/// - P0's next turn (with no red card left to meet a color requirement):
///   it can; activating it trashes `DELAY_MEM` and P0 gains 2 memory, paying
///   no cost, with no further decision.
#[test]
fn delay_optional_trash_option_not_the_turn_it_entered_no_color_or_cost() {
    cite!(
        "16-17-1",
        "99b324fe4d535b21554d991f5e25ae56206990c41e98d25fe85c1ee2e265c2ad"
    );
    cite!(
        "16-17-3",
        "62b27bc0d8e1e267548487e503b1cde2916a897ad6a7d1115eb1ba78e1a0e5d4"
    );
    let mut r = delay_placed("DELAY_MEM");
    assert_eq!(r.memory(), 2, "the Option's use cost was paid");
    let delay = find_top(&r, 0, "DELAY_MEM");
    assert!(
        !legal(&r, 0, field_main_action(delay)),
        "not the turn it was placed in the battle area"
    );

    next_turn(&mut r);
    next_turn(&mut r);
    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.current_phase(), GamePhase::Main);
    let delay = find_top(&r, 0, "DELAY_MEM");
    assert!(
        legal(&r, 0, field_main_action(delay)),
        "a later turn: it can"
    );
    let memory = r.memory();
    let before = trash_in_order(&r, 0).len();
    r.game.decode_action(field_main_action(delay), 0);
    let prompts = settle_declining(&mut r);
    assert!(prompts.is_empty(), "no further decision: {prompts:?}");
    assert_eq!(
        trashed_after(&r, 0, before),
        owned(&["DELAY_MEM"]),
        "by trashing it"
    );
    assert!(!field_top_ids(&r, 0).contains(&"DELAY_MEM".to_string()));
    assert_eq!(r.memory(), memory + 2, "no cost or color requirement");
}

/// §16-17-2 "The processing from <Delay> is OPTIONAL. ..."
///
/// `DELAY_MEM` placed on P0's previous turn: at P0's next main phase nothing
/// is pending, activating it and PASS are both legal; P0 passes and
/// `DELAY_MEM` stays in the battle area. (Matrix row `Delay`: optional,
/// agrees.)
#[test]
fn delay_optionality_matches_rule() {
    cite!(
        "16-17-2",
        "4b27a2d15cd1a72f76dd8fb89f88873ada84747ddde7c47f0aaa884a5b31c371"
    );
    let mut r = delay_placed("DELAY_MEM");
    next_turn(&mut r);
    next_turn(&mut r);
    assert_eq!(r.turn_player(), 0);
    assert!(r.pending_selection().is_none());
    let delay = find_top(&r, 0, "DELAY_MEM");
    assert!(legal(&r, 0, field_main_action(delay)));
    assert!(legal(&r, 0, PASS));
    next_turn(&mut r);
    assert!(
        field_top_ids(&r, 0).contains(&"DELAY_MEM".to_string()),
        "kept"
    );
}

/// §16-17-2 "The processing from <Delay> is OPTIONAL. ..." -- for a
/// turn-scheduled <Delay>.
///
/// `DELAY_SOYT` ("[Start of Your Turn] <Delay> ・Gain 1 memory") placed on
/// P0's turn: at the start of P0's next turn P0 is asked whether to activate
/// it (the first decision offered); declining leaves it in the battle area and
/// P0 gains nothing; accepting trashes it and P0 gains 1 memory.
#[test]
fn delay_scheduled_window_is_optional() {
    cite!(
        "16-17-2",
        "4b27a2d15cd1a72f76dd8fb89f88873ada84747ddde7c47f0aaa884a5b31c371"
    );
    let run = |accept_it: bool| {
        let mut r = delay_placed("DELAY_SOYT");
        next_turn(&mut r);
        let cp = r.event_checkpoint();
        r.end_turn();
        let p = current_prompt(&r).expect("P0 is asked about the [Start of Your Turn] <Delay>");
        assert!(p.optional && p.player == 0, "{p:?}");
        if accept_it {
            accept(&mut r, 0);
        } else {
            r.decline_optional_trigger().expect("decline <Delay>");
        }
        settle_declining(&mut r);
        assert_eq!(r.turn_player(), 0);
        (r, cp)
    };
    let (r, cp) = run(false);
    assert!(
        field_top_ids(&r, 0).contains(&"DELAY_SOYT".to_string()),
        "declined: kept"
    );
    assert_eq!(memory_events_from(&r, cp, "DELAY_SOYT"), 0);

    let (r, cp) = run(true);
    assert!(
        !field_top_ids(&r, 0).contains(&"DELAY_SOYT".to_string()),
        "trashed"
    );
    assert!(trash_ids(&r, 0).contains(&"DELAY_SOYT".to_string()));
    assert_eq!(memory_events_from(&r, cp, "DELAY_SOYT"), 1);
}

// ─── §16-20 <Save> ──────────────────────────────────────────────────────────

/// P0 holds `MAIN_DEL1`; P1 has `p1_stack` (slot 0) and, with `tamer`, the
/// Tamer `[FILL, PLAIN_TAMER]` (a card already under it).
fn save_board(p1_stack: &[&str], tamer: bool) -> DebugRunner {
    let mut r = board(&["MAIN_DEL1"], &[]);
    r.place_stack(1, p1_stack);
    if tamer {
        r.place_stack(1, &[FILL, "PLAIN_TAMER"]);
    }
    red_anchor(&mut r);
    r
}

/// §16-20-1 "<Save> is a keyword effect that allows you to place this card
/// under 1 of your Tamers."
/// §16-20-3 "The processing from <Save> is optional."
///
/// P0's `MAIN_DEL1` deletes P1's `SAVE` ("[On Deletion] <Save>"):
/// - P1 has `PLAIN_TAMER`: P1 may choose it; `SAVE` is placed under it (not in
///   the trash).
/// - P1 has no Tamer: nothing is offered and `SAVE` stays in the trash.
#[test]
fn save_optional_places_under_tamer() {
    cite!(
        "16-20-1",
        "3428a2fb9ffdf6f5b87b8e28b200d5576b5178f2fb77bbf60359e918d4951ce2"
    );
    cite!(
        "16-20-3",
        "fa2076907649de7044772c41711359c1d6d696d8965a31283e7c7041cb217709"
    );
    let mut r = save_board(&["SAVE"], true);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)), "P1 may <Save>");
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    pick_field(&mut r, tamer.index as u16);
    settle_declining(&mut r);
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    let under = stack_ids(&r, tamer);
    assert!(
        under.contains(&"SAVE".to_string()),
        "under the Tamer: {under:?}"
    );
    assert!(!trash_ids(&r, 1).contains(&"SAVE".to_string()));

    let mut r = save_board(&["SAVE"], false);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(
        !offered_to(&prompts, 1),
        "no Tamer: nothing to offer: {prompts:?}"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["SAVE"]));
}

/// §16-20-2 "<Save> effects execute processing."
///
/// <Save> is processed by the [On Deletion] effect that contains it: P1's
/// deleted `[INH_OD_MEM, SAVE]` has two [On Deletion] effects (`SAVE`'s and
/// the inherited "gain 1 memory"), which pend together; P1 orders them.
/// Choosing `INH_OD_MEM` first, its memory gain resolves, and only then is
/// the <Save> placement offered and made, while `SAVE`'s own [On Deletion]
/// resolves. (The reverse order is not staged: once <Save> moves `SAVE` out
/// of the trash, the inherited effect pending on that former top card can no
/// longer activate, §15-8-3-5 / §15-4-4-3, proven in `ch15_08`.)
#[test]
fn save_kind_matches_rule() {
    cite!(
        "16-20-2",
        "66e88236c71b28cfc442cc26ed22a3da30bec990c1183467dc08493fcbb44d5b"
    );
    let mut r = save_board(&["INH_OD_MEM", "SAVE"], true);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let mut ids: Vec<String> = order_choices(&r, 1)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, owned(&["INH_OD_MEM", "SAVE"]));
    choose(&mut r, "INH_OD_MEM");
    assert!(
        advance_to(&mut r, optional_for(1)),
        "then the <Save> Tamer pick"
    );
    assert_eq!(
        memory_events_from(&r, cp, "INH_OD_MEM"),
        1,
        "already resolved"
    );
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    pick_field(&mut r, tamer.index as u16);
    let after = settle_declining(&mut r);
    assert!(decisions(&after).is_empty(), "{after:?}");
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert!(stack_ids(&r, tamer).contains(&"SAVE".to_string()));
}

/// §16-20-3 "The processing from <Save> is OPTIONAL."
///
/// P1 declines <Save>: `SAVE` stays in the trash and the Tamer's stack is
/// unchanged. (Matrix row `Save`: optional, agrees.)
#[test]
fn save_optionality_matches_rule() {
    cite!(
        "16-20-3",
        "fa2076907649de7044772c41711359c1d6d696d8965a31283e7c7041cb217709"
    );
    let mut r = save_board(&["SAVE"], true);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger().expect("decline <Save>");
    settle_declining(&mut r);
    assert_eq!(trash_ids(&r, 1), owned(&["SAVE"]));
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert_eq!(stack_ids(&r, tamer), owned(&[FILL, "PLAIN_TAMER"]));
}

// ─── §16-21 <Material Save> ─────────────────────────────────────────────────

/// P0 holds `MAIN_DEL1`; P1 has `[FILL, XROSSA, XROSSA, XROSSB, MSAVE2]`
/// (slot 0; `MSAVE2`: DigiXros [Xrossa] x [Xrossb], <Material Save 2>) and
/// the Tamer `[FILL, PLAIN_TAMER]` (a card already under it).
fn material_save_board() -> DebugRunner {
    save_board(&[FILL, "XROSSA", "XROSSA", "XROSSB", "MSAVE2"], true)
}

/// §16-21-2 "<Material Save> is an immediate-type effect that triggers when
/// the Digimon with this effect WOULD be deleted. ..."
///
/// When P1 is offered <Material Save>, `MSAVE2` is still in the battle area
/// and nothing of it is in P1's trash.
#[test]
fn material_save_kind_matches_rule() {
    cite!(
        "16-21-2",
        "b4eebc4754207c3b4d0f8c7f5d648e87f8e3801229f1881a7d24cf554fb3a534"
    );
    let mut r = material_save_board();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(
        advance_to(&mut r, optional_for(1)),
        "P1 is offered <Material Save>"
    );
    assert!(
        field_top_ids(&r, 1).contains(&"MSAVE2".to_string()),
        "offered before the deletion: {:?}",
        field_top_ids(&r, 1)
    );
    assert!(trash_ids(&r, 1).is_empty(), "{:?}", trash_ids(&r, 1));
}

/// §16-21-1 "<Material Save> is a keyword effect. When a Digimon with this
/// effect is deleted, this effect allows you to place a number of the cards
/// specified in the top card's DigiXros requirements from among the
/// digivolution cards under 1 of your Tamers. ..."
/// §16-21-3 "... if <Material Save> will be processed, the number of cards
/// specified by this effect must be placed under your Tamer whenever possible."
/// §16-21-5 "As long as you use a card specified in the DigiXros requirements,
/// multiple copies of the same card can be placed under the Tamer ..."
/// §16-21-6 "If there are any cards stacked under the Tamer when processing
/// <Material Save>, the new card is placed at the bottom."
///
/// P1 processes <Material Save 2> onto `[FILL, PLAIN_TAMER]`: the cards
/// offered are the 2 `XROSSA` and the `XROSSB` (not the unspecified `FILL`,
/// not the top card `MSAVE2`); choosing both `XROSSA` places them under the
/// Tamer, below the `FILL` already there.
#[test]
fn material_save_allows_duplicate_specified_cards_and_places_at_bottom() {
    cite!(
        "16-21-1",
        "027bd0ed2689eca8c707f88a65f3e3168d56973af6cbf08ff7dffd8e036edbf2"
    );
    cite!(
        "16-21-5",
        "88d6df4b4e855d7213bf040400c94c3fda2dfdf16efc349f4294349e31688bb1"
    );
    cite!(
        "16-21-6",
        "08231cc9dcb13025e98e5cf032eded6d5f132271f6b85d0982682dce4f431cb8"
    );
    let mut r = material_save_board();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    process_material_save(&mut r);
    let p = current_prompt(&r).expect("P1 chooses the cards to place");
    let mut offered: Vec<String> = p.actions.iter().map(|&a| picked_card(&r, a)).collect();
    offered.sort();
    assert_eq!(
        offered,
        owned(&["XROSSA", "XROSSA", "XROSSB"]),
        "the specified cards, both copies of [Xrossa]"
    );
    let first = pick_material(&mut r, "XROSSA");
    let second = pick_material(&mut r, "XROSSA");
    assert_ne!(first, second);
    settle_declining(&mut r);
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert_eq!(
        stack_ids(&r, tamer),
        owned(&["XROSSA", "XROSSA", FILL, "PLAIN_TAMER"]),
        "placed at the bottom, under the card already there"
    );
}

/// §16-21-3 "The processing from <Material Save> is optional. However, if
/// <Material Save> will be processed, the number of cards specified by this
/// effect must be placed under your Tamer whenever possible."
///
/// P1 processes <Material Save 2> with 3 specified cards available: after
/// placing 1 card, P1 can't stop -- the next decision is not optional -- and
/// 2 cards end up under the Tamer. (DCGO `MaterialSaveProcess`:
/// `canEndNotMax: false` with `maxCount = min(2, matching cards)`.)
#[test]
fn material_save_places_the_full_number_whenever_possible() {
    cite!(
        "16-21-3",
        "a6b45d1101b59e1daac87590542d11d809faa7d23c7bfdd40b56b1980cba3acd"
    );
    let mut r = material_save_board();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    process_material_save(&mut r);
    pick_material(&mut r, "XROSSB");
    let p = current_prompt(&r).expect("a 2nd card must be placed");
    assert!(!p.optional, "P1 can't stop at 1 card: {p:?}");
    settle_accepting(&mut r);
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert_eq!(stack_ids(&r, tamer).len(), 4, "2 cards placed");
}

/// §16-21-4 "When multiple cards would be placed under 1 Tamer for 1
/// instance of <Material Save>, the procedure for placing them is 1 card at a
/// time, but they are considered to be placed at the same time. In addition,
/// the placement order is chosen by the player processing <Material Save>."
///
/// P1 places `XROSSA` and `XROSSB`: choosing them in one order or the other
/// gives the two different stackings under the Tamer, so the order is P1's
/// choice. (The "at the same time" half has no observer staged.)
#[test]
fn material_save_player_chooses_placement_order() {
    cite!(
        "16-21-1",
        "027bd0ed2689eca8c707f88a65f3e3168d56973af6cbf08ff7dffd8e036edbf2"
    );
    cite!(
        "16-21-4",
        "2cc6804ebeb781490a8f74de58dcff3bc7a215dd98ec83bc5761ca729f4cd356"
    );
    let run = |order: [&str; 2]| {
        let mut r = material_save_board();
        use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
        process_material_save(&mut r);
        pick_material(&mut r, order[0]);
        pick_material(&mut r, order[1]);
        settle_declining(&mut r);
        let tamer = find_top(&r, 1, "PLAIN_TAMER");
        stack_ids(&r, tamer)
    };
    let ab = run(["XROSSA", "XROSSB"]);
    let ba = run(["XROSSB", "XROSSA"]);
    for stack in [&ab, &ba] {
        let mut placed = stack[..2].to_vec();
        placed.sort();
        assert_eq!(placed, owned(&["XROSSA", "XROSSB"]), "{stack:?}");
        assert_eq!(&stack[2..], &owned(&[FILL, "PLAIN_TAMER"])[..], "{stack:?}");
    }
    assert_ne!(ab, ba, "the player's order decides the stacking");
}

/// Take the <Material Save> offer and choose P1's `PLAIN_TAMER` (the engine
/// may offer the Tamer pick itself as the optional decision, or an accept
/// first).
fn process_material_save(r: &mut DebugRunner) {
    assert!(
        advance_to(r, optional_for(1)),
        "P1 is offered <Material Save>"
    );
    let p = current_prompt(r).unwrap();
    if !matches!(p.kind, SelectionKind::OwnField) {
        accept(r, 1);
    }
    let tamer = find_top(r, 1, "PLAIN_TAMER");
    pick_field(r, tamer.index as u16);
}

/// The card a pick action of the pending card choice names: a card of P1's
/// trash (`TRASH_EFFECT_START + i`) or a digivolution card in P1's battle
/// area (`SOURCE_SELECT_START + field * SOURCES_PER_FIELD + source`).
fn picked_card(r: &DebugRunner, action: u16) -> String {
    if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action) {
        let i = (action - TRASH_EFFECT_START) as usize;
        return card_ids(r, &r.game.player(1).trash)[i].clone();
    }
    if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action) {
        let k = action - SOURCE_SELECT_START;
        let (field, source) = (k / SOURCES_PER_FIELD, k % SOURCES_PER_FIELD);
        let h = PermanentHandle {
            player: 1,
            index: field as u8,
        };
        return stack_ids(r, h)[source as usize].clone();
    }
    panic!("unexpected card-choice action {action}")
}

/// Pick a `card_id` option of the pending card choice; returns the action.
fn pick_material(r: &mut DebugRunner, card_id: &str) -> u16 {
    let p = current_prompt(r).expect("a card choice");
    let action = *p
        .actions
        .iter()
        .find(|&&a| picked_card(r, a) == card_id)
        .unwrap_or_else(|| panic!("{card_id} is not offered: {p:?}"));
    r.execute_action(p.player, action)
        .unwrap_or_else(|e| panic!("pick {card_id}: {e:?}"));
    action
}

/// §16-21-3 "The processing from <Material Save> is OPTIONAL. ..."
///
/// P1 declines <Material Save>: every card of the deleted stack is in P1's
/// trash and the Tamer's stack is unchanged. (Matrix row `MaterialSave`:
/// optional, agrees.)
#[test]
fn material_save_optionality_matches_rule() {
    cite!(
        "16-21-3",
        "a6b45d1101b59e1daac87590542d11d809faa7d23c7bfdd40b56b1980cba3acd"
    );
    let mut r = material_save_board();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    assert!(advance_to(&mut r, optional_for(1)));
    r.decline_optional_trigger()
        .expect("decline <Material Save>");
    settle_declining(&mut r);
    assert_eq!(
        trash_ids(&r, 1),
        owned(&["MSAVE2", FILL, "XROSSA", "XROSSA", "XROSSB"])
    );
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert_eq!(stack_ids(&r, tamer), owned(&[FILL, "PLAIN_TAMER"]));
}

/// §16-21-1 "... WHEN A DIGIMON WITH THIS EFFECT IS DELETED ..."
///
/// P0's `MAIN_BOUNCE1` returns `MSAVE2` to P1's hand: no <Material Save> is
/// offered and its digivolution cards go to the trash.
#[test]
fn material_save_timing_matches_rule() {
    cite!(
        "16-21-1",
        "027bd0ed2689eca8c707f88a65f3e3168d56973af6cbf08ff7dffd8e036edbf2"
    );
    let mut r = board(&["MAIN_BOUNCE1"], &[]);
    r.place_stack(1, &[FILL, "XROSSA", "XROSSA", "XROSSB", "MSAVE2"]);
    r.place_stack(1, &[FILL, "PLAIN_TAMER"]);
    red_anchor(&mut r);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 1), "not deleted: {prompts:?}");
    assert_eq!(hand_ids(&r, 1), owned(&["MSAVE2"]));
    let tamer = find_top(&r, 1, "PLAIN_TAMER");
    assert_eq!(stack_ids(&r, tamer), owned(&[FILL, "PLAIN_TAMER"]));
}

// ─── §16-26 <Blast Digivolve> ───────────────────────────────────────────────

/// P1 holds `BLAST` ("{Hand} [Counter] <Blast Digivolve>", Lv.5 from red
/// Lv.4); P1's field is `LV4_PLAIN` (slot 0), `LV4_PLAIN` (slot 1) and the
/// Lv.3 `FILL` (slot 2). P0's `LV5_PLAIN` attacks P1.
fn blast_board() -> DebugRunner {
    let mut r = board(&[], &["BLAST"]);
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r.place_on_field(1, "LV4_PLAIN", Some(0));
    r.place_on_field(1, FILL, Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_player(attacker, 1, false);
    r
}

/// §16-26-1 "<Blast Digivolve> is a keyword effect that allows one of your
/// Digimon to digivolve into a card with this keyword effect in the hand
/// without paying the cost."
/// §16-26-3 "The processing from <Blast Digivolve> is optional."
/// §16-26-4 "<Blast Digivolve> is an effect that digivolves the chosen
/// Digimon in the battle area."
///
/// In P1's counter timing P1 may <Blast Digivolve>: the Digimon P1 can choose
/// are its two `LV4_PLAIN` (not the Lv.3 `FILL`); choosing slot 1 digivolves
/// that Digimon into `BLAST` at no cost (the memory doesn't move).
#[test]
fn blast_digivolve_optional_free_digivolve_of_chosen_battle_area_digimon() {
    cite!(
        "16-26-1",
        "71b90e1a2b737343d3848b522ffa6e552b2f25de4b1f3501b75c7409fd445741"
    );
    cite!(
        "16-26-3",
        "2be999c1b4404e620a0f9705c34e72eb728d1d3cd0bb84d27da33c2fd0ddb350"
    );
    cite!(
        "16-26-4",
        "83253f4d1570ea78de49d22c25cff2f10941d45837de1eaf76d57932e7d0c4bf"
    );
    let mut r = blast_board();
    let p = current_prompt(&r).expect("P1's counter timing");
    assert!(p.optional && p.player == 1, "{p:?}");
    let blast: Vec<u16> = (0..3).map(|slot| encode_digivolve(0, slot)).collect();
    assert!(
        p.actions.contains(&blast[0]) && p.actions.contains(&blast[1]),
        "{p:?}"
    );
    assert!(!p.actions.contains(&blast[2]), "not the Lv.3: {p:?}");
    let memory = r.memory();
    r.execute_action(1, blast[1])
        .expect("<Blast Digivolve> slot 1");
    assert_eq!(r.memory(), memory, "without paying the cost");
    let one = PermanentHandle {
        player: 1,
        index: 1,
    };
    assert_eq!(stack_ids(&r, one), owned(&["LV4_PLAIN", "BLAST"]));
    let zero = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(stack_ids(&r, zero), owned(&["LV4_PLAIN"]));
    assert!(
        !hand_ids(&r, 1).contains(&"BLAST".to_string()),
        "from the hand"
    );
}

/// §16-26-2 "<Blast Digivolve> effects execute processing."
///
/// The digivolution is processed by the counter-timing effect itself: the
/// one choice of P1's Digimon (the counter-timing pick) is followed by no
/// further decision and no activation of its own; `BLAST` is that Digimon's
/// top card when the attack goes on (P0's `LV5_PLAIN` then checks P1's
/// security).
#[test]
fn blast_digivolve_kind_matches_rule() {
    cite!(
        "16-26-2",
        "2937d576fba9ef87b2748e13ab2300a9371ed47d853157f2c8e4add4797da7fa"
    );
    let mut r = blast_board();
    let cp = r.event_checkpoint();
    r.execute_action(1, encode_digivolve(0, 0))
        .expect("<Blast Digivolve> slot 0");
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    let zero = PermanentHandle {
        player: 1,
        index: 0,
    };
    assert_eq!(stack_ids(&r, zero), owned(&["LV4_PLAIN", "BLAST"]));
    assert_eq!(
        event_indices(&r, cp, |e| matches!(e, GameEvent::SecurityReveal { .. })).len(),
        1,
        "then the attack went on"
    );
}

/// §16-26-3 "The processing from <Blast Digivolve> is OPTIONAL."
///
/// P1 declines: `BLAST` stays in P1's hand and P1's Digimon are unchanged.
/// (Matrix row `BlastDigivolve`: optional, agrees.)
#[test]
fn blast_digivolve_optionality_matches_rule() {
    cite!(
        "16-26-3",
        "2be999c1b4404e620a0f9705c34e72eb728d1d3cd0bb84d27da33c2fd0ddb350"
    );
    let mut r = blast_board();
    assert!(current_prompt(&r).is_some_and(|p| optional_for(1)(&p)));
    r.decline_optional_trigger()
        .expect("decline the counter timing");
    settle_declining(&mut r);
    assert_eq!(hand_ids(&r, 1), owned(&["BLAST"]));
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["LV4_PLAIN", "LV4_PLAIN", FILL])
    );
}

// ─── §16-27 <Fortitude> ─────────────────────────────────────────────────────

/// §16-27-1 "<Fortitude> is a keyword effect. When a Digimon with
/// digivolution cards and this effect is deleted, you play this Digimon
/// without paying the cost."
/// §16-27-3 "The processing from <Fortitude> is mandatory."
///
/// - P0's `MAIN_DEL1` deletes P1's `[FILL, FORT]`: with no decision, `FORT`
///   is played again (alone, unsuspended) from the trash, without paying the
///   cost (the memory doesn't move); `FILL` stays in the trash.
/// - P1's `FORT` with no digivolution cards: it stays in the trash.
#[test]
fn fortitude_mandatory_play_from_trash_when_deleted_with_sources() {
    cite!(
        "16-27-1",
        "2809eec382df1a5b61ce7be38a5383976686be499b5d64ec5055c8368f02328c"
    );
    cite!(
        "16-27-3",
        "b8016ebbcc6c737217fc13f84950eedc8354933271ed4265b0c19e7306b945f6"
    );
    let mut r = save_board(&[FILL, "FORT"], false);
    let memory = r.memory();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let prompts = settle_declining(&mut r);
    assert!(!offered_to(&prompts, 1), "mandatory: {prompts:?}");
    let fort = find_top(&r, 1, "FORT");
    assert_eq!(
        stack_ids(&r, fort),
        owned(&["FORT"]),
        "played as a new card"
    );
    assert!(!is_suspended(&r, fort));
    assert_eq!(trash_ids(&r, 1), owned(&[FILL]));
    assert_eq!(r.memory(), memory, "without paying the cost");

    let mut r = save_board(&["FORT"], false);
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    settle_declining(&mut r);
    assert!(
        field_top_ids(&r, 1).is_empty(),
        "no digivolution cards: not played"
    );
    assert_eq!(trash_ids(&r, 1), owned(&["FORT"]));
}

/// §16-27-2 "<Fortitude> is a trigger-type effect that triggers when a
/// Digimon with digivolution cards and this effect is deleted. ..."
///
/// P1's deleted `[INH_OD_MEM, FORT]`: <Fortitude> and the inherited
/// "[On Deletion] Gain 1 memory" triggered by the same deletion pend together;
/// P1 orders them, and choosing `INH_OD_MEM` first resolves its memory gain
/// before `FORT` is played.
#[test]
fn fortitude_kind_matches_rule() {
    cite!(
        "16-27-2",
        "5336433ec70483b5fd4ef892e909f1c69fe63d8eedd4d9a3263176b8296a54cc"
    );
    let mut r = save_board(&["INH_OD_MEM", "FORT"], false);
    let cp = r.event_checkpoint();
    use_on_opponent_slot(&mut r, "MAIN_DEL1", 0);
    let mut ids: Vec<String> = order_choices(&r, 1)
        .iter()
        .map(|l| branch_card(l).to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, owned(&["FORT", "INH_OD_MEM"]));
    choose(&mut r, "INH_OD_MEM");
    settle_declining(&mut r);
    let gain = event_indices(
        &r,
        cp,
        |e| matches!(e, GameEvent::MemoryChange { source_card_id: Some(id), .. } if id == "INH_OD_MEM"),
    );
    let play = event_indices(
        &r,
        cp,
        |e| matches!(e, GameEvent::Play { card_id, .. } if card_id == "FORT"),
    );
    assert_eq!((gain.len(), play.len()), (1, 1));
    assert!(gain[0] < play[0], "in the order P1 chose");
}

/// §16-27-2 "... triggers when a Digimon with digivolution cards and this
/// effect IS DELETED."
///
/// - P0's `MAIN_BOUNCE1` returns P1's `[FILL, FORT]` to the hand: not
///   deleted, nothing is played.
/// - P0's `LV5_PLAIN` (7000) deletes P1's suspended `[FILL, FORT]` (5000) in
///   battle: <Fortitude> plays `FORT`.
#[test]
fn fortitude_timing_matches_rule() {
    cite!(
        "16-27-2",
        "5336433ec70483b5fd4ef892e909f1c69fe63d8eedd4d9a3263176b8296a54cc"
    );
    let mut r = board(&["MAIN_BOUNCE1"], &[]);
    r.place_stack(1, &[FILL, "FORT"]);
    red_anchor(&mut r);
    use_on_opponent_slot(&mut r, "MAIN_BOUNCE1", 0);
    settle_declining(&mut r);
    assert_eq!(hand_ids(&r, 1), owned(&["FORT"]));
    assert!(field_top_ids(&r, 1).is_empty());

    let mut r = board(&[], &[]);
    let fort = r.place_stack(1, &[FILL, "FORT"]);
    set_suspended(&mut r, fort, true);
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_digimon(attacker, fort, false);
    settle_declining(&mut r);
    assert_eq!(
        field_top_ids(&r, 1),
        owned(&["FORT"]),
        "deleted in battle: played"
    );
}

// ─── §16-28 <Mind Link> ─────────────────────────────────────────────────────

/// `MINDLINK`'s printed face, as production loads it: the DSL can't grant
/// <Mind Link>, so the runtime behaviour rides on the printed-keyword parse
/// (same staging as `ch15_08`'s <Training>).
fn mind_link_card() -> CardData {
    let mut card = make_test_card("MINDLINK", "Bond Tamer");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    let card = printed(
        card,
        concat!(
            "[Main] ＜Mind Link＞ with 1 of your Digimon. (Place this Tamer as that Digimon's ",
            "bottom digivolution card if there are no Tamer cards in its digivolution cards.)"
        ),
    );
    assert!(
        card.keywords.contains(&Keyword::MindLink),
        "the printed text parses to <Mind Link>: {:?}",
        card.keywords
    );
    card
}

/// P0's `MINDLINK` (slot 0), `LV4_PLAIN` (slot 1, no Tamer card in its
/// digivolution cards) and `[PLAIN_TAMER, LV4_PLAIN]` (slot 2, a Tamer card
/// in its digivolution cards).
fn mind_link_board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = board_on(ch16_proc().add_card(mind_link_card()), &[], &[]);
    let tamer = r.place_on_field(0, "MINDLINK", Some(0));
    let free = r.place_on_field(0, "LV4_PLAIN", Some(0));
    r.place_stack(0, &["PLAIN_TAMER", "LV4_PLAIN"]);
    (r, tamer, free)
}

/// §16-28-1 "<Mind Link> is a keyword effect that places a Tamer with this
/// effect in the digivolution cards of a Digimon with no Tamer cards in its
/// digivolution cards."
/// §16-28-3 "The processing from <Mind Link> is mandatory. However, if it
/// uses the [Main] activation timing, it can be activated and processed at a
/// timing chosen by the player."
///
/// P0 activates `MINDLINK`'s [Main] <Mind Link>: the only Digimon P0 can
/// choose is the Tamer-less `LV4_PLAIN` (slot 1), and once activated the
/// choice can't be declined; `MINDLINK` becomes its bottom digivolution card
/// and is no longer on the field.
#[test]
fn mind_link_places_tamer_under_tamerless_digimon_mandatory() {
    cite!(
        "16-28-3",
        "d7fd5721ce3b7c69bff1c61b31052bfab5b629db1f039b4e9e34b72d43a70db1"
    );
    let (mut r, tamer, free) = mind_link_board();
    r.game.decode_action(field_main_action(tamer), 0);
    let p = current_prompt(&r).expect("P0 chooses the Digimon");
    assert_eq!(p.player, 0);
    assert_eq!(
        p.actions,
        vec![ATTACK_START + free.index as u16],
        "only the Tamer-less Digimon"
    );
    assert!(!p.optional, "mandatory once activated: {p:?}");
    r.execute_action(0, p.actions[0]).expect("pick");
    settle_declining(&mut r);
    assert_eq!(field_top_ids(&r, 0), owned(&["LV4_PLAIN", "LV4_PLAIN"]));
    let linked = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(stack_ids(&r, linked), owned(&["MINDLINK", "LV4_PLAIN"]));
}

/// §16-28-2 "<Mind Link> effects execute processing."
///
/// §16-28-1 "<Mind Link> is a keyword effect that places a Tamer with this
/// effect in the digivolution cards of a Digimon with no Tamer cards in its
/// digivolution cards."
///
/// <Mind Link> is processed by the [Main] effect that contains it: the only
/// Digimon P0 can choose is the Tamer-less `LV4_PLAIN` (slot 1); once chosen,
/// `MINDLINK` is its bottom digivolution card with no further activation or
/// decision.
#[test]
fn mind_link_kind_matches_rule() {
    cite!(
        "16-28-1",
        "80f4ca2834771246050ad8aadd1633987980737deda024e7c737e1e1fc3bfde7"
    );
    cite!(
        "16-28-2",
        "74a5bb0fb732b99869ddc5f446d5aa60485c683dd9607833202fa50f4f9836a9"
    );
    let (mut r, tamer, free) = mind_link_board();
    r.game.decode_action(field_main_action(tamer), 0);
    let p = current_prompt(&r).expect("P0 chooses the Digimon");
    assert_eq!(
        p.actions,
        vec![ATTACK_START + free.index as u16],
        "only the Tamer-less Digimon"
    );
    pick_field(&mut r, free.index as u16);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    let linked = PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(stack_ids(&r, linked), owned(&["MINDLINK", "LV4_PLAIN"]));
}

/// §16-28-3 "... if it uses the [Main] activation timing, it can be activated
/// and processed AT A TIMING CHOSEN BY THE PLAYER."
///
/// In P0's main phase the [Main] <Mind Link> and PASS are both legal; P0 ends
/// the turn without it and `MINDLINK` stays on the field. With no Tamer-less
/// Digimon (only `[PLAIN_TAMER, LV4_PLAIN]`) it can't be activated: §16-28-1
/// places the Tamer under "a Digimon with no Tamer cards in its digivolution
/// cards".
#[test]
fn mind_link_timing_matches_rule() {
    cite!(
        "16-28-3",
        "d7fd5721ce3b7c69bff1c61b31052bfab5b629db1f039b4e9e34b72d43a70db1"
    );
    cite!(
        "16-28-1",
        "80f4ca2834771246050ad8aadd1633987980737deda024e7c737e1e1fc3bfde7"
    );
    let (mut r, tamer, _) = mind_link_board();
    assert!(r.pending_selection().is_none());
    assert!(legal(&r, 0, field_main_action(tamer)));
    assert!(legal(&r, 0, PASS));
    next_turn(&mut r);
    assert!(field_top_ids(&r, 0).contains(&"MINDLINK".to_string()));

    let mut r = board_on(ch16_proc().add_card(mind_link_card()), &[], &[]);
    let tamer = r.place_on_field(0, "MINDLINK", Some(0));
    r.place_stack(0, &["PLAIN_TAMER", "LV4_PLAIN"]);
    assert!(
        !legal(&r, 0, field_main_action(tamer)),
        "no Digimon without a Tamer card: nothing to place it in"
    );
}

// ─── §16-31 <Blast DNA Digivolve> ───────────────────────────────────────────

/// P1 holds `[BDNA, RIGHT5]` (`BDNA`: "DNA Digivolve: red Lv.5 + blue Lv.5;
/// {Hand} [Counter] <Blast DNA Digivolve> <[Leftmon] + [Rightmon]>"); P1's
/// field is `left` (slot 0). P0's `LV5_PLAIN` attacks P1.
fn blast_dna_board(left: &str) -> DebugRunner {
    let mut r = board(&[], &["BDNA", "RIGHT5"]);
    r.place_on_field(1, left, Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_player(attacker, 1, false);
    r
}

/// The Blast DNA action for hand index 0 (`BDNA`).
const BLAST_DNA: u16 = DNA_DIGIVOLVE_START;

fn blast_dna_offered(r: &DebugRunner) -> bool {
    current_prompt(r).is_some_and(|p| p.player == 1 && p.actions.contains(&BLAST_DNA))
}

/// §16-31-2 "<Blast DNA Digivolve> effects execute processing."
/// §16-31-4 "<Blast DNA Digivolve> is an effect that performs DNA
/// digivolution. ..."
/// §16-31-6 "When DNA digivolution is performed using <Blast DNA Digivolve>,
/// the player reveals the cards that will become digivolution cards from
/// their hand at the same time as when they choose their Digimon that meet the
/// DNA digivolution requirements."
///
/// P1's field `LEFT5` ([Leftmon], red Lv.5) and hand `RIGHT5` ([Rightmon],
/// blue Lv.5): in P1's counter timing P1 chooses <Blast DNA Digivolve>, the
/// field Digimon and the hand card; both are chosen before any card moves
/// (at the hand-card choice `BDNA` and `RIGHT5` are still in the hand and
/// `LEFT5` is still alone). The result is one DNA digivolution at no cost,
/// stacked per §8-2-2-2 (the Digimon on the left of the requirement, red
/// `LEFT5`, on top of the digivolution cards): `[RIGHT5, LEFT5, BDNA]`.
#[test]
fn blast_dna_kind_matches_rule() {
    cite!(
        "16-31-2",
        "9463ceef9ec38a90998bf704aed39fa9af890ca531d20d4d9ea91e4f7c6aa18c"
    );
    cite!(
        "16-31-4",
        "c8ee07972fef4954a886dbffa5b93918b8dc95f44a5a9bbb4bcc1e1179fc6574"
    );
    cite!(
        "16-31-6",
        "482adf190a9126299cf6e39687468a06e26e37098b4a65d89d1394a7506dde6e"
    );
    let mut r = blast_dna_board("LEFT5");
    assert!(blast_dna_offered(&r), "{:?}", current_prompt(&r));
    let memory = r.memory();
    r.execute_action(1, BLAST_DNA)
        .expect("<Blast DNA Digivolve>");
    step_prompt(&mut r); // the field Digimon (LEFT5 only)
    assert!(r.pending_selection().is_some(), "then the hand card");
    assert_eq!(
        hand_ids(&r, 1),
        owned(&["BDNA", "RIGHT5"]),
        "nothing moved yet"
    );
    assert_eq!(
        stack_ids(
            &r,
            PermanentHandle {
                player: 1,
                index: 0
            }
        ),
        owned(&["LEFT5"])
    );
    step_prompt(&mut r); // the hand card (RIGHT5 only)
    assert_eq!(
        stack_ids(
            &r,
            PermanentHandle {
                player: 1,
                index: 0
            }
        ),
        owned(&["RIGHT5", "LEFT5", "BDNA"])
    );
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert_eq!(r.memory(), memory, "without paying the cost");
}

/// §16-31-1 "<Blast DNA Digivolve> is a keyword effect. One of your Digimon
/// specified in this effect and a card from your hand MAY digivolve ..."
/// §16-31-3 "The processing from <Blast DNA Digivolve> is optional."
///
/// The counter timing that offers it is optional; P1 declines: `BDNA` and
/// `RIGHT5` stay in the hand, `LEFT5` is unchanged.
#[test]
fn blast_dna_optionality_matches_rule() {
    cite!(
        "16-31-1",
        "af01e521d36d028ac10c5cbf984bd18dae092af942019b5d6402cb428022bb7f"
    );
    cite!(
        "16-31-3",
        "e8a3004a6e511bd3940339db8e0a93a08f0738d25b867c9f5c75ade7326a373e"
    );
    let mut r = blast_dna_board("LEFT5");
    assert!(blast_dna_offered(&r));
    assert!(
        current_prompt(&r).is_some_and(|p| p.optional),
        "{:?}",
        current_prompt(&r)
    );
    r.decline_optional_trigger().expect("decline");
    settle_declining(&mut r);
    assert_eq!(hand_ids(&r, 1), owned(&["BDNA", "RIGHT5"]));
    assert_eq!(field_top_ids(&r, 1), owned(&["LEFT5"]));
}

/// §16-31-5 "A Digimon card's DNA digivolution requirements can't be ignored
/// for <Blast DNA Digivolve>."
///
/// `BDNA`'s DNA requirement is "red Lv.5 + blue Lv.5". P1's field `LEFT4` is
/// a [Leftmon] but level 4: it doesn't meet the requirement, so <Blast DNA
/// Digivolve> is not offered with it. Control: `LEFT5` (level 5) is.
#[test]
fn blast_dna_cannot_ignore_dna_requirements() {
    cite!(
        "16-31-5",
        "aea0042eadc9cfcd7a1c47c8e7351868f3bc79a569016c612a3bd6b6876232ec"
    );
    let r = blast_dna_board("LEFT5");
    assert!(blast_dna_offered(&r), "control: {:?}", current_prompt(&r));

    let r = blast_dna_board("LEFT4");
    assert!(
        !blast_dna_offered(&r),
        "a level 4 [Leftmon] doesn't meet the DNA requirement: {:?}",
        current_prompt(&r)
    );
}

/// §16-31-4 "<Blast DNA Digivolve> is an effect that performs DNA
/// digivolution." A DNA digivolution needs a DNA requirement to be met
/// (§8-2-1), so a card that prints <Blast DNA Digivolve> but no DNA
/// digivolution requirement (`BDNA_NOREQ`) is not offered, even with the
/// specified [Leftmon] on the field and [Rightmon] in the hand. Control: `BDNA`
/// (same blast materials plus the requirement) is offered on the same board.
#[test]
fn blast_dna_without_dna_requirement_is_not_offered() {
    cite!(
        "16-31-4",
        "c8ee07972fef4954a886dbffa5b93918b8dc95f44a5a9bbb4bcc1e1179fc6574"
    );
    let r = blast_dna_board("LEFT5");
    assert!(blast_dna_offered(&r), "control: {:?}", current_prompt(&r));

    let mut r = board(&[], &["BDNA_NOREQ", "RIGHT5"]);
    r.place_on_field(1, "LEFT5", Some(0));
    let attacker = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.attack_player(attacker, 1, false);
    assert!(
        !blast_dna_offered(&r),
        "no DNA requirement to meet: {:?}",
        current_prompt(&r)
    );
}

// ─── §16-40 <Link +> ────────────────────────────────────────────────────────

/// Whether P0 may link `LINK_A` (hand index 0; link requirement: a level 4
/// Digimon) now.
fn can_link(r: &DebugRunner) -> bool {
    legal(r, 0, HAND_EFFECT_START)
}

/// P0 (hand `[LINK_A]`) has `host` (slot 0) with `links` link cards
/// (`LINK_B`) and, with `aura`, `LINKPLUS_AURA` ("[All Turns] Your Digimon
/// gain <Link +1>").
fn link_board(host: &str, links: usize, aura: bool) -> DebugRunner {
    let mut r = board(&["LINK_A"], &[]);
    let h = r.place_on_field(0, host, Some(0));
    for _ in 0..links {
        r.push_linked_owned(h, "LINK_B", 0);
    }
    if aura {
        r.place_on_field(0, "LINKPLUS_AURA", Some(0));
    }
    // Continuous effects are materialized at action boundaries; staging
    // placed the cards directly.
    r.game.tick_declarative_effects();
    r
}

/// Link `LINK_A` from P0's hand onto `host` (the only Digimon), taking the
/// first option of any decision (the host pick; which existing link card to
/// trash, if asked). `None` when the link can't be declared; otherwise the
/// host's link cards and P0's trash.
fn link_outcome(host: &str, links: usize, aura: bool) -> Option<(Vec<String>, Vec<String>)> {
    let mut r = link_board(host, links, aura);
    if !can_link(&r) {
        return None;
    }
    r.game.decode_action(HAND_EFFECT_START, 0);
    for _ in 0..8 {
        let Some(p) = current_prompt(&r) else {
            break;
        };
        let a = p.actions[0];
        r.execute_action(p.player, a)
            .unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
    }
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    let linked = card_ids(&r, &r.game.player(0).battle_area[0].linked_cards);
    Some((linked, trash_ids(&r, 0)))
}

/// Below the maximum, linking adds `LINK_A` and trashes nothing.
fn assert_link_adds(host: &str, links: usize, aura: bool) {
    let (linked, trash) = link_outcome(host, links, aura)
        .unwrap_or_else(|| panic!("{host} with {links} link cards (aura {aura}): can link"));
    assert_eq!(
        linked.len(),
        links + 1,
        "{host}/{links}/{aura}: added: {linked:?}"
    );
    assert!(linked.contains(&"LINK_A".to_string()), "{linked:?}");
    assert!(
        !trash.contains(&"LINK_B".to_string()),
        "nothing trashed: {trash:?}"
    );
}

/// At the maximum, linking is still legal and REPLACES: `LINK_A` is linked,
/// the count stays the same, and exactly 1 existing link card is trashed.
/// Simultaneity (§4-9-5 "trashed at the same time as the newly linked
/// cards") is not asserted.
fn assert_link_replaces(host: &str, links: usize, aura: bool) {
    let (linked, trash) = link_outcome(host, links, aura).unwrap_or_else(|| {
        panic!("{host} with {links} link cards (aura {aura}): at the limit the link is still legal")
    });
    assert_eq!(
        linked.len(),
        links,
        "{host}/{links}/{aura}: replaced, not added: {linked:?}"
    );
    assert!(linked.contains(&"LINK_A".to_string()), "{linked:?}");
    assert_eq!(
        trash.iter().filter(|c| *c == "LINK_B").count(),
        1,
        "1 existing link card trashed: {trash:?}"
    );
}

/// §16-40-2 "<Link +> is a persistent effect. ..."
/// §4-9-5 "1 card can have a maximum of 1 link card. When linking to a
/// Digimon that has already reached the link limit, the same number of the
/// existing link cards are trashed at the same time as the newly linked
/// cards."
///
/// `LINK_HOST_LV4` with `LINKPLUS_AURA` ("Your Digimon gain <Link +1>") on
/// the field has a maximum of 2 as soon as the aura applies, with nothing
/// activated or decided for it: with 1 link card, linking `LINK_A` adds it
/// (2 link cards, nothing trashed); with 2, linking `LINK_A` replaces 1 of
/// them (still 2, 1 trashed).
#[test]
fn link_plus_kind_matches_rule() {
    cite!(
        "16-40-2",
        "fd7cb513430e7f9af8b81f14ad80fd7da63cdcd38afd6adf9a6f9ec496fad7fc"
    );
    let r = link_board("LINK_HOST_LV4", 1, true);
    assert!(r.pending_selection().is_none(), "no decision");
    assert_link_adds("LINK_HOST_LV4", 1, true);
    assert_link_replaces("LINK_HOST_LV4", 2, true);
}

/// §16-40-1 "<Link +> is a keyword effect that adds the specified number to
/// the maximum link cards of the Digimon with this effect."
/// §16-40-3 "If 1 Digimon has multiple instances of <Link +>, the maximum
/// number of link cards will increase according to those numerical values.
/// However, even if a card has multiple instances of <Link +>, the effect
/// isn't considered to be an effect with the combined values from <Link +>.
/// ..."
/// §4-9-5 "1 card can have a maximum of 1 link card. When linking to a
/// Digimon that has already reached the link limit, the same number of the
/// existing link cards are trashed at the same time as the newly linked
/// cards."
///
/// `LINKPLUS2` (two <Link +1>): maximum 1 + 1 + 1 = 3. With 2 link cards,
/// linking `LINK_A` adds it (3, nothing trashed); with 3, it replaces 1 (still
/// 3, 1 trashed). (The "isn't considered <Link +2>" half is not staged: no
/// effect here references a <Link +> value.)
#[test]
fn link_plus_adds_to_max_link_cards_and_instances_add() {
    cite!(
        "16-40-1",
        "2636d95bc17ed9aa333e5b8e2b8116ea6e5e14032b2f8b1498afef57c45132cb"
    );
    cite!(
        "16-40-3",
        "53808aa3e4596d6b262f330ec958fb2138d92e62ba7cf9a626fe405651381b9c"
    );
    assert_link_adds("LINKPLUS2", 2, false);
    assert_link_replaces("LINKPLUS2", 3, false);
}

/// §4-9-5 "1 card can have a maximum of 1 link card. When linking to a
/// Digimon that has already reached the link limit, the same number of the
/// existing link cards are trashed at the same time as the newly linked
/// cards."
///
/// `LINK_HOST_LV4` (no <Link +>): with no link card, linking `LINK_A` adds it;
/// with 1, linking `LINK_A` is legal and replaces it (still 1 link card,
/// `LINK_B` trashed).
#[test]
fn link_max_base_is_one() {
    cite!(
        "4-9-5",
        "fcec093c198b9ce1bb55b9d769eefe9b189ae82d63c7002091435e834fb2c980"
    );
    assert_link_adds("LINK_HOST_LV4", 0, false);
    assert_link_replaces("LINK_HOST_LV4", 1, false);
}

/// §4-9-5 "When linking to a Digimon that has already reached the link
/// limit, the same number of the existing link cards are trashed at the same
/// time as the newly linked cards." -- for a declared link (the twin of
/// `effect_link_at_limit_replaces_existing_link_card_by_choice`).
///
/// P0's `LINKPLUS2` (max 3) holds 3 `LINK_B`; P0 declares the link of
/// `LINK_A` from hand onto it. Before anything is trashed or plugged in, P0
/// (the linking player) is asked which EXISTING link card is replaced: the 3
/// `LINK_B`, never `LINK_A`, and the host never holds 4. The pick trashes it in
/// the link step itself: nothing is left for a later §17-1-3-2-5 trim.
#[test]
fn declared_link_at_limit_replaces_existing_link_card_by_choice() {
    cite!(
        "4-9-5",
        "fcec093c198b9ce1bb55b9d769eefe9b189ae82d63c7002091435e834fb2c980"
    );
    let mut r = link_board("LINKPLUS2", 3, false);
    let link_a = r.game.player(0).hand[0].handle();
    let existing: Vec<_> = r.game.player(0).battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert!(can_link(&r), "at its limit the host still takes the link");
    r.game.decode_action(HAND_EFFECT_START, 0);
    let mut choice = None;
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.prompt.starts_with("Choose a link card to trash") {
            choice = Some((
                sel.selecting_player,
                sel.effect_choices.clone().unwrap_or_default(),
            ));
            break;
        }
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.execute_action(player, action)
            .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
    }
    let (chooser, options) = choice.expect("P0 chooses the replaced link card");
    assert_eq!(chooser, 0, "the linking player chooses");
    let offered: Vec<_> = options.iter().filter_map(|c| c.source_card).collect();
    assert_eq!(
        offered, existing,
        "exactly the existing link cards are offered"
    );
    assert!(!offered.contains(&link_a), "the new card is not an option");
    assert_eq!(
        r.game.player(0).battle_area[0].linked_cards.len(),
        3,
        "nothing trashed or plugged in before the choice; never 4"
    );
    assert!(trash_ids(&r, 0).is_empty(), "{:?}", trash_ids(&r, 0));

    let pick = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, pick).expect("pick the first LINK_B");
    assert!(
        r.pending_selection().is_none(),
        "the trash happened in the link step; no §17-1-3-2-5 trim follows: {:?}",
        r.pending_selection()
    );
    let linked: Vec<_> = r.game.player(0).battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked.len(), 3);
    assert!(linked.contains(&link_a), "LINK_A plugged in");
    assert!(!linked.contains(&existing[0]), "the picked LINK_B left");
    assert_eq!(trash_ids(&r, 0), vec!["LINK_B".to_string()]);
}

/// §4-9-5 "When linking to a Digimon that has already reached the link
/// limit, the same number of the existing link cards are trashed at the same
/// time as the newly linked cards." -- for a link made by an effect.
///
/// P0's `LINKPLUS2` (max 3) holds 3 `LINK_B`. P0 plays `EFFLINK` ("[On Play]
/// Link 1 Digimon card from your hand to 1 of your Digimon") and links
/// `LINK_A` onto it. P0 must choose which EXISTING link card (one of the 3
/// `LINK_B`) is trashed, as part of the link; `LINK_A` is never one of the
/// options.
///
/// Red: the effect path (`link_cards` → `link_chosen_card_into_host`) adds
/// `LINK_A` as a 4th link card, and the §17-1-3-2-5 rule check then asks P0
/// to trim one of all 4, `LINK_A` included
/// (`G-ENGINE-EFFECT-LINK-AT-LIMIT-NO-REPLACEMENT-CHOICE`).
#[test]
fn effect_link_at_limit_replaces_existing_link_card_by_choice() {
    cite!(
        "4-9-5",
        "fcec093c198b9ce1bb55b9d769eefe9b189ae82d63c7002091435e834fb2c980"
    );
    let mut r = board(&["EFFLINK", "LINK_A"], &[]);
    let host = r.place_on_field(0, "LINKPLUS2", Some(0));
    for _ in 0..3 {
        r.push_linked_owned(host, "LINK_B", 0);
    }
    r.game.tick_declarative_effects();
    r.play(0, 0).expect("EFFLINK is played");
    let link_a = r.game.player(0).hand[0].handle();
    let mut choice = None;
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.prompt.starts_with("Choose a link card to trash") {
            choice = Some(sel.effect_choices.clone().unwrap_or_default());
            break;
        }
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.execute_action(player, action)
            .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
    }
    let choice = choice.expect("P0 chooses the replaced link card");
    assert_eq!(choice.len(), 3, "the 3 existing link cards: {choice:?}");
    assert!(
        choice.iter().all(|c| c.source_card != Some(link_a)),
        "the newly linked card is not a replacement option: {choice:?}"
    );
}

/// §4-9-5 "When linking to a Digimon that has already reached the link
/// limit, the same number of the existing link cards are trashed at the same
/// time as the newly linked cards." -- for an effect that links its own
/// standing Digimon (the relink twin of
/// `effect_link_at_limit_replaces_existing_link_card_by_choice`).
///
/// P0's `LINKPLUS2` (max 3) holds 3 `LINK_B`. P0 plays `RELINK` ("[On Play]
/// Link this Digimon to 1 of your other Digimon") and picks `LINKPLUS2`.
/// Before anything moves, P0 (the linking player) is asked which EXISTING link
/// card is replaced: the 3 `LINK_B`, never `RELINK`; the host never holds 4
/// and `RELINK` is still standing. The pick trashes it in the link itself:
/// nothing is left for a later §17-1-3-2-5 trim.
#[test]
fn effect_relink_at_limit_offers_the_same_choice() {
    cite!(
        "4-9-5",
        "fcec093c198b9ce1bb55b9d769eefe9b189ae82d63c7002091435e834fb2c980"
    );
    let mut r = board(&["RELINK"], &[]);
    let host = r.place_on_field(0, "LINKPLUS2", Some(0));
    for _ in 0..3 {
        r.push_linked_owned(host, "LINK_B", 0);
    }
    r.game.tick_declarative_effects();
    let relink = r.game.player(0).hand[0].handle();
    let existing: Vec<_> = r.game.player(0).battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    r.play(0, 0).expect("RELINK is played");
    let mut choice = None;
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else {
            break;
        };
        if sel.prompt.starts_with("Choose a link card to trash") {
            choice = Some((
                sel.selecting_player,
                sel.effect_choices.clone().unwrap_or_default(),
            ));
            break;
        }
        let (player, action) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.execute_action(player, action)
            .unwrap_or_else(|e| panic!("pick rejected: {e:?}"));
    }
    let (chooser, options) = choice.expect("P0 chooses the replaced link card");
    assert_eq!(chooser, 0, "the linking player chooses");
    let offered: Vec<_> = options.iter().filter_map(|c| c.source_card).collect();
    assert_eq!(
        offered, existing,
        "exactly the existing link cards are offered"
    );
    assert!(!offered.contains(&relink), "the relinked card is not an option");
    assert_eq!(
        r.game.player(0).battle_area[0].linked_cards.len(),
        3,
        "nothing trashed or linked before the choice; never 4"
    );
    assert!(
        r.game.player(0).battle_area.iter().any(|p| p.top_card().handle() == relink),
        "RELINK is still standing during the choice"
    );
    assert!(trash_ids(&r, 0).is_empty(), "{:?}", trash_ids(&r, 0));

    let pick = r.pending_selection().unwrap().valid_action_ids[0];
    r.execute_action(0, pick).expect("pick the first LINK_B");
    assert!(
        r.pending_selection().is_none(),
        "the trash happened in the link; no §17-1-3-2-5 trim follows: {:?}",
        r.pending_selection()
    );
    let linked: Vec<_> = r.game.player(0).battle_area[0]
        .linked_cards
        .iter()
        .map(|c| c.handle())
        .collect();
    assert_eq!(linked.len(), 3);
    assert!(linked.contains(&relink), "RELINK linked");
    assert!(!linked.contains(&existing[0]), "the picked LINK_B left");
    assert_eq!(trash_ids(&r, 0), vec!["LINK_B".to_string()]);
}

// ─── §16-41 <Training> ──────────────────────────────────────────────────────

/// `TRAIN`'s printed face, as production loads it (the DSL's `grant_keyword:
/// Training` is a declaration only; see `ch15_08/TRAIN.yaml`).
fn train_card() -> CardData {
    let mut card = make_test_card("TRAIN", "Drillmon");
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    let card = printed(
        card,
        concat!(
            "＜Training＞ (In the main phase, by suspending this Digimon, place your deck's top ",
            "card face down as this Digimon's bottom digivolution card. This effect can also ",
            "activate in the breeding area.)"
        ),
    );
    assert!(
        card.keywords.contains(&Keyword::Training),
        "{:?}",
        card.keywords
    );
    card
}

/// P0 (deck `[FILL x5, OD_MEM]`, `OD_MEM` on top) to move.
fn training_board() -> DebugRunner {
    ch16_proc()
        .add_card(train_card())
        .deck(0, &[FILL, FILL, FILL, FILL, FILL, "OD_MEM"])
        .deck(1, &[FILL; 6])
        .security(0, &[FILL; 3])
        .security(1, &[FILL; 3])
        .memory(5)
        .start()
}

/// The breeding-area [Main] action bit.
fn breeding_main_action() -> u16 {
    FIELD_EFFECT_START + BREEDING_TARGET * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

/// §16-41-1 "<Training> is a keyword effect that reads "By suspending this
/// Digimon during the main phase, place the top card of your deck at the
/// bottom of this Digimon's digivolution cards. This effect can also activate
/// in the breeding area.""
/// §16-41-3 ""By suspending this Digimon" in <Training> is an optional
/// processing condition. If this condition is performed, "place the top card
/// of your deck at the bottom of this Digimon's digivolution cards" is
/// mandatory processing."
///
/// - Battle area: P0 declares `TRAIN`'s <Training>: it is suspended and, with
///   no decision, `OD_MEM` (the deck's top card) becomes its bottom
///   digivolution card, face down.
/// - Breeding area: the same, for a `TRAIN` in P0's breeding area.
#[test]
fn training_optional_cost_then_mandatory_place_also_in_breeding_area() {
    cite!(
        "16-41-1",
        "2cd1d3cb4507919859aeb143296cad00dd204c1c442f6fac6b2970caa05e0c44"
    );
    cite!(
        "16-41-3",
        "dc9cf60b12212696d99b172936aeb439e90be9e94a41d79ed9870204905d9c18"
    );
    let mut r = training_board();
    let train = r.place_on_field(0, "TRAIN", Some(0));
    r.game.decode_action(field_main_action(train), 0);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    assert!(is_suspended(&r, train));
    assert_eq!(stack_ids(&r, train), owned(&["OD_MEM", "TRAIN"]));
    assert!(r.game.players[0].battle_area[train.index as usize].card_sources[0].face_down);
    assert_eq!(r.deck_size(0), 5);

    let mut r = training_board();
    r.place_in_breeding(0, "TRAIN");
    assert!(
        legal(&r, 0, breeding_main_action()),
        "can activate in the breeding area"
    );
    r.game.decode_action(breeding_main_action(), 0);
    assert!(
        r.pending_selection().is_none(),
        "{:?}",
        r.pending_selection()
    );
    let breeding = r.game.players[0]
        .breeding_area
        .as_ref()
        .expect("breeding Digimon");
    assert!(breeding.is_suspended);
    assert_eq!(
        card_ids(&r, &breeding.card_sources),
        owned(&["OD_MEM", "TRAIN"])
    );
    assert!(breeding.card_sources[0].face_down);
}

/// §16-41-2 "<Training> is an activation-type effect. ..."
///
/// Nothing activates it by itself: in P0's main phase it is a declarable
/// action (its bit and PASS both legal, nothing pending), and when P0 ends
/// the turn without declaring it `TRAIN` is unchanged.
#[test]
fn training_kind_matches_rule() {
    cite!(
        "16-41-2",
        "ef9db98b8f4a1eaa8623abd19ca936819885c3c12f5e1e17bd0decb22fcd16ec"
    );
    let mut r = training_board();
    let train = r.place_on_field(0, "TRAIN", Some(0));
    assert!(r.pending_selection().is_none());
    assert!(legal(&r, 0, field_main_action(train)));
    assert!(legal(&r, 0, PASS));
    next_turn(&mut r);
    assert_eq!(stack_ids(&r, train), owned(&["TRAIN"]));
    assert_eq!(r.deck_size(0), 6);
}

/// §16-41-3 ""By suspending this Digimon" in <Training> is an OPTIONAL
/// processing condition. If this condition is performed, "place ..." is
/// MANDATORY processing."
///
/// Declaring it is P0's choice (above); once declared, nothing lets P0 skip
/// the placement: the card is placed with no prompt. (Matrix row `Training`:
/// `optional_cost_then_mandatory`, agrees.)
#[test]
fn training_optionality_matches_rule() {
    cite!(
        "16-41-3",
        "dc9cf60b12212696d99b172936aeb439e90be9e94a41d79ed9870204905d9c18"
    );
    let mut r = training_board();
    let train = r.place_on_field(0, "TRAIN", Some(0));
    assert!(legal(&r, 0, PASS), "not declaring is allowed");
    r.game.decode_action(field_main_action(train), 0);
    assert!(r.pending_selection().is_none(), "nothing to decline");
    assert!(is_suspended(&r, train));
    assert_eq!(stack_ids(&r, train).len(), 2);
}

// ─── §4-8-1: a trashed TOP card is not a digivolution card ──────────────────

/// §4-8-1 "A digivolution card refers to a card placed under a Digimon."
///
/// An effect that trashes a Digimon's TOP card ("trash the top card of 1 of
/// your Digimon", the Burst Digivolve turn-end trash idiom) trashes no
/// digivolution card: the card was on top, not under the Digimon. P0's
/// `OPT_TRASH_OWN_TOP` on P0's own `[FILL, LV4_PLAIN, LV5_PLAIN]`: P0's
/// `OBS_SRC_TRASH_MEM` ("When cards are trashed from the digivolution cards
/// of one of your Digimon, gain 1 memory") does not trigger, and P0's
/// `OBS_TOP_TRASH_MEM` ("When the top stacked card of one of your Digimon is
/// trashed, gain 1 memory") triggers once. "Trash the top 2 stacked cards"
/// (`OPT_TRASH_OWN_TOP2`, the BT21-030 idiom) trashes both by ONE event: the
/// top-card observer triggers once for the two cards (§16-12-5's "trashed at
/// the same time" reading, DCGO `ITrashStack`), the source observer never.
#[test]
fn trashing_a_top_card_is_not_a_digivolution_card_trash() {
    cite!(
        "4-8-1",
        "c7f86609e036189f3d1409393cec5bb311f782d477519631c8aa7d60f34ee357"
    );
    let mut r = board(&["OPT_TRASH_OWN_TOP"], &[]);
    let stack = r.place_stack(0, &[FILL, "LV4_PLAIN", "LV5_PLAIN"]);
    r.place_on_field(0, "OBS_SRC_TRASH_MEM", Some(0));
    r.place_on_field(0, "OBS_TOP_TRASH_MEM", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "OPT_TRASH_OWN_TOP");
    pick_field(&mut r, stack.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(
        stack_ids(&r, stack),
        owned(&[FILL, "LV4_PLAIN"]),
        "the top card is trashed"
    );
    assert_eq!(
        memory_events_from(&r, cp, "OBS_SRC_TRASH_MEM"),
        0,
        "no digivolution card was trashed"
    );
    assert_eq!(
        memory_events_from(&r, cp, "OBS_TOP_TRASH_MEM"),
        1,
        "the top card was trashed"
    );

    let mut r = board(&["OPT_TRASH_OWN_TOP2"], &[]);
    let stack = r.place_stack(0, &[FILL, "LV4_PLAIN", "LV5_PLAIN"]);
    r.place_on_field(0, "OBS_SRC_TRASH_MEM", Some(0));
    r.place_on_field(0, "OBS_TOP_TRASH_MEM", Some(0));
    let cp = r.event_checkpoint();
    use_option(&mut r, 0, "OPT_TRASH_OWN_TOP2");
    pick_field(&mut r, stack.index as u16);
    resolve_prompts(&mut r);
    assert_eq!(stack_ids(&r, stack), owned(&[FILL]), "2 top cards trashed");
    assert_eq!(memory_events_from(&r, cp, "OBS_SRC_TRASH_MEM"), 0);
    assert_eq!(
        memory_events_from(&r, cp, "OBS_TOP_TRASH_MEM"),
        1,
        "one event for the two cards"
    );
}
