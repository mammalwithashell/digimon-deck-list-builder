//! BT12-092 waives payment, not the result card's digivolution requirements.
//! Official Q1740 and CR4.3 8-1-3-1; tokens additionally obey 4-21-3.
//! Real shipping Marcus/Ruin Mode effects and production printed card data.
use digimon_engine::{
    card_data::CardData,
    debug_runner::DebugRunner,
    enums::GamePhase,
    events::GameEvent,
    permanent::PermanentHandle,
};

const MARCUS: &str = "BT12-092";
const RUIN: &str = "EX4-074";

fn fixture(player: u8, base: &str, hand: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let data = CardData::load_from_file(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json"
    ))).expect("unchanged production cards.json");
    let mut r = DebugRunner::builder()
        .dsl_card(MARCUS).expect("shipping Marcus effect")
        .dsl_card(RUIN).expect("shipping Ruin Mode effect and named route")
        .with_card_data(data)
        .hand(player, hand)
        .deck(player, &["ST1-03"; 5])
        .deck(1-player, &["ST1-03"; 5])
        .memory(2)
        .start();
    r.game.turn_order = vec![player, 1-player];
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    let marcus = r.place_on_field(player, MARCUS, Some(0));
    let host = r.place_on_field(player, base, Some(0));
    r.game.drain_events();
    (r, marcus, host)
}

fn rejects_base(base: &str) {
    for player in 0..2 {
        let (mut r, marcus, host) = fixture(player, base, &[RUIN]);
        let original = r.top_card(host);
        let hand: Vec<_> = r.game.player(player).hand.iter().map(|c| c.handle()).collect();
        let deck: Vec<_> = r.game.player(player).deck.iter().map(|c| c.handle()).collect();
        let memory = r.memory();
        r.game.suspend(marcus);
        assert!(r.pending_selection().is_none(), "Marcus must not offer an illegal digivolution base: {base}");
        assert_eq!(r.top_card(host), original);
        assert_eq!(r.game.player(player).hand.iter().map(|c| c.handle()).collect::<Vec<_>>(), hand);
        assert_eq!(r.game.player(player).deck.iter().map(|c| c.handle()).collect::<Vec<_>>(), deck);
        assert_eq!(r.memory(), memory);
        assert!(!r.game.drain_events().iter().any(|e| matches!(e, GameEvent::EvalDigivolve { .. })));
    }
}

fn accepts_base(base: &str) {
    for player in 0..2 {
        let (mut r, marcus, host) = fixture(player, base, &[RUIN]);
        let result = r.game.player(player).hand[0].handle();
        let original = r.top_card(host);
        let memory = r.memory();
        let deck_len = r.game.player(player).deck.len();
        r.game.suspend(marcus);
        assert!(r.pending_selection().is_some(), "legal Marcus evolution must remain available");
        for _ in 0..16 {
            let Some(view) = r.pending_selection_view() else { break };
            let action = *view.valid_action_ids.first().expect("legal decision is exposed");
            assert!(digimon_engine::build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.5);
            r.execute_action(view.selecting_player, action).expect("choose through the legal mask");
        }
        assert!(r.pending_selection().is_none(), "finite effect resolution");
        assert_eq!(r.top_card(host), result, "Marcus must digivolve the exact physical Ruin Mode card");
        assert!(r.game.player(player).battle_area[host.index as usize].card_sources.iter().any(|c| c.handle()==original));
        assert!(!r.game.player(player).hand.iter().any(|c| c.handle()==result));
        assert_eq!(r.memory(), memory, "payment is waived even when printed cost exceeds available memory");
        assert_eq!(r.game.player(player).deck.len(), deck_len-1, "exactly one digivolution draw");
    }
}

#[test]
fn marcus_cannot_skip_levels_from_yellow_agumon_to_ruin_mode() { rejects_base("BT12-034"); }
#[test]
fn marcus_cannot_ignore_wrong_level6_colors() { rejects_base("BT12-070"); }
#[test]
fn ruin_mode_named_requirement_is_not_a_shinegreymon_substring() { rejects_base(RUIN); }
#[test]
fn marcus_never_offers_a_petrification_token_as_an_evolution_base() { rejects_base("TOKEN_PETRIFICATION"); }
#[test]
fn marcus_can_digivolve_yellow_level6_into_ruin_mode_free() { accepts_base("BT1-063"); }
#[test]
fn marcus_can_digivolve_purple_level6_into_ruin_mode_free() { accepts_base("BT4-087"); }
#[test]
fn marcus_can_use_ruin_modes_exact_shinegreymon_route_free() { accepts_base("BT12-043"); }
#[test]
fn marcus_hand_mask_excludes_greymons_that_cannot_evolve_onto_the_selected_base() {
    for player in 0..2 {
        let (mut r, marcus, _) = fixture(player, "BT1-063", &[RUIN, "BT12-038"]);
        r.game.suspend(marcus);
        // The optional trigger confirmation precedes the base selection.
        // Drive both public prompts; inspect the physical hand menu only
        // after the selected host has actually been bound.
        for expected in [digimon_engine::selection::SelectionKind::Replacement,
                         digimon_engine::selection::SelectionKind::OwnField] {
            let view = r.pending_selection_view().expect("Marcus selection is installed");
            assert_eq!(view.kind, expected, "outer optional trigger, then eligible host");
            let action = view.valid_action_ids[0];
            assert!(digimon_engine::build_action_mask(&r.game, view.selecting_player)[action as usize] > 0.5);
            r.execute_action(view.selecting_player, action).expect("confirm trigger and choose eligible base");
        }
        assert!(matches!(r.pending_selection_view().expect("hand choice").kind,
            digimon_engine::selection::SelectionKind::Hand));
        let hand_view = r.pending_selection_view().expect("hand menu");
        assert_eq!(hand_view.zone_owner, Some(player));
        let mask = digimon_engine::build_action_mask(&r.game, player);
        // This DSL primitive uses the ordinary phase-aware hand contract:
        // PLAY_HAND_START + physical hand position, not OwnedCardChoice rows.
        let offered: Vec<_> = hand_view.valid_action_ids.iter().map(|action| {
            assert!(mask[*action as usize] > 0.5);
            let index = action.checked_sub(digimon_engine::action::space::PLAY_HAND_START)
                .expect("phase-aware hand action") as usize;
            r.game.player(player).hand[index].card_id(&r.game.card_data)
        }).collect();
        assert_eq!(offered, vec![RUIN],
            "yellow/Greymon name is not enough; the selected base must satisfy the result's requirements");
    }
}
