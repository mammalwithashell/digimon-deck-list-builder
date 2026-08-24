//! BT19-101 ZeedMillenniummon — Digimon, Lv.7, Red/Purple/Black, DP 16000,
//! Cost 16. Traits: Wicked God. Form: Mega. Attribute: Virus.
//!
//! # Card text (official Bandai DB / data/card_bundles/BT19-101.md — verbatim)
//!
//! ```text
//! <Overclock ([Composite] Trait)> (At the end of your turn, by deleting 1 of
//!   your Tokens or other [Composite] trait Digimon, this Digimon attacks a
//!   player without suspending.)
//! [On Play] [When Digivolving] [When Attacking] By returning 1 Digimon card
//!   from your opponent's trash to the top of the deck, return 1 of their
//!   Digimon to the bottom of the deck.
//! [All Turns] This Digimon with no digivolution cards can't be suspended,
//!   and isn't affected by your opponent's effects.
//! ```
//!
//! Digivolution: standard Lv.6 (red/purple/black) / cost 6; alt-path
//! `[Digivolve] [MoonMillenniummon]: Cost 2` (data/cards.json `xros_req`).
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT19/Red/BT19_101.cs`
//!
//! ## DCGO crosscheck (per clause)
//!
//! - Alternate Digivolution (`EffectTiming.None`):
//!   `AddSelfDigivolutionRequirementStaticEffect` gated on
//!   `TopCard.EqualsCardName("MoonMillenniummon")`, `digivolutionCost: 2`. →
//!   `alt_paths: [{ kind: digivolve, from: { name_is: "MoonMillenniummon" },
//!   cost: 2 }]` alongside the standard Lv.6/cost 6 evo box.
//! - Overclock (`EffectTiming.OverclockSelfEffect`, trait "Composite"): a
//!   face-up `grant_keyword: Overclock` with `overclock_cost_filter: { any_of:
//!   [{ kind: token }, { all_of: [{ kind: digimon }, { trait_has: Composite
//!   }] }] }` — same shape as EX7-027/BT22-036 Puppet-trait Overclock cards.
//! - On Play / When Digivolving / When Attacking (`EffectTiming.
//!   OnEnterFieldAnyone` x2 shared `ActivateCoroutine` + `OnAllyAttack`):
//!   **OPTIONAL** — `SetUpActivateClass(..., isOptional: true, ...)` at
//!   `BT19_101.cs:44` / `:161` / `:278`, and the inner `SelectCardEffect` is
//!   additionally `canNoSelect: () => true`. `general_rule.pdf` §15-7-1 names
//!   "By X, Y" an OPTIONAL PROCESSING CONDITION and §15-7-4 lets the player
//!   decline it "regardless of whether or not the content of the conditions
//!   can be executed"; §15-7-2 makes the decline skip the post-condition
//!   processing too. The outer `CanActivateCondition`
//!   (`Enemy.TrashCards.Count(IsDigimon) >= 1`) only decides whether the
//!   clause activates at all — it does NOT convert the payment into a
//!   mandatory one. (This doc previously claimed "the pick is forced
//!   through"; corrected.) Once ACCEPTED: `SelectCardEffect` selects 1 Digimon
//!   card from the opponent's trash → `AddLibraryTopCards` (returns it to the
//!   top of THEIR deck); `AfterSelectCardCoroutine` then mandatorily
//!   `SelectPermanentEffect` (`Mode.PutLibraryBottom`, `maxCount: min(1, opp
//!   Digimon count)`) over the opponent's battle-area Digimon → bottom-decks
//!   1. → clause `optional: true` + `outer_prompt: true` (the body's first
//!   step `select_trash` is not itself declinable), then `select_trash: { of:
//!   opponent, filter: { kind: digimon } }` → `move_trash_card_to_deck_top: {
//!   of: opponent }`, then `select_opponent_permanent: { kind: digimon }`
//!   (mandatory once accepted, silently no-ops with 0 opponent Digimon) →
//!   `return_to_deck: { position: bottom }`.
//! - All Turns (`EffectTiming.None` x2 — `CanNotAffectedClass` +
//!   `CanNotSuspendClass`, both gated on the SHARED
//!   `ImmunityAndNoSuspendCondition`: `card.PermanentOfThisCard().
//!   DigivolutionCards.Count == 0`): two self-auras sharing `active_when: {
//!   stack_size_lte: 1 }` (no digivolution cards — `card_sources.len() ==
//!   1`, BT1-085/BT12-031/BT17-077 idiom) — one installs `modifier:
//!   CannotSuspend` (permanent-scoped enforcement is RESOLVED 2026-06-13, see
//!   `docs/RUST_ENGINE_GAPS.md`), the other installs `effect_immunity: {
//!   source_controller: opponent }` (all source kinds — no `source_kind`
//!   filter, per DCGO's `IsOpponentEffect` SkillCondition with NO
//!   `IsDigimonEffect` gate, unlike EX8-073's Digimon-only variant;
//!   G-DSL-AURA-EFFECT-IMMUNITY, EX8-073/BT17-016 idiom).
//!
//! # Patterns this test covers
//! - H11 Overclock (parametric `[Composite]` trait cost filter).
//! - Shared [On Play][When Digivolving][When Attacking] trash-return cost +
//!   mandatory bottom-deck follow-up (BT23-059-style 3-timing clause; EX10-068
//!   / LM-030 `select_trash` + `move_trash_card_to_deck_top` idiom).
//! - F6/F7 combined: `stack_size_lte: 1`-gated CannotSuspend + all-source-kind
//!   ImmunityToOpponentEffects self-auras (BT17-016/EX8-073 `effect_immunity`
//!   idiom, BT1-085/BT17-077 `stack_size_lte: 1` "no digivolution cards"
//!   idiom).
//! - G1-adjacent named alt-digivolve (`from: { name_is: "MoonMillenniummon"
//!   }`, cost 2) alongside the standard evo box.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, Expiry, Keyword, ModifierType,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT19-101";

fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}

fn token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.traits = vec!["Token".to_string()];
    card
}

fn composite_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.traits = vec!["Composite".to_string()];
    card
}

fn non_composite_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

fn moon_millenniummon() -> CardData {
    let mut card = make_test_card("MOON-MILLENNIUMMON", "MoonMillenniummon");
    card.card_kind = CardKind::Digimon;
    card.level = Some(7);
    card.colors = vec![CardColor::Purple];
    card.dp = Some(15000);
    card
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-101 in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 4000))
        .add_card(opp_digimon("OPP-B", 5000))
        .add_card(token("TOKEN-A"))
        .add_card(composite_digimon("COMPOSITE-A"))
        .add_card(non_composite_digimon("PLAIN-A"))
        .add_card(moon_millenniummon())
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(10)
        .start()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

/// Drop a card straight into `player`'s trash (bypassing draw/discard).
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let src = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(src);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_101_compiles_from_dsl_pack() {
    let card = compiled(CARD_ID);
    assert_eq!(card.level, Some(7));
    assert_eq!(card.cost, Some(16));
    assert_eq!(card.dp, Some(16000));
    assert!(card.traits.iter().any(|t| t == "Wicked God"));
}

#[test]
fn bt19_101_has_standard_and_named_alt_digivolve_paths() {
    let card = compiled(CARD_ID);
    let standard = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(6))
    });
    assert!(
        standard.is_some(),
        "standard Lv.6/cost 6 digivolve box must compile"
    );

    let named = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
    });
    assert!(
        named.is_some(),
        "named [MoonMillenniummon] cost-2 alt-digivolve must compile"
    );
}

#[test]
fn bt19_101_trash_return_clause_covers_all_three_timings() {
    let card = compiled(CARD_ID);
    let triple = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Triggered(t)
                    if t.when.contains(&CompiledTiming::OnPlay)
                        && t.when.contains(&CompiledTiming::WhenDigivolving)
                        && t.when.contains(&CompiledTiming::WhenAttacking)
            )
        })
        .count();
    assert_eq!(
        triple, 1,
        "one shared [On Play][When Digivolving][When Attacking] clause"
    );
}

#[test]
fn bt19_101_grants_overclock_keyword_declaratively() {
    let card = compiled(CARD_ID);
    let has_overclock_grant = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Overclock"
        )
    });
    assert!(
        has_overclock_grant,
        "Overclock must be a face-up keyword grant"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — <Overclock ([Composite] Trait)>
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt19_101_has_overclock_while_face_up() {
    let mut runner = base_runner();
    let zeed = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(zeed, Keyword::Overclock));
}

#[test]
fn bt19_101_overclock_sacrifice_accepts_token() {
    let mut runner = base_runner();
    runner.set_first_player(0);
    let zeed = runner.place_on_field(0, CARD_ID, Some(0));
    let _tok = runner.place_on_field(0, "TOKEN-A", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::EndOfTurnAction;

    runner
        .game
        .activate_overclock(zeed.index as usize)
        .expect("Overclock with a Token sacrifice available must install a prompt");

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("pending selection installed");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert!(sel.is_optional, "<Overclock> may be declined");
}

#[test]
fn bt19_101_overclock_sacrifice_accepts_other_composite_digimon() {
    let mut runner = base_runner();
    runner.set_first_player(0);
    let zeed = runner.place_on_field(0, CARD_ID, Some(0));
    let _composite = runner.place_on_field(0, "COMPOSITE-A", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::EndOfTurnAction;

    runner
        .game
        .activate_overclock(zeed.index as usize)
        .expect("a [Composite]-trait Digimon is a legal Overclock sacrifice");
}

#[test]
fn bt19_101_overclock_sacrifice_rejects_non_composite_non_token() {
    let mut runner = base_runner();
    runner.set_first_player(0);
    let zeed = runner.place_on_field(0, CARD_ID, Some(0));
    let _plain = runner.place_on_field(0, "PLAIN-A", Some(0));
    runner.game.current_phase = digimon_engine::enums::GamePhase::EndOfTurnAction;

    let err = runner.game.activate_overclock(zeed.index as usize);
    assert!(
        err.is_err(),
        "a non-Token, non-[Composite] Digimon is not a legal Overclock sacrifice"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play][When Digivolving][When Attacking] trash-return cost
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: with >=1 opponent-trash Digimon, the shared clause first surfaces
/// the OUTER accept/decline confirm for its optional processing condition;
/// accepting then installs the Trash-selection cost prompt.
#[test]
fn bt19_101_on_play_with_opp_trash_digimon_installs_trash_prompt() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::OnPlay, zeed);

    // §15-7-1: "Optional processing conditions include text such as 'by X,
    // Y.'" — "By returning 1 Digimon card from your opponent's trash to the
    // top of the deck, return 1 of their Digimon to the bottom of the deck" is
    // that shape. §15-7-4: the player "can choose whether or not to execute
    // the content of optional processing conditions, regardless of whether or
    // not the content of the conditions can be executed". So the FIRST thing
    // to surface is the accept/decline confirm, not the cost pick.
    //
    // This test used to assert `!view.is_optional` on a Trash-kind prompt,
    // reasoning that "the pick is mandatory once the clause activates (DCGO
    // canNoSelect never true here)". Both halves were wrong: DCGO's
    // `BT19_101.cs` passes `isOptional: true` to `SetUpActivateClass` on all
    // three timings (:44, :161, :278), and its inner `SelectCardEffect` IS
    // `canNoSelect: () => true`. A satisfiable outer `CanActivateCondition`
    // does not convert an optional processing condition into a mandatory one
    // (§15-7-4's "regardless of whether ... can be executed").
    let outer = r
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert_eq!(
        outer.kind,
        SelectionKind::Replacement,
        "the outer accept/decline confirm installs before any cost is paid"
    );
    assert!(
        outer.is_optional,
        "the outer confirm must expose PASS so the player can decline (§15-7-4)"
    );

    // Accepting reaches the (now committed) cost pick.
    r.accept_optional_trigger()
        .expect("accepting the optional processing condition must be reachable");
    let view = r
        .pending_selection_view()
        .expect("opponent-trash Digimon cost prompt must install after accept");
    assert_eq!(view.kind, SelectionKind::Trash);
}

/// NEGATIVE: with NO opponent-trash Digimon, the clause does not activate —
/// no prompt installs at all (outer CanActivateCondition gate).
#[test]
fn bt19_101_on_play_without_opp_trash_digimon_does_nothing() {
    let mut r = base_runner();
    let zeed = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::OnPlay, zeed);

    assert!(
        r.pending_selection().is_none(),
        "no opponent-trash Digimon -> the clause does not fire"
    );
}

/// Behavioral: paying the cost moves the chosen trash Digimon to the TOP of
/// the opponent's deck, then a mandatory opponent-permanent pick bottom-decks
/// 1 of their Digimon.
#[test]
fn bt19_101_on_play_returns_trash_to_top_then_bottom_decks_opp_digimon() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    let opp_perm = r.place_on_field(1, "OPP-B", Some(0));

    let opp_deck_before = r.game.players[1].deck.len();
    let opp_trash_before = r.game.players[1].trash.len();

    fire(&mut r, EffectTiming::OnPlay, zeed);

    // §15-7-1/§15-7-4: the optional processing condition prompts first; this
    // test drives the ACCEPT branch (the decline branch is covered by
    // `bt19_101_optional_processing_condition_may_be_declined`).
    r.accept_optional_trigger()
        .expect("accept the optional 'by returning ...' condition");
    let cost_view = r
        .pending_selection_view()
        .expect("trash-return cost prompt");
    assert_eq!(cost_view.kind, SelectionKind::Trash);
    r.execute_action(0, cost_view.valid_action_ids[0])
        .expect("select the opponent-trash Digimon");

    // Cost paid: trash card returned to opponent's deck TOP; trash shrinks.
    assert_eq!(r.game.players[1].trash.len(), opp_trash_before - 1);
    assert_eq!(r.game.players[1].deck.len(), opp_deck_before + 1);
    assert_eq!(
        r.game.players[1]
            .deck
            .last()
            .map(|cs| cs.card_id(&r.game.card_data)),
        Some("OPP-A"),
        "the returned card must sit on TOP of the opponent's deck"
    );

    let bounce_view = r
        .pending_selection_view()
        .expect("mandatory opponent-Digimon bottom-deck follow-up");
    assert_eq!(bounce_view.kind, SelectionKind::OppField);
    r.execute_action(0, bounce_view.valid_action_ids[0])
        .expect("select the opponent Digimon to bottom-deck");
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(1),
        0,
        "the opponent Digimon left the field"
    );
    assert_eq!(
        r.game.players[1]
            .deck
            .first()
            .map(|cs| cs.card_id(&r.game.card_data)),
        Some("OPP-B"),
        "the bounced Digimon lands at the BOTTOM of the deck"
    );
    let _ = opp_perm;
}

/// Mandatory bottom-deck follow-up still silently no-ops if the opponent has
/// no Digimon on the field (DCGO: maxCount min(1, count) == 0).
#[test]
fn bt19_101_on_play_bottom_deck_step_noops_with_no_opp_digimon_on_field() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::OnPlay, zeed);
    // §15-7-1/§15-7-4: accept the optional processing condition first.
    r.accept_optional_trigger()
        .expect("accept the optional 'by returning ...' condition");
    let cost_view = r
        .pending_selection_view()
        .expect("trash-return cost prompt");
    r.execute_action(0, cost_view.valid_action_ids[0])
        .expect("select the opponent-trash Digimon");
    let _ = r.auto_resolve();

    assert!(
        r.pending_selection().is_none(),
        "with no opponent battle-area Digimon the bottom-deck pick silently skips"
    );
}

/// The same shared clause fires on [When Digivolving].
#[test]
fn bt19_101_when_digivolving_fires_same_clause() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::WhenDigivolving, zeed);
    // 15-7-1/15-7-4: the outer accept/decline confirm gates every timing
    // of this shared clause — DCGO passes isOptional=true on all three.
    r.accept_optional_trigger()
        .expect("accept the optional 'by returning ...' condition");
    let view = r
        .pending_selection_view()
        .expect("[When Digivolving] also surfaces the trash-return cost");
    assert_eq!(view.kind, SelectionKind::Trash);
}

/// The same shared clause fires on [When Attacking].
#[test]
fn bt19_101_when_attacking_fires_same_clause() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));

    fire(&mut r, EffectTiming::WhenAttacking, zeed);
    // 15-7-1/15-7-4: the outer accept/decline confirm gates every timing
    // of this shared clause — DCGO passes isOptional=true on all three.
    r.accept_optional_trigger()
        .expect("accept the optional 'by returning ...' condition");
    let view = r
        .pending_selection_view()
        .expect("[When Attacking] also surfaces the trash-return cost");
    assert_eq!(view.kind, SelectionKind::Trash);
}

/// Event-log check: the trash-return cost + bottom-deck follow-up is a
/// pure trash<->deck zone move — it must NOT touch the security stack
/// (no `SecurityReveal` event) even though the same card carries a separate
/// [When Attacking] timing that could plausibly be confused with a security
/// check.
#[test]
fn bt19_101_trash_return_cost_does_not_touch_security() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    let _opp_perm = r.place_on_field(1, "OPP-B", Some(0));

    let cp = r.event_checkpoint();
    fire(&mut r, EffectTiming::OnPlay, zeed);
    // §15-7-1/§15-7-4: accept the optional processing condition first.
    r.accept_optional_trigger()
        .expect("accept the optional 'by returning ...' condition");
    let cost_view = r
        .pending_selection_view()
        .expect("trash-return cost prompt");
    r.execute_action(0, cost_view.valid_action_ids[0])
        .expect("select the opponent-trash Digimon");
    let _ = r.auto_resolve();

    let events = r.events_since(cp);
    let security_noise = events
        .iter()
        .filter(|e| matches!(e, digimon_engine::events::GameEvent::SecurityReveal { .. }))
        .count();
    assert_eq!(
        security_noise, 0,
        "a trash<->deck cost/effect pair must not touch the security stack"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] no-digivolution-cards: CannotSuspend + immunity
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE: freshly played (no digivolution cards, stack_size == 1) ⇒
/// CannotSuspend is installed once the declarative tick runs.
#[test]
fn bt19_101_cannot_suspend_while_no_digivolution_cards() {
    let mut r = base_runner();
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.modifiers.has(zeed, ModifierType::CannotSuspend),
        "with no digivolution cards, ZeedMillenniummon gains CannotSuspend"
    );
}

/// NEGATIVE: with >=1 digivolution card (stack_size >= 2), CannotSuspend must
/// NOT be installed.
#[test]
fn bt19_101_no_cannot_suspend_with_digivolution_cards() {
    let mut r = base_runner();
    let zeed = r.place_stack(0, &["FILLER", CARD_ID]);
    r.game.tick_declarative_effects();

    assert!(
        !r.game.modifiers.has(zeed, ModifierType::CannotSuspend),
        "with a digivolution card underneath, CannotSuspend must not apply"
    );
}

/// Enforcement: CannotSuspend actually blocks a basic attack declaration
/// (permanent-scoped enforcement RESOLVED 2026-06-13 — combat/mod.rs
/// can_attack* + action/mask.rs can_basic_attack).
#[test]
fn bt19_101_cannot_suspend_blocks_basic_attack() {
    let mut r = base_runner();
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();
    r.game.current_phase = digimon_engine::enums::GamePhase::Main;

    assert!(
        !r.game.can_attack(zeed, false),
        "CannotSuspend must block basic-attack declaration for a Digimon with no digivolution cards"
    );
}

/// POSITIVE: freshly played (no digivolution cards) ⇒ immune to ALL opponent
/// effect source kinds.
#[test]
fn bt19_101_immune_to_all_opponent_effect_kinds_while_no_digivolution_cards() {
    let mut r = base_runner();
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    r.game.tick_declarative_effects();

    for kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
    ] {
        assert!(
            r.game.permanent_is_unaffected_by_effect(zeed, 1, kind),
            "no digivolution cards: unaffected by opponent {kind:?} effects"
        );
    }
    // ... but still affected by its own controller's effects.
    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(zeed, 0, EffectSourceKind::Digimon),
        "the immunity is opponent-scoped only"
    );
}

/// NEGATIVE: with >=1 digivolution card, the immunity must be off.
#[test]
fn bt19_101_not_immune_with_digivolution_cards() {
    let mut r = base_runner();
    let zeed = r.place_stack(0, &["FILLER", CARD_ID]);
    r.game.tick_declarative_effects();

    assert!(
        !r.game
            .permanent_is_unaffected_by_effect(zeed, 1, EffectSourceKind::Digimon),
        "with a digivolution card underneath, the immunity must be off"
    );
}

/// End-to-end: an opponent's deletion effect fizzles against the immune,
/// sourceless ZeedMillenniummon.
#[test]
fn bt19_101_opponent_delete_effect_fizzles_while_immune() {
    let mut r = base_runner();
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    r.game.tick_declarative_effects();

    let opp_card = r.top_card(opp);
    {
        let mut ctx = EffectContext::new(&mut r.game, opp_card, Some(opp), 1);
        ctx.delete_permanent(zeed);
    }
    r.game.drain_effect_queue();
    assert_eq!(
        r.battle_area_size(0),
        1,
        "the opponent's effect deletion must fizzle against the immunity"
    );
}

/// The stack-size gate is a LIVE, continuously re-evaluated condition (not a
/// one-shot install at OnPlay): shrinking the digivolution stack back down to
/// just the top card (stack_size == 1) turns BOTH the CannotSuspend lock and
/// the immunity ON on the very next declarative tick. Directly truncates
/// `card_sources` to simulate a de-digivolve's end state (the `bt20_083.rs`
/// `card_sources` mutation idiom), since `Game::de_digivolve_core` is
/// `pub(crate)` and not reachable from the external tests binary.
#[test]
fn bt19_101_locks_and_immunity_turn_on_once_stack_shrinks_to_no_sources() {
    let mut r = base_runner();
    let zeed = r.place_stack(0, &["FILLER", CARD_ID]);
    r.game.tick_declarative_effects();
    assert!(!r.game.modifiers.has(zeed, ModifierType::CannotSuspend));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(zeed, 1, EffectSourceKind::Digimon));

    {
        let permanent = &mut r.game.players[zeed.player as usize].battle_area[zeed.index as usize];
        let top = permanent.card_sources.pop().expect("top card exists");
        permanent.card_sources.clear();
        permanent.card_sources.push(top);
    }
    r.game.tick_declarative_effects();

    assert!(
        r.game.modifiers.has(zeed, ModifierType::CannotSuspend),
        "once the stack shrinks to just the top card, CannotSuspend turns on"
    );
    assert!(
        r.game
            .permanent_is_unaffected_by_effect(zeed, 1, EffectSourceKind::Digimon),
        "once the stack shrinks to just the top card, the opponent-effect immunity turns on"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — the optional processing condition may be DECLINED (§15-7)
// ════════════════════════════════════════════════════════════════════════════

/// §15-7-1: "Optional processing conditions include text such as 'by X, Y.'"
/// ZeedMillenniummon's shared clause prints exactly that: "BY RETURNING 1
/// Digimon card from your opponent's trash to the top of the deck, return 1 of
/// their Digimon to the bottom of the deck."
///
/// §15-7-4: "A player can choose whether or not to execute the content of
/// optional processing conditions, REGARDLESS of whether or not the content of
/// the conditions can be executed." So a satisfied clause `condition:` (>=1
/// opponent-trash Digimon) does NOT make the return mandatory.
///
/// §15-7-2: if the condition's content isn't executed, "the processing after
/// the conditions can't be executed" — so declining must leave BOTH halves
/// undone: the opponent's trash card stays in the trash AND their Digimon
/// stays on the field.
///
/// DCGO agrees: `BT19_101.cs` passes `isOptional: true` to
/// `SetUpActivateClass` on all three timings (:44 On Play, :161 When
/// Digivolving, :278 When Attacking).
///
/// This is the branch the engine had no way to reach — the clause fired
/// unconditionally, so the trash-return cost was auto-paid (rule 17: no
/// auto-selections).
#[test]
fn bt19_101_optional_processing_condition_may_be_declined() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    let _opp_perm = r.place_on_field(1, "OPP-B", Some(0));

    let opp_trash_before = r.game.players[1].trash.len();
    let opp_deck_before = r.game.players[1].deck.len();
    let opp_field_before = r.battle_area_size(1);
    assert_eq!(opp_field_before, 1, "precondition: 1 opponent Digimon");

    fire(&mut r, EffectTiming::OnPlay, zeed);

    let outer = r
        .pending_selection_view()
        .expect("the optional processing condition must surface a prompt (rule 17)");
    assert!(
        outer.is_optional,
        "the outer confirm must be optional so PASS is legal (§15-7-4)"
    );
    r.execute_action(outer.selecting_player, PASS)
        .expect("declining must be reachable from the action space");
    let _ = r.auto_resolve();

    // §15-7-2, half 1: the COST was not paid.
    assert_eq!(
        r.game.players[1].trash.len(),
        opp_trash_before,
        "declining must NOT return the opponent's trash Digimon to their deck"
    );
    assert_eq!(
        r.game.players[1].deck.len(),
        opp_deck_before,
        "declining must leave the opponent's deck size untouched"
    );

    // §15-7-2, half 2: "the processing after the conditions can't be executed".
    assert_eq!(
        r.battle_area_size(1),
        opp_field_before,
        "declining must NOT bottom-deck the opponent's Digimon"
    );
    assert!(
        r.pending_selection().is_none(),
        "declining must resolve cleanly with no leftover prompt"
    );
}

/// Declining is reachable on the [When Attacking] timing too — the same shared
/// clause, the same §15-7-4 choice. (DCGO: `BT19_101.cs:278`, isOptional=true.)
#[test]
fn bt19_101_when_attacking_optional_condition_may_be_declined() {
    let mut r = base_runner();
    push_to_trash(&mut r, 1, "OPP-A");
    let zeed = r.place_on_field(0, CARD_ID, Some(0));
    let _opp_perm = r.place_on_field(1, "OPP-B", Some(0));

    let opp_trash_before = r.game.players[1].trash.len();

    fire(&mut r, EffectTiming::WhenAttacking, zeed);

    let outer = r
        .pending_selection_view()
        .expect("[When Attacking] must also surface the accept/decline confirm");
    r.execute_action(outer.selecting_player, PASS)
        .expect("declining must be reachable on the [When Attacking] timing");
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].trash.len(),
        opp_trash_before,
        "declining on [When Attacking] must not pay the trash-return cost"
    );
    assert_eq!(
        r.battle_area_size(1),
        1,
        "declining on [When Attacking] must not bottom-deck the opponent's Digimon"
    );
}
