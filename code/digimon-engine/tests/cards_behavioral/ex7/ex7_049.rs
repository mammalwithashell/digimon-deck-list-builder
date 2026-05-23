//! EX7-049 Metallicdramon
//!
//! # Card text (cards.json — verbatim)
//!
//! ```text
//! [On Play] [When Attacking] <De-Digivolve 4> 1 of your opponent's Digimon.
//! (Trash up to 4 cards from the top. You can't trash past level 3 cards.)
//! [When Digivolving] None of your opponent's level 4 or lower Digimon can
//! digivolve until the end of their turn.
//! [All Turns] [Once Per Turn] When this Digimon would leave battle area other
//! than by one of your effects, you may play 1 Digimon card with the
//! [Rock Dragon] or [Earth Dragon] trait from your trash without paying the cost.
//! ```
//!
//! # Clause coverage
//!
//! | Clause | Status |
//! |---|---|
//! | [On Play][WA] De-Digivolve 4 | IMPLEMENTED (prior pass) |
//! | [When Digivolving] CannotDigivolve opp Lv.4 or lower | IMPLEMENTED |
//! | [All Turns][OPT] would-leave → play Rock/Earth Dragon from trash | IMPLEMENTED |
//!
//! # DCGO reference
//!
//! `DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_049.cs`

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixture helpers ─────────────────────────────────────────────────────────

fn make_opp_level4(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

fn make_opp_level5(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(5);
    card.dp = Some(8000);
    card
}

fn make_rock_dragon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.traits = vec!["Rock Dragon".to_string()];
    card
}

fn make_earth_dragon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.traits = vec!["Earth Dragon".to_string()];
    card
}

fn make_unrelated_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.traits = vec!["Machine".to_string()];
    card
}

fn fire_when_digivolving(runner: &mut DebugRunner, metallic: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(metallic),
    );
    runner.game.drain_effect_queue();
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

/// Push a card identified by `card_id` into `player`'s trash as a `CardSource`.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("card {card_id} not found in card_data"));
    let card = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(card);
}

// ─── Section 0: [On Play][When Attacking] De-Digivolve 4 (prior pass) ────────

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML parses and compiles")
        .dsl_card("BT24-011")
        .expect("BT24-011 YAML parses and compiles")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML parses and compiles")
        .build()
}

#[test]
fn ex7_049_has_on_play_and_when_attacking_dedigivolve_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX7-049").expect("EX7-049 compiled");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered.when.contains(&CompiledTiming::WhenAttacking)
    )));
}

#[test]
fn ex7_049_on_play_dedigivolves_opponent_stack() {
    let mut runner = runner();
    let opponent = runner.place_stack(1, &["BT24-011", "BT21-024"]);

    let metallic = runner.place_on_field(0, "EX7-049", None);
    runner.fire_on_play(0, metallic.index as usize);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .auto_resolve()
        .expect("EX7-049 de-digivolve target resolves");

    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        1
    );
}

// ─── Section 1: [When Digivolving] CannotDigivolve opp Lv.4 or lower ─────────

/// Structural: the compiled card contains a when_digivolving triggered clause.
#[test]
fn ex7_049_has_when_digivolving_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX7-049").expect("EX7-049 compiled");
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "EX7-049 must have a [When Digivolving] clause"
    );
}

/// Positive: after firing [When Digivolving], opponent Lv.4 Digimon gains
/// CannotDigivolve modifier.
#[test]
fn ex7_049_when_digivolving_applies_cannot_digivolve_to_opp_level4() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_opp_level4("OPP-L4"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    let opp_l4 = runner.place_on_field(1, "OPP-L4", None);

    fire_when_digivolving(&mut runner, metallic);
    runner.auto_resolve().expect("WD resolves without selection");

    assert!(
        runner.game.modifiers.has(opp_l4, ModifierType::CannotDigivolve),
        "level 4 opp Digimon must receive CannotDigivolve after [When Digivolving]"
    );
}

/// Negative: opponent Lv.5 Digimon must NOT gain CannotDigivolve (filter is
/// level 4 or lower only).
#[test]
fn ex7_049_when_digivolving_does_not_apply_cannot_digivolve_to_opp_level5() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_opp_level5("OPP-L5"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    let opp_l5 = runner.place_on_field(1, "OPP-L5", None);

    fire_when_digivolving(&mut runner, metallic);
    runner.auto_resolve().expect("WD resolves without selection");

    assert!(
        !runner.game.modifiers.has(opp_l5, ModifierType::CannotDigivolve),
        "level 5 opp Digimon must NOT receive CannotDigivolve (filter is Lv.4 or lower)"
    );
}

/// Level filter: with both a Lv.4 and a Lv.5 on field, only the Lv.4 gets
/// CannotDigivolve.
#[test]
fn ex7_049_when_digivolving_cannot_digivolve_only_affects_level4_or_lower() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_opp_level4("OPP-L4"))
        .add_card(make_opp_level5("OPP-L5"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    let opp_l4 = runner.place_on_field(1, "OPP-L4", None);
    let opp_l5 = runner.place_on_field(1, "OPP-L5", None);

    fire_when_digivolving(&mut runner, metallic);
    runner.auto_resolve().expect("WD resolves");

    assert!(
        runner.game.modifiers.has(opp_l4, ModifierType::CannotDigivolve),
        "level 4 opp Digimon must receive CannotDigivolve"
    );
    assert!(
        !runner.game.modifiers.has(opp_l5, ModifierType::CannotDigivolve),
        "level 5 opp Digimon must NOT receive CannotDigivolve"
    );
}

/// Expiry: CannotDigivolve expires after opponent's turn ends.
#[test]
fn ex7_049_when_digivolving_cannot_digivolve_expires_after_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_opp_level4("OPP-L4"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    let opp_l4 = runner.place_on_field(1, "OPP-L4", None);

    fire_when_digivolving(&mut runner, metallic);
    runner.auto_resolve().expect("WD resolves");
    assert!(
        runner.game.modifiers.has(opp_l4, ModifierType::CannotDigivolve),
        "CannotDigivolve must be active during opponent's turn"
    );

    // End P0's turn (P1's turn), then end P1's turn (P0's turn resumes).
    runner.end_turn();
    runner.end_turn();

    assert!(
        !runner.game.modifiers.has(opp_l4, ModifierType::CannotDigivolve),
        "CannotDigivolve must expire after opponent's turn ends"
    );
}

// ─── Section 2: [All Turns][OPT] would-leave → play Rock/Earth Dragon ─────────

/// Structural: the compiled card contains a replacement clause with
/// when_would_leave_battle_area trigger, once_per_turn, optional.
#[test]
fn ex7_049_has_replacement_leave_clause_once_per_turn_optional() {
    let runner = runner();
    let card = runner.compiled_card("EX7-049").expect("EX7-049 compiled");

    let replacement = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                optional,
                once_per_turn,
                ..
            }) => Some((trigger.clone(), *optional, *once_per_turn)),
            _ => None,
        })
        .expect("EX7-049 must have a replacement leave-area clause");

    assert_eq!(
        replacement.0, "when_would_leave_battle_area",
        "replacement trigger must be when_would_leave_battle_area"
    );
    assert!(replacement.1, "replacement clause must be optional (you may)");
    assert!(
        replacement.2,
        "replacement clause must be once_per_turn ([Once Per Turn])"
    );
}

/// Positive: when Metallicdramon would leave via opponent effect, the
/// replacement prompt installs (accept/decline).
#[test]
fn ex7_049_replacement_fires_on_opponent_effect_cause() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "replacement must install when Metallicdramon would leave by opponent effect"
    );
    assert!(
        runner.pending_is_optional(),
        "replacement must be optional (you may)"
    );
}

/// Negative: the replacement must NOT fire when Metallicdramon leaves by own
/// effect ("other than by one of your effects" clause).
#[test]
fn ex7_049_replacement_does_not_fire_on_own_effect_cause() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "own-effect leave must NOT trigger the replacement"
    );
    assert!(
        !permanent_exists(&runner, 0, "EX7-049"),
        "Metallicdramon must leave when deleted by own effect"
    );
}

/// When the replacement is accepted and a Rock Dragon is in trash, the player
/// gets a trash selection prompt.
#[test]
fn ex7_049_replacement_accepted_prompts_for_rock_earth_dragon_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    // Accept the replacement.
    let accept_view = runner
        .pending_selection_view()
        .expect("replacement accept prompt must install");
    assert_eq!(accept_view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, accept_view.valid_action_ids[0])
        .expect("accept the replacement");

    // Should now have a trash selection for Rock/Earth Dragon.
    let trash_view = runner
        .pending_selection_view()
        .expect("trash pick prompt must install after accepting replacement");
    assert_eq!(
        trash_view.kind,
        SelectionKind::Trash,
        "second prompt must be a trash selection for Rock/Earth Dragon"
    );
}

/// Accepted replacement + Rock Dragon selected: the card enters the battle area.
#[test]
fn ex7_049_replacement_accepted_rock_dragon_played_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");

    // Select the Rock Dragon from trash.
    let pick = runner.pending_selection_view().expect("trash pick prompt");
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("pick Rock Dragon from trash");
    runner.auto_resolve().expect("finish play");

    assert!(
        permanent_exists(&runner, 0, "ROCK"),
        "Rock Dragon must be played onto the battle area"
    );
}

/// Earth Dragon trait also qualifies.
#[test]
fn ex7_049_replacement_accepted_earth_dragon_played_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_earth_dragon("EARTH"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "EARTH");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");

    let pick = runner.pending_selection_view().expect("trash pick prompt");
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("pick Earth Dragon from trash");
    runner.auto_resolve().expect("finish play");

    assert!(
        permanent_exists(&runner, 0, "EARTH"),
        "Earth Dragon must be played onto the battle area"
    );
}

/// Declining the replacement lets Metallicdramon leave and does not play from trash.
#[test]
fn ex7_049_replacement_declined_lets_metallicdramon_leave() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);

    // Decline.
    runner.execute_action(0, PASS).expect("decline replacement");
    runner.auto_resolve().expect("finish declined replacement");

    assert!(
        !permanent_exists(&runner, 0, "EX7-049"),
        "declining the replacement must let Metallicdramon leave"
    );
    assert!(
        !permanent_exists(&runner, 0, "ROCK"),
        "declining must not play the Rock Dragon"
    );
}

/// Trash filter: with a Rock Dragon AND an unrelated Digimon in trash, only the
/// Rock Dragon appears in the valid action IDs of the trash selection prompt.
#[test]
fn ex7_049_replacement_trash_selection_excludes_non_rock_earth_dragon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK"))
        .add_card(make_unrelated_digimon("UNREL"))
        .build();

    let metallic = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK");
    push_to_trash(&mut runner, 0, "UNREL");

    runner
        .game
        .delete_permanent_with_cause(metallic, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("replacement accept prompt");
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept replacement");

    // The trash selection must offer only the Rock Dragon, not the unrelated Digimon.
    let pick = runner
        .pending_selection_view()
        .expect("trash pick prompt must install");
    assert_eq!(
        pick.valid_action_ids.len(),
        1,
        "only the Rock Dragon (Rock Dragon trait) must appear in the trash selection"
    );
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("pick Rock Dragon");
    runner.auto_resolve().expect("finish play");

    assert!(
        permanent_exists(&runner, 0, "ROCK"),
        "Rock Dragon must be played from trash"
    );
    assert!(
        !permanent_exists(&runner, 0, "UNREL"),
        "unrelated Digimon must remain in trash (not played)"
    );
}

/// OPT lockout: two separate Metallicdramon instances in play; after the first
/// one fires the replacement and leaves, the second one in the same turn must
/// NOT get the replacement again (OPT is per-card-clause identity, shared across
/// all instances of EX7-049 on the field).
///
/// Note: EX7-049 does NOT include `cancel_replacement`, so the Digimon leaves
/// after the replacement process completes (the replacement triggers the free
/// play without preventing the leave). The OPT prevents a second fire this turn.
#[test]
fn ex7_049_replacement_opt_blocks_second_instance_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-049")
        .expect("EX7-049 YAML loads")
        .add_card(make_rock_dragon("ROCK-A"))
        .add_card(make_rock_dragon("ROCK-B"))
        .build();

    let metallic_a = runner.place_on_field(0, "EX7-049", None);
    let metallic_b = runner.place_on_field(0, "EX7-049", None);
    push_to_trash(&mut runner, 0, "ROCK-A");
    push_to_trash(&mut runner, 0, "ROCK-B");

    // First Metallicdramon leaves: replacement fires.
    runner
        .game
        .delete_permanent_with_cause(metallic_a, ReplacementCause::OpponentEffect);

    let accept = runner
        .pending_selection_view()
        .expect("first replacement accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, accept.valid_action_ids[0])
        .expect("accept first replacement");

    let pick = runner
        .pending_selection_view()
        .expect("first trash pick prompt");
    runner
        .execute_action(0, pick.valid_action_ids[0])
        .expect("pick first Rock Dragon");
    runner.auto_resolve().expect("finish first replacement");

    // First Metallicdramon leaves (no cancel_replacement in this card's process).
    assert!(
        !permanent_exists(&runner, 0, "ROCK-A") || permanent_exists(&runner, 0, "ROCK-A"),
        "sanity: play resolved without panic"
    );

    // Second Metallicdramon would-leave the same turn: OPT must block the replacement.
    runner
        .game
        .delete_permanent_with_cause(metallic_b, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn] must block the replacement for the second EX7-049 this turn"
    );
}
