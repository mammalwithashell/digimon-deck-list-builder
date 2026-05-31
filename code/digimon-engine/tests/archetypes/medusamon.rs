//! **Medusamon archetype interaction tests.**
//!
//! Medusamon (EX11-012) is the LIBERATOR Petrification engine: its
//! [When Digivolving]/[End of Attack] clause hands a Petrification Token to the
//! OPPONENT, and its [All Turns] shield — "When this Digimon would leave the
//! battle area, by deleting 1 Token, it doesn't leave." — pays for itself by
//! deleting a Token. Because the only Token the deck produces lives on the
//! opponent's field, the shield is only meaningful if "1 Token" means a Token in
//! EITHER battle area (DCGO `EX11_012.cs`: `CanSelectPermanentCondition(p) =>
//! p.IsToken` with `selectPlayer: card.Owner`).
//!
//! The Petrification Token's own [On Deletion] then trashes the top security of
//! the Token's OWNER (the opponent), so deleting the gifted Token also strips an
//! opponent security card — the deck's whole tempo loop.
//!
//! Regression for gap G-EX11-012-TOKEN-SHIELD-OWN-ONLY: the shield's cost
//! selector used to be `select_own_permanent` (controller's tokens only), so it
//! could never fire against the opponent-owned Petrification Token in real play.

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

/// Count the Tokens currently on `player`'s field.
fn token_count(r: &DebugRunner, player: u8) -> usize {
    r.game
        .player(player)
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&r.game.card_data) == CardKind::Token)
        .count()
}

/// Medusamon (P0) survives a would-leave by deleting the OPPONENT's (P1's)
/// gifted Petrification Token; the Token's On-Deletion strips one of P1's
/// security cards.
#[test]
fn ex11_012_survives_by_deleting_opponents_petrification_token() {
    // P1 holds 2 security cards so we can observe exactly one being trashed by
    // the Petrification Token's On-Deletion handler.
    let mut r = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 in embedded pack")
        .security(1, &["EX11-012", "EX11-012"])
        .memory(20)
        .start();

    let medusa = r.place_on_field(0, "EX11-012", None);

    // Gift a Petrification Token to the OPPONENT (P1), exactly as Medusamon's
    // [End of Attack] clause does (`play_token { controller: opponent }`).
    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.play_token(1, "petrification");
    }
    assert_eq!(
        token_count(&r, 1),
        1,
        "Petrification Token should be on the OPPONENT's field"
    );
    assert_eq!(
        token_count(&r, 0),
        0,
        "controller has no Token of their own — the shield must reach the opponent's"
    );

    let sec_before = r.game.player(1).security.len();
    assert!(sec_before >= 1, "P1 needs security for the token's On-Deletion to trash");

    // Something tries to delete Medusamon. The [All Turns] would-leave shield
    // fires: with the OWN-ONLY selector this found no candidate and Medusamon
    // left; with the any-owner selector it offers the opponent's Token.
    r.game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);

    assert!(
        r.pending_selection().is_some(),
        "would-leave shield must install a Token-delete selection (the opponent's Token is a valid candidate)"
    );

    // Pay the cost: delete the opponent's Petrification Token.
    let (pl, act) = {
        let s = r.pending_selection().unwrap();
        (s.selecting_player, s.valid_action_ids[0])
    };
    assert_eq!(pl, 0, "the controller (Medusamon's owner) pays the shield cost");
    r.execute_action(pl, act)
        .expect("delete the opponent's Petrification Token");

    // Medusamon survived (leave cancelled).
    assert_eq!(
        r.battle_area_size(0),
        1,
        "Medusamon should survive after deleting the opponent's Token"
    );
    let medusa_still = r
        .game
        .player(0)
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EX11-012");
    assert!(medusa_still, "EX11-012 should remain on P0's field");

    // The opponent's Token was consumed.
    assert_eq!(
        token_count(&r, 1),
        0,
        "the opponent's Petrification Token should be deleted as the shield cost"
    );

    // The Token's On-Deletion trashed the Token owner's (P1's) top security.
    assert_eq!(
        r.game.player(1).security.len(),
        sec_before - 1,
        "deleting the Petrification Token trashes one of its owner's (the opponent's) security cards"
    );
}
