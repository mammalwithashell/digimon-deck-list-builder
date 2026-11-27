//! BT10-092 Nene Amano — behavioral tests.
//!
//! Printed:
//!   [On Play] Reveal the top 4 cards of your deck. Add 1 card with
//!     [Knightmon] or [DeadlyAxemon] in its name or the [Twilight] trait among
//!     them to your hand. Place the rest at the bottom of your deck in any order.
//!   [Opponent's Turn] All of your Digimon with [DarkKnightmon] in their names
//!     or the [Twilight] trait gain <Blocker>.
//!   [On Deletion] Gain 2 memory.
//!   [Security] Play this card without paying its memory cost.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn bt10_092_grants_blocker_on_opponents_turn_and_gains_2_memory_on_deletion() {
    let mut twilight = make_test_card("BT10-092-TW", "SkullKnightmon");
    twilight.card_kind = CardKind::Digimon;
    twilight.level = Some(4);
    twilight.colors = vec![CardColor::Black];
    twilight.dp = Some(5000);
    twilight.traits = vec!["Twilight".to_string()];

    let mut filler = make_test_card("BT10-092-FILL", "Filler");
    filler.level = Some(3);

    let mut r = DebugRunner::builder()
        .dsl_card("BT10-092")
        .unwrap()
        .add_card(twilight)
        .add_card(filler)
        .deck(0, &["BT10-092-FILL", "BT10-092-FILL"])
        .deck(1, &["BT10-092-FILL", "BT10-092-FILL"])
        .start();
    r.skip_mulligan();

    let nene = r.place_on_field(0, "BT10-092", Some(0));
    let sk = r.place_on_field(0, "BT10-092-TW", Some(0));

    // Not opponent's turn yet -> no Blocker from Nene.
    assert!(!r.game.has_keyword(sk, Keyword::Blocker));

    // Switch to opponent's turn -> Twilight Digimon gains Blocker.
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(
        r.game.has_keyword(sk, Keyword::Blocker),
        "BT10-092 must grant <Blocker> to [Twilight] Digimon on opponent's turn"
    );

    // Back to player 0's turn; deleting Nene gains 2 memory for player 0.
    r.game.turn_player_idx = 0;
    r.game.memory = 0;
    let _ = r
        .game
        .delete_permanent_with_cause(nene, ReplacementCause::OpponentEffect);
    assert_eq!(
        r.game.memory, 2,
        "BT10-092 [On Deletion] must gain 2 memory"
    );
}
