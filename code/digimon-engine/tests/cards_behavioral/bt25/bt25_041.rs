//! BT25-041 Murasamemon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Beastkin, Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! <Alliance>.
//! [When Digivolving] [When Attacking] [Once Per Turn] If it's your turn, by
//!   adding your top security card to the hand or trashing the bottom face-down
//!   card under any of your Tamers, you may play or use 1 [Glowing Dawn] card
//!   from your hand with the cost reduced by 3.
//! Inherited: [End of Attack] [Once Per Turn] By trashing the bottom face-down
//!   card from under any of your Tamers, this [Glowing Dawn] Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_041.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - H10 Alliance (grant)
//! - alt-digivolve from Glowing Dawn Lv.4
//! - G-PLAY-OR-USE-FROM-HAND: the [WD][WA][OPT] 2-way pay-cost choice
//!   (add-top-security-to-hand | trash-FD-under-Tamer) → play/use 1 [Glowing
//!   Dawn] card from hand at cost-3 (Digimon play OR Option use), optional decline
//! - inherited End-of-Attack OPT: trash-FD-under-Tamer (process cost) → unsuspend self
//!
//! # Verdict — IMPLEMENTED
//! The main [WD][WA][OPT] clause is now implemented on the unified
//! `play_or_use_from_hand` substrate (G-PLAY-OR-USE-FROM-HAND): an optional
//! engagement gate + a `select_effect_choice` 2-way pay-cost menu (add top
//! security to hand OR trash a face-down card under a Tamer), each branch gated
//! by its resource, then an optional `select_hand` over [Glowing Dawn] cards →
//! `play_or_use_from_hand` at cost reduced by 3. Alliance, the Glowing Dawn
//! alt-digivolve, and the inherited unsuspend are also IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-041";

fn murasamemon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// A Glowing Dawn HOST Digimon — Murasamemon's inherited [End of Attack] clause
/// applies to this host when Murasamemon is a digivolution source beneath it.
fn make_glowing_dawn_host(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(9000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A [Glowing Dawn] Digimon to PLAY from hand (cost 5 → cost-3 = pay 2).
fn gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A [Glowing Dawn] Option to USE from hand (use cost 5 → cost-3 = pay 2).
fn gd_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// Non-Glowing-Dawn security filler (so security loss never starts a battle).
fn sec_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn hand_index_of(runner: &DebugRunner, p: u8, card_id: &str) -> usize {
    runner.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand of p{p}"))
}

/// Build a runner with Murasamemon on p0's field (suspended-irrelevant), with
/// `sec_count` face-down security cards on p0 (so the "add top security" pay
/// option is available), and the standard card pool. p0 is the active player.
fn main_clause_base(sec_count: usize) -> (DebugRunner, PermanentHandle) {
    let mut b = DebugRunner::builder()
        .add_card(murasamemon())
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("FILLER"))
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(gd_option("GD-OPT"))
        .add_card(sec_filler("SEC"))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8]);
    if sec_count > 0 {
        b = b.security(0, &vec!["SEC"; sec_count]);
    }
    let mut runner = b.memory(8).start();
    runner.set_first_player(0);
    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, murasame)
}

/// Add a p0 Tamer carrying a bottom face-down digivolution source (the trash-FD
/// pay option). Returns the Tamer handle.
fn add_tamer_with_face_down(runner: &mut DebugRunner) -> PermanentHandle {
    let tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;
    tamer
}

fn fire_when_digivolving(runner: &mut DebugRunner, murasame: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(murasame),
    );
    runner.game.drain_effect_queue();
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_041_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
}

#[test]
fn bt25_041_has_alliance_alt_digivolve_and_inherited_end_of_attack() {
    let card = compiled(CARD_ID);

    // Alliance: a non-inherited grant_keyword declarative.
    let has_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::FaceUp
        )
    });
    assert!(has_alliance, "<Alliance> grant present (face-up)");

    // Alt-digivolve (Glowing Dawn Lv.4).
    assert!(
        card.alt_paths
            .iter()
            .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve)),
        "alt-digivolve path present"
    );

    // Inherited End-of-Attack OPT unsuspend clause.
    let inherited_eoa = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when == vec![CompiledTiming::EndOfAttack])
        .expect("inherited end_of_attack clause present");
    assert_eq!(inherited_eoa.scope, CompiledScope::Inherited);
    assert!(inherited_eoa.once_per_turn, "once per turn");
    assert!(inherited_eoa.optional, "'By trashing …' → optional");
}

#[test]
fn bt25_041_main_clause_is_present() {
    // The main [WD][WA][OPT] play/use-reduced clause is now implemented: a
    // face-up triggered clause firing on WhenDigivolving + WhenAttacking,
    // once-per-turn, optional.
    let card = compiled(CARD_ID);
    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::FaceUp
                && t.when.contains(&CompiledTiming::WhenAttacking)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("the main [WD][WA] play/use-reduced clause must be present");
    assert!(main.once_per_turn, "[Once Per Turn]");
    assert!(
        !main.optional,
        "DCGO runs this non-optional; the decline is the menu's 'Don't pay'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_041_inherited_unsuspends_host_after_attack_by_trashing_face_down() {
    let mut runner = DebugRunner::builder()
        .add_card(murasamemon())
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .deck(0, &["STASH"; 3])
        .deck(1, &["STASH"; 3])
        .memory(10)
        .start();

    // Stack: [Murasamemon (inherited source), HOST (Glowing Dawn top)].
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    // Suspend the host (as if it just attacked).
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    // Tamer + face-down stash beneath it (the unsuspend cost).
    let tamer = runner.place_stack(0, &["STASH", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;

    let trash_before = runner.trash_size(0);

    // Fire the inherited End-of-Attack clause on the HOST (bt21_021 idiom).
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "the inherited end_of_attack OPT must install a prompt (Glowing Dawn host + face-down stash)"
    );
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "the Glowing Dawn host must unsuspend after paying the face-down trash cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down stash source was trashed as the cost"
    );
}

#[test]
fn bt25_041_inherited_no_unsuspend_when_no_face_down_stash() {
    let mut runner = DebugRunner::builder()
        .add_card(murasamemon())
        .add_card(make_glowing_dawn_host("HOST"))
        .add_card(make_tamer("TAMER"))
        .add_card(make_filler("STASH"))
        .deck(0, &["STASH"; 3])
        .deck(1, &["STASH"; 3])
        .memory(10)
        .start();

    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    // Tamer with NO face-down stash (plain) → cost unpayable.
    runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "with no face-down stash the unsuspend cost is unpayable → host stays suspended"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Main [WD][WA][OPT] 2-way pay → play/use at cost-3
//             (G-PLAY-OR-USE-FROM-HAND)
// ═══════════════════════════════════════════════════════════════════════════════

/// PAY via "trash a face-down card under a Tamer", then PLAY a [Glowing Dawn]
/// Digimon from hand at cost-3 (5 → pay 2). No security needed for this branch.
#[test]
fn bt25_041_trash_face_down_then_play_glowing_dawn_digimon() {
    let (mut runner, murasame) = main_clause_base(0);
    let _tamer = add_tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let trash_before = runner.trash_size(0);
    let mem_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();

    fire_when_digivolving(&mut runner, murasame);

    // 2) 2-way pay-cost choice. With no security, only the trash-FD branch (idx 1)
    //    + "don't pay" are meaningful; pick the trash-FD branch.
    let v = runner.pending_selection_view().expect("pay-cost choice");
    assert!(
        v.effect_choices.is_some(),
        "the pay-cost choice is an EffectChoice menu"
    );
    runner
        .execute_branch(1)
        .expect("choose trash-FD-under-Tamer");

    // 3) Tamer pick (the trash).
    let v = runner.pending_selection_view().expect("Tamer pick");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();

    // 4) Hand pick — the GD Digimon.
    let gd_idx = hand_index_of(&runner, 0, "GD-DIGI");
    let v = runner.pending_selection_view().expect("hand pick");
    let want = digimon_engine::action::space::PLAY_HAND_START + gd_idx as u16;
    assert!(
        v.valid_action_ids.contains(&want),
        "GD-DIGI is a legal pick"
    );
    runner.execute_action(v.selecting_player, want).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down card was trashed as the pay cost"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI"),
        "the [Glowing Dawn] Digimon was played"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before + 1,
        "exactly one new permanent entered"
    );
    assert_eq!(runner.memory(), mem_before - 2, "cost 5 - 3 = 2 memory");
}

/// PAY via "add your top security card to the hand", then USE a [Glowing Dawn]
/// Option from hand at cost-3 (5 → pay 2). The added security card is now also
/// in hand but it is a plain Tamer (not Glowing Dawn) so it is not a pick.
#[test]
fn bt25_041_add_security_then_use_glowing_dawn_option() {
    let (mut runner, murasame) = main_clause_base(2);
    push_to_hand(&mut runner, 0, "GD-OPT");

    let hand_before = runner.game.players[0].hand.len();
    let sec_before = runner.security_count(0);
    let mem_before = runner.memory();

    fire_when_digivolving(&mut runner, murasame);

    // Pay-cost choice → branch 0 "Add your top security card to the hand".
    let v = runner.pending_selection_view().expect("pay-cost choice");
    runner.execute_branch(0).expect("choose add-top-security");

    // Hand pick — the GD Option (the added security card is a plain Tamer).
    let opt_idx = hand_index_of(&runner, 0, "GD-OPT");
    let v = runner.pending_selection_view().expect("hand pick");
    let want = digimon_engine::action::space::PLAY_HAND_START + opt_idx as u16;
    assert!(v.valid_action_ids.contains(&want), "GD-OPT is a legal pick");
    runner.execute_action(v.selecting_player, want).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before - 1,
        "the top security card was added to hand as the pay cost"
    );
    // Hand: started with GD-OPT (+1 over the base), +1 security card added, -1 GD
    // Option used.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1 - 1,
        "added the security card to hand and used the GD Option"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "GD-OPT"),
        "the [Glowing Dawn] Option was USED"
    );
    assert_eq!(runner.memory(), mem_before - 2, "use cost 5 - 3 = 2 memory");
}

/// DECLINE: the player may decline to pay/play entirely (choose "Don't pay").
#[test]
fn bt25_041_main_clause_is_declinable() {
    let (mut runner, murasame) = main_clause_base(2);
    let _tamer = add_tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let trash_before = runner.trash_size(0);
    let sec_before = runner.security_count(0);
    let field_before = runner.game.players[0].battle_area.len();

    fire_when_digivolving(&mut runner, murasame);

    // Decline = "Don't pay the cost" (DCGO value 3) on the one pay-cost menu.
    let _ = runner.pending_selection_view().expect("pay-cost choice");
    runner.execute_branch(2).expect("choose don't pay");
    let _ = runner.auto_resolve();

    assert_eq!(runner.trash_size(0), trash_before, "no trash on decline");
    assert_eq!(
        runner.security_count(0),
        sec_before,
        "no security added on decline"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "no GD card played on decline"
    );
}

/// NEGATIVE: pay the cost, but no [Glowing Dawn] card in hand → nothing is
/// played (the select_hand finds no candidate).
#[test]
fn bt25_041_no_glowing_dawn_card_in_hand_no_play() {
    let (mut runner, murasame) = main_clause_base(0);
    let _tamer = add_tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "FILLER"); // not Glowing Dawn

    let field_before = runner.game.players[0].battle_area.len();

    fire_when_digivolving(&mut runner, murasame);

    let v = runner.pending_selection_view().expect("pay-cost choice");
    runner.execute_branch(1).expect("choose trash-FD");
    let v = runner.pending_selection_view().expect("Tamer pick");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "FILLER"),
        "the non-Glowing-Dawn card stays in hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "no GD card available → nothing played"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Hand-pick legality: only cards that can actually be played/used
// ═══════════════════════════════════════════════════════════════════════════════
//
// DCGO BT25_041.cs:54-55 `isGlowingDawnCard` = `HasGlowingDawnTraits` only, and
// the pick (BT25_041.cs:138-151, `SelectHandEffect.Mode.Custom`) offers every
// [Glowing Dawn] card in hand; the chosen card is then handed to
// `PlayOptionCards` / `PlayPermanentCards(payCost: true)` (BT25_041.cs:229-248),
// which refuse a Digi-Egg, a card with no free frame / under a play lock, an
// Option whose colour requirement is unmet, or a cost the player can't pay.
// DCGO is looser than the rules here: CR 4.3 1-3-2 (an impossible action is not
// carried out), 7-1-1 (only a card with a play cost is "played" — never a
// Digi-Egg such as Frimon BT25-003), 4-22-1..3 (an Option can't be used without
// meeting its colour requirement; an effect use is still a use, Q1500 / Q2785).
// The engine follows the rules: only cards the play/use would accept, at the
// reduced cost, are offered.

fn gd_egg(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::DigiEgg;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(2);
    c.dp = None;
    c.play_cost = 0;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A green [Glowing Dawn] Option — unusable with only yellow cards on the field.
fn gd_option_green(id: &str) -> CardData {
    let mut c = gd_option(id);
    c.colors = vec![CardColor::Green];
    c
}

/// A [Glowing Dawn] Digimon whose reduced cost (22 − 3 = 19) is unpayable from
/// memory 8 (8 − 19 = −11, past the opponent's 10).
fn gd_digimon_expensive(id: &str) -> CardData {
    let mut c = gd_digimon(id);
    c.play_cost = 22;
    c
}

fn legality_pool(b: digimon_engine::debug_runner::DebugRunnerBuilder) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    b.add_card(gd_egg("GD-EGG"))
        .add_card(gd_option_green("GD-OPT-GREEN"))
        .add_card(gd_digimon_expensive("GD-EXP"))
}

fn offered_ids(runner: &DebugRunner) -> Vec<String> {
    use digimon_engine::action::space::{PASS, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;
    let Some(v) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if v.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let ids: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = v
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| ids[(id - PLAY_HAND_START) as usize].clone())
        .collect();
    out.sort();
    out
}

/// Branch B (trash a face-down card under a Tamer): `hand` in hand, `setup` run
/// before the trigger. Returns the sorted hand-pick candidates.
fn offered_trash_branch(hand: &[&str], setup: impl FnOnce(&mut DebugRunner)) -> Vec<String> {
    let mut b = legality_pool(
        DebugRunner::builder()
            .add_card(murasamemon())
            .add_card(make_tamer("TAMER"))
            .add_card(make_filler("FILLER"))
            .add_card(gd_digimon("GD-DIGI"))
            .add_card(gd_option("GD-OPT"))
            .add_card(sec_filler("SEC")),
    )
    .deck(0, &["FILLER"; 8])
    .deck(1, &["FILLER"; 8]);
    b = b.memory(8);
    let mut runner = b.start();
    runner.set_first_player(0);
    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let _tamer = add_tamer_with_face_down(&mut runner);
    for h in hand {
        push_to_hand(&mut runner, 0, h);
    }
    setup(&mut runner);
    fire_when_digivolving(&mut runner, murasame);
    runner.execute_branch(1).expect("choose trash-FD-under-Tamer");
    let v = runner.pending_selection_view().expect("Tamer pick");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    offered_ids(&runner)
}

/// Regression guard: a playable GD Digimon and a usable (yellow) GD Option are
/// both offered.
#[test]
fn bt25_041_hand_pick_offers_playable_digimon_and_usable_option() {
    assert_eq!(
        offered_trash_branch(&["GD-DIGI", "GD-OPT"], |_| {}),
        vec!["GD-DIGI".to_string(), "GD-OPT".to_string()]
    );
}

/// A [Glowing Dawn] Digi-Egg (Frimon BT25-003 shape) has no play cost: not offered.
#[test]
fn bt25_041_hand_pick_excludes_glowing_dawn_digi_egg() {
    assert_eq!(
        offered_trash_branch(&["GD-EGG", "GD-DIGI"], |_| {}),
        vec!["GD-DIGI".to_string()]
    );
}

/// A green [Glowing Dawn] Option with only yellow cards on the field: its
/// colour requirement is unmet (CR 4.3 4-22), so it can't be used — not offered.
#[test]
fn bt25_041_hand_pick_excludes_option_whose_colour_requirement_is_unmet() {
    assert_eq!(
        offered_trash_branch(&["GD-OPT-GREEN", "GD-DIGI"], |_| {}),
        vec!["GD-DIGI".to_string()]
    );
}

/// No affordability filter, as in DCGO: BT25_041.cs:138-151 offers every
/// [Glowing Dawn] card (`HasGlowingDawnTraits`) and lets PlayPermanentCards /
/// PlayOptionCards charge the cost. The cost is only settled at payment — the
/// payer may still opt into "before paying" reducers such as Kyo Sawashiro
/// (BT25-088) — so a card whose cost − 3 is past the floor is still a legal
/// pick. If it can't be paid, CR 4.3 7-1-2-3 (9-1-8 for an Option) returns
/// it to the hand and memory doesn't move.
#[test]
fn bt25_041_hand_pick_offers_card_whose_reduced_cost_is_unpayable() {
    assert_eq!(
        offered_trash_branch(&["GD-EXP", "GD-DIGI"], |_| {}),
        vec!["GD-DIGI".to_string(), "GD-EXP".to_string()]
    );
}

/// Picking a card whose reduced cost can't be paid (22 − 3 = 19 from memory 8,
/// no reducer available) plays nothing: the card stays in hand and memory
/// doesn't move (CR 4.3 7-1-2-3).
#[test]
fn bt25_041_unpayable_pick_returns_card_and_memory_does_not_move() {
    let mut runner = legality_pool(
        DebugRunner::builder()
            .add_card(murasamemon())
            .add_card(make_tamer("TAMER"))
            .add_card(make_filler("FILLER"))
            .add_card(gd_digimon("GD-DIGI"))
            .add_card(gd_option("GD-OPT"))
            .add_card(sec_filler("SEC")),
    )
    .deck(0, &["FILLER"; 8])
    .deck(1, &["FILLER"; 8])
    .memory(8)
    .start();
    runner.set_first_player(0);
    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let _tamer = add_tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-EXP");
    fire_when_digivolving(&mut runner, murasame);
    runner.execute_branch(1).expect("choose trash-FD-under-Tamer");
    let v = runner.pending_selection_view().expect("Tamer pick");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    assert_eq!(offered_ids(&runner), vec!["GD-EXP".to_string()]);
    let memory_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();
    let v = runner.pending_selection_view().expect("hand pick");
    let pick = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("pick GD-EXP");
    assert_eq!(runner.memory(), memory_before, "an unpayable play moves no memory");
    assert_eq!(runner.game.players[0].battle_area.len(), field_before, "nothing was played");
    assert_eq!(hand_index_of(&runner, 0, "GD-EXP"), 0, "GD-EXP returned to the hand");
}

/// Kyo Sawashiro (BT25-088): "[Your Turn][Once Per Turn] When a card with the
/// [Glowing Dawn] trait would be played, by trashing the bottom face-down card
/// under any of your Tamers, reduce the play cost by 1." With Kyo on the field
/// and a face-down card left to trash, a [Glowing Dawn] Digimon whose cost − 3
/// is ONE past the floor (22 − 3 = 19 from memory 8 → −11) is payable at
/// 22 − 3 − 1 = 18 (→ −10), so it must be offered.
#[test]
fn bt25_041_hand_pick_offers_card_payable_only_with_kyo_sawashiro_reduction() {
    let mut runner = legality_pool(
        DebugRunner::builder()
            .add_card(murasamemon())
            .add_card(make_tamer("TAMER"))
            .add_card(make_filler("FILLER"))
            .add_card(gd_digimon("GD-DIGI"))
            .add_card(gd_option("GD-OPT"))
            .add_card(sec_filler("SEC"))
            .dsl_card("BT25-088")
            .expect("BT25-088 Kyo Sawashiro compiles"),
    )
    .deck(0, &["FILLER"; 8])
    .deck(1, &["FILLER"; 8])
    .memory(8)
    .start();
    runner.set_first_player(0);
    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    // Kyo Sawashiro is the only Tamer, with two face-down cards under it: one
    // for Murasamemon's cost, one left for Kyo's own reducer.
    let kyo = runner.place_stack(0, &["FILLER", "FILLER", "BT25-088"]);
    for i in 0..2 {
        runner.game.players[0].battle_area[kyo.index as usize].card_sources[i].face_down = true;
    }
    push_to_hand(&mut runner, 0, "GD-EXP");
    fire_when_digivolving(&mut runner, murasame);
    runner.execute_branch(1).expect("choose trash-FD-under-Tamer");
    let v = runner.pending_selection_view().expect("Tamer pick");
    runner
        .execute_action(v.selecting_player, v.valid_action_ids[0])
        .unwrap();
    assert_eq!(
        offered_ids(&runner),
        vec!["GD-EXP".to_string()],
        "payable at 18 with Kyo Sawashiro's -1, so it is offered"
    );
    // The pick goes through with Kyo's reduction: walk the reducer prompts,
    // always accepting / taking the first option.
    let v = runner.pending_selection_view().expect("hand pick");
    let pick = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .unwrap();
    runner.execute_action(0, pick).expect("pick GD-EXP");
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 10, "reducer prompts did not settle: {:?}", v.kind);
        if runner.game.players[0].battle_area.iter().any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-EXP") {
            break;
        }
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != digimon_engine::action::space::PASS)
            .unwrap_or(v.valid_action_ids[0]);
        runner.execute_action(v.selecting_player, action).unwrap();
    }
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-EXP"),
        "GD-EXP was played with Kyo Sawashiro's reduction"
    );
    // `game.memory` is from the turn player's side; the turn may already have
    // passed to p1 now that p0 is at -10.
    let p0_memory = if runner.game.turn_player() == 0 { runner.memory() } else { -runner.memory() };
    assert_eq!(p0_memory, -10, "8 - (22 - 3 - 1) = -10 (Kyo Sawashiro's -1 was taken)");
}

/// A full battle area: the GD Digimon can't be played; the GD Option can still
/// be used.
#[test]
fn bt25_041_hand_pick_excludes_digimon_when_battle_area_full() {
    assert_eq!(
        offered_trash_branch(&["GD-DIGI", "GD-OPT"], |r| {
            r.game.rules.field_slots = r.game.players[0].battle_area.len() as u8;
        }),
        vec!["GD-OPT".to_string()]
    );
}

/// "You can't use Option cards": the GD Option is not offered; the Digimon is.
#[test]
fn bt25_041_hand_pick_excludes_option_when_options_cannot_be_used() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    assert_eq!(
        offered_trash_branch(&["GD-DIGI", "GD-OPT"], |r| {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(ModifierType::CannotUseOptionCards, 0, Expiry::Permanent, None, 1),
            );
        }),
        vec!["GD-DIGI".to_string()]
    );
}

/// Branch A (add the top security card to the hand): the card added is a
/// [Glowing Dawn] Digi-Egg — it joins the hand but is not offered; the GD
/// Option already in hand is.
#[test]
fn bt25_041_add_security_branch_excludes_added_digi_egg() {
    let mut runner = legality_pool(
        DebugRunner::builder()
            .add_card(murasamemon())
            .add_card(make_tamer("TAMER"))
            .add_card(make_filler("FILLER"))
            .add_card(gd_digimon("GD-DIGI"))
            .add_card(gd_option("GD-OPT"))
            .add_card(sec_filler("SEC")),
    )
    .deck(0, &["FILLER"; 8])
    .deck(1, &["FILLER"; 8])
    // Last element = top security card.
    .security(0, &["SEC", "GD-EGG"])
    .memory(8)
    .start();
    runner.set_first_player(0);
    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    push_to_hand(&mut runner, 0, "GD-OPT");
    fire_when_digivolving(&mut runner, murasame);
    runner.execute_branch(0).expect("choose add-top-security");
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-EGG"),
        "the GD Digi-Egg was added to the hand as the cost"
    );
    assert_eq!(offered_ids(&runner), vec!["GD-OPT".to_string()]);
}
