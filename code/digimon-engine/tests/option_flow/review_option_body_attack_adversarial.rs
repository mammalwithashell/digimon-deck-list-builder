//! Adversarial lines for Options used during an attack that an Option's own
//! `[Main]` declared (G-ENGINE-OPTION-BODY-ATTACK-NOT-HELD, resolved by
//! reversal).
//!
//! Monarchlizamon (BT25-057) Option face "Final Judgment": "[Main] 1 of your
//! Digimon gains <Rush>, <Security A. +1> and +5000 DP until your opponent's
//! turn ends. Then, it may attack." The Option finishes (its <Arts Digivolve>
//! offer, or the trash) as soon as the attack has been DECLARED, before the
//! attack's [When Attacking] effects and the counter timing, as DCGO does
//! (AttackProcess.cs:242, CardController.cs:2026-2129,
//! TurnStateMachine.cs:1277-1286; Q4962, Q4346). The project owner's default
//! is "Generally let's look to match dcgo"; applying it here is the
//! orchestrator's call. These lines were written against the former hold
//! (`Game::held_options`, removed) and are re-expressed for that order: an
//! inner Option that removes the attacker, two [When Attacking] uses in one
//! attack, an Option used by the DEFENDING player when the attack target
//! changes (the shape of Wolvermon (BT26-053)), a blocked attack, the outer
//! Option used from the trash by another card's effect, the game ending
//! during the attack, and a clone taken at every decision of one line.

use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const MONARCH: &str = "BT25-057";

const ATTACKER_YAML: &str = r#"
card: T-ATK
name: Attacker
kind: digimon
level: 5
color: [green]
cost: 5
dp: 7000
traits: ["Glowing Dawn"]
effects:
  - when: when_attacking
    optional: true
    summary: "[When Attacking] You may use 1 Option card from your hand without paying the cost"
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
          prompt: "Use 1 Option card from your hand"
"#;

/// Two separate [When Attacking] effects, each using 1 Option from hand.
const TWO_USES_ATTACKER_YAML: &str = r#"
card: T-ATK2
name: Double User
kind: digimon
level: 5
color: [green]
cost: 5
dp: 7000
traits: ["Glowing Dawn"]
effects:
  - when: when_attacking
    optional: true
    summary: "[When Attacking] You may use 1 Option card from your hand without paying the cost (A)"
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
          prompt: "Use A"
  - when: when_attacking
    optional: true
    summary: "[When Attacking] You may use 1 Option card from your hand without paying the cost (B)"
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
          prompt: "Use B"
"#;

const PLAIN_OPT_YAML: &str = r#"
card: T-PLAINOPT
name: Plain Option
kind: option
color: [green]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

const PLAIN_OPT2_YAML: &str = r#"
card: T-PLAINOPT2
name: Plain Option 2
kind: option
color: [green]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// "[Main] Delete 1 of your Digimon. Then, 1 of your Digimon gets +1000 DP
/// for the turn." The second pick parks a selection after the attacker is
/// gone, so the inner Option is still in flight when the attack could end.
const DELETE_OWN_OPT_YAML: &str = r#"
card: T-DELOPT
name: Delete Own Option
kind: option
color: [green]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] Delete 1 of your Digimon. Then, 1 of your Digimon gets +1000 DP for the turn"
    process:
      - select_own_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Delete 1 of your Digimon"
      - delete_permanent: { target: victim }
      - select_own_permanent:
          bind_as: buffed
          filter: { kind: digimon }
          prompt: "Give 1 of your Digimon +1000 DP"
      - add_dp_modifier: { target: buffed, value: 1000, expiry: end_of_turn }
"#;

/// "[Main] Return 1 of your Digimon to the hand."
const BOUNCE_OWN_OPT_YAML: &str = r#"
card: T-BOUNCEOPT
name: Bounce Own Option
kind: option
color: [green]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] Return 1 of your Digimon to the hand"
    process:
      - select_own_permanent:
          bind_as: bounced
          filter: { kind: digimon }
          prompt: "Return 1 of your Digimon"
      - return_to_hand: { target: bounced }
"#;

/// A defending <Blocker> whose [All Turns] "when attack targets change" uses
/// 1 Option from its owner's hand (Wolvermon (BT26-053) without its cost).
const DEFENDER_USER_YAML: &str = r#"
card: T-DEFUSER
name: Defending User
kind: digimon
level: 4
color: [green]
cost: 4
dp: 1000
traits: ["Glowing Dawn"]
effects:
  - kind: grant_keyword
    keyword: Blocker
    summary: "<Blocker>"
  - when: on_attack_target_change
    optional: true
    active_when: { all_turns: true }
    summary: "[All Turns] When attack targets change, you may use 1 Option card from your hand without paying the cost"
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: { kind: option }
          prompt: "Defender uses 1 Option card"
"#;

/// "[On Play] You may use 1 Option card from your trash without paying the
/// cost."
const TRASH_USER_YAML: &str = r#"
card: T-TRASHUSER
name: Trash User
kind: digimon
level: 3
color: [green]
cost: 0
dp: 2000
traits: []
effects:
  - when: on_play
    optional: true
    summary: "[On Play] You may use 1 Option card from your trash without paying the cost"
    process:
      - use_option_from_trash:
          of: you
          optional: true
          filter: { kind: option }
          cost: free
          prompt: "Use 1 Option card from your trash"
"#;

fn gd_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Green];
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(MONARCH)
        .expect("BT25-057")
        .from_dsl_yaml(ATTACKER_YAML)
        .expect("attacker")
        .from_dsl_yaml(TWO_USES_ATTACKER_YAML)
        .expect("double user")
        .from_dsl_yaml(PLAIN_OPT_YAML)
        .expect("plain")
        .from_dsl_yaml(PLAIN_OPT2_YAML)
        .expect("plain 2")
        .from_dsl_yaml(DELETE_OWN_OPT_YAML)
        .expect("delete own")
        .from_dsl_yaml(BOUNCE_OWN_OPT_YAML)
        .expect("bounce own")
        .from_dsl_yaml(DEFENDER_USER_YAML)
        .expect("defender user")
        .from_dsl_yaml(TRASH_USER_YAML)
        .expect("trash user")
        .add_card(make_test_card("SEC", "Sec"))
        .add_card(make_test_card("FILL", "Fill"))
        .add_card(gd_base("GD-BASE"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
}

#[derive(Clone, Copy, Debug)]
enum Pick {
    First,
    Pass,
    Id(u16),
}

/// Answer parked selections by prompt-substring rules (each spent once);
/// unmatched optional prompts are declined, unmatched mandatory ones take
/// their first action. Returns `(prompt log, actions taken)`.
fn drive(
    r: &mut DebugRunner,
    rules: &mut Vec<(&str, Pick)>,
    mut stop: impl FnMut(&DebugRunner) -> bool,
) -> (Vec<String>, Vec<(PlayerId, u16)>) {
    let mut seen = Vec::new();
    let mut taken = Vec::new();
    for _ in 0..80 {
        if stop(r) {
            break;
        }
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        let actor = sel.selecting_player;
        let ids = sel.valid_action_ids.clone();
        let prompt = sel.prompt.clone();
        seen.push(format!("p{actor} {:?} {prompt:?} {ids:?}", sel.kind));
        let rule = rules.iter().position(|(needle, _)| prompt.contains(needle));
        let pick = match rule {
            Some(i) => rules.remove(i).1,
            None if sel.is_optional => Pick::Pass,
            None => Pick::First,
        };
        let action = match pick {
            Pick::First => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
            Pick::Pass => PASS,
            Pick::Id(a) => a,
        };
        taken.push((actor, action));
        r.execute_action(actor, action)
            .unwrap_or_else(|e| panic!("answer {action} to {prompt:?}: {e:?}\nseen: {seen:#?}"));
    }
    (seen, taken)
}

fn count_in(cards: &[CardSource], r: &DebugRunner, id: &str) -> usize {
    cards
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == id)
        .count()
}

fn trash_has(r: &DebugRunner, player: PlayerId, id: &str) -> usize {
    count_in(&r.game.player(player).trash, r, id)
}

fn hand_has(r: &DebugRunner, player: PlayerId, id: &str) -> usize {
    count_in(&r.game.player(player).hand, r, id)
}

fn field_has(r: &DebugRunner, player: PlayerId, id: &str) -> usize {
    r.game
        .player(player)
        .battle_area
        .iter()
        .map(|p| {
            // `card_sources` may or may not list the top card; count it once.
            let under = count_in(&p.card_sources, r, id);
            let top = (p.top_card().card_id(&r.game.card_data) == id) as usize;
            if p.card_sources.iter().any(|c| c.handle() == p.top_card().handle()) {
                under
            } else {
                under + top
            }
        })
        .sum()
}

fn use_monarch_and_attack(r: &mut DebugRunner, hand_index: usize, buffed: PermanentHandle) {
    assert_eq!(
        r.game.play_option_from_hand(0, hand_index),
        OptionPlayResult::Pending
    );
    r.execute_action(0, encode_attack(0, buffed.index as u16))
        .expect("pick the Digimon to buff");
    assert_eq!(r.pending_kind(), Some(SelectionKind::Target), "the attack prompt");
    r.execute_action(0, encode_attack(buffed.index as u16, SECURITY_TARGET))
        .expect("attack the player");
}

fn settled(r: &DebugRunner) {
    assert!(r.game.pending_selection.is_none(), "no decision left");
    assert!(r.game.pending_attack.is_none(), "the attack ended");
    assert!(r.game.pending_option.is_none(), "no Option in flight");
}

/// The outer Option is offered <Arts Digivolve> onto GD-BASE at the
/// declaration (declined: trashed). The inner Option then deletes the
/// attacker and parks a second pick while it is still in flight; the outer
/// one is already in the trash and is not touched again. The attack ends with
/// no security check and the inner Option is trashed.
#[test]
fn inner_option_deletes_the_attacker_then_parks_a_pick() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-DELOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);
    let arts = r.game.pending_selection.as_ref().expect("the Arts offer");
    assert!(
        arts.prompt.contains("Arts Digivolve")
            && arts.valid_action_ids.contains(&encode_attack(0, base.index as u16)),
        "the outer Arts offer follows the declaration: {:?}",
        arts.prompt
    );
    assert!(r.game.pending_attack.is_some());
    r.execute_action(0, PASS).expect("decline the outer Arts");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed at the declaration");
    assert!(r.game.pending_option.is_none());

    let mut rules = vec![
        ("Use 1 Option card", Pick::First),
        ("Delete 1 of your Digimon", Pick::Id(encode_attack(0, atk.index as u16))),
    ];
    let (seen, _) = drive(&mut r, &mut rules, |r| {
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|s| s.prompt.contains("+1000 DP"))
    });
    assert!(rules.is_empty(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-ATK"), 1, "the attacker was deleted: {seen:#?}");
    // The inner Option is mid-body; the outer one finished long ago.
    assert!(r.game.pending_option.is_some(), "inner still in flight: {seen:#?}");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer already in the trash");

    let (seen, _) = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(trash_has(&r, 0, "T-DELOPT"), 1, "inner trashed: {seen:#?}");
    assert_eq!(r.security_count(1), 4, "no security check: {seen:#?}");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed once: {seen:#?}");
    assert_eq!(field_has(&r, 0, MONARCH), 0);
    settled(&r);
}

/// The inner Option returns the attacker to hand: the attack ends with no
/// check, both Options end in the trash (the outer one at the declaration).
#[test]
fn inner_option_bounces_the_attacker() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-BOUNCEOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed at the declaration");
    assert!(r.game.pending_option.is_none());
    let mut rules = vec![
        ("Use 1 Option card", Pick::First),
        ("Return 1 of your Digimon", Pick::Id(encode_attack(0, atk.index as u16))),
    ];
    let (seen, _) = drive(&mut r, &mut rules, |_| false);
    assert!(rules.is_empty(), "{seen:#?}");
    assert_eq!(hand_has(&r, 0, "T-ATK"), 1, "attacker bounced: {seen:#?}");
    assert_eq!(r.security_count(1), 4);
    assert_eq!(trash_has(&r, 0, "T-BOUNCEOPT"), 1);
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    settled(&r);
}

/// Two different [When Attacking] effects each use an Option in one attack.
#[test]
fn two_when_attacking_effects_each_use_an_option() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLAINOPT", "T-PLAINOPT2"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK2", Some(0));
    r.game.enter_main_phase();
    let hand_before = r.game.player(0).hand.len();
    use_monarch_and_attack(&mut r, 0, atk);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed at the declaration");
    assert!(r.game.pending_option.is_none());
    let mut rules = vec![("Use A", Pick::First), ("Use B", Pick::First)];
    // Accept both optional activations, whatever their prompts.
    let (seen, _) = drive_accepting(&mut r, &mut rules);
    assert!(rules.is_empty(), "both uses offered: {seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-PLAINOPT"), 1, "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-PLAINOPT2"), 1, "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "two checks");
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert_eq!(r.game.player(0).hand.len(), hand_before - 3 + 2);
    settled(&r);
}

/// Like `drive`, but accepts every optional activation prompt that is not a
/// rule (so both [When Attacking] effects activate) — except Arts Digivolve
/// offers and blocks, which are declined.
fn drive_accepting(
    r: &mut DebugRunner,
    rules: &mut Vec<(&str, Pick)>,
) -> (Vec<String>, Vec<(PlayerId, u16)>) {
    let mut seen = Vec::new();
    let mut taken = Vec::new();
    for _ in 0..80 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        let actor = sel.selecting_player;
        let ids = sel.valid_action_ids.clone();
        let prompt = sel.prompt.clone();
        seen.push(format!("p{actor} {:?} {prompt:?} {ids:?}", sel.kind));
        let rule = rules.iter().position(|(needle, _)| prompt.contains(needle));
        let action = match rule {
            Some(i) => match rules.remove(i).1 {
                Pick::First => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
                Pick::Pass => PASS,
                Pick::Id(a) => a,
            },
            None if actor == 1 || prompt.contains("Arts") || prompt.contains("blocker") => {
                if ids.contains(&PASS) {
                    PASS
                } else {
                    ids[0]
                }
            }
            None => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
        };
        taken.push((actor, action));
        r.execute_action(actor, action)
            .unwrap_or_else(|e| panic!("answer {action} to {prompt:?}: {e:?}\nseen: {seen:#?}"));
    }
    (seen, taken)
}

/// The defending player blocks; its "when attack targets change" uses an
/// Option from ITS hand during the attack. The attacker (GD-BASE, a green
/// Lv.4 [Glowing Dawn] Digimon) is the outer Option's only <Arts Digivolve>
/// host; that offer comes at the declaration and is declined, so the outer
/// Option is already in the trash when the defender uses its Option, which
/// resolves for the defender, and the battle happens.
#[test]
fn defender_uses_an_option_when_the_target_changes_during_the_attack() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH])
        .hand(1, &["T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "GD-BASE", Some(0));
    r.place_on_field(1, "T-DEFUSER", None);
    r.game.enter_main_phase();
    let opp_hand = r.game.player(1).hand.len();
    use_monarch_and_attack(&mut r, 0, atk);
    let arts = r.game.pending_selection.as_ref().expect("the Arts offer");
    assert!(arts.prompt.contains("Arts Digivolve"), "{:?}", arts.prompt);
    assert!(r.game.pending_attack.is_some());
    r.execute_action(0, PASS).expect("decline the Arts Digivolve");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed at the declaration");
    let mut rules = vec![
        ("Declare a blocker", Pick::First),
        ("Defender uses 1 Option card", Pick::First),
    ];
    let (seen, _) = drive_accepting(&mut r, &mut rules);
    assert!(rules.is_empty(), "block + defender use offered: {seen:#?}");
    assert_eq!(trash_has(&r, 1, "T-PLAINOPT"), 1, "defender's Option trashed: {seen:#?}");
    assert_eq!(r.game.player(1).hand.len(), opp_hand - 1 + 1, "defender drew 1");
    assert_eq!(trash_has(&r, 1, "T-DEFUSER"), 1, "the blocker lost the battle");
    assert_eq!(r.security_count(1), 4, "blocked: no check");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "{seen:#?}");
    assert_eq!(field_has(&r, 0, MONARCH), 0, "{seen:#?}");
    settled(&r);
}

/// The outer Option is used from the TRASH by another card's [On Play] (an
/// effect-driven use, so its body runs inside that effect's resolution);
/// its attack uses a further Option from hand. The outer Option goes back to
/// the trash once, at the declaration, before the [When Attacking] use.
#[test]
fn outer_option_used_from_trash_by_an_effect() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &["T-TRASHUSER", "T-PLAINOPT"])
        .memory(8)
        .start();
    r.inject_trash(0, MONARCH);
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    let _ = r.play(0, 0);
    let mut rules = vec![
        ("Use 1 Option card from your trash", Pick::First),
        ("Choose 1 of your Digimon to gain", Pick::Id(encode_attack(0, atk.index as u16))),
        ("Use 1 Option card", Pick::First),
    ];
    let (seen, _) = drive_accepting_with_attack(&mut r, &mut rules, atk);
    assert!(rules.is_empty(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-PLAINOPT"), 1, "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "two checks: {seen:#?}");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "back in the trash once: {seen:#?}");
    settled(&r);
}

/// `drive_accepting`, answering the effect's attack-target prompt with an
/// attack on the player.
fn drive_accepting_with_attack(
    r: &mut DebugRunner,
    rules: &mut Vec<(&str, Pick)>,
    atk: PermanentHandle,
) -> (Vec<String>, Vec<(PlayerId, u16)>) {
    let mut all = (Vec::new(), Vec::new());
    for _ in 0..10 {
        if r.pending_kind() == Some(SelectionKind::Target)
            && r.game
                .pending_selection
                .as_ref()
                .is_some_and(|s| s.valid_action_ids.contains(&encode_attack(atk.index as u16, SECURITY_TARGET)))
        {
            all.1.push((0, encode_attack(atk.index as u16, SECURITY_TARGET)));
            r.execute_action(0, encode_attack(atk.index as u16, SECURITY_TARGET))
                .expect("attack the player");
            continue;
        }
        let (s, t) = drive_accepting_until_target(r, rules);
        let progressed = !t.is_empty();
        all.0.extend(s);
        all.1.extend(t);
        if !progressed {
            break;
        }
    }
    all
}

fn drive_accepting_until_target(
    r: &mut DebugRunner,
    rules: &mut Vec<(&str, Pick)>,
) -> (Vec<String>, Vec<(PlayerId, u16)>) {
    let mut seen = Vec::new();
    let mut taken = Vec::new();
    for _ in 0..80 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        if sel.kind == SelectionKind::Target {
            break;
        }
        let actor = sel.selecting_player;
        let ids = sel.valid_action_ids.clone();
        let prompt = sel.prompt.clone();
        seen.push(format!("p{actor} {:?} {prompt:?} {ids:?}", sel.kind));
        let rule = rules.iter().position(|(needle, _)| prompt.contains(needle));
        let action = match rule {
            Some(i) => match rules.remove(i).1 {
                Pick::First => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
                Pick::Pass => PASS,
                Pick::Id(a) => a,
            },
            None if actor == 1 || prompt.contains("Arts") || prompt.contains("blocker") => {
                if ids.contains(&PASS) {
                    PASS
                } else {
                    ids[0]
                }
            }
            None => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
        };
        taken.push((actor, action));
        r.execute_action(actor, action)
            .unwrap_or_else(|e| panic!("answer {action} to {prompt:?}: {e:?}\nseen: {seen:#?}"));
    }
    (seen, taken)
}

/// The game ends during the attack (the defender has no security left): no
/// panic, and the outer card is accounted for exactly once -- in the trash,
/// where it went at the declaration.
#[test]
fn game_ends_during_the_attack() {
    let mut r = builder()
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed at the declaration");
    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let (seen, _) = drive(&mut r, &mut rules, |_| false);
    assert!(r.game_over(), "direct attack on 0 security wins: {seen:#?}");
    assert_eq!(r.winner(), Some(0));
    assert!(r.game.pending_option.is_none());
    assert_eq!(
        trash_has(&r, 0, MONARCH) + field_has(&r, 0, MONARCH),
        1,
        "the outer card is somewhere exactly once"
    );
    // A clone of the finished game is exact.
    let c = r.game.clone();
    assert_eq!(c.verification_digest(), r.game.verification_digest());
}

/// Clone the game at EVERY decision of the reproduction line (plus a
/// blocker-free inner use and the outer Arts offer), finish each clone with
/// the original's remaining decisions, and compare its end state with the
/// original's.
#[test]
fn clone_at_every_decision_matches_the_original() {
    let mut r = builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-DELOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    let atk2 = r.place_on_field(0, "T-ATK", Some(2));
    r.game.enter_main_phase();

    // Script: use the outer Option from hand, buff `atk`, attack; accept the
    // outer Arts onto `base` (offered at the declaration, before the
    // [When Attacking]); the [When Attacking] uses T-DELOPT, which deletes
    // `atk2` (not the attacker) and buffs `base`.
    let mut snapshots: Vec<digimon_engine::game::Game> = Vec::new();
    let mut actions: Vec<(PlayerId, u16)> = Vec::new();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    let mut rules = vec![
        ("Choose 1 of your Digimon to gain", Pick::Id(encode_attack(0, atk.index as u16))),
        ("Use 1 Option card", Pick::First),
        ("Delete 1 of your Digimon", Pick::Id(encode_attack(0, atk2.index as u16))),
        ("+1000 DP", Pick::Id(encode_attack(0, base.index as u16))),
    ];
    for _ in 0..80 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        snapshots.push(r.game.clone());
        let actor = sel.selecting_player;
        let ids = sel.valid_action_ids.clone();
        let prompt = sel.prompt.clone();
        let action = if sel.kind == SelectionKind::Target {
            encode_attack(atk.index as u16, SECURITY_TARGET)
        } else if sel.kind == SelectionKind::OwnField
            && sel.is_optional
            && prompt.contains("Arts Digivolve")
            && ids.contains(&encode_attack(0, base.index as u16))
            && r.game.pending_option.as_ref().is_some_and(|p| {
                p.card.card_id(&r.game.card_data) == MONARCH
            })
        {
            encode_attack(0, base.index as u16)
        } else if let Some(i) = rules.iter().position(|(n, _)| prompt.contains(n)) {
            match rules.remove(i).1 {
                Pick::Id(a) => a,
                Pick::First => ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS),
                Pick::Pass => PASS,
            }
        } else if sel.kind == SelectionKind::TriggerOrder {
            // Order the simultaneous triggers (the Arts Digivolve's [When
            // Digivolving] with the attacker's [When Attacking]); PASS here
            // would decline every optional one of them.
            ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS)
        } else if sel.is_optional {
            PASS
        } else {
            ids[0]
        };
        actions.push((actor, action));
        r.execute_action(actor, action)
            .unwrap_or_else(|e| panic!("answer {action} to {prompt:?}: {e:?}"));
    }
    assert!(rules.is_empty(), "every scripted prompt was seen: {rules:?}");
    assert!(snapshots.len() >= 5, "a multi-decision line: {}", snapshots.len());
    assert_eq!(top_id(&r, base), MONARCH, "outer Arts-digivolved onto GD-BASE");
    assert_eq!(trash_has(&r, 0, "T-DELOPT"), 1);
    assert_eq!(trash_has(&r, 0, "T-ATK"), 1, "atk2 deleted");
    assert_eq!(r.security_count(1), 2);
    settled(&r);
    let end = r.game.verification_digest();

    for (k, snap) in snapshots.into_iter().enumerate() {
        let mut c = DebugRunner::wrap(snap);
        for &(actor, action) in &actions[k..] {
            c.execute_action(actor, action)
                .unwrap_or_else(|e| panic!("clone {k}: {action}: {e:?}"));
        }
        assert!(c.game.pending_selection.is_none(), "clone {k} settled");
        assert_eq!(c.game.verification_digest(), end, "clone taken at decision {k}");
        assert!(c.game.pending_option.is_none());
    }
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.player(h.player).battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}
