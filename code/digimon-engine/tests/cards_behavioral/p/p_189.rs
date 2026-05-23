//! P-189 Dimetromon — Digimon, Lv.4, Red, DP 6000, Cost 6.
//! Traits: Reptile, LIBERATOR
//!
//! # Card text (cards.json)
//!
//! [Security] You may play 1 card with the [LIBERATOR] trait and a play cost
//! of 4 or less from your hand or trash without paying the cost.
//! ＜Progress＞ (While attacking, your opponent's effects don't affect this Digimon.)
//!
//! Inherited:
//! [Your Turn] [Once Per Turn] When your opponent's security stack is removed
//! from, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_189.cs
//!
//! # Clause map
//! - Clause 1 (on_security, FaceUp, optional): "you may play 1 LIBERATOR card
//!   with play cost 4 or less from hand or trash without paying the cost".
//!   select_effect_choice (From hand / From trash) → select_hand / select_trash
//!   with filter `all_of: [trait_has: LIBERATOR, play_cost_lte: 4]` →
//!   play_from_hand_free / play_from_trash_free.
//! - Clause 2 (declarative grant_keyword): `<Progress>` — installs at runtime,
//!   discoverable via `Game::has_keyword`, gates opponent effects while attacking.
//! - Clause 3 (inherited, on_opponent_security_removed, OPT, your_turn): gain
//!   1 memory when the opponent's security stack is removed from.
//!
//! # Patterns this test covers
//! - E2  OPT + optional (security clause is optional, inherited is OPT)
//! - H-Progress: declarative Progress keyword grant (behavioral, runtime install)
//! - G-INHERITED-DISPATCH: inherited triggered on_opponent_security_removed
//! - G-OPT-TRIGGERED: once_per_turn on inherited triggered clause
//! - UnionZone-adjacent: security clause selects from hand OR trash
//!
//! # Gap status (orchestrator-verified 2026-05-21, re-verified in qa/resolved-gaps.md)
//! - G-PLAY-COST-LTE — RESOLVED 2026-05-01. `play_cost_lte` is parsed, compiled,
//!   evaluated against `CardData::play_cost`, and wired into the union-zone
//!   valid-action filtering. Clause 1's filter enforces `trait_has: LIBERATOR`
//!   + `play_cost_lte: 4`.
//! - G-DECLARATIVE-KEYWORD — RESOLVED 2026-05-02. `EffectTiming::Declarative`
//!   is wired; declarative `grant_keyword` fires at runtime. The `<Progress>`
//!   grant installs and is discovered by `Game::has_keyword`.
//! - G-INHERITED-DISPATCH + G-OPT-TRIGGERED — RESOLVED. Inherited triggered
//!   effects dispatch from the digivolution stack; OPT enforced in queue drain.
//!
//! # Engine gap discovered during this implementation — RESOLVED
//! - G-SECURITY-SKILL-RESUME-REFIRE (engine, combat.rs) — RESOLVED 2026-05-21.
//!   The security-resolution drain arms now record `phase_enqueue_done` on
//!   `SecurityResolutionState`; a resume after a **declined** optional
//!   `on_security` selection no longer re-enqueues `EffectTiming::SecuritySkill`
//!   (which previously re-collected the same effect from the still-parked
//!   `pending_security` card — an infinite loop).
//!   `p_189_security_clause_can_be_declined` pins the decline path.

const P189_YAML: &str = include_str!("../../../cards/p/P-189.yaml");

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

// ── Fixture builders ──────────────────────────────────────────────────────

/// A LIBERATOR-trait Digimon with play cost 3 — eligible for the [Security]
/// clause selection (LIBERATOR + cost ≤ 4).
fn make_liberator_low_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 3;
    c
}

/// A LIBERATOR-trait Digimon with play cost exactly 4 — the boundary case;
/// eligible for the [Security] clause (cost == 4 passes `play_cost_lte: 4`).
fn make_liberator_cost_4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 4;
    c
}

/// A LIBERATOR-trait Digimon with play cost 5 — INELIGIBLE: the trait matches
/// but the cost exceeds 4, so `play_cost_lte: 4` excludes it.
fn make_liberator_high_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 5;
    c
}

/// A non-LIBERATOR Digimon with play cost 3 — INELIGIBLE: the cost is low
/// enough but the trait filter excludes it.
fn make_non_liberator(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c.play_cost = 3;
    c
}

/// A strong attacker for the security-attack path — DP high enough to win
/// the DP battle against P-189 (6000 DP) so the check resolves cleanly.
#[allow(dead_code)]
fn make_strong_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(9000);
    c.level = Some(6);
    c.play_cost = 8;
    c
}

/// Standard runner: P-189 from YAML, plus filler cards, 5 memory, game started.
fn dimetromon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses without error")
        .add_card(make_liberator_low_cost("LIB-1"))
        .add_card(make_liberator_low_cost("LIB-2"))
        .add_card(make_non_liberator("NON-LIB-1"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .memory(5)
        .start()
}

/// Register P-189 into a defender's (player 1's) security stack as the top
/// security card, so an attack on P1's security reveals P-189 and fires its
/// face-up [Security] clause. Mirrors `bt21_015`'s security-attack harness.
fn push_p189_into_security(runner: &mut DebugRunner, defender: u8) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "P-189")
        .expect("P-189 registered in card_data");
    let next = runner.game.next_card_index();
    let src = CardSource::new(data_idx, defender, next);
    runner.game.players[defender as usize].security.push(src);
}

/// Insert P-189 as the bottom digivolution source of the permanent at
/// `carrier.index` on `player`'s field. After this call the permanent's
/// `card_sources` has P-189 at position 0 (inherited) and the original top
/// card preserved as the visible top.
fn insert_p189_as_source(
    runner: &mut DebugRunner,
    player: u8,
    carrier: digimon_engine::permanent::PermanentHandle,
) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == "P-189")
        .expect("P-189 registered in card_data");
    let next = game.next_card_index();
    let mut src = CardSource::new(data_idx, player, next);
    src.card_index = next;
    let perm = &mut game.players[player as usize].battle_area[carrier.index as usize];
    perm.card_sources.insert(0, src);
}

/// Push `card_id` (which must be registered in `card_data`) into `player`'s
/// trash zone. The DebugRunner builder has no `trash(...)` helper, so trash
/// fixtures are seeded post-`start()` (mirrors the BT24-017 test pattern).
fn push_into_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_into_trash: unknown card_id {card_id}"));
    let next = game.next_card_index();
    game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-189 must compile to exactly 3 clauses:
///   [0] on_security triggered (optional, FaceUp scope)
///   [1] grant_keyword Progress (declarative, FaceUp scope)
///   [2] on_opponent_security_removed triggered (inherited, once_per_turn)
#[test]
fn p_189_compiles_to_three_clauses() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("P-189")
        .expect("P-189 must be compiled and registered");
    assert_eq!(
        compiled.effects.len(),
        3,
        "Expected 3 compiled clauses: on_security, grant_keyword(Progress), inherited OPT"
    );
}

/// Clause 0 — on_security, FaceUp scope, optional, no once_per_turn.
#[test]
fn p_189_security_clause_is_face_up_optional_not_opt() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("P-189 must have an OnSecurity clause");

    assert_eq!(
        sec_clause.scope,
        CompiledScope::FaceUp,
        "Security clause is own-card effect → FaceUp scope"
    );
    assert!(
        sec_clause.optional,
        "'You may' text → optional must be true"
    );
    assert!(
        !sec_clause.once_per_turn,
        "Security clause has no [Once Per Turn]"
    );
}

/// Clause 1 — declarative GrantKeyword(Progress), FaceUp scope.
#[test]
fn p_189_has_progress_declarative_grant_keyword_clause() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let has_progress = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => keyword.to_lowercase().contains("progress") && *scope == CompiledScope::FaceUp,
        _ => false,
    });

    assert!(
        has_progress,
        "P-189 must have a declarative GrantKeyword(Progress) clause with FaceUp scope"
    );
}

/// Clause 2 — on_opponent_security_removed, Inherited scope, once_per_turn.
#[test]
fn p_189_inherited_clause_is_opt_on_opponent_security_removed() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let inherited_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .next()
        .expect("P-189 must have an Inherited triggered clause");

    assert!(
        inherited_clause
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "Inherited clause must fire on OnOpponentSecurityRemoved"
    );
    assert!(
        inherited_clause.once_per_turn,
        "[Once Per Turn] in text → once_per_turn must be true"
    );
    assert!(
        !inherited_clause.optional,
        "Inherited clause fires unconditionally when conditions pass (not user-optional)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [Security] clause: optionality (integrated path)
// ─────────────────────────────────────────────────────────────────────────────

/// Positive structural: the security clause has optional=true.
#[test]
fn p_189_security_clause_optional_allows_decline() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let sec = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_security clause");

    assert!(sec.optional, "Security 'you may' clause must be optional");
}

/// Integrated: P-189 sits in P1's security; P0 attacks the security. The
/// [Security] clause is printed "you may", so the `select_union_zone` prompt
/// is itself optional — declining it via PASS must play nothing.
///
/// Regression for G-SECURITY-SKILL-RESUME-REFIRE (RESOLVED): the
/// security-resolution drain arms of `combat.rs::drive_security_resolution`
/// now record `phase_enqueue_done` on `SecurityResolutionState`, so a
/// resume after a **declined** optional `[Security]` selection advances past
/// the security-skill phase instead of re-enqueuing `EffectTiming::SecuritySkill`
/// (which previously re-collected the same effect — an infinite loop). The
/// play path was always fine; this test pins the decline path.
#[test]
fn p_189_security_clause_can_be_declined() {
    use digimon_engine::action::space::PASS;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-1"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-1"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let hand_before = runner.game.player(1).hand.len();
    let battle_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);

    // The [Security] clause installs an optional union-zone selection.
    let prompt = runner
        .pending_selection_view()
        .expect("'You may' [Security] effect must install a union-zone selection");
    assert!(
        matches!(prompt.kind, SelectionKind::UnionZone { .. }),
        "the [Security] clause surfaces a UnionZone (hand ∪ trash) selection"
    );
    assert!(
        prompt.is_optional,
        "the union-zone selection must be optional (PASS legal — 'you may')"
    );
    runner
        .execute_action(prompt.selecting_player, PASS)
        .expect("decline the optional [Security] effect");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.player(1).hand.len(),
        hand_before,
        "declining the [Security] clause must leave P1's hand unchanged"
    );
    assert_eq!(
        runner.battle_area_size(1),
        battle_before,
        "declining the [Security] clause must play nothing onto P1's field"
    );
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "LIB-1"),
        "the LIBERATOR card stays in hand when the [Security] clause is declined"
    );
}

/// Integrated: when both hand and trash hold eligible LIBERATOR cards, the
/// [Security] clause's union-zone selection offers exactly two candidates
/// (one from each zone).
#[test]
fn p_189_security_clause_offers_hand_and_trash_candidates() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-HAND"))
        .add_card(make_liberator_low_cost("LIB-TRASH"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    push_into_trash(&mut runner, 1, "LIB-TRASH");
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must install after the security reveal");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the [Security] clause surfaces a UnionZone selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "exactly the hand LIBERATOR card and the trash LIBERATOR card are legal"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [Security] clause filter enforcement (G-PLAY-COST-LTE resolved)
// ─────────────────────────────────────────────────────────────────────────────

/// Cost-filter, positive boundary: a LIBERATOR card costing exactly 4 IS
/// selectable. `play_cost_lte: 4` is inclusive of 4. Playing it puts the card
/// on P1's field for free.
#[test]
fn p_189_security_filter_includes_cost_4_liberator_card() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_cost_4("LIB-4"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-4"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must install for the eligible cost-4 card");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the [Security] clause surfaces a UnionZone selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the cost-4 LIBERATOR card passes play_cost_lte: 4 and is the sole candidate"
    );

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play the cost-4 LIBERATOR card free");
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "LIB-4"),
        "the cost-4 LIBERATOR card must be played free onto P1's field"
    );
}

/// Cost-filter, negative: a LIBERATOR card costing 5 is EXCLUDED. The cost-5
/// LIBERATOR card fails `play_cost_lte: 4`; with no eligible candidate the
/// [Security] clause plays nothing — the card stays in hand.
#[test]
fn p_189_security_filter_excludes_high_cost_liberator_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_high_cost("LIB-5"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-5"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    // `auto_resolve` accepts the optional union-zone prompt if it installs
    // (PASS, since there are zero eligible candidates) or no prompt installs
    // at all — either way nothing is played.
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "LIB-5"),
        "a cost-5 LIBERATOR card must NOT be playable — play_cost_lte: 4 excludes it"
    );
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LIB-5"),
        "the cost-5 LIBERATOR card must remain in hand (excluded by play_cost_lte: 4)"
    );
}

/// Trait-filter, negative: a non-LIBERATOR card (cost ≤ 4) is EXCLUDED. The
/// non-LIBERATOR Digimon fails the `trait_has: LIBERATOR` filter; with no
/// eligible candidate the [Security] clause plays nothing.
#[test]
fn p_189_security_filter_excludes_non_liberator_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_non_liberator("NON-LIB"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["NON-LIB"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "NON-LIB"),
        "a non-LIBERATOR card must NOT be playable — trait_has: LIBERATOR excludes it"
    );
    assert!(
        runner
            .game
            .player(1)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "NON-LIB"),
        "the non-LIBERATOR card must remain in hand (excluded by the trait filter)"
    );
}

/// Combined filter: with a mix of one eligible (LIBERATOR cost 3) and two
/// ineligible cards (LIBERATOR cost 5, non-LIBERATOR cost 3) in hand, exactly
/// one card is a legal union-zone candidate.
#[test]
fn p_189_security_filter_offers_only_eligible_card_among_mixed_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-OK"))
        .add_card(make_liberator_high_cost("LIB-TOO-COSTLY"))
        .add_card(make_non_liberator("NON-LIB"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-OK", "LIB-TOO-COSTLY", "NON-LIB"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must install (one eligible card in hand)");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the [Security] clause surfaces a UnionZone selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the LIBERATOR cost-3 card passes both filters; \
         the cost-5 LIBERATOR and the non-LIBERATOR card are excluded"
    );

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play the sole eligible card free");
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "LIB-OK"),
        "the eligible LIBERATOR cost-3 card must be played free"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3b — [Security] clause: trash-origin play + free play accounting
// ─────────────────────────────────────────────────────────────────────────────

/// Integrated: when only the trash holds an eligible LIBERATOR card (cost ≤ 4),
/// the [Security] clause's union-zone selection offers that trash card and
/// `play_union_bound_free` plays it free — the card leaves the trash zone and
/// enters the battle area (origin-preserving play).
#[test]
fn p_189_security_clause_plays_liberator_free_from_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-TRASH"))
        .add_card(make_non_liberator("NON-LIB-HAND"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        // A non-LIBERATOR card in hand proves the union-zone pick reaches the
        // trash branch: the only legal candidate is the trash card.
        .hand(1, &["NON-LIB-HAND"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    push_into_trash(&mut runner, 1, "LIB-TRASH");
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    let hand_before = runner.game.player(1).hand.len();
    let battle_before = runner.battle_area_size(1);

    runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must install (candidate only in trash)");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "the [Security] clause surfaces a UnionZone selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "the LIBERATOR cost-3 card in trash is the sole eligible candidate"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play the LIBERATOR card free from trash");
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "LIB-TRASH"),
        "the LIBERATOR card must be played free from trash onto P1's field"
    );
    assert_eq!(
        runner.battle_area_size(1),
        battle_before + 1,
        "P1's battle area gains exactly the one free-played card"
    );
    // LIB-TRASH must no longer be in the trash zone — assert by card identity,
    // not zone count: the revealed security card P-189 is itself disposed into
    // P1's trash after the security check, so the raw trash count can stay flat
    // (−LIB-TRASH +P-189). Identity is the load-bearing check.
    assert!(
        !runner
            .game
            .player(1)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LIB-TRASH"),
        "LIB-TRASH must leave the trash zone when played free onto the field"
    );
    assert_eq!(
        runner.game.player(1).hand.len(),
        hand_before,
        "the hand is untouched — the played card came from trash"
    );
}

/// Free-play accounting: the [Security] play does NOT charge memory. Playing a
/// cost-3 LIBERATOR card via the [Security] clause leaves memory unchanged
/// (compared to the steady-state reading just before the play resolves).
#[test]
fn p_189_security_free_play_charges_no_memory() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-HAND"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("FILL", "Fill"))
        .hand(1, &["LIB-HAND"])
        .security(1, &["P-189"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must install");
    // Read memory immediately before resolving the free play.
    let memory_before_play = runner.memory();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play the LIBERATOR card free");
    let _ = runner.auto_resolve();

    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "LIB-HAND"),
        "the LIBERATOR card must be played onto P1's field"
    );
    assert_eq!(
        runner.memory(),
        memory_before_play,
        "the [Security] clause plays 'without paying the cost' — memory unchanged \
         by the cost-3 free play (before={memory_before_play}, after={})",
        runner.memory()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — steady-state / on-play sanity
// ─────────────────────────────────────────────────────────────────────────────

/// Fire the security clause via place_on_field to confirm pending_selection
/// is None at steady state before any trigger.
#[test]
fn p_189_no_pending_selection_at_steady_state() {
    let mut runner = dimetromon_runner();
    let _perm = runner.place_on_field(0, "P-189", Some(0));
    assert!(
        runner.pending_selection().is_none(),
        "No pending selection at steady state"
    );
}

/// When P-189 is played normally (on_play path, not security), there is no
/// on_play clause — so pending_selection should be None after play.
#[test]
fn p_189_on_play_no_effects_no_pending_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-1"))
        .hand(0, &["P-189"])
        .memory(10)
        .start();

    let before_hand = runner.game.players[0].hand.len();
    let _perm_idx = runner.play(0, 0);
    assert!(
        runner.pending_selection().is_none(),
        "P-189 has no on_play clause; no pending selection after play"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        before_hand - 1,
        "Hand size decreased by 1 after playing P-189"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Inherited clause behavioral (on_opponent_security_removed)
// ─────────────────────────────────────────────────────────────────────────────

/// Positive: when Dimetromon is in a digivolution stack (inherited) and the
/// opponent's security is removed, the controller gains 1 memory.
#[test]
fn p_189_inherited_gains_1_memory_on_opp_security_removed() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC-CARD", "SecCard"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC-CARD"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    insert_p189_as_source(&mut runner, 0, carrier_h);

    let memory_before = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let memory_after = runner.memory();

    assert!(
        memory_after > memory_before,
        "Dimetromon inherited clause must gain 1 memory when opponent security removed; \
         memory: {memory_before} -> {memory_after}"
    );
}

/// Negative condition: inherited clause should NOT fire on opponent's turn
/// (active_when: your_turn guards this).
#[test]
fn p_189_inherited_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER-P1", "AttackerP1"))
        .add_card(make_test_card("SEC-P0", "SecP0"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(0, &["SEC-P0"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    insert_p189_as_source(&mut runner, 0, carrier_h);

    runner.end_turn();

    let attacker = runner.place_on_field(1, "ATTACKER-P1", Some(0));
    let memory_before = runner.memory();
    runner.attack_player(attacker, 0, false);
    let _ = runner.auto_resolve();
    let memory_after = runner.memory();

    assert_eq!(
        memory_after, memory_before,
        "Dimetromon inherited clause must NOT fire on opponent's turn; \
         memory: {memory_before} -> {memory_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — OPT enforcement (inherited clause)
// ─────────────────────────────────────────────────────────────────────────────

/// OPT lockout: second firing in the same turn must be gated.
#[test]
fn p_189_inherited_opt_blocks_second_activation_same_turn() {
    // memory(5) leaves headroom for the inherited clause's +1 gain (memory
    // is clamped to rules.memory_range = (-10, 10)).
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC-1", "Sec1"))
        .add_card(make_test_card("SEC-2", "Sec2"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC-1", "SEC-2"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    insert_p189_as_source(&mut runner, 0, carrier_h);

    let m0 = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let m1 = runner.memory();
    let first_delta = m1 - m0;

    assert!(
        first_delta >= 1,
        "first security removal must gain memory; delta={first_delta}"
    );

    if runner.game_over() {
        return;
    }

    let carrier2 = runner.perm_handle(0, 0);
    let m2 = runner.memory();
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    let m3 = runner.memory();
    let second_delta = m3 - m2;

    assert!(
        second_delta < first_delta,
        "OPT must block second inherited trigger; first_delta={first_delta}, second_delta={second_delta}"
    );
}

/// OPT lockout clears after end_turn.
#[test]
fn p_189_inherited_opt_clears_after_end_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC-P1-1", "SecP1A"))
        .add_card(make_test_card("SEC-P1-2", "SecP1B"))
        .add_card(make_test_card("FILL", "Fill"))
        .security(1, &["SEC-P1-1", "SEC-P1-2"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();

    let carrier_h = runner.place_on_field(0, "CARRIER", Some(0));
    insert_p189_as_source(&mut runner, 0, carrier_h);

    let m0 = runner.memory();
    runner.attack_player(carrier_h, 1, false);
    let _ = runner.auto_resolve();
    let first_delta = runner.memory() - m0;

    if runner.game_over() {
        return;
    }

    // End P0's turn, then P1's turn back to P0's turn.
    runner.end_turn();
    if runner.game_over() {
        return;
    }
    runner.end_turn();

    let carrier_after = runner.perm_handle(0, 0);
    let m2 = runner.memory();
    runner.attack_player(carrier_after, 1, false);
    let _ = runner.auto_resolve();
    let second_delta = runner.memory() - m2;

    assert!(
        first_delta >= 1 && second_delta >= 1,
        "OPT must reset after turn cycle; first_delta={first_delta}, second_delta={second_delta}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 7 — Progress keyword (declarative clause, G-DECLARATIVE-KEYWORD resolved)
// ─────────────────────────────────────────────────────────────────────────────

/// Behavioral: a P-189 permanent on the field has `<Progress>` active — the
/// declarative `grant_keyword` clause installs at runtime and the keyword is
/// discoverable via `Game::has_keyword`.
#[test]
fn p_189_progress_keyword_active_on_field() {
    let mut runner = dimetromon_runner();
    let p189 = runner.place_on_field(0, "P-189", Some(0));

    assert!(
        runner.game.has_keyword(p189, Keyword::Progress),
        "the declarative grant_keyword(Progress) clause must install <Progress> \
         at runtime on a P-189 permanent on the field"
    );
}

/// Inject a `PendingAttack` with P-189 as the attacker so the Progress gate's
/// "carrier is the current attacker" check passes. `attack_player` resolves
/// the whole attack to completion, so by the time the test could observe
/// `current_attacker()` it is already `None`; this mirrors the established
/// pattern in `tests/keyword_phase_f/progress.rs`.
fn inject_p189_as_attacker(
    runner: &mut DebugRunner,
    attacker: digimon_engine::permanent::PermanentHandle,
) {
    use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};
    runner.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: AttackTarget::Player(1 - attacker.player),
        effective_target: AttackTarget::Player(1 - attacker.player),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        return_phase: digimon_engine::enums::GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
}

/// Behavioral positive: while P-189 is attacking, `<Progress>` excludes the
/// opponent's effects — `progress_excludes` returns true for an
/// opponent-sourced mutation targeting the attacking P-189.
#[test]
fn p_189_progress_excludes_opponent_effects_while_attacking() {
    let mut runner = dimetromon_runner();
    let p189 = runner.place_on_field(0, "P-189", Some(0));

    // Put P-189 into the attack window (it is the current attacker).
    inject_p189_as_attacker(&mut runner, p189);

    assert!(
        runner.game.has_keyword(p189, Keyword::Progress),
        "P-189 must carry <Progress> while on the field"
    );
    assert!(
        runner.game.progress_excludes(p189, Some(1)),
        "<Progress>: while P-189 is attacking, opponent (player 1) effects must \
         not affect it — progress_excludes must return true"
    );
}

/// Behavioral negative: `<Progress>` is opponent-only. The carrier's OWN
/// controller's effects always pass through — `progress_excludes` returns
/// false for a player-0-sourced mutation targeting P-189 (controlled by 0).
#[test]
fn p_189_progress_does_not_exclude_own_effects() {
    let mut runner = dimetromon_runner();
    let p189 = runner.place_on_field(0, "P-189", Some(0));

    inject_p189_as_attacker(&mut runner, p189);

    assert!(
        !runner.game.progress_excludes(p189, Some(0)),
        "<Progress> is opponent-only — P-189's own controller (player 0) effects \
         must pass through; progress_excludes must return false"
    );
    // Sanity: an opponent-sourced effect IS excluded in the same window.
    assert!(
        runner.game.progress_excludes(p189, Some(1)),
        "opponent-sourced effects ARE excluded while P-189 attacks (control check)"
    );
}

/// Behavioral negative: `<Progress>` is dormant when P-189 is NOT attacking.
/// The keyword is on the card, but with no attack in flight
/// `progress_excludes` returns false.
#[test]
fn p_189_progress_dormant_when_not_attacking() {
    let mut runner = dimetromon_runner();
    let p189 = runner.place_on_field(0, "P-189", Some(0));

    // No attack declared — P-189 is not the current attacker.
    assert!(
        runner.game.has_keyword(p189, Keyword::Progress),
        "the keyword is present on the card regardless of attack state"
    );
    assert!(
        !runner.game.progress_excludes(p189, Some(1)),
        "not attacking → <Progress> is dormant → opponent effects are NOT excluded"
    );
}
