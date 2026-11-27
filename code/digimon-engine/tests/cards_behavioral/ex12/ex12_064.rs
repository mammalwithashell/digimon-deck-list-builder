#![allow(unused_imports)]
use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

use super::me_machine_puppet_big::*;
use super::support::{
    digimon, field_contains, hand_contains, hand_index, push_to_trash, select_hand_card,
    source_count, trash_contains, DebugRunner,
};

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

