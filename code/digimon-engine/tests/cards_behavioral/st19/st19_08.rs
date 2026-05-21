//! ST19-08 ShoeShoemon.
//! Printed text:
//! - [Security] You may play 1 [LIBERATOR] card with play cost 4 or less
//!   from hand or trash free.
//! - <Overclock ([Puppet] Trait)>.
//! - Inherited [Your Turn] all opponent security Digimon get -3000 DP.
//!
//! The [Security] hand-or-trash union play uses the G014 substrate
//! (select_union_zone + play_union_bound_free) landed in the Task-8 sweep.
//! The inherited opponent security Digimon DP aura remains blocked by
//! G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

#[test]
fn st19_08_yaml_loads() {
    let _runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
}

#[test]
fn st19_08_grants_overclock_with_puppet_cost_filter() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();

    let shoe = runner.place_on_field(0, "ST19-08", Some(0));

    assert!(runner.game.has_keyword(shoe, Keyword::Overclock));

    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");
    let overclock_clause = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                overclock_cost_filter,
                ..
            }) => keyword
                .eq_ignore_ascii_case("Overclock")
                .then_some(overclock_cost_filter),
            _ => None,
        })
        .expect("ST19-08 must grant Overclock");
    let filter = overclock_clause
        .as_ref()
        .expect("Overclock must carry a Puppet/token sacrifice filter");

    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.kind == Some(CompiledCardKind::Token)),
        "Overclock cost allows deleting one of your Tokens"
    );
    assert!(
        filter.any_of.iter().any(|branch| {
            branch
                .all_of
                .iter()
                .any(|leaf| leaf.trait_has.as_deref() == Some("Puppet"))
        }),
        "Overclock cost allows other Puppet trait Digimon"
    );
}

#[test]
fn st19_08_security_may_play_liberator_cost_4_or_less_from_hand_or_trash() {
    let runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");

    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Triggered(triggered) => {
                triggered
                    .when
                    .contains(&digimon_dsl::compiled::CompiledTiming::OnSecurity)
                    && triggered.optional
            }
            _ => false,
        }),
        "Security text should compile to an optional on_security union-zone play"
    );
}

// ─── G014 behavioral helpers ──────────────────────────────────────────────────

/// A strong attacker for player 0 — high enough DP to always win the security
/// battle against ST19-08 (5 000 DP) so the security-skill pipeline runs.
fn strong_attacker(id: &str) -> CardData {
    let mut card = make_test_card(id, "Strong Attacker");
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(9000);
    card.play_cost = 8;
    card.colors = vec![CardColor::Red];
    card
}

/// A legal target: [LIBERATOR] trait, play cost ≤ 4.
fn liberator_cost4(id: &str) -> CardData {
    let mut card = make_test_card(id, "Liberator4");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 4;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

/// An illegal target: [LIBERATOR] trait but play cost > 4 (cost 5).
fn liberator_cost5(id: &str) -> CardData {
    let mut card = make_test_card(id, "Liberator5");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Puppet".to_string(), "LIBERATOR".to_string()];
    card
}

/// An illegal target: non-[LIBERATOR] (e.g. Beast trait).
fn non_liberator(id: &str) -> CardData {
    let mut card = make_test_card(id, "NonLiberator");
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Beast".to_string()];
    card
}

/// Push a registered card directly into player `p`'s trash zone.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: card {} not registered", card_id));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

/// Helper: is `card_id` present in player's battle area?
fn in_battle_area(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.player(player).battle_area.iter().any(|perm| {
        perm.card_sources
            .iter()
            .any(|src| src.card_id(&runner.game.card_data) == card_id)
    })
}

#[test]
#[ignore = "pending: G-OPPONENT-SECURITY-DP-AURA / PUPPETS-G008 - DSL cannot express inherited applies_to_opponent_security_dp"]
fn st19_08_inherited_reduces_opponent_security_digimon_dp_during_your_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");

    assert!(
        compiled.effects.iter().any(|clause| match clause {
            CompiledClause::Declarative(aura) => format!("{aura:?}").contains("opponent_security"),
            _ => false,
        }),
        "Inherited aura should lower to opponent-security-Digimon DP adjustment"
    );
}

/// G014 behavioral test: ST19-08 in security offers exactly the two legal
/// [LIBERATOR]-cost-≤-4 candidates (one from hand, one from trash) and
/// excludes illegal ones.
///
/// Setup
/// -----
/// - Player 0: a 9 000-DP attacker on the field (beats ST19-08's 5 000 DP).
/// - Player 1: ST19-08 in security; hand = [LIB_HAND (legal), NONLIB_HAND (no LIBERATOR trait)];
///   trash = [LIB_TRASH (legal), LIB_COST5_TRASH (illegal — cost 5)].
///
/// Flow
/// ----
/// 1. `attack_player` → security skill fires → `select_union_zone` pending →
///    returns `InProgress`.
/// 2. Assert `SelectionKind::UnionZone`, optional, exactly 2 valid action IDs
///    (LIB_HAND and LIB_TRASH).
/// 3. Select any one valid action; `auto_resolve` completes the security pipeline.
/// 4. Assert one card entered the battle area for free (exactly one card left
///    the combined hand ∪ trash, memory unchanged).
#[test]
fn st19_08_security_g014_filters_and_plays_liberator_cost4_from_union_zone() {
    // ── Build the runner ────────────────────────────────────────────────────
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .add_card(strong_attacker("ATK"))
        .add_card(liberator_cost4("LIB_HAND"))    // legal hand candidate
        .add_card(liberator_cost4("LIB_TRASH"))   // legal trash candidate
        .add_card(non_liberator("NONLIB_HAND"))   // illegal: wrong trait
        .add_card(liberator_cost5("LIB_COST5_TRASH")) // illegal: cost > 4
        // Player 1's hand: one legal, one wrong-trait (cost≤4 but no LIBERATOR)
        .hand(1, &["LIB_HAND", "NONLIB_HAND"])
        // Decks must be non-empty only if draw effects fire; keep minimal.
        .deck(0, &[])
        .deck(1, &[])
        // ST19-08 in player 1's security stack (top = last pushed).
        .security(1, &["ST19-08"])
        .memory(5)
        .start();

    // Seed player 1's trash manually (push_to_trash helper above).
    push_to_trash(&mut runner, 1, "LIB_TRASH");
    push_to_trash(&mut runner, 1, "LIB_COST5_TRASH");

    let memory_before = runner.memory();

    // ── Trigger the security check ──────────────────────────────────────────
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let result = runner.attack_player(atk, 1, false);

    // The on_security clause installed a union-zone selection → InProgress.
    assert_eq!(
        result,
        AttackResult::InProgress,
        "on_security union-zone pick must park the security pipeline"
    );

    // ── Assert the pending selection ────────────────────────────────────────
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending after security reveal");

    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "selection kind must be UnionZone (hand ∪ trash)"
    );
    assert!(
        runner.pending_is_optional(),
        "printed 'You may' must surface PASS at the union-zone selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "exactly LIB_HAND (hand) and LIB_TRASH (trash) must be legal; \
         NONLIB_HAND (no LIBERATOR trait) and LIB_COST5_TRASH (cost 5 > 4) must be excluded"
    );

    // ── Choose any valid action and complete the pipeline ───────────────────
    let selected_action = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, selected_action)
        .expect("executing a valid union-zone action must succeed");
    runner
        .auto_resolve()
        .expect("security pipeline must complete after selection");

    // ── Post-resolution assertions ──────────────────────────────────────────
    // The printed text plays exactly ONE [LIBERATOR] card. The [Security]
    // clause resolves once, so exactly one of the two legal candidates
    // reaches the battle area and the other stays in its origin zone.
    let lib_hand_played = in_battle_area(&runner, 1, "LIB_HAND");
    let lib_trash_played = in_battle_area(&runner, 1, "LIB_TRASH");
    assert!(
        lib_hand_played ^ lib_trash_played,
        "exactly 1 [LIBERATOR] card must be played to the battle area for free \
         (LIB_HAND played={lib_hand_played}, LIB_TRASH played={lib_trash_played})"
    );

    // The unplayed candidate stays untouched in its origin zone.
    if lib_hand_played {
        assert!(
            runner.game.players[1]
                .trash
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "LIB_TRASH"),
            "the unplayed LIB_TRASH must remain in trash"
        );
    } else {
        assert!(
            runner.game.players[1]
                .hand
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "LIB_HAND"),
            "the unplayed LIB_HAND must remain in hand"
        );
    }

    // Memory must not have changed — play_union_bound_free charges nothing.
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the played card's play cost"
    );
}

/// G014 behavioral test — trash-origin branch: ST19-08 in security with NO
/// legal hand candidate forces a single trash-origin action, proving the
/// trash branch is reached deterministically.
///
/// Setup
/// -----
/// - Player 0: a 9 000-DP attacker on the field.
/// - Player 1: ST19-08 in security; hand = [NONLIB_HAND (no LIBERATOR trait)];
///   trash = [LIB_TRASH (legal, cost ≤ 4)].
///
/// Flow
/// ----
/// 1. `attack_player` → security skill fires → exactly 1 valid action ID
///    (the trash-origin LIB_TRASH).
/// 2. Select it; `auto_resolve` completes the pipeline.
/// 3. Assert LIB_TRASH is in the battle area (trash-origin confirmed), LIB_TRASH
///    is no longer in the trash, hand was not touched, memory unchanged.
#[test]
fn st19_08_security_g014_plays_liberator_from_trash_when_no_legal_hand_candidate() {
    // ── Build the runner ────────────────────────────────────────────────────
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .add_card(strong_attacker("ATK"))
        .add_card(liberator_cost4("LIB_TRASH"))   // legal trash candidate
        .add_card(non_liberator("NONLIB_HAND"))   // illegal: wrong trait
        // Player 1's hand: one wrong-trait card — no legal hand candidate.
        .hand(1, &["NONLIB_HAND"])
        .deck(0, &[])
        .deck(1, &[])
        // ST19-08 in player 1's security stack.
        .security(1, &["ST19-08"])
        .memory(5)
        .start();

    // Seed only the trash-origin candidate.
    push_to_trash(&mut runner, 1, "LIB_TRASH");

    let hand_before = runner.game.players[1].hand.len(); // 1

    // ── Trigger the security check ──────────────────────────────────────────
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let result = runner.attack_player(atk, 1, false);

    assert_eq!(
        result,
        AttackResult::InProgress,
        "on_security union-zone pick must park the security pipeline"
    );

    // ── Assert the pending selection ────────────────────────────────────────
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending after security reveal");

    assert!(
        matches!(view.kind, SelectionKind::UnionZone { .. }),
        "selection kind must be UnionZone"
    );
    assert!(
        runner.pending_is_optional(),
        "printed 'You may' must surface PASS"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only LIB_TRASH (trash) should be legal when there is no legal hand candidate"
    );

    let memory_before = runner.memory();

    // ── Select the single (trash-origin) action ─────────────────────────────
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("executing the single trash-origin action must succeed");
    runner
        .auto_resolve()
        .expect("security pipeline must complete after selection");

    // ── Post-resolution assertions ──────────────────────────────────────────
    // LIB_TRASH must be in the battle area — confirms the trash-origin branch.
    assert!(
        in_battle_area(&runner, 1, "LIB_TRASH"),
        "LIB_TRASH must have been played to the battle area from trash"
    );
    // LIB_TRASH must no longer be in the trash.
    let lib_still_in_trash = runner.game.players[1]
        .trash
        .iter()
        .any(|src| src.card_id(&runner.game.card_data) == "LIB_TRASH");
    assert!(
        !lib_still_in_trash,
        "LIB_TRASH must have been removed from trash when played to the battle area"
    );
    // The hand must not have changed — the card came from trash, not hand.
    assert_eq!(
        runner.game.players[1].hand.len(),
        hand_before,
        "hand must be untouched — the played card came from trash"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the played card's play cost"
    );
}
