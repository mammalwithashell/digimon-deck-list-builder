//! BT26-053 Wolvermon — Digimon, Lv.4, Black, DP 5000, Cost 5.
//! Traits: Cyborg, Glowing Dawn, BEATBREAK. Form: Champion. Attribute: Vaccine.
//! Digivolve: Black Lv.3 / cost 2; [Digivolve] Lv.3 w/[Glowing Dawn] trait: Cost 2.
//!
//! # Printed text (official Bandai DB — data/card_bundles/BT26-053.md)
//!
//! ```text
//! <Blocker> [All Turns] [Once Per Turn] When attack targets change, by
//! trashing the bottom face-down card from under any of your Tamers, you may
//! use 1 Option card with the [Glowing Dawn] trait and a use cost of 4 or less
//! from your hand without paying the cost.
//! Inherited: <Blocker> (This Digimon can block in the blocker timing.)
//! ```
//!
//! Owner ruling 2026-10-20: "the bottom face-down card" is the LOWEST
//! face-down card, even with face-up cards below it.
//! Official Q&A (BT26-004, Q6957): a trashed face-down card goes to the trash.
//! Rules 4-6-1 / 4-6-6-2: a DUAL card can be referenced as an Option card, by
//! its Option information (use cost).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT26/Black/BT26_053.cs
//!   - AddSelfDigivolutionRequirementStaticEffect(level 3, EqualsTraits
//!     "Glowing Dawn", cost 2)
//!   - BlockerSelfStaticEffect (own + inherited)
//!   - OnAttackTargetChanged (any switch), maxCount 1, CanActivate = an own
//!     Tamer with a trashable face-down card; Tamer pick canNoSelect:true
//!     (declining = not paying, RemoveUse) → trash its bottom face-down card →
//!     canNoSelect:true hand pick over IsOption && [Glowing Dawn] && cost <= 4
//!     → PlayOptionCards(payCost: false).
//!
//! # Patterns
//! - alt digivolve (`level_eq` + `trait_has`)
//! - own + inherited `grant_keyword Blocker`
//! - `on_attack_target_change` + `[Once Per Turn]` + "by trashing" cost
//!   (`trash_bottom_face_down_source_under_tamer`) + optional
//!   `use_option_from_hand` (free)

use super::support::*;
use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-053";

/// Black [Glowing Dawn] Option, use cost 4: "[Main] <Draw 1>" — eligible.
const GD_OPT4_YAML: &str = r#"
card: T-GDOPT4
name: GD Option Four
kind: option
color: [black]
cost: 4
traits: [Glowing Dawn]
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Black [Glowing Dawn] Option, use cost 5 — too expensive.
const GD_OPT5_YAML: &str = r#"
card: T-GDOPT5
name: GD Option Five
kind: option
color: [black]
cost: 5
traits: [Glowing Dawn]
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Black Option without the trait, use cost 2 — wrong trait.
const PLAIN_OPT_YAML: &str = r#"
card: T-PLAINOPT
name: Plain Option
kind: option
color: [black]
cost: 2
traits: []
effects:
  - when: main_from_hand
    summary: "[Main] <Draw 1>"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Test-only observer: "[All Turns] When effects trash cards from under your
/// Tamers, gain 1 memory" — proof that the cost trash is a real
/// digivolution-card trash event.
const TRASH_OBSERVER_YAML: &str = r#"
card: T-OBS
name: Trash Observer
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_digivolution_card_trashed
    active_when: { all_turns: true }
    timing_conditions:
      - when: on_digivolution_card_trashed
        condition:
          event_host_is_own_tamer: true
    summary: "[All Turns] When effects trash cards from under your Tamers, gain 1 memory"
    process:
      - gain_memory: 1
"#;

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-053 in the embedded DSL pack")
        .dsl_card("BT26-057")
        .expect("BT26-057 (a [Glowing Dawn] DUAL, Option use cost 4)")
        .from_dsl_yaml(GD_OPT4_YAML)
        .expect("opt4")
        .from_dsl_yaml(GD_OPT5_YAML)
        .expect("opt5")
        .from_dsl_yaml(PLAIN_OPT_YAML)
        .expect("plain opt")
        .from_dsl_yaml(TRASH_OBSERVER_YAML)
        .expect("observer")
        .add_card(filler("FILLER"))
        .add_card(tamer("GDT", "GD Tamer", CardColor::Purple, &["Glowing Dawn"]))
        .add_card(tamer("PT", "Plain Tamer", CardColor::Red, &[]))
        .add_card(digimon("GD3", "GD Lv3", CardColor::Yellow, 3, 3, &["Glowing Dawn"]))
        .add_card(digimon("PLAIN3", "Plain Lv3", CardColor::Yellow, 3, 3, &[]))
        .add_card(digimon("GDDIGI", "GD Digimon", CardColor::Black, 3, 3, &["Glowing Dawn"]))
        .add_card(digimon("ATK", "Attacker", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("ATK2", "Attacker 2", CardColor::Red, 5, 7, &[]))
        .add_card(blocker("BLK"))
        .add_card(blocker("BLK2"))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(0, &["FILLER"; 4])
        .security(1, &["FILLER"; 4])
        .memory(4)
        .start();
    r.set_first_player(0);
    r
}

/// A level-3 (3000 DP) Digimon with a printed <Blocker>.
fn blocker(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = digimon(id, id, CardColor::Red, 3, 3, &[]);
    c.keywords = vec![Keyword::Blocker];
    c
}

/// The current handle of `p`'s battle-area permanent topped by `id` (indices
/// shift when a battle deletes a permanent).
fn handle_of(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let idx = field_ids(r, p)
        .iter()
        .position(|x| x == id)
        .unwrap_or_else(|| panic!("{id} not on player {p}'s field"));
    PermanentHandle {
        player: p,
        index: idx as u8,
    }
}

/// P0 attacks P1 with `atk`; P1 blocks with `blk` (an attack target change).
fn own_attack_gets_blocked(r: &mut DebugRunner, atk: &str, blk: &str) {
    let a = handle_of(r, 0, atk);
    let b = handle_of(r, 1, blk);
    r.attack_player(a, 1, false);
    r.game
        .resolve_selection(1, encode_attack(0, b.index as u16))
        .expect("P1 blocks");
}

fn hand_action(r: &DebugRunner, card_id: &str) -> u16 {
    let idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand"));
    PLAY_HAND_START + idx as u16
}

fn offered(r: &DebugRunner, action: u16) -> bool {
    r.pending_selection_view()
        .is_some_and(|v| v.valid_action_ids.contains(&action))
}

/// Board for the trigger: a [Glowing Dawn] Tamer with 1 face-down card,
/// Wolvermon, an attacker ATK, and an opposing blocker BLK. Returns the Tamer.
fn board(r: &mut DebugRunner) -> PermanentHandle {
    let t = tamer_with_face_down(r, 0, "GDT", 1);
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(1, "BLK", Some(0));
    t
}

// ─── Section 1: structural ──────────────────────────────────────────────────

#[test]
fn bt26_053_structure() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(c.level, Some(4));
    assert_eq!(c.dp, Some(5000));
    assert_eq!(c.alt_paths.len(), 1, "[Digivolve] Lv.3 w/[Glowing Dawn]");
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 1);
    let t = trig[0];
    assert_eq!(t.when, vec![CompiledTiming::OnAttackTargetChange]);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.optional, "the 'by trashing' cost is the player's choice");
    let dbg = format!("{c:?}");
    assert!(dbg.matches("Blocker").count() >= 2, "own + inherited <Blocker>");
}

// ─── Alt digivolve ──────────────────────────────────────────────────────────

#[test]
fn bt26_053_digivolves_from_a_glowing_dawn_level_3_for_2() {
    let mut r = setup();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "GD3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    let mem = r.memory();
    r.game.decode_action(action, 0);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, host), CARD_ID);
    assert_eq!(r.memory(), mem - 2);
}

#[test]
fn bt26_053_alt_digivolve_needs_the_glowing_dawn_trait() {
    let mut r = setup();
    r.game.enter_main_phase();
    let host = r.place_on_field(0, "PLAIN3", Some(0));
    push_hand(&mut r, 0, CARD_ID);
    let action = encode_digivolve(0, host.index as u16);
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 0.0);
}

// ─── <Blocker> (own + inherited) ────────────────────────────────────────────

#[test]
fn bt26_053_has_blocker() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(w, Keyword::Blocker));
}

#[test]
fn bt26_053_inherited_blocker() {
    let mut r = setup();
    let h = r.place_stack(0, &[CARD_ID, "GD3"]);
    assert!(r.game.has_keyword(h, Keyword::Blocker), "inherited <Blocker>");
    let plain = r.place_on_field(0, "GD3", Some(0));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
}

#[test]
fn bt26_053_blocks_and_its_own_block_triggers_the_effect() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    let w = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "T-GDOPT4");
    r.pass_turn(); // → P1
    let opp = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(opp, 0, false);
    r.game
        .resolve_selection(0, encode_attack(0, w.index as u16))
        .expect("Wolvermon blocks");
    r.accept_optional_trigger()
        .expect("its block changed the attack target");
    pick_first(&mut r, 0); // the Tamer
    assert_eq!(sources(&r, t), 0);
    let a = hand_action(&r, "T-GDOPT4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"T-GDOPT4".to_string()), "Option used");
}

// ─── [All Turns][OPT] attack target change ──────────────────────────────────

#[test]
fn bt26_053_target_change_trashes_face_down_and_uses_a_gd_option_free() {
    let mut r = setup();
    let t = board(&mut r);
    push_hand(&mut r, 0, "T-GDOPT4");
    let mem = r.memory();
    let hand_before = r.hand_size(0);
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    assert!(r.pending_is_optional(), "'by trashing' — the player may decline");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0); // the only Tamer
    assert_eq!(sources(&r, t), 0, "the face-down card is trashed");
    assert!(
        trash_ids(&r, 0).contains(&"FILLER".to_string()),
        "Q6957: the trashed face-down card goes to the trash"
    );
    assert!(r.pending_is_optional(), "'you may use' — an optional hand pick");
    let a = hand_action(&r, "T-GDOPT4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), mem, "used without paying the cost");
    assert!(trash_ids(&r, 0).contains(&"T-GDOPT4".to_string()));
    assert_eq!(
        r.hand_size(0),
        hand_before - 1 + 1,
        "Option left the hand, its [Main] <Draw 1> resolved"
    );
}

#[test]
fn bt26_053_cost_trash_is_a_digivolution_card_trash_event() {
    let mut r = setup();
    board(&mut r);
    r.place_on_field(0, "T-OBS", Some(0));
    let mem = r.memory();
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0); // the Tamer
    // No Option in hand: the use step offers nothing.
    let _ = r.auto_resolve();
    assert_eq!(
        r.memory(),
        mem + 1,
        "the face-down card trashed from under your Tamer fires its observers"
    );
}

#[test]
fn bt26_053_offers_only_gd_options_with_use_cost_4_or_less() {
    let mut r = setup();
    board(&mut r);
    for id in ["T-GDOPT4", "T-GDOPT5", "T-PLAINOPT", "GDDIGI", "BT26-057"] {
        push_hand(&mut r, 0, id);
    }
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    assert!(offered(&r, hand_action(&r, "T-GDOPT4")), "GD, use cost 4");
    assert!(
        offered(&r, hand_action(&r, "BT26-057")),
        "a [Glowing Dawn] DUAL is an Option card by its Option information (use cost 4)"
    );
    assert!(!offered(&r, hand_action(&r, "T-GDOPT5")), "use cost 5 > 4");
    assert!(!offered(&r, hand_action(&r, "T-PLAINOPT")), "no [Glowing Dawn] trait");
    assert!(!offered(&r, hand_action(&r, "GDDIGI")), "not an Option card");
}

#[test]
fn bt26_053_may_decline_the_option_after_paying() {
    let mut r = setup();
    let t = board(&mut r);
    push_hand(&mut r, 0, "T-GDOPT4");
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    assert_eq!(sources(&r, t), 0);
    r.execute_action(0, PASS).expect("'you may' — decline the use");
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"T-GDOPT4".to_string()), "Option stays in hand");
}

#[test]
fn bt26_053_paying_without_an_eligible_option_still_trashes() {
    let mut r = setup();
    let t = board(&mut r);
    push_hand(&mut r, 0, "T-GDOPT5");
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "the cost is paid");
    assert!(hand_ids(&r, 0).contains(&"T-GDOPT5".to_string()));
}

#[test]
fn bt26_053_declined_does_nothing_and_keeps_the_once_per_turn() {
    let mut r = setup();
    let t = board(&mut r);
    r.place_on_field(0, "ATK2", Some(0));
    r.place_on_field(1, "BLK2", Some(0));
    push_hand(&mut r, 0, "T-GDOPT4");
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1, "nothing trashed");
    assert!(hand_ids(&r, 0).contains(&"T-GDOPT4".to_string()));
    own_attack_gets_blocked(&mut r, "ATK2", "BLK2");
    assert!(
        r.pending_is_optional(),
        "declining did not spend the [Once Per Turn]"
    );
}

#[test]
fn bt26_053_is_once_per_turn_and_resets() {
    let mut r = setup();
    let t = board(&mut r);
    let t2 = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.place_on_field(0, "ATK2", Some(0));
    r.place_on_field(1, "BLK2", Some(0));
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t) + sources(&r, t2), 1);
    own_attack_gets_blocked(&mut r, "ATK2", "BLK2");
    assert!(
        !r.pending_is_optional(),
        "[Once Per Turn] spent: a second target change is not offered"
    );
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t) + sources(&r, t2), 1);

    // Next own turn: available again (P1's blocker is unsuspended by then).
    r.pass_turn();
    r.pass_turn();
    for id in ["ATK", "ATK2"] {
        if let Some(i) = field_ids(&r, 0).iter().position(|x| x == id) {
            r.game.players[0].battle_area[i].is_suspended = false;
        }
    }
    r.place_on_field(1, "BLK", Some(0));
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    assert!(r.pending_is_optional(), "reset on a new turn");
}

#[test]
fn bt26_053_needs_a_face_down_card_under_your_tamer() {
    let mut r = setup();
    let t = r.place_stack(0, &["FILLER", "GDT"]); // face-up card only
    r.place_on_field(0, CARD_ID, Some(0));
    tamer_with_face_down(&mut r, 1, "GDT", 1); // the opponent's Tamer doesn't count
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(1, "BLK", Some(0));
    push_hand(&mut r, 0, "T-GDOPT4");
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    assert!(!r.pending_is_optional(), "the cost can't be paid → not offered");
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    assert!(hand_ids(&r, 0).contains(&"T-GDOPT4".to_string()));
}

#[test]
fn bt26_053_does_not_trigger_without_a_target_change() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.place_on_field(0, CARD_ID, Some(0));
    let atk = r.place_on_field(0, "ATK", Some(0));
    push_hand(&mut r, 0, "T-GDOPT4");
    r.attack_player(atk, 1, false); // no blocker: the target never changes
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 1);
    assert!(hand_ids(&r, 0).contains(&"T-GDOPT4".to_string()));
}

#[test]
fn bt26_053_trashes_the_lowest_face_down_card_above_a_face_up_one() {
    // Owner ruling 2026-10-20 (#19): bottom face-down = lowest face-down card.
    let mut r = setup();
    let t = r.place_stack(0, &["FILLER", "GD3", "GDT"]);
    {
        let stack = &mut r.game.players[0].battle_area[t.index as usize].card_sources;
        stack[0].face_down = false; // face-up FILLER at the very bottom
        stack[1].face_down = true; // face-down GD3 above it
    }
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ATK", Some(0));
    r.place_on_field(1, "BLK", Some(0));
    own_attack_gets_blocked(&mut r, "ATK", "BLK");
    r.accept_optional_trigger().unwrap();
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["FILLER".to_string()], "face-up card stays");
    assert!(trash_ids(&r, 0).contains(&"GD3".to_string()));
}

#[test]
fn bt26_053_trigger_works_in_the_opponents_turn() {
    // [All Turns]: P1's attack on P0 is redirected by P0's other blocker.
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "GDT", 1);
    r.place_on_field(0, CARD_ID, Some(0));
    let own_blk = r.place_on_field(0, "BLK", Some(0));
    push_hand(&mut r, 0, "T-GDOPT4");
    r.pass_turn(); // → P1
    let opp = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(opp, 0, false);
    r.game
        .resolve_selection(0, encode_attack(0, own_blk.index as u16))
        .expect("P0 blocks with its other blocker");
    r.accept_optional_trigger().expect("[All Turns]");
    pick_first(&mut r, 0);
    assert_eq!(sources(&r, t), 0);
    let a = hand_action(&r, "T-GDOPT4");
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&"T-GDOPT4".to_string()));
}
