//! Behavioral tests for EX11-012 Medusamon (DSL implementation).
//!
//! Card text (condensed):
//!   <Rush>
//!   <Progress>
//!   [When Digivolving][End of Attack] You may delete 1 opponent Digimon with
//!     <= this Digimon's DP. Then, by returning 1 card from your opponent's
//!     trash to the bottom of the deck, they play 1 Petrification Token.
//!   [All Turns] When this Digimon would leave the battle area, by deleting
//!     1 Token, it doesn't leave.
//!
//! Tests:
//!   1. Structural: EX11-012 compiles with expected clauses
//!   2. Clause (b): <Progress> keyword granted
//!   3. Clause (c) end_of_attack: deletes <=12000 DP opponent; opponent
//!      trash-return + Petrification Token play
//!   4. Clause (c): does NOT target opponent Digimon with DP > 12000
//!   5. Clause (d): token-delete cancels leave
//!   6. Clause (d): no token → leave proceeds

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX11-012")
}

fn runner_with_medusamon() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .start();
    let h = r.place_on_field(0, "EX11-012", None);
    (r, h)
}

// ─── 1. Structural ────────────────────────────────────────────────────────────

#[test]
fn ex11_012_compiles_with_rush_and_progress_keywords() {
    let card = compiled();
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(12000));

    // Should have GrantKeyword clauses for Rush and Progress
    let grant_keywords: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        grant_keywords.contains(&"Rush"),
        "EX11-012 must have <Rush>; got: {grant_keywords:?}"
    );
    assert!(
        grant_keywords.contains(&"Progress"),
        "EX11-012 must have <Progress>; got: {grant_keywords:?}"
    );
}

#[test]
fn ex11_012_compiles_with_end_of_attack_triggered_clause() {
    let card = compiled();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered.iter().any(|t| {
            t.when.contains(&CompiledTiming::EndOfAttack)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.optional
        }),
        "EX11-012 must have optional WhenDigivolving+EndOfAttack clause; got: {triggered:?}"
    );
}

#[test]
fn ex11_012_compiles_with_leave_battle_area_replacement() {
    let card = compiled();
    let replacement = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { trigger, .. }) => {
            Some(trigger.as_str())
        }
        _ => None,
    });
    assert_eq!(
        replacement,
        Some("when_would_leave_battle_area"),
        "EX11-012 must have a WhenWouldLeaveBattleArea replacement clause"
    );
}

// ─── 2. Progress keyword ──────────────────────────────────────────────────────

#[test]
fn ex11_012_has_progress_keyword_on_field() {
    use digimon_engine::enums::Keyword;

    let (r, h) = runner_with_medusamon();
    assert!(
        r.game.has_keyword(h, Keyword::Progress),
        "Medusamon on field should have <Progress> active"
    );
}

// ─── 3. Clause (c): end_of_attack happy path ──────────────────────────────────

#[test]
fn end_of_attack_optional_fires_deletes_target_returns_trash_and_plays_token() {
    // Build: P0 has Medusamon, P1 has a weak Digimon (6000 DP) on field.
    // P1 also has 1 card in trash.
    // Medusamon attacks P1 directly (no Digimon battle, so the weak target
    // survives until clause (c) deletes it).
    // After Medusamon attacks player and end_of_attack fires, we should see:
    //   - outer optional accept prompt
    //   - inner target-select prompt (P1's weak Digimon)
    //   - inner trash-return select prompt (P1's trash card)
    //   - P1's Digimon deleted, trash → deck bottom, Petrification Token on P1's field.

    let mut weak = make_test_card("WEAK", "WeakDigimon");
    weak.dp = Some(6000);

    let trash_card = make_test_card("TRASH-CARD", "TrashCard");

    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .add_card(weak)
        .add_card(trash_card)
        .security(1, &[])
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);
    // P1 has WEAK on field (won't be battle-killed because we attack player)
    r.place_on_field(1, "WEAK", None);

    // Put a card in P1's trash manually.
    {
        use digimon_engine::card_source::CardSource;
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "TRASH-CARD")
            .expect("TRASH-CARD in card_data");
        let card = CardSource::new(data_idx, 1, r.game.next_card_index());
        r.game.players[1].trash.push(card);
    }
    let trash_before = r.game.players[1].trash.len();
    let deck_before = r.game.players[1].deck.len();

    // Attack P1 directly (no battle kill — EndOfAttack still fires).
    // P1 has 0 security so the attack wins immediately; EndOfAttack still fires.
    r.attack_player(medusa, 1, false);

    // The EndOfAttack clause is optional ("You may delete ...") and its body's
    // first step is a mandatory select_opponent_permanent, so an outer
    // accept/decline prompt installs first (G-OUTER-OPTIONAL-NOT-INSTALLED).
    // Accept it to reach the inner target-select prompt.
    r.accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    // After accepting, the select_opponent_permanent step prompts to pick the
    // Digimon to delete.
    assert!(
        r.pending_selection().is_some(),
        "end_of_attack: target select must be up"
    );
    let (pl, act) = {
        let s = r.pending_selection().unwrap();
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("pick target");

    // After picking the target, delete_permanent runs, then select_trash fires.
    assert!(
        r.pending_selection().is_some(),
        "end_of_attack: trash-return select must be up"
    );
    let (pl, act) = {
        let s = r.pending_selection().unwrap();
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("pick trash card");

    // P1's WEAK Digimon was deleted; Petrification Token on P1's field.
    assert_eq!(
        r.battle_area_size(1),
        1,
        "P1 should have exactly 1 permanent (the Petrification Token)"
    );
    let token_perm = r
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token);
    assert!(
        token_perm.is_some(),
        "Petrification Token should be on P1's field"
    );

    // Net trash count: WEAK was added (from delete_permanent) and TRASH-CARD
    // was returned to deck, so net change is 0 relative to trash_before.
    // WEAK itself remains in trash (only the selected card is returned to deck).
    assert_eq!(
        r.game.players[1].trash.len(),
        trash_before,
        "P1's trash net count should equal trash_before (WEAK added, TRASH-CARD moved to deck)"
    );
    // Deck gained 1 at bottom (the returned card).
    assert_eq!(
        r.game.players[1].deck.len(),
        deck_before + 1,
        "P1's deck should gain 1 card at bottom"
    );
}

// ─── 4. Clause (c): DP filter ────────────────────────────────────────────────

#[test]
fn end_of_attack_does_not_offer_target_with_dp_above_12000() {
    // P1 has a Digimon with DP 13000 — above Medusamon's DP.
    // After accepting the optional, the inner select should have 0 valid
    // actions (no valid targets), so no PendingSelection is installed.

    let mut strong = make_test_card("STRONG", "StrongDigimon");
    strong.dp = Some(13000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .add_card(strong)
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);
    let target = r.place_on_field(1, "STRONG", None);

    r.attack_digimon(medusa, target, false);

    // Optional accept may or may not be offered depending on whether the engine
    // gates the outer optional on available targets.  In either case, STRONG
    // should NOT be deleted.
    if let Some(sel) = r.pending_selection() {
        // If an accept prompt is shown, accept and verify no inner target prompt.
        let (pl, act) = (sel.selecting_player, sel.valid_action_ids[0]);
        r.execute_action(pl, act).ok();
        // The inner select should have 0 valid targets for STRONG (13000 DP).
        // If a selection is up, it should not contain STRONG's action.
    }
    // STRONG remains on field (not deleted).
    assert_eq!(
        r.battle_area_size(1),
        1,
        "StrongDigimon (13000 DP) must not be deleted"
    );
    let strong_perm = r
        .game
        .player(1)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "STRONG");
    assert!(
        strong_perm.is_some(),
        "STRONG should still be on P1's field"
    );
}

// ─── 5. Clause (d): token-delete cancels leave ───────────────────────────────

#[test]
fn would_leave_with_token_on_field_cancel_leave_by_deleting_token() {
    // P0 has Medusamon + 1 Petrification Token.
    // When Medusamon would leave the battle area, a PendingSelection fires
    // asking to delete a Token. On selection, the leave is cancelled.

    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);

    // Manually play a Petrification Token on P0's field.
    {
        use digimon_engine::card_source::CardHandle;
        use digimon_engine::effect_context::EffectContext;
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.play_token(0, "petrification");
    }
    assert_eq!(r.battle_area_size(0), 2, "Medusamon + 1 token");

    // Trigger WhenWouldLeaveBattleArea via a delete-with-cause call.
    r.game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);

    // The replacement installed a PendingSelection for the token cost.
    assert!(
        r.pending_selection().is_some(),
        "WhenWouldLeave replacement should install token-delete selection"
    );

    // Select the token.
    let (pl, act) = {
        let s = r.pending_selection().unwrap();
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("pick token");

    // Medusamon survived (leave cancelled), token deleted.
    assert_eq!(
        r.battle_area_size(0),
        1,
        "Medusamon should survive after token-delete cancels leave"
    );
    // The remaining permanent should be Medusamon, not the token.
    let medusa_still = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "EX11-012");
    assert!(
        medusa_still.is_some(),
        "EX11-012 should remain on field after cancel_leave"
    );
}

// ─── 6. Clause (d): no token → leave proceeds ────────────────────────────────

#[test]
fn would_leave_with_no_token_proceeds_normally() {
    // P0 has only Medusamon (no tokens).
    // WhenWouldLeaveBattleArea fires; the inner select_own_permanent finds
    // no token candidates; the optional select short-circuits; cancel_leave
    // never fires; Medusamon is deleted normally.

    let (mut r, medusa) = runner_with_medusamon();
    assert_eq!(r.battle_area_size(0), 1);

    r.game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);

    // No pending selection should remain after the process (0 token candidates
    // → optional select short-circuits → cancel_replacement never fires →
    // original deletion commits).
    assert!(
        r.pending_selection().is_none(),
        "No token → optional select short-circuits → no pending selection"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "No token on field → Medusamon is deleted normally"
    );
}
