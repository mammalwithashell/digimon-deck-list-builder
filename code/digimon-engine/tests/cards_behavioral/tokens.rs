//! Behavioral tests for `ctx.play_token` and token-aware
//! `delete_permanent` (remove-from-game instead of trash).
//!
//! Uses TEST-023, a synthetic test card whose OnPlay is
//! `ctx.play_token(player, "petrification")`, so we exercise the full
//! play -> registry -> EffectContext -> mutations path rather than
//! calling `play_token` from a test in isolation.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

/// Happy path: playing TEST-023 materializes a Petrification Token on
/// P0's field. The token reads as `CardKind::Token`, carries the
/// Petrification stats, and takes up a field slot.
#[test]
fn test_023_play_token_spawns_petrification() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5) // pre-fund cost (TEST-023 uses default cost=3)
        .start();

    assert_eq!(r.battle_area_size(0), 0);
    r.play(0, 0);

    // After the play, the TEST-023 permanent itself is on the field AND
    // a Petrification Token sits next to it.
    assert_eq!(r.battle_area_size(0), 2, "test card + token");

    // Find the token by kind.
    let token_perm = r
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing from battle_area");
    assert_eq!(
        token_perm.top_card().card_name(&r.game.card_data),
        "Petrification Token"
    );
    assert_eq!(token_perm.base_dp(&r.game.card_data), Some(3000));
}

/// Removal-from-game contract: deleting a token empties the battle_area
/// slot WITHOUT growing P0's trash. Contrast with a normal Digimon, which
/// would land in trash.
#[test]
fn token_delete_removes_from_game_not_trash() {
    for (card_id, card_name) in [
        ("TEST-023", "PlayPetrificationToken"),
        ("TEST-029", "PlayFamiliarToken"),
    ] {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card(card_id, card_name))
            .hand(0, &[card_id])
            .memory(5)
            .start();
        r.play(0, 0);
        assert_eq!(r.trash_size(0), 0, "{card_name}");

        let token_field_idx = r
            .game
            .player(0)
            .battle_area
            .iter()
            .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
            .expect("token missing");
        r.game.players[0].delete_permanent(token_field_idx);

        // Battle area loses the token; trash does NOT gain it.
        assert_eq!(r.battle_area_size(0), 1, "only the test card remains");
        assert_eq!(r.trash_size(0), 0, "{card_name} removed from game");
    }
}

/// Petrification Token OnDeletion: when the token is deleted via the
/// full effect-firing path, the top card of the token-owner's security
/// stack goes to trash.
#[test]
fn petrification_on_deletion_trashes_top_security() {
    use digimon_engine::permanent::PermanentHandle;

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .add_card(make_test_card("SEC-A", "SecA"))
        .add_card(make_test_card("SEC-B", "SecB"))
        .add_card(make_test_card("SEC-C", "SecC"))
        .hand(0, &["TEST-023"])
        .security(0, &["SEC-A", "SEC-B", "SEC-C"])
        .memory(5)
        .start();
    r.play(0, 0);
    let sec_before = r.security_count(0);
    let trash_before = r.trash_size(0);

    // Locate the token on P0's field.
    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .expect("token missing");

    // Use the full deletion path so OnDeletion observers fire.
    let handle = PermanentHandle {
        player: 0,
        index: token_idx as u8,
    };
    r.game.delete_permanent_with_effects(handle);

    assert_eq!(
        r.security_count(0),
        sec_before - 1,
        "Petrification OnDeletion trashed top of security"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the trashed security card landed in trash (token itself removed from game)"
    );
}

#[test]
fn familiar_on_deletion_prompts_opponent_target_and_applies_minus_3000() {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::{encode_attack, PASS};
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    let mut opp_low = make_test_card("OPP-LOW", "OppLow");
    opp_low.dp = Some(4000);
    let mut opp_high = make_test_card("OPP-HIGH", "OppHigh");
    opp_high.dp = Some(7000);

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-029", "PlayFamiliarToken"))
        .add_card(opp_low)
        .add_card(opp_high)
        .hand(0, &["TEST-029"])
        .memory(5)
        .start();

    r.place_on_field(1, "OPP-LOW", Some(0));
    r.place_on_field(1, "OPP-HIGH", Some(0));
    r.play(0, 0).expect("TEST-029 plays");

    let token_idx = r
        .game
        .player(0)
        .battle_area
        .iter()
        .position(|p| p.top_card().card_name(&r.game.card_data) == "Familiar Token")
        .expect("Familiar Token missing");

    r.game.delete_permanent_with_effects(PermanentHandle {
        player: 0,
        index: token_idx as u8,
    });

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Familiar OnDeletion must ask for an opponent Digimon");
    assert_eq!(pending.kind, SelectionKind::OppField);
    assert_eq!(pending.selecting_player, 0);
    assert_eq!(pending.valid_action_ids.len(), 2);
    assert!(!pending.valid_action_ids.contains(&PASS));

    let target_action = encode_attack(0, 1);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[target_action as usize], 1.0);

    r.game.decode_action(target_action, 0);

    let target = PermanentHandle {
        player: 1,
        index: 1,
    };
    assert_eq!(
        r.game.effective_dp(target),
        Some(4000),
        "OPP-HIGH should be 7000 DP with a -3000 DP until-turn-end modifier"
    );
}
