//! DNA candidate predicates for the two DNA shapes `dna_field_pair` /
//! `dna_hand_candidate` did not cover.
//!
//! * `dna_field_pair { role: result, first, second, cost }`
//!   (G-DSL-DNA-FIELD-PAIR-FIXED-MATERIALS): "This Digimon and 1 of your other
//!   Digimon may DNA digivolve into X in the hand" (BanchoLeomon BT20-036,
//!   EX3-008) binds both materials BEFORE the result. The result card must fit
//!   exactly that pair: a hand card whose only legal pair is two other
//!   Digimon would be a pick the commit refuses (CR 4.3 1-3-2). DCGO
//!   `CanPlayJogress(true)` / `CanJogressFromTargetPermanents`.
//! * `dna_trash_candidate` (G-DSL-DNA-TRASH-PARTNER-CANDIDATE): "1 of your
//!   [Machinedramon] in play and 1 [Kimeramon] in the trash may DNA digivolve
//!   into [Millenniummon] in the hand" (BT18-015 / BT18-073) picks the hand
//!   result first, then the field material, then the trash material. Each
//!   stage offers only cards that complete a recipe-satisfying, payable
//!   (field, trash) pair, the checks
//!   `effect_initiated_dna_digivolve_trash_partner` makes at commit.
use digimon_engine::{
    action::space::{encode_attack, PASS, PLAY_HAND_START, TRASH_EFFECT_START},
    card_data::CardData,
    debug_runner::{make_test_card, make_test_dna_card, DebugRunner},
    enums::{CardKind, Expiry, GamePhase, ModifierType},
    modifiers::PlayerModifierEntry,
    permanent::PermanentHandle,
};

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c
}

fn dna(id: &str, a: u8, b: u8, cost: i16) -> CardData {
    let mut c = make_test_dna_card(id, id, a, b, cost);
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c
}

/// The non-PASS action ids of the open selection, sorted.
fn offered(r: &DebugRunner) -> Vec<u16> {
    let mut ids: Vec<u16> = r
        .game
        .pending_selection
        .as_ref()
        .map(|p| p.valid_action_ids.iter().copied().filter(|&id| id != PASS).collect())
        .unwrap_or_default();
    ids.sort();
    ids
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

// ─── Fixed-pair result (BT20-036 / EX3-008 shape) ───────────────────────────

const FIXED_PAIR: &str = r#"
card: FIXED-PAIR-SOURCE
name: Fixed pair source
kind: digimon
level: 6
color: [white]
cost: 0
dp: 5000
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: dna_partner
          optional: true
          filter:
            all_of:
              - kind: digimon
              - other: true
          prompt: Choose your other DNA material
      - select_hand:
          of: you
          bind_as: dna_result
          optional: true
          filter:
            all_of:
              - kind: digimon
              - dna_field_pair: { role: result, first: source, second: dna_partner }
          prompt: DNA digivolve into a card in your hand
      - effect_initiated_dna_digivolve:
          target_a: source
          target_b: dna_partner
          from_hand: dna_result
          cost: printed
"#;

/// Field: [source Lv.6, Lv.5, Lv.4]. Hand: R65 (Lv.6 + Lv.5, cost 3), R64
/// (Lv.6 + Lv.4, cost 3), R54 (Lv.5 + Lv.4, cost 3), R65X (Lv.6 + Lv.5,
/// cost 20: more than 3 memory can pay, the floor being -10).
fn fixed_pair(yaml: &str) -> (DebugRunner, [PermanentHandle; 3]) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon("F5", 5))
        .add_card(digimon("F4", 4))
        .add_card(dna("R65", 6, 5, 3))
        .add_card(dna("R64", 6, 4, 3))
        .add_card(dna("R54", 5, 4, 3))
        .add_card(dna("R65X", 6, 5, 20))
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["F4"; 20])
        .deck(1, &["F4"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    for id in ["R65", "R64", "R54", "R65X"] {
        r.add_to_hand(0, id);
    }
    let source = r.place_on_field(0, "FIXED-PAIR-SOURCE", None);
    let f5 = r.place_on_field(0, "F5", None);
    let f4 = r.place_on_field(0, "F4", None);
    r.fire_on_play(source.player, source.index as usize);
    (r, [source, f5, f4])
}

fn pick_partner(r: &mut DebugRunner, partner: PermanentHandle) {
    let id = encode_attack(0, partner.index as u16);
    assert!(offered(r).contains(&id), "partner {partner:?} is offered");
    r.execute_action(0, id).unwrap();
}

#[test]
fn fixed_pair_result_offers_only_cards_whose_recipe_that_pair_satisfies() {
    let (mut r, [_, f5, _]) = fixed_pair(FIXED_PAIR);
    pick_partner(&mut r, f5);
    assert_eq!(
        offered(&r),
        vec![PLAY_HAND_START],
        "only R65: R64 needs a Lv.4, R54's pair (Lv.5 + Lv.4) excludes the source, R65X is unpayable"
    );
}

#[test]
fn fixed_pair_result_follows_the_chosen_partner() {
    let (mut r, [_, _, f4]) = fixed_pair(FIXED_PAIR);
    pick_partner(&mut r, f4);
    assert_eq!(offered(&r), vec![PLAY_HAND_START + 1], "only R64 (Lv.6 + Lv.4)");
}

#[test]
fn unfixed_result_stage_still_accepts_any_field_pair() {
    // Control: the historical `role: result` asks only whether SOME pair
    // exists, which is why the fixed form is needed.
    let yaml = FIXED_PAIR.replace(
        "dna_field_pair: { role: result, first: source, second: dna_partner }",
        "dna_field_pair: { role: result }",
    );
    let (mut r, [_, f5, _]) = fixed_pair(&yaml);
    pick_partner(&mut r, f5);
    assert_eq!(offered(&r), vec![PLAY_HAND_START, PLAY_HAND_START + 1, PLAY_HAND_START + 2]);
}

#[test]
fn fixed_pair_cost_mirrors_a_free_dna_step() {
    let yaml = FIXED_PAIR
        .replace(
            "second: dna_partner }",
            "second: dna_partner, cost: free }",
        )
        .replace("cost: printed", "cost: free");
    let (mut r, [_, f5, _]) = fixed_pair(&yaml);
    pick_partner(&mut r, f5);
    assert_eq!(
        offered(&r),
        vec![PLAY_HAND_START, PLAY_HAND_START + 3],
        "free: the cost-20 R65X is offered too"
    );
}

#[test]
fn fixed_pair_commit_digivolves_the_two_bound_materials() {
    let (mut r, [source, f5, f4]) = fixed_pair(FIXED_PAIR);
    pick_partner(&mut r, f5);
    r.execute_action(0, PLAY_HAND_START).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 0, "printed DNA cost 3 paid");
    let tops: Vec<String> = (0..r.battle_area_size(0))
        .map(|i| top_id(&r, PermanentHandle { player: 0, index: i as u8 }))
        .collect();
    assert!(tops.contains(&"R65".to_string()), "{tops:?}");
    assert!(tops.contains(&"F4".to_string()), "the unchosen Lv.4 stays: {tops:?}");
    assert!(!tops.contains(&"F5".to_string()));
    let _ = (source, f4);
}

#[test]
fn declined_partner_leaves_no_result_to_choose() {
    let (mut r, _) = fixed_pair(FIXED_PAIR);
    r.execute_action(0, PASS).unwrap();
    assert!(
        r.game.pending_selection.is_none(),
        "with no bound partner no hand card fits the fixed pair"
    );
}

// ─── Field + trash DNA (BT18-015 / BT18-073 shape) ──────────────────────────

const TRASH_DNA: &str = r#"
card: TRASH-DNA-SOURCE
name: Trash DNA source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter:
            all_of:
              - kind: digimon
              - dna_trash_candidate: { role: result, cost: printed }
          prompt: Choose the DNA result in your hand
      - if:
          condition: { binding_present: result }
          then:
            - select_own_permanent:
                bind_as: field_partner
                filter:
                  all_of:
                    - kind: digimon
                    - dna_trash_candidate: { role: field, result: result, cost: printed }
                prompt: Choose the DNA material in play
            - select_trash:
                of: you
                bind_as: trash_partner
                filter:
                  all_of:
                    - kind: digimon
                    - dna_trash_candidate: { role: partner, result: result, field: field_partner, cost: printed }
                prompt: Choose the DNA material in your trash
            - effect_initiated_dna_digivolve_trash_partner:
                target: field_partner
                trash_partner: trash_partner
                from_hand: result
                cost: printed
                ignore_requirements: false
"#;

/// Field: [source Tamer, F6 (Lv.6), F4 (Lv.4)]. Trash: [T5 (Lv.5), T3
/// (Lv.3)]. Hand: [R65 (6+5, cost 3), R43 (4+3, cost 3), R77 (7+7), R65X
/// (6+5, cost 20)].
fn trash_dna(yaml: &str) -> (DebugRunner, [PermanentHandle; 3]) {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon("F6", 6))
        .add_card(digimon("F4", 4))
        .add_card(digimon("T5", 5))
        .add_card(digimon("T3", 3))
        .add_card(dna("R65", 6, 5, 3))
        .add_card(dna("R43", 4, 3, 3))
        .add_card(dna("R77", 7, 7, 0))
        .add_card(dna("R65X", 6, 5, 20))
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["F4"; 20])
        .deck(1, &["F4"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    for id in ["R65", "R43", "R77", "R65X"] {
        r.add_to_hand(0, id);
    }
    r.inject_trash(0, "T5");
    r.inject_trash(0, "T3");
    let source = r.place_on_field(0, "TRASH-DNA-SOURCE", None);
    let f6 = r.place_on_field(0, "F6", None);
    let f4 = r.place_on_field(0, "F4", None);
    (r, [source, f6, f4])
}

fn trash_index(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap() as u16
}

#[test]
fn trash_dna_result_offers_only_cards_with_a_payable_field_and_trash_pair() {
    let (mut r, [source, _, _]) = trash_dna(TRASH_DNA);
    r.fire_on_play(source.player, source.index as usize);
    assert_eq!(
        offered(&r),
        vec![PLAY_HAND_START, PLAY_HAND_START + 1],
        "R65 (F6 + T5) and R43 (F4 + T3); R77 has no pair, R65X is unpayable"
    );
}

#[test]
fn trash_dna_field_and_partner_stages_are_correlated() {
    let (mut r, [source, f6, _]) = trash_dna(TRASH_DNA);
    r.fire_on_play(source.player, source.index as usize);
    r.execute_action(0, PLAY_HAND_START).unwrap();
    assert_eq!(
        offered(&r),
        vec![encode_attack(0, f6.index as u16)],
        "for R65 only the Lv.6 in play has a Lv.5 trash partner"
    );
    r.execute_action(0, encode_attack(0, f6.index as u16)).unwrap();
    let t5 = trash_index(&r, "T5");
    assert_eq!(offered(&r), vec![TRASH_EFFECT_START + t5], "only the Lv.5 in the trash");
    r.execute_action(0, TRASH_EFFECT_START + t5).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, f6), "R65");
    assert_eq!(r.memory(), 0, "printed DNA cost 3 paid");
    assert!(r.game.players[0]
        .trash
        .iter()
        .all(|c| c.card_id(&r.game.card_data) != "T5"));
}

#[test]
fn trash_dna_offers_nothing_while_digivolving_by_effect_is_forbidden() {
    let (mut r, [source, _, _]) = trash_dna(TRASH_DNA);
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(
            ModifierType::CannotDigivolveDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
    r.fire_on_play(source.player, source.index as usize);
    assert!(r.game.pending_selection.is_none(), "the commit would refuse every pair");
}

#[test]
fn trash_dna_free_cost_admits_the_expensive_result() {
    let yaml = TRASH_DNA.replace("cost: printed", "cost: free");
    let (mut r, [source, _, _]) = trash_dna(&yaml);
    r.fire_on_play(source.player, source.index as usize);
    assert_eq!(offered(&r), vec![PLAY_HAND_START, PLAY_HAND_START + 1, PLAY_HAND_START + 3]);
}
