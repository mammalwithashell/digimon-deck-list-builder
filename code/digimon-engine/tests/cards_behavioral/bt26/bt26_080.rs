//! BT26-080 Bacchusmon // Reversal of the Dead — DUAL (Lv.6 Digimon // Option).
//!
//! <Security A. +1> <Succession ([Bacchusmon])>
//! [When Digivolving] By suspending 1 Digimon, this Digimon may attack without
//! suspending.
//! [When Attacking] [Once Per Turn] Delete 1 of your opponent's Digimon with
//! the same orientation as this Digimon.
//! Option: <Use Req. ([TS] trait)> [Main] You may unsuspend 1 Digimon. Then,
//! delete all of your opponent's unsuspended Digimon with the lowest DP.
//! <Arts Digivolve>.
//!
//! <Succession ([Bacchusmon])>: G-ENGINE-SUCCESSION-KEYWORD.
//!
//! DCGO: BT26/Purple/BT26_080.cs.

use super::support::*;
use digimon_dsl::compiled::CompiledCardKind;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT26-080";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-080")
        .dsl_card("BT25-077")
        .expect("BT25-077 Bacchusmon")
        .add_card(digimon(
            "TS-SMALL",
            "Ts Small",
            CardColor::Black,
            3,
            4,
            &["TS"],
        ))
        .add_card(filler("FILLER"))
        .add_card(digimon("D1", "One", CardColor::Red, 1, 1, &[]))
        .add_card(digimon("D3", "Three", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("D5", "Five", CardColor::Red, 5, 5, &[]))
        .add_card(digimon("MINE", "Mine", CardColor::Green, 4, 5, &[]))
        .add_card(tamer("TS-TAMER", "TS Tamer", CardColor::Purple, &["TS"]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .security(1, &["FILLER"; 3])
        .memory(6)
        .start();
    r.set_first_player(0);
    r
}

fn picks(r: &DebugRunner) -> usize {
    r.pending_selection_view()
        .map(|v| v.valid_action_ids.iter().filter(|&&a| a != PASS).count())
        .unwrap_or(0)
}

fn suspend(r: &mut DebugRunner, h: PermanentHandle) {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = true;
}

fn alive(r: &DebugRunner, p: u8) -> Vec<String> {
    field_ids(r, p)
}

#[test]
fn bt26_080_metadata_and_alt_path() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    assert_eq!(c.kind, CompiledCardKind::Dual);
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
    assert!(format!("{:?}", c.alt_paths).contains("Bacchusmon"));
}

#[test]
fn bt26_080_security_attack_plus_one() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    let _ = b;
    let c = r.compiled_card(CARD_ID).unwrap();
    assert!(c.effects.iter().any(|e| matches!(
        e,
        digimon_dsl::compiled::CompiledClause::Declarative(
            digimon_dsl::compiled::CompiledDeclarativeClause::Aura {
                security_attack: Some(1),
                ..
            }
        )
    )));
}

#[test]
fn bt26_080_wd_suspend_then_attack_without_suspending() {
    let mut r = setup();
    r.game.turn_count = 1;
    let b = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_on_field(0, "MINE", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    let v = r.pending_selection_view().expect("suspend pick");
    assert!(v.is_optional, "by suspending — optional");
    assert_eq!(picks(&r), 2, "Bacchusmon + MINE");
    let a = *v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .last()
        .unwrap();
    r.execute_action(0, a).unwrap();
    assert!(r.game.players[0].battle_area[mine.index as usize].is_suspended);
    let v = r.pending_selection_view().expect("attack prompt");
    assert!(v.is_optional, "may attack");
    let atk = encode_attack(b.index as u16, SECURITY_TARGET);
    assert!(v.valid_action_ids.contains(&atk));
    r.game.resolve_selection(0, atk).unwrap();
    let _ = r.auto_resolve();
    assert!(
        !r.game.players[0].battle_area[b.index as usize].is_suspended,
        "attacked without suspending"
    );
    assert_eq!(r.security_count(1), 1, "<Security A. +1> → 2 checks");
}

#[test]
fn bt26_080_wd_decline_no_attack() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    let mine = r.place_on_field(0, "MINE", Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, b);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!r.game.players[0].battle_area[mine.index as usize].is_suspended);
    assert_eq!(r.security_count(1), 3);
}

#[test]
fn bt26_080_wa_unsuspended_deletes_unsuspended_once_per_turn() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    let _s = {
        let h = r.place_on_field(1, "D3", Some(0));
        suspend(&mut r, h);
        h
    };
    r.place_on_field(1, "D5", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, b);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional, "mandatory");
    assert_eq!(picks(&r), 1, "only the unsuspended one");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(alive(&r, 1), vec!["D3".to_string()]);
    r.place_on_field(1, "D5", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, b);
    assert!(r.pending_selection_view().is_none(), "once per turn");
    assert_eq!(alive(&r, 1).len(), 2);
}

#[test]
fn bt26_080_wa_suspended_deletes_suspended() {
    let mut r = setup();
    let b = r.place_on_field(0, CARD_ID, Some(0));
    suspend(&mut r, b);
    let s = r.place_on_field(1, "D3", Some(0));
    suspend(&mut r, s);
    r.place_on_field(1, "D5", Some(0));
    fire(&mut r, EffectTiming::WhenAttacking, b);
    assert_eq!(picks(&r), 1, "only the suspended one");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(alive(&r, 1), vec!["D5".to_string()]);
}

fn use_option(r: &mut DebugRunner) {
    push_hand(r, 0, CARD_ID);
    let idx = r.game.players[0].hand.len() - 1;
    let _ = r.game.play_option_from_hand(0, idx);
}

#[test]
fn bt26_080_option_deletes_all_lowest_unsuspended() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    r.place_on_field(1, "D3", Some(0));
    r.place_on_field(1, "D3", Some(0));
    r.place_on_field(1, "D5", Some(0));
    let low = r.place_on_field(1, "D1", Some(0));
    suspend(&mut r, low); // suspended → not considered
    use_option(&mut r);
    let v = r.pending_selection_view().expect("unsuspend pick");
    assert!(v.is_optional, "may unsuspend");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    let mut f = alive(&r, 1);
    f.sort();
    assert_eq!(
        f,
        vec!["D1".to_string(), "D5".to_string()],
        "both 3000s deleted"
    );
}

#[test]
fn bt26_080_option_unsuspend_changes_lowest() {
    let mut r = setup();
    r.place_on_field(0, "TS-TAMER", Some(0));
    r.place_on_field(1, "D3", Some(0));
    let low = r.place_on_field(1, "D1", Some(0));
    suspend(&mut r, low);
    use_option(&mut r);
    // Unsuspend the opponent's D1 (the only suspended Digimon … pick it).
    let v = r.pending_selection_view().expect("unsuspend pick");
    // Candidates: TS-TAMER is not a Digimon; D3, D1 are. Pick D1 = last.
    let a = *v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .last()
        .unwrap();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(
        alive(&r, 1),
        vec!["D3".to_string()],
        "D1 now lowest unsuspended"
    );
}

// ─── <Succession ([Bacchusmon])> ────────────────────────────────────────────

fn resolve_all(r: &mut DebugRunner) {
    for _ in 0..20 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    let _ = r.auto_resolve();
}

#[test]
fn bt26_080_succession_adopts_bacchusmon_when_digivolving() {
    let mut r = setup();
    r.game.players[0].hand.clear();
    let h = r.place_stack(0, &["BT25-077", CARD_ID]);
    r.add_to_hand(0, "TS-SMALL");
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    resolve_all(&mut r);
    assert!(
        alive(&r, 0).contains(&"TS-SMALL".to_string()),
        "BT25-077's [WD] free play ran from the carrier: {:?}",
        alive(&r, 0)
    );
}

#[test]
fn bt26_080_without_bacchusmon_source_no_free_play() {
    let mut r = setup();
    r.game.players[0].hand.clear();
    let h = r.place_stack(0, &["MINE", CARD_ID]);
    r.add_to_hand(0, "TS-SMALL");
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    resolve_all(&mut r);
    assert!(!alive(&r, 0).contains(&"TS-SMALL".to_string()));
}
