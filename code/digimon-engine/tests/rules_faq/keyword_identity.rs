//! Keyword identity — surface R. Ledger rows MP-26, MP-27.
//!
//! MP-26: "I have a Digimon with a keyword effect that activates at a specific
//! time — e.g. [On Play]. Can I choose it with an effect that specifies Digimon
//! with that keyword, even outside its activation timing? Yes. The Digimon has
//! the keyword effect, even if it isn't currently being activated."
//!
//! MP-27: "I have a Digimon that gains a keyword effect when certain conditions
//! are met... If I have an effect that specifies Digimon with <Blocker>, does it
//! count this Digimon even when [the condition is not met]? No. Digimon that gain
//! keywords under certain conditions or from an effect are only considered to
//! have that keyword when in play in the Battle Area, and when the condition is
//! being met."
//!
//! Vehicles: BT21-026 WarGreymon (unconditional self-granted <Rush>/<Blocker>);
//! BT24-041 Minervamon (declarative aura granting <Reboot>/<Blocker> to your
//! [Iliad] Digimon ONLY during the opponent's turn).

#![allow(unused_imports)]

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Keyword, PlayerId};

fn opponent(p: PlayerId) -> PlayerId {
    if p == 0 {
        1
    } else {
        0
    }
}

/// MP-26 — a Digimon's unconditional self-granted keyword (`<Rush>`/`<Blocker>`
/// on BT21-026, effect-granted not printed) is present on the permanent, so a
/// keyword-specifying effect can see it regardless of activation timing.
#[test]
fn mp26_unconditional_granted_keyword_is_present() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT21-026")
        .expect("BT21-026 (WarGreymon) in embedded DSL pack")
        .start();
    let p = r.turn_player();
    let h = r.place_on_field(p, "BT21-026", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(h, Keyword::Rush),
        "FAQ MP-26: BT21-026's unconditional <Rush> must be present on the permanent"
    );
    assert!(
        r.game.has_keyword(h, Keyword::Blocker),
        "FAQ MP-26: BT21-026's unconditional <Blocker> must be present on the permanent"
    );
}

/// MP-27 — a conditionally-gained keyword is present ONLY while the condition
/// holds. BT24-041 grants <Reboot>/<Blocker> to your [Iliad] Digimon only during
/// the opponent's turn: absent on your turn, present on the opponent's turn.
#[test]
fn mp27_conditional_keyword_only_while_condition_holds() {
    let mut r = DebugRunner::builder()
        .dsl_card("BT24-041")
        .expect("BT24-041 (Minervamon) in embedded DSL pack")
        .deck(0, &["BT24-041", "BT24-041", "BT24-041"])
        .deck(1, &["BT24-041", "BT24-041", "BT24-041"])
        .start();
    let p = r.turn_player();
    let h = r.place_on_field(p, "BT24-041", Some(0)); // Minervamon has [Iliad]
    r.game.tick_declarative_effects();

    // On the controller's OWN turn the "[Opponent's Turn]" aura is inactive →
    // the conditionally-gained keyword is NOT present.
    assert!(
        !r.game.has_keyword(h, Keyword::Reboot),
        "FAQ MP-27: a keyword gained only '[Opponent's Turn]' is NOT present on your own turn"
    );
    assert!(
        !r.game.has_keyword(h, Keyword::Blocker),
        "FAQ MP-27: the conditionally-gained <Blocker> is NOT present when the condition is unmet"
    );

    // Advance to the opponent's turn → the condition now holds.
    r.end_turn();
    assert_eq!(r.turn_player(), opponent(p), "now the opponent's turn");
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(h, Keyword::Reboot),
        "FAQ MP-27: the keyword IS present while the condition (opponent's turn) holds"
    );
    assert!(
        r.game.has_keyword(h, Keyword::Blocker),
        "FAQ MP-27: <Blocker> IS present while the condition holds"
    );
}
