//! BT9-108 Eye of the Gorgon — Option, Purple, Cost 8.
//!
//! # Card text (cards.json)
//!
//! [Main] Delete 1 of your opponent's unsuspended Digimon. If you do, you may
//! play 1 purple level 3 Digimon card from your trash without paying its memory
//! cost. Any [On Play] effects on the Digimon played with this effect don't
//! activate.
//!
//! Security Effect [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT9/Purple/BT9_108.cs
//!
//! # Patterns this test covers
//! - A4 trash → play recursion (play purple Lv.3 from trash).
//! - E2 optional decline on the trash-play sub-step ("you may play").
//! - F-adjacent: mandatory delete of an opponent **unsuspended** Digimon.
//! - PUPPETS-G030: `play_from_trash_free` with `suppress_on_play: true` —
//!   the played Digimon's own [On Play] effects do NOT activate.
//! - SECURITY-MIRROR: the [Security] clause re-runs the [Main] effect.

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A purple level-3 Digimon with **no** scripted [On Play] effect. The clean
/// trash-play candidate.
fn purple_l3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

/// A purple level-4 Digimon (wrong level) — must NOT be a legal trash-play
/// candidate (the clause filters on level 3).
fn purple_l4(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

/// A yellow level-3 Digimon (wrong color) — must NOT be a legal trash-play
/// candidate (the clause filters on purple).
fn yellow_l3(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.level = Some(3);
    card.dp = Some(2000);
    card
}

/// An opponent Digimon to delete.
fn opp_digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(5);
    card.dp = Some(6000);
    card
}

/// A purple level-3 Digimon whose [On Play] draws 1 card. The draw is the
/// observable signal: when BT9-108 plays this card with `suppress_on_play:
/// true`, the OnPlay `draw` must NOT fire.
const ONPLAY_PURPLE_L3_YAML: &str = r#"
card: ONPLAY-PURPLE-L3
name: OnPlay Purple Imp
kind: digimon
level: 3
color: [purple]
cost: 3
dp: 2000
effects:
  - when: on_play
    summary: "[On Play] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

/// Seed `player`'s trash with a single copy of `card_id`.
fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let card = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("seed_trash: unknown card_id {card_id}"));
        let next_idx = runner.game.next_card_index();
        digimon_engine::card_source::CardSource::new(data_idx, player as u8, next_idx)
    };
    runner.game.players[player].trash.push(card);
}

/// Base runner: BT9-108 in P0's hand, helper card-data registered, ample memory.
fn gorgon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT9-108")
        .expect("BT9-108 YAML parses and compiles")
        .from_dsl_yaml(ONPLAY_PURPLE_L3_YAML)
        .expect("ONPLAY-PURPLE-L3 inline DSL compiles")
        .add_card(purple_l3("PURPLE-L3"))
        .add_card(purple_l4("PURPLE-L4"))
        .add_card(yellow_l3("YELLOW-L3"))
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(opp_digimon("OPP-DIGI2"))
        .hand(0, &["BT9-108"])
        .deck(0, &["PURPLE-L3", "PURPLE-L3", "PURPLE-L3"])
        .memory(10)
        .start()
}

// ─── Section 1 — Structural assertions ────────────────────────────────────────

#[test]
fn bt9_108_is_purple_option_cost_8() {
    let runner = gorgon_runner();
    let compiled = runner
        .compiled_card("BT9-108")
        .expect("BT9-108 compiled card");

    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(8));
    assert!(
        compiled.color.contains(&CompiledColor::Purple),
        "Eye of the Gorgon must be a purple Option"
    );
}

#[test]
fn bt9_108_has_exactly_one_main_and_one_security_clause() {
    let runner = gorgon_runner();
    let compiled = runner
        .compiled_card("BT9-108")
        .expect("BT9-108 compiled card");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT9-108 ships exactly the [Main] clause and the [Security] mirror clause"
    );
}

#[test]
fn bt9_108_main_clause_is_face_up_main_from_hand() {
    let runner = gorgon_runner();
    let compiled = runner
        .compiled_card("BT9-108")
        .expect("BT9-108 compiled card");

    let main = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause present");

    assert_eq!(main.scope, CompiledScope::FaceUp);
}

#[test]
fn bt9_108_security_clause_is_inherited_on_security() {
    let runner = gorgon_runner();
    let compiled = runner
        .compiled_card("BT9-108")
        .expect("BT9-108 compiled card");

    let security = compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause present");

    assert_eq!(
        security.scope,
        CompiledScope::Inherited,
        "the [Security] mirror rides on the option's inherited surface"
    );
}

// ─── Section 2 — Condition gating (delete-target availability) ────────────────
//
// The printed "Delete 1 of your opponent's unsuspended Digimon" only resolves
// when an opponent UNSUSPENDED Digimon exists. The "If you do" gate then
// predicates the trash-play on a delete actually happening.

#[test]
fn bt9_108_main_with_unsuspended_opponent_digimon_prompts_delete() {
    let mut runner = gorgon_runner();
    runner.place_on_field(1, "OPP-DIGI", Some(0)); // unsuspended by default

    runner.play(0, 0).expect("play BT9-108 from hand");

    let sel = runner
        .pending_selection_view()
        .expect("[Main] must prompt to delete an opponent unsuspended Digimon");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert!(
        !sel.is_optional,
        "the printed 'Delete 1' is mandatory when a legal target exists"
    );
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "the single unsuspended opponent Digimon is the only legal delete target"
    );
}

#[test]
fn bt9_108_main_with_only_suspended_opponent_digimon_installs_no_delete() {
    let mut runner = gorgon_runner();
    let suspended = runner.place_on_field(1, "OPP-DIGI", Some(0));
    runner.game.suspend(suspended);

    runner.play(0, 0).expect("play BT9-108 from hand");

    assert!(
        runner.pending_selection().is_none(),
        "a suspended opponent Digimon is not a legal target — no delete prompt installs, \
         and the 'If you do' gate keeps the trash-play from running"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "the suspended opponent Digimon must not be deleted"
    );
}

// ─── Section 3 — Behavioral outcome per clause ────────────────────────────────

#[test]
fn bt9_108_main_only_unsuspended_digimon_offered_as_delete_target() {
    let mut runner = gorgon_runner();
    let unsuspended = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let suspended = runner.place_on_field(1, "OPP-DIGI2", Some(0));
    runner.game.suspend(suspended);

    runner.play(0, 0).expect("play BT9-108 from hand");

    let sel = runner
        .pending_selection_view()
        .expect("[Main] delete prompt installs");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, unsuspended.index as u16)],
        "only the unsuspended opponent Digimon is a legal delete target; the suspended one is filtered out"
    );
}

#[test]
fn bt9_108_main_deletes_chosen_unsuspended_digimon() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    // No purple Lv.3 in trash → the trash-play sub-step finds no candidates and
    // short-circuits; this test isolates the delete outcome.

    runner.play(0, 0).expect("play BT9-108 from hand");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "the chosen unsuspended opponent Digimon must be deleted"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the deleted opponent Digimon goes to its owner's trash"
    );
}

#[test]
fn bt9_108_main_after_delete_prompts_optional_purple_l3_trash_play() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-L3");
    seed_trash(&mut runner, 0, "PURPLE-L4"); // wrong level — filtered out
    seed_trash(&mut runner, 0, "YELLOW-L3"); // wrong color — filtered out

    runner.play(0, 0).expect("play BT9-108 from hand");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");

    let sel = runner
        .pending_selection_view()
        .expect("after the delete, the 'If you do' trash-play prompt installs");
    assert_eq!(
        sel.kind,
        SelectionKind::Trash,
        "the trash-play selects a Digimon from your own trash"
    );
    assert_eq!(
        sel.selecting_player, 0,
        "the controller chooses the card to play"
    );
    assert!(
        sel.is_optional,
        "the printed 'you may play' makes the trash play optional"
    );
    assert_eq!(
        sel.valid_action_ids.len(),
        1,
        "only the purple level-3 Digimon is a legal candidate \
         (the purple level-4 and yellow level-3 cards are filtered out)"
    );
}

#[test]
fn bt9_108_main_declining_trash_play_leaves_trash_card_in_place() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-L3");

    runner.play(0, 0).expect("play BT9-108 from hand");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");

    let battle_before = runner.battle_area_size(0);
    runner
        .execute_action(0, PASS)
        .expect("decline the optional trash play");

    assert!(
        runner.pending_selection().is_none(),
        "declining the optional trash play ends the effect"
    );
    assert_eq!(
        runner.battle_area_size(0),
        battle_before,
        "declining must not add any Digimon to your battle area"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PURPLE-L3"),
        "the declined purple Lv.3 stays in the trash"
    );
}

#[test]
fn bt9_108_main_plays_chosen_purple_l3_from_trash_into_battle_area() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-L3");

    runner.play(0, 0).expect("play BT9-108 from hand");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");

    // Snapshot the battle area right before the free play so the +1 delta
    // isolates the trash-played Digimon (the BT9-108 option permanent is
    // already accounted for here).
    let battle_before_free_play = runner.battle_area_size(0);

    let sel = runner
        .pending_selection_view()
        .expect("trash-play prompt installs");
    runner
        .execute_action(0, sel.valid_action_ids[0])
        .expect("play the purple Lv.3 from trash for free");
    runner.auto_resolve().expect("resolve the free play");

    assert_eq!(
        runner.battle_area_size(0),
        battle_before_free_play + 1,
        "the chosen purple Lv.3 Digimon enters your battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "PURPLE-L3"),
        "the played card is the purple Lv.3 from trash"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PURPLE-L3"),
        "the played card has left the trash"
    );
}

#[test]
fn bt9_108_main_free_play_pays_no_memory() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    seed_trash(&mut runner, 0, "PURPLE-L3");

    runner.play(0, 0).expect("play BT9-108 from hand");
    // Playing BT9-108 (cost 8) from 10 memory leaves 2.
    let memory_after_option = runner.memory();

    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");
    let sel = runner
        .pending_selection_view()
        .expect("trash-play prompt installs");
    runner
        .execute_action(0, sel.valid_action_ids[0])
        .expect("play the purple Lv.3 from trash for free");
    runner.auto_resolve().expect("resolve the free play");

    assert_eq!(
        runner.memory(),
        memory_after_option,
        "playing the purple Lv.3 from trash costs no memory"
    );
}

// ─── Section 4 — On Play suppression (PUPPETS-G030) ───────────────────────────

#[test]
fn bt9_108_main_suppresses_on_play_of_played_digimon() {
    let mut runner = gorgon_runner();
    let victim = runner.place_on_field(1, "OPP-DIGI", Some(0));
    seed_trash(&mut runner, 0, "ONPLAY-PURPLE-L3");

    runner.play(0, 0).expect("play BT9-108 from hand");
    runner
        .execute_action(0, encode_attack(0, victim.index as u16))
        .expect("delete the unsuspended opponent Digimon");

    // Snapshot hand/deck right before the free play so the assertion isolates
    // the OnPlay draw delta (playing BT9-108 itself already emptied the hand).
    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let sel = runner
        .pending_selection_view()
        .expect("trash-play prompt installs");
    runner
        .execute_action(0, sel.valid_action_ids[0])
        .expect("play the OnPlay purple Lv.3 from trash for free");
    runner.auto_resolve().expect("resolve the free play");

    // The OnPlay Digimon entered the battle area.
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ONPLAY-PURPLE-L3"),
        "the OnPlay purple Lv.3 entered the battle area"
    );
    // suppress_on_play: true — the [On Play] draw must NOT fire.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "the suppressed [On Play] draw must not add a card to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "the suppressed [On Play] draw must not consume a card from the deck"
    );
}

/// Scoping proof: suppression is scoped to BT9-108's play event only. The SAME
/// OnPlay Digimon, played through an ordinary effect-play-from-trash path (no
/// suppression), DOES fire its [On Play] draw. Guards against a global OnPlay
/// disable.
#[test]
fn bt9_108_on_play_suppression_does_not_leak_to_normal_plays() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ONPLAY_PURPLE_L3_YAML)
        .expect("ONPLAY-PURPLE-L3 inline DSL compiles")
        .deck(0, &["ONPLAY-PURPLE-L3", "ONPLAY-PURPLE-L3"])
        .memory(10)
        .start();
    seed_trash(&mut runner, 0, "ONPLAY-PURPLE-L3");

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    runner
        .game
        .play_from_trash(0, 0)
        .expect("OnPlay Digimon plays from trash");
    runner.auto_resolve().expect("resolve [On Play] draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "an unsuppressed [On Play] draw must add a card to the hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "an unsuppressed [On Play] draw must consume a card from the deck"
    );
}

// ─── Section 5 — [Security] mirror clause ─────────────────────────────────────
//
// "[Security] Activate this card's [Main] effects." — the inherited Security
// clause re-runs the [Main] effect when BT9-108 is checked from security.

/// Build a runner where P0 attacks P1, whose single Security card is BT9-108.
/// P0 also has a second, NON-attacking Digimon that stays unsuspended — it is
/// the legal delete target for the mirrored [Main] effect (the attacker itself
/// suspends on attack declaration, so `is_unsuspended` filters it out).
/// Returns the runner, the attacker handle, and the bystander handle.
fn security_runner() -> (
    DebugRunner,
    digimon_engine::permanent::PermanentHandle,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT9-108")
        .expect("BT9-108 YAML parses and compiles")
        .add_card(attacker)
        .add_card(opp_digimon("OPP-DIGI"))
        .add_card(purple_l3("PURPLE-L3"))
        .security(1, &["BT9-108"])
        .deck(1, &["ATTACKER", "ATTACKER", "ATTACKER"])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    // A second unsuspended P0 Digimon that does NOT attack — the mirror's
    // delete target.
    let bystander = runner.place_on_field(0, "OPP-DIGI", Some(0));
    (runner, attacker, bystander)
}

#[test]
fn bt9_108_security_mirror_prompts_to_delete_unsuspended_digimon() {
    // P1 reveals BT9-108 from security on P0's attack. The [Security] mirror
    // runs the [Main] effect for P1 (the security owner), targeting P0's
    // unsuspended Digimon. The attacker suspended on attack declaration, so the
    // bystander is the legal target.
    let (mut runner, attacker, bystander) = security_runner();

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("BT9-108 [Security] mirror must install a delete prompt");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert_eq!(
        sel.selecting_player, 1,
        "the security owner controls the mirrored [Main] effect"
    );
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, bystander.index as u16)],
        "only P0's unsuspended bystander is a legal delete target — the attacker \
         suspended on attack declaration and is filtered out by is_unsuspended"
    );
}

#[test]
fn bt9_108_security_mirror_deletes_chosen_digimon() {
    let (mut runner, attacker, _bystander) = security_runner();

    let battle_before = runner.battle_area_size(0);

    runner.attack_player(attacker, 1, false);

    let sel = runner
        .pending_selection_view()
        .expect("[Security] mirror delete prompt installs");
    let pick = sel.valid_action_ids[0];
    runner
        .execute_action(1, pick)
        .expect("security owner deletes the chosen unsuspended Digimon");
    runner
        .auto_resolve()
        .expect("resolve the rest of the mirror");

    assert_eq!(
        runner.battle_area_size(0),
        battle_before - 1,
        "the mirrored [Main] delete removes P0's unsuspended bystander Digimon"
    );
}
