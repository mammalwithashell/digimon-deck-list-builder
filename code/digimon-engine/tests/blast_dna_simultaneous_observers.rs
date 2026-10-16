//! Blast DNA is one evolution event, including the defender's global observers.
//! Minimal DSL fixtures preserve real counter/material actions; no queue injection.
use digimon_engine::action::{
    build_action_mask,
    space::{DNA_DIGIVOLVE_START, PASS, PLAY_HAND_START},
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::selection::SelectionKind;

fn setup(self_delete: bool) -> (DebugRunner, CardHandle) {
    let result = format!(
        r#"
card: BLAST-BATCH-RESULT
name: Blast Batch Result
kind: digimon
level: 7
color: [green, blue]
cost: 9
dp: 15000
alt_paths:
  - kind: blast_dna_digivolve
    materials:
      - {{ kind: digimon, name_is: Breakdramon }}
      - {{ kind: digimon, name_is: Slayerdramon }}
    cost: 0
    stacks_unsuspended: true
effects:
  - kind: grant_keyword
    keyword: BlastDigivolve
  - when: when_digivolving
    condition: {{ dna_origin: true }}
    process:
      - gain_memory: 1
{}  - when: on_dna_digivolve
    condition: {{ dna_origin: true }}
    process:
      - gain_memory: 2
"#,
        if self_delete {
            "      - delete_permanent: { target: source }\n"
        } else {
            ""
        }
    );
    let observer = r#"
card: BLAST-BATCH-OBSERVER
name: Blast Batch Observer
kind: tamer
color: [white]
cost: 1
effects:
  - when: on_digivolve
    condition:
      all_of:
        - event_card_name_is: Blast Batch Result
        - event_is_effect_initiated: true
        - dna_origin: true
    process:
      - gain_memory: 4
"#;
    let mut field = make_test_card("FIELD", "Breakdramon");
    field.level = Some(6);
    field.dp = Some(12000);
    field.colors = vec![CardColor::Green];
    let mut hand = make_test_card("HAND", "Slayerdramon");
    hand.level = Some(6);
    hand.dp = Some(12000);
    hand.colors = vec![CardColor::Blue];
    let mut atk = make_test_card("ATK", "Attacker");
    atk.level = Some(6);
    atk.dp = Some(1000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&result)
        .unwrap()
        .from_dsl_yaml(observer)
        .unwrap()
        .add_card(field)
        .add_card(hand)
        .add_card(atk)
        .add_card(make_test_card("BONUS", "Bonus"))
        .hand(1, &["BLAST-BATCH-RESULT", "HAND"])
        .deck(1, &["BONUS", "BONUS"])
        .security(1, &["BONUS"])
        // Keep the attacker's turn through all defender gains; a later normal
        // draw must not contaminate the one Blast DNA bonus-draw assertion.
        .memory(10)
        .start();
    let result = r.game.players[1].hand[0].handle();
    assert_eq!(r.game.turn_player(), 0);
    let a = r.place_on_field(0, "ATK", Some(0));
    let target = r.place_on_field(1, "FIELD", Some(0));
    r.place_on_field(1, "BLAST-BATCH-OBSERVER", Some(0));
    r.attack_digimon(a, target, false);
    choose(&mut r, DNA_DIGIVOLVE_START);
    choose(&mut r, 0);
    choose(&mut r, PLAY_HAND_START + 1);
    (r, result)
}

fn choose(r: &mut DebugRunner, action: u16) {
    let view = r.pending_selection_view().expect("legal defender choice");
    assert_eq!(view.selecting_player, 1);
    assert_eq!(build_action_mask(&r.game, 1)[action as usize], 1.0);
    r.game.resolve_selection(1, action).unwrap();
}

fn assert_order(r: &DebugRunner, timings: &[EffectTiming]) {
    let view = r
        .pending_selection_view()
        .expect("joint Blast DNA trigger order");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert_eq!(
        view.selecting_player, 1,
        "the defender controls these triggers"
    );
    assert!(!view.is_optional);
    assert_eq!(build_action_mask(&r.game, 1)[PASS as usize], 0.0);
    let choices = view.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), timings.len());
    for timing in timings {
        let c = choices
            .iter()
            .find(|c| c.timing == Some(*timing))
            .expect("all event families offered");
        assert!(c.source_card.is_some());
        assert!(c
            .label
            .starts_with(if *timing == EffectTiming::OnDigivolve {
                "BLAST-BATCH-OBSERVER"
            } else {
                "BLAST-BATCH-RESULT"
            }));
        assert_eq!(build_action_mask(&r.game, 1)[c.action_id as usize], 1.0);
    }
}

fn pick(r: &mut DebugRunner, timing: EffectTiming) {
    let view = r.pending_selection_view().unwrap();
    let action = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.timing == Some(timing))
        .unwrap()
        .action_id;
    choose(r, action);
}

#[test]
fn blast_dna_defender_can_choose_all_six_orders_with_cloned_origin_and_bonus_draw() {
    use EffectTiming::{OnDigivolve as O, OnDnaDigivolve as D, WhenDigivolving as W};
    let (base, result) = setup(false);
    assert_order(&base, &[W, D, O]);
    assert_eq!(base.memory(), 10);
    let before = defender_memory(&base);
    for order in [
        [W, D, O],
        [W, O, D],
        [D, W, O],
        [D, O, W],
        [O, W, D],
        [O, D, W],
    ] {
        let mut r = DebugRunner::wrap(base.game.clone());
        pick(&mut r, order[0]);
        assert_order(&r, &order[1..]);
        pick(&mut r, order[1]);
        assert_eq!(
            defender_memory(&r) - before,
            7,
            "defender's three effects each resolve once"
        );
        assert_eq!(r.game.turn_player(), 0, "attacker retains the turn");
        assert_eq!(r.game.players[1].battle_area[0].top_card().handle(), result);
        assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 3);
        assert_eq!(r.hand_size(1), 1, "bonus draw occurs once before effects");
        assert_eq!(r.game.players[1].deck.len(), 1);
    }
}

#[test]
fn blast_dna_observer_keeps_captured_event_card_after_result_departs() {
    use EffectTiming::{OnDigivolve as O, OnDnaDigivolve as D, WhenDigivolving as W};
    let (base, result) = setup(true);
    assert_order(&base, &[W, D, O]);
    let before = defender_memory(&base);
    for _ in 0..2 {
        let mut r = DebugRunner::wrap(base.game.clone());
        pick(&mut r, W);
        assert_eq!(
            defender_memory(&r) - before,
            5,
            "native +1 and observer +4; departed source's DNA effect cannot activate"
        );
        assert_eq!(r.game.turn_player(), 0, "attacker retains the turn");
        assert!(r.game.players[1].trash.iter().any(|c| c.handle() == result));
        assert_eq!(r.battle_area_size(1), 1);
        assert_eq!(
            r.game.players[1].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "BLAST-BATCH-OBSERVER"
        );
        assert_eq!(r.hand_size(1), 1);
    }
}

fn defender_memory(r: &DebugRunner) -> i16 {
    // Stored from the current turn player's perspective: completing the
    // counter can advance from the attacker to the defender's turn.
    if r.game.turn_player() == 1 {
        r.memory()
    } else {
        -r.memory()
    }
}
