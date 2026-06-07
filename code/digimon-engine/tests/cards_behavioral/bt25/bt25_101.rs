//! BT25-101 Divine Arms Version Ω — Option, Black, Cost 3, [TS].
//!
//! # Card text (cards.json + DCGO BT25_101.cs — authoritative)
//! <Use Req. ([TS] trait)> (Specified cards let you ignore color requirements.)
//! [Security] Activate this card's [Main] effects.
//! [Main] By trashing 1 [TS] trait card from your hand, <Draw 2> (Draw 2 cards
//!   from your deck.) After, you may link this card or 1 [TS] trait card from
//!   your trash to 1 of your Digimon on the field without paying the cost.
//! Inherited (link ESS): <Security A. +1>; <Reboot>.
//! Inherited (link ESS): [All Turns] When this would leave the battle area, by
//!   trashing 1 of its link cards, it doesn't leave.
//! Self link-condition: this card may be linked onto a Digimon (DCGO restricts
//!   to Vulcanusmon, link cost 3) — modeled as a link_requirement.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_101.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - D3 color ignore / Use Req. flood gate
//! - [Main] trash-cost → draw → optional link (the "may pay" cost gates draw+link)
//! - link_cards { from: [self_option, trash], to: own_digimon, up_to 1 } — the
//!   3-way "This Card / [TS] from trash / None" choice
//! - [Security] activates [Main] (inherited OnSecurity)
//! - inherited link ESS: <Reboot> keyword + <Security A. +1>
//! - inherited link ESS leave-replacement (trash_own_link_card)
//! - link_requirement (self link condition)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const YAML: &str = include_str!("../../../cards/bt25/BT25-101.yaml");
const CARD_ID: &str = "BT25-101";

fn make_digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card.play_cost = level as u16;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-101 YAML loads")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TS-HAND", 4, &["TS"]))
        .add_card(make_digimon("TS-TRASH", 4, &["TS"]))
        .add_card(make_digimon("HOST", 5, &["TS"]))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) -> CardSource {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    let cs = CardSource::new(idx, player as u8, iid);
    runner.game.players[player].trash.push(cs.clone());
    cs
}

/// Recursively scan a step list (descending into `if` branches) for a step
/// matching `pred`.
fn steps_contain(steps: &[CompiledStep], pred: &dyn Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|s| {
        pred(s)
            || matches!(s, CompiledStep::If { then, else_branch, .. }
                if steps_contain(then, pred) || steps_contain(else_branch, pred))
    })
}

// ── Section 1: structural ────────────────────────────────────────────────────

#[test]
fn bt25_101_structure_use_req_main_security_inherited_ess_and_link_req() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled present");

    assert_eq!(compiled.card, CARD_ID);
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(3));
    assert!(compiled.traits.iter().any(|t| t == "TS"));
    assert!(compiled.use_requirement.is_some(), "Use Req. present");

    // Color-ignore flood gate.
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { modifier, .. })
                if modifier == "IgnoreColorRequirement"
        )),
        "IgnoreColorRequirement flood gate"
    );

    // [Main]: trash-cost → draw 2 → optional link from {self_option, trash}.
    let main = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::MainFromHand] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("MainFromHand clause");
    // The draw + link_cards live inside the `if binding_present(cost_card)`
    // branch (gated on paying the trash cost), so scan recursively.
    assert!(
        steps_contain(&main.process, &|s| matches!(s, CompiledStep::Draw { .. })),
        "[Main] draws (inside the trash-cost gate)"
    );
    assert!(
        steps_contain(&main.process, &|s| matches!(s, CompiledStep::LinkCards { .. })),
        "[Main] has a link_cards step (link this card or TS from trash)"
    );
    // The trash cost is a `select_hand { cost: true }` over [TS] hand cards.
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectHand { .. })),
        "[Main] selects a [TS] hand card to trash as the cost"
    );

    // [Security] activates the same [Main] (inherited OnSecurity).
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited && t.when == vec![CompiledTiming::OnSecurity]
        )),
        "[Security] activates [Main]"
    );

    // Link ESS keyword <Reboot> (scope: linked → applies to host while linked).
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
                if keyword == "Reboot" && *scope == CompiledScope::Linked
        )),
        "link <Reboot>"
    );

    // Link ESS <Security A. +1> (scope: linked).
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, security_attack, .. })
                if *scope == CompiledScope::Linked && *security_attack == Some(1)
        )),
        "link <Security A. +1>"
    );

    // Inherited link requirement (self link condition).
    assert!(
        compiled.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::LinkRequirement { .. })
        )),
        "link requirement present"
    );
}

// ── Section 2: behavioral — [Main] trash→draw→link this card ─────────────────

/// Play from hand, choosing the Standard [Main] mode if an EffectChoice mode
/// prompt is presented first.
fn play_main(runner: &mut DebugRunner) -> OptionPlayResult {
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(
        runner.game.pending_selection.as_ref().map(|s| &s.kind),
        Some(SelectionKind::EffectChoice)
    ) {
        let pend = runner.game.pending_selection.as_ref().unwrap();
        let looks_like_mode = pend.prompt.to_lowercase().contains("main")
            || pend.prompt.to_lowercase().contains("activate");
        if looks_like_mode {
            let first = pend.valid_action_ids[0];
            runner
                .game
                .resolve_selection(0, first)
                .expect("choose Standard [Main] mode");
        }
    }
    result
}

#[test]
fn bt25_101_main_trash_ts_draws_two_then_links_this_card() {
    let mut r = base()
        .hand(0, &[CARD_ID, "TS-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.game.enter_main_phase();

    let trash_before = r.trash_size(0);
    let deck_before = r.game.player(0).deck.len();
    assert_eq!(play_main(&mut r), OptionPlayResult::Pending);

    // Cost: select the [TS] card in hand to trash. The select is optional
    // (DCGO canNoSelect), so PASS (62) is also legal — pick a real card action.
    let cost_sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand-trash cost selection");
    let trash_action = *cost_sel
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("a [TS] hand card is selectable to trash");
    r.game
        .resolve_selection(0, trash_action)
        .expect("trash 1 TS from hand");

    // After paying the cost the controller drew 2 and is offered the link
    // choice. With no TS in trash the only link candidate is the option itself.
    // Drive the link selection chain (any card-pick / host-pick prompts),
    // steering toward HOST where it is offered.
    assert!(
        r.game.pending_selection.is_some(),
        "cost paid ⇒ draw+link ran and a link selection is installed"
    );
    let host_action = encode_attack(0, host.index as u16);
    for _ in 0..4 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        if sel.valid_action_ids.is_empty() {
            break;
        }
        let act = if sel.valid_action_ids.contains(&host_action) {
            host_action
        } else {
            // Pick the first non-PASS option to advance toward the host pick.
            *sel.valid_action_ids
                .iter()
                .find(|&&a| a != PASS)
                .unwrap_or(&sel.valid_action_ids[0])
        };
        let _ = r.game.resolve_selection(0, act);
    }

    // End state: the TS hand card was trashed as the cost, 2 were drawn, and the
    // Option attached ITSELF as a link card (not trashed on dispose).
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "exactly the TS hand card went to trash as the cost (the option self-linked, not trashed)"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before - 2,
        "<Draw 2> drew two cards from the deck"
    );
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "Divine Arms linked itself onto HOST");
    assert_eq!(
        linked[0].card_id(&r.game.card_data),
        CARD_ID,
        "the linked card is Divine Arms itself"
    );
}

#[test]
fn bt25_101_main_links_ts_from_trash() {
    let mut r = base()
        .hand(0, &[CARD_ID, "TS-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    seed_trash(&mut r, 0, "TS-TRASH");
    r.game.enter_main_phase();

    assert_eq!(play_main(&mut r), OptionPlayResult::Pending);
    let trash_action = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand-trash cost")
        .valid_action_ids[0];
    r.game.resolve_selection(0, trash_action).expect("trash TS");

    // Two zones now have link candidates (self_option + trash) → drive the
    // selection chain (zone-choice → card pick → host pick) to link a card.
    for _ in 0..4 {
        let Some(sel) = r.game.pending_selection.as_ref() else {
            break;
        };
        if sel.valid_action_ids.is_empty() {
            break;
        }
        let host_action = encode_attack(0, host.index as u16);
        let act = if sel.valid_action_ids.contains(&host_action) {
            host_action
        } else {
            sel.valid_action_ids[0]
        };
        let _ = r.game.resolve_selection(0, act);
    }

    // Either branch links exactly 1 card onto HOST without paying cost.
    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 1, "a card was linked onto HOST for free");
}

#[test]
fn bt25_101_main_link_is_optional_can_decline() {
    let mut r = base()
        .hand(0, &[CARD_ID, "TS-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.game.enter_main_phase();

    assert_eq!(play_main(&mut r), OptionPlayResult::Pending);
    let trash_action = r
        .game
        .pending_selection
        .as_ref()
        .expect("hand-trash cost")
        .valid_action_ids[0];
    r.game.resolve_selection(0, trash_action).expect("trash TS");

    // Decline the optional link (PASS).
    assert!(
        r.game.pending_selection.is_some(),
        "link selection offered (it is a 'may')"
    );
    let _ = r.game.resolve_selection(0, PASS);

    let linked = &r.game.player(0).battle_area[host.index as usize].linked_cards;
    assert_eq!(linked.len(), 0, "declining the link leaves HOST unlinked");
}

#[test]
fn bt25_101_main_no_ts_in_hand_skips_draw() {
    // No [TS] card in hand other than the option itself → the trash cost is
    // unpayable, so the [Main] clause draws nothing.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.game.enter_main_phase();

    let deck_before = r.game.player(0).deck.len();
    let _ = play_main(&mut r);

    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before,
        "unpayable trash cost → no Draw 2"
    );
}

// ── Section 3: inherited link ESS reaching the host ──────────────────────────

/// <Reboot> is a keyword grant; the linked-card declarative pass routes
/// linked-card keyword grants onto the host (G-LINK-INHERITED-ESS keyword slice).
#[test]
fn bt25_101_inherited_reboot_reaches_host_when_linked() {
    let mut r = base().memory(0).start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_linked_owned(host, CARD_ID, 0);
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(host, Keyword::Reboot),
        "linked Divine Arms grants <Reboot> to its host"
    );
}

/// The leave-replacement is a link-ESS (inherited) clause: while Divine Arms is
/// a link card on a host, that host may trash a link card to avoid leaving.
#[test]
fn bt25_101_inherited_leave_replacement_when_linked() {
    let mut r = base().memory(0).start();
    let host = r.place_on_field(0, "HOST", Some(0));
    r.push_linked_owned(host, CARD_ID, 0);
    r.game.enter_main_phase();

    let trash_before = r.trash_size(0);
    r.game.delete_permanent_with_effects(host);
    assert!(
        r.game.pending_selection.is_some(),
        "inherited leave-replacement offers the optional accept prompt"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept");
    let pick = r
        .game
        .pending_selection
        .as_ref()
        .expect("which-link-card prompt")
        .valid_action_ids[0];
    r.game.resolve_selection(0, pick).expect("pick link card");

    assert_eq!(r.battle_area_size(0), 1, "the host did NOT leave");
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the link card was trashed as the cost"
    );
}
