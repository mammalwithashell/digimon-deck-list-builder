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
//! - End-of-all-turns movement of the top stacked card into security via the
//!   Track E `security_place_top_stacked_card` DSL verb.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/White/BT20_084.cs
//!
//! # Patterns this test covers
//! - A5 stack shift / digivolve from trash (trash-resident on_ally_played observer)
//! - E2 optional decline (printed "may" on the trash digivolve)
//! - F7 cannot suspend (shared On Play / When Digivolving lock)
//! - Track E stacked-card-to-security movement (PUPPETS-G026 / PUPPETS-G027 closure)

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
        3,
        "trash observer, shared lock, and end-of-all-turns stacked-card clauses should compile"
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
    assert!(trash_clause.optional, "printed trash digivolve uses 'may'");
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

    let stacked_to_security = compiled
        .effects
        .iter()
        .find_map(|effect| match effect {
            CompiledClause::Triggered(clause)
                if clause.when.contains(&CompiledTiming::EndOfYourTurn)
                    && clause.when.contains(&CompiledTiming::EndOfOpponentsTurn) =>
            {
                Some(clause)
            }
            _ => None,
        })
        .expect("end-of-all-turns top-stacked-card-to-security clause");
    assert!(matches!(
        stacked_to_security.process.as_slice(),
        [CompiledStep::SecurityPlaceTopStackedCard { carrier, of, position, face_up }]
            if *carrier == digimon_dsl::compiled::CompiledBindingRef::Source
                && *of == digimon_dsl::compiled::CompiledPlayerRef::You
                && *position == digimon_dsl::compiled::CompiledStackPosition::Top
                && !*face_up
    ));
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
fn bt20_084_trash_observer_decline_leaves_card_in_trash_and_sistermon_ciel_untouched() {
    // OPT/optional lockout: the printed "may" must allow declining; on decline
    // BT20-084 stays in trash and the field Sistermon Ciel is not digivolved.
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
    runner
        .decline_optional_trigger()
        .expect("decline the optional trash digivolve");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional trigger clears the prompt"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|card| card.handle() == bt20_084),
        "declined trash digivolve must leave BT20-084 in trash"
    );
    let stack = &runner.game.players[0].battle_area[base.index as usize];
    assert_eq!(
        stack.card_sources.len(),
        1,
        "declined trigger must not stack BT20-084 onto the Sistermon Ciel"
    );
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        "SISTERMON-CIEL-BASE",
        "Sistermon Ciel remains the untouched top card"
    );
}

#[test]
fn bt20_084_trash_observer_no_prompt_when_card_is_not_in_trash() {
    // Condition gating (negative): the [Trash] precondition fails when no
    // BT20-084 copy resides in trash, so playing an ally raises no prompt.
    let mut runner = bt20_084_runner()
        .hand(0, &["ALLY-DIGIMON"])
        .memory(10)
        .start();
    runner.place_on_field(0, "SISTERMON-CIEL-BASE", Some(0));

    runner.play(0, 0).expect("play allied Digimon");
    runner
        .auto_resolve()
        .expect("no trash-resident observer to fire");

    assert!(
        runner.pending_selection().is_none(),
        "no BT20-084 in trash means the [Trash] observer does not fire"
    );
}

#[test]
fn bt20_084_trash_observer_no_prompt_when_no_sistermon_ciel_on_field() {
    // Condition gating (negative): with BT20-084 in trash but no field
    // [Sistermon Ciel], there is no legal free-digivolve target, so no prompt.
    let mut runner = bt20_084_runner()
        .hand(0, &["ALLY-DIGIMON"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "BT20-084");

    runner.play(0, 0).expect("play allied Digimon");
    runner
        .auto_resolve()
        .expect("no Sistermon Ciel target available");

    assert!(
        runner.pending_selection().is_none(),
        "no field Sistermon Ciel means the trash observer has no legal target"
    );
}

#[test]
fn bt20_084_trash_observer_no_prompt_when_played_card_is_not_a_digimon() {
    // Condition gating (negative): `event_target_kind: digimon` gates the
    // observer — playing a Tamer must not fire the trash digivolve.
    let mut runner = bt20_084_runner().hand(0, &["OPP-TAMER"]).memory(10).start();
    runner.place_on_field(0, "SISTERMON-CIEL-BASE", Some(0));
    push_to_trash(&mut runner, 0, "BT20-084");

    runner.play(0, 0).expect("play a Tamer (not a Digimon)");
    runner
        .auto_resolve()
        .expect("playing a non-Digimon does not fire the observer");

    assert!(
        runner.pending_selection().is_none(),
        "the trash observer only fires when one of your Digimon is played"
    );
}

#[test]
fn bt20_084_end_of_all_turns_places_top_stacked_card_as_top_security() {
    let mut runner = bt20_084_runner().start();
    let carrier = runner.place_stack(0, &["SISTERMON-CIEL-BASE", "BT20-084"]);
    let moved_card =
        runner.game.players[0].battle_area[carrier.index as usize].card_sources[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .expect("security top")
            .handle(),
        moved_card,
        "top stacked card below BT20-084 should become top security"
    );
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&moved_card.0),
        "BT20-084 places the stacked card face-down (printed text gives no \
         'face up' instruction; CR 3-7-4 + DCGO AddSecurityCard faceUp:false)"
    );
    let remaining_stack = &runner.game.players[0].battle_area[carrier.index as usize].card_sources;
    assert_eq!(
        remaining_stack.len(),
        1,
        "carrier must remain legal with its visible BT20-084 top card"
    );
    assert_eq!(
        remaining_stack[0].card_id(&runner.game.card_data),
        "BT20-084"
    );
}

#[test]
fn bt20_084_end_of_all_turns_fires_on_the_opponents_turn_too() {
    // [End of All Turns] integrated coverage of the second timing: the
    // top-stacked-card-to-security movement also fires at the opponent's turn end.
    let mut runner = bt20_084_runner().start();
    let carrier = runner.place_stack(0, &["SISTERMON-CIEL-BASE", "BT20-084"]);
    let moved_card =
        runner.game.players[0].battle_area[carrier.index as usize].card_sources[0].handle();

    runner.game.enqueue_triggered(
        EffectTiming::EndOfOpponentsTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .expect("security top")
            .handle(),
        moved_card,
        "top stacked card should move to top security at the opponent's turn end too"
    );
}

#[test]
fn bt20_084_end_of_all_turns_no_op_when_no_digivolution_cards() {
    // Condition gating (negative): the C# CanActivate requires
    // DigivolutionCards.Count > 0. A lone BT20-084 with no card stacked beneath
    // it has no top stacked card to move, so security is left untouched.
    let mut runner = bt20_084_runner().start();
    let carrier = runner.place_on_field(0, "BT20-084", Some(0));
    let security_before = runner.game.players[0].security.len();

    runner.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[0].security.len(),
        security_before,
        "no stacked card under BT20-084 means nothing moves to security"
    );
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "the lone BT20-084 carrier is left intact"
    );
    assert!(
        runner.pending_selection().is_none(),
        "the gated end-of-turn clause resolves without surfacing a prompt"
    );
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
