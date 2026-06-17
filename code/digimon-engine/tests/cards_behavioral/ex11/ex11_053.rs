//! EX11-053 Omekamon.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{
    encode_attack, encode_source_select, PASS, PLAY_HAND_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

fn digimon(card_id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(5000);
    card.play_cost = 5;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

/// An [Omnimon (X Antibody)] card — the only legal play-target of EX11-053's
/// On Deletion clause. White, Mega, with a real play cost (DCGO gates on
/// `HasPlayCost`), so the free play is meaningfully a discount.
fn omnimon_x(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, "Omnimon (X Antibody)");
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::White];
    card.level = Some(7);
    card.dp = Some(15000);
    card.play_cost = 16;
    card.traits = vec!["X Antibody".to_string(), "Royal Knight".to_string()];
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(digimon("NON-RK", "Plain Digimon", &[]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .memory(10)
        .start()
}

#[test]
fn ex11_053_has_printed_metadata_and_on_play_clause() {
    let runner = runner();
    let card = runner.compiled_card("EX11-053").expect("compiled card");

    assert_eq!(card.name, "Omekamon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(card.traits.iter().any(|name| name == "Puppet"));
    assert!(card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnPlay)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::PlaceAsBottomSource { .. }))
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::Draw { count: 1, .. }))
        }
        _ => false,
    }));
}

#[test]
fn ex11_053_on_play_places_royal_knight_from_hand_under_fielded_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(digimon("NON-RK", "Plain Digimon", &[]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["EX11-053", "ROYAL", "NON-RK"])
        .deck(0, &["DRAW"])
        .memory(20)
        .start();
    let king = runner.place_on_field(0, "KING", Some(0));
    let royal_handle = runner.game.players[0].hand[1].handle();

    runner.play(0, 0).expect("play Omekamon");
    let king_prompt = runner
        .pending_selection_view()
        .expect("King Drasil selection opens");
    assert_eq!(king_prompt.kind, SelectionKind::OwnField);
    assert_eq!(
        king_prompt.valid_action_ids,
        vec![encode_attack(0, king.index as u16)]
    );
    runner
        .execute_action(0, encode_attack(0, king.index as u16))
        .expect("choose King Drasil");

    let hand_prompt = runner
        .pending_selection_view()
        .expect("Royal Knight hand selection opens");
    assert_eq!(hand_prompt.kind, SelectionKind::Hand);
    assert_eq!(
        hand_prompt.valid_action_ids,
        vec![PLAY_HAND_START],
        "after Omekamon leaves hand, only Magnamon is a legal Royal Knight"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("choose Magnamon");
    runner.auto_resolve().expect("place source and draw");

    let king_perm = &runner.game.players[0].battle_area[king.index as usize];
    assert_eq!(king_perm.card_sources[0].handle(), royal_handle);
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW"),
        "Omekamon draws after the Royal Knight card is placed"
    );
}

#[test]
fn ex11_053_on_play_short_circuits_without_fielded_king_drasil() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .add_card(make_test_card("DRAW", "Draw Filler"))
        .hand(0, &["EX11-053", "ROYAL"])
        .deck(0, &["DRAW"])
        .memory(20)
        .start();

    runner.play(0, 0).expect("play Omekamon");

    assert!(runner.pending_selection().is_none());
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "ROYAL"),
        "Royal Knight stays in hand when there is no fielded King Drasil"
    );
    assert!(
        runner.game.players[0]
            .deck
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "DRAW"),
        "draw does not occur when the placement cost cannot be paid"
    );
}

// ── [On Deletion] union hand/source play + attach-self (G-UNION-HAND-SOURCE-PLAY) ──
//
// Printed: "[On Deletion] If you have 1 or fewer security cards, you may play 1
// [Omnimon (X Antibody)] from your hand or under your [King Drasil_7D6]s on the
// field without paying the cost. Then, place this card as the played Digimon's
// bottom digivolution card." (DCGO EX11_053.cs: hand Omnimon X OR a field/breeding
// King Drasil carrier whose sources contain an Omnimon X; then AddDigivolution-
// CardsBottom this Omekamon under the played card.)

/// Build a runner with EX11-053 (Omekamon) on P0's field at index 0, ready to
/// be deleted to fire its On Deletion clause. Adds the helper card defs.
fn deletion_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(omnimon_x("OMNI-X"))
        .add_card(digimon("NON-KING", "Plain Carrier", &[]))
        .add_card(digimon("ROYAL", "Magnamon", &["Royal Knight"]))
        .security(0, &[]) // 0 security → gate (<=1) satisfied
        .memory(10)
        .start()
}

/// Inject a card directly into player 0's hand after staging.
fn inject_hand(runner: &mut DebugRunner, card_id: &str) {
    runner
        .game
        .stage_inject_card(0, card_id, "hand")
        .unwrap_or_else(|e| panic!("inject_hand: {e}"));
}

/// Delete the Omekamon at battle-area index `idx` to fire its On Deletion.
fn delete_omekamon(runner: &mut DebugRunner, idx: usize) {
    runner
        .game
        .delete_permanent_with_effects(PermanentHandle {
            player: 0,
            index: idx as u8,
        });
}

/// (DISCRIMINATING first-fail) The On Deletion union-zone material scan must be
/// restricted to [King Drasil_7D6] carriers: an [Omnimon (X Antibody)] sitting
/// under a NON-King-Drasil Digimon must NOT be offered. Pre-fix the material
/// scan offers every field permanent's sources, so the non-King source leaks in.
#[test]
fn ex11_053_on_deletion_material_scan_restricted_to_king_drasil_carriers() {
    let mut runner = deletion_runner();

    // King Drasil carrier (index 0) with an Omnimon X source beneath its top.
    let king = runner.place_stack(0, &["OMNI-X", "KING"]);
    // A NON-King-Drasil carrier (index 1) ALSO holding an Omnimon X source.
    let non_king = runner.place_stack(0, &["OMNI-X", "NON-KING"]);
    // Omekamon on field at index 2 — the one we delete.
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));

    delete_omekamon(&mut runner, omekamon.index as usize);

    let pending = runner
        .pending_selection()
        .expect("On Deletion surfaces a union-zone selection");

    // The Omnimon X under the King Drasil carrier (source index 0) is legal.
    let king_src = encode_source_select(king.index as u16, 0).expect("king source action");
    // The Omnimon X under the non-King carrier (source index 0) must be MASKED.
    let non_king_src =
        encode_source_select(non_king.index as u16, 0).expect("non-king source action");

    let ids = &pending.valid_action_ids;
    assert!(
        ids.contains(&king_src),
        "Omnimon X under King Drasil_7D6 must be a legal pick; ids = {ids:?}"
    );
    assert!(
        !ids.contains(&non_king_src),
        "Omnimon X under a NON-King-Drasil Digimon must NOT be offered; ids = {ids:?}"
    );
    // Only the King Drasil source is legal (no hand candidates here).
    assert_eq!(
        ids.len(),
        1,
        "exactly one legal candidate (the King Drasil source); ids = {ids:?}"
    );
}

/// (a) Play an Omnimon X from HAND, then attach this Omekamon as its bottom
/// digivolution source.
#[test]
fn ex11_053_on_deletion_plays_omnimon_x_from_hand_and_attaches_self() {
    let mut runner = deletion_runner();
    inject_hand(&mut runner, "OMNI-X");
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));
    let omekamon_handle = runner.game.player(0).battle_area[omekamon.index as usize]
        .top_card()
        .handle();

    delete_omekamon(&mut runner, omekamon.index as usize);

    let pending = runner
        .pending_selection()
        .expect("On Deletion union-zone selection opens");
    assert!(pending.is_optional, "printed 'you may play' must expose PASS");
    // Hand Omnimon X is the only legal pick (hand index 0).
    assert_eq!(
        pending.valid_action_ids,
        vec![PLAY_HAND_START],
        "the hand Omnimon X is the single legal candidate"
    );

    let mem_before = runner.memory();
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("play Omnimon X from hand");
    runner.auto_resolve().ok();

    // Omnimon X entered the battle area for free.
    let omni_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "OMNI-X")
        .expect("Omnimon X enters the battle area");
    assert_eq!(
        runner.memory(),
        mem_before,
        "the Omnimon X is played without paying its cost"
    );
    // Omekamon (now in trash) is tucked as the played Omnimon X's BOTTOM source.
    assert_eq!(
        omni_perm.card_sources[0].handle(),
        omekamon_handle,
        "Omekamon is placed as Omnimon X's bottom digivolution source"
    );
    assert!(
        !runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.handle() == omekamon_handle),
        "Omekamon left the trash to become the bottom source"
    );
}

/// (b) Play an Omnimon X from UNDER a fielded King Drasil_7D6 (its source), then
/// attach this Omekamon as the played card's bottom digivolution source.
#[test]
fn ex11_053_on_deletion_plays_omnimon_x_from_king_drasil_source_and_attaches_self() {
    let mut runner = deletion_runner();
    let king = runner.place_stack(0, &["OMNI-X", "KING"]);
    let omni_source_handle = runner.game.player(0).battle_area[king.index as usize].card_sources[0]
        .handle();
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));
    let omekamon_handle = runner.game.player(0).battle_area[omekamon.index as usize]
        .top_card()
        .handle();

    let king_sources_before =
        runner.game.player(0).battle_area[king.index as usize].card_sources.len();

    delete_omekamon(&mut runner, omekamon.index as usize);

    let pending = runner
        .pending_selection()
        .expect("On Deletion union-zone selection opens");
    let king_src = encode_source_select(king.index as u16, 0).expect("king source action");
    assert_eq!(
        pending.valid_action_ids,
        vec![king_src],
        "the King Drasil Omnimon X source is the single legal candidate"
    );

    runner
        .execute_action(0, king_src)
        .expect("play Omnimon X from King Drasil's sources");
    runner.auto_resolve().ok();

    // The Omnimon X left the King Drasil stack and entered the battle area.
    let king_sources_after =
        runner.game.player(0).battle_area[king.index as usize].card_sources.len();
    assert_eq!(
        king_sources_after,
        king_sources_before - 1,
        "the played Omnimon X leaves the King Drasil stack"
    );
    let omni_perm = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| p.top_card().handle() == omni_source_handle)
        .expect("the King Drasil Omnimon X source enters the battle area");
    assert_eq!(
        omni_perm.card_sources[0].handle(),
        omekamon_handle,
        "Omekamon is placed as the played Omnimon X's bottom digivolution source"
    );
}

/// (c) An [Omnimon (X Antibody)] under a NON-King-Drasil Digimon, with no other
/// legal source, must produce NO union selection at all (no candidate).
#[test]
fn ex11_053_on_deletion_no_prompt_when_only_omnimon_x_is_under_non_king() {
    let mut runner = deletion_runner();
    runner.place_stack(0, &["OMNI-X", "NON-KING"]);
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));

    delete_omekamon(&mut runner, omekamon.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "an Omnimon X under a non-King-Drasil Digimon is not a legal candidate, so no selection opens"
    );
}

/// (d) With more than 1 security card the clause does not fire at all.
#[test]
fn ex11_053_on_deletion_does_not_fire_above_one_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 YAML loads")
        .add_card(digimon("KING", "King Drasil_7D6", &["CS"]))
        .add_card(omnimon_x("OMNI-X"))
        .add_card(make_test_card("SEC1", "Sec One"))
        .add_card(make_test_card("SEC2", "Sec Two"))
        .security(0, &["SEC1", "SEC2"]) // 2 security → gate (<=1) fails
        .memory(10)
        .start();
    inject_hand(&mut runner, "OMNI-X");
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));

    delete_omekamon(&mut runner, omekamon.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "with 2 security the On Deletion clause must not fire (gate is <= 1)"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OMNI-X"),
        "Omnimon X stays in hand when the clause does not fire"
    );
}

/// (e) Declining the optional play (PASS) leaves the board unchanged — Omekamon
/// goes to trash via normal deletion, no Omnimon X is played.
#[test]
fn ex11_053_on_deletion_decline_leaves_board_unchanged() {
    let mut runner = deletion_runner();
    inject_hand(&mut runner, "OMNI-X");
    let omekamon = runner.place_on_field(0, "EX11-053", Some(0));
    let omekamon_handle = runner.game.player(0).battle_area[omekamon.index as usize]
        .top_card()
        .handle();
    let battle_before = runner.game.player(0).battle_area.len();

    delete_omekamon(&mut runner, omekamon.index as usize);

    let pending = runner
        .pending_selection()
        .expect("On Deletion union-zone selection opens");
    assert!(pending.is_optional, "must be declinable");
    runner
        .execute_action(0, PASS)
        .expect("decline the optional play");
    runner.auto_resolve().ok();

    // Omekamon was deleted (index 0 was removed), so the board shrank by 1.
    assert_eq!(
        runner.game.player(0).battle_area.len(),
        battle_before - 1,
        "declining adds no new permanent; only Omekamon's deletion removed one"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OMNI-X"),
        "no Omnimon X is played when the optional play is declined"
    );
    assert!(
        runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.handle() == omekamon_handle),
        "declining lets normal deletion move Omekamon to trash"
    );
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "OMNI-X"),
        "the hand Omnimon X stays in hand when declined"
    );
}
