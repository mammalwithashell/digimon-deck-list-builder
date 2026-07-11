//! BT25-072 Shutmon — Digimon, Lv.5, Black/Purple, DP 7000, Cost 7.
//! Trait line (official Bandai DB / card image): Ult./Appmon | Tool | Forced Termination.
//!
//! # Printed text (data/card_bundles/BT25-072.md — authoritative)
//!
//! Digivolve circles (card image): split Black/Purple "Lv.4 / cost 4" circle
//!   (official DB: Black Lv.4/4 AND Purple Lv.4/4) + rainbow "Sup. / cost 3"
//!   circle (any colour, Appmon Super-grade form gate — DCGO
//!   HasSuperAppTraits = EqualsTraits("Sup."), no level gate).
//! [App Fusion] [Logamon] & [Timemon]: Cost 0.
//! <Jamming> (self).
//! [On Play] [When Digivolving] [When Attacking] If it's your turn, you may
//!   link 1 [Social], [Tool] or [Game] trait Digimon card from your TRASH or
//!   this Digimon's digivolution cards to this Digimon with the cost reduced
//!   by 2. (DCGO optional: false / isSkippable: true — the trigger fires, the
//!   link itself is declinable.)
//! [All Turns] [Once Per Turn] When this Digimon gets linked, 1 of your
//!   opponent's Digimon or Tamers can't digivolve until their turn ends.
//!   (Mandatory when a target exists — canNoSelect: false; OPT tag printed.)
//! Link box: <Link> [Appmon] trait: Cost 3 · Link DP +4000 ·
//!   [When Linking] 2 of your opponent's Digimon or Tamers can't unsuspend
//!   until their turn ends. (SetIsLinkedEffect(true); DCGO maxCount =
//!   min(2, available), canNoSelect: false, canEndNotMax: false → mandatory,
//!   full count.)
//!
//! # DCGO C# reference (READ-ONLY)
//! DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_072.cs
//! (Link DP +4000 and the standard Black/Purple Lv.4 circles are data-driven
//! in DCGO, not per-card C# — authored in YAML as a scope:linked dp aura and
//! equal-cost alt_paths per pool convention, same as BT25-056 / BT25-061.)
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3; BT25-056 is the near-twin)
//! - Standard-circle + trait-gated ("Sup.") + App Fusion alt-path registration
//! - link_cards from [trash, self_sources] (trash variant of the BT25-056 hand link)
//! - <Jamming> self keyword + linked +4000 DP aura
//! - B3 when_card_linked_to_this host-side trigger, [All Turns] + [Once Per
//!   Turn] (positive + tamer target + all-turns + OPT lockout + OPT reset)
//! - Linked-scope [When Linking] capped 2-target CannotUnsuspend

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{
    encode_digivolve, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_LINK, FIELD_EFFECT_START, PASS,
};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{
    CardColor, CardKind, EffectTiming, Keyword, ModifierType, PlaySource, PlayerId,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT25-072";

// ── Helpers ───────────────────────────────────────────────────────────────────

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

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card
}

fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(idx, player as u8, iid));
}

fn link_bit(perm: PermanentHandle) -> usize {
    (FIELD_EFFECT_START + perm.index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_LINK)
        as usize
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-072 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(make_digimon("TOOL-IN-TRASH", 4, 4000, 4, &["Tool"]))
        .add_card(make_digimon("PLAIN-X", 3, 2000, 3, &["Beast"]))
        .add_card(make_digimon("HOST-APP", 4, 4000, 4, &["Appmon"]))
        .add_card(make_digimon("OPP-A", 4, 4000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-B", 4, 4000, 4, &["Beast"]))
        .add_card(make_tamer("OPP-TAMER"))
        .add_card(make_digimon("LINK-FODDER", 3, 1000, 2, &["Game"]))
        // Digivolve bases. `make_test_card` defaults to Red, so SUP-BASE
        // matches ONLY the "Sup. / 3" circle (rainbow ring, no colour gate)
        // and RED-BASE (plain Beast) matches NO printed circle at all.
        .add_card(make_digimon("SUP-BASE", 4, 5000, 4, &["Sup.", "Appmon"]))
        .add_card(make_colored_digimon(
            "BLACK-BASE",
            4,
            5000,
            4,
            &["Beast"],
            &[CardColor::Black],
        ))
        // Black Lv.4 base that is ALSO a [Tool] Digimon — after digivolving it
        // becomes a digivolution card matching the shared clause's filter.
        .add_card(make_colored_digimon(
            "BLACK-TOOL-BASE",
            4,
            5000,
            4,
            &["Tool"],
            &[CardColor::Black],
        ))
        .add_card(make_colored_digimon(
            "PURPLE-BASE",
            4,
            5000,
            4,
            &["Beast"],
            &[CardColor::Purple],
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

/// Drive Shutmon's own link action onto a host: decode_action → resolve the
/// (mandatory) host pick. Same pattern as bt25_056.rs / bt25_061.rs.
fn do_link(r: &mut DebugRunner, linking: PermanentHandle) {
    r.game.decode_action(link_bit(linking) as u16, 0);
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
}

/// Fire Shutmon's host-side when_card_linked_to_this trigger by pushing a
/// plain fodder card as a linked card onto `host` (the Shutmon permanent)
/// and dispatching OnLink, then draining. (BT25-052 / BT24-067 idiom.)
fn fire_link_onto_host(runner: &mut DebugRunner, host: PermanentHandle) {
    let linked_handle = runner.push_linked_owned(host, "LINK-FODDER", 0);
    for pid in 0..2usize {
        runner.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: linked_handle,
            },
        );
    }
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_072_yaml_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Shutmon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    assert_eq!(
        card.color,
        vec![CompiledColor::Black, CompiledColor::Purple],
        "Shutmon is Black/Purple"
    );
    // Trait line Ult./Appmon | Tool | Forced Termination (official Bandai DB)
    // — production merges form + attribute + type into `traits`.
    for t in ["Ult.", "Appmon", "Tool", "Forced Termination"] {
        assert!(
            card.traits.iter().any(|x| x == t),
            "trait line must include {t:?}"
        );
    }
    assert_eq!(
        card.attribute.as_deref(),
        Some("Tool"),
        "printed Attribute: Tool (was mis-authored Virus)"
    );
}

#[test]
fn bt25_072_has_link_condition_appmon_cost_3() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::LinkCondition { cost, filter, .. })
            if *cost == 3 && filter.trait_has.as_deref() == Some("Appmon")
    ));
    assert!(
        has,
        "BT25-072 must declare a self link-condition with cost 3 over [Appmon] hosts"
    );
}

/// Link box "DP +4000": scope-linked aura applying +4000 DP to the host.
/// Pre-fix (2026-07-10) the aura was missing entirely.
#[test]
fn bt25_072_has_linked_dp_aura_4000() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Linked,
                dp_modifier: Some(4000),
                ..
            })
        )
    });
    assert!(
        has,
        "BT25-072 must declare a scope:linked +4000 DP aura (printed Link DP +4000)"
    );
}

#[test]
fn bt25_072_grants_jamming() {
    let mut r = base().memory(5).start();
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    assert!(
        r.game.has_keyword(shut, Keyword::Jamming),
        "BT25-072 has <Jamming>"
    );
}

#[test]
fn bt25_072_registers_printed_circles_and_sup_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    // Printed split circle → two standard rows: Black Lv.4/4 AND Purple Lv.4/4.
    for color in [CompiledColor::Black, CompiledColor::Purple] {
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
            "BT25-072 must register the printed standard circle {color:?} Lv.4 / cost 4"
        );
    }
    // Printed "Sup. / 3" circle: trait gate "Sup." (DCGO HasSuperAppTraits =
    // EqualsTraits("Sup.")), cost 3, NO level gate, NO colour gate.
    // Pre-fix (2026-07-10) this was authored as { level_eq: 4, trait_has:
    // "Super App" } — a trait string no production card carries (dead path).
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
        "BT25-072 must register the printed \"Sup. / cost 3\" circle as a trait_has \
         \"Sup.\" alt-path with no level/colour gate"
    );
}

/// [App Fusion] [Logamon] & [Timemon]: Cost 0 (DCGO AddAppfuseMethodByName).
/// Pre-fix this was OMITTED as BLOCKED; the app_fusion alt-path primitive has
/// since landed (BT25-052/BT25-056 precedent).
#[test]
fn bt25_072_registers_app_fusion_logamon_timemon() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
            && p.materials.first().is_some_and(|m| {
                m.filter.name_in.as_ref().is_some_and(|names| {
                    names.contains(&"Logamon".to_string())
                        && names.contains(&"Timemon".to_string())
                })
            })
    });
    assert!(
        has,
        "BT25-072 must register [App Fusion] [Logamon] & [Timemon]: Cost 0"
    );
}

#[test]
fn bt25_072_clause_shapes() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    // One shared triggered clause covering OP + WD + WA, gated on your_turn,
    // with no OPT (the printed OPT tag belongs to the [All Turns] clause).
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

    // [All Turns][Once Per Turn] host-side when-linked trigger — mandatory
    // selection, OPT flag set (printed [Once Per Turn], unlike BT25-056).
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
        host_side[0].once_per_turn,
        "printed [Once Per Turn] tag on the [All Turns] deny-digivolve clause"
    );
    assert!(
        host_side[0].active_when.is_none(),
        "[All Turns] — no turn gate on the deny-digivolve clause"
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
        "BT25-072 must have a scope:linked [When Linking] clause"
    );
}

// ─── Section 2 — Shared [OP]/[WD]/[WA] self-link (from TRASH / sources) ──────

#[test]
fn bt25_072_on_play_links_tool_card_from_trash_to_self() {
    // Tool card sits in player 0's trash; Shutmon's On Play links it onto
    // itself (printed zone is the TRASH, not the hand — contrast BT25-056).
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    advance_to_main(&mut r);

    let shut_idx = r.play(0, 0).expect("Shutmon played");
    // On Play (your turn) installs the link selection over the trash Tool card.
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[shut_idx].linked_cards.len(),
        1,
        "the Tool card from trash attached to Shutmon"
    );
    assert_eq!(r.trash_size(0), 0, "Tool card left the trash");
}

#[test]
fn bt25_072_on_play_link_is_declinable() {
    // Printed "you may link 1" (DCGO isSkippable: true) — PASS declines the
    // link; nothing links and the [All Turns] deny-digivolve never fires.
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let shut_idx = r.play(0, 0).expect("Shutmon played");
    assert!(
        r.game.pending_selection.is_some(),
        "On Play self-link installs a selection"
    );
    r.execute_action(0, PASS).expect("decline the optional link");
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[shut_idx].linked_cards.len(),
        0,
        "declined ⇒ nothing linked"
    );
    assert_eq!(r.trash_size(0), 1, "declined ⇒ the Tool card stays in trash");
    assert!(
        !r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "declined ⇒ Shutmon never got linked ⇒ no deny-digivolve"
    );
}

#[test]
fn bt25_072_on_play_no_matching_candidate_no_prompt() {
    // Trash holds only a Beast Digimon (not [Social]/[Tool]/[Game]) — the
    // filter has no candidate, so the link step no-ops with no selection.
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    seed_trash(&mut r, 0, "PLAIN-X");
    advance_to_main(&mut r);

    let shut_idx = r.play(0, 0).expect("Shutmon played");
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[shut_idx].linked_cards.len(),
        0,
        "no [Social]/[Tool]/[Game] Digimon in trash ⇒ nothing linked"
    );
    assert_eq!(r.trash_size(0), 1, "the Beast card stays in trash");
}

#[test]
fn bt25_072_when_digivolving_links_tool_from_digivolution_cards() {
    // [When Digivolving] shares the same clause, and the second printed zone
    // is "this Digimon's digivolution cards": digivolve over a black Lv.4
    // [Tool] base (printed Black Lv.4 / cost 4 circle) — the base becomes a
    // digivolution card matching the filter, and the link selection surfaces.
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    advance_to_main(&mut r);
    let slot = r.place_on_field(0, "BLACK-TOOL-BASE", Some(0)).index as usize;

    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "Shutmon digivolves over the black Lv.4 [Tool] base");

    assert!(
        r.game.pending_selection.is_some(),
        "[When Digivolving] self-link installs a selection over the digivolution card"
    );
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);
    r.auto_resolve().ok();

    assert_eq!(
        r.game.player(0).battle_area[slot].linked_cards.len(),
        1,
        "the [Tool] digivolution card linked to Shutmon after digivolving"
    );
}

// ─── Section 3 — Digivolution paths ──────────────────────────────────────────

/// Digivolve Shutmon from hand over `base_id`, asserting the mask offers it
/// and exactly `cost` memory is paid. (BT25-056 idiom.)
fn assert_digivolve_ok(base_id: &str, cost: i16) {
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    r.game.turn_count = 1;
    advance_to_main(&mut r);
    let slot = r.place_on_field(0, base_id, Some(0)).index as usize;

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 1.0,
        "mask must offer Shutmon's digivolve over {base_id}"
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(ok, "Shutmon digivolves over {base_id}");
    assert_eq!(
        mem_before - r.game.memory,
        cost,
        "digivolving over {base_id} pays exactly {cost}"
    );
    r.auto_resolve().ok();
}

#[test]
fn bt25_072_digivolves_over_sup_form_base_for_3() {
    // "Sup. / 3" circle: any colour (SUP-BASE is default Red — neither
    // colour circle matches), gated only on the "Sup." form trait.
    assert_digivolve_ok("SUP-BASE", 3);
}

#[test]
fn bt25_072_digivolves_over_black_lv4_for_4() {
    assert_digivolve_ok("BLACK-BASE", 4);
}

#[test]
fn bt25_072_digivolves_over_purple_lv4_for_4() {
    assert_digivolve_ok("PURPLE-BASE", 4);
}

#[test]
fn bt25_072_rejected_over_unmatched_red_lv4_base() {
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
        "mask must NOT offer Shutmon's digivolve over an unmatched red Lv.4"
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(!ok, "digivolve over the unmatched red Lv.4 must be rejected");
    assert_eq!(r.game.memory, mem_before, "no memory paid");
}

// ─── Section 4 — [All Turns][OPT] when this gets linked: deny-digivolve ──────

#[test]
fn bt25_072_when_linked_denies_opponent_digivolve() {
    // Integration path: On Play self-link from trash → when_card_linked_to_this
    // fires → mandatory pick → CannotDigivolve on the chosen opponent.
    let mut r = base().hand(0, &[CARD_ID]).memory(10).start();
    seed_trash(&mut r, 0, "TOOL-IN-TRASH");
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    let _shut = r.play(0, 0).expect("Shutmon played");
    // Resolve the self-link (trash Tool) → fires when_card_linked_to_this.
    let link_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, link_action);

    // Host-side trigger: select the opponent Digimon to deny digivolve
    // (mandatory — a selection MUST surface, no auto-skip).
    assert!(
        r.game.pending_selection.is_some(),
        "When-linked deny-digivolve prompt surfaces"
    );
    let deny_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, deny_action);
    r.auto_resolve().ok();

    assert!(
        r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "opponent Digimon can't digivolve after Shutmon got linked"
    );
}

#[test]
fn bt25_072_when_linked_can_deny_opponent_tamer() {
    // Printed target set is "Digimon or Tamers": an opponent TAMER is legal.
    let mut r = base().memory(10).start();
    advance_to_main(&mut r);
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp_tamer = r.place_on_field(1, "OPP-TAMER", Some(0));

    fire_link_onto_host(&mut r, shut);

    assert!(
        r.game.pending_selection.is_some(),
        "when-linked fires with a Tamer as the only target"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    assert!(
        r.game.modifiers.has(opp_tamer, ModifierType::CannotDigivolve),
        "the opponent Tamer can't digivolve after Shutmon got linked"
    );
}

#[test]
fn bt25_072_when_linked_fires_on_opponents_turn() {
    // [All Turns]: the trigger has NO turn gate — a link during the
    // opponent's turn still fires (contrast BT25-052's [Your Turn] gate).
    let mut r = base().memory(10).start();
    // Advance to player 1's turn, then give the turn player memory so their
    // turn does NOT immediately end after the effect resolves (memory is
    // turn-player-relative; `check_turn_end` fires on memory < 0, and the
    // modifier's printed window is "until THEIR turn ends" — asserting after
    // the turn flipped back would correctly see it expired).
    r.end_turn();
    r.game.memory = 10;
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));

    fire_link_onto_host(&mut r, shut);

    assert!(
        r.game.pending_selection.is_some(),
        "[All Turns]: the deny-digivolve trigger fires on the opponent's turn too"
    );
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    assert_eq!(r.game.turn_player(), 1, "still the opponent's turn");
    assert!(
        r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "deny-digivolve resolves on the opponent's turn"
    );

    // And the printed window closes with THEIR turn: once the opponent's
    // turn ends, the lock expires.
    r.end_turn();
    assert!(
        !r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "the lock expires at the end of the opponent's turn"
    );
}

#[test]
fn bt25_072_when_linked_no_targets_no_prompt() {
    // Mandatory selection, but with no opponent permanents there is no legal
    // target — the effect no-ops silently (DCGO HasMatchConditionPermanent).
    let mut r = base().memory(10).start();
    advance_to_main(&mut r);
    let shut = r.place_on_field(0, CARD_ID, Some(0));

    fire_link_onto_host(&mut r, shut);

    assert!(
        r.game.pending_selection.is_none(),
        "no opponent Digimon/Tamer ⇒ no prompt"
    );
}

/// [Once Per Turn]: a second link onto Shutmon in the same turn must not
/// re-fire the deny-digivolve trigger.
#[test]
fn bt25_072_when_linked_opt_blocks_second_link_same_turn() {
    let mut r = base().memory(10).start();
    advance_to_main(&mut r);
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let opp_b = r.place_on_field(1, "OPP-B", Some(0));

    // First link — fires; resolve the pick (consumes the OPT).
    fire_link_onto_host(&mut r, shut);
    assert!(r.game.pending_selection.is_some(), "first link fires");
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    // Second link in the same turn: OPT must lock the trigger out even
    // though another legal target remains.
    fire_link_onto_host(&mut r, shut);
    assert!(
        r.game.pending_selection.is_none(),
        "[Once Per Turn]: second link in the same turn must not prompt"
    );
    let denied = [opp_a, opp_b]
        .iter()
        .filter(|h| r.game.modifiers.has(**h, ModifierType::CannotDigivolve))
        .count();
    assert_eq!(denied, 1, "only the first link denied a permanent");
}

/// OPT resets across turns: after an end-turn cycle the trigger fires again.
#[test]
fn bt25_072_when_linked_opt_resets_after_end_turn() {
    let mut r = base().memory(10).start();
    advance_to_main(&mut r);
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));

    // First link — fires; resolve (consumes the OPT).
    fire_link_onto_host(&mut r, shut);
    assert!(r.game.pending_selection.is_some(), "first link fires");
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    let _ = r.game.resolve_selection(0, action);
    r.game.drain_effect_queue();

    // End-turn cycle: player 0 → player 1 → player 0 again.
    r.end_turn();
    r.end_turn();
    r.game.enter_main_phase();

    fire_link_onto_host(&mut r, shut);
    assert!(
        r.game.pending_selection.is_some(),
        "OPT must clear after end_turn cycle — the trigger fires again"
    );
}

// ─── Section 5 — Link box behavioral ─────────────────────────────────────────

#[test]
fn bt25_072_linked_aura_gives_host_plus_4000_dp() {
    // No opponent permanents: the [When Linking] pick has no legal target and
    // no-ops, isolating the Link DP +4000 aura.
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0)); // 4000 base DP
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    advance_to_main(&mut r);

    let dp_before = r.effective_dp(host).expect("host on field");
    do_link(&mut r, shut);
    r.auto_resolve().ok();

    assert_eq!(
        r.effective_dp(host),
        Some(dp_before + 4000),
        "host effective DP must increase by +4000 while Shutmon is linked (printed Link DP +4000)"
    );
}

#[test]
fn bt25_072_when_linking_locks_two_opponents() {
    // [When Linking]: 2 opp Digimon or Tamers can't unsuspend until their
    // turn ends. With a Digimon AND a Tamer available, BOTH must be picked
    // (DCGO maxCount = min(2, available), canEndNotMax: false — mandatory
    // full count; the Tamer proves the printed "Digimon or Tamers" scope).
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp_a = r.place_on_field(1, "OPP-A", Some(0));
    let opp_tamer = r.place_on_field(1, "OPP-TAMER", Some(0));
    advance_to_main(&mut r);

    do_link(&mut r, shut);
    assert!(
        r.game.pending_selection.is_some(),
        "[When Linking] 2-target pick surfaces"
    );
    r.auto_resolve().ok();

    assert!(
        r.game.modifiers.has(opp_a, ModifierType::CannotUnsuspend),
        "the opponent Digimon can't unsuspend"
    );
    assert!(
        r.game.modifiers.has(opp_tamer, ModifierType::CannotUnsuspend),
        "the opponent Tamer can't unsuspend (both of the 2 required picks applied)"
    );
}

#[test]
fn bt25_072_when_linking_caps_at_one_available_target() {
    // Only 1 opponent permanent: the pick caps at min(2, 1) = 1 and the link
    // still completes (aura live).
    let mut r = base().memory(10).start();
    let host = r.place_on_field(0, "HOST-APP", Some(0));
    let shut = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    advance_to_main(&mut r);

    do_link(&mut r, shut);
    assert!(
        r.game.pending_selection.is_some(),
        "[When Linking] pick surfaces with a single target"
    );
    r.auto_resolve().ok();

    assert!(
        r.game.modifiers.has(opp, ModifierType::CannotUnsuspend),
        "the lone opponent permanent can't unsuspend"
    );
    assert_eq!(
        r.effective_dp(host),
        Some(4000 + 4000),
        "link completed (DP aura live) with only one available target"
    );
}
