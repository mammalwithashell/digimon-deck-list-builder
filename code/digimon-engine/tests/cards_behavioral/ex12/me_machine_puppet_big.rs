//! EX12 [ME] / Machine big-Digimon slice: Metal Empire (EX12-072), Andromon
//! (EX12-055), HiAndromon (EX12-058), Megadramon (EX12-064), Machinedramon
//! (EX12-059), Chaosdramon (EX12-060) and Kapurimon (EX12-003).
//!
//! All boards are synthetic; behaviour is checked against the printed text
//! (card faces), DCGO `EX12_0xx.cs` and CR 4.3.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::support::{
    digimon, field_contains, hand_contains, hand_index, push_to_trash, select_hand_card,
    source_count, trash_contains, DebugRunner,
};

fn me(id: &str, color: CardColor, level: u8, cost: u16, traits: &[&str]) -> CardData {
    digimon(id, color, level, cost, 3000, traits)
}

fn triggered_clauses(r: &DebugRunner, card: &str) -> Vec<(CompiledScope, Vec<CompiledTiming>)> {
    r.compiled_card(card)
        .expect("compiled card")
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some((t.scope, t.when.clone())),
            _ => None,
        })
        .collect()
}

fn reveal_offered(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    out.sort();
    out
}

fn reveal_pick(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let a = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == id)
        })
        .unwrap_or_else(|| panic!("{id} must be a legal reveal pick: {view:?}"));
    r.execute_action(view.selecting_player, a).expect("pick revealed card");
}

fn deck_top(top: &[&'static str]) -> Vec<&'static str> {
    let mut deck: Vec<&str> = vec!["FILL"; 4];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}

// ═════════════════════════ Metal Empire (EX12-072) ═════════════════════════

fn metal_empire_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX12-072")
        .expect("EX12-072 YAML loads")
        .add_card(me("ME-ENABLER", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("ME-LV5", CardColor::Black, 5, 7, &["ME"]))
        .add_card(me("ME-LV6", CardColor::Black, 6, 9, &["ME"]))
        .add_card(me("PLAIN-LV4", CardColor::Black, 4, 5, &["Beast"]))
        .add_card(me("BOTTOM", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("ATTACKER", CardColor::Red, 4, 5, &["Beast"]))
}

#[test]
fn ex12_072_use_requirement_needs_an_me_field_card() {
    let mut r = metal_empire_builder().hand(0, &["EX12-072"]).memory(5).start();
    assert_eq!(
        r.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "<Use Req. ([ME] trait)> without an [ME] Digimon/Tamer cannot ignore the colour"
    );
}

#[test]
fn ex12_072_main_adds_bottom_security_to_hand_and_places_self_face_up_bottom() {
    let mut r = metal_empire_builder()
        .hand(0, &["EX12-072"])
        .security(0, &["BOTTOM"])
        .memory(5)
        .start();
    r.place_on_field(0, "ME-ENABLER", Some(0));
    let slot = hand_index(&r, 0, "EX12-072");
    assert_ne!(r.game.play_option_from_hand(0, slot), OptionPlayResult::Invalid);
    r.auto_resolve().expect("resolve Metal Empire Main");
    assert!(hand_contains(&r, 0, "BOTTOM"));
    assert_eq!(r.game.players[0].security[0].card_id(&r.game.card_data), "EX12-072");
    let idx = r.game.players[0].security[0].card_index;
    assert!(r.game.players[0].face_up_security.contains(&idx), "placed face up");
}

#[test]
fn ex12_072_security_effect_plays_a_level_5_or_lower_me_digimon_from_hand_free() {
    let mut r = metal_empire_builder()
        .hand(1, &["ME-LV5", "ME-LV6", "PLAIN-LV4"])
        .security(1, &["EX12-072"])
        .memory(0)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = r.memory();
    r.attack_player(attacker, 1, false);
    let view = r.pending_selection_view().expect("security effect prompts");
    assert_eq!(view.kind, SelectionKind::Hand);
    // Only the level 5 [ME] Digimon qualifies (level 6 is too high; the Beast is not [ME]).
    let offered = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(offered, 1);
    select_hand_card(&mut r, 1, "ME-LV5");
    r.auto_resolve().expect("resolve Metal Empire security effect");
    assert!(field_contains(&r, 1, "ME-LV5"));
    assert_eq!(r.memory(), memory_before, "played without paying the cost");
}

#[test]
fn ex12_072_face_up_in_security_gives_me_digimon_guard() {
    let mut r = metal_empire_builder().security(0, &["EX12-072"]).start();
    let me_digimon = r.place_on_field(0, "ME-ENABLER", Some(0));
    let other = r.place_on_field(0, "PLAIN-LV4", Some(1));
    assert!(!r.game.has_keyword(me_digimon, Keyword::Guard), "face-down: no aura");
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(me_digimon, Keyword::Guard), "[ME] Digimon gain <Guard>");
    assert!(!r.game.has_keyword(other, Keyword::Guard), "non-[ME] Digimon do not");
}

// ═════════════════════════ Andromon (EX12-055) ═════════════════════════

fn andromon_runner(top: &[&'static str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-055")
        .expect("EX12-055 YAML loads")
        .add_card(me("ME-3", CardColor::Black, 3, 3, &["ME"]))
        .add_card(me("MACH-5", CardColor::Black, 4, 5, &["Machine"]))
        .add_card(me("CYB-6", CardColor::Black, 5, 6, &["Cyborg"]))
        .add_card(me("PLAIN", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["EX12-055"])
        .deck(0, &deck_top(top))
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start()
}

#[test]
fn ex12_055_on_play_may_play_a_cost_5_or_lower_machine_cyborg_me_card_free_and_trashes_the_rest() {
    let mut r = andromon_runner(&["ME-3", "CYB-6", "PLAIN"]);
    let slot = hand_index(&r, 0, "EX12-055");
    let memory_before = r.memory();
    assert!(r.play(0, slot).is_some());
    // CYB-6 costs 6 (> 5) and PLAIN is not [Machine]/[Cyborg]/[ME].
    assert_eq!(reveal_offered(&r), vec!["ME-3".to_string()]);
    reveal_pick(&mut r, "ME-3");
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 0, "ME-3"));
    assert_eq!(memory_before - r.memory(), 7, "only Andromon's own cost 7 is paid");
    assert!(trash_contains(&r, 0, "CYB-6"));
    assert!(trash_contains(&r, 0, "PLAIN"));
}

#[test]
fn ex12_055_on_play_the_play_is_optional_and_the_rest_is_trashed_anyway() {
    let mut r = andromon_runner(&["MACH-5", "PLAIN", "CYB-6"]);
    let slot = hand_index(&r, 0, "EX12-055");
    assert!(r.play(0, slot).is_some());
    assert_eq!(reveal_offered(&r), vec!["MACH-5".to_string()]);
    r.execute_action(0, PASS).expect("decline the free play");
    let _ = r.auto_resolve();
    assert!(!field_contains(&r, 0, "MACH-5"));
    assert!(trash_contains(&r, 0, "MACH-5"));
    assert!(trash_contains(&r, 0, "PLAIN"));
    assert!(trash_contains(&r, 0, "CYB-6"));
}

#[test]
fn ex12_055_printed_digivolve_lines_and_clause_shape() {
    let r = andromon_runner(&["PLAIN", "PLAIN", "PLAIN"]);
    let card = r.compiled_card("EX12-055").unwrap();
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::DnaDigivolve));
    assert!(
        card.alt_paths.iter().filter(|p| p.kind == CompiledAltPathKind::Digivolve).count() >= 2,
        "black Lv.4 / 4 and Lv.4 w/[Machine]/[ME] / 3"
    );
    let clauses = triggered_clauses(&r, "EX12-055");
    assert!(clauses.contains(&(CompiledScope::FaceUp, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving])));
    assert!(clauses.contains(&(CompiledScope::FaceUp, vec![CompiledTiming::Counter])));
    assert!(clauses.contains(&(CompiledScope::Inherited, vec![CompiledTiming::OnOpponentAttack])));
}

// ═════════════════════════ HiAndromon (EX12-058) ═════════════════════════

#[test]
fn ex12_058_reveal_plays_a_cost_7_or_lower_card_and_trashes_the_rest() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-058")
        .expect("EX12-058 YAML loads")
        .add_card(me("MACH-7", CardColor::Black, 5, 7, &["Machine"]))
        .add_card(me("CYB-8", CardColor::Black, 5, 8, &["Cyborg"]))
        .add_card(me("PLAIN", CardColor::Black, 3, 3, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["EX12-058"])
        .deck(0, &deck_top(&["CYB-8", "MACH-7", "PLAIN"]))
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    let slot = hand_index(&r, 0, "EX12-058");
    assert!(r.play(0, slot).is_some());
    assert_eq!(reveal_offered(&r), vec!["MACH-7".to_string()], "cost 8 is too high");
    reveal_pick(&mut r, "MACH-7");
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 0, "MACH-7"));
    assert!(trash_contains(&r, 0, "CYB-8"));
    assert!(trash_contains(&r, 0, "PLAIN"));
}

#[test]
fn ex12_058_gives_all_your_me_digimon_alliance_and_reboot() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-058")
        .expect("EX12-058 YAML loads")
        .add_card(me("ME-4", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("BEAST-4", CardColor::Black, 4, 5, &["Beast"]))
        .start();
    let hi = r.place_on_field(0, "EX12-058", Some(0));
    let ally = r.place_on_field(0, "ME-4", Some(1));
    let beast = r.place_on_field(0, "BEAST-4", Some(2));
    r.game.tick_declarative_effects();
    for kw in [Keyword::Alliance, Keyword::Reboot] {
        assert!(r.game.has_keyword(hi, kw), "HiAndromon is itself an [ME] Digimon");
        assert!(r.game.has_keyword(ally, kw));
        assert!(!r.game.has_keyword(beast, kw));
    }
}

// ═════════════════════════ Megadramon (EX12-064) ═════════════════════════

fn megadramon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-064")
        .expect("EX12-064 YAML loads")
        .add_card(me("OPP-LV3", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("OPP-LV4", CardColor::Red, 4, 5, &["Beast"]))
        .add_card(me("OPP-LV5", CardColor::Red, 5, 7, &["Beast"]))
        .add_card(me("OPP-BASE", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .start()
}

#[test]
fn ex12_064_on_play_deletes_an_opponent_level_4_or_lower_digimon() {
    let mut r = megadramon_runner();
    let mega = r.place_on_field(0, "EX12-064", Some(0));
    let lv4 = r.place_on_field(1, "OPP-LV4", Some(0));
    let lv5 = r.place_stack(1, &["OPP-BASE", "OPP-LV5"]); // bottom -> top: the level 5 card is on top
    r.fire_on_play(0, mega.index as usize);
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    let view = r.pending_selection_view().unwrap();
    // only the level 4 Digimon is a legal target; the level 5 stack is not
    assert!(view.valid_action_ids.iter().filter(|&&a| a != PASS).count() == 1);
    let _ = (lv4, lv5);
    r.auto_resolve().expect("delete the level 4 Digimon");
    assert!(!field_contains(&r, 1, "OPP-LV4"));
    assert!(field_contains(&r, 1, "OPP-LV5"), "the level 5 Digimon is untouched and not de-digivolved");
}

#[test]
fn ex12_064_on_play_de_digivolves_when_nothing_was_deleted() {
    let mut r = megadramon_runner();
    let mega = r.place_on_field(0, "EX12-064", Some(0));
    let big = r.place_stack(1, &["OPP-LV4", "OPP-LV5"]); // bottom -> top: level 5 on top of a level 4 source
    r.fire_on_play(0, mega.index as usize);
    let _ = r.auto_resolve();
    assert_eq!(r.game.players[1].battle_area.len(), 1, "no level 4 or lower Digimon: nothing deleted");
    let top = r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data).to_string();
    assert_eq!(top, "OPP-LV4", "<De-Digivolve 1> instead");
    assert!(trash_contains(&r, 1, "OPP-LV5"));
    let _ = big;
}

#[test]
fn ex12_064_assembly_and_alt_digivolve_are_printed() {
    let r = megadramon_runner();
    let card = r.compiled_card("EX12-064").unwrap();
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::Assembly));
    let clauses = triggered_clauses(&r, "EX12-064");
    assert!(clauses.contains(&(CompiledScope::Inherited, vec![CompiledTiming::EndOfAttack])));
}

// ═════════════════════════ Machinedramon (EX12-059) ═════════════════════════

fn dramon_runner(card: &str) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(card)
        .unwrap_or_else(|e| panic!("{card} YAML loads: {e}"))
        .add_card(me("OPP-LV3", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("OPP-LV4", CardColor::Red, 4, 5, &["Beast"]))
        .add_card(me("OPP-LV5", CardColor::Red, 5, 7, &["Beast"]))
        .add_card(me("OPP-BASE", CardColor::Red, 3, 3, &["Beast"]))
        .add_card(me("ME-A", CardColor::Black, 4, 5, &["ME"]))
        .add_card(me("ME-B", CardColor::Black, 5, 7, &["Machine"]))
        .add_card(me("TOO-BIG", CardColor::Black, 6, 9, &["ME"]))
        .add_card(me("CHEAP-A", CardColor::Red, 3, 2, &["Beast"]))
        .add_card(me("CHEAP-B", CardColor::Red, 3, 1, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, 3, &["Beast"]))
        .hand(0, &["ME-A", "TOO-BIG"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .start()
}

#[test]
fn ex12_059_de_digivolves_3_then_places_two_cards_and_locks_stack_trashing() {
    let mut r = dramon_runner("EX12-059");
    push_to_trash(&mut r, 0, "ME-B");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    let mine = r.place_on_field(0, "FILL", Some(1));
    let opp = r.place_stack(1, &["OPP-BASE", "OPP-LV3", "OPP-LV4", "OPP-LV5"]); // bottom -> top
    let sources_before = source_count(&r, 0, dramon.index as usize);
    r.fire_on_play(0, dramon.index as usize);
    // <De-Digivolve 3> 1 of your opponent's Digimon (the only one: auto/mandatory pick)
    if r.pending_kind() == Some(SelectionKind::OppField) {
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("pick the de-digivolve target");
    }
    // <De-Digivolve 3> is "up to 3": answer the §16-12-1 count declaration with 3.
    r.declare_max_count().expect("de-digivolve count declaration");
    // Two placements: ME-A from the hand, ME-B from the trash (TOO-BIG is level 6).
    for _ in 0..2 {
        assert!(r.pending_selection_view().is_some(), "expected a placement pick");
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("place a card");
    }
    let _ = r.auto_resolve();
    assert_eq!(source_count(&r, 0, dramon.index as usize), sources_before + 2);
    for perm in [dramon, mine] {
        assert!(
            r.modifiers().has(perm, ModifierType::ImmuneFromStackTrashing),
            "opponent effects can't trash any of your Digimon's stacked cards"
        );
    }
    assert!(hand_contains(&r, 0, "TOO-BIG"), "level 6 can not be placed");
    // <De-Digivolve 3> trashes the level 5 and level 4 cards and stops at the level 3 card
    // ("You can't trash past level 3 cards").
    assert_eq!(
        r.game.players[1].battle_area[0].top_card().card_id(&r.game.card_data),
        "OPP-LV3"
    );
    assert!(trash_contains(&r, 1, "OPP-LV5"));
    assert!(trash_contains(&r, 1, "OPP-LV4"));
    let _ = opp;
}

#[test]
fn ex12_059_declining_the_placement_installs_no_protection() {
    let mut r = dramon_runner("EX12-059");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    r.place_on_field(1, "OPP-LV3", Some(0));
    r.fire_on_play(0, dramon.index as usize);
    if r.pending_kind() == Some(SelectionKind::OppField) {
        let view = r.pending_selection_view().unwrap();
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("pick the de-digivolve target");
    }
    // <De-Digivolve 3> is "up to 3": answer the §16-12-1 count declaration with 3.
    r.declare_max_count().expect("de-digivolve count declaration");
    // Only ONE qualifying card (ME-A) in hand+trash: the placement needs 2, so it never prompts.
    let _ = r.auto_resolve();
    assert!(!r.modifiers().has(dramon, ModifierType::ImmuneFromStackTrashing));
}

#[test]
fn ex12_059_keywords_and_ace_overflow_are_printed() {
    let mut r = dramon_runner("EX12-059");
    let dramon = r.place_on_field(0, "EX12-059", Some(0));
    assert!(r.game.has_keyword(dramon, Keyword::Reboot));
    assert!(r.game.has_keyword(dramon, Keyword::Fragment(2)));
    assert!(r.game.has_keyword(dramon, Keyword::BlastDigivolve));
}

// ═════════════════════════ Chaosdramon (EX12-060) ═════════════════════════

#[test]
fn ex12_060_printed_keywords_dna_and_assembly() {
    let mut r = dramon_runner("EX12-060");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    assert!(r.game.has_keyword(chaos, Keyword::Piercing));
    assert!(r.game.has_keyword(chaos, Keyword::Fragment(2)));
    assert!(r.game.has_keyword(chaos, Keyword::Engage));
    assert!(r.game.has_keyword(chaos, Keyword::SecurityAttackPlus(1)));
    let card = r.compiled_card("EX12-060").unwrap();
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::DnaDigivolve));
    assert!(card.alt_paths.iter().any(|p| p.kind == CompiledAltPathKind::Assembly));
}

#[test]
fn ex12_060_de_digivolves_all_then_deletes_two_by_play_cost_after_placing_two() {
    let mut r = dramon_runner("EX12-060");
    push_to_trash(&mut r, 0, "ME-B");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    r.place_on_field(1, "CHEAP-A", Some(0));
    r.place_on_field(1, "CHEAP-B", Some(1));
    r.place_on_field(1, "OPP-LV5", Some(2));
    r.fire_on_play(0, chaos.index as usize);
    for _ in 0..2 {
        let view = r.pending_selection_view().expect("placement pick");
        let a = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
        r.execute_action(0, a).expect("place a card");
    }
    // Two digivolution cards under Chaosdramon: Digimon with play cost <= 2 are deleted (2 of them).
    let _ = r.auto_resolve();
    assert_eq!(source_count(&r, 0, chaos.index as usize), 2);
    assert!(!field_contains(&r, 1, "CHEAP-A"));
    assert!(!field_contains(&r, 1, "CHEAP-B"));
    assert!(field_contains(&r, 1, "OPP-LV5"), "play cost 7 > 2 is untouched");
}

#[test]
fn ex12_060_declining_the_placement_deletes_nothing() {
    let mut r = dramon_runner("EX12-060");
    push_to_trash(&mut r, 0, "ME-B");
    let chaos = r.place_on_field(0, "EX12-060", Some(0));
    r.place_on_field(1, "CHEAP-A", Some(0));
    r.fire_on_play(0, chaos.index as usize);
    let view = r.pending_selection_view().expect("first placement pick");
    r.execute_action(0, PASS).expect("decline the placement");
    let _ = view;
    let _ = r.auto_resolve();
    assert!(field_contains(&r, 1, "CHEAP-A"), "no placement, no deletion");
    assert_eq!(source_count(&r, 0, chaos.index as usize), 0);
}

// ═════════════════════════ Kapurimon (EX12-003) ═════════════════════════

#[test]
fn ex12_003_is_an_inherited_would_leave_replacement() {
    let r = DebugRunner::builder()
        .dsl_card("EX12-003")
        .expect("EX12-003 YAML loads")
        .start();
    let card = r.compiled_card("EX12-003").expect("compiled");
    assert!(
        card.effects.iter().any(|e| matches!(e, CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. }))),
        "[All Turns] 'would leave the battle area' is a replacement-window clause"
    );
}
