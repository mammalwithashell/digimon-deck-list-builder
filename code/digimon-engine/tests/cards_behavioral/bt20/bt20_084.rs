//! BT20-084 Sistermon Ciel (Awakened).
//!
//! Printed text (`data/cards.json`):
//! - [Trash] [All Turns] When any of your Digimon are played, 1 of your
//!   [Sistermon Ciel]s may digivolve into this card without paying the cost.
//! - [On Play] [When Digivolving] 1 of your opponent's Digimon or Tamers can't
//!   suspend until the end of their turn.
//! - [End of All Turns] Place this Digimon's top stacked card as the top
//!   security card.
//!
//! Supported slice:
//! - Printed card identity and [Sistermon Ciel] alternate digivolution path.
//! - Shared On Play / When Digivolving target choice for opponent Digimon/Tamers
//!   with `CannotSuspend` expiring at the end of the opponent's turn.
//! - Trash-resident [All Turns] observer that exposes the optional choice to
//!   digivolve a field [Sistermon Ciel] into this exact trash card for free.
//!
//! Known gaps:
//! - PUPPETS-G027: end-of-all-turns top-stack-card to security movement needs a
//!   faithful stack extraction to security-top primitive that does not leave an
//!   invalid empty permanent when the top card is the only card in the stack.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

#[test]
fn bt20_084_has_printed_stats_alt_path_and_supported_shared_lock_clause() {
    let runner = bt20_084_runner().start();
    let compiled = runner.compiled_card("BT20-084").expect("BT20-084 compiled");

    assert_eq!(compiled.card, "BT20-084");
    assert_eq!(compiled.name, "Sistermon Ciel (Awakened)");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(4));
    assert_eq!(compiled.color, vec![CompiledColor::White]);
    assert_eq!(compiled.cost, Some(5));
    assert_eq!(compiled.dp, Some(6000));

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(1))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|from| from.name_is.as_deref() == Some("Sistermon Ciel"))
        }),
        "BT20-084 must digivolve from [Sistermon Ciel] for cost 1"
    );

    assert_eq!(
        compiled.effects.len(),
        2,
        "trash observer and shared lock clauses should both compile"
    );
    let trash_clause = compiled
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(clause)
                if clause.when.contains(&CompiledTiming::OnAllyPlayed) =>
            {
                Some(clause)
            }
            _ => None,
        })
        .expect("trash on_ally_played clause");
    assert!(
        trash_clause.optional,
        "printed trash digivolve uses 'may'"
    );
    assert!(matches!(
        trash_clause.process.as_slice(),
        [
            CompiledStep::SelectOwnPermanent { bind_as, .. },
            CompiledStep::EffectInitiatedDigivolve { .. },
        ] if bind_as.as_deref() == Some("target")
    ));

    let lock = compiled
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(clause)
                if clause.when.contains(&CompiledTiming::OnPlay)
                    && clause.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(clause)
            }
            _ => None,
        })
        .expect("supported shared lock clause");
    assert!(lock.when.contains(&CompiledTiming::OnPlay));
    assert!(lock.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(
        !lock.optional,
        "printed lock effect is mandatory when a target exists"
    );

    assert!(matches!(
        lock.process.as_slice(),
        [
            CompiledStep::SelectOpponentPermanent { bind_as, .. },
            CompiledStep::AddModifier { modifier, expiry, .. },
        ] if bind_as.as_deref() == Some("target")
            && modifier == "CannotSuspend"
            && expiry == "end_of_opponents_turn"
    ));

    if let CompiledStep::SelectOpponentPermanent { filter, .. } = &lock.process[0] {
        assert!(
            filter
                .any_of
                .iter()
                .any(|part| part.kind == Some(CompiledCardKind::Digimon))
                && filter
                    .any_of
                    .iter()
                    .any(|part| part.kind == Some(CompiledCardKind::Tamer)),
            "target filter must accept opponent Digimon or Tamers"
        );
    }
}

#[test]
fn bt20_084_on_play_locks_selected_opponent_digimon_until_their_turn_ends() {
    let mut runner = bt20_084_runner().hand(0, &["BT20-084"]).memory(10).start();
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    let opp_option = runner.place_on_field(1, "OPP-OPTION", Some(0));

    runner.play(0, 0).expect("play BT20-084");

    let view = runner
        .pending_selection_view()
        .expect("On Play target selection");
    assert!(!view.valid_action_ids.contains(&PASS));
    assert!(view
        .valid_action_ids
        .contains(&encode_permanent(opp_digimon)));
    assert!(view.valid_action_ids.contains(&encode_permanent(opp_tamer)));
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_permanent(opp_option)),
        "opponent Option permanents are not legal lock targets"
    );
    choose_permanent(&mut runner, opp_digimon, "choose opponent Digimon");
    runner.auto_resolve().expect("finish On Play lock");

    assert_cannot_suspend_until_opponent_turn_end(&mut runner, opp_digimon);
}

#[test]
fn bt20_084_when_digivolving_locks_selected_opponent_tamer() {
    let mut runner = bt20_084_runner().hand(0, &["BT20-084"]).memory(10).start();
    let base = runner.place_on_field(0, "SISTERMON-CIEL-BASE", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    let evo_card = runner.game.players[0].hand.remove(0);
    runner.game.players[0].battle_area[base.index as usize]
        .card_sources
        .push(evo_card);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(base),
    );
    runner.game.drain_effect_queue();

    choose_permanent(&mut runner, opp_tamer, "choose opponent Tamer");
    runner.auto_resolve().expect("finish When Digivolving lock");

    assert!(runner
        .modifiers()
        .has(opp_tamer, ModifierType::CannotSuspend));
}

#[test]
fn bt20_084_on_play_with_no_opponent_digimon_or_tamer_resolves_without_prompt() {
    let mut runner = bt20_084_runner().hand(0, &["BT20-084"]).memory(10).start();
    runner.place_on_field(1, "OPP-OPTION", Some(0));

    runner.play(0, 0).expect("play BT20-084");
    runner.auto_resolve().expect("no legal target to choose");

    assert!(
        runner.pending_selection().is_none(),
        "no Digimon/Tamer target means no pending choice"
    );
}

#[test]
fn bt20_084_trash_observer_may_digivolve_sistermon_ciel_for_free_when_your_digimon_is_played() {
    let mut runner = bt20_084_runner()
        .hand(0, &["ALLY-DIGIMON"])
        .memory(10)
        .start();
    let base = runner.place_on_field(0, "SISTERMON-CIEL-BASE", Some(0));
    let bt20_084 = push_to_trash(&mut runner, 0, "BT20-084");

    runner.play(0, 0).expect("play allied Digimon");

    let view = runner
        .pending_selection_view()
        .expect("trash observer optional digivolve target");
    assert!(view.is_optional, "printed 'may' must be declinable");
    let mask = build_action_mask(&runner.game, view.selecting_player);
    assert!(
        mask[PASS as usize] > 0.5,
        "printed 'may' must expose a decline action"
    );
    assert!(
        view.valid_action_ids.contains(&encode_permanent(base)),
        "Sistermon Ciel must be a legal free-digivolve target"
    );

    choose_permanent(&mut runner, base, "choose Sistermon Ciel");
    runner.auto_resolve().expect("finish trash digivolve");

    let stack = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        stack.top_card().handle(),
        bt20_084,
        "BT20-084 from trash should become the top card"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .all(|card| card.handle() != bt20_084),
        "effect digivolve must consume the exact trash source"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G027 — end-of-all-turns top-stack-card to top security needs faithful stack movement and empty-permanent cleanup"]
fn bt20_084_end_of_all_turns_places_top_stacked_card_as_top_security() {
    todo!("place BT20-084 on a stack, end either player's turn, assert the top stack card moves to your top security and the battle-area stack remains legal");
}

fn bt20_084_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT20-084")
        .expect("BT20-084 YAML must be embedded")
        .add_card(sistermon_ciel_base())
        .add_card(make_test_card("ALLY-DIGIMON", "Ally Digimon"))
        .add_card(make_test_card("OPP-DIGIMON", "Opponent Digimon"))
        .add_card(make_tamer("OPP-TAMER"))
        .add_card(make_option("OPP-OPTION"))
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card registered");
    let card_idx = runner.game.next_card_index();
    let source = CardSource::new(data_idx, player, card_idx);
    let handle = source.handle();
    runner.game.players[player as usize].trash.push(source);
    handle
}

fn sistermon_ciel_base() -> CardData {
    let mut card = make_test_card("SISTERMON-CIEL-BASE", "Sistermon Ciel");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn make_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = encode_permanent(handle);
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: expected action {action}, got {:?}",
        view.valid_action_ids
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn assert_cannot_suspend_until_opponent_turn_end(
    runner: &mut DebugRunner,
    handle: PermanentHandle,
) {
    assert!(runner.modifiers().has(handle, ModifierType::CannotSuspend));
    assert!(
        has_cannot_suspend_entry(runner, handle, Expiry::EndOfOpponentsTurn),
        "CannotSuspend must expire at the end of the opponent's turn"
    );

    runner.game.modifiers.expire_end_of_turn(0);
    assert!(
        runner.modifiers().has(handle, ModifierType::CannotSuspend),
        "lock should not expire at the end of BT20-084 controller's turn"
    );

    runner.game.modifiers.expire_end_of_turn(1);
    assert!(
        !runner.modifiers().has(handle, ModifierType::CannotSuspend),
        "lock should expire at the end of the target controller's turn"
    );
}

fn has_cannot_suspend_entry(runner: &DebugRunner, handle: PermanentHandle, expiry: Expiry) -> bool {
    runner
        .modifiers()
        .get(handle, ModifierType::CannotSuspend)
        .iter()
        .any(|entry| entry.modifier == ModifierType::CannotSuspend && entry.expiry == expiry)
}
