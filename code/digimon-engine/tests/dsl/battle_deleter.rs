//! G-DSL-BATTLE-DELETER — `event_battle_deleter: { <permanent predicate> }` on
//! an `on_ally_won_battle` (EndOfBattle) observer: "When any of your Digimon
//! with [X] in their texts delete your opponent's Digimon in battle" (EX13-041
//! Groundramon). The deleter is the combatant that SURVIVED while its battle
//! opponent was actually deleted (DCGO fixes `LoserPermanents` to the
//! destroyed ones and reads the winner's real top card).

use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

/// Observer: when any of your Digimon named [Hero] deletes an opponent's
/// Digimon in battle, trash the opponent's top security card.
const OBSERVER: &str = r#"
card: BD-OBS
name: Deleter Observer
kind: digimon
level: 4
color: [red]
cost: 4
dp: 1000
effects:
  - when: on_ally_won_battle
    condition:
      event_battle_deleter:
        owner: you
        kind: digimon
        name_is: Hero
    summary: "When your [Hero] deletes an opponent's Digimon in battle, trash their top security card"
    process:
      - trash_top_security: { of: opponent }
"#;

fn card(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card_with_level(id, name, 5);
    c.colors = vec![CardColor::Red];
    c.dp = Some(dp);
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(OBSERVER)
        .expect("observer compiles")
        .add_card(card("BD-HERO", "Hero", 8000))
        .add_card(card("BD-HERO-TIE", "Hero", 5000))
        .add_card(card("BD-VILLAIN", "Villain", 5000))
        .add_card(card("BD-BRUTE", "Brute", 12000))
        .add_card(card("BD-PAD", "Pad", 1000))
        .security(1, &["BD-PAD"; 4])
        .start()
}

fn protect(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.game.modifiers.add(
        h,
        ModifierEntry::simple(
            ModifierType::CannotBeDestroyedByBattle,
            0,
            Expiry::Permanent,
            h.player,
        ),
    );
}

#[test]
fn own_winner_that_deletes_its_opponent_fires() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "villain deleted in battle");
    assert_eq!(r.security_count(1), 3, "observer trashed 1 security");
}

#[test]
fn prevented_loser_deletion_does_not_fire() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    protect(&mut r, villain);
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 1, "villain survived (protected)");
    assert_eq!(r.security_count(1), 4, "no deletion → no trigger");
}

#[test]
fn deleter_must_match_the_inner_predicate() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let brute = r.place_on_field(0, "BD-BRUTE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(brute, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.security_count(1), 4, "Brute is not [Hero]");
}

#[test]
fn opponent_deleter_does_not_satisfy_owner_you() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let brute = r.place_on_field(1, "BD-BRUTE", Some(0));
    r.battle_digimon(hero, brute);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 4, "the opponent's Digimon was the deleter");
}

#[test]
fn mutual_destruction_has_no_deleter() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.security_count(1), 4, "both deleted: no surviving deleter");
}

#[test]
fn tie_with_a_protected_survivor_counts_as_deleting() {
    let mut r = runner();
    r.place_on_field(0, "BD-OBS", Some(0));
    let hero = r.place_on_field(0, "BD-HERO-TIE", Some(0));
    let villain = r.place_on_field(1, "BD-VILLAIN", Some(0));
    protect(&mut r, hero);
    r.battle_digimon(hero, villain);
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(1), 0, "villain deleted");
    assert_eq!(r.battle_area_size(0), 2, "hero survived the tie");
    assert_eq!(r.security_count(1), 3, "hero deleted its opponent in battle");
}
