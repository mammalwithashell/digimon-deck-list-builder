//! Training-surface check for the zone-card selections added for BT26-060,
//! BT26-081 and BT26-102: an agent that only ever picks from the RL action mask
//! (`build_action_mask`) can drive every prompt to completion, and the mask
//! offers exactly the prompt's choices (+ PASS when optional).

use super::support::*;
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;

/// Answer every pending selection from the mask alone. `pick` chooses among
/// the legal non-PASS ids (so different runs exercise different branches).
fn drive_by_mask(r: &mut DebugRunner, pick: fn(&[u16]) -> u16) -> usize {
    let mut decisions = 0;
    while let Some(sel) = r.game.pending_selection.as_ref() {
        let p = sel.selecting_player;
        let mask = build_action_mask(&r.game, p);
        let legal: Vec<u16> = (0..mask.len() as u16).filter(|&a| mask[a as usize] > 0.0).collect();
        let mut expected: Vec<u16> = sel.valid_action_ids.clone();
        if sel.is_optional && !expected.contains(&PASS) {
            expected.push(PASS);
        }
        expected.sort_unstable();
        expected.dedup();
        assert_eq!(legal, expected, "mask == the prompt's choices ({})", sel.prompt);
        let non_pass: Vec<u16> = legal.iter().copied().filter(|&a| a != PASS).collect();
        let a = if non_pass.is_empty() { PASS } else { pick(&non_pass) };
        r.execute_action(p, a).unwrap();
        decisions += 1;
        assert!(decisions < 60, "selection loop did not terminate");
    }
    decisions
}

fn first(ids: &[u16]) -> u16 {
    ids[0]
}

fn last(ids: &[u16]) -> u16 {
    ids[ids.len() - 1]
}

#[test]
fn bt26_060_return_order_is_fully_mask_driven() {
    for pick in [first as fn(&[u16]) -> u16, last] {
        let mut r = DebugRunner::builder()
            .dsl_card("BT26-060")
            .unwrap()
            .add_card(filler("FILLER"))
            .add_card(digimon("A", "A", CardColor::Blue, 3, 3, &[]))
            .add_card(digimon("B", "B", CardColor::Blue, 4, 4, &[]))
            .deck(0, &["FILLER"; 6])
            .deck(1, &["FILLER"; 6])
            .start();
        r.set_first_player(0);
        let me = r.place_on_field(0, "BT26-060", Some(0));
        r.place_stack(1, &["A", "B", "A", "B"]);
        r.place_stack(1, &["B", "A", "B"]);
        let deck_before = r.game.players[1].deck.len();
        fire(&mut r, EffectTiming::OnPlay, me);
        let n = drive_by_mask(&mut r, pick);
        assert!(n >= 2 + 5, "2 Digimon picks + 5 order picks (+ the optional delete)");
        assert_eq!(r.game.players[1].deck.len(), deck_before + 5);
    }
}

#[test]
fn bt26_081_budget_play_is_fully_mask_driven() {
    for pick in [first as fn(&[u16]) -> u16, last] {
        let mut r = DebugRunner::builder()
            .dsl_card("BT26-081")
            .unwrap()
            .add_card(filler("FILLER"))
            .add_card(digimon("IL3", "Il3", CardColor::Yellow, 3, 3, &["Iliad"]))
            .add_card(digimon("IL4", "Il4", CardColor::Yellow, 4, 4, &["Iliad"]))
            .add_card(digimon("IL5", "Il5", CardColor::Yellow, 4, 5, &["Iliad"]))
            .add_card(digimon("OPP", "Opp", CardColor::Red, 9, 9, &[]))
            .deck(0, &["FILLER"; 6])
            .deck(1, &["FILLER"; 6])
            .start();
        r.set_first_player(0);
        r.game.players[0].hand.clear();
        push_hand(&mut r, 0, "IL3");
        push_hand(&mut r, 0, "IL5");
        push_trash(&mut r, 0, "IL4");
        push_trash(&mut r, 0, "IL5");
        let me = r.place_on_field(0, "BT26-081", Some(0));
        r.place_on_field(1, "OPP", Some(0));
        fire(&mut r, EffectTiming::OnPlay, me);
        drive_by_mask(&mut r, pick);
        let played: i32 = field_ids(&r, 0)
            .iter()
            .filter(|id| id.starts_with("IL"))
            .map(|id| id[2..].parse::<i32>().unwrap())
            .sum();
        assert!(played > 0 && played <= 8, "within the 8-cost budget: {played}");
    }
}

#[test]
fn bt26_102_materials_are_fully_mask_driven() {
    for pick in [first as fn(&[u16]) -> u16, last] {
        let mut r = DebugRunner::builder()
            .dsl_card("BT26-102")
            .unwrap()
            .add_card(filler("FILLER"))
            .add_card(digimon("SC", "Seven", CardColor::White, 5, 5, &["Seven Code"]))
            .deck(0, &["FILLER"; 6])
            .deck(1, &["FILLER"; 6])
            .memory(10)
            .start();
        r.set_first_player(0);
        r.game.players[0].hand.clear();
        let host = r.place_on_field(0, "SC", Some(0));
        r.place_on_field(0, "SC", Some(0));
        let data_idx = r.game.card_data.iter().position(|c| c.card_id == "SC").unwrap();
        let next = r.game.next_card_index();
        r.game.players[0].battle_area[host.index as usize]
            .linked_cards
            .push(CardSource::new(data_idx, 0, next));
        for _ in 0..5 {
            push_trash(&mut r, 0, "SC");
        }
        push_hand(&mut r, 0, "BT26-102");
        r.game.enter_main_phase();
        let _ = r.game.play_option_from_hand(0, 0);
        drive_by_mask(&mut r, pick);
        let biggest = r.game.players[0]
            .battle_area
            .iter()
            .map(|p| p.card_sources.len())
            .max()
            .unwrap();
        assert_eq!(biggest, 7, "6 materials placed under one host");
        let _ = PermanentHandle { player: 0, index: 0 };
    }
}
