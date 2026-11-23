//! An attack declared inside an Option's `[Main]` body holds that Option in
//! flight until the attack ends, and Options used DURING that attack resolve
//! on their own. G-ENGINE-OPTION-BODY-ATTACK-NOT-HELD.
//!
//! Monarchlizamon (BT25-057) Option face "Final Judgment": "[Main] 1 of your
//! Digimon gains <Rush>, <Security A. +1> and +5000 DP for the turn. Then, it
//! may attack." The attack is one of the effect's processes (CR 4.3 15-1-2,
//! 15-1-4; 11-1-5), so the Option is trashed / offered for <Arts Digivolve>
//! only after it (9-1-5, 4-20-1/4-20-2). DCGO BT25_057.cs runs
//! `SelectAttackEffect.Activate()` inside the OptionSkill coroutine, and the
//! attack's own [When Attacking] effects may use further Options there.
//!
//! Commit 32d148445 kept the Option in `Game::pending_option` for the whole
//! attack, and `play_option_core` allows one in-flight Option, so any Option
//! used during that attack panicked ("reentrant Option play while another is
//! mid-resolution"). The engine now moves the Option to `Game::held_options`
//! when the attack is installed and puts it back when the attack ends.

use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const MONARCH: &str = "BT25-057";

// ─── Fixtures ───────────────────────────────────────────────────────────────

/// A Lv.5 [Glowing Dawn] Digimon whose [When Attacking] may use any Option
/// card from hand for free (the shape of EX7-059 / BT26-026 / EX13-014 /
/// BT25-083 without their costs).
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

/// An Option whose `[Main]` places it in the battle area (the
/// `place_self_as_delay_option` shape of Royal Knights of the Purge
/// (BT13-110)).
const PLACED_OPT_YAML: &str = r#"
card: T-PLACEDOPT
name: Placed Option
kind: option
color: [green]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>. Place this card in the battle area"
    process:
      - draw: { of: you, count: 1 }
      - place_self_as_delay_option: {}
  - kind: delay
    trigger: delayed
    summary: "[Main] <Delay> Gain 1 memory"
    process:
      - gain_memory: 1
"#;

/// A plain [Security] Option: "[Security] <Draw 1>".
const SEC_DRAW_OPT_YAML: &str = r#"
card: T-SECDRAW
name: Security Draw Option
kind: option
color: [blue]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
  - scope: inherited
    when: on_security
    summary: "[Security] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// An attacker whose [When Attacking] deletes itself.
const SELF_DELETE_ATTACKER_YAML: &str = r#"
card: T-SELFDEL
name: Self Deleter
kind: digimon
level: 5
color: [green]
cost: 5
dp: 7000
traits: ["Glowing Dawn"]
effects:
  - when: when_attacking
    summary: "[When Attacking] Delete this Digimon"
    process:
      - delete_permanent: { target: this }
"#;

/// An attacker whose [When Attacking] ends the attack (15-15-1).
const END_ATTACK_ATTACKER_YAML: &str = r#"
card: T-ENDATK
name: Attack Ender
kind: digimon
level: 5
color: [green]
cost: 5
dp: 7000
traits: ["Glowing Dawn"]
effects:
  - when: when_attacking
    summary: "[When Attacking] End this attack"
    process:
      - end_attack: true
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

fn plain_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 1;
    c.colors = vec![CardColor::Purple];
    c
}

fn ts_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Black];
    c.traits = vec!["TS".to_string()];
    c
}

fn base_builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(MONARCH)
        .expect("BT25-057")
        .from_dsl_yaml(ATTACKER_YAML)
        .expect("attacker")
        .from_dsl_yaml(PLAIN_OPT_YAML)
        .expect("plain option")
        .from_dsl_yaml(PLACED_OPT_YAML)
        .expect("placed option")
        .add_card(make_test_card("SEC", "Sec"))
        .add_card(make_test_card("FILL", "Fill"))
        .add_card(gd_base("GD-BASE"))
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
}

// ─── Driving helpers ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
enum Pick {
    /// The first offered action that is not PASS.
    First,
    /// Decline (PASS).
    Pass,
    /// This exact action id.
    Id(u16),
}

/// Answer the parked selections. Each `(prompt substring, pick)` rule answers
/// the first prompt whose text contains it and is then spent; any other
/// prompt is declined when optional and otherwise answered with its first
/// action. Stops when no selection is parked, or before answering when
/// `stop` returns true. Returns the prompts seen.
fn drive(
    r: &mut DebugRunner,
    rules: &mut Vec<(&str, Pick)>,
    mut stop: impl FnMut(&DebugRunner) -> bool,
) -> Vec<String> {
    let mut seen = Vec::new();
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
        r.execute_action(actor, action)
            .unwrap_or_else(|e| panic!("answer {action} to {prompt:?}: {e:?}\nseen: {seen:#?}"));
    }
    seen
}

fn in_zone(cards: &[CardSource], r: &DebugRunner, id: &str) -> usize {
    cards
        .iter()
        .filter(|c| c.card_id(&r.game.card_data) == id)
        .count()
}

fn trash_has(r: &DebugRunner, player: PlayerId, id: &str) -> usize {
    in_zone(&r.game.player(player).trash, r, id)
}

fn hand_has(r: &DebugRunner, player: PlayerId, id: &str) -> usize {
    in_zone(&r.game.player(player).hand, r, id)
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.player(h.player).battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn held_ids(r: &DebugRunner) -> Vec<String> {
    r.game
        .held_options
        .iter()
        .map(|h| h.option.card.card_id(&r.game.card_data).to_string())
        .collect()
}

fn arts_on(base: PermanentHandle) -> impl Fn(&DebugRunner) -> bool {
    move |r: &DebugRunner| {
        r.game.pending_selection.as_ref().is_some_and(|s| {
            s.kind == SelectionKind::OwnField
                && s.is_optional
                && s.valid_action_ids
                    .contains(&encode_attack(0, base.index as u16))
        })
    }
}

/// Use Monarchlizamon (BT25-057) from hand index `hand_index`, give `buffed`
/// the buffs and declare its attack on the opponent. Leaves the engine at the
/// first decision of the attack.
fn use_monarch_and_attack(r: &mut DebugRunner, hand_index: usize, buffed: PermanentHandle) {
    assert_eq!(
        r.game.play_option_from_hand(0, hand_index),
        OptionPlayResult::Pending
    );
    r.execute_action(0, encode_attack(0, buffed.index as u16))
        .expect("pick the Digimon to buff");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Target),
        "the effect's attack prompt"
    );
    r.execute_action(0, encode_attack(buffed.index as u16, SECURITY_TARGET))
        .expect("attack the player");
}

// ─── 1. The reviewer's reproduction ─────────────────────────────────────────

/// BT25-057 used from hand on an attacker whose [When Attacking] may use an
/// Option from hand: the attack declared inside the Option's effect opens,
/// the [When Attacking] uses another Option, that Option resolves completely
/// (draw, trash), the attack continues through its two security checks
/// (<Security A. +1>), and only then is BT25-057 trashed (no Lv.4 for its
/// <Arts Digivolve>). Red before the fix: panic "reentrant Option play while
/// another is mid-resolution: player=0 incoming_card=T-PLAINOPT
/// from_source=Hand(0) in_flight_card=BT25-057
/// in_flight_resolution_phase=MainEffectDrain in_counter_window=false".
#[test]
fn option_used_by_when_attacking_during_an_option_body_attack() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    let hand_before = r.game.player(0).hand.len();

    use_monarch_and_attack(&mut r, 0, atk);

    // The [When Attacking] prompt: BT25-057 is held, not in flight and in
    // no zone; the attack is open.
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()]);
    assert!(r.game.pending_option.is_none(), "the slot is free for the inner use");
    assert!(r.game.pending_attack.is_some());
    assert_eq!(trash_has(&r, 0, MONARCH), 0);
    assert_eq!(hand_has(&r, 0, MONARCH), 0);

    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let seen = drive(&mut r, &mut rules, |_| false);
    assert!(rules.is_empty(), "the inner use was offered: {seen:#?}");

    assert_eq!(trash_has(&r, 0, "T-PLAINOPT"), 1, "inner trashed: {seen:#?}");
    assert!(r.game.pending_attack.is_none(), "the attack ended: {seen:#?}");
    assert_eq!(r.security_count(1), 2, "two security checks (<Security A. +1>)");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "BT25-057 trashed after its attack");
    assert!(r.game.held_options.is_empty());
    assert!(r.game.pending_option.is_none(), "no Option left in flight");
    assert!(r.game.pending_selection.is_none(), "{seen:#?}");
    // Both Options left the hand and the inner one drew 1.
    assert_eq!(r.game.player(0).hand.len(), hand_before - 2 + 1);
}

/// Same line with a <Blocker> on the defending side: at the block timing
/// (after the [When Attacking] effects, 11-1-3) the inner Option has fully
/// resolved, the outer one is still held, and no security check has been
/// made yet.
#[test]
fn inner_option_finishes_before_the_outer_attack_checks_security() {
    let mut blocker = make_test_card("OPP-BLK", "Blocker");
    blocker.card_kind = CardKind::Digimon;
    blocker.level = Some(3);
    blocker.dp = Some(1000);
    blocker.keywords = vec![Keyword::Blocker];
    let mut r = base_builder()
        .add_card(blocker)
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.place_on_field(1, "OPP-BLK", None);
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);

    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let defender_decides = |r: &DebugRunner| {
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|s| s.selecting_player == 1)
    };
    let seen = drive(&mut r, &mut rules, defender_decides);
    assert!(defender_decides(&r), "the block timing: {seen:#?}");
    assert!(rules.is_empty(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-PLAINOPT"), 1, "inner resolved: {seen:#?}");
    assert!(r.game.pending_attack.is_some(), "the attack is still open");
    assert_eq!(r.security_count(1), 4, "no security check yet");
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()], "outer still held");
    assert!(r.game.pending_option.is_none());
    assert_eq!(trash_has(&r, 0, MONARCH), 0);

    // No block: the attack checks twice, then the outer Option is trashed.
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(r.security_count(1), 2, "{seen:#?}");
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert!(r.game.held_options.is_empty());
}

// ─── 4. Inner Options that are placed / Arts-digivolve ──────────────────────

/// The inner Option places itself in the battle area: it lands there (not in
/// the trash), the attack continues, and the outer Option is offered for
/// <Arts Digivolve> after the attack (GD-BASE is a green Lv.4) and accepted.
#[test]
fn inner_placed_option_during_the_outer_attack_then_outer_arts() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLACEDOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);

    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let seen = drive(&mut r, &mut rules, arts_on(base));
    assert!(arts_on(base)(&r), "the outer Arts Digivolve offer: {seen:#?}");
    assert!(rules.is_empty(), "{seen:#?}");
    assert!(r.game.pending_attack.is_none(), "Arts is offered after the attack");
    assert_eq!(r.security_count(1), 2, "both checks happened first");
    let placed = r
        .game
        .player(0)
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&r.game.card_data) == "T-PLACEDOPT")
        .count();
    assert_eq!(placed, 1, "the inner Option was placed in the battle area");
    assert_eq!(trash_has(&r, 0, "T-PLACEDOPT"), 0);
    assert!(r.game.held_options.is_empty(), "released for its Arts offer");

    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept the outer Arts Digivolve");
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(top_id(&r, base), MONARCH, "outer Arts Digivolved onto GD-BASE");
    assert_eq!(trash_has(&r, 0, MONARCH), 0);
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

// ─── 2. Real pool attackers ─────────────────────────────────────────────────

fn cougarmon_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = base_builder()
        .dsl_card("BT26-026")
        .expect("BT26-026")
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, MONARCH])
        .memory(8)
        .start();
    let cougar = r.place_on_field(0, "BT26-026", Some(0));
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    r.game.enter_main_phase();
    (r, cougar, base)
}

/// Cougarmon (BT26-026) [When Attacking][Once Per Turn]: "By trashing ...
/// your top security card, you may use 1 Option card with the [Glowing Dawn]
/// trait from your hand with the cost reduced by 2." Used during the attack
/// that Monarchlizamon (BT25-057)'s "Final Judgment" declared, on a second
/// copy of Final Judgment. The inner copy buffs Cougarmon again, offers no
/// attack (11-2-4: an attack is in progress), is declined for <Arts
/// Digivolve> and trashed; the outer attack then checks security three times
/// (<Security A. +1> from each copy) and the outer copy Arts-digivolves.
#[test]
fn cougarmon_uses_a_second_final_judgment_during_the_first_ones_attack() {
    let (mut r, cougar, base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    let memory_after_outer = r.game.memory;
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()]);

    let mut rules = vec![
        ("Will you pay the cost", Pick::First),
        ("with the cost reduced by 2", Pick::First),
        (
            "Choose 1 of your Digimon to gain",
            Pick::Id(encode_attack(0, cougar.index as u16)),
        ),
    ];
    // Stop at the inner copy's Arts offer (it is in flight; outer held).
    let seen = drive(&mut r, &mut rules, |r| {
        r.game.pending_option.is_some() && arts_on(base)(r)
    });
    assert!(arts_on(base)(&r), "inner Arts offer: {seen:#?}");
    assert!(rules.is_empty(), "{seen:#?}");
    assert!(r.game.pending_attack.is_some(), "still inside the outer attack");
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()]);
    assert_eq!(r.security_count(0), 1, "Cougarmon paid with its top security card");
    assert_eq!(r.game.memory, memory_after_outer - 2, "Use 4 reduced by 2");
    r.execute_action(0, PASS)
        .expect("decline the inner Arts Digivolve");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "the inner copy is trashed");
    // (No further decision belongs to the attack here, so it runs to its end
    // within this action and the outer copy is released for its own offer.)

    let seen = drive(&mut r, &mut Vec::new(), arts_on(base));
    assert!(arts_on(base)(&r), "outer Arts offer after the attack: {seen:#?}");
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.security_count(1), 2, "3 checks: <Security A. +1> from each copy");
    assert!(r.game.held_options.is_empty());
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept the outer Arts Digivolve");
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(top_id(&r, base), MONARCH);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "only the inner copy is in the trash");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// Same line with the inner copy ACCEPTING its <Arts Digivolve> onto the
/// attacking Cougarmon mid-attack (4-20-1): Cougarmon becomes Monarchlizamon,
/// keeps attacking, and the outer copy is still offered its own Arts
/// Digivolve after the attack (onto GD-BASE).
#[test]
fn inner_final_judgment_arts_digivolves_the_attacker_mid_attack() {
    let (mut r, cougar, base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    let mut rules = vec![
        ("Will you pay the cost", Pick::First),
        ("with the cost reduced by 2", Pick::First),
        (
            "Choose 1 of your Digimon to gain",
            Pick::Id(encode_attack(0, cougar.index as u16)),
        ),
    ];
    let seen = drive(&mut r, &mut rules, |r| {
        r.game.pending_option.is_some() && arts_on(base)(r)
    });
    assert!(arts_on(base)(&r), "inner Arts offer: {seen:#?}");
    r.execute_action(0, encode_attack(0, cougar.index as u16))
        .expect("inner Arts Digivolve onto the attacker");
    assert_eq!(top_id(&r, cougar), MONARCH, "the attacker Arts-digivolved");

    let seen = drive(&mut r, &mut Vec::new(), arts_on(base));
    assert!(arts_on(base)(&r), "outer Arts offer after the attack: {seen:#?}");
    assert_eq!(
        r.security_count(1),
        2,
        "the attack kept checking after the Arts Digivolve: {seen:#?}"
    );
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept the outer Arts Digivolve");
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(top_id(&r, base), MONARCH);
    assert_eq!(trash_has(&r, 0, MONARCH), 0, "both copies Arts-digivolved");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// BeelStarmon ACE (EX7-059) [When Attacking][Once Per Turn]: "By trashing 1
/// Option card from this Digimon's digivolution cards, you may use 1 [Three
/// Musketeers] trait Option card from your hand without paying the cost."
/// The inner Option is Hurricane Screw Shot (EX7-071), whose [Main] places it
/// as the bottom digivolution card of a [Three Musketeers] Digimon.
#[test]
fn beelstarmon_ace_uses_hurricane_screw_shot_during_final_judgments_attack() {
    let mut r = base_builder()
        .dsl_card("EX7-059")
        .expect("EX7-059")
        .dsl_card("EX7-071")
        .expect("EX7-071")
        .add_card(plain_option("T-SRCOPT"))
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "EX7-071"])
        .memory(8)
        .start();
    let beel = r.place_on_field(0, "EX7-059", Some(0));
    r.push_source(beel, "T-SRCOPT");
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, beel);
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()]);

    let mut rules = vec![
        ("Trash 1 Option card from this Digimon", Pick::First),
        ("[Three Musketeers] trait Option card from your hand", Pick::First),
        (
            "Place this card as the bottom digivolution card",
            Pick::Id(encode_attack(0, beel.index as u16)),
        ),
    ];
    let seen = drive(&mut r, &mut rules, arts_on(base));
    assert!(rules.is_empty(), "every inner prompt was offered: {seen:#?}");
    assert!(arts_on(base)(&r), "outer Arts offer after the attack: {seen:#?}");
    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.security_count(1), 2, "two checks (<Security A. +1>)");
    assert_eq!(trash_has(&r, 0, "T-SRCOPT"), 1, "the cost was paid");
    let beel_sources = &r.game.player(0).battle_area[beel.index as usize].card_sources;
    assert_eq!(
        in_zone(beel_sources, &r, "EX7-071"),
        1,
        "Hurricane Screw Shot was placed under BeelStarmon ACE"
    );
    assert_eq!(trash_has(&r, 0, "EX7-071"), 0);
    r.execute_action(0, PASS)
        .expect("decline the outer Arts Digivolve");
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "outer trashed");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

// ─── 3. Security Options checked during the held attack ─────────────────────

/// "[Security] Activate this card's [Main] effect" on a Plug-In Option, Iron
/// Slash (BT25-100): its link step must link IRON SLASH to its owner's [TS]
/// Digimon. While Final Judgment sat in `pending_option` the link step read
/// it as the Option being linked.
#[test]
fn security_iron_slash_links_itself_during_final_judgments_attack() {
    let mut r = base_builder()
        .dsl_card("BT25-100")
        .expect("BT25-100")
        .add_card(ts_digimon("OPP-TS"))
        .security(1, &["SEC", "SEC", "SEC", "BT25-100"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    let opp_ts = r.place_on_field(1, "OPP-TS", None);
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);

    let is_link = |r: &DebugRunner| {
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|s| s.selecting_player == 1 && s.prompt.contains("link this Option"))
    };
    let seen = drive(&mut r, &mut Vec::new(), is_link);
    assert!(is_link(&r), "Iron Slash's owner is offered the link: {seen:#?}");
    let link_pick = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.execute_action(1, link_pick).expect("link Iron Slash");
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    let linked = &r.game.player(1).battle_area[opp_ts.index as usize].linked_cards;
    assert_eq!(
        in_zone(linked, &r, "BT25-100"),
        1,
        "Iron Slash linked to its owner's [TS] Digimon: {seen:#?}"
    );
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "Final Judgment trashed after the attack");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// A plain [Security] effect on an Option checked during the held attack.
#[test]
fn plain_security_option_effect_during_final_judgments_attack() {
    let mut r = base_builder()
        .from_dsl_yaml(SEC_DRAW_OPT_YAML)
        .expect("security option")
        .security(1, &["SEC", "SEC", "SEC", "T-SECDRAW"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    let opp_hand = r.game.player(1).hand.len();
    use_monarch_and_attack(&mut r, 0, atk);
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(
        r.game.player(1).hand.len(),
        opp_hand + 1,
        "[Security] <Draw 1>: {seen:#?}"
    );
    assert_eq!(trash_has(&r, 1, "T-SECDRAW"), 1);
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

// ─── 5. No attack: no hold ──────────────────────────────────────────────────

#[test]
fn declined_attack_holds_nothing() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    r.execute_action(0, encode_attack(0, atk.index as u16))
        .unwrap();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Target));
    r.execute_action(0, PASS).expect("decline the attack");
    assert!(r.game.held_options.is_empty(), "nothing held");
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert_eq!(r.security_count(1), 4);
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// The buffed Digimon is suspended, so it cannot attack: no attack prompt,
/// no hold, the Option goes to the trash.
#[test]
fn no_legal_attack_holds_nothing() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.player_mut(0).battle_area[atk.index as usize].is_suspended = true;
    r.game.enter_main_phase();
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    r.execute_action(0, encode_attack(0, atk.index as u16))
        .unwrap();
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(
        seen.iter().all(|s| !s.contains("Target")),
        "no attack prompt: {seen:#?}"
    );
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

// ─── 6. The attack ends early ───────────────────────────────────────────────

fn early_end_runner(yaml: &str, id: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = base_builder()
        .from_dsl_yaml(yaml)
        .expect("attacker")
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, id, Some(0));
    r.game.enter_main_phase();
    (r, atk)
}

/// The attacker's [When Attacking] deletes it: 11-1-5 still runs the
/// attack's timings, then it ends; the held Option is finished exactly once.
#[test]
fn attacker_deleted_by_its_when_attacking_finishes_the_held_option_once() {
    let (mut r, atk) = early_end_runner(SELF_DELETE_ATTACKER_YAML, "T-SELFDEL");
    use_monarch_and_attack(&mut r, 0, atk);
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-SELFDEL"), 1, "the attacker was deleted");
    assert_eq!(r.security_count(1), 4, "no security check");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// The attacker's [When Attacking] ends the attack (15-15-1).
#[test]
fn attack_ended_by_an_effect_finishes_the_held_option_once() {
    let (mut r, atk) = early_end_runner(END_ATTACK_ATTACKER_YAML, "T-ENDATK");
    use_monarch_and_attack(&mut r, 0, atk);
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 4, "the attack ended before any check");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

/// The attacker loses a security battle (7000 + 5000 against a 15000-DP
/// security Digimon) and is deleted: the attack ends after the first check
/// and the held Option is finished once.
#[test]
fn attacker_deleted_in_a_security_battle_finishes_the_held_option_once() {
    let mut big = make_test_card("SEC-BIG", "Big Security");
    big.card_kind = CardKind::Digimon;
    big.level = Some(6);
    big.dp = Some(15000);
    let mut r = base_builder()
        .add_card(big)
        .security(1, &["SEC", "SEC", "SEC", "SEC-BIG"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-ATK"), 1, "lost the security battle");
    assert_eq!(r.security_count(1), 3, "the second check never happened");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
    assert!(r.game.held_options.is_empty());
}

// ─── 7. Clone mid-state ─────────────────────────────────────────────────────

/// Clone the game while Final Judgment is held and the second copy is in
/// flight (parked on its own buff pick); drive the original and the clone
/// with the same decisions; they end in the same state.
#[test]
fn clone_while_held_and_inner_in_flight_replays_identically() {
    let (mut r, cougar, _base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    let mut rules = vec![
        ("Will you pay the cost", Pick::First),
        ("with the cost reduced by 2", Pick::First),
    ];
    let seen = drive(&mut r, &mut rules, |r| r.game.pending_option.is_some());
    assert!(r.game.pending_option.is_some(), "inner in flight: {seen:#?}");
    assert_eq!(held_ids(&r), vec![MONARCH.to_string()]);
    assert!(r.game.pending_selection.is_some());

    let mut c = DebugRunner::wrap(r.game.clone());
    assert_eq!(r.game.verification_digest(), c.game.verification_digest());
    assert_eq!(
        format!("{:?}", r.game.held_options),
        format!("{:?}", c.game.held_options)
    );

    let script = || {
        vec![(
            "Choose 1 of your Digimon to gain",
            Pick::Id(encode_attack(0, cougar.index as u16)),
        )]
    };
    let a = drive(&mut r, &mut script(), |_| false);
    let b = drive(&mut c, &mut script(), |_| false);
    assert_eq!(a, b, "same prompts in the same order");
    assert_eq!(r.game.verification_digest(), c.game.verification_digest());
    assert_eq!(r.security_count(1), c.security_count(1));
    assert_eq!(r.security_count(1), 2, "three checks in both");
    assert_eq!(trash_has(&r, 0, MONARCH), 2, "both copies declined Arts: trashed");
    assert_eq!(trash_has(&c, 0, MONARCH), 2);
    assert!(r.game.held_options.is_empty() && c.game.held_options.is_empty());
    assert!(r.game.pending_option.is_none() && c.game.pending_option.is_none());
}

// ─── 8. The guard still fires ───────────────────────────────────────────────

/// A second Option starting while another Option's body is mid-resolution
/// with no attack hold is still refused loudly.
#[test]
#[should_panic(expected = "reentrant Option play while another is mid-resolution")]
fn reentrant_option_use_without_an_attack_hold_still_panics() {
    let mut r = base_builder()
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    // Final Judgment parks on its buff pick: its body is mid-resolution and
    // no attack holds it.
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    assert!(r.game.pending_option.is_some());
    assert!(r.game.held_options.is_empty());
    // Hand index 0 is now T-PLAINOPT.
    let _ = r.game.use_option_from_hand_without_paying_cost(0, 0);
}
