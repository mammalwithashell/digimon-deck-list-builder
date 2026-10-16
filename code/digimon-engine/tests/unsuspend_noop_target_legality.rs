//! BT20-027: a legal unsuspend target need not currently be suspended.
//! Official target conditions: general_rule.pdf Ã‚Â§15-10-2, Ã‚Â§15-11-1-3.
//! DCGO BT20_027.cs228-232 uses CanUnsuspend, not CanChangeOrientation.
use digimon_engine::action::{build_action_mask, space::{ATTACK_START, PASS}};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn setup() -> DebugRunner {
    // The compiled DSL fixture omits printed text used by in_text_contains.
    // Load generated runtime metadata, whose source-verified text is the target condition.
    let mut cards = digimon_engine::card_data::CardData::load_from_str(
        include_str!("../../../data/cards.json")
    ).unwrap();
    let slayer = cards.remove("BT20-027").unwrap();
    let wing = cards.remove("EX3-018").unwrap();
    let phoenix = cards.remove("ST1-10").unwrap();
    assert!(slayer.effect_text.contains("Dracomon"));
    assert!(wing.inherited_text.contains("Examon"));
    // Allows RED replay against an archived pre-fix YAML without editing production.
    let yaml = std::env::var("UNSUSPEND_NOOP_BASELINE_YAML")
        .map(|path| std::fs::read_to_string(path).unwrap())
        .unwrap_or_else(|_| include_str!("../cards/BT20/BT20-027.yaml").to_owned());
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml).unwrap()
        .from_dsl_yaml(include_str!("../cards/EX3/EX3-018.yaml")).unwrap()
        .from_dsl_yaml(include_str!("../cards/ST1/ST1-10.yaml")).unwrap()
        .add_card(slayer).add_card(wing).add_card(phoenix)
        .add_card(make_test_card("NOOP-FILL", "Filler"))
        .deck(0, &["NOOP-FILL"; 12]).deck(1, &["NOOP-FILL"; 12])
        .security(0, &["NOOP-FILL"; 5]).security(1, &["NOOP-FILL"; 5])
        .memory(10).start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "BT20-027", None);
    r
}
fn remove_opponent_security(r: &mut DebugRunner) {
    let card = r.game.players[1].security.last().unwrap().handle();
    assert!(EffectContext::new(&mut r.game, card, None, 1).trash_top_security(1));
    r.game.drain_effect_queue();
}
fn choose(r: &mut DebugRunner, action: u16) {
    let view = r.pending_selection_view().expect("legal unsuspend choice must be exposed");
    assert!(view.valid_action_ids.contains(&action) || (action == PASS && view.is_optional));
    assert!(build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.0);
    r.execute_action(view.selecting_player, action).unwrap();
}
fn restrict(r: &mut DebugRunner, h: PermanentHandle, modifier: ModifierType) {
    r.game.modifiers.add(h, ModifierEntry::simple(modifier, 0, Expiry::Permanent, 0));
}

#[test]
fn accepting_unsuspended_target_is_legal_and_consumes_opt_in_clones() {
    let mut r = setup();
    remove_opponent_security(&mut r);
    assert_eq!(r.pending_selection_view().expect("unsuspended Slayerdramon is eligible").valid_action_ids, vec![ATTACK_START]);
    let frozen = r.game.clone();
    for cloned in [false, true] {
        if cloned { r.game = frozen.clone(); }
        choose(&mut r, ATTACK_START);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        r.game.players[0].battle_area[0].is_suspended = true;
        remove_opponent_security(&mut r);
        assert!(r.pending_selection().is_none(), "accepted no-op spent OPT, including clone resume");
        assert!(r.game.players[0].battle_area[0].is_suspended);
    }
}
#[test]
fn declining_unsuspended_target_preserves_opt_in_clones() {
    let mut r = setup();
    remove_opponent_security(&mut r);
    assert!(r.pending_selection_view().expect("no-op target must be offered").is_optional);
    let frozen = r.game.clone();
    for cloned in [false, true] {
        if cloned { r.game = frozen.clone(); }
        choose(&mut r, PASS);
        r.game.players[0].battle_area[0].is_suspended = true;
        remove_opponent_security(&mut r);
        choose(&mut r, ATTACK_START);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
    }
}
#[test]
fn cannot_suspend_does_not_exclude_a_legal_unsuspend_noop() {
    let mut r = setup();
    restrict(&mut r, PermanentHandle { player: 0, index: 0 }, ModifierType::CannotSuspend);
    remove_opponent_security(&mut r);
    choose(&mut r, ATTACK_START);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn cannot_unsuspend_excludes_both_orientations_and_no_target_preserves_opt() {
    for suspended in [false, true] {
        let mut r = setup();
        let h = PermanentHandle { player: 0, index: 0 };
        r.game.players[0].battle_area[0].is_suspended = suspended;
        restrict(&mut r, h, ModifierType::CannotUnsuspend);
        remove_opponent_security(&mut r);
        assert!(r.pending_selection().is_none(), "prohibited unsuspend is not eligible");
        r.game.modifiers.clear_permanent(h);
        remove_opponent_security(&mut r);
        choose(&mut r, ATTACK_START);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
    }
}
#[test]
fn mixed_target_mask_includes_both_qualifying_orientations_and_excludes_plain_card() {
    let mut r = setup();
    let good = r.place_on_field(0, "EX3-018", None);
    let bad = r.place_on_field(0, "ST1-10", None);
    r.game.players[0].battle_area[good.index as usize].is_suspended = true;
    r.game.players[0].battle_area[bad.index as usize].is_suspended = true;
    remove_opponent_security(&mut r);
    let v = r.pending_selection_view().expect("mixed legal targets");
    assert_eq!(v.valid_action_ids, vec![ATTACK_START, ATTACK_START + good.index as u16]);
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[ATTACK_START as usize] > 0.0);
    assert!(mask[(ATTACK_START + good.index as u16) as usize] > 0.0);
    assert_eq!(mask[(ATTACK_START + bad.index as u16) as usize], 0.0);
}
