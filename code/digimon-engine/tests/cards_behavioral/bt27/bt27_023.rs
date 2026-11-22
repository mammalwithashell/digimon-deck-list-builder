//! BT27-023: Bandai English card face and Q7643/Q7644 (2026-11-20).
//! https://world.digimoncard.com/cards/index.php?category=522040&search=true
//! Evade optional replacement: CR4.3 16-22. Synthetic cards are targets only.
#[path = "../bt26/support.rs"]
mod support;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{
    CardColor, EffectTiming, GamePhase, Keyword, ModifierType, PlaySource,
};
use support::*;
const ID: &str = "BT27-023";
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT27-023 production YAML required")
        .add_card(filler("FILLER"))
        .add_card(digimon("VB", "VB ally", CardColor::Red, 3, 3, &["VB"]))
        .add_card(digimon("PLAIN", "Plain", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("FIVE", "Five", CardColor::Red, 5, 5, &[]))
        .add_card(digimon("SIX", "Six", CardColor::Red, 6, 6, &[]))
        .add_card(tamer("TAMER", "Gammamon Tamer", CardColor::Red, &["VB"]))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .security(1, &["FILLER"; 5])
        .memory(10)
}
fn setup() -> DebugRunner {
    let mut r = base().start();
    r.set_first_player(0);
    r
}
fn choose(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("player must choose");
    assert!(!v.is_optional);
    let a = v.valid_action_ids[0];
    r.execute_action(0, a).unwrap();
}
#[test]
fn bt27_023_on_play_offers_only_own_qualifying_digimon() {
    let mut r = base().hand(0, &[ID]).start();
    r.set_first_player(0);
    let ally = r.place_on_field(0, "VB", Some(0));
    r.place_on_field(0, "PLAIN", None);
    r.place_on_field(0, "TAMER", None);
    r.place_on_field(1, "VB", None);
    r.play(0, 0).unwrap();
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2,
        "ally and Kaus only"
    );
    choose(&mut r);
    assert!(r
        .game
        .modifiers
        .has(ally, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_023_full_printed_text_domains_qualify_without_name_or_vb() {
    for domain in 0..5 {
        let mut c = digimon("TEXT", "Unrelated", CardColor::Red, 3, 3, &[]);
        match domain {
            0 => c.effect_text = "[Gammamon]".into(),
            1 => c.inherited_text = "[Gammamon]".into(),
            2 => c.security_text = "[Gammamon]".into(),
            3 => c.requirement_text = "[Digivolve] [Gammamon]: Cost 2".into(),
            _ => c.traits = vec!["Gammamon".into()],
        }
        let mut r = base().add_card(c).hand(0, &[ID]).start();
        r.set_first_player(0);
        let h = r.place_on_field(0, "TEXT", None);
        r.play(0, 0).unwrap();
        assert_eq!(
            r.pending_selection_view().unwrap().valid_action_ids.len(),
            2,
            "domain {domain}"
        );
        choose(&mut r);
        assert!(r
            .game
            .modifiers
            .has(h, ModifierType::CannotBeDestroyedByBattle));
    }
}
#[test]
fn bt27_023_real_breeding_move_triggers_protection() {
    let mut r = setup();
    let h = r.place_on_field(0, "VB", Some(0));
    r.place_in_breeding(0, ID);
    assert!(r.pending_selection_view().is_none());
    r.set_phase(GamePhase::Breeding);
    assert!(r.move_from_breeding(0));
    choose(&mut r);
    assert!(r
        .game
        .modifiers
        .has(h, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_023_protection_prevents_losing_battle_but_not_effect_deletion() {
    let mut r = base().hand(0, &[ID]).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, "VB", Some(0));
    let opp = r.place_on_field(1, "SIX", None);
    r.play(0, 0).unwrap();
    choose(&mut r);
    r.battle_digimon(h, opp);
    assert!(field_ids(&r, 0).contains(&"VB".to_string()));
    r.game.delete_permanent_with_effects(h);
    assert!(!field_ids(&r, 0).contains(&"VB".to_string()));
}
#[test]
fn bt27_023_protection_expires_at_end_of_current_turn() {
    let mut r = base().hand(0, &[ID]).start();
    r.set_first_player(0);
    let h = r.place_on_field(0, "VB", Some(0));
    r.play(0, 0).unwrap();
    choose(&mut r);
    r.end_turn();
    assert!(!r
        .game
        .modifiers
        .has(h, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_023_real_attack_restricts_level_five_and_lower_only() {
    let mut r = setup();
    let h = r.place_on_field(0, ID, Some(0));
    let five = r.place_on_field(1, "FIVE", None);
    r.place_on_field(1, "PLAIN", None);
    r.place_on_field(1, "SIX", None);
    r.place_on_field(1, "TAMER", None);
    r.attack_player(h, 1, false);
    assert_eq!(
        r.pending_selection_view().unwrap().valid_action_ids.len(),
        2
    );
    choose(&mut r);
    r.auto_resolve().unwrap();
    assert!(r.game.modifiers.has(five, ModifierType::CannotSuspend));
    r.game.suspend(five);
    assert!(!r.game.players[1].battle_area[five.index as usize].is_suspended);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_none(), "OPT spent");
    r.end_turn();
    assert!(r.game.modifiers.has(five, ModifierType::CannotSuspend));
    r.end_turn();
    assert!(!r.game.modifiers.has(five, ModifierType::CannotSuspend));
    r.game.suspend(five);
    assert!(r.game.players[1].battle_area[five.index as usize].is_suspended);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_some(), "OPT resets next turn");
}
#[test]
fn bt27_023_attack_has_no_target_for_level_six_or_tamer() {
    let mut r = setup();
    let h = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "SIX", None);
    r.place_on_field(1, "TAMER", None);
    fire(&mut r, EffectTiming::WhenAttacking, h);
    assert!(r.pending_selection_view().is_none());
}
#[test]
fn bt27_023_no_protection_when_digivolving_or_passive_on_field() {
    let mut r = setup();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, EffectTiming::WhenDigivolving, h);
    assert!(r.pending_selection_view().is_none());
    assert!(!r
        .game
        .modifiers
        .has(h, ModifierType::CannotBeDestroyedByBattle));
}
#[test]
fn bt27_023_inherited_evade_accept_and_decline_are_real_choices() {
    for accept in [true, false] {
        let mut r = setup();
        let h = r.place_stack(0, &[ID, "FIVE"]);
        assert!(r.game.has_keyword(h, Keyword::Evade));
        r.game.delete_permanent_with_effects(h);
        let v = r.pending_selection_view().unwrap();
        assert!(v.is_optional);
        assert!(v.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
        r.execute_action(0, if accept { REPLACEMENT_ACCEPT } else { PASS })
            .unwrap();
        assert_eq!(field_ids(&r, 0).contains(&"FIVE".into()), accept);
        if accept {
            assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
        }
    }
}
#[test]
fn bt27_023_evade_is_inherited_and_cannot_pay_while_suspended() {
    let mut r = setup();
    let top = r.place_on_field(0, ID, Some(0));
    assert!(!r.game.has_keyword(top, Keyword::Evade));
    let h = r.place_stack(0, &[ID, "FIVE"]);
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    r.game.delete_permanent_with_effects(h);
    assert!(r.pending_selection_view().is_none());
    assert!(!field_ids(&r, 0).contains(&"FIVE".into()));
}
#[test]
fn bt27_023_standard_and_special_evolution_routes() {
    for (name, color, lv, traits, expected) in [
        ("Blue", CardColor::Blue, 3, vec![], Some(3)),
        ("Black", CardColor::Black, 3, vec![], Some(3)),
        ("Gammamon", CardColor::Red, 3, vec![], Some(2)),
        ("VB", CardColor::Green, 3, vec!["VB"], Some(2)),
        ("VB4", CardColor::Green, 4, vec!["VB"], None),
        ("Other", CardColor::Red, 3, vec![], None),
        ("BetelGammamon", CardColor::Red, 3, vec![], None),
    ] {
        let c = digimon("BASE", name, color, lv, 3, &traits);
        let mut r = base().add_card(c).hand(0, &[ID]).start();
        r.set_first_player(0);
        let h = r.place_on_field(0, "BASE", None);
        let before = r.memory();
        let ok = r
            .game
            .digivolve_from_hand(0, 0, h.index as usize, PlaySource::ByHand);
        assert_eq!(ok, expected.is_some(), "{name}");
        if let Some(cost) = expected {
            assert_eq!(before - r.memory(), cost, "{name}");
            assert!(r.pending_selection_view().is_none());
        }
    }
}
#[test]
fn bt27_023_another_digimon_moving_does_not_trigger_self_clause() {
    let mut r = setup();
    r.place_on_field(0, ID, Some(0));
    r.place_in_breeding(0, "VB");
    r.set_phase(GamePhase::Breeding);
    assert!(r.move_from_breeding(0));
    assert!(r.pending_selection_view().is_none());
}
#[test]
fn bt27_023_battle_protection_also_survives_stronger_security_digimon() {
    let mut r = base().hand(0, &[ID]).security(1, &["SIX"]).start();
    r.set_first_player(0);
    let ally = r.place_on_field(0, "VB", Some(0));
    r.play(0, 0).unwrap();
    choose(&mut r);
    r.attack_player(ally, 1, false);
    r.auto_resolve().unwrap();
    assert!(field_ids(&r, 0).contains(&"VB".into()));
    assert_eq!(r.security_count(1), 0);
}
#[test]
fn bt27_023_inherited_evade_cancels_battle_deletion_and_respects_suspend_prohibition() {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::ModifierEntry;
    use digimon_engine::replacement::ReplacementCause;
    for locked in [false, true] {
        let mut r = setup();
        let h = r.place_stack(0, &[ID, "FIVE"]);
        if locked {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotSuspend, 1, Expiry::Permanent, 1),
            );
        }
        r.game
            .delete_permanent_with_cause(h, ReplacementCause::Battle);
        if locked {
            assert!(r.pending_selection_view().is_none());
            assert!(field_ids(&r, 0).is_empty());
        } else {
            assert!(r.pending_selection_view().unwrap().is_optional);
            r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
            assert_eq!(field_ids(&r, 0), vec!["FIVE"]);
        }
    }
}
