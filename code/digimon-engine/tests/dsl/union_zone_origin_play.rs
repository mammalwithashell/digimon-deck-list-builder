//! Task 7 (PUPPETS-G014 substrate): origin-preserving union-zone play
//! consumer.
//!
//! A `select_union_zone` (hand ∪ trash ∪ material) selection now records the *origin*
//! zone of the chosen card, not just its `CardHandle`. The new
//! `play_union_bound_free` step consumes that binding and plays the card
//! for free from its true origin zone — `play_from_hand_free` for a
//! hand-origin pick, `play_from_trash_free` for a trash-origin pick.
//!
//! These tests prove all three branches:
//!   (a) pick the hand card  → it is played FROM HAND  (hand shrinks).
//!   (b) pick the trash card → it is played FROM TRASH (trash shrinks).
//!   (c) decline the optional union pick → the mandatory tail step still runs.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

/// The fixture body: `select_union_zone` (hand ∪ trash, optional) then
/// `play_union_bound_free` consuming the binding. `GainMemory(1)` is a
/// trivial mandatory tail step so we can also assert the decline path.
fn fixture_steps() -> Vec<CompiledStep> {
    vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Hand, CompiledZone::Trash],
            material_of: None,
            filter: CompiledPredicate::default(),
            bind_as: Some("union_pick".to_string()),
            prompt: "Pick from hand or trash".to_string(),
            prompt_key: None,
            optional: true,
            cost: false,
            then: vec![],
        },
        CompiledStep::PlayUnionBoundFree {
            binding: "union_pick".to_string(),
            bind_as: Some("played".to_string()),
            suppress_on_play: false,
        },
        // Mandatory tail step — must run even when the optional union pick
        // is declined.
        CompiledStep::GainMemory(1),
    ]
}

/// (a) Selecting the HAND card must play it from hand: hand shrinks by 1,
/// trash is untouched, and a permanent enters the battle area.
#[test]
fn union_zone_origin_play_plays_hand_card_from_hand() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("HAND-CARD", "HandCard"))
        .add_card(make_test_card("TRASH-CARD", "TrashCard"))
        .hand(0, &["SRC", "HAND-CARD"])
        .memory(5)
        .build();

    push_to_trash(&mut runner, 0, "TRASH-CARD");
    let src_card = runner.game.players[0].hand[0].handle();

    let hand_before = runner.game.players[0].hand.len();
    let trash_before = runner.game.players[0].trash.len();
    let battle_before = runner.game.players[0].battle_area.len();
    let memory_before = runner.game.memory;

    let steps = fixture_steps();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    // 2 hand cards (SRC + HAND-CARD) + 1 trash card = 3 candidates.
    assert_eq!(pending.valid_action_ids.len(), 3);
    let selecting_player = pending.selecting_player;

    // Resolve the HAND-CARD pick. Hand candidates are encoded in the
    // PLAY_HAND range; HAND-CARD sits at hand index 1.
    use digimon_engine::action::space::PLAY_HAND_START;
    let action_id = PLAY_HAND_START + 1;
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve hand-origin pick");

    assert!(runner.game.pending_selection.is_none());
    // HAND-CARD left the hand; SRC remains → hand shrinks by exactly 1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand-origin pick must consume the card from HAND"
    );
    // Trash must be completely untouched.
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "hand-origin pick must NOT touch trash"
    );
    // A permanent entered the battle area.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "the played card must enter the battle area"
    );
    // Played card identity check.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "HAND-CARD"),
        "the battle-area permanent must be the hand card"
    );
    // Free play — memory unchanged minus the +1 from the tail GainMemory.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "play_union_bound_free must not pay cost; only tail GainMemory(1) applies"
    );
}

fn material_fixture_steps() -> Vec<CompiledStep> {
    vec![
        CompiledStep::SelectUnionZone {
            of: CompiledPlayerRef::You,
            zones: vec![CompiledZone::Material],
            material_of: None,
            filter: CompiledPredicate::default(),
            bind_as: Some("union_pick".to_string()),
            prompt: "Pick from materials".to_string(),
            prompt_key: None,
            optional: true,
            cost: false,
            then: vec![],
        },
        CompiledStep::PlayUnionBoundFree {
            binding: "union_pick".to_string(),
            bind_as: Some("played".to_string()),
            suppress_on_play: false,
        },
    ]
}

/// (b) Selecting the TRASH card must play it from trash: trash shrinks by 1,
/// hand is untouched, and a permanent enters the battle area.
#[test]
fn union_zone_origin_play_plays_trash_card_from_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("HAND-CARD", "HandCard"))
        .add_card(make_test_card("TRASH-CARD", "TrashCard"))
        .hand(0, &["SRC", "HAND-CARD"])
        .memory(5)
        .build();

    push_to_trash(&mut runner, 0, "TRASH-CARD");
    let src_card = runner.game.players[0].hand[0].handle();

    let hand_before = runner.game.players[0].hand.len();
    let trash_before = runner.game.players[0].trash.len();
    let battle_before = runner.game.players[0].battle_area.len();
    let memory_before = runner.game.memory;

    let steps = fixture_steps();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    assert_eq!(pending.valid_action_ids.len(), 3);
    let selecting_player = pending.selecting_player;

    // Resolve the TRASH-CARD pick. Trash candidates are encoded in the
    // TRASH_EFFECT range; TRASH-CARD sits at trash index 0.
    use digimon_engine::action::space::TRASH_EFFECT_START;
    let action_id = TRASH_EFFECT_START;
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve trash-origin pick");

    assert!(runner.game.pending_selection.is_none());
    // TRASH-CARD left the trash → trash shrinks by exactly 1.
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before - 1,
        "trash-origin pick must consume the card from TRASH"
    );
    // Hand must be completely untouched.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "trash-origin pick must NOT touch hand"
    );
    // A permanent entered the battle area.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "the played card must enter the battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "TRASH-CARD"),
        "the battle-area permanent must be the trash card"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "play_union_bound_free must not pay cost; only tail GainMemory(1) applies"
    );
}

/// (c) Declining the optional union pick must still run the mandatory tail
/// (`GainMemory(1)`). No card is played; only the tail fires.
#[test]
fn union_zone_origin_play_decline_runs_mandatory_tail() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("HAND-CARD", "HandCard"))
        .add_card(make_test_card("TRASH-CARD", "TrashCard"))
        .hand(0, &["SRC", "HAND-CARD"])
        .memory(5)
        .build();

    push_to_trash(&mut runner, 0, "TRASH-CARD");
    let src_card = runner.game.players[0].hand[0].handle();

    let hand_before = runner.game.players[0].hand.len();
    let trash_before = runner.game.players[0].trash.len();
    let battle_before = runner.game.players[0].battle_area.len();
    let memory_before = runner.game.memory;

    let steps = fixture_steps();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_union_zone must install a pending selection");
    assert!(pending.is_optional, "the union pick is optional");
    let selecting_player = pending.selecting_player;

    // Decline the optional selection by submitting the PASS action.
    use digimon_engine::action::space::PASS;
    runner
        .game
        .resolve_selection(selecting_player, PASS)
        .expect("decline optional union pick");

    assert!(runner.game.pending_selection.is_none());
    // No card was played — both zones and the battle area are untouched.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "decline must not play any card from hand"
    );
    assert_eq!(
        runner.game.players[0].trash.len(),
        trash_before,
        "decline must not play any card from trash"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before,
        "decline must not add any permanent"
    );
    // The mandatory tail step still ran.
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "declining the optional union pick must still run the mandatory tail GainMemory(1)"
    );
}

/// (d) Selecting a MATERIAL card must expose source-select action IDs and
/// play the selected digivolution card from the source stack for free.
#[test]
fn union_zone_origin_play_plays_material_card_from_source_stack() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("MATERIAL-CARD", "MaterialCard"))
        .memory(5)
        .build();

    let host = runner.place_stack(0, &["MATERIAL-CARD", "SRC"]);
    let src_card = runner.top_card(host);

    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();
    let source_count_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let memory_before = runner.game.memory;

    let steps = material_fixture_steps();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(host), 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("material union selection must install a pending selection");
    assert_eq!(pending.valid_action_ids.len(), 1);
    let selecting_player = pending.selecting_player;

    let source_action = digimon_engine::action::space::encode_source_select(host.index as u16, 0)
        .expect("source action id");
    assert!(
        pending.valid_action_ids.contains(&source_action),
        "material union selection must expose the source-select action"
    );

    runner
        .game
        .resolve_selection(selecting_player, source_action)
        .expect("resolve material-origin pick");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "material-origin pick must not consume a hand card"
    );
    assert_eq!(
        runner.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        source_count_before - 1,
        "material-origin pick must remove the selected source card"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "the material card must be played as a new permanent"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "MATERIAL-CARD"),
        "the battle-area permanent must be the material card"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "play_union_bound_free must not pay the material card's play cost"
    );
}
