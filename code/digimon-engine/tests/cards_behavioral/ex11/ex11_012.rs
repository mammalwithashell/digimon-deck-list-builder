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

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
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
fn ex11_012_delete_trash_token_clause_uses_native_trash_return_step() {
    let card = compiled();
    let clause = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::EndOfAttack)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("EX11-012 optional delete/trash-return/token clause");

    assert!(
        !clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "EX11-012 trash-return clause must use native DSL, not raw_rust"
    );
    assert!(
        clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::ReturnTrashListToDeckBottom { .. })),
        "EX11-012 must return the selected trash binding with return_trash_list_to_deck_bottom"
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

// --- 7. Cross-card: the returned corpse MISSES ITS OWN [On Deletion] --------
//
// Medusamon's clause does two things in ONE effect: it deletes an opponent
// Digimon, and then -- as the optional processing condition for the Token --
// "by returning 1 card from your opponent's trash to the bottom of the deck".
// Those can be the SAME card: the corpse it just made.
//
// When they are, the deleted Digimon's own [On Deletion] must NOT resolve.
// 15-4-4-3: "when a card with an effect that's pending activation becomes a new
// card before the effect activates, the effect can no longer be activated." The
// deletion queues the trigger, but Medusamon's effect keeps resolving (15-4: an
// effect resolves completely before queued triggers activate), and by the time
// the queue drains the card has left the trash for the deck. It is a new card.
// The trigger misses timing.
//
// Same rule as ex12_047_ascension_first_makes_the_on_deletion_miss_timing,
// reached from the opposite direction: there the carrier moved ITSELF via
// <Ascension>; here an OPPONENT'S effect moves it.
//
// WHAT ENFORCES IT HERE IS NOT KNOWN, and that is deliberately recorded rather
// than guessed. The obvious candidate is the 15-4-4-3 guard in
// `Game::queued_effect_source_is_live` (effect_queue.rs), which invalidates a
// batched OnDeletion whose card has left the trash the deletion put it in --
// but STUBBING THAT GUARD OUT (`return true`) leaves all three tests below
// green, so it is not what produces this outcome. Measured mid-line: right
// after the corpse reaches the deck the queue still holds 3 entries and the
// TriggerOrder prompt is still up, so nothing has been culled at that point;
// whatever stops them happens during resolution.
//
// The behaviour is RIGHT and these tests pin it. But correctness we cannot
// attribute may be incidental -- e.g. <Fortitude> failing merely because the
// card is no longer in the trash it would play from -- and incidental
// correctness breaks silently. Treat the enforcement point as an open question,
// not as covered.
//
// Both targets are 12000 DP, exactly Medusamon's own, so the "as much or less
// DP as this Digimon" gate is satisfied at the boundary (12000 read off the
// EX11-012 card face).

/// Resolve the pending prompt by naming a TRASH card, so the pick cannot
/// silently follow a reordering of the trash.
fn pick_trash_by_id(r: &mut DebugRunner, card_id: &str) {
    // A trash-selection action id is TRASH_EFFECT_START + index into the
    // ZONE OWNER's trash (action/decode.rs:199-200), so the id can be resolved
    // back to a card rather than picked positionally.
    use digimon_engine::action::space::TRASH_EFFECT_START;
    let (player, action) = {
        let s = r
            .pending_selection()
            .expect("a trash-return prompt must be pending");
        let owner = s.zone_owner.unwrap_or(s.selecting_player) as usize;
        let trash = &r.game.players[owner].trash;
        let hit = s
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| {
                a.checked_sub(TRASH_EFFECT_START)
                    .and_then(|i| trash.get(i as usize))
                    .is_some_and(|c| c.card_id(&r.game.card_data) == card_id)
            })
            .unwrap_or_else(|| {
                panic!(
                    "{card_id} must be offered by the trash-return prompt; ids {:?}",
                    s.valid_action_ids
                )
            });
        (s.selecting_player, hit)
    };
    r.execute_action(player, action)
        .unwrap_or_else(|e| panic!("pick {card_id} from trash: {e:?}"));
}

fn deck_contains(r: &DebugRunner, player: usize, card_id: &str) -> bool {
    r.game.players[player]
        .deck
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == card_id)
}

fn field_has(r: &DebugRunner, player: usize, card_id: &str) -> bool {
    r.game.players[player]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == card_id)
}

fn trash_has(r: &DebugRunner, player: usize, card_id: &str) -> bool {
    r.game.players[player]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == card_id)
}

fn seed_decoy_trash(r: &mut DebugRunner) {
    use digimon_engine::card_source::CardSource;
    let idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DECOY-TRASH")
        .expect("DECOY-TRASH in card_data");
    let card = CardSource::new(idx, 1, r.game.next_card_index());
    r.game.players[1].trash.push(card);
}

fn fire_when_digivolving_on(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::WhenDigivolving,
        digimon_engine::selection::TriggerSource::Permanent(h),
    );
    r.game.drain_effect_queue();
}

/// EX12-065 Kaguyamon carries BOTH <Fortitude> ("play this Digimon from the
/// trash without paying the cost", 16-26, MANDATORY) and [On Deletion] ("Return
/// 1 of your opponent's lowest level Digimon to the bottom of the deck").
/// Returning it to the deck must cost it both.
#[test]
fn when_digivolving_returning_the_corpse_makes_kaguyamon_miss_its_on_deletion() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .dsl_card("EX12-065")
        .expect("EX12-065 in embedded pack")
        .add_card(make_test_card("DECOY-TRASH", "DecoyTrash"))
        .add_card(make_test_card("FODDER-SRC", "FodderSrc"))
        .security(1, &[])
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);
    // <Fortitude> (16-26) only fires for a Digimon deleted WITH digivolution
    // cards, so Kaguyamon is stacked over a source. Without one its
    // mandatory trigger legitimately no-ops and could not signal anything.
    r.place_stack(1, &["FODDER-SRC", "EX12-065"]);
    r.game.tick_declarative_effects();

    // A second trash card so the return is a REAL choice between two cards
    // rather than a forced single candidate -- otherwise this would pass even
    // if the engine ignored the pick entirely.
    seed_decoy_trash(&mut r);

    fire_when_digivolving_on(&mut r, medusa);

    r.accept_optional_trigger()
        .expect("accept Medusamon's outer optional prompt");

    let (pl, act) = {
        let s = r.pending_selection().expect("delete-target prompt");
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("delete Kaguyamon");

    assert!(
        trash_has(&r, 1, "EX12-065"),
        "the deletion must put Kaguyamon in its owner's trash first"
    );

    // The processing condition: return the CORPSE, not the decoy.
    pick_trash_by_id(&mut r, "EX12-065");
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();

    assert!(
        deck_contains(&r, 1, "EX12-065"),
        "Kaguyamon must have been returned to the bottom of its owner's deck"
    );
    assert!(
        !field_has(&r, 1, "EX12-065"),
        "Kaguyamon reached the deck, so its MANDATORY <Fortitude> (16-26) must NOT have played it back. The control below proves this is a real signal: leave Kaguyamon in the trash and <Fortitude> DOES return it to the field"
    );
    assert!(
        field_has(&r, 0, "EX11-012"),
        "Kaguyamon's [On Deletion] (return 1 of your opponent's lowest level Digimon to the bottom of the deck) must not have resolved either, so Medusamon must still be on the field"
    );
    assert!(
        !deck_contains(&r, 0, "EX11-012"),
        "Medusamon must not have been bottom-decked by a trigger that never should have activated"
    );
}

/// EX12-047 Amaterasumon carries <Ascension> ("place this card as the top
/// security card") alongside its [On Deletion]. Neither may resolve once the
/// card has been returned to the deck.
#[test]
fn when_digivolving_returning_the_corpse_makes_amaterasumon_miss_both_triggers() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .dsl_card("EX12-047")
        .expect("EX12-047 in embedded pack")
        .add_card(make_test_card("DECOY-TRASH", "DecoyTrash"))
        .add_card(make_test_card("FODDER-SRC", "FodderSrc"))
        .security(1, &[])
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);
    r.place_on_field(1, "EX12-047", None);
    r.game.tick_declarative_effects();
    seed_decoy_trash(&mut r);
    let security_before = r.game.players[1].security.len();

    fire_when_digivolving_on(&mut r, medusa);

    r.accept_optional_trigger()
        .expect("accept Medusamon's outer optional prompt");

    let (pl, act) = {
        let s = r.pending_selection().expect("delete-target prompt");
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("delete Amaterasumon");

    pick_trash_by_id(&mut r, "EX12-047");
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();

    assert!(
        deck_contains(&r, 1, "EX12-047"),
        "Amaterasumon must have been returned to the bottom of its owner's deck"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        security_before,
        "<Ascension> would place Amaterasumon as the top security card; it reached the deck instead, so the security stack must be unchanged"
    );
    assert!(
        !r.game.players[1]
            .security
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX12-047"),
        "Amaterasumon must not be in security -- it is in the deck"
    );
}

/// POSITIVE CONTROL for the two miss-timing tests above.
///
/// A test that asserts "nothing happened" is worthless without a sibling that
/// makes the same machinery visibly happen. Same board, same clause, same
/// deletion -- the ONLY difference is which card the processing condition
/// returns to the deck: the DECOY instead of the corpse.
///
/// Kaguyamon therefore STAYS in the trash where the deletion left it, so its
/// triggers keep their timing and must fire:
///   * <Fortitude> (16-26, MANDATORY) plays it back from the trash;
///   * [On Deletion] returns Medusamon (P0's only, hence lowest-level, Digimon)
///     to the bottom of P0's deck.
///
/// If this control ever goes quiet, the two negative tests above stop meaning
/// anything and must be re-derived rather than trusted.
#[test]
fn when_digivolving_returning_the_decoy_leaves_kaguyamon_triggers_intact() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .dsl_card("EX12-065")
        .expect("EX12-065 in embedded pack")
        .add_card(make_test_card("DECOY-TRASH", "DecoyTrash"))
        .add_card(make_test_card("FODDER-SRC", "FodderSrc"))
        .security(1, &[])
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);
    // <Fortitude> (16-26) only fires for a Digimon deleted WITH digivolution
    // cards, so Kaguyamon is stacked over a source. Without one its
    // mandatory trigger legitimately no-ops and could not signal anything.
    r.place_stack(1, &["FODDER-SRC", "EX12-065"]);
    r.game.tick_declarative_effects();
    seed_decoy_trash(&mut r);

    fire_when_digivolving_on(&mut r, medusa);
    r.accept_optional_trigger()
        .expect("accept Medusamon's outer optional prompt");

    let (pl, act) = {
        let s = r.pending_selection().expect("delete-target prompt");
        (s.selecting_player, s.valid_action_ids[0])
    };
    r.execute_action(pl, act).expect("delete Kaguyamon");
    assert!(
        trash_has(&r, 1, "EX12-065"),
        "the deletion must put Kaguyamon in its owner's trash first"
    );

    // The ONLY divergence from the negative tests: return the decoy.
    pick_trash_by_id(&mut r, "DECOY-TRASH");
    let _ = r.auto_resolve();
    r.game.drain_effect_queue();

    assert!(
        !deck_contains(&r, 1, "EX12-065"),
        "Kaguyamon must NOT have been returned to the deck in the control"
    );
    assert!(
        field_has(&r, 1, "EX12-065") || deck_contains(&r, 0, "EX11-012"),
        "CONTROL FAILED: Kaguyamon kept its timing (it stayed in the trash), so \
         at least one of its triggers had to resolve -- <Fortitude> playing it \
         back to P1's field, or [On Deletion] bottom-decking Medusamon. Neither \
         happened, which means the negative miss-timing tests above are passing \
         for some unrelated reason and prove nothing."
    );
}
