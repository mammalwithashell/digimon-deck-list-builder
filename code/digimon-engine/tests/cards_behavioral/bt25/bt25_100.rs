//! BT25-100 Iron Slash — Option, Black, Cost 3, [TS].
//!
//! # Card text (cards.json)
//! <Use Req. ([TS] trait)> (Specified cards let you ignore color requirements.)
//! [Security] Activate this card's [Main] effects.
//! [Main] <De-Digivolve 2> 1 of your opponent's Digimon. (Trash up to 2 cards
//!   from the top. You can't trash past level 3 cards.) Then, you may link this
//!   card to 1 of your Digimon on the field without paying the cost.
//! Inherited: <Piercing>
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_100.cs
//!
//! # Patterns this test covers
//! - D3 color ignore / bypass (Use Req. flood gate)
//! - C1/C3-adjacent: link_to_own_digimon (Option self-link)
//! - De-Digivolve as a [Main] effect
//! - [Security] activates [Main]
//! - H3 Piercing (inherited keyword grant)

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt25/BT25-100.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_100_structure_use_req_main_security_piercing_and_link_requirement() {
    let runner = iron_runner().start();
    let compiled = runner
        .compiled_card("BT25-100")
        .expect("BT25-100 compiled card present");

    assert_eq!(compiled.card, "BT25-100");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(compiled.use_requirement.is_some());

    // Color-ignore flood gate.
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
            if modifier == "IgnoreColorRequirement"
    )));

    // [Main]: de-digivolve then optional free link.
    let main = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::MainFromHand] => Some(t),
            _ => None,
        })
        .expect("MainFromHand clause");
    assert!(main
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::DeDigivolve { .. })));
    assert!(main.process.iter().any(|s| matches!(
        s,
        CompiledStep::LinkToOwnDigimon {
            optional: true,
            free: true,
            ..
        }
    )));

    // [Security] activates the same [Main] effects (inherited scope).
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited && t.when == vec![CompiledTiming::OnSecurity]
    )));

    // Inherited link requirement.
    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { cost: 2, filter, .. })
            if filter.trait_has.as_deref() == Some("TS")
    )));
}

// Link-card inherited (ESS) effects — e.g. <Piercing> here — are aggregated
// onto the host as of G-LINK-INHERITED-ESS closure (2026-07): the collectors'
// linked-card passes now fold a link card in as an inherited-style source, so a
// `scope: inherited` `<Piercing>` grant on the linked Option reaches the host.
#[test]
fn bt25_100_grants_inherited_piercing_to_host() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    runner
        .execute_action(lview.selecting_player, encode_attack(0, host.index as u16))
        .expect("link Iron Slash to host");

    let host_handle = PermanentHandle {
        player: 0,
        index: host.index,
    };
    assert!(
        runner.game.has_keyword(host_handle, Keyword::Piercing),
        "host hosting Iron Slash should gain inherited <Piercing>"
    );
}

// ── Section 2: behavioral — [Main] ───────────────────────────────────────

#[test]
fn bt25_100_main_de_digivolves_two_then_can_decline_link() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    // A TS Tamer satisfies the Use Req. (color ignore) but is not a legal link
    // host (the link filter is kind: digimon), so no link prompt installs.
    runner.place_on_field(0, "TS-TAMER", Some(0));
    // Opponent Lv5 with a 3-card stack (Lv3 → Lv4 → Lv5).
    let opp = runner.place_field_stack(1, &["OPP-LV3", "OPP-LV4", "OPP-LV5"], false, 1);
    runner.game.enter_main_phase();

    let trash_before = runner.trash_size(1);
    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    assert_eq!(tview.kind, SelectionKind::OppField);
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    // De-Digivolve 2 trashes the top two sources (Lv5 + Lv4), leaving Lv3 top.
    assert_eq!(
        runner.trash_size(1),
        trash_before + 2,
        "De-Digivolve 2 trashes two source cards"
    );
    let top_id = runner.game.player(1).battle_area[opp.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top_id, "OPP-LV3", "top card is now the Lv3");

    // Only a TS Tamer is on our field (no Digimon), so the optional link step
    // finds no legal host and resolution completes with no pending selection.
    assert!(
        runner.pending_selection().is_none(),
        "no Digimon host → link step skipped, effect resolves"
    );
}

#[test]
fn bt25_100_main_links_to_chosen_digimon() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    runner
        .execute_action(lview.selecting_player, encode_attack(0, host.index as u16))
        .expect("link to host");

    let linked = &runner.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].card_id(&runner.game.card_data), "BT25-100");
}

// ── Section 3: gating — link offered only when a legal host exists ────────

#[test]
fn bt25_100_no_link_prompt_when_no_own_digimon() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    // TS Tamer satisfies Use Req. but is not a legal link host (kind: digimon).
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner
        .pending_selection_view()
        .expect("de-digi target prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");

    // No own Digimon → no link prompt installs; resolution completes.
    assert!(
        runner.pending_selection().is_none(),
        "no link host → no link selection installs"
    );
}

/// The effect link still honours the printed Link condition ("[TS] trait"):
/// DCGO filters hosts through `card.CanLinkToTargetPermanent(permanent, false,
/// true)` (BT25_100.cs:85 → CardSource.cs:3346-3358 `linkCondition
/// .digimonCondition`). A non-[TS] Digimon is never offered.
#[test]
fn bt25_100_link_host_must_meet_the_ts_link_condition() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let plain = runner.place_on_field(0, "OPP-LV4", Some(0));
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner.pending_selection_view().expect("de-digi prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    assert!(lview
        .valid_action_ids
        .contains(&encode_attack(0, host.index as u16)));
    assert!(
        !lview
            .valid_action_ids
            .contains(&encode_attack(0, plain.index as u16)),
        "a non-[TS] Digimon fails the Link condition"
    );
}

/// Printed Link box: DP+2000, <Collision>, <Piercing> (card image / official
/// bundle; DCGO BT25_100.cs:45-61).
#[test]
fn bt25_100_linked_host_gains_dp_2000_collision_and_piercing() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    let host = runner.place_on_field(0, "HOST", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();
    let host_handle = PermanentHandle { player: 0, index: host.index };
    let before = runner.effective_dp(host_handle).expect("dp");

    assert_eq!(play_iron_standard(&mut runner), OptionPlayResult::Pending);
    let tview = runner.pending_selection_view().expect("de-digi prompt");
    runner
        .execute_action(tview.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
    let lview = runner.pending_selection_view().expect("link prompt");
    runner
        .execute_action(lview.selecting_player, encode_attack(0, host.index as u16))
        .expect("link");
    runner.game.tick_declarative_effects();

    assert_eq!(runner.effective_dp(host_handle), Some(before + 2000));
    assert!(runner.game.has_keyword(host_handle, Keyword::Collision));
    assert!(runner.game.has_keyword(host_handle, Keyword::Piercing));
}

// ── Section 4: [Security] Activate this card's [Main] — the link tail ─────
//
// G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON. A security-flipped Iron Slash
// runs the same [Main] body, and its "you may link this card" tail must lift
// the card OUT of the security resolution onto the chosen host (DCGO
// `Permanent.AddLinkCard` → `RemoveFromAllArea`; §13-1-7-4 trashes a checked
// card only "unless it belongs to an area"). Before the fix the link step
// silently returned (no `pending_option`), Iron Slash was trashed, and the
// host never gained the Link DP / <Piercing>.

#[test]
fn bt25_100_security_flip_links_to_defenders_digimon() {
    // P1 defends with Iron Slash on top of security and a TS Digimon host;
    // P0 attacks with a Lv5 stack for De-Digivolve 2 to bite.
    let mut runner = iron_runner()
        .add_card(make_ts_digimon("HOST", 3))
        .security(1, &["OPP-LV3", "BT25-100"])
        .memory(5)
        .start();
    let host = runner.place_on_field(1, "HOST", Some(1));
    let attacker = runner.place_field_stack(0, &["OPP-LV3", "OPP-LV4", "OPP-LV5"], false, 0);
    runner.game.enter_main_phase();

    let host_dp_before = runner
        .effective_dp(PermanentHandle { player: 1, index: host.index })
        .expect("host dp");
    let trash1_before = runner.trash_size(1);
    let _ = runner.attack_player(attacker, 1, false);

    // [Main] step 1 (defender picks the attacker to De-Digivolve 2).
    let tview = runner
        .pending_selection_view()
        .expect("security [Main]: de-digi target prompt");
    assert_eq!(tview.selecting_player, 1);
    assert_eq!(tview.kind, SelectionKind::OppField);
    runner
        .execute_action(1, encode_attack(0, attacker.index as u16))
        .expect("defender picks the attacking stack");

    // [Main] step 2: the link tail — an OPTIONAL own-Digimon pick, asked even
    // though the card came from security.
    let lview = runner
        .pending_selection_view()
        .expect("security [Main]: link-host prompt must be asked");
    assert_eq!(lview.selecting_player, 1);
    assert_eq!(lview.kind, SelectionKind::OwnField);
    assert!(lview.is_optional, "printed 'you may link'");
    runner
        .execute_action(1, encode_attack(0, host.index as u16))
        .expect("link Iron Slash to the defender's host");
    assert!(runner.pending_selection().is_none());

    // The card is plugged into the host, NOT trashed — and only once.
    let host_perm = &runner.game.player(1).battle_area[host.index as usize];
    assert_eq!(host_perm.linked_cards.len(), 1);
    assert_eq!(host_perm.linked_cards[0].card_id(&runner.game.card_data), "BT25-100");
    assert_eq!(
        runner.trash_size(1),
        trash1_before,
        "a linked security card must not also be trashed at dispose"
    );
    assert_eq!(runner.security_count(1), 1, "one security card was checked");
    // Link DP (+2000) and the linked <Piercing> reach the host.
    let host_handle = PermanentHandle { player: 1, index: host.index };
    assert_eq!(
        runner.effective_dp(host_handle).expect("host dp"),
        host_dp_before + 2000
    );
    assert!(runner.game.has_keyword(host_handle, Keyword::Piercing));
    // De-Digivolve 2 bit on the attacker: Lv5 + Lv4 trashed, Lv3 remains.
    let top_id = runner.game.player(0).battle_area[attacker.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(top_id, "OPP-LV3");
}

#[test]
fn bt25_100_security_flip_declined_link_trashes_the_card_once() {
    let mut runner = iron_runner()
        .add_card(make_ts_digimon("HOST", 3))
        .security(1, &["OPP-LV3", "BT25-100"])
        .memory(5)
        .start();
    runner.place_on_field(1, "HOST", Some(1));
    let attacker = runner.place_field_stack(0, &["OPP-LV3", "OPP-LV4", "OPP-LV5"], false, 0);
    runner.game.enter_main_phase();

    let trash1_before = runner.trash_size(1);
    let _ = runner.attack_player(attacker, 1, false);
    runner
        .execute_action(1, encode_attack(0, attacker.index as u16))
        .expect("defender picks the attacking stack");
    let lview = runner.pending_selection_view().expect("link-host prompt");
    assert!(lview.is_optional);
    runner.execute_action(1, PASS).expect("decline the link");
    assert!(runner.pending_selection().is_none());

    let host_perm = &runner.game.player(1).battle_area[0];
    assert!(host_perm.linked_cards.is_empty());
    assert_eq!(
        runner.trash_size(1),
        trash1_before + 1,
        "declined: the checked Option is trashed exactly once"
    );
    assert!(runner
        .game
        .player(1)
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "BT25-100"));
}

#[test]
fn bt25_100_security_flip_without_host_trashes_the_card() {
    // No TS Digimon on the defender's side: no link prompt, card is trashed
    // by the ordinary security dispose (DCGO `HasMatchConditionPermanent`
    // guard skips the prompt the same way).
    let mut runner = iron_runner()
        .security(1, &["OPP-LV3", "BT25-100"])
        .memory(5)
        .start();
    let attacker = runner.place_field_stack(0, &["OPP-LV3", "OPP-LV4", "OPP-LV5"], false, 0);
    runner.game.enter_main_phase();

    let trash1_before = runner.trash_size(1);
    let _ = runner.attack_player(attacker, 1, false);
    runner
        .execute_action(1, encode_attack(0, attacker.index as u16))
        .expect("defender picks the attacking stack");
    assert!(runner.pending_selection().is_none(), "no host → no link prompt");
    assert_eq!(runner.trash_size(1), trash1_before + 1);
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn iron_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-100 YAML loads")
        .add_card(make_digimon("OPP-DIGIMON", 4))
        .add_card(make_digimon("OPP-LV3", 3))
        .add_card(make_digimon("OPP-LV4", 4))
        .add_card(make_digimon("OPP-LV5", 5))
}

fn play_iron_standard(runner: &mut digimon_engine::debug_runner::DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let first = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner
            .game
            .resolve_selection(0, first)
            .expect("choose Standard [Main] mode");
    }
    result
}

fn make_ts_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_digimon(id, level);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_digimon(id: &str, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card
}

fn make_ts_tamer(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["TS".to_string()];
    card
}

// ── Section 5: the Link PLAY MODE is gated on a legal host ───────────────
//
// F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST (qa/dcgo-exams/BT25/NOTES-BT25-091.md).
// Playing a dual-mode Plug-In Option from hand used to offer the "Plug in via
// Link Requirements" branch even with NO legal host on the board; taking it
// paid the link cost and dropped the card in the trash linked to nothing.
//
// general_rule.pdf §10-1-3-1: "The player declares a link and reveals 1 card to
// link. 1 link requirement is chosen on the revealed card, then the player
// chooses 1 of their Digimon that meets the requirement." With no Digimon that
// meets the requirement the link procedure cannot be performed, so the §6-5-1-4
// main-phase link action is not available.
//
// DCGO agrees: `CardEffectFactory.LinkEffect` returns `null` outright when
// `!CardEffectCommons.HasMatchConditionPermanent(CanSelectPermanentCondition)`
// (DCGO/Assets/Scripts/Script/CardEffectFactory/KeyWordEffects/Link.cs:24, and
// again in `CanUseCondition` at :53), so the declaration is never offered.
//
// The engine already holds this contract for the Digimon-side (Shape-B) hand
// link — `Game::hand_digimon_link_available` requires `!hosts.is_empty()`.

/// No own Digimon at all: the mode-select must not be installed — the play
/// goes straight to the Standard `[Main]` body.
#[test]
fn bt25_100_no_mode_select_when_no_link_host_exists() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    // Only a TS Tamer: satisfies the Use Req., but the printed Link condition
    // names a [TS] *Digimon*, so there is no legal host.
    runner.place_on_field(0, "TS-TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    let before = runner.memory();
    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let view = runner
        .pending_selection_view()
        .expect("a selection is parked");
    assert_ne!(
        view.kind,
        SelectionKind::EffectChoice,
        "no legal link host → the Link mode is not offered, so no mode-select installs"
    );
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "the Standard [Main] body runs directly (its De-Digivolve target prompt)"
    );
    assert_eq!(
        runner.memory(),
        before - 3,
        "the direct Standard play charges the printed use cost 3, not the Link cost 2"
    );
    runner
        .execute_action(view.selecting_player, encode_attack(0, opp.index as u16))
        .expect("choose opponent Digimon");
}

/// A Digimon that FAILS the printed `[TS]` Link condition is not a host
/// either: the Link mode stays off the mode-select.
#[test]
fn bt25_100_no_mode_select_when_only_non_ts_digimon_on_board() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_tamer("TS-TAMER"))
        .memory(20)
        .start();
    runner.place_on_field(0, "TS-TAMER", Some(0));
    // A plain (non-[TS]) Digimon fails `link_requirement.filter.trait_has: TS`.
    runner.place_on_field(0, "OPP-LV4", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    let result = runner.game.play_option_from_hand(0, 0);
    assert_eq!(result, OptionPlayResult::Pending);
    let view = runner
        .pending_selection_view()
        .expect("a selection is parked");
    assert_ne!(
        view.kind,
        SelectionKind::EffectChoice,
        "a non-[TS] Digimon is not a legal host → no Link mode, no mode-select"
    );
}

/// Positive control: with a legal [TS] Digimon host the dual-mode prompt is
/// still offered, both branches labelled.
#[test]
fn bt25_100_mode_select_is_offered_when_a_legal_host_exists() {
    let mut runner = iron_runner()
        .hand(0, &["BT25-100"])
        .add_card(make_ts_digimon("HOST", 5))
        .memory(20)
        .start();
    runner.place_on_field(0, "HOST", Some(0));
    runner.place_on_field(1, "OPP-DIGIMON", Some(1));
    runner.game.enter_main_phase();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending
    );
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("mode-select installs");
    assert_eq!(pending.kind, SelectionKind::EffectChoice);
    let choices = pending
        .effect_choices
        .as_ref()
        .expect("labelled mode choices");
    assert_eq!(choices.len(), 2);
    assert!(choices[0].label.contains("Main"));
    assert!(choices[1].label.contains("Link"));
}
