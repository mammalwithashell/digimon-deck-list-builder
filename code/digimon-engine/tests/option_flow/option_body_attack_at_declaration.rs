//! An attack declared inside an Option's `[Main]` body: the Option finishes
//! (its own <Arts Digivolve> offer, or the trash, or its placement) as soon as
//! that attack has been DECLARED -- before the attack's queued [When
//! Attacking] effects resolve and before the counter timing. Options used
//! during that attack therefore find no Option in flight.
//! G-ENGINE-OPTION-BODY-ATTACK-NOT-HELD (resolved by reversal).
//!
//! Monarchlizamon (BT25-057) Option face "Final Judgment": "[Main] 1 of your
//! Digimon gains <Rush>, <Security A. +1> and +5000 DP until your opponent's
//! turn ends. Then, it may attack." plus <Arts Digivolve>.
//!
//! Why the declaration and not the end of the attack: the project owner's
//! standing default (2026-11-24) is "Generally let's look to match dcgo"; the
//! application of that default to this card is the orchestrator's, not a new
//! owner ruling. DCGO finishes the Option at the declaration:
//! `AttackProcess.Attack()` returns once the attack is declared
//! (AttackProcess.cs:242 `State = AttackState.Counter`), `SelectAttackEffect`
//! awaits only that (SelectAttackEffect.cs:547), `CardController.UseOption`
//! then resolves the card's own Arts Digivolve or trashes it
//! (CardController.cs:2026-2129), and only afterwards does the phase loop
//! resolve the queued [When Attacking] effects and run counter / block /
//! battle (TurnStateMachine.cs:1277-1286). Official Q&A agrees: Q4962 ("If
//! you use BT22-010 [Meramon]'s effect to attack, you gain 1 memory after the
//! attack declaration") and Q4346 (an effect's attack resolves at the
//! declaration; the next effect activates before the counter timing). CR 4.3
//! 11-2-4 (no new attack while one is in progress) still holds: a second
//! "it may attack" during the attack offers nothing.
//!
//! Before the reversal the engine held the Option until the attack ended
//! (`Game::held_options`, commits 32d148445 / a7a55dac1); these tests were
//! that mechanism's tests and are re-expressed for the declaration order.
//! The one DCGO detail not copied: DCGO runs an `IsBackgroundProcess` on-ally-
//! attack effect inline during the declaration (AutoProcessing.cs:1006),
//! before the Option finishes; the engine queues every [When Attacking].

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
/// BT25-083 without their costs). Lv.5, so never an <Arts Digivolve> host
/// for Monarchlizamon (BT25-057).
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

/// A green Lv.4 [Glowing Dawn] Digimon whose OWN (non-inherited) [When
/// Attacking] gains 1 memory: an <Arts Digivolve> host that, when it is the
/// attacker, has a [When Attacking] pending at the declaration.
const MEMORY_HOST_YAML: &str = r#"
card: T-MEMHOST
name: Memory Host
kind: digimon
level: 4
color: [green]
cost: 4
dp: 4000
traits: ["Glowing Dawn"]
effects:
  - when: when_attacking
    summary: "[When Attacking] Gain 1 memory"
    process:
      - gain_memory: 1
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

fn blocker(id: &str) -> CardData {
    let mut c = make_test_card(id, "Blocker");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.keywords = vec![Keyword::Blocker];
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
/// prompt is declined when optional (a trigger-order prompt takes its first
/// entry) and otherwise answered with its first action. Stops when no
/// selection is parked, or before answering when `stop` returns true.
/// Returns the prompts seen.
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
            None if sel.kind == SelectionKind::TriggerOrder => Pick::First,
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

fn in_flight_id(r: &DebugRunner) -> Option<String> {
    r.game
        .pending_option
        .as_ref()
        .map(|p| p.card.card_id(&r.game.card_data).to_string())
}

/// The <Arts Digivolve> offer with `host` among its legal hosts.
fn arts_on(host: PermanentHandle) -> impl Fn(&DebugRunner) -> bool {
    move |r: &DebugRunner| {
        r.game.pending_selection.as_ref().is_some_and(|s| {
            s.kind == SelectionKind::OwnField
                && s.is_optional
                && s.prompt.contains("Arts Digivolve")
                && s.valid_action_ids
                    .contains(&encode_attack(0, host.index as u16))
        })
    }
}

/// Use Monarchlizamon (BT25-057) from hand index `hand_index`, give `buffed`
/// the buffs and declare its attack on the opponent. Leaves the engine right
/// after the declaration.
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

/// Right after the declaration with no <Arts Digivolve> host: the Option is
/// already in the trash, nothing is in flight, the attack is open, and no
/// security card has been checked.
fn assert_finished_at_declaration(r: &DebugRunner, security_before: usize) {
    assert!(r.game.pending_attack.is_some(), "the attack is declared and open");
    assert_eq!(trash_has(r, 0, MONARCH), 1, "trashed at the declaration");
    assert!(r.game.pending_option.is_none(), "no Option in flight");
    assert_eq!(r.security_count(1), security_before, "no security check yet");
}

// ─── 1. The reproduction line ───────────────────────────────────────────────

/// BT25-057 used from hand on an attacker whose [When Attacking] may use an
/// Option from hand. The attack is declared inside the Option's effect; the
/// Option is trashed at once (no Lv.4 host for its <Arts Digivolve>); THEN
/// the [When Attacking] uses another Option, which finds the in-flight slot
/// free, resolves completely (draw, trash), and the attack continues through
/// its two security checks (<Security A. +1>). (Under the former hold this
/// line needed `held_options`; before that it panicked "reentrant Option
/// play while another is mid-resolution".)
#[test]
fn option_used_by_when_attacking_after_the_option_finished_at_declaration() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    let hand_before = r.game.player(0).hand.len();

    use_monarch_and_attack(&mut r, 0, atk);

    // The [When Attacking] prompt comes after the Option finished.
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    assert_finished_at_declaration(&r, 4);

    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let seen = drive(&mut r, &mut rules, |_| false);
    assert!(rules.is_empty(), "the inner use was offered: {seen:#?}");

    assert_eq!(trash_has(&r, 0, "T-PLAINOPT"), 1, "inner trashed: {seen:#?}");
    assert!(r.game.pending_attack.is_none(), "the attack ended: {seen:#?}");
    assert_eq!(r.security_count(1), 2, "two security checks (<Security A. +1>)");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed once");
    assert!(r.game.pending_option.is_none(), "no Option left in flight");
    assert!(r.game.pending_selection.is_none(), "{seen:#?}");
    // Both Options left the hand and the inner one drew 1.
    assert_eq!(r.game.player(0).hand.len(), hand_before - 2 + 1);
}

/// Same line with a <Blocker> on the defending side: at the block timing
/// (after the [When Attacking] effects, 11-1-3) both Options have finished
/// and no security check has been made yet.
#[test]
fn both_options_finish_before_the_block_timing() {
    let mut r = base_builder()
        .add_card(blocker("OPP-BLK"))
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    r.place_on_field(1, "OPP-BLK", None);
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);
    assert_finished_at_declaration(&r, 4);

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
    assert!(r.game.pending_option.is_none());
    assert_eq!(trash_has(&r, 0, MONARCH), 1);

    // No block: the attack checks twice.
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert_eq!(r.security_count(1), 2, "{seen:#?}");
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
}

// ─── 2. <Arts Digivolve> at the declaration ─────────────────────────────────

/// GD-BASE (a green Lv.4 [Glowing Dawn] Digimon) is on the field: right after
/// the declaration the outer Option offers its <Arts Digivolve>, before any
/// [When Attacking] and with no security card checked. Accepted onto GD-BASE;
/// then the [When Attacking] uses an Option that places itself in the battle
/// area, and the attack checks twice.
#[test]
fn outer_arts_at_declaration_then_inner_placed_option() {
    let mut r = base_builder()
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH, "T-PLACEDOPT"])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-ATK", Some(0));
    let base = r.place_on_field(0, "GD-BASE", Some(1));
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);

    assert!(arts_on(base)(&r), "the Arts offer follows the declaration");
    assert!(r.game.pending_attack.is_some(), "the attack is open");
    assert_eq!(in_flight_id(&r).as_deref(), Some(MONARCH));
    assert_eq!(r.security_count(1), 4, "no security check before the Arts offer");
    assert_eq!(hand_has(&r, 0, "T-PLACEDOPT"), 1, "no [When Attacking] yet");
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("accept the Arts Digivolve");
    assert_eq!(top_id(&r, base), MONARCH, "Arts Digivolved onto GD-BASE");
    assert!(r.game.pending_option.is_none(), "the Option finished");
    assert_eq!(r.security_count(1), 4);

    let mut rules = vec![("Use 1 Option card", Pick::First)];
    let seen = drive(&mut r, &mut rules, |_| false);
    assert!(rules.is_empty(), "{seen:#?}");
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "both checks after the Arts Digivolve");
    let placed = r
        .game
        .player(0)
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&r.game.card_data) == "T-PLACEDOPT")
        .count();
    assert_eq!(placed, 1, "the inner Option was placed in the battle area");
    assert_eq!(trash_has(&r, 0, "T-PLACEDOPT"), 0);
    assert_eq!(trash_has(&r, 0, MONARCH), 0);
    assert!(r.game.pending_option.is_none());
}

/// Arts accepted onto the ATTACKER itself in the middle of the declaration
/// (4-20-1). The attacker (T-MEMHOST, a green Lv.4 [Glowing Dawn] Digimon
/// whose own [When Attacking] gains 1 memory) becomes Monarchlizamon and
/// stays the
/// attacker: the attack continues through the counter timing, the block
/// timing (the defender has a <Blocker> and is asked) and two security
/// checks (<Security A. +1> stays on the permanent).
///
/// Monarchlizamon (BT25-057)'s own "[When Digivolving] [When Attacking]
/// [Once Per Turn] ... <De-Digivolve 1>" does not activate as a [When
/// Attacking] for this attack (official Q6344: "Can I then activate the [When
/// Attacking] effect on the Digimon that attacked? -- No"): it was not on the
/// attacker when the attack was declared. Its [When Digivolving] effects do
/// trigger from the Arts Digivolve.
///
/// The old top card's own [When Attacking] (T-MEMHOST's "Gain 1 memory") was
/// pending from the declaration and the Arts Digivolve took that text away
/// before it activated. CR 4.3 15-4-4-4: "When a card with an effect that's
/// pending activation loses the effect before it activates, the effect can no
/// longer be activated." DCGO agrees (traced in source, not observed at
/// runtime): the queued SkillInfo records the permanent and its top card when
/// it triggered (Script/AutoProcessing.cs:107-114), and
/// `ICardEffect.CanActivate` is false for a non-inherited, non-linked effect
/// whose source card is no longer its permanent's top card
/// (Script/ICardEffect.cs:487-492). No memory is gained.
#[test]
fn arts_onto_the_attacker_at_declaration_keeps_the_attack_going() {
    let mut r = base_builder()
        .from_dsl_yaml(MEMORY_HOST_YAML)
        .expect("memory host")
        .add_card(blocker("OPP-BLK"))
        .security(1, &["SEC", "SEC", "SEC", "SEC"])
        .hand(0, &[MONARCH])
        .memory(8)
        .start();
    let atk = r.place_on_field(0, "T-MEMHOST", Some(0));
    r.place_on_field(1, "OPP-BLK", None);
    r.game.enter_main_phase();
    use_monarch_and_attack(&mut r, 0, atk);

    assert!(arts_on(atk)(&r), "the Arts offer names the attacker");
    assert!(r.game.pending_attack.is_some());
    r.execute_action(0, encode_attack(0, atk.index as u16))
        .expect("Arts Digivolve onto the attacker");
    assert_eq!(top_id(&r, atk), MONARCH, "the attacker is now Monarchlizamon");
    assert!(r.game.pending_option.is_none(), "the Option finished");
    assert!(r.game.pending_attack.is_some(), "still attacking");
    assert_eq!(r.security_count(1), 4);
    let memory_after_arts = r.game.memory;

    // Answer everything up to the defender's block decision, recording the
    // attacker side's prompts on the way.
    let defender_decides = |r: &DebugRunner| {
        r.game
            .pending_selection
            .as_ref()
            .is_some_and(|s| s.selecting_player == 1)
    };
    let seen = drive(&mut r, &mut Vec::new(), defender_decides);
    assert!(defender_decides(&r), "the block timing is reached: {seen:#?}");
    assert!(r.game.pending_attack.is_some(), "{seen:#?}");
    assert_eq!(r.security_count(1), 4, "no check before the block timing");
    assert_eq!(
        r.game.memory, memory_after_arts,
        "CR 15-4-4-4: the old top card's pending [When Attacking] (Gain 1 memory) lost its text and did not activate: {seen:#?}"
    );
    let de_digivolve_offers = seen
        .iter()
        .filter(|s| s.contains("De-Digivolve") || s.contains("face-down"))
        .count();
    assert!(
        de_digivolve_offers <= 1,
        "Monarchlizamon's [When Digivolving]/[When Attacking] is offered at most \
         once, as [When Digivolving] (Q6344): {seen:#?}"
    );

    // No block: the attack checks security twice.
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "two checks after the Arts: {seen:#?}");
    assert_eq!(top_id(&r, atk), MONARCH);
    assert_eq!(trash_has(&r, 0, MONARCH), 0);
}

// ─── 3. Real pool attackers ─────────────────────────────────────────────────

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

const COUGAR_RULES: [(&str, Pick); 2] = [
    ("Will you pay the cost", Pick::First),
    ("with the cost reduced by 2", Pick::First),
];

/// Cougarmon (BT26-026) [When Attacking][Once Per Turn]: "By trashing ...
/// your top security card, you may use 1 Option card with the [Glowing Dawn]
/// trait from your hand with the cost reduced by 2." Used during the attack
/// that Monarchlizamon (BT25-057)'s "Final Judgment" declared, on a second
/// copy of Final Judgment. The first copy has already finished at the
/// declaration (its Arts offer declined: trashed). The second copy buffs
/// Cougarmon again, offers no attack (11-2-4: an attack is in progress), and
/// is offered its own Arts at its own end: accepted onto GD-BASE. The attack
/// then checks three times (<Security A. +1> from each copy).
#[test]
fn cougarmon_uses_a_second_final_judgment_after_the_first_finished() {
    let (mut r, cougar, base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    assert!(arts_on(base)(&r), "the first copy's Arts offer at the declaration");
    assert!(r.game.pending_attack.is_some());
    assert_eq!(r.security_count(1), 5);
    r.execute_action(0, PASS).expect("decline the first copy's Arts");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "the first copy is trashed");
    assert!(r.game.pending_option.is_none());
    let memory_after_outer = r.game.memory;

    let mut rules = COUGAR_RULES.to_vec();
    rules.push((
        "Choose 1 of your Digimon to gain",
        Pick::Id(encode_attack(0, cougar.index as u16)),
    ));
    // Stop at the second copy's Arts offer.
    let seen = drive(&mut r, &mut rules, |r| {
        in_flight_id(r).as_deref() == Some(MONARCH) && arts_on(base)(r)
    });
    assert!(arts_on(base)(&r), "second copy's Arts offer: {seen:#?}");
    assert!(rules.is_empty(), "{seen:#?}");
    assert!(
        seen.iter().all(|s| !s.contains("Target")),
        "no second attack during the attack (11-2-4): {seen:#?}"
    );
    assert!(r.game.pending_attack.is_some(), "inside the attack");
    assert_eq!(r.security_count(0), 1, "Cougarmon paid with its top security card");
    assert_eq!(r.game.memory, memory_after_outer - 2, "Use 4 reduced by 2");
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("second copy Arts Digivolves onto GD-BASE");
    assert_eq!(top_id(&r, base), MONARCH);

    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "3 checks: <Security A. +1> from each copy");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "only the first copy is in the trash");
    assert!(r.game.pending_option.is_none());
}

/// The first copy Arts-digivolves onto GD-BASE at the declaration; the second
/// copy, used by Cougarmon (BT26-026)'s [When Attacking], Arts-digivolves
/// onto the ATTACKING Cougarmon (4-20-1): Cougarmon becomes Monarchlizamon
/// and keeps attacking.
#[test]
fn second_final_judgment_arts_digivolves_the_attacker_mid_attack() {
    let (mut r, cougar, base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    assert!(arts_on(base)(&r), "first copy's Arts offer at the declaration");
    r.execute_action(0, encode_attack(0, base.index as u16))
        .expect("first copy onto GD-BASE");
    assert_eq!(top_id(&r, base), MONARCH);
    assert!(r.game.pending_option.is_none());

    let mut rules = COUGAR_RULES.to_vec();
    rules.push((
        "Choose 1 of your Digimon to gain",
        Pick::Id(encode_attack(0, cougar.index as u16)),
    ));
    let seen = drive(&mut r, &mut rules, |r| {
        in_flight_id(r).as_deref() == Some(MONARCH) && arts_on(cougar)(r)
    });
    assert!(arts_on(cougar)(&r), "second copy's Arts offer: {seen:#?}");
    r.execute_action(0, encode_attack(0, cougar.index as u16))
        .expect("second copy Arts Digivolves onto the attacker");
    assert_eq!(top_id(&r, cougar), MONARCH, "the attacker Arts-digivolved");

    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(
        r.security_count(1),
        2,
        "the attack kept checking after the Arts Digivolve: {seen:#?}"
    );
    assert_eq!(trash_has(&r, 0, MONARCH), 0, "both copies Arts-digivolved");
    assert!(r.game.pending_option.is_none());
}

/// BeelStarmon ACE (EX7-059) [When Attacking][Once Per Turn]: "By trashing 1
/// Option card from this Digimon's digivolution cards, you may use 1 [Three
/// Musketeers] trait Option card from your hand without paying the cost."
/// The inner Option is Hurricane Screw Shot (EX7-071), whose [Main] places it
/// as the bottom digivolution card of a [Three Musketeers] Digimon. Final
/// Judgment's Arts offer comes first, at the declaration (declined: trashed).
#[test]
fn beelstarmon_ace_uses_hurricane_screw_shot_after_final_judgment_finished() {
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
    assert!(arts_on(base)(&r), "Arts offer at the declaration");
    assert_eq!(trash_has(&r, 0, "T-SRCOPT"), 0, "no [When Attacking] yet");
    r.execute_action(0, PASS).expect("decline the Arts Digivolve");
    assert_finished_at_declaration(&r, 4);

    let mut rules = vec![
        ("Trash 1 Option card from this Digimon", Pick::First),
        ("[Three Musketeers] trait Option card from your hand", Pick::First),
        (
            "Place this card as the bottom digivolution card",
            Pick::Id(encode_attack(0, beel.index as u16)),
        ),
    ];
    let seen = drive(&mut r, &mut rules, |_| false);
    assert!(rules.is_empty(), "every inner prompt was offered: {seen:#?}");
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 2, "two checks (<Security A. +1>)");
    assert_eq!(trash_has(&r, 0, "T-SRCOPT"), 1, "the cost was paid");
    let beel_sources = &r.game.player(0).battle_area[beel.index as usize].card_sources;
    assert_eq!(
        in_zone(beel_sources, &r, "EX7-071"),
        1,
        "Hurricane Screw Shot was placed under BeelStarmon ACE"
    );
    assert_eq!(trash_has(&r, 0, "EX7-071"), 0);
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert!(r.game.pending_option.is_none());
}

// ─── 4. Security Options checked during the attack ──────────────────────────

/// "[Security] Activate this card's [Main] effect" on a Plug-In Option, Iron
/// Slash (BT25-100): its link step links IRON SLASH to its owner's [TS]
/// Digimon. Final Judgment finished at the declaration, so nothing else is in
/// flight when the security check uses the Option.
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
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "Final Judgment finished long before");
    let link_pick = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.execute_action(1, link_pick).expect("link Iron Slash");
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    let linked = &r.game.player(1).battle_area[opp_ts.index as usize].linked_cards;
    assert_eq!(
        in_zone(linked, &r, "BT25-100"),
        1,
        "Iron Slash linked to its owner's [TS] Digimon: {seen:#?}"
    );
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert!(r.game.pending_option.is_none());
}

/// A plain [Security] effect on an Option checked during the attack.
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
}

// ─── 5. No attack ───────────────────────────────────────────────────────────

/// Declining "it may attack" leaves no attack behind: no attack opens, no
/// MayAttack grant is left for later, the Option is trashed, no security is
/// checked.
#[test]
fn declined_attack_leaves_nothing_deferred() {
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
    let _ = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "no attack opened");
    assert!(
        !r.game
            .modifiers
            .has(atk, digimon_engine::enums::ModifierType::MayAttack),
        "no deferred MayAttack grant"
    );
    assert!(!r.game.player(0).battle_area[atk.index as usize].is_suspended);
    assert_eq!(trash_has(&r, 0, MONARCH), 1);
    assert_eq!(r.security_count(1), 4);
    assert!(r.game.pending_option.is_none());
}

/// The buffed Digimon is suspended, so it cannot attack: no attack prompt,
/// the Option goes to the trash.
#[test]
fn no_legal_attack_trashes_the_option() {
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
}

// ─── 6. The attack ends early ───────────────────────────────────────────────
//
// The Option finished at the declaration, so an attack that ends early has
// nothing left to finish: the Option must be in the trash exactly once.

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
/// attack's timings, then it ends.
#[test]
fn attacker_deleted_by_its_when_attacking_after_the_option_finished() {
    let (mut r, atk) = early_end_runner(SELF_DELETE_ATTACKER_YAML, "T-SELFDEL");
    use_monarch_and_attack(&mut r, 0, atk);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed at the declaration");
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-SELFDEL"), 1, "the attacker was deleted");
    assert_eq!(r.security_count(1), 4, "no security check");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
}

/// The attacker's [When Attacking] ends the attack (15-15-1).
#[test]
fn attack_ended_by_an_effect_after_the_option_finished() {
    let (mut r, atk) = early_end_runner(END_ATTACK_ATTACKER_YAML, "T-ENDATK");
    use_monarch_and_attack(&mut r, 0, atk);
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed at the declaration");
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(r.security_count(1), 4, "the attack ended before any check");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
}

/// The attacker loses a security battle (7000 + 5000 against a 15000-DP
/// security Digimon) and is deleted: the attack ends after the first check.
#[test]
fn attacker_deleted_in_a_security_battle_after_the_option_finished() {
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
    // No decision follows the declaration here (the [When Attacking] has no
    // Option to use), so the whole attack runs inside this call.
    use_monarch_and_attack(&mut r, 0, atk);
    let seen = drive(&mut r, &mut Vec::new(), |_| false);
    assert!(r.game.pending_attack.is_none(), "{seen:#?}");
    assert_eq!(trash_has(&r, 0, "T-ATK"), 1, "lost the security battle");
    assert_eq!(r.security_count(1), 3, "the second check never happened");
    assert_eq!(trash_has(&r, 0, MONARCH), 1, "trashed exactly once");
    assert!(r.game.pending_option.is_none());
}

// ─── 7. Clone safety ────────────────────────────────────────────────────────

/// Clone the game (a) at the <Arts Digivolve> offer made at the declaration
/// and (b) at the first [When Attacking] prompt, then drive the original and
/// each clone with the same decisions: they reach identical digests.
#[test]
fn clone_at_the_arts_offer_and_at_the_first_when_attacking_prompt() {
    let (mut r, cougar, base) = cougarmon_runner();
    use_monarch_and_attack(&mut r, 0, cougar);
    assert!(arts_on(base)(&r), "the Arts offer at the declaration");
    let at_arts = r.game.clone();
    assert_eq!(at_arts.verification_digest(), r.game.verification_digest());

    r.execute_action(0, PASS).expect("decline the first copy's Arts");
    assert!(r.game.pending_attack.is_some());
    assert!(r.game.pending_selection.is_some(), "the first [When Attacking] prompt");
    assert!(r.game.pending_option.is_none());
    let at_when_attacking = r.game.clone();
    assert_eq!(
        at_when_attacking.verification_digest(),
        r.game.verification_digest()
    );

    let script = || {
        let mut rules = COUGAR_RULES.to_vec();
        rules.push((
            "Choose 1 of your Digimon to gain",
            Pick::Id(encode_attack(0, cougar.index as u16)),
        ));
        rules
    };
    let a = drive(&mut r, &mut script(), |_| false);
    let end = r.game.verification_digest();
    assert_eq!(r.security_count(1), 2, "three checks");
    assert_eq!(trash_has(&r, 0, MONARCH), 2, "both copies declined Arts: trashed");
    assert!(r.game.pending_option.is_none() && r.game.pending_attack.is_none());

    let mut c = DebugRunner::wrap(at_when_attacking);
    let b = drive(&mut c, &mut script(), |_| false);
    assert_eq!(a, b, "same prompts in the same order");
    assert_eq!(c.game.verification_digest(), end, "clone at the [When Attacking]");

    let mut c = DebugRunner::wrap(at_arts);
    c.execute_action(0, PASS)
        .expect("decline the first copy's Arts on the clone");
    let b = drive(&mut c, &mut script(), |_| false);
    assert_eq!(a, b, "same prompts in the same order");
    assert_eq!(c.game.verification_digest(), end, "clone at the Arts offer");
}

// ─── 8. The guard still fires ───────────────────────────────────────────────

/// A second Option starting while another Option's body is mid-resolution is
/// still refused loudly.
#[test]
#[should_panic(expected = "reentrant Option play while another is mid-resolution")]
fn reentrant_option_use_while_another_is_mid_resolution_still_panics() {
    let mut r = base_builder()
        .hand(0, &[MONARCH, "T-PLAINOPT"])
        .memory(8)
        .start();
    r.place_on_field(0, "T-ATK", Some(0));
    r.game.enter_main_phase();
    // Final Judgment parks on its buff pick: its body is mid-resolution.
    assert_eq!(r.game.play_option_from_hand(0, 0), OptionPlayResult::Pending);
    assert!(r.game.pending_option.is_some());
    // Hand index 0 is now T-PLAINOPT.
    let _ = r.game.use_option_from_hand_without_paying_cost(0, 0);
}
