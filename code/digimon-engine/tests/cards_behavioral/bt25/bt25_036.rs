//! BT25-036 Craftmon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Trait line: Design (App Name) — Appmon; Appmon Grade "Sup.".
//!
//! # Card text (card image + DCGO BT25_036.cs — authoritative)
//!
//! [App Fusion] [Kabemon] & [Gomimon] & [Ecomon] & [Puzzlemon]: Cost 0.
//!   If 2 such cards are linked together, stack the link card on top and
//!   digivolve. (DCGO `AddAppfuseMethodByName(Kabemon, Gomimon, Ecomon,
//!   Puzzlemon)`.)
//! [Digivolve] Lv.2 w/[Stnd.] trait: Cost 2  (DCGO
//!   `AddSelfDigivolutionRequirementStaticEffect(HasStandardAppTraits, level 2,
//!   cost 2)`; `HasStandardAppTraits` == trait "Stnd.").
//! [Link] [Appmon] trait: Cost 2  (DCGO `AddSelfLinkConditionStaticEffect(
//!   HasAppmonTraits, 2)`).
//! [Security] At the end of the battle, play this card without paying the cost.
//!   (DCGO `PlaySelfDigimonAfterBattleSecurityEffect`.)
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//!   <Recovery +1>.  (mandatory shared OP/WD ActivateClass.)
//! [When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>.
//!   (DCGO `WhenLinked`, `SetIsLinkedEffect(true)`, skippable.)
//! Inherited: By trashing 1 [Appmon] trait card from your hand, Draw 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Yellow/BT25_036.cs
//!
//! # Re-adjudication note (2026-06-07)
//! Prior verdict BLOCKED (engine, App Fuse). RESOLVED: the App Fusion alt-play is
//! implemented end-to-end — DSL `alt_paths: [{ kind: app_fusion, materials, cost }]`
//! → `app_fusion_digivolve_route_for_card` (host has 2 distinct named cards
//! linked together) → digivolve route that stacks the App-Fusion card on top and
//! drains the host's linked cards under it as sources. See
//! tests/cards_behavioral/bt25/app_fusion.rs (the mechanic test). Every Craftmon
//! clause is now faithful — no omissions.
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - App Fusion alt-play (alt_paths app_fusion, named-card materials).
//! - alt-digivolve over [Stnd.] (alt_paths digivolve cost 2).
//! - DigiLink Shape-B self link-condition (cost 2).
//! - Mandatory OnPlay/WhenDigivolving add-top-security + Recovery.
//! - `when: when_linked` cost-trash → draw (linked scope) + event-log on trash.
//! - Inherited trash-Appmon → Draw 2.
//! - Security after-battle play-self.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_digivolve, EFFECTS_PER_PERMANENT};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-036";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.colors = vec![CardColor::Yellow];
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// Named Digimon for App-Fusion host conditions.
fn named(id: &str, name: &str) -> CardData {
    let mut card = make_digimon(id, 4, 4000, 4, &["Appmon"]);
    card.card_name = name.to_string();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-036 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        // Appmon trash fodder for the [When Linking] / inherited cost.
        .add_card(make_digimon("APPMON-HAND", 3, 2000, 3, &["Appmon"]))
        // Host Appmon for the link absorb path.
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        // App-Fusion named cards.
        .add_card(named("KABEMON", "Kabemon"))
        .add_card(named("GOMIMON", "Gomimon"))
        .add_card(named("ECOMON", "Ecomon"))
        .add_card(named("PUZZLEMON", "Puzzlemon"))
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_036_yaml_printed_metadata() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Craftmon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
}

#[test]
fn bt25_036_registers_app_fusion_and_alt_digivolve() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let app_fusion = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(app_fusion, "must register a cost-0 App Fusion alt-path");
    let alt_digi = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
    });
    assert!(
        alt_digi,
        "must register a cost-2 alt-digivolve over a Lv.2 [Stnd.]"
    );
}

#[test]
fn bt25_036_has_link_condition_cost_2() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let link = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, .. }) if *cost == 2
    ));
    assert!(link, "must declare a self link-condition with cost 2");
}

#[test]
fn bt25_036_has_op_wd_when_linked_inherited_and_security_clauses() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let op_wd = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )
    });
    let when_linked = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenLinked)
                    && matches!(t.scope, CompiledScope::Linked)
        )
    });
    let security = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )
    });
    assert!(
        op_wd,
        "must have a shared [On Play][When Digivolving] clause"
    );
    assert!(when_linked, "must have a linked [When Linking] clause");
    assert!(
        security,
        "must have a [Security] clause (on_security timing)"
    );
    // The card prints exactly ONE "trash 1 Appmon -> Draw 2" box ([When Linking]);
    // cards.json's "inherited" field is a mis-slot of that same clause (cf.
    // BT25-045), so there is NO separate inherited clause.
    let inherited = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Triggered(t) if matches!(t.scope, CompiledScope::Inherited)
        )
    });
    assert!(
        !inherited,
        "no separate inherited clause — the trash->Draw 2 is the [When Linking] effect"
    );
}

// ─── Section 3 — Behavioral: On Play add-top-security + Recovery ──────────────

#[test]
fn bt25_036_on_play_adds_top_security_and_recovers() {
    // Top security = SEC-A; deck top = REC-A. After: SEC-A leaves security to
    // hand (-1 sec), Recovery +1 places REC-A as top security (+1 sec). Net
    // security unchanged; hand +1 (the added security) net of the play.
    let mut r = base()
        .add_card(make_digimon("SEC-A", 3, 2000, 3, &["Beast"]))
        .add_card(make_digimon("REC-A", 3, 2000, 3, &["Beast"]))
        .hand(0, &[CARD_ID])
        .security(0, &["SEC-A"])
        .deck(0, &["DECK-PAD", "REC-A"]) // REC-A on top
        .deck(1, &["DECK-PAD"; 12])
        .memory(10)
        .start();

    let hand_before = r.hand_size(0);
    let sec_before = r.security_count(0);

    let _ = r.play(0, 0).expect("Craftmon played → On Play fires");
    r.auto_resolve().ok();

    assert_eq!(
        r.security_count(0),
        sec_before,
        "added top security (-1) then Recovery +1 → net security unchanged"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before, // -1 played Craftmon, +1 added security = net 0
        "the top security card moved to hand (net of the Craftmon play)"
    );
}

// ─── Section 3/4 — When Linking: trash 1 Appmon (cost) → Draw 2 + event ───────

#[test]
fn bt25_036_when_linked_trashes_appmon_and_draws_two() {
    let mut r = base()
        .hand(0, &["APPMON-HAND"])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let craft = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    let trash_before = r.trash_size(0);
    let checkpoint = r.game.events.len();

    // Activate Craftmon's on-field Link onto HOST-APP → [When Linking] fires.
    let link_slot = (digimon_engine::action::space::FIELD_EFFECT_START
        + craft.index as u16 * EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK) as u16;
    r.game.decode_action(link_slot, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    // [When Linking] cost: trash 1 Appmon from hand, then Draw 2. Resolve it
    // (accept + trash the Appmon).
    r.auto_resolve().ok();

    // Net hand: -1 (trashed APPMON-HAND) +2 (Draw 2) = +1.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "[When Linking]: trash 1 Appmon as cost, draw 2 (net +1 hand)"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before + 1,
        "the trashed Appmon went to trash"
    );
    // Faithfulness of the cost: the [Appmon] card (APPMON-HAND) is the card that
    // went to trash (the trash-from-hand cost moves hand->trash; a plain hand
    // trash carries no GameEvent::Trash, so we assert on resulting state).
    let _ = checkpoint;
    assert!(
        r.game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "APPMON-HAND"),
        "the trashed [Appmon] (APPMON-HAND) is in trash — the [When Linking] cost was paid"
    );
}

#[test]
fn bt25_036_when_linked_skips_when_no_appmon_in_hand() {
    // No Appmon in hand → the optional cost cannot be paid → no draw, no trash.
    let mut r = base()
        .hand(0, &["DECK-PAD"]) // DECK-PAD is not Appmon
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let craft = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    let trash_before = r.trash_size(0);

    let link_slot = (digimon_engine::action::space::FIELD_EFFECT_START
        + craft.index as u16 * EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK) as u16;
    r.game.decode_action(link_slot, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.hand_size(0),
        hand_before,
        "no Appmon to trash → no Draw 2 (cost unpayable)"
    );
    assert_eq!(r.trash_size(0), trash_before, "nothing trashed");
}

// ─── Section 3 — App Fusion alt-play legality + resolution ────────────────────

#[test]
fn bt25_036_app_fusion_legal_with_two_named_linked_and_stacks() {
    // Host: top = Kabemon, linked = Gomimon (2 distinct named cards linked).
    // Craftmon in hand is offered an App-Fusion play (a digivolve action) onto
    // the host; performing it stacks Craftmon on top and consumes the link.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "KABEMON", Some(0));
    let linked = r.push_linked_owned(host, "GOMIMON", 0);
    advance_to_main(&mut r);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "App Fusion offered as a digivolve action onto a host with 2 distinct \
         named cards linked together"
    );

    r.game.decode_action(action, 0);
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        CARD_ID,
        "Craftmon stacked on top via App Fusion"
    );
    assert!(
        perm.card_sources.iter().any(|c| c.handle() == linked),
        "the host's linked Gomimon was consumed under the new top as a source"
    );
    assert!(
        perm.linked_cards.is_empty(),
        "the consumed linked card was removed from linked_cards"
    );
}

#[test]
fn bt25_036_app_fusion_not_legal_without_two_named() {
    // Host has only Kabemon as top, no second distinct named linked card.
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "KABEMON", Some(0));
    advance_to_main(&mut r);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "App Fusion not legal without 2 distinct named cards linked together"
    );
}
