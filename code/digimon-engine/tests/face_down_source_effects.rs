//! CR 4.3 4-7-9: "A face-down card under another card is considered to be a
//! card without card information, and its information can't be referenced."
//!
//! So no effect of a face-down digivolution card applies: not its inherited
//! static DP, not its inherited keywords (printed or granted), not its
//! inherited triggers. DCGO agrees: `CEntity_EffectController` skips every
//! `cardSource.IsFlipped` source when it gathers a permanent's effects.
//!
//! Logged shape (matrix scan of the base-fixed engine): Marcus Damon & Thomas
//! H. Norstein (ST24-13) places the top card of its deck face down under
//! itself. With ShineGreymon: Burst Mode (BT25-104) on the field, "[Your
//! Turn] All of your [Marcus Damon]s are also treated as 12000 DP Digimon"
//! makes the Tamer a 12000 DP Digimon -- and the engine then added the
//! "[Your Turn] This Digimon gets +2000 DP" inherited effect of every
//! face-down GeoGreymon (ST24-05) / Agumon (ST24-04) under it, reading 14000
//! or 16000.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn filler() -> CardData {
    make_test_card("FILL", "Filler")
}

fn set_face_down(runner: &mut DebugRunner, host: PermanentHandle, source_index: usize) {
    runner.game.players[host.player as usize].battle_area[host.index as usize].card_sources
        [source_index]
        .face_down = true;
}

/// The logged shape: the Tamer treated as a 12000 DP Digimon by
/// ShineGreymon: Burst Mode (BT25-104) carries two face-down cards whose
/// inherited text is "[Your Turn] This Digimon gets +2000 DP" (GeoGreymon
/// ST24-05, Agumon ST24-04). Neither may apply.
#[test]
fn face_down_sources_under_marcus_add_no_dp_when_treated_as_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .dsl_card("ST24-13")
        .expect("ST24-13 in pack")
        .dsl_card("ST24-05")
        .expect("ST24-05 in pack")
        .dsl_card("ST24-04")
        .expect("ST24-04 in pack")
        .add_card(filler())
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    let marcus = runner.place_on_field(0, "ST24-13", Some(0));
    runner.push_source(marcus, "ST24-05");
    runner.push_source(marcus, "ST24-04");
    set_face_down(&mut runner, marcus, 0);
    set_face_down(&mut runner, marcus, 1);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shine));
    runner.game.drain_effect_queue();
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(marcus),
        Some(12000),
        "CR 4-7-9: the face-down GeoGreymon (ST24-05) / Agumon (ST24-04) under \
         Marcus Damon & Thomas H. Norstein (ST24-13) have no card information, so \
         their inherited +2000 DP does not apply"
    );
}

/// Control for the logged shape: the same cards FACE UP under the Tamer do
/// apply (+2000 each), so the assertion above is not vacuous.
#[test]
fn face_up_sources_under_marcus_add_their_inherited_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 in pack")
        .dsl_card("ST24-13")
        .expect("ST24-13 in pack")
        .dsl_card("ST24-05")
        .expect("ST24-05 in pack")
        .dsl_card("ST24-04")
        .expect("ST24-04 in pack")
        .add_card(filler())
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();
    runner.set_first_player(0);

    let shine = runner.place_on_field(0, "BT25-104", Some(0));
    let marcus = runner.place_on_field(0, "ST24-13", Some(0));
    runner.push_source(marcus, "ST24-05");
    runner.push_source(marcus, "ST24-04");

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(shine));
    runner.game.drain_effect_queue();
    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(marcus), Some(16000));
}

/// A synthetic digivolution card carrying every kind of inherited effect the
/// engine gathers from a source: static DP, a printed inherited keyword
/// (`inherited_text`), a granted inherited keyword (`grant_keyword` clause)
/// and an inherited trigger.
const FD_SOURCE_YAML: &str = r#"
card: FDSRC
name: "Face-Down Source Probe"
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
traits: [Dinosaur]
effects:
  - scope: inherited
    kind: aura
    target: {}
    dp_modifier: 3000
    summary: "Inherited: This Digimon gets +3000 DP"
  - scope: inherited
    kind: grant_keyword
    keyword: Blocker
    summary: "Inherited: <Blocker>"
  - scope: inherited
    when: when_attacking
    summary: "Inherited [When Attacking] Gain 1 memory."
    process:
      - gain_memory: 1
"#;

fn host_card() -> CardData {
    let mut c = make_test_card("HOST", "Host Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.colors = vec![CardColor::Red];
    c
}

/// A digivolution card whose PRINTED inherited text carries keywords (the
/// `card_data` keyword path, not a DSL clause).
fn printed_keyword_source() -> CardData {
    let mut c = make_test_card("KWSRC", "Printed Keyword Source");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    // Printed keywords use the full-width brackets of the card text.
    c.inherited_text = "＜Security A. +1＞ ＜Piercing＞".to_string();
    c.keywords = vec![Keyword::SecurityAttackPlus(1), Keyword::Piercing];
    c
}

fn probe_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FD_SOURCE_YAML)
        .expect("probe YAML lowers")
        .add_card(host_card())
        .add_card(printed_keyword_source())
        .add_card(filler())
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(0)
        .start();
    runner.set_first_player(0);
    runner
}

fn fire_when_attacking(runner: &mut DebugRunner, host: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    while runner.pending_selection().is_some() {
        runner.accept_optional_trigger().expect("resolve pending trigger");
        runner.game.drain_effect_queue();
    }
}

/// Face up, the probe's inherited effects all reach the host (control).
#[test]
fn face_up_source_effects_reach_the_host() {
    let mut runner = probe_runner();
    let host = runner.place_on_field(0, "HOST", Some(0));
    runner.push_source(host, "FDSRC");
    runner.push_source(host, "KWSRC");
    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(host), Some(8000));
    assert!(runner.game.has_keyword(host, Keyword::Blocker));
    assert!(runner.game.has_keyword(host, Keyword::Piercing));
    assert_eq!(runner.game.security_attack_keyword_bonus(host), 1);

    let memory = runner.memory();
    fire_when_attacking(&mut runner, host);
    assert_eq!(runner.memory(), memory + 1);
}

/// CR 4-7-9: face down, none of them apply -- DP, granted keyword, printed
/// inherited keywords, Security Attack, trigger.
#[test]
fn face_down_source_effects_do_not_reach_the_host() {
    let mut runner = probe_runner();
    let host = runner.place_on_field(0, "HOST", Some(0));
    runner.push_source(host, "FDSRC");
    runner.push_source(host, "KWSRC");
    set_face_down(&mut runner, host, 0);
    set_face_down(&mut runner, host, 1);
    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(host), Some(5000), "no inherited +3000 DP");
    assert!(
        !runner.game.has_keyword(host, Keyword::Blocker),
        "no inherited granted <Blocker>"
    );
    assert!(
        !runner.game.has_keyword(host, Keyword::Piercing),
        "no printed inherited <Piercing>"
    );
    assert_eq!(
        runner.game.security_attack_keyword_bonus(host),
        0,
        "no printed inherited <Security A. +1>"
    );
    assert_eq!(runner.game.source_dp_contribution(host, 0), 0);

    let memory = runner.memory();
    fire_when_attacking(&mut runner, host);
    assert_eq!(runner.memory(), memory, "no inherited [When Attacking] trigger");

    // The face-down cards are still cards: they count as digivolution cards.
    let stack = &runner.game.players[0].battle_area[host.index as usize].card_sources;
    assert_eq!(stack.len(), 3);
}

/// A card turned face down keeps nothing: flipping the probe face down after
/// its effects applied removes them at the next re-evaluation.
#[test]
fn flipping_a_source_face_down_removes_its_effects() {
    let mut runner = probe_runner();
    let host = runner.place_on_field(0, "HOST", Some(0));
    runner.push_source(host, "FDSRC");
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(host), Some(8000));
    assert!(runner.game.has_keyword(host, Keyword::Blocker));

    set_face_down(&mut runner, host, 0);
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(host), Some(5000));
    assert!(!runner.game.has_keyword(host, Keyword::Blocker));
}
