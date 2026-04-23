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
    let token_perm = r.game.player(0).battle_area.iter().find(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing from battle_area");
    assert_eq!(token_perm.top_card().card_name(&r.game.card_data), "Petrification Token");
    assert_eq!(token_perm.base_dp(&r.game.card_data), Some(3000));
}

/// Removal-from-game contract: deleting a token empties the battle_area
/// slot WITHOUT growing P0's trash. Contrast with a normal Digimon, which
/// would land in trash.
#[test]
fn token_delete_removes_from_game_not_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-023", "PlayPetrificationToken"))
        .hand(0, &["TEST-023"])
        .memory(5)
        .start();
    r.play(0, 0);
    assert_eq!(r.trash_size(0), 0);

    let token_field_idx = r.game.player(0).battle_area.iter().position(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing");
    r.game.players[0].delete_permanent(token_field_idx);

    // Battle area loses the token; trash does NOT gain it.
    assert_eq!(r.battle_area_size(0), 1, "only the TEST-023 remains");
    assert_eq!(r.trash_size(0), 0, "token removed from game, not trashed");
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
    let token_idx = r.game.player(0).battle_area.iter().position(|p| {
        p.top_card().card_kind(&r.game.card_data) == CardKind::Token
    }).expect("token missing");

    // Use the full deletion path so OnDeletion observers fire.
    let handle = PermanentHandle { player: 0, index: token_idx as u8 };
    r.game.delete_permanent_with_effects(handle);

    assert_eq!(r.security_count(0), sec_before - 1,
        "Petrification OnDeletion trashed top of security");
    assert_eq!(r.trash_size(0), trash_before + 1,
        "the trashed security card landed in trash (token itself removed from game)");
}
