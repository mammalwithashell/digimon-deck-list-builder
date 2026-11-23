//! EX12 [ME] / Machine / Puppet slice: Hagurumon (EX12-053), Guardromon
//! (EX12-054), ToyAgumon (EX12-008), Kokuwamon (EX12-038), Thundermon
//! (EX12-041), and the Kokuwamon (EX13-046) reveal that must reach Thundermon
//! (EX12-041) through its `(Rule) Name: Treated as including [Mamemon]`.
//!
//! All boards are synthetic; card behaviour is checked against the printed text
//! (data/cards.json), DCGO `EX12_0xx.cs` and CR 4.3.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

use super::support::{
    digimon, hand_contains, hand_ids, select_hand_card, trash_contains, DebugRunner,
};

fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
    f(p)
        || p.all_of.iter().any(|q| pred_any(q, f))
        || p.any_of.iter().any(|q| pred_any(q, f))
        || p.none_of.iter().any(|q| pred_any(q, f))
        || p.not.as_ref().is_some_and(|q| pred_any(q, f))
}

fn me(id: &str, color: CardColor, level: u8, traits: &[&str]) -> CardData {
    digimon(id, color, level, if level <= 3 { 3 } else { 5 }, 3000, traits)
}

fn hand_slot(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in P0's hand"))
}

/// Digivolve hand card `id` onto field slot `field_index`; returns the memory
/// it cost.
fn digivolve_cost(r: &mut DebugRunner, id: &str, field_index: usize) -> Option<i16> {
    let slot = hand_slot(r, id);
    let before = r.memory();
    r.game
        .digivolve_from_hand(0, slot, field_index, PlaySource::ByHand)
        .then(|| before - r.memory())
}

fn deck_with_top<'a>(top: &[&'a str], filler: &'a str, n: usize) -> Vec<&'a str> {
    let mut deck: Vec<&str> = vec![filler; n];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
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

fn pick_revealed(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
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
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

fn deck_bottom_ids(r: &DebugRunner, n: usize) -> Vec<String> {
    r.game.players[0].deck[..n]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn assert_me_alt_path(r: &DebugRunner, card_id: &str, level: u8, cost: i32) {
    let card = r.compiled_card(card_id).expect("compiled card");
    let path = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.cost == Some(CompiledCost::Literal(cost))
                && p.from.as_ref().is_some_and(|f| {
                    pred_any(f, |q| q.trait_has.as_deref() == Some("ME"))
                        && pred_any(f, |q| q.level_eq == Some(level))
                })
        })
        .unwrap_or_else(|| panic!("{card_id}: printed `Lv.{level} w/[ME] trait: Cost {cost}` path"));
    assert!(path.from.is_some());
}

// ═════════════════════════ Hagurumon (EX12-053) ═════════════════════════

fn hagurumon_runner(top: &[&str]) -> DebugRunner {
    let deck = deck_with_top(top, "FILL", 4);
    DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("DUAL", CardColor::Black, 4, &["Machine", "ME"]))
        .add_card(me("MACH", CardColor::Black, 4, &["Machine"]))
        .add_card(me("CYB", CardColor::Black, 4, &["Cyborg"]))
        .add_card(me("ME-ONLY", CardColor::Black, 4, &["ME"]))
        .add_card(me("PLAIN", CardColor::Black, 4, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX12-053"])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

fn play_hagurumon(r: &mut DebugRunner) {
    let slot = hand_slot(r, "EX12-053");
    assert!(r.play(0, slot).is_some(), "Hagurumon should be playable");
}

#[test]
fn ex12_053_on_play_adds_one_machine_cyborg_mutant_and_one_me_card() {
    let mut r = hagurumon_runner(&["DUAL", "MACH", "ME-ONLY"]);
    play_hagurumon(&mut r);
    assert_eq!(
        r.pending_kind().map(|k| format!("{k:?}")).unwrap_or_default().split(' ').next().unwrap(),
        "RevealBucket"
    );
    assert_eq!(offered_reveal_ids(&r), vec!["DUAL".to_string(), "MACH".to_string()]);
    pick_revealed(&mut r, "DUAL");
    // One card never fills both slots: DUAL is out of the [ME] bucket.
    assert_eq!(offered_reveal_ids(&r), vec!["ME-ONLY".to_string()]);
    pick_revealed(&mut r, "ME-ONLY");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["DUAL".to_string(), "ME-ONLY".to_string()]);
    assert_eq!(deck_bottom_ids(&r, 1), vec!["MACH".to_string()], "rest to the deck bottom");
    assert_eq!(r.deck_size(0), 5);
}

#[test]
fn ex12_053_a_machine_and_me_card_fills_exactly_one_slot() {
    let mut r = hagurumon_runner(&["DUAL", "PLAIN", "FILL"]);
    play_hagurumon(&mut r);
    assert_eq!(offered_reveal_ids(&r), vec!["DUAL".to_string()]);
    pick_revealed(&mut r, "DUAL");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DUAL".to_string()]);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn ex12_053_cyborg_and_mutant_count_for_the_first_slot() {
    let mut r = hagurumon_runner(&["CYB", "ME-ONLY", "PLAIN"]);
    play_hagurumon(&mut r);
    assert_eq!(offered_reveal_ids(&r), vec!["CYB".to_string()]);
    pick_revealed(&mut r, "CYB");
    assert_eq!(offered_reveal_ids(&r), vec!["ME-ONLY".to_string()]);
    pick_revealed(&mut r, "ME-ONLY");
    let _ = r.auto_resolve();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["CYB".to_string(), "ME-ONLY".to_string()]);
}

#[test]
fn ex12_053_no_match_returns_all_three_to_the_bottom() {
    let mut r = hagurumon_runner(&["PLAIN", "FILL", "PLAIN"]);
    play_hagurumon(&mut r);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

#[test]
fn ex12_053_digivolves_from_a_level_2_me_digimon_for_free() {
    // Red Lv.2 [ME]: not the printed black circle, so only the `Lv.2 w/[ME]
    // trait: Cost 0` line can allow this.
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("RED-ME-2", CardColor::Red, 2, &["ME"]))
        .add_card(me("RED-PLAIN-2", CardColor::Red, 2, &["Beast"]))
        .hand(0, &["EX12-053", "EX12-053"])
        .memory(5)
        .start();
    let me_base = r.place_on_field(0, "RED-ME-2", Some(0));
    let plain_base = r.place_on_field(0, "RED-PLAIN-2", Some(1));
    assert_me_alt_path(&r, "EX12-053", 2, 0);
    assert_eq!(digivolve_cost(&mut r, "EX12-053", me_base.index as usize), Some(0));
    r.auto_resolve().ok();
    assert_eq!(
        digivolve_cost(&mut r, "EX12-053", plain_base.index as usize),
        None,
        "a non-[ME] red Lv.2 is not a legal base"
    );
}

#[test]
fn ex12_053_inherited_grants_blocker() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-053")
        .expect("EX12-053 YAML loads")
        .add_card(me("CARRIER", CardColor::Black, 4, &["Machine"]))
        .start();
    let carrier = r.place_stack(0, &["EX12-053", "CARRIER"]);
    assert!(r.game.has_keyword(carrier, Keyword::Blocker));
}

// ═════════════════════════ Guardromon (EX12-054) ═════════════════════════

fn guardromon_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .add_card(me("MACH", CardColor::Black, 3, &["Machine"]))
        .add_card(me("CYB", CardColor::Black, 3, &["Cyborg"]))
        .add_card(me("ME-ONLY", CardColor::Black, 3, &["ME"]))
        .add_card(me("MUTANT", CardColor::Black, 3, &["Mutant"]))
        .add_card(me("PLAIN", CardColor::Black, 3, &["Beast"]))
        .add_card(me("DRAW", CardColor::Black, 3, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 6])
        .memory(10)
        .start()
}

#[test]
fn ex12_054_on_play_trashes_a_machine_cyborg_or_me_card_to_draw_2() {
    let mut r = guardromon_runner(&["EX12-054", "CYB", "PLAIN", "MUTANT"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    // [Machine]/[Cyborg]/[ME] only: [Mutant] and plain cards are not offered.
    let view = r.pending_selection_view().unwrap();
    let offered: usize = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(offered, 1, "only the [Cyborg] card qualifies");
    select_hand_card(&mut r, 0, "CYB");
    r.auto_resolve().expect("resolve Guardromon On Play");
    assert!(trash_contains(&r, 0, "CYB"));
    assert_eq!(r.hand_size(0), 2 + 2, "PLAIN + MUTANT kept, drew 2");
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex12_054_declining_the_trash_draws_nothing() {
    let mut r = guardromon_runner(&["EX12-054", "ME-ONLY"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    r.execute_action(0, PASS).expect("decline the optional trash");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "ME-ONLY"));
    assert_eq!(r.deck_size(0), 6, "no draw without the trash");
}

#[test]
fn ex12_054_no_qualifying_hand_card_skips_the_effect() {
    let mut r = guardromon_runner(&["EX12-054", "PLAIN", "MUTANT"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn ex12_054_when_digivolving_trigger_and_me_alt_path() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .add_card(me("RED-ME-3", CardColor::Red, 3, &["ME"]))
        .add_card(me("MACH", CardColor::Black, 3, &["Machine"]))
        .add_card(me("DRAW", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX12-054", "MACH"])
        .deck(0, &["DRAW"; 4])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-054", 3, 2);
    let base = r.place_on_field(0, "RED-ME-3", Some(0));
    assert_eq!(digivolve_cost(&mut r, "EX12-054", base.index as usize), Some(2));
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand), "[When Digivolving] cost pick");
    select_hand_card(&mut r, 0, "MACH");
    r.auto_resolve().expect("resolve Guardromon When Digivolving");
    assert!(trash_contains(&r, 0, "MACH"));
    assert!(r.deck_size(0) <= 2, "<Draw 2> drew after the trash (plus the digivolution draw)");
}

#[test]
fn ex12_054_has_blocker() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .start();
    let g = r.place_on_field(0, "EX12-054", Some(0));
    assert!(r.game.has_keyword(g, Keyword::Blocker));
}

// ═════════════════════════ ToyAgumon (EX12-008) ═════════════════════════

fn toy_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("PUPPET", CardColor::Red, 3, &["Puppet"]))
        .add_card(me("ME-ONLY", CardColor::Red, 3, &["ME"]))
        .add_card(me("PLAIN", CardColor::Red, 3, &["Beast"]))
        .add_card(me("DRAW", CardColor::Red, 3, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 4])
        .memory(2)
        .start()
}

#[test]
fn ex12_008_start_of_main_trashes_puppet_or_me_to_draw_and_gain_memory() {
    let mut r = toy_runner(&["PUPPET", "PLAIN"]);
    r.place_on_field(0, "EX12-008", Some(0));
    let memory_before = r.memory();
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    select_hand_card(&mut r, 0, "PUPPET");
    r.auto_resolve().expect("resolve ToyAgumon Start of Main");
    assert!(trash_contains(&r, 0, "PUPPET"));
    assert!(hand_contains(&r, 0, "PLAIN"));
    assert_eq!(r.hand_size(0), 2, "PLAIN + 1 drawn");
    assert_eq!(r.deck_size(0), 3);
    assert_eq!(r.memory(), memory_before + 1);
}

#[test]
fn ex12_008_start_of_main_can_trash_an_me_card_and_can_be_declined() {
    let mut r = toy_runner(&["ME-ONLY"]);
    r.place_on_field(0, "EX12-008", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "ME-ONLY"));
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex12_008_no_puppet_or_me_in_hand_does_nothing() {
    let mut r = toy_runner(&["PLAIN"]);
    r.place_on_field(0, "EX12-008", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), None);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn ex12_008_digivolves_from_a_level_2_me_digimon_for_free() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("BLUE-ME-2", CardColor::Blue, 2, &["ME"]))
        .add_card(me("BLUE-PLAIN-2", CardColor::Blue, 2, &["Beast"]))
        .hand(0, &["EX12-008", "EX12-008"])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-008", 2, 0);
    let me_base = r.place_on_field(0, "BLUE-ME-2", Some(0));
    let plain_base = r.place_on_field(0, "BLUE-PLAIN-2", Some(1));
    assert_eq!(digivolve_cost(&mut r, "EX12-008", me_base.index as usize), Some(0));
    r.auto_resolve().ok();
    assert_eq!(digivolve_cost(&mut r, "EX12-008", plain_base.index as usize), None);
}

#[test]
fn ex12_008_inherited_grants_raid() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("CARRIER", CardColor::Red, 4, &["Puppet"]))
        .start();
    let carrier = r.place_stack(0, &["EX12-008", "CARRIER"]);
    assert!(r.game.has_keyword(carrier, Keyword::Raid));
}

// ═════════════════════════ Kokuwamon (EX12-038) ═════════════════════════

fn kokuwamon_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-038")
        .expect("EX12-038 YAML loads")
        .add_card(me("MUTANT", CardColor::Yellow, 3, &["Mutant"]))
        .add_card(me("ME-ONLY", CardColor::Yellow, 3, &["ME"]))
        .add_card(me("MACH", CardColor::Yellow, 3, &["Machine"]))
        .add_card(me("DRAW", CardColor::Yellow, 3, &["Beast"]))
        .add_card(me("OPP", CardColor::Red, 4, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 6])
        .deck(1, &["DRAW"; 6])
        .security(1, &["DRAW"; 3])
        .memory(10)
        .start()
}

#[test]
fn ex12_038_on_play_trashes_a_mutant_or_me_card_to_draw_2() {
    let mut r = kokuwamon_runner(&["EX12-038", "MUTANT", "MACH"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "[Machine] is not [Mutant]/[ME]: only MUTANT qualifies"
    );
    select_hand_card(&mut r, 0, "MUTANT");
    r.auto_resolve().expect("resolve Kokuwamon On Play");
    assert!(trash_contains(&r, 0, "MUTANT"));
    assert_eq!(r.hand_size(0), 1 + 2, "MACH kept + 2 drawn");
}

#[test]
fn ex12_038_on_play_declined_or_without_a_match_draws_nothing() {
    let mut r = kokuwamon_runner(&["EX12-038", "ME-ONLY"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 6);

    let mut r = kokuwamon_runner(&["EX12-038", "MACH"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 6, "no [Mutant]/[ME] card in hand");
}

#[test]
fn ex12_038_inherited_when_attacking_gives_minus_2000_dp_for_the_turn() {
    let mut r = kokuwamon_runner(&[]);
    let carrier = r.place_stack(0, &["EX12-038", "MACH"]);
    let victim = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(victim).unwrap();
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(victim), Some(before - 2000));
}

// ═════════════════════════ Thundermon (EX12-041) ═════════════════════════

fn thundermon_runner(hand: &[&str]) -> DebugRunner {
    let mut opt = me("ME-OPTION", CardColor::Black, 3, &["ME"]);
    opt.card_kind = digimon_engine::enums::CardKind::Option;
    opt.level = None;
    opt.dp = None;
    opt.play_cost = 3;
    DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("ME-4", CardColor::Black, 4, &["ME"]))
        .add_card(me("MUTANT-3", CardColor::Black, 3, &["Mutant"]))
        .add_card(me("PLAIN-3", CardColor::Black, 3, &["Beast"]))
        .add_card(opt)
        .hand(0, hand)
        .memory(10)
        .start()
}

#[test]
fn ex12_041_main_plays_a_mutant_or_me_card_for_2_less() {
    let mut r = thundermon_runner(&["ME-4", "PLAIN-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let field_before = r.battle_area_size(0);
    let memory_before = r.memory();
    assert!(r.game.activate_field_main(0, t.index as usize));
    select_hand_card(&mut r, 0, "ME-4");
    r.auto_resolve().expect("resolve Thundermon Main");
    assert_eq!(r.battle_area_size(0), field_before + 1);
    assert_eq!(memory_before - r.memory(), 3, "play cost 5 reduced by 2");
}

#[test]
fn ex12_041_main_never_offers_a_card_without_the_mutant_or_me_trait() {
    let mut r = thundermon_runner(&["PLAIN-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    assert!(
        !r.game.activate_field_main(0, t.index as usize),
        "no [Mutant]/[ME] card in hand: the [Main] is not usable"
    );
}

/// Thundermon (EX12-041): a [Main] is activation-type (CR 4.3 15-8-4); the
/// player activates it by declaring it, and CR 4.3 15-14-1-4 counts that
/// declaration as the [Once Per Turn] use "even if the effect's processing
/// can't be executed". Passing the card pick afterwards still uses it. DCGO
/// EX12_041.cs `if (!isUsed) activateClass.RemoveUse()` refunds it; the rules
/// text decides.
#[test]
fn ex12_041_main_declared_then_pick_passed_spends_the_once_per_turn() {
    let mut r = thundermon_runner(&["MUTANT-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    assert!(r.game.activate_field_main(0, t.index as usize));
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(
        !r.game.activate_field_main(0, t.index as usize),
        "the declared [Main] used the [Once Per Turn]"
    );
    assert!(
        r.game.players[0].hand.iter().any(|c| c.card_id(&r.game.card_data) == "MUTANT-3"),
        "nothing was played"
    );
}

#[test]
fn ex12_041_main_can_use_an_me_option_for_2_less() {
    let mut r = thundermon_runner(&["ME-OPTION"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let memory_before = r.memory();
    assert!(r.game.activate_field_main(0, t.index as usize));
    select_hand_card(&mut r, 0, "ME-OPTION");
    let _ = r.auto_resolve();
    assert_eq!(memory_before - r.memory(), 1, "use cost 3 reduced by 2");
}

#[test]
fn ex12_041_digivolves_from_a_level_3_me_digimon_for_2() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("RED-ME-3", CardColor::Red, 3, &["ME"]))
        .add_card(me("RED-PLAIN-3", CardColor::Red, 3, &["Beast"]))
        .hand(0, &["EX12-041", "EX12-041"])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-041", 3, 2);
    let me_base = r.place_on_field(0, "RED-ME-3", Some(0));
    let plain_base = r.place_on_field(0, "RED-PLAIN-3", Some(1));
    assert_eq!(digivolve_cost(&mut r, "EX12-041", me_base.index as usize), Some(2));
    r.auto_resolve().ok();
    assert_eq!(digivolve_cost(&mut r, "EX12-041", plain_base.index as usize), None);
}

#[test]
fn ex12_041_inherited_when_attacking_gives_minus_2000_dp_for_the_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("CARRIER", CardColor::Yellow, 4, &["Mutant"]))
        .add_card(me("OPP", CardColor::Red, 4, &["Beast"]))
        .deck(0, &["OPP"; 4])
        .deck(1, &["OPP"; 4])
        .security(1, &["OPP"; 3])
        .start();
    let carrier = r.place_stack(0, &["EX12-041", "CARRIER"]);
    let victim = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(victim).unwrap();
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(victim), Some(before - 2000));
}

#[test]
fn ex12_041_rule_name_includes_mamemon_and_is_found_by_the_kokuwamon_reveal() {
    // Kokuwamon (EX13-046) [On Play] adds "1 card with [Mamemon] in its text":
    // Thundermon (EX12-041) prints `(Rule) Name: Treated as including
    // [Mamemon]` and must be selectable in that bucket.
    let deck = deck_with_top(&["EX12-041", "PLAIN", "FILL"], "FILL", 4);
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-046")
        .expect("EX13-046 YAML loads")
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("PLAIN", CardColor::Black, 4, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX13-046"])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let slot = hand_slot(&r, "EX13-046");
    assert!(r.play(0, slot).is_some());
    assert_eq!(offered_reveal_ids(&r), vec!["EX12-041".to_string()]);
    pick_revealed(&mut r, "EX12-041");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "EX12-041"));
}
