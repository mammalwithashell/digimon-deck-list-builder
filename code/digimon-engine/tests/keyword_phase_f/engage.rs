//! EX12 keyword behavior: `<Engage>`.
//!
//! Comprehensive Rules Ver. 4.3 §16-44: "<Engage> is a keyword effect where a
//! Digimon with this effect may attack at the end of your turn" (§16-44-1);
//! a trigger-type effect at the End of Your Turn timing (§16-44-2); optional
//! (§16-44-3). Unlike Execute, Engage does not say it can attack unsuspended
//! Digimon, does not self-delete, and does not bypass normal attack legality.
//! Its processing is its own decision inside the `[End of Your Turn]` pool:
//! one optional prompt over the carrier's legal attack declarations (PASS =
//! decline), not a post-drain `EndOfTurnAction` window
//! (G-ENGINE-ENGAGE-ATTACK-AFTER-END-OF-TURN-TRIGGERS).

use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{GamePhase, Keyword, ModifierType};

use super::helpers::plain_digimon;

fn engage_card(id: &str, dp: i32) -> CardData {
    let mut c = plain_digimon(id);
    c.dp = Some(dp);
    c.keywords = vec![Keyword::Engage];
    c
}

#[test]
fn engage_offers_normal_end_of_turn_attack_against_security() {
    let mut r = DebugRunner::builder()
        .add_card(engage_card("ENGAGE", 5000))
        .start();

    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    assert_eq!(r.game.turn_player(), 0);

    r.game.end_turn();

    assert!(
        !r.game.modifiers.has(engage, ModifierType::MayAttack),
        "§16-44-2: Engage offers its attack itself, it grants no MayAttack window"
    );
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
    let sel = r
        .pending_selection()
        .expect("§16-44-2: Engage's attack decision inside the end-of-turn pool");
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional, "§16-44-3: optional");

    let mask = digimon_engine::build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_attack(engage.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "the security attack is offered"
    );
    assert_eq!(mask[PASS as usize], 1.0, "PASS declines Engage");
}

#[test]
fn engage_does_not_allow_unsuspended_digimon_targets() {
    let mut r = DebugRunner::builder()
        .add_card(engage_card("ENGAGE", 5000))
        .add_card({
            let mut c = plain_digimon("DEF");
            c.dp = Some(2000);
            c
        })
        .start();

    let engage = r.place_on_field(0, "ENGAGE", Some(0));
    let defender = r.place_on_field(1, "DEF", Some(0));
    assert!(
        !r.game.players[1].battle_area[0].is_suspended,
        "defender must be unsuspended for Engage's target restriction test"
    );

    r.game.end_turn();
    assert!(r.pending_selection().is_some(), "Engage's decision");

    let mask = digimon_engine::build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_attack(engage.index as u16, defender.index as u16) as usize],
        0.0,
        "Engage must use normal attack targeting and not attack unsuspended Digimon"
    );
    assert_eq!(
        mask[encode_attack(engage.index as u16, SECURITY_TARGET) as usize],
        1.0
    );
}

#[test]
fn aura_granted_engage_offers_normal_end_of_turn_attack() {
    let yaml = r#"
card: TEST-ENGAGE-AURA-SRC
name: Engage Aura Source
kind: digimon
color: [red]
level: 4
cost: 4
dp: 3000
traits: []
effects:
  - kind: aura
    target: { owner: you, kind: digimon, trait: ME }
    grant_keyword: { keyword: Engage }
"#;

    let mut carrier = plain_digimon("AURA-ENGAGE-CARRIER");
    carrier.traits.push("ME".to_string());
    carrier.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("register Engage aura DSL source")
        .add_card(carrier)
        .start();

    r.place_on_field(0, "TEST-ENGAGE-AURA-SRC", Some(0));
    let engage = r.place_on_field(0, "AURA-ENGAGE-CARRIER", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(engage, Keyword::Engage),
        "aura should grant Engage to the ME carrier"
    );

    r.game.end_turn();

    assert!(
        r.pending_selection().is_some(),
        "aura-granted Engage triggers at the end of the turn and offers its attack"
    );
    let mask = digimon_engine::build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_attack(engage.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "the security attack is offered"
    );
    assert_eq!(mask[PASS as usize], 1.0);
}
