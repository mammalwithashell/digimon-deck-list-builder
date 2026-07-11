//! EX10-020 Puppetmon — Digimon, Lv.6, Green, DP 11000, Cost 11.
//! Traits: Puppet / Dark Masters. Attribute: Virus.
//!
//! # Card text (card image + judge-quiz PDF p.5 + cards.json — agree)
//! `[Hand][Main] If you don't have any Digimon other than Digimon with`
//! `  [Dark Masters] in their texts, you may play this card with the play`
//! `  cost reduced by 5. At turn end, delete the Digimon this effect played.`
//! `[On Play][When Attacking] Return 1 of your opponent's suspended Digimon`
//! `  to the bottom of the deck.`
//! `[All Turns] This Digimon can only digivolve into [Apocalymon].`
//! `[On Deletion] If you have no green face-up security cards, place this`
//! `  Digimon face up as the bottom security card.`
//! `Inherited [Security]: If this card was face-up, you may play 1 level 5`
//! `  or lower card with [Dark Masters] in its text from your hand or trash`
//! `  without paying the cost.`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX10/Green/EX10_020.cs
//!
//! # Verdict
//! PARTIAL — the [Hand][Main] reduced self-play
//! (G-DSL-HAND-MAIN-SELF-PLAY-REDUCED) and the [Security] was-face-up gate
//! (G-DSL-SECURITY-WAS-FACE-UP-GATE) are OMITTED pending DSL vocabulary; see
//! the YAML header and qa/dsl-vocab-gaps.md. The judge-quiz Q3 clauses
//! ([All Turns] restriction + the rest) are implemented and pinned here.

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX10-020")
}

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c
}

/// A Lv.7 card NAMED "Apocalymon" with a printed digivolve route from a
/// green Lv.6 (the only legal digivolve target the restriction allows).
fn apocalymon() -> CardData {
    let mut c = make_test_card("APOC", "Apocalymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(12000);
    c.evo_costs = vec![evo_green_lv6()];
    c
}

/// A Lv.7 NON-Apocalymon card with the same green Lv.6 digivolve route —
/// the restriction must block it.
fn megadramon() -> CardData {
    let mut c = make_test_card("MEGA", "Megadramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(7);
    c.dp = Some(12000);
    c.evo_costs = vec![evo_green_lv6()];
    c
}

/// Printed-style evo cost "from green Lv.6 / memory 5". The green color
/// index matches `action::mask::evo_color`'s cards.json convention.
fn evo_green_lv6() -> EvoCost {
    EvoCost {
        card_color: 3,
        level: 6,
        memory_cost: 5,
    }
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn ex10_020_metadata_is_green_lv6_11000() {
    let card = compiled();
    assert_eq!(card.card, "EX10-020");
    assert_eq!(card.name, "Puppetmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(11000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Green));
}

// ─── SECTION 1b — NO digivolve routes INTO Puppetmon ────────────────────────
// Official Bandai DB (data/card_bundles/EX10-020.md) and the card image agree:
// EX10-020 prints NO standard digivolve circles at all (only "Play 11" and the
// [Hand][Main] reduced-cost self-play). Nothing may digivolve into it — an
// authored digivolve alt_path here would INVENT a circle the card doesn't have
// (the BT21-015/017 cheaper/wider-than-printed bug class).

#[test]
fn ex10_020_prints_no_digivolve_circles_nothing_digivolves_into_it() {
    // Structural: the compiled card must carry no digivolve alt paths.
    let card = compiled();
    assert!(
        !card.alt_paths.iter().any(|ap| matches!(
            ap.kind,
            digimon_dsl::compiled::CompiledAltPathKind::Digivolve
        )),
        "EX10-020 prints NO digivolve circles (official DB + card image) — a digivolve \
         alt_path invents a route the card does not have; got {:?}",
        card.alt_paths
    );

    // Behavioral: a green Lv.5 base (the invented route's `from` gate) must NOT
    // be able to digivolve into Puppetmon.
    let mut green_lv5 = make_test_card("GREEN-LV5", "GreenLv5");
    green_lv5.card_kind = CardKind::Digimon;
    green_lv5.level = Some(5);
    green_lv5.dp = Some(6000);
    green_lv5.colors = vec![CardColor::Green];
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(green_lv5)
        .hand(0, &["EX10-020"])
        .memory(15)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "GREEN-LV5", Some(0)).index as usize;

    let mem_before = r.game.memory;
    let proceeded = r
        .game
        .digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !proceeded,
        "EX10-020 has no digivolve requirements — digivolve_from_hand must reject"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "no cost-choice or other selection may park for a route that doesn't exist"
    );
    assert_eq!(r.game.memory, mem_before, "no memory paid");
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        "GREEN-LV5",
        "the base is unchanged — Puppetmon can only be PLAYED, never digivolved into"
    );
}

// ─── SECTION 2 — [On Play][When Attacking] bottom-deck a suspended opp ──────

#[test]
fn ex10_020_on_play_returns_suspended_opp_digimon_to_deck_bottom() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(opp_digimon("OPP-SUS"))
        .add_card(opp_digimon("OPP-UP"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();
    let puppet = r.place_on_field(0, "EX10-020", Some(0));
    let sus = r.place_on_field(1, "OPP-SUS", Some(0));
    let _up = r.place_on_field(1, "OPP-UP", Some(0));
    r.game.players[1].battle_area[sus.index as usize].is_suspended = true;
    let deck_before = r.deck_size(1);

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(puppet));
    r.game.drain_effect_queue();

    // Only the SUSPENDED opponent Digimon is a legal pick (mandatory select
    // resolves it; the unsuspended one is not a candidate).
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(1),
        1,
        "the suspended opponent Digimon was returned to the deck"
    );
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "OPP-UP",
        "the UNSUSPENDED Digimon stays"
    );
    assert_eq!(r.deck_size(1), deck_before + 1, "returned to the deck");
    assert_eq!(
        r.game.players[1].deck[0].card_id(&r.game.card_data),
        "OPP-SUS",
        "placed at the BOTTOM of the deck"
    );
}

#[test]
fn ex10_020_on_play_no_suspended_target_no_prompt() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(opp_digimon("OPP-UP"))
        .memory(10)
        .start();
    let puppet = r.place_on_field(0, "EX10-020", Some(0));
    let _up = r.place_on_field(1, "OPP-UP", Some(0));

    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(puppet));
    r.game.drain_effect_queue();
    assert!(
        r.game.pending_selection.is_none(),
        "no suspended opponent Digimon → the mandatory pick has no candidates → no prompt"
    );
    assert_eq!(r.battle_area_size(1), 1);
}

// ─── SECTION 3 — [All Turns] can only digivolve into [Apocalymon] ───────────

/// Battle area: the restriction is ACTIVE — a non-Apocalymon Lv.7 with an
/// otherwise-valid route is blocked; an "Apocalymon"-named one digivolves.
#[test]
fn ex10_020_battle_area_restriction_blocks_non_apocalymon_digivolve() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(apocalymon())
        .add_card(megadramon())
        .hand(0, &["MEGA", "APOC"])
        .memory(20)
        .start();
    r.skip_mulligan();
    let puppet = r.place_on_field(0, "EX10-020", Some(0));
    r.game.tick_declarative_effects(); // install the restriction aura
    r.set_phase(GamePhase::Main);

    // Megadramon (hand 0): valid green Lv.6 route, but the name is not
    // [Apocalymon] → BLOCKED.
    let blocked = r
        .game
        .digivolve_from_hand(0, 0, puppet.index as usize, PlaySource::ByHand);
    assert!(
        !blocked,
        "the [All Turns] restriction must block a non-[Apocalymon] digivolve \
         while Puppetmon is in the BATTLE AREA"
    );
    assert_eq!(
        r.game.players[0].battle_area[puppet.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "EX10-020",
        "Puppetmon unchanged"
    );

    // Apocalymon (now hand 0 after no removal — MEGA still in hand at 0;
    // APOC is at hand index 1).
    let allowed = r
        .game
        .digivolve_from_hand(0, 1, puppet.index as usize, PlaySource::ByHand);
    assert!(allowed, "[Apocalymon] is the allowed digivolve target");
    assert_eq!(
        r.game.players[0].battle_area[puppet.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "APOC",
        "Puppetmon digivolved into Apocalymon"
    );
}

// ─── SECTION 4 — [On Deletion] self to bottom security (no green face-up) ──

#[test]
fn ex10_020_on_deletion_places_self_face_up_as_bottom_security() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(make_test_card("SEC", "Sec"))
        .security(0, &["SEC", "SEC"])
        .memory(10)
        .start();
    let puppet = r.place_on_field(0, "EX10-020", Some(0));
    let sec_before = r.security_count(0);

    r.game.delete_permanent_with_cause(
        puppet,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(0),
        sec_before + 1,
        "Puppetmon was placed into security ([On Deletion], no green face-up)"
    );
    assert_eq!(
        r.game.players[0].security[0].card_id(&r.game.card_data),
        "EX10-020",
        "placed at the BOTTOM of the security stack"
    );
    assert_eq!(r.trash_size(0), 0, "the card left the trash for security");
}

#[test]
fn ex10_020_on_deletion_skipped_when_green_face_up_security_present() {
    // A GREEN face-up security card gates the placement OFF.
    let mut green_sec = make_test_card("GREENSEC", "Green Sec");
    green_sec.colors = vec![CardColor::Green];
    let mut r = DebugRunner::builder()
        .dsl_card("EX10-020")
        .expect("EX10-020 in pack")
        .add_card(green_sec)
        .security(0, &["GREENSEC"])
        .memory(10)
        .start();
    // Mark the security card face-up (the engine tracks face-up security by
    // card_index).
    let key = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(key);

    let puppet = r.place_on_field(0, "EX10-020", Some(0));
    let sec_before = r.security_count(0);

    r.game.delete_permanent_with_cause(
        puppet,
        digimon_engine::replacement::ReplacementCause::OpponentEffect,
    );
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(0),
        sec_before,
        "a GREEN face-up security card means the placement does NOT happen"
    );
    assert_eq!(r.trash_size(0), 1, "Puppetmon stays in the trash");
}
