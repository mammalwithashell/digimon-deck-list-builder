//! ST19-08 ShoeShoemon — Digimon, Lv.4, Yellow, Puppet/LIBERATOR.
//!
//! # Card text (cards.json)
//!
//! [Security] You may play 1 card with the [LIBERATOR] trait and a play cost
//! of 4 or less from your hand or trash without paying the cost.
//!
//! <Overclock ([Puppet] Trait)> (At the end of your turn, by deleting 1 of
//! your Tokens or other [Puppet] trait Digimon, this Digimon attacks a player
//! without suspending.)
//!
//! Inherited Effect [Your Turn] All of your opponent's security Digimon get
//! -3000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST19/Yellow/ST19_08.cs
//!
//! # Patterns this test covers
//! - H11 Overclock (Puppet/token sacrifice cost filter)
//! - C5-adjacent: [Security] hand-or-trash union-zone free play (G014 substrate:
//!   select_union_zone + play_union_bound_free)
//! - D4 declarative aura — inherited opponent-security-Digimon DP debuff
//!   (PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA, RESOLVED Track I 2026-05-17;
//!   `applies_to_opponent_security_dp: true`)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledDeclarativeClause};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
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

/// `field_index`/`effect_slot` → action ID for a field-effect activation.
fn encode_field_effect(field_index: u16, effect_slot: u16) -> u16 {
    FIELD_EFFECT_START + field_index * EFFECTS_PER_PERMANENT + effect_slot
}

/// A `Token` card usable as an Overclock sacrifice cost.
fn make_token(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Token;
    card.level = None;
    card
}

/// A Lv.4 Digimon with explicit traits — used as legal / illegal Overclock costs.
fn make_digimon_with_traits(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.colors = vec![CardColor::Yellow];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Integrated Overclock: at end-of-turn ShoeShoemon's <Overclock> is a legal
/// field-effect action; activating it surfaces a cost selection that masks
/// in a Token and another Puppet Digimon but excludes a non-Puppet Digimon,
/// and PASS remains legal (the keyword cost is optional).
#[test]
fn st19_08_overclock_end_of_turn_masks_only_token_or_other_puppet_costs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .add_card(make_token("COST-TOKEN"))
        .add_card(make_digimon_with_traits("NON-PUPPET", &["Beast"]))
        .add_card(make_digimon_with_traits("PUPPET-ALLY", &["Puppet"]))
        .start();

    let shoe = runner.place_on_field(0, "ST19-08", Some(0));
    runner.place_on_field(0, "COST-TOKEN", Some(0));
    runner.place_on_field(0, "NON-PUPPET", Some(0));
    runner.place_on_field(0, "PUPPET-ALLY", Some(0));
    runner.game.current_phase = GamePhase::EndOfTurnAction;

    // ShoeShoemon (field index 0) exposes Overclock as a legal end-turn action.
    let overclock_action = encode_field_effect(shoe.index as u16, FIELD_EFFECT_SLOT_FOR_OVERCLOCK);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[overclock_action as usize], 1.0,
        "ShoeShoemon's <Overclock> is a legal end-of-turn field effect"
    );

    runner.game.decode_action(overclock_action, 0);

    // The Overclock cost prompt: delete 1 Token or other Puppet Digimon.
    let cost_mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        cost_mask[encode_attack(0, 1) as usize],
        1.0,
        "a Token is a legal Overclock sacrifice cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 2) as usize],
        0.0,
        "a non-Puppet non-token Digimon is not a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[encode_attack(0, 3) as usize],
        1.0,
        "another Puppet Digimon is a legal Overclock cost"
    );
    assert_eq!(
        cost_mask[PASS as usize], 1.0,
        "Overclock is optional — PASS must stay legal (no auto-pick)"
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

/// Structural: the inherited clause must lower to an `Aura` carrying
/// `applies_to_opponent_security_dp == true`, `scope == Inherited`, and a
/// `-3000` `dp_modifier`. A plain string match on the Debug output would be a
/// false-positive trap — the `applies_to_opponent_security_dp` field name is
/// printed for *every* `Aura` variant (including the unrelated GrantKeyword
/// aura ST19-08 also declares), so this test inspects the field value.
#[test]
fn st19_08_inherited_security_dp_aura_lowers_with_opponent_security_flag() {
    let runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("ST19-08")
        .expect("ST19-08 must be compiled");

    let aura = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                applies_to_opponent_security_dp,
                dp_modifier,
                scope,
                ..
            }) if *applies_to_opponent_security_dp => Some((dp_modifier, scope)),
            _ => None,
        })
        .expect("ST19-08 must declare an opponent-security-DP aura");

    let (dp_modifier, scope) = aura;
    assert_eq!(
        *dp_modifier,
        Some(-3000),
        "the inherited aura must lower opponent security Digimon DP by 3000"
    );
    assert_eq!(
        *scope,
        digimon_dsl::compiled::CompiledScope::Inherited,
        "the security-DP aura is an inherited effect — it rides under the stack"
    );
}

// ─── Inherited opponent-security-DP aura: behavioral coverage ─────────────────
//
// PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA. The aura rides under the
// attacker's stack and is consulted at security-battle time. These tests follow
// the ST19-03 pattern: insert ST19-08 as an inherited (under-the-top) card in
// the attacker's stack, then assert the security battle outcome flips.

/// A Lv.4 attacker with the given DP.
fn attacker_lv4(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card.colors = vec![CardColor::Red];
    card
}

/// A Digimon used as a security card with the given DP.
fn digimon_security_card(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}

/// Insert a registered `card_id` as an inherited card (index 0, under the top)
/// of the permanent at `handle`.
fn insert_inherited(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
    card_id: &str,
) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("insert_inherited: {card_id} not registered"));
    let next = game.next_card_index();
    let perm = &mut game.players[handle.player as usize].battle_area[handle.index as usize];
    let mut src = CardSource::new(data_idx, handle.player, next);
    src.card_index = next;
    perm.card_sources.insert(0, src);
}

/// Positive: ST19-08 inherited under the attacker's stack drops the opponent's
/// 9 000-DP security Digimon to 6 000, letting an 8 000-DP attacker survive the
/// security battle.
#[test]
fn st19_08_inherited_reduces_opponent_security_digimon_dp_during_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    insert_inherited(&mut runner, atk, "ST19-08");

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "ST19-08's -3000 inherited aura drops the 9000 security Digimon to 6000; \
         the 8000 attacker survives"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the attacker is not deleted by the (now weakened) security Digimon"
    );
}

/// Negative — carrier absent: with no ST19-08 in the attacker's stack the aura
/// is not consulted, so the 9 000-DP security Digimon deletes the 8 000-DP
/// attacker. Same board as the positive test minus the inherited carrier.
#[test]
fn st19_08_no_security_dp_debuff_when_carrier_absent_from_attacker_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-08")
        .expect("ST19-08 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without ST19-08 in the stack the aura never fires; \
         the 8000 attacker loses to the 9000 security Digimon"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "the attacker is deleted by the full-strength security Digimon"
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
        .add_card(liberator_cost4("LIB_HAND")) // legal hand candidate
        .add_card(liberator_cost4("LIB_TRASH")) // legal trash candidate
        .add_card(non_liberator("NONLIB_HAND")) // illegal: wrong trait
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
        .add_card(liberator_cost4("LIB_TRASH")) // legal trash candidate
        .add_card(non_liberator("NONLIB_HAND")) // illegal: wrong trait
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
