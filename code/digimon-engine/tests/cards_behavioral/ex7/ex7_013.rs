//! EX7-013 MagnaKidmon — Digimon, Lv.6, Red, DP 12000, Cost 12.
//! Traits: Dragonkin / Three Musketeers. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX7-013.md — official Bandai DB, confirmed
//! against the card image EX7-013.webp)
//!
//! Digivolve: Red Lv.5 / cost 4 (standard circle) and
//! [Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4.
//!
//! [On Play] [When Digivolving] You may use 1 [Three Musketeers] trait Option
//! card from your hand without paying the cost. Then, draw cards from the top
//! of your deck until you have 6 in your hand.
//! [End of Your Turn] [Once Per Turn] By trashing 1 Option card from this
//! Digimon's digivolution cards, 1 of your [Three Musketeers] trait Digimon
//! gains ＜Security A. +1＞ for the turn, and attacks.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Red/EX7_013.cs
//! - Digivolution Condition: AddSelfDigivolutionRequirementStaticEffect
//!   (TopCard.HasText("Three Musketeers") && Level == 5, cost 4).
//! - On Play / When Digivolving (two identical ActivateClass bodies,
//!   `-1, false` = unlimited + mandatory clause): SelectHandEffect
//!   (canNoSelect: true) over IsOption && Three Musketeers trait &&
//!   HasUseCost && !CanNotPlayThisOption → PlayOptionCards(payCost: false).
//!   Then `if (Library >= 1 && Hand < 6) while (Hand < 6) Draw(1)`.
//! - End of Your Turn (`1, true` = OPT + optional; IsOwnerTurn): mandatory
//!   SelectCardEffect over THIS permanent's DigivolutionCards filtered
//!   IsOption → ITrashDigivolutionCards; then SelectPermanentEffect over own
//!   battle-area Digimon → ChangeDigimonSAttack(+1, UntilEachTurnEnd) → if
//!   CanAttack: SelectAttackEffect with SetCanNotSelectNotAttack (mandatory
//!   attack). NOTE: DCGO's permanent filter is ANY own Digimon; the printed
//!   text says "[Three Musketeers] trait Digimon" — printed text wins.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H alt digivolution path (level_eq + in_text_contains)
//! - A/E optional use-Option-from-hand free + formula draw (G-DSL-DRAW-FORMULA-COUNT)
//! - E2 OPT + optional End-of-Your-Turn cost (trash own Option source)
//! - F force_attack after a modifier grant (SecurityAttackChange +1)
//! - Cost observability: the trashed Option source lands in the trash zone

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardColor;
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX7-013";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A visible `[Main]` Option body that draws 1 card for its owner when it
/// resolves, so tests can prove the Option was actually USED (not just moved).
struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
            })
            .build()]
    }
}

fn make_filler(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    // Option-kind filler: a security check on it reveals no effect and just
    // trashes it (no battle), keeping security-attack arithmetic simple.
    card.card_kind = CardKind::Option;
    card
}

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_option(id: &str, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.colors = vec![CardColor::Red];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-013 YAML parses and compiles")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_option("TM-OPT", 6, &["Three Musketeers"]))
        .add_card(make_option("PLAIN-OPT", 4, &[]))
        .add_card(make_digimon("TM-DIGI", 4, 5000, &["Three Musketeers"]))
        .add_card(make_digimon("PLAIN-DIGI", 4, 5000, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI", 4, 4000, &["Beast"]))
        .deck(1, &["DECK-PAD"; 10])
}

fn resolve_trigger_order_if_present(runner: &mut DebugRunner) {
    while matches!(runner.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let act = runner.pending_selection().unwrap().valid_action_ids[0];
        runner
            .execute_action(0, act)
            .expect("resolve TriggerOrder pick");
    }
}

fn fire_on_play(runner: &mut DebugRunner, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn fire_end_of_turn(runner: &mut DebugRunner, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ex7_013_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("EX7-013 in embedded pack");
    assert_eq!(card.name, "MagnaKidmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Dragonkin", "Three Musketeers"] {
        assert!(card.traits.contains(&t.to_string()), "missing trait {t}");
    }
}

#[test]
fn ex7_013_has_lv5_tm_in_text_cost4_alt_path_and_standard_red_circle() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let special = card.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.all_of
                    .iter()
                    .any(|q| q.in_text_contains.as_deref() == Some("Three Musketeers"))
                    && f.all_of.iter().any(|q| q.level_eq == Some(5))
            })
    });
    assert!(
        special.is_some(),
        "must register the printed [Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4 route"
    );
    let standard = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(4))
            && p.from.as_ref().is_some_and(|f| {
                f.all_of.iter().any(|q| q.level_eq == Some(5))
                    && f.all_of.iter().any(|q| q.color_is.is_some())
            })
    });
    assert!(
        standard,
        "must also register the standard Red Lv.5 / cost 4 circle"
    );
}

#[test]
fn ex7_013_has_op_wd_mandatory_clause_and_eot_opt_optional_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "exactly two triggered clauses");

    let op_wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert!(op_wd.when.contains(&CompiledTiming::WhenDigivolving));
    assert_eq!(op_wd.scope, CompiledScope::FaceUp);
    assert!(
        !op_wd.optional,
        "the clause itself is mandatory (only the use is 'you may')"
    );
    assert!(!op_wd.once_per_turn);

    let eot = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::EndOfYourTurn))
        .expect("[End of Your Turn] clause");
    assert_eq!(eot.when, vec![CompiledTiming::EndOfYourTurn]);
    assert!(eot.optional, "DCGO isOptional: true");
    assert!(eot.once_per_turn, "printed [Once Per Turn]");
}

// ─── Section 2 — [On Play][When Digivolving] use Option + draw to 6 ─────────

/// Positive: accept the optional use → the TM Option resolves free (its body
/// draws 1), then the hand is topped up to exactly 6.
#[test]
fn ex7_013_op_uses_tm_option_free_then_draws_to_six() {
    let mut runner = base()
        .hand(0, &["TM-OPT", "DECK-PAD", "DECK-PAD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(3)
        .start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();

    fire_on_play(&mut runner, magna);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "must offer the [Three Musketeers] Option in hand"
    );
    assert!(runner.pending_is_optional(), "'you may use' is declinable");
    let view = runner.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "only the [Three Musketeers] Option is a candidate (fillers are not)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("use the TM Option");
    runner.auto_resolve().expect("resolve remaining prompts");

    assert_eq!(
        runner.hand_size(0),
        6,
        "hand must be topped up to exactly 6"
    );
    assert_eq!(runner.memory(), mem_before, "used without paying the cost");
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"),
        "the used Option is trashed after resolution"
    );
}

/// Negative (decline): declining the optional use still runs the mandatory
/// draw-to-6 tail.
#[test]
fn ex7_013_op_declining_use_still_draws_to_six() {
    let mut runner = base()
        .hand(0, &["TM-OPT", "DECK-PAD", "DECK-PAD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(3)
        .start();
    runner.register_effect("TM-OPT", Arc::new(OptionMainDraw));
    let magna = runner.place_on_field(0, CARD_ID, Some(0));

    fire_on_play(&mut runner, magna);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    runner.execute_action(0, PASS).expect("decline the use");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.hand_size(0),
        6,
        "draw-to-6 runs even when the use is declined"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"),
        "declined Option stays in hand"
    );
}

/// Negative (no candidate): with no [Three Musketeers] Option in hand no use
/// prompt installs; the draw-to-6 still runs.
#[test]
fn ex7_013_op_no_tm_option_skips_use_prompt_and_draws_to_six() {
    let mut runner = base()
        .hand(0, &["PLAIN-OPT", "DECK-PAD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));

    fire_on_play(&mut runner, magna);
    assert!(
        runner.pending_selection().is_none(),
        "a non-[Three Musketeers] Option must not be offered"
    );
    assert_eq!(runner.hand_size(0), 6);
}

/// Boundary: a hand already at 6 or more draws nothing.
#[test]
fn ex7_013_op_hand_already_six_or_more_draws_nothing() {
    let mut runner = base()
        .hand(0, &["DECK-PAD"; 7])
        .deck(0, &["DECK-PAD"; 10])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);

    fire_on_play(&mut runner, magna);
    runner.auto_resolve().ok();

    assert_eq!(runner.hand_size(0), 7, "no draw when already holding 6+");
    assert_eq!(runner.deck_size(0), deck_before);
}

/// Boundary (DCGO `LibraryCards.Count >= 1` guard): an empty deck draws
/// nothing and does NOT deck the player out.
#[test]
fn ex7_013_op_empty_deck_draws_nothing_without_deckout() {
    let mut runner = base().hand(0, &["DECK-PAD", "DECK-PAD"]).memory(3).start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    assert_eq!(runner.deck_size(0), 0);

    fire_on_play(&mut runner, magna);
    runner.auto_resolve().ok();

    assert_eq!(runner.hand_size(0), 2, "nothing to draw from an empty deck");
    assert!(
        !runner.game_over(),
        "the guarded draw must not deck the player out"
    );
}

/// Partial deck: draws only what is needed (hand 2 → 6 = 4 cards).
#[test]
fn ex7_013_op_draws_exactly_the_shortfall() {
    let mut runner = base()
        .hand(0, &["DECK-PAD", "DECK-PAD"])
        .deck(0, &["DECK-PAD"; 10])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));

    fire_on_play(&mut runner, magna);
    runner.auto_resolve().ok();

    assert_eq!(runner.hand_size(0), 6);
    assert_eq!(runner.deck_size(0), 6, "exactly 4 cards drawn");
}

// ─── Section 3 — [End of Your Turn][OPT] trash Option source → SA+1 + attack ─

fn eot_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 10])
        .security(1, &["DECK-PAD"; 3])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(magna, "PLAIN-OPT");
    let tm = runner.place_on_field(0, "TM-DIGI", Some(0));
    (runner, magna, tm)
}

/// Positive: accept → trash the Option source → the chosen [Three Musketeers]
/// Digimon gains Security A. +1 and is forced to attack (2 security checks).
#[test]
fn ex7_013_eot_trashes_option_source_grants_sa_plus_one_and_attacks() {
    let (mut runner, magna, tm) = eot_runner();
    let sec_before = runner.security_count(1);
    let cp = runner.event_checkpoint();

    fire_end_of_turn(&mut runner, magna);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "optional clause installs an outer accept/decline gate"
    );
    runner.accept_optional_trigger().expect("accept");

    // Cost: pick the Option digivolution card under MagnaKidmon.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: UnionZoneSet::MATERIAL
        }),
        "the cost pick is rooted at this Digimon's digivolution cards"
    );
    assert!(
        !runner.pending_is_optional(),
        "once activated the trash cost is mandatory"
    );
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, act)
        .expect("trash the Option source");

    // Target: 1 of your [Three Musketeers] Digimon.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().unwrap();
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, tm.index as u16)),
        "the [Three Musketeers] Digimon must be a candidate"
    );
    runner
        .execute_action(0, encode_attack(0, tm.index as u16))
        .expect("pick the TM Digimon");

    assert!(
        runner
            .game
            .modifiers
            .has(tm, ModifierType::SecurityAttackChange),
        "the chosen Digimon gains <Security A. +1> for the turn"
    );

    // "…and attacks." — mandatory attack prompt.
    let pending = runner
        .pending_selection()
        .expect("force_attack installs a mandatory attack-target prompt");
    assert!(!pending.is_optional, "'and attacks' is not declinable");
    let atk = encode_attack(tm.index as u16, SECURITY_TARGET);
    assert!(pending.valid_action_ids.contains(&atk));
    runner.execute_action(0, atk).expect("attack the player");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(1),
        sec_before - 2,
        "Security A. +1 → two security cards checked"
    );
    let stack = &runner.game.players[0].battle_area[magna.index as usize].card_sources;
    assert_eq!(stack.len(), 1, "the Option source was trashed as the cost");
    // The engine logs whole-permanent trashes as `GameEvent::Trash`; a
    // digivolution-source trash is observable via the trash zone itself (and
    // the `OnDigivolutionCardTrashed` observer timing), so assert the zone.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PLAIN-OPT"),
        "the trashed Option source must land in its owner's trash"
    );
    let _ = cp;
}

/// Negative (decline): declining the outer gate leaves the stack and the
/// opponent's security untouched.
#[test]
fn ex7_013_eot_decline_leaves_everything_untouched() {
    let (mut runner, magna, tm) = eot_runner();
    let sec_before = runner.security_count(1);

    fire_end_of_turn(&mut runner, magna);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Replacement));
    runner.decline_optional_trigger().expect("decline");

    assert!(runner.pending_selection().is_none());
    let stack = &runner.game.players[0].battle_area[magna.index as usize].card_sources;
    assert_eq!(stack.len(), 2, "Option source stays");
    assert_eq!(runner.security_count(1), sec_before);
    assert!(!runner
        .game
        .modifiers
        .has(tm, ModifierType::SecurityAttackChange));
}

/// Negative (no Option source): the clause cannot be activated at all.
#[test]
fn ex7_013_eot_no_option_source_no_prompt() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 10])
        .security(1, &["DECK-PAD"; 3])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(magna, "PLAIN-DIGI"); // a non-Option source only
    runner.place_on_field(0, "TM-DIGI", Some(0));

    fire_end_of_turn(&mut runner, magna);
    assert!(
        runner.pending_selection().is_none(),
        "no Option in this Digimon's digivolution cards → no activation"
    );
}

/// Negative (target filter): a non-[Three Musketeers] Digimon is not a valid
/// recipient (printed text; DCGO's any-own-Digimon filter is looser).
#[test]
fn ex7_013_eot_only_tm_digimon_are_candidates() {
    let mut runner = base()
        .deck(0, &["DECK-PAD"; 10])
        .security(1, &["DECK-PAD"; 3])
        .memory(3)
        .start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(magna, "PLAIN-OPT");
    let tm = runner.place_on_field(0, "TM-DIGI", Some(0));
    let plain = runner.place_on_field(0, "PLAIN-DIGI", Some(0));

    fire_end_of_turn(&mut runner, magna);
    runner.accept_optional_trigger().expect("accept");
    let act = runner.pending_selection().unwrap().valid_action_ids[0];
    runner.execute_action(0, act).expect("pay cost");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let view = runner.pending_selection_view().unwrap();
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, tm.index as u16)));
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, magna.index as u16)),
        "MagnaKidmon itself carries [Three Musketeers] and is a legal target"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, plain.index as u16)),
        "a Digimon without [Three Musketeers] must not be offered"
    );
}

/// OPT: a second activation in the same turn is locked; the lock clears
/// after the turn rotates back.
#[test]
fn ex7_013_eot_opt_locks_second_activation_same_turn() {
    let (mut runner, magna, tm) = eot_runner();
    runner.push_source(magna, "PLAIN-OPT"); // two Option sources available

    fire_end_of_turn(&mut runner, magna);
    runner.accept_optional_trigger().expect("accept");
    runner.auto_resolve().expect("first activation resolves");

    fire_end_of_turn(&mut runner, magna);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] must lock the second activation"
    );

    runner.game.end_turn(); // P0 → P1 (the natural EOT fire is locked too)
    runner.auto_resolve().ok();
    runner.game.end_turn(); // P1 → P0
    runner.auto_resolve().ok();
    // Give the opponent security back so the forced attack has a target.
    fire_end_of_turn(&mut runner, magna);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the OPT lock clears once the turn comes back around"
    );
}
