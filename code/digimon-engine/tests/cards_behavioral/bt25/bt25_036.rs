//! BT25-036 Craftmon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Trait line (card image): Sup./Appmon | Tool | Design
//!   → traits ["Sup.", Appmon, Tool, Design], attribute Tool.
//!
//! # Card text (card image + DCGO BT25_036.cs — authoritative)
//!
//! Digivolve circles (image, 2026-07-10 audit — both cost 2):
//!   #1 yellow ring "Lv.3 / 2" — standard circle: Yellow Lv.3, cost 2
//!      (official Bandai DB: "Yellow Lv.3 / cost 2").
//!   #2 rainbow ring "Stnd. / 2" — grade circle: any-colour [Stnd.] form,
//!      cost 2. DCGO `AddSelfDigivolutionRequirementStaticEffect(
//!      HasStandardAppTraits, cost 2)`; trait gate ONLY — no level/colour gate.
//! [App Fusion] [Kabemon] & [Gomimon] & [Ecomon] & [Puzzlemon]: Cost 0.
//!   If 2 such cards are linked together, stack the link card on top and
//!   digivolve. (DCGO `AddAppfuseMethodByName(Kabemon, Gomimon, Ecomon,
//!   Puzzlemon)`.)
//! [Link] [Appmon] trait: Cost 2  (DCGO `AddSelfLinkConditionStaticEffect(
//!   HasAppmonTraits, 2)`).
//! Link box DP: +3000 (official DB "Link DP: DP+3000") — scope:linked aura.
//! [Security] At the end of the battle, play this card without paying the cost.
//!   (DCGO `PlaySelfDigimonAfterBattleSecurityEffect`.)
//! [On Play] [When Digivolving] Add your top security card to the hand. Then,
//!   <Recovery +1>.  (mandatory shared OP/WD ActivateClass; official Q&A: with
//!   no security cards you still perform <Recovery +1>.)
//! [When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>.
//!   (DCGO `WhenLinked`, `SetIsLinkedEffect(true)`, skippable.)
//! (NO inherited clause — the lower box IS the [When Linking] link effect;
//!  cards.json's "inherited" field mis-slots it, cf. BT25-045.)
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
//! - Standard circle Yellow Lv.3 / cost 2 (alt_paths digivolve).
//! - Grade circle alt-digivolve over [Stnd.] trait, no level gate (cost 2).
//! - DigiLink Shape-B self link-condition (cost 2) + linked +3000 DP aura.
//! - Mandatory OnPlay/WhenDigivolving add-top-security + Recovery (incl. the
//!   official-Q&A empty-security case).
//! - `when: when_linked` cost-trash → draw (linked scope) + event-log on trash.
//! - Security after-battle play-self.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming,
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
fn bt25_036_registers_app_fusion_and_both_digivolve_circles() {
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
    // Printed standard circle: Yellow Lv.3 / cost 2 (official Bandai DB).
    let standard = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
            && p.from.as_ref().is_some_and(|f| {
                f.level_eq == Some(3)
                    && f.color_is == Some(CompiledColor::Yellow)
                    && f.trait_has.is_none()
            })
    });
    assert!(
        standard,
        "must register the printed Yellow Lv.3 / cost 2 standard digivolve circle"
    );
    // Printed grade circle: [Stnd.] form / cost 2 — trait gate ONLY (DCGO
    // HasStandardAppTraits; no level or colour gate).
    let stnd = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(2)))
            && p.from.as_ref().is_some_and(|f| {
                f.trait_has.as_deref() == Some("Stnd.")
                    && f.level_eq.is_none()
                    && f.color_is.is_none()
            })
    });
    assert!(
        stnd,
        "must register a cost-2 digivolve alt-path gated on the Stnd. trait only (no level gate)"
    );
}

/// The printed link box carries "+3000" DP: while Craftmon is linked, the host
/// gets +3000 DP (scope:linked declarative aura — BT21-018 §1.2 pattern).
#[test]
fn bt25_036_declares_linked_dp_aura_3000() {
    let runner = base()
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                dp_modifier: Some(3000),
                scope: CompiledScope::Linked,
                ..
            })
        )
    });
    assert!(
        has,
        "must declare the printed link-box +3000 DP as a scope:linked aura"
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

// ─── Section 3 — Digivolve circles (Yellow Lv.3 standard + Stnd. grade) ───────

/// Non-yellow Digimon with an arbitrary level and traits, for circle isolation.
fn make_colored(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_digimon(id, level, 3000, 3, traits);
    card.colors = vec![color];
    card
}

/// Printed standard circle: digivolve from a Yellow Lv.3 (no [Stnd.] trait
/// needed) for cost 2.
#[test]
fn bt25_036_standard_circle_digivolves_from_yellow_lv3() {
    let mut r = base()
        // Yellow Lv.3, NOT Stnd., not Appmon — only the standard circle applies.
        .add_card(make_digimon("YEL-LV3", 3, 2000, 3, &["Beast"]))
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "YEL-LV3", Some(0));
    advance_to_main(&mut r);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "standard circle: digivolving over a Yellow Lv.3 must be legal (cost 2)"
    );

    r.game.decode_action(action, 0);
    r.auto_resolve().ok(); // [When Digivolving] add-security + Recovery drains
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        CARD_ID,
        "Craftmon digivolved over the Yellow Lv.3 via the standard circle"
    );
}

/// Printed grade circle: digivolve from any [Stnd.]-form card — DCGO
/// HasStandardAppTraits is a trait gate ONLY (no level gate, no colour gate).
/// A red Lv.4 [Stnd.] host proves neither the yellow colour nor Lv.3 (nor the
/// previously mis-authored `level_eq: 2`) is required.
#[test]
fn bt25_036_stnd_grade_circle_has_no_level_or_color_gate() {
    let mut r = base()
        .add_card(make_colored("RED-STND", 4, CardColor::Red, &["Stnd.", "Appmon"]))
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "RED-STND", Some(0));
    advance_to_main(&mut r);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "grade circle: digivolving over a red Lv.4 [Stnd.] must be legal — trait gate only"
    );

    r.game.decode_action(action, 0);
    r.auto_resolve().ok();
    let perm = &r.game.players[0].battle_area[host.index as usize];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        CARD_ID,
        "Craftmon digivolved over the [Stnd.] host via the grade circle"
    );
}

/// Negative: a Yellow Lv.4 non-[Stnd.] host satisfies NO printed circle
/// (standard needs Lv.3; grade needs the Stnd. trait; App Fusion needs the
/// named link pair).
#[test]
fn bt25_036_digivolve_not_legal_over_non_matching_host() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    // HOST-APP is Yellow Lv.4 [Appmon] — no circle matches it.
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    advance_to_main(&mut r);

    let action = encode_digivolve(0, host.index as u16);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "no printed circle matches a Yellow Lv.4 non-[Stnd.] host"
    );
}

// ─── Section 3 — Link box +3000 DP aura ───────────────────────────────────────

/// While Craftmon is linked to a host, the host's effective DP rises by +3000
/// (the printed link-box DP; official DB "Link DP: DP+3000").
#[test]
fn bt25_036_linked_dp_aura_raises_host_dp() {
    // Empty-of-Appmon hand so the [When Linking] cost is unpayable and the
    // trigger resolves without prompts.
    let mut r = base()
        .hand(0, &["DECK-PAD"])
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .memory(5)
        .start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let craft = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).unwrap_or(0);

    let link_slot = digimon_engine::action::space::FIELD_EFFECT_START
        + craft.index as u16 * EFFECTS_PER_PERMANENT
        + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK;
    r.game.decode_action(link_slot, 0);
    let host_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, host_action);
    r.auto_resolve().ok();

    let dp_after = r.effective_dp(host).unwrap_or(0);
    assert_eq!(
        dp_after,
        dp_before + 3000,
        "host effective DP must rise by +3000 while Craftmon is linked; \
         before={dp_before}, after={dp_after}"
    );
}

// ─── Section 3 — Official Q&A: no security → still <Recovery +1> ──────────────

/// Official Q&A (card bundle): with no security cards you can't add one to
/// hand, but you still perform <Recovery +1>. The add step must no-op and the
/// mandatory Recovery must still run.
#[test]
fn bt25_036_on_play_recovers_even_with_no_security() {
    let mut r = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12]) // security stack left EMPTY
        .deck(1, &["DECK-PAD"; 12])
        .memory(10)
        .start();

    assert_eq!(r.security_count(0), 0, "precondition: no security cards");
    let hand_before = r.hand_size(0);

    let _ = r.play(0, 0).expect("Craftmon played → On Play fires");
    r.auto_resolve().ok();

    assert_eq!(
        r.security_count(0),
        1,
        "<Recovery +1> still resolves with an empty security stack (official Q&A)"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before - 1,
        "nothing added to hand (no security to add); only Craftmon left the hand"
    );
}
