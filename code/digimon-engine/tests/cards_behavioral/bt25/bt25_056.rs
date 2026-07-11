//! BT25-056 Bootmon — Digimon, Lv.5, Green/Yellow, DP 7000, Cost 7.
//! Trait line (official Bandai DB / card image): Ult./Appmon | Tool | Super Boot.
//!
//! # Printed text (data/card_bundles/BT25-056.md — authoritative)
//!
//! Digivolve circles (card image): split Green/Yellow "Lv.4 / cost 4" circle
//!   (official DB: Green Lv.4/4 AND Yellow Lv.4/4) + rainbow "Sup. / cost 3"
//!   circle (any colour, Appmon Super-grade form gate).
//! [App Fusion] [Logimon] & [Craftmon]: Cost 0.
//! <Barrier> (self).
//! [On Play] [When Digivolving] [When Attacking] If it's your turn, you may
//!   link 1 [Social], [Tool] or [Game] trait Digimon card from your hand or
//!   this Digimon's digivolution cards to this Digimon with the cost reduced
//!   by 2. (DCGO optional: false / isSkippable: true — the trigger fires, the
//!   link itself is declinable.)
//! [All Turns] When this Digimon gets linked, suspend 1 of your opponent's
//!   Digimon or Tamers. (Mandatory — canNoSelect: false; no OPT tag.)
//! Link box: <Link> [Appmon] trait: Cost 3 · Link DP +4000 ·
//!   [When Linking] Return 1 of your opponent's suspended Digimon to the
//!   bottom of the deck. (SetIsLinkedEffect(true); canNoSelect: false →
//!   mandatory when a target exists; DCGO DeckBottomBounceClass runs
//!   DiscardEvoRoots() first — the target's digivolution cards are TRASHED
//!   and only its top card is placed on the deck bottom.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_056.cs
//! (Link DP +4000 and the standard Green/Yellow Lv.4 circles are data-driven
//! in DCGO, not per-card C# — authored in YAML as a scope:linked dp aura and
//! equal-cost alt_paths per pool convention, same as BT21-073 / BT25-061.)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{
    encode_digivolve, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, PlaySource, PlayerId};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-056";

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_colored_digimon(
    id: &str,
    level: u8,
    dp: i32,
    cost: u16,
    traits: &[&str],
    colors: &[CardColor],
) -> CardData {
    let mut card = make_digimon(id, level, dp, cost, traits);
    card.colors = colors.to_vec();
    card
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-056 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-HAND", 4, 4000, 4, &["Tool"]))
        .add_card(make_digimon("PLAIN-X", 3, 2000, 3, &["Beast"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("OPP-A", 4, 4000, 4, &["Beast"]))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        // Digivolve bases. `make_test_card` defaults to Red, so SUP-BASE
        // matches ONLY the "Sup. / 3" circle (rainbow ring, no colour gate)
        // and RED-BASE (plain Beast) matches NO printed circle at all.
        .add_card(make_digimon("SUP-BASE", 4, 5000, 4, &["Sup.", "Appmon"]))
        .add_card(make_colored_digimon(
            "GREEN-BASE",
            4,
            5000,
            4,
            &["Beast"],
            &[CardColor::Green],
        ))
        .add_card(make_colored_digimon(
            "YELLOW-BASE",
            4,
            5000,
            4,
            &["Beast"],
            &[CardColor::Yellow],
        ))
        .add_card(make_colored_digimon(
            "RED-BASE",
            4,
            5000,
            4,
            &["Beast"],
            &[CardColor::Red],
        ))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

/// Drive Bootmon's own link action onto a host: decode_action → resolve the
/// (mandatory) host pick. Same pattern as bt25_061.rs.
fn do_link(r: &mut DebugRunner, linking: PermanentHandle) {
    r.game.decode_action(link_bit(linking) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_056_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Bootmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    // Printed: Green/Yellow (bundle + card image) — was mis-authored
    // green/blue (the user-reported BT25-056 colour bug's YAML sibling).
    assert_eq!(
        card.color,
        vec![CompiledColor::Green, CompiledColor::Yellow],
        "Bootmon is Green/Yellow"
    );
    // Trait line Ult./Appmon | Tool | Super Boot (official Bandai DB) —
    // production merges form + attribute + type into `traits`.
    for t in ["Ult.", "Appmon", "Tool", "Super Boot"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "trait line must include {t:?}"
        );
    }
    assert_eq!(
        card.attribute.as_deref(),
        Some("Tool"),
        "printed Attribute: Tool"
    );
}

#[test]
fn bt25_056_has_link_condition_appmon_cost_3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
            if *cost == 3 && filter.trait_has.as_deref() == Some("Appmon")
    ));
    assert!(
        has,
        "BT25-056 must declare a self link-condition with cost 3 over [Appmon] hosts"
    );
}

#[test]
fn bt25_056_has_linked_scope_dp_aura_4000() {
    // Link box: "DP +4000" — scope:linked self-aura on the host.
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let aura = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) if *scope == CompiledScope::Linked => Some(*dp_modifier),
        _ => None,
    });
    assert_eq!(
        aura,
        Some(Some(4000)),
        "BT25-056 must declare a linked-scope dp aura of +4000 DP (printed Link DP +4000)"
    );
}

#[test]
fn bt25_056_grants_barrier() {
    let mut r = base().memory(5).start();
    let boot = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(boot, Keyword::Barrier),
        "BT25-056 has <Barrier>"
    );
}

#[test]
fn bt25_056_registers_printed_circles_and_sup_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // Printed split circle → two standard rows: Green Lv.4/4 AND Yellow Lv.4/4.
    for color in [CompiledColor::Green, CompiledColor::Yellow] {
        let has = card.alt_paths.iter().any(|p| {
            matches!(p.kind, CompiledAltPathKind::Digivolve)
                && matches!(p.cost, Some(CompiledCost::Literal(4)))
                && p.from
                    .as_ref()
                    .map(|f| f.level_eq == Some(4) && f.color_is == Some(color))
                    .unwrap_or(false)
        });
        assert!(
            has,
            "BT25-056 must register the printed standard circle {color:?} Lv.4 / cost 4"
        );
    }
    // Printed "Sup. / 3" circle: trait gate "Sup." (DCGO HasSuperAppTraits =
    // EqualsTraits("Sup.")), cost 3, NO level gate, NO colour gate.
    let sup = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(3)))
            && p.from
                .as_ref()
                .map(|f| {
                    f.trait_has.as_deref() == Some("Sup.")
                        && f.level_eq.is_none()
                        && f.color_is.is_none()
                })
                .unwrap_or(false)
    });
    assert!(
        sup,
        "BT25-056 must register the printed \"Sup. / cost 3\" circle as a trait_has \
         \"Sup.\" alt-path with no level/colour gate"
    );
}

#[test]
fn bt25_056_registers_app_fusion_logimon_craftmon() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.materials.first().is_some_and(|m| {
                m.filter
                    .name_in
                    .as_ref()
                    .is_some_and(|names| names.contains(&"Logimon".to_string())
                        && names.contains(&"Craftmon".to_string()))
            })
    });
    assert!(
        has,
        "BT25-056 must register [App Fusion] [Logimon] & [Craftmon]: Cost 0"
    );
}

#[test]
fn bt25_056_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    // One shared triggered clause covering OP + WD + WA, gated on your_turn,
    // with no OPT (printed text carries no [Once Per Turn]).
    let shared: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        shared.len(),
        1,
        "exactly 1 triggered clause must cover OnPlay+WhenDigivolving+WhenAttacking"
    );
    assert_eq!(
        shared[0].active_when.as_ref().and_then(|p| p.your_turn),
        Some(true),
        "the shared clause is gated on \"If it's your turn\""
    );
    assert!(
        !shared[0].once_per_turn,
        "no [Once Per Turn] tag on the shared clause"
    );

    // [All Turns] host-side when-linked trigger — mandatory, no OPT.
    let host_side: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenCardLinkedToThis) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        host_side.len(),
        1,
        "exactly 1 [All Turns] when-this-gets-linked clause"
    );
    assert!(
        !host_side[0].once_per_turn,
        "[All Turns] suspend fires on EVERY link (no OPT)"
    );

    // Link box [When Linking] — linked scope.
    let when_linking = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.when.contains(&CompiledTiming::WhenLinked)
                && matches!(t.scope, CompiledScope::Linked)
        }
        _ => false,
    });
    assert!(
        when_linking,
        "BT25-056 must have a scope:linked [When Linking] clause"
    );
}

// ─── Section 2 — Shared [OP]/[WD]/[WA] self-link behavioral ─────────────────

#[test]
fn bt25_056_on_play_links_tool_from_hand_then_suspends_opponent() {
    let mut r = base().hand(0, &[CARD_ID, "TOOL-IN-HAND"]).memory(10).start();
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let boot_idx = r.play(0, 0).expect("Bootmon played");
    // On Play (your turn) installs the link selection over the hand Tool card.
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    assert_eq!(
        r.game.player(0).battle_area[boot_idx].linked_cards.len(),
        1,
        "the Tool card from hand attached to Bootmon"
    );

    // [All Turns] When this Digimon gets linked: suspend 1 opponent permanent
    // (mandatory — a selection MUST surface, no auto-skip).
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked suspend prompt surfaces"
    );
    let suspend_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, suspend_action);

    assert!(
        r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "opponent Digimon suspended after Bootmon got linked"
    );
}

#[test]
fn bt25_056_on_play_link_is_declinable() {
    // Printed "you may link 1" (DCGO isSkippable: true) — PASS declines the
    // link; nothing links and the [All Turns] suspend never fires.
    let mut r = base().hand(0, &[CARD_ID, "TOOL-IN-HAND"]).memory(10).start();
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let boot_idx = r.play(0, 0).expect("Bootmon played");
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    r.execute_action(0, PASS).expect("decline the optional link");
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[boot_idx].linked_cards.len(),
        0,
        "declined ⇒ nothing linked"
    );
    assert!(
        !r.game.player(1).battle_area[opp.index as usize].is_suspended,
        "declined ⇒ Bootmon never got linked ⇒ no suspend"
    );
}

#[test]
fn bt25_056_on_play_no_matching_candidate_no_prompt() {
    // Hand holds only a Beast Digimon (not [Social]/[Tool]/[Game]) — the
    // filter has no candidate, so the link step no-ops with no selection.
    let mut r = base().hand(0, &[CARD_ID, "PLAIN-X"]).memory(10).start();
    advance_to_main(&mut r);

    let boot_idx = r.play(0, 0).expect("Bootmon played");
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[boot_idx].linked_cards.len(),
        0,
        "no [Social]/[Tool]/[Game] Digimon available ⇒ nothing linked"
    );
}

#[test]
fn bt25_056_when_digivolving_offers_link() {
    // [When Digivolving] shares the same clause: digivolve over a green Lv.4
    // base (printed Green Lv.4 / cost 4 circle) with a Tool card in hand →
    // the link selection surfaces and resolves.
    let mut r = base()
        .hand(0, &[CARD_ID, "TOOL-IN-HAND"])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    advance_to_main(&mut r);
    let slot = r.place_on_field(0, "GREEN-BASE", Some(0)).index as usize;

    let ok = r
        .game
        .digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "Bootmon digivolves over the green Lv.4 base");

    assert!(
        r.game.pending_selection.is_some(),
        "[When Digivolving] self-link installs a selection"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[slot].linked_cards.len(),
        1,
        "the Tool card from hand linked to Bootmon after digivolving"
    );
}

// ─── Section 3 — Digivolution paths ──────────────────────────────────────────

/// Digivolve Bootmon from hand over `base_id`, asserting the mask offers it
/// and exactly `cost` memory is paid.
fn assert_digivolve_ok(base_id: &str, cost: i16) {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    advance_to_main(&mut r);
    let slot = r.place_on_field(0, base_id, Some(0)).index as usize;

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 1.0,
        "mask must offer Bootmon's digivolve over {base_id}"
    );

    let mem_before = r.game.memory;
    let ok = r
        .game
        .digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "Bootmon digivolves over {base_id}");
    assert_eq!(
        mem_before - r.game.memory,
        cost,
        "digivolving over {base_id} pays exactly {cost}"
    );
    r.auto_resolve().ok();
}

#[test]
fn bt25_056_digivolves_over_sup_form_base_for_3() {
    // "Sup. / 3" circle: any colour (SUP-BASE is default Red — neither
    // colour circle matches), gated only on the "Sup." form trait.
    assert_digivolve_ok("SUP-BASE", 3);
}

#[test]
fn bt25_056_digivolves_over_green_lv4_for_4() {
    assert_digivolve_ok("GREEN-BASE", 4);
}

#[test]
fn bt25_056_digivolves_over_yellow_lv4_for_4() {
    assert_digivolve_ok("YELLOW-BASE", 4);
}

#[test]
fn bt25_056_rejected_over_unmatched_red_lv4_base() {
    // Negative control: a red Lv.4 with no "Sup." trait matches NO printed
    // circle — mask and action layer must both reject.
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    advance_to_main(&mut r);
    let slot = r.place_on_field(0, "RED-BASE", Some(0)).index as usize;

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 0.0,
        "mask must NOT offer Bootmon's digivolve over an unmatched red Lv.4"
    );

    let mem_before = r.game.memory;
    let ok = r
        .game
        .digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(!ok, "digivolve over the unmatched red Lv.4 must be rejected");
    assert_eq!(r.game.memory, mem_before, "no memory paid");
}

// ─── Section 4 — Link box behavioral ─────────────────────────────────────────

#[test]
fn bt25_056_linked_aura_gives_host_plus_4000_dp() {
    // No opponent permanents: the [When Linking] pick has no legal target and
    // no-ops, isolating the Link DP +4000 aura.
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0)); // 4000 base DP
    let boot = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).expect("host on field");
    do_link(&mut r, boot);
    r.auto_resolve().ok();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 4000),
        "host effective DP must increase by +4000 while Bootmon is linked (printed Link DP +4000)"
    );
}

#[test]
fn bt25_056_when_linking_bottom_decks_suspended_opponent() {
    // [When Linking]: return 1 opp SUSPENDED Digimon to the deck bottom.
    // The bounced Digimon's digivolution card is TRASHED (DCGO
    // DeckBottomBounceClass → DiscardEvoRoots()); only the top card
    // bottom-decks.
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let boot = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_stack(1, &["OPP-SRC", "OPP-A"]);
    r.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    advance_to_main(&mut r);

    let opp_trash_before = r.trash_size(1);
    do_link(&mut r, boot);
    // [When Linking] pick: the suspended opponent Digimon (mandatory).
    assert!(
        r.game.pending_selection.is_some(),
        "[When Linking] bounce pick surfaces"
    );
    r.auto_resolve().ok();

    assert!(
        r.game.players[1].battle_area.is_empty(),
        "the suspended opponent Digimon left the battle area"
    );
    assert_eq!(
        r.game.players[1].deck[0].card_id(&r.game.card_data),
        "OPP-A",
        "the bounced top card sits on the BOTTOM of the opponent's deck"
    );
    assert_eq!(
        r.trash_size(1),
        opp_trash_before + 1,
        "the bounced Digimon's digivolution card went to the trash, not the deck"
    );
    assert!(
        r.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "OPP-SRC"),
        "the trashed card is the digivolution source"
    );
}

#[test]
fn bt25_056_when_linking_ignores_unsuspended_opponent() {
    // Filter negative: an UNSUSPENDED opponent Digimon is not a legal target —
    // the pick never fires, the link still completes (aura live).
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let boot = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0)); // standing
    advance_to_main(&mut r);

    do_link(&mut r, boot);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.players[1].battle_area.len(),
        1,
        "the unsuspended opponent Digimon stays on the field"
    );
    assert_eq!(
        r.effective_dp(host),
        Some(4000 + 4000),
        "link completed (DP aura live) even with no suspended opponent Digimon"
    );
}
