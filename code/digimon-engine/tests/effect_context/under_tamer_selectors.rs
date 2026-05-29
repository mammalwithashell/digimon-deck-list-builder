use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn card(id: &str, kind: CardKind) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: kind,
        level: matches!(kind, CardKind::Digimon).then_some(4),
        dp: matches!(kind, CardKind::Digimon).then_some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn insert_bottom_source(
    runner: &mut DebugRunner,
    host: PermanentHandle,
    card_id: &str,
) -> CardHandle {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|data| data.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown test card {card_id}"));
    let card = CardSource::new(data_idx, host.player, runner.game.next_card_index());
    let handle = card.handle();
    runner.game.players[host.player as usize].battle_area[host.index as usize]
        .card_sources
        .insert(0, card);
    handle
}

fn is_match(game: &digimon_engine::game::Game, source_ref: SourceSelectionRef) -> bool {
    game.card_data_for_handle(source_ref.card)
        .is_some_and(|data| data.card_id.starts_with("MATCH"))
}

#[test]
fn select_own_tamer_sources_spans_own_tamers_and_preserves_origin() {
    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER-A", CardKind::Tamer))
        .add_card(card("TAMER-B", CardKind::Tamer))
        .add_card(card("OPP-TAMER", CardKind::Tamer))
        .add_card(card("DIGI-HOST", CardKind::Digimon))
        .add_card(card("MATCH-A", CardKind::Digimon))
        .add_card(card("MATCH-B", CardKind::Digimon))
        .add_card(card("MATCH-OPP", CardKind::Digimon))
        .add_card(card("MATCH-DIGI", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer_a = runner.place_on_field(0, "TAMER-A", None);
    let tamer_b = runner.place_on_field(0, "TAMER-B", None);
    let digi_host = runner.place_on_field(0, "DIGI-HOST", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", None);

    let _match_a = insert_bottom_source(&mut runner, tamer_a, "MATCH-A");
    let match_b = insert_bottom_source(&mut runner, tamer_b, "MATCH-B");
    let _match_digi = insert_bottom_source(&mut runner, digi_host, "MATCH-DIGI");
    let _match_opp = insert_bottom_source(&mut runner, opp_tamer, "MATCH-OPP");

    let picked = Arc::new(Mutex::new(Vec::<SourceSelectionRef>::new()));
    let source_card = runner.game.players[0].battle_area[tamer_a.index as usize]
        .top_card()
        .handle();

    {
        let picked = Arc::clone(&picked);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer_a), 0);
        ctx.select_own_tamer_sources(
            "Choose a matching card under one of your Tamers",
            1,
            1,
            is_match,
            move |_ctx, refs| {
                *picked.lock().unwrap() = refs;
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("matching own Tamer sources install a prompt");
    let tamer_a_action = encode_source_select(tamer_a.index as u16, 0).unwrap();
    let tamer_b_action = encode_source_select(tamer_b.index as u16, 0).unwrap();
    let digi_action = encode_source_select(digi_host.index as u16, 0).unwrap();

    assert_eq!(
        pending.valid_action_ids,
        vec![tamer_a_action, tamer_b_action]
    );
    assert!(!pending.valid_action_ids.contains(&digi_action));

    runner
        .game
        .resolve_selection(pending.selecting_player, tamer_b_action)
        .expect("resolve under-Tamer source selection");

    let picked = picked.lock().unwrap();
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].permanent, tamer_b);
    assert_eq!(picked[0].field_index, tamer_b.index);
    assert_eq!(picked[0].source_index, 0);
    assert_eq!(picked[0].card, match_b);
}

#[test]
fn select_sources_under_one_own_tamer_excludes_other_tamer_stashes() {
    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER-A", CardKind::Tamer))
        .add_card(card("TAMER-B", CardKind::Tamer))
        .add_card(card("MATCH-A", CardKind::Digimon))
        .add_card(card("MATCH-B", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer_a = runner.place_on_field(0, "TAMER-A", None);
    let tamer_b = runner.place_on_field(0, "TAMER-B", None);

    let match_a = insert_bottom_source(&mut runner, tamer_a, "MATCH-A");
    let _match_b = insert_bottom_source(&mut runner, tamer_b, "MATCH-B");

    let picked = Arc::new(Mutex::new(Vec::<SourceSelectionRef>::new()));
    let source_card = runner.game.players[0].battle_area[tamer_a.index as usize]
        .top_card()
        .handle();

    {
        let picked = Arc::clone(&picked);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer_a), 0);
        ctx.select_sources_under_own_tamer(
            tamer_a,
            "Choose a matching card under this Tamer",
            1,
            1,
            is_match,
            move |_ctx, refs| {
                *picked.lock().unwrap() = refs;
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("matching source under the chosen Tamer installs a prompt");
    let tamer_a_action = encode_source_select(tamer_a.index as u16, 0).unwrap();
    let tamer_b_action = encode_source_select(tamer_b.index as u16, 0).unwrap();
    assert_eq!(pending.valid_action_ids, vec![tamer_a_action]);
    assert!(!pending.valid_action_ids.contains(&tamer_b_action));

    runner
        .game
        .resolve_selection(pending.selecting_player, tamer_a_action)
        .expect("resolve specific Tamer source selection");

    let picked = picked.lock().unwrap();
    assert_eq!(picked.len(), 1);
    assert_eq!(picked[0].permanent, tamer_a);
    assert_eq!(picked[0].card, match_a);
}

#[test]
fn select_own_tamer_sources_with_no_matching_stash_skips_prompt() {
    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER-A", CardKind::Tamer))
        .add_card(card("OTHER", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer_a = runner.place_on_field(0, "TAMER-A", None);
    insert_bottom_source(&mut runner, tamer_a, "OTHER");

    let source_card = runner.game.players[0].battle_area[tamer_a.index as usize]
        .top_card()
        .handle();
    let called = Arc::new(Mutex::new(false));

    {
        let called = Arc::clone(&called);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer_a), 0);
        ctx.select_own_tamer_sources(
            "Choose a matching card under one of your Tamers",
            1,
            1,
            is_match,
            move |_ctx, _refs| {
                *called.lock().unwrap() = true;
            },
        );
    }

    assert!(
        runner.game.pending_selection.is_none(),
        "no matching under-Tamer sources should not install a prompt"
    );
    assert!(
        !*called.lock().unwrap(),
        "required no-target selection should not call the callback"
    );
}

#[test]
fn select_own_tamer_sources_optional_decline_returns_empty_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(card("TAMER-A", CardKind::Tamer))
        .add_card(card("MATCH-A", CardKind::Digimon))
        .memory(5)
        .start();

    let tamer_a = runner.place_on_field(0, "TAMER-A", None);
    insert_bottom_source(&mut runner, tamer_a, "MATCH-A");

    let source_card = runner.game.players[0].battle_area[tamer_a.index as usize]
        .top_card()
        .handle();
    let picked = Arc::new(Mutex::new(None::<Vec<SourceSelectionRef>>));

    {
        let picked = Arc::clone(&picked);
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(tamer_a), 0);
        ctx.select_own_tamer_sources(
            "Optionally choose a matching card under one of your Tamers",
            0,
            1,
            is_match,
            move |_ctx, refs| {
                *picked.lock().unwrap() = Some(refs);
            },
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("optional matching source still installs a prompt");
    assert!(pending.is_optional);
    assert!(pending.valid_action_ids.contains(&PASS));

    runner
        .game
        .resolve_selection(pending.selecting_player, PASS)
        .expect("resolve optional decline");

    assert_eq!(*picked.lock().unwrap(), Some(Vec::new()));
    assert_eq!(
        runner.game.players[0].battle_area[tamer_a.index as usize]
            .card_sources
            .len(),
        2,
        "declining does not move or consume the source"
    );
}

#[test]
fn select_own_tamer_sources_ignores_opponent_only_tamer_stash() {
    let mut runner = DebugRunner::builder()
        .add_card(card("OWN-TAMER", CardKind::Tamer))
        .add_card(card("OPP-TAMER", CardKind::Tamer))
        .add_card(card("MATCH-OPP", CardKind::Digimon))
        .memory(5)
        .start();

    let own_tamer = runner.place_on_field(0, "OWN-TAMER", None);
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", None);
    insert_bottom_source(&mut runner, opp_tamer, "MATCH-OPP");

    let source_card = runner.game.players[0].battle_area[own_tamer.index as usize]
        .top_card()
        .handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(own_tamer), 0);
        ctx.select_own_tamer_sources(
            "Choose a matching card under one of your Tamers",
            1,
            1,
            is_match,
            |_ctx, _refs| panic!("opponent-only stash must not be selectable"),
        );
    }

    assert!(
        runner.game.pending_selection.is_none(),
        "matching cards under opponent Tamers are not legal own-Tamer source choices"
    );
}
