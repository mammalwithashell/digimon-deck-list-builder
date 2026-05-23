use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::{EffectContext, RevealBucketSelection};
use digimon_engine::enums::{CardKind, EffectSourceKind, GamePhase};
use digimon_engine::selection::{PendingSelection, SelectionKind};

fn free_digimon() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TEST-FREE", "Free Candidate");
    card.traits = vec!["Free".to_string()];
    card
}

fn ken_tamer() -> digimon_engine::card_data::CardData {
    let mut card = make_test_card("TEST-KEN", "Ken Ichijoji");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn runner_with_wormmon_reveal() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-047")
        .expect("BT12-047 should be in embedded DSL pack")
        .add_card(free_digimon())
        .add_card(ken_tamer())
        .add_card(make_test_card("TEST-FILLER", "Filler"))
        .hand(0, &["BT12-047"])
        .deck(0, &["TEST-FILLER", "TEST-KEN", "TEST-FREE"])
        .memory(0)
        .start()
}

fn reveal_for_test(
    r: &mut DebugRunner,
    player: digimon_engine::enums::PlayerId,
    card_id: &str,
) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    r.game.revealed_cards.push(card);
    handle
}

#[test]
fn pending_selection_defers_turn_end_after_memory_crossing_play() {
    let mut r = runner_with_wormmon_reveal();
    let player = r.turn_player();
    r.game.enter_main_phase();

    r.game.decode_action(PLAY_HAND_START, player);

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("BT12-047 should install a reveal-bucket pending selection");
    assert!(matches!(pending.kind, SelectionKind::RevealBucket { .. }));
    assert_eq!(pending.selecting_player, player);
    assert_eq!(
        r.current_phase(),
        GamePhase::SelectReveal,
        "mandatory reveal selection must stay active instead of parking in EndTurn"
    );

    let mask = build_action_mask(&r.game, player);
    assert_eq!(
        mask[PASS as usize], 0.0,
        "mandatory reveal selection must not expose pass"
    );
    for action_id in &pending.valid_action_ids {
        assert_eq!(
            mask[*action_id as usize], 1.0,
            "pending action id {action_id} should be legal in the headless mask"
        );
    }
}

#[test]
fn turn_end_runs_after_pending_selection_resolves() {
    let mut r = runner_with_wormmon_reveal();
    let player = r.turn_player();
    r.game.enter_main_phase();
    r.game.decode_action(PLAY_HAND_START, player);

    while let Some(action_id) = r
        .game
        .pending_selection
        .as_ref()
        .and_then(|pending| pending.valid_action_ids.first().copied())
    {
        r.game
            .resolve_selection(player, action_id)
            .expect("pending reveal selection action should resolve");
    }

    assert_ne!(
        r.turn_player(),
        player,
        "turn should rotate once the mandatory selection and follow-up effects finish"
    );
    assert_eq!(
        r.memory(),
        3,
        "BT12-047 paid 3 from zero, so the next player should start at 3"
    );
}

#[test]
fn reveal_selection_mask_exposes_pass_only_when_optional() {
    let mut r = DebugRunner::builder().add_card(free_digimon()).start();
    let player = r.turn_player();
    let revealed = reveal_for_test(&mut r, player, "TEST-FREE");

    {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, player);
        ctx.select_reveal_buckets(
            vec![RevealBucketSelection {
                bind_as: "optional_pick".to_string(),
                min: 0,
                max: 1,
                candidates: vec![revealed],
            }],
            "Choose up to 1 card",
            true,
            |_ctx, _picked| {},
        );
    }

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional reveal selection should be pending");
    assert!(pending.is_optional);

    let mask = build_action_mask(&r.game, player);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "optional reveal selections should expose pass"
    );
    for action_id in &pending.valid_action_ids {
        assert_eq!(mask[*action_id as usize], 1.0);
    }
}

#[test]
fn pending_selection_takes_precedence_over_main_phase_mask_and_decode() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-HAND", "Hand Card"))
        .hand(0, &["TEST-HAND"])
        .memory(5)
        .start();
    let player = r.turn_player();
    r.game.enter_main_phase();

    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: player,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![59],
        is_optional: false,
        prompt: "Resolve synthetic pending choice".to_string(),
        effect_choices: None,
        source_card: CardHandle(0),
        source_permanent: None,
        source_kind: EffectSourceKind::Rule,
        callback: Box::new(|game, _action_id| {
            game.set_memory(4);
        }),
        on_decline: None,
    });
    // This mirrors the bad training recordings: a prompt is pending, but the
    // phase has been restored to Main while the prompt is still live.
    r.game.current_phase = GamePhase::Main;

    let mask = build_action_mask(&r.game, player);
    assert_eq!(
        mask[59], 1.0,
        "pending valid actions must be exposed even when previous_phase is Main"
    );
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "ordinary Main play actions must not leak while a prompt is pending"
    );

    r.game.decode_action(59, player);

    assert!(
        r.game.pending_selection.is_none(),
        "pending selection should resolve through decode_action before Main routing"
    );
    assert_eq!(
        r.memory(),
        4,
        "selection callback should have run instead of Main-phase hand-effect decode"
    );
}

#[test]
fn start_main_phase_selection_resumes_to_main_not_breeding() {
    let agumon = make_test_card("TEST-AGU", "Agumon");
    let mut r = DebugRunner::builder()
        .dsl_card("BT22-084")
        .expect("BT22-084 should be in embedded DSL pack")
        .add_card(agumon)
        .hand(0, &["TEST-AGU"])
        .start();
    let player = r.turn_player();
    r.place_on_field(player, "BT22-084", Some(0));
    r.game.current_phase = GamePhase::Breeding;

    r.game.enter_main_phase();

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("BT22-084 should install a start-main prompt");
    assert_eq!(
        pending.previous_phase,
        GamePhase::Main,
        "start-main prompts must resume Main, not re-enter Breeding"
    );
}
