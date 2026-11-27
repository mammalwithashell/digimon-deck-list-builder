//! P-154 Maildramon — behavioral tests.
//!
//! Printed:
//!   [All Turns] When another of your Digimon with [Knightmon] in its text
//!     would leave the battle area by an opponent's effect, by placing this
//!     Digimon as that Digimon's bottom digivolution card, it doesn't leave.
//!   Inherited: <Blocker>

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn p_154_replacement_tucks_self_under_knightmon_text_ally_and_inherited_blocker() {
    let mut ally = make_test_card("P-154-ALLY", "LordKnightmon");
    ally.card_kind = CardKind::Digimon;
    ally.level = Some(6);
    ally.colors = vec![CardColor::Black];
    ally.dp = Some(12000);
    ally.effect_text = "[On Play] Play 1 [Knightmon] from trash.".to_string();

    let mut r = DebugRunner::builder()
        .dsl_card("P-154")
        .unwrap()
        .add_card(ally)
        .start();
    r.skip_mulligan();

    let ally_perm = r.place_on_field(0, "P-154-ALLY", Some(0));
    let _mail = r.place_on_field(0, "P-154", Some(0));

    let _ = r.game.delete_permanent_with_cause(ally_perm, ReplacementCause::OpponentEffect);
    let sel = r
        .pending_selection_view()
        .expect("P-154 replacement must trigger when [Knightmon]-text ally would leave");
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Maildramon must tuck under the ally, leaving only the ally in battle area"
    );
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        2,
        "Maildramon must become bottom digivolution card of ally"
    );
    let surviving = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(
        r.game.has_keyword(surviving, Keyword::Blocker),
        "Tucked P-154 must grant inherited <Blocker>"
    );
}
