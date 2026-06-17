//! BT13-019 Gankoomon
//!
//! Implemented slice:
//! - <Blocker> static keyword.
//! - [On Play][When Digivolving] Without paying the cost, you may play 1 Digimon
//!   card with [Sistermon] in its name from your trash OR 1 Digimon card with the
//!   [Royal Knight] trait from the digivolution cards of your Digimon in the
//!   breeding area. This effect can't play [Omnimon] or [Gankoomon].
//!   (G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY — a single union selection over
//!   {trash} ∪ {breeding-area digivolution sources}, each half carrying its own
//!   filter, with a fixed name exclusion.)

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const TRASH_EFFECT_START: u16 = digimon_engine::action::space::TRASH_EFFECT_START;

fn sistermon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Sistermon Blanc");
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 5;
    card.traits = vec!["Holy Warrior".to_string()];
    card
}

fn royal_knight(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 12;
    card.traits = vec!["Royal Knight".to_string()];
    card
}

/// A plain breeding-area carrier Digimon to host digivolution sources.
fn breeding_host(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

/// Insert `card_id` directly as a bottom digivolution source under the
/// breeding-area carrier. Mirrors `bt13_007.rs::add_breeding_source`.
fn add_breeding_source(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("add_breeding_source: unknown card_id {card_id}"));
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_idx, player, card_index);
    let breeding = runner.game.players[player as usize]
        .breeding_area
        .as_mut()
        .expect("breeding permanent exists");
    // Insert beneath the carrier's top card so it counts as a digivolution card.
    breeding.card_sources.insert(0, source);
}

/// The breeding-source action id for the source at `source_index` under
/// player `owner`'s breeding carrier.
fn breeding_source_action(owner: u8, source_index: u16) -> u16 {
    digimon_engine::action::space::encode_breeding_source_select(owner, source_index)
        .expect("valid breeding-source action id")
}

#[test]
fn bt13_019_has_blocker_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-019")
        .expect("load")
        .start();
    let card = runner.compiled_card("BT13-019").expect("compiled card");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
    )));
}

/// Build a runner with BT13-019 on P0's field, a Sistermon + an excluded
/// Omnimon in P0's trash, and a breeding carrier holding a Royal Knight, an
/// excluded Gankoomon, and a non-Royal-Knight source. Returns the runner with
/// BT13-019 already on the field at index 0 (not yet fired).
fn union_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-019")
        .expect("BT13-019 must load from embedded DSL pack")
        .add_card(sistermon("SISTERMON-TRASH"))
        .add_card(royal_knight("OMNIMON-TRASH", "Omnimon")) // excluded by name, in trash
        .add_card(breeding_host("BREED-HOST"))
        .add_card(royal_knight("RK-SRC", "Alphamon")) // legal breeding RK source
        .add_card(royal_knight("GANKOOMON-SRC", "Gankoomon")) // excluded by name, breeding source
        .add_card(breeding_host("PLAIN-SRC")) // non-Royal-Knight breeding source
        .memory(10)
        .start();

    runner.place_on_field(0, "BT13-019", Some(0));

    // Trash: a legal Sistermon + an excluded Omnimon Royal-Knight-name card.
    runner.inject_trash(0, "SISTERMON-TRASH");
    runner.inject_trash(0, "OMNIMON-TRASH");

    // Breeding carrier with three digivolution sources beneath its top card:
    //   bottom→top inserts: PLAIN-SRC, GANKOOMON-SRC, RK-SRC
    runner.place_in_breeding(0, "BREED-HOST");
    add_breeding_source(&mut runner, 0, "PLAIN-SRC");
    add_breeding_source(&mut runner, 0, "GANKOOMON-SRC");
    add_breeding_source(&mut runner, 0, "RK-SRC");

    runner
}

/// (a) trash Sistermon is legal, (b) breeding Royal Knight is legal,
/// (c) Gankoomon/Omnimon are excluded, (e) both zones populated → union of
/// candidates surfaces in ONE prompt.
#[test]
fn bt13_019_plays_sistermon_or_royal_knight_from_breeding_sources() {
    let mut runner = union_runner();

    runner.fire_on_play(0, 0);

    let pending = runner
        .pending_selection()
        .expect("BT13-019 On Play surfaces a union selection over trash + breeding sources");

    assert!(
        pending.is_optional,
        "printed 'you may play 1' must expose PASS (decline)"
    );

    // Trash indices: SISTERMON-TRASH at trash[0] (legal), OMNIMON-TRASH at
    // trash[1] (excluded by name).
    let sistermon_action = TRASH_EFFECT_START; // trash[0]
    let omnimon_action = TRASH_EFFECT_START + 1; // trash[1]

    // `add_breeding_source` inserts each new source at card_sources index 0, so
    // the three inserts (PLAIN-SRC, then GANKOOMON-SRC, then RK-SRC) leave the
    // stack as: idx 0 = RK-SRC (last inserted), idx 1 = GANKOOMON-SRC, idx 2 =
    // PLAIN-SRC, idx 3 = BREED-HOST (top). RK-SRC is the legal Royal Knight;
    // GANKOOMON-SRC is excluded by name; PLAIN-SRC is not a Royal Knight.
    let rk_action = breeding_source_action(0, 0);
    let gankoomon_action = breeding_source_action(0, 1);
    let plain_src_action = breeding_source_action(0, 2);

    let ids = &pending.valid_action_ids;
    assert!(
        ids.contains(&sistermon_action),
        "trash [Sistermon] must be a legal pick; ids = {ids:?}"
    );
    assert!(
        ids.contains(&rk_action),
        "breeding [Royal Knight] source must be a legal pick; ids = {ids:?}"
    );
    assert!(
        !ids.contains(&omnimon_action),
        "Omnimon (name-excluded) must be masked from trash; ids = {ids:?}"
    );
    assert!(
        !ids.contains(&gankoomon_action),
        "Gankoomon (name-excluded) must be masked from breeding sources; ids = {ids:?}"
    );
    assert!(
        !ids.contains(&plain_src_action),
        "non-Royal-Knight breeding source must be masked; ids = {ids:?}"
    );

    // Both zones offered in one prompt: exactly two legal candidates.
    assert_eq!(
        ids.len(),
        2,
        "union must offer exactly the legal trash Sistermon + breeding Royal Knight; ids = {ids:?}"
    );

    // Submit the trash Sistermon → it enters P0's battle area for free.
    let mem_before = runner.memory();
    runner
        .execute_action(0, sistermon_action)
        .expect("play Sistermon from trash");
    runner.auto_resolve().ok();

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SISTERMON-TRASH"),
        "the chosen Sistermon must enter the battle area as a new permanent"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "playing from trash via this effect is free (no cost paid)"
    );
}

/// (b) A [Royal Knight] under a breeding Digimon is a legal pick and plays
/// free, moving the source out of the breeding stack into the battle area.
#[test]
fn bt13_019_plays_royal_knight_from_breeding_only() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-019")
        .expect("load")
        .add_card(breeding_host("BREED-HOST"))
        .add_card(royal_knight("RK-SRC", "Alphamon"))
        .memory(10)
        .start();

    runner.place_on_field(0, "BT13-019", Some(0));
    runner.place_in_breeding(0, "BREED-HOST");
    add_breeding_source(&mut runner, 0, "RK-SRC");

    runner.fire_on_play(0, 0);

    let pending = runner
        .pending_selection()
        .expect("breeding-only union still surfaces a selection");
    let rk_action = breeding_source_action(0, 0);
    assert_eq!(
        pending.valid_action_ids,
        vec![rk_action],
        "only the single breeding Royal Knight is legal"
    );

    let breeding_sources_before = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .map(|p| p.card_sources.len())
        .unwrap_or(0);

    runner
        .execute_action(0, rk_action)
        .expect("play Royal Knight from breeding sources");
    runner.auto_resolve().ok();

    let breeding_sources_after = runner
        .game
        .player(0)
        .breeding_area
        .as_ref()
        .map(|p| p.card_sources.len())
        .unwrap_or(0);
    assert_eq!(
        breeding_sources_after,
        breeding_sources_before - 1,
        "the played Royal Knight leaves the breeding stack"
    );
    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "RK-SRC"),
        "the breeding Royal Knight enters the battle area as a new permanent"
    );
}

/// (d) PASS/decline is exposed and leaves the board unchanged.
#[test]
fn bt13_019_decline_leaves_board_unchanged() {
    let mut runner = union_runner();
    let battle_before = runner.game.player(0).battle_area.len();
    let trash_before = runner.trash_size(0);

    runner.fire_on_play(0, 0);
    let pending = runner
        .pending_selection()
        .expect("union selection surfaces");
    assert!(pending.is_optional, "must be declinable");

    // Decline the optional selection (PASS).
    let selecting_player = pending.selecting_player;
    runner
        .execute_action(selecting_player, digimon_engine::action::space::PASS)
        .expect("decline the optional play");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.game.player(0).battle_area.len(),
        battle_before,
        "declining must not add any permanent"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining must not move any trash card"
    );
    assert!(
        runner.pending_selection().is_none(),
        "no further selection after declining"
    );
}
