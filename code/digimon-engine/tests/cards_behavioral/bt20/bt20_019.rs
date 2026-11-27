//! BT20-019 Jesmon (X Antibody) — behavioral tests.
//!
//! Printed:
//!   <Alliance>
//!   [When Digivolving] If [Jesmon] or [X Antibody] is in this Digimon's
//!     digivolution cards, for the turn, 1 of your Digimon isn't affected by
//!     your opponent's effects. Then, 1 of your Digimon may attack.
//!   [Your Turn] All of your Digimon with [Sistermon] in their names or the
//!     [Royal Knight] trait gain <Piercing> and can also attack your
//!     opponent's unsuspended Digimon.
//!   Inherited: [Your Turn] While this Digimon is [Jesmon GX], all of your
//!     Digimon gain <Piercing> and can also attack your opponent's unsuspended
//!     Digimon.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};

#[test]
fn bt20_019_has_alliance_grants_effect_immunity_and_royal_knight_aura() {
    let mut jesmon_base = make_test_card("BT20-019-BASE", "Jesmon");
    jesmon_base.card_kind = CardKind::Digimon;
    jesmon_base.level = Some(6);
    jesmon_base.colors = vec![CardColor::Red];
    jesmon_base.dp = Some(12000);
    jesmon_base.traits = vec!["Holy Warrior".to_string(), "Royal Knight".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT20-019")
        .unwrap()
        .add_card(jesmon_base)
        .start();
    r.skip_mulligan();
    let jx = r.place_stack(0, &["BT20-019-BASE", "BT20-019"]);
    r.game.tick_declarative_effects();

    assert!(r.game.has_keyword(jx, Keyword::Alliance));
    assert!(
        r.game.has_keyword(jx, Keyword::Piercing),
        "BT20-019 [Your Turn] aura must grant <Piercing> to [Royal Knight] Digimon"
    );
    assert!(
        r.game.modifiers.has(jx, ModifierType::CanAttackUnsuspended),
        "BT20-019 [Your Turn] aura must grant CanAttackUnsuspended"
    );

    // [WD] with [Jesmon] in sources: choose 1 own Digimon for CannotBeAffected, then decline attack.
    r.game.enqueue_triggered(digimon_engine::enums::EffectTiming::WhenDigivolving, digimon_engine::selection::TriggerSource::Permanent(jx));
    r.game.drain_effect_queue();
    let sel_imm = r
        .pending_selection_view()
        .expect("BT20-019 [WD] must prompt for CannotBeAffected target");
    let pick_imm = *sel_imm
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    r.execute_action(0, pick_imm).unwrap();
    assert!(r.game.modifiers.has(jx, ModifierType::CannotBeAffected));

    if r.pending_selection_view().is_some() {
        r.execute_action(0, PASS).unwrap();
    }
}
