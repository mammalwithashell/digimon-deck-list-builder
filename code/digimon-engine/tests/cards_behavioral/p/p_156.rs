//! P-156 Future Potential! - Option, Cost 2, Black.
//!
//! cards.json evidence:
//! - "While you have a Tamer, you can ignore this card's color requirements."
//! - "[Main] Choose 1 Tamer. You may play 1 Digimon card with the same color as
//!   the chosen Tamer with a play cost of 3 or less from your hand or trash
//!   without paying the cost."
//! - "Security Effect [Security] You may play 1 Tamer card from your hand
//!   without paying the cost. Then, add this card to your hand."
//!
//! Supported faithfully here:
//! - Conditional Option color bypass via top-level `use_requirement`.
//! - Main chosen-Tamer color binding with one optional pick over hand + trash
//!   (no dead "From hand / From trash" choice) and free play of an eligible
//!   Digimon.
//! - Security optional Tamer free-play from hand (visible Play/Decline branch),
//!   followed by the mandatory add-this-option-to-hand tail.
//!
//! DCGO reference: DCGO/Assets/Scripts/CardEffect/P/White/P_156.cs
//!
//! Pattern tags:
//! - option-color-bypass-use-requirement
//! - chosen-permanent-color-binding
//! - hand-or-trash-free-play
//! - optional-substep-mandatory-tail (PUPPETS-G017 card-shaped adoption)
//! - security-add-this-option-to-hand

use std::path::Path;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn yaml() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/p/P-156.yaml"))
        .expect("P-156 YAML must exist at cards/p/P-156.yaml")
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML must parse and compile")
        .memory(10)
        .start()
}

fn filler(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

fn tamer(id: &str, color: CardColor) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Color Tamer");
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.play_cost = 3;
    card
}

fn digimon(id: &str, color: CardColor, cost: u16) -> digimon_engine::CardData {
    let mut card = make_test_card(id, "Color Digimon");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = cost;
    card
}

fn zone_contains_card(runner: &DebugRunner, player: PlayerId, zone: Zone, card_id: &str) -> bool {
    let player = runner.game.player(player);
    let cards = match zone {
        Zone::Hand => &player.hand,
        Zone::Trash => &player.trash,
    };
    cards.iter().any(|card| {
        runner
            .game
            .card_data_for_handle(card.handle())
            .is_some_and(|data| data.card_id == card_id)
    })
}

enum Zone {
    Hand,
    Trash,
}

fn battle_area_contains_card(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        runner
            .game
            .card_data_for_handle(perm.top_card().handle())
            .is_some_and(|data| data.card_id == card_id)
    })
}

#[test]
fn p_156_yaml_parses_and_compiles() {
    let _runner = runner();
}

#[test]
fn p_156_is_white_option_cost_2_with_tamer_use_requirement() {
    let runner = runner();
    let compiled = runner.compiled_card("P-156").expect("compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(2));
    assert!(
        compiled.use_requirement.is_some(),
        "P-156 must encode the printed Tamer-based color-bypass condition as an Option use requirement"
    );
}

#[test]
fn p_156_tamer_use_requirement_matches_tamer_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    let source = runner.game.players[0].hand[0].handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    let use_req = runner
        .compiled_card("P-156")
        .expect("compiled card")
        .use_requirement
        .as_ref()
        .expect("P-156 use requirement");

    assert!(
        eval_predicate(use_req, &ctx, PredicateSubject::None),
        "having any Tamer should satisfy the printed color-bypass condition"
    );
}

#[test]
fn p_156_use_requirement_does_not_match_without_tamer() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    let source = runner.game.players[0].hand[0].handle();
    let ctx = EffectReadContext::new(&runner.game, source, None, 0);
    let use_req = runner
        .compiled_card("P-156")
        .expect("compiled card")
        .use_requirement
        .as_ref()
        .expect("P-156 use requirement");

    assert!(
        !eval_predicate(use_req, &ctx, PredicateSubject::None),
        "without a Tamer, the color-bypass use requirement must not be satisfied"
    );
}

#[test]
fn p_156_security_tail_adds_this_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .security(1, &["P-156"])
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    // No Tamer in the defender's hand: the count_gte guard suppresses the
    // optional Play/Decline prompt; only the mandatory tail runs. The Security
    // clause still resolves through the deferred-selection pipeline, so the
    // attack returns `InProgress` and `auto_resolve` finalizes it.
    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security clause resolves");

    assert_eq!(runner.security_count(1), 0);
    assert!(zone_contains_card(&runner, 1, Zone::Hand, "P-156"));
    assert!(!zone_contains_card(&runner, 1, Zone::Trash, "P-156"));
}

#[test]
fn p_156_has_main_and_security_clauses() {
    let runner = runner();
    let compiled = runner.compiled_card("P-156").expect("compiled card");

    let has_main = compiled.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => triggered.when.contains(&CompiledTiming::Main),
        _ => false,
    });

    assert!(has_main, "P-156 must expose its printed Main effect");
    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(triggered) => {
                triggered.when.contains(&CompiledTiming::OnSecurity)
            }
            _ => false,
        }),
        "P-156 must expose its Security effect"
    );
}

#[test]
fn p_156_main_chooses_tamer_then_filters_hand_or_trash_digimon_by_bound_tamer_color_and_cost() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("YELLOW-HAND", CardColor::Yellow, 3))
        .add_card(digimon("RED-HAND", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HIGH", CardColor::Yellow, 4))
        .add_card(filler("FILL"))
        .hand(0, &["P-156", "YELLOW-HAND", "RED-HAND", "YELLOW-HIGH"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    runner.place_on_field(1, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let memory_after_option_cost = runner.memory();

    let tamer_choice = runner
        .pending_selection_view()
        .expect("chosen Tamer selection");
    assert_eq!(
        tamer_choice.valid_action_ids.len(),
        2,
        "own and opponent Tamers should both be visible choices"
    );
    runner
        .execute_action(
            tamer_choice.selecting_player,
            tamer_choice.valid_action_ids[0],
        )
        .expect("choose the yellow Tamer");

    assert_eq!(
        offered(&runner),
        vec![("YELLOW-HAND".to_string(), "hand")],
        "only the yellow play-cost-3 Digimon should be selectable"
    );
    let pick = physical_choice::card_action(
        &runner.game,
        "YELLOW-HAND",
        digimon_engine::owned_card_choice::CardChoiceZone::Hand,
    );
    runner
        .execute_action(0, pick)
        .expect("play yellow hand Digimon");
    runner.auto_resolve().expect("finish P-156 Main");

    assert!(battle_area_contains_card(&runner, 0, "YELLOW-HAND"));
    assert!(!battle_area_contains_card(&runner, 0, "RED-HAND"));
    assert!(!battle_area_contains_card(&runner, 0, "YELLOW-HIGH"));
    assert_eq!(
        runner.memory(),
        memory_after_option_cost,
        "selected Digimon must be played without paying its cost"
    );
}

#[test]
fn p_156_not_legal_without_tamer_white_color_never_matches() {
    // P-156 is WHITE: no battle-area permanent can color-match it, so the
    // ONLY route to legality is the printed Use Req. ("While you have a
    // Tamer, you may ignore this card's color requirements"). Option use
    // legality is otherwise phase + cost + color (rule 1-3-11-1) — never
    // effect-target availability.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("BLACK-SOURCE", CardColor::Black, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLACK-SOURCE", Some(0));
    runner.game.enter_main_phase();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "white P-156 with no Tamer: color never matches and the Use Req. bypass is unmet"
    );
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "direct Option use must share the action-mask preflight"
    );
}

#[test]
fn p_156_main_can_choose_opponents_tamer() {
    // The [Main] "Choose 1 Tamer" is side-agnostic (official Q&A: it refers
    // to a card sharing a color with the CHOSEN card, wherever it is). The
    // user still needs their OWN Tamer to use white P-156 at all (Use Req.
    // bypass) — here a BLUE one, whose color matches nothing in hand, so the
    // hand filter proves the chosen OPPONENT Tamer's color drives it.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(digimon("RED-HAND", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HAND", CardColor::Yellow, 3))
        .add_card(filler("FILL"))
        .hand(0, &["P-156", "RED-HAND", "YELLOW-HAND"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "BLUE-TAMER", Some(0));
    runner.place_on_field(1, "RED-TAMER", Some(0));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let tamer_choice = runner.pending_selection_view().expect("Tamer selection");
    assert_eq!(
        tamer_choice.valid_action_ids.len(),
        2,
        "both the own and the opponent Tamer should be choosable"
    );
    // AnyField selections encode candidates as
    // `100 + player * TARGETS_PER_ATTACKER + index` — pick player 1's Tamer.
    let opp_base = 100 + digimon_engine::action::space::TARGETS_PER_ATTACKER;
    let opp_tamer_id = *tamer_choice
        .valid_action_ids
        .iter()
        .find(|id| **id >= opp_base)
        .expect("opponent Tamer should be a legal choice");
    runner
        .execute_action(tamer_choice.selecting_player, opp_tamer_id)
        .expect("choose opponent red Tamer");

    assert_eq!(
        offered(&runner),
        vec![("RED-HAND".to_string(), "hand")],
        "the selected opponent Tamer's color should filter hand Digimon"
    );
}

#[test]
fn p_156_main_trash_branch_uses_same_bound_tamer_color_and_cost_filter() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(digimon("YELLOW-TRASH", CardColor::Yellow, 3))
        .add_card(digimon("RED-TRASH", CardColor::Red, 3))
        .add_card(digimon("YELLOW-HIGH", CardColor::Yellow, 4))
        .add_card(filler("FILL"))
        .hand(0, &["P-156"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    for card_id in ["YELLOW-TRASH", "RED-TRASH", "YELLOW-HIGH"] {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|data| data.card_id == card_id)
            .expect("test card registered");
        let next_idx = runner.game.next_card_index();
        runner.game.players[0]
            .trash
            .push(digimon_engine::card_source::CardSource::new(
                data_idx, 0, next_idx,
            ));
    }
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let tamer_choice = runner
        .pending_selection_view()
        .expect("chosen Tamer selection");
    runner
        .execute_action(
            tamer_choice.selecting_player,
            tamer_choice.valid_action_ids[0],
        )
        .expect("choose yellow Tamer");

    assert_eq!(
        offered(&runner),
        vec![("YELLOW-TRASH".to_string(), "trash")],
        "only the yellow play-cost-3 Digimon should be selectable from trash"
    );
    let pick = physical_choice::card_action(
        &runner.game,
        "YELLOW-TRASH",
        digimon_engine::owned_card_choice::CardChoiceZone::Trash,
    );
    runner
        .execute_action(0, pick)
        .expect("play yellow trash Digimon");
    runner.auto_resolve().expect("finish P-156 Main");

    assert!(battle_area_contains_card(&runner, 0, "YELLOW-TRASH"));
    assert!(!battle_area_contains_card(&runner, 0, "RED-TRASH"));
    assert!(!battle_area_contains_card(&runner, 0, "YELLOW-HIGH"));
}

/// Structural: the [Security] clause itself is mandatory. The printed "you may"
/// governs only the inner Tamer play; the "Then, add this card to your hand"
/// tail is unconditional, so the whole Security effect must never be declinable
/// at the clause level. The Tamer-play optionality lives in an inner branch.
#[test]
fn p_156_security_clause_is_mandatory_with_inner_optional_tamer_play() {
    let runner = runner();
    let compiled = runner.compiled_card("P-156").expect("compiled card");

    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::OnSecurity) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("P-156 must expose a Security clause");

    assert!(
        !security.optional,
        "P-156's Security clause is mandatory: the add-this-option-to-hand tail \
         is unconditional and must never be skippable by declining the clause"
    );
}

/// Positive: a Tamer in the defending player's hand during the Security check
/// surfaces a visible Play/Decline branch. Choosing Play installs a hand
/// selection of Tamer cards; selecting the Tamer plays it to the battle area
/// for free. The mandatory tail then routes P-156 itself to the defender's hand.
#[test]
fn p_156_security_can_play_tamer_from_hand_then_add_this_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .add_card(tamer("SEC-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(1, &["SEC-TAMER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["P-156"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: P-156 in security"
    );
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "precondition: defender has no permanents"
    );

    // The Security clause parks on its first prompt, so the attack returns
    // `InProgress`; the pending selection is then driven explicitly.
    let _ = runner.attack_player(attacker, 1, false);

    // The Security clause installs a visible Play/Decline branch because the
    // defender holds a Tamer that the optional sub-effect can play.
    let branch = runner
        .pending_selection_view()
        .expect("Security Play/Decline branch must install when a Tamer is in hand");
    assert_eq!(branch.kind, SelectionKind::EffectChoice);
    assert_eq!(
        branch.selecting_player, 1,
        "the security holder resolves the inherited Security effect"
    );
    runner
        .execute_action(branch.selecting_player, branch.valid_action_ids[0])
        .expect("choose to play a Tamer");

    // Choosing Play installs a hand selection over the defender's Tamer cards.
    let hand_choice = runner
        .pending_selection_view()
        .expect("hand Tamer selection must install after choosing Play");
    assert_eq!(hand_choice.kind, SelectionKind::Hand);
    assert_eq!(
        hand_choice.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the Tamer in hand should be selectable"
    );
    runner
        .execute_action(hand_choice.selecting_player, PLAY_HAND_START)
        .expect("play the Tamer from hand for free");
    runner.auto_resolve().expect("finish P-156 Security");

    assert!(
        battle_area_contains_card(&runner, 1, "SEC-TAMER"),
        "the chosen Tamer must be played to the defender's battle area"
    );
    // Mandatory tail: P-156 leaves security and is added to the defender's hand.
    assert_eq!(runner.security_count(1), 0, "P-156 left the security stack");
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "P-156"),
        "the mandatory add-this-option-to-hand tail must still run after the optional play"
    );
    assert!(!zone_contains_card(&runner, 1, Zone::Trash, "P-156"));
}

/// Negative (decline): with a Tamer in hand, the player declines the optional
/// play. The Tamer stays in hand, nothing reaches the battle area, and the
/// mandatory add-this-option-to-hand tail still resolves.
#[test]
fn p_156_security_can_decline_tamer_play_and_still_add_this_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .add_card(tamer("SEC-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(1, &["SEC-TAMER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["P-156"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let _ = runner.attack_player(attacker, 1, false);

    let branch = runner
        .pending_selection_view()
        .expect("Security Play/Decline branch must install when a Tamer is in hand");
    assert_eq!(branch.kind, SelectionKind::EffectChoice);
    // Branch index 1 is the "Decline" label.
    runner
        .execute_action(branch.selecting_player, branch.valid_action_ids[1])
        .expect("decline the optional Tamer play");
    runner.auto_resolve().expect("finish P-156 Security");

    assert!(
        !battle_area_contains_card(&runner, 1, "SEC-TAMER"),
        "declining must leave the Tamer unplayed"
    );
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "SEC-TAMER"),
        "the declined Tamer stays in the defender's hand"
    );
    // Mandatory tail still runs even though the optional play was declined.
    assert_eq!(runner.security_count(1), 0, "P-156 left the security stack");
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "P-156"),
        "declining the optional play must not skip the mandatory add-to-hand tail"
    );
    assert!(!zone_contains_card(&runner, 1, Zone::Trash, "P-156"));
}

/// Negative (no candidate): the defender has no Tamer in hand, so no Play/Decline
/// prompt installs at all. The mandatory add-this-option-to-hand tail still runs.
#[test]
fn p_156_security_no_tamer_in_hand_skips_prompt_but_still_adds_this_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .add_card(digimon("HAND-DIGIMON", CardColor::Blue, 3))
        .add_card(filler("FILL"))
        .hand(1, &["HAND-DIGIMON"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["P-156"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    assert_eq!(
        runner.security_count(1),
        1,
        "precondition: P-156 in security"
    );

    let _ = runner.attack_player(attacker, 1, false);

    // No Tamer in hand: the count_gte guard suppresses the Play/Decline prompt.
    assert!(
        runner.pending_selection_view().is_none(),
        "no Tamer in hand must not install any optional-play prompt"
    );
    runner.auto_resolve().expect("security clause resolves");

    // Mandatory tail still runs with no Tamer candidate available.
    assert_eq!(runner.security_count(1), 0, "P-156 left the security stack");
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "P-156"),
        "the mandatory add-to-hand tail must run even with no Tamer candidate"
    );
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "HAND-DIGIMON"),
        "the non-Tamer hand card is untouched"
    );
    assert!(
        !battle_area_contains_card(&runner, 1, "HAND-DIGIMON"),
        "a non-Tamer Digimon must never be playable by the Security Tamer sub-effect"
    );
    assert!(!zone_contains_card(&runner, 1, Zone::Trash, "P-156"));
}

// ─── Candidate legality ──────────────────────────────────────────────────────
// [Main]: DCGO P_156.cs:58-75 `SelectDigimonCondition` = `IsDigimon &&
// CanPlayAsNewPermanent(source, false, activateClass) && GetCostItself <= 3 &&
// shares a colour with the chosen Tamer`, for both the hand pick
// (P_156.cs:157-172) and the trash pick (P_156.cs:176-195, root Trash).
// [Security]: P_156.cs:243-249 `HasTamer` = `CanPlayAsNewPermanent(source,
// false, activateClass) && IsTamer`, which also gates the whole optional play
// (P_156.cs:258 `HasMatchConditionOwnersHand(card, HasTamer)`).
// `CanPlayAsNewPermanent` (GameContextDeterminarion.cs:419-432) needs an empty
// battle-area frame and no play prohibition, and refuses a trash card while
// the trash can't be played from. CR 4.3 1-3-11-1: a card can be declared for
// play only if it can be played (7-1-1), so a card that can't be played is not
// a legal "play" choice.

fn add_player_modifier(runner: &mut DebugRunner, player: PlayerId, kind: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 1),
    );
}

#[test]
fn p_156_main_pick_excludes_digimon_under_effect_play_prohibition() {
    let mut runner = main_after_tamer_choice(&["YELLOW-HAND"], &["YELLOW-TRASH"], |r| {
        add_player_modifier(r, 0, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect)
    });
    assert!(
        runner.pending_selection_view().is_none(),
        "\"can't play Digimon by effects\": no hand or trash Digimon may be offered"
    );
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(zone_contains_card(&runner, 0, Zone::Hand, "YELLOW-HAND"));
    assert!(zone_contains_card(&runner, 0, Zone::Trash, "YELLOW-TRASH"));
}

#[test]
fn p_156_main_pick_excludes_digimon_when_battle_area_full() {
    let mut runner =
        main_after_tamer_choice(&["YELLOW-HAND"], &["YELLOW-TRASH"], |r| r.game.rules.field_slots = 1);
    assert!(
        runner.pending_selection_view().is_none(),
        "the Tamer fills the only slot: no hand or trash Digimon may be offered"
    );
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(zone_contains_card(&runner, 0, Zone::Hand, "YELLOW-HAND"));
    assert!(zone_contains_card(&runner, 0, Zone::Trash, "YELLOW-TRASH"));
}

#[test]
fn p_156_main_pick_excludes_trash_digimon_while_trash_play_is_locked() {
    let runner = main_after_tamer_choice(&["YELLOW-HAND"], &["YELLOW-TRASH"], |r| {
        add_player_modifier(r, 0, digimon_engine::enums::ModifierType::CannotPlayFromTrash)
    });
    assert_eq!(
        offered(&runner),
        vec![("YELLOW-HAND".to_string(), "hand")],
        "\"can't play cards from the trash\": only the hand Digimon may be offered"
    );
}

#[test]
fn p_156_main_asks_nothing_when_only_trash_candidate_is_locked() {
    let mut runner = main_after_tamer_choice(&[], &["YELLOW-TRASH"], |r| {
        add_player_modifier(r, 0, digimon_engine::enums::ModifierType::CannotPlayFromTrash)
    });
    assert!(
        runner.pending_selection_view().is_none(),
        "the only candidate is in a locked trash: no zone choice and no pick"
    );
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(zone_contains_card(&runner, 0, Zone::Trash, "YELLOW-TRASH"));
}

/// [Security] with a Tamer in hand that can't be played: no Play/Decline
/// branch and no hand pick install; the add-this-card-to-hand tail still runs.
fn security_with_unplayable_tamer(setup: impl FnOnce(&mut DebugRunner)) {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(digimon("ATTACKER", CardColor::Red, 3))
        .add_card(digimon("DEF-DIGIMON", CardColor::Blue, 3))
        .add_card(tamer("SEC-TAMER", CardColor::Blue))
        .add_card(filler("FILL"))
        .hand(1, &["SEC-TAMER"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .security(1, &["P-156"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    setup(&mut runner);
    let _ = runner.attack_player(attacker, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "the Tamer can't be played, so neither the Play/Decline branch nor a Tamer pick installs"
    );
    runner.auto_resolve().expect("security clause resolves");
    assert!(!battle_area_contains_card(&runner, 1, "SEC-TAMER"));
    assert!(zone_contains_card(&runner, 1, Zone::Hand, "SEC-TAMER"));
    assert!(
        zone_contains_card(&runner, 1, Zone::Hand, "P-156"),
        "the mandatory add-to-hand tail still runs"
    );
}

#[test]
fn p_156_security_does_not_offer_tamer_when_battle_area_full() {
    security_with_unplayable_tamer(|r| {
        r.place_on_field(1, "DEF-DIGIMON", Some(0));
        r.game.rules.field_slots = 1;
    });
}

#[test]
fn p_156_security_does_not_offer_tamer_under_effect_play_prohibition() {
    security_with_unplayable_tamer(|r| {
        add_player_modifier(r, 1, digimon_engine::enums::ModifierType::CannotPlayTamerByEffect)
    });
}

// ─── [Main] zone choice ──────────────────────────────────────────────────────
// Printed: "You may play 1 Digimon card ... from your hand or trash" — ONE card
// chosen from the two zones together. DCGO P_156.cs:124-150 shows its "From
// hand / From trash" prompt only when BOTH zones hold a `SelectDigimonCondition`
// candidate; with one zone it fixes that zone (`SetBool(canSelectHand)`), and
// with none it asks nothing. So a zone with no playable candidate must never be
// offered. The engine asks one optional pick over hand ∪ trash (the
// `select_union_zone` pattern of AvengeKidmon (P-170) / Dimetromon (P-189)).

/// Committable rows of the pending physical card menu, as (card id, zone).
fn offered(runner: &DebugRunner) -> Vec<(String, &'static str)> {
    use digimon_engine::owned_card_choice::CardChoiceZone;
    physical_choice::commits(&runner.game)
        .into_iter()
        .map(|row| {
            let zone = match row.zone {
                Some(CardChoiceZone::Hand) => "hand",
                Some(CardChoiceZone::Trash) => "trash",
                _ => "other",
            };
            (row.card_id.unwrap_or_default(), zone)
        })
        .collect()
}

/// Use P-156 with the yellow Tamer on the field, `hand` added to the hand and
/// `trash` put in the trash, then choose that Tamer. `setup` runs before use.
fn main_after_tamer_choice(
    hand: &[&str],
    trash: &[&str],
    setup: impl FnOnce(&mut DebugRunner),
) -> DebugRunner {
    let mut hand_ids = vec!["P-156"];
    hand_ids.extend_from_slice(hand);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("P-156 YAML parses")
        .add_card(tamer("YELLOW-TAMER", CardColor::Yellow))
        .add_card(digimon("YELLOW-HAND", CardColor::Yellow, 3))
        .add_card(digimon("YELLOW-TRASH", CardColor::Yellow, 3))
        .add_card(digimon("RED-TRASH", CardColor::Red, 3))
        .add_card(filler("FILL"))
        .hand(0, &hand_ids)
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-TAMER", Some(0));
    for id in trash {
        runner.inject_trash(0, id);
    }
    setup(&mut runner);
    runner.game.enter_main_phase();
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let view = runner.pending_selection_view().expect("Tamer pick");
    assert_eq!(view.valid_action_ids.len(), 1, "only the yellow Tamer is on the field");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose the Tamer");
    runner
}

#[test]
fn p_156_main_offers_no_zone_choice_when_only_hand_has_a_candidate() {
    let runner = main_after_tamer_choice(&["YELLOW-HAND"], &["RED-TRASH"], |_| {});
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "the trash holds no same-colour Digimon, so \"From trash\" is a dead option"
    );
    assert_eq!(offered(&runner), vec![("YELLOW-HAND".to_string(), "hand")]);
}

#[test]
fn p_156_main_offers_no_zone_choice_when_only_trash_has_a_candidate() {
    let runner = main_after_tamer_choice(&[], &["YELLOW-TRASH", "RED-TRASH"], |_| {});
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "the hand holds no candidate, so \"From hand\" is a dead option"
    );
    assert_eq!(offered(&runner), vec![("YELLOW-TRASH".to_string(), "trash")]);
}

#[test]
fn p_156_main_asks_nothing_when_no_zone_has_a_candidate() {
    let mut runner = main_after_tamer_choice(&[], &["RED-TRASH"], |_| {});
    assert!(
        runner.pending_selection_view().is_none(),
        "no same-colour Digimon in hand or trash: no zone choice and no pick"
    );
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(zone_contains_card(&runner, 0, Zone::Trash, "RED-TRASH"));
}

#[test]
fn p_156_main_offers_hand_and_trash_candidates_in_one_optional_pick() {
    let mut runner = main_after_tamer_choice(&["YELLOW-HAND"], &["YELLOW-TRASH"], |_| {});
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::UnionZone {
            zones: digimon_engine::selection::UnionZoneSet::HAND
                | digimon_engine::selection::UnionZoneSet::TRASH
        })
    );
    assert!(runner.pending_selection_view().unwrap().is_optional, "\"you may play\"");
    let mut got = offered(&runner);
    got.sort();
    assert_eq!(
        got,
        vec![
            ("YELLOW-HAND".to_string(), "hand"),
            ("YELLOW-TRASH".to_string(), "trash"),
        ]
    );
    let memory = runner.memory();
    let pick = physical_choice::card_action(
        &runner.game,
        "YELLOW-TRASH",
        digimon_engine::owned_card_choice::CardChoiceZone::Trash,
    );
    runner.execute_action(0, pick).expect("play the trash Digimon");
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(battle_area_contains_card(&runner, 0, "YELLOW-TRASH"));
    assert!(zone_contains_card(&runner, 0, Zone::Hand, "YELLOW-HAND"));
    assert_eq!(runner.memory(), memory, "played without paying the cost");
}

#[test]
fn p_156_main_pick_can_be_declined() {
    let mut runner = main_after_tamer_choice(&["YELLOW-HAND"], &["YELLOW-TRASH"], |_| {});
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional play");
    runner.auto_resolve().expect("finish P-156 Main");
    assert!(zone_contains_card(&runner, 0, Zone::Hand, "YELLOW-HAND"));
    assert!(zone_contains_card(&runner, 0, Zone::Trash, "YELLOW-TRASH"));
    assert!(!battle_area_contains_card(&runner, 0, "YELLOW-HAND"));
}

#[path = "../../support/owned_card_choice.rs"]
mod physical_choice;
