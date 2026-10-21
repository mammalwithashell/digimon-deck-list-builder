//! BT24-074 SkullSeadramon — Digimon, Lv.5, Purple/Blue, DP 7000, cost 7.
//! Traits: Undead / Titan / TS (+ [Aquatic] by Rule). Form: Ultimate.
//! Attribute: Virus.
//!
//! Printed text (official Bandai DB — data/card_bundles/BT24-074.md; card
//! image agrees):
//!   Digivolve: Purple Lv.4 / cost 4; Blue Lv.4 / cost 4;
//!     [Digivolve] Lv.4 w/[Aqua] or [Sea Animal] in any trait or w/[TS]
//!     trait: Cost 3.
//!   [On Play] [When Digivolving] Trash any 3 digivolution cards from 1 of your
//!     opponent's Digimon. Then, if played by effects, delete 1 of your
//!     opponent's Digimon with no digivolution cards.
//!   [On Deletion] You may play 1 level 4 or lower Digimon card with
//!     [Seadramon] in its name or the [TS] trait from your trash without paying
//!     the cost.
//!   (Rule) Trait: Has [Aquatic] Type.
//!   Inherited: [When Attacking] [Once Per Turn] By placing 1 of your other
//!     Digimon as this Digimon's bottom digivolution card, it unsuspends.
//!   Official Q&A: the [On Deletion] target is a level 4 or lower Digimon card
//!     with [Seadramon] in its name OR a level 4 or lower [TS] Digimon card.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_074.cs
//!
//! Pattern rows: alt-digivolve by trait substring, shared OP/WD source strip
//! (BT20-027 idiom) with an OnPlay-only "if played by effects" tail, optional
//! [On Deletion] free play from the trash, inherited OPT place-permanent-as-
//! source → unsuspend (BT24-059 idiom).

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use support::*;

const CARD_ID: &str = "BT24-074";

/// Test-only [On Play] "play 1 Digimon card from your trash without paying
/// the cost" — an EFFECT play.
const EFFECT_PLAYER_YAML: &str = r#"
card: T-EFFPLAY
name: Effect Player
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Play 1 Digimon card from your trash without paying the cost"
    process:
      - select_trash:
          of: you
          bind_as: back
          filter: { kind: digimon }
          prompt: "Play 1 Digimon card from your trash without paying the cost"
      - play_from_trash_free: { of: you, hand_index: back }
"#;

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-074 in embedded DSL pack")
        .from_dsl_yaml(EFFECT_PLAYER_YAML)
        .expect("effect player")
        .add_card(filler("FILLER"))
        .add_card(filler("SRC"))
        .add_card(digimon("PURPLE4", "Purple Four", CardColor::Purple, 4, 5, &[]))
        .add_card(digimon("BLUE4", "Blue Four", CardColor::Blue, 4, 5, &[]))
        .add_card(digimon("RED4SEA", "Red Sea", CardColor::Red, 4, 5, &["Sea Animal"]))
        .add_card(digimon("RED4AQ", "Red Aqua", CardColor::Red, 4, 5, &["Aquabeast"]))
        .add_card(digimon("RED4TS", "Red TS", CardColor::Red, 4, 5, &["TS"]))
        .add_card(digimon("PURPLE4TS", "Purple TS", CardColor::Purple, 4, 5, &["TS"]))
        .add_card(digimon("RED4", "Red Four", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("RED3TS", "Red Three TS", CardColor::Red, 3, 4, &["TS"]))
        .add_card(digimon("SEADRA4", "MegaSeadramon Jr", CardColor::Blue, 4, 5, &[]))
        .add_card(digimon("TS3", "TS Three", CardColor::Yellow, 3, 4, &["TS"]))
        .add_card(digimon("TS5", "TS Five", CardColor::Yellow, 5, 7, &["TS"]))
        .add_card(digimon("PLAIN4", "Plain Four", CardColor::Yellow, 4, 5, &[]))
        .add_card(digimon("ALLY", "Ally", CardColor::Purple, 3, 3, &[]))
        .add_card(digimon("OPPTOP", "Opp Top", CardColor::Red, 5, 7, &[]))
        .add_card(digimon("OPPBARE", "Opp Bare", CardColor::Red, 3, 3, &[]))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .security(1, &["FILLER"; 3])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not on player {p}'s field"));
    PermanentHandle { player: p, index: index as u8 }
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) -> bool {
    let idx = r.add_to_hand(0, CARD_ID);
    r.game.digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByDigivolve)
}

/// Opponent Digimon OPPTOP carrying `n` SRC digivolution cards.
fn opp_with_sources(r: &mut DebugRunner, n: usize) -> PermanentHandle {
    let mut ids: Vec<&str> = vec!["SRC"; n];
    ids.push("OPPTOP");
    r.place_stack(1, &ids)
}

/// Answer the source-trash multi-pick by taking the first legal card until
/// the prompt closes (the picks are mandatory: "trash any 3").
fn take_sources(r: &mut DebugRunner) {
    use digimon_engine::selection::SelectionKind;
    let mut picks = 0;
    while let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::OppField | SelectionKind::OwnField) {
            break; // the follow-up delete pick
        }
        assert!(!v.is_optional, "\"Trash any 3\" is mandatory ({:?})", v.kind);
        let a = *v.valid_action_ids.first().expect("a source pick");
        r.execute_action(v.selecting_player, a).unwrap();
        picks += 1;
        assert!(picks <= 3, "at most 3 digivolution cards are trashed");
    }
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt24_074_metadata_and_clause_shape() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "SkullSeadramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.color.len(), 2, "Purple/Blue");
    for t in ["Undead", "Titan", "TS", "Aquatic"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t}");
    }
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 3);
    let opwd = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("OP/WD");
    assert_eq!(opwd.when, vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]);
    assert!(!opwd.optional && !opwd.once_per_turn);
    let del = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnDeletion))
        .expect("On Deletion");
    assert_eq!(del.scope, CompiledScope::FaceUp);
    let inh = trig
        .iter()
        .find(|t| t.scope == CompiledScope::Inherited)
        .expect("inherited");
    assert_eq!(inh.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh.once_per_turn);
    assert!(inh.optional, "\"By placing …\" is an optional cost");
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

#[test]
fn bt24_074_colour_circles_cost_4() {
    for base_id in ["PURPLE4", "BLUE4"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(top_id(&r, base), CARD_ID);
        assert_eq!(r.memory(), 10 - 4, "{base_id}");
    }
}

#[test]
fn bt24_074_alt_digivolve_aqua_sea_animal_or_ts_costs_3() {
    for base_id in ["RED4SEA", "RED4AQ", "RED4TS"] {
        let mut r = setup();
        let base = r.place_on_field(0, base_id, Some(0));
        assert!(digivolve_onto(&mut r, base), "{base_id}");
        assert_eq!(r.memory(), 10 - 3, "{base_id}");
    }
}

#[test]
fn bt24_074_alt_route_needs_level_4() {
    let mut r = setup();
    let base = r.place_on_field(0, "RED3TS", Some(0));
    assert!(!digivolve_onto(&mut r, base));
}

#[test]
fn bt24_074_cannot_digivolve_from_unrelated_red_lv4() {
    let mut r = setup();
    let base = r.place_on_field(0, "RED4", Some(0));
    assert!(!digivolve_onto(&mut r, base));
}

// ─── Section 3 — [On Play][When Digivolving] ─────────────────────────────────

#[test]
fn bt24_074_on_play_from_hand_trashes_3_sources_but_no_delete() {
    let mut r = setup();
    let host = opp_with_sources(&mut r, 4);
    r.place_on_field(1, "OPPBARE", Some(0));
    let idx = r.add_to_hand(0, CARD_ID);
    r.play(0, idx).expect("plays");
    let v = r.pending_selection_view().expect("host pick");
    assert!(!v.is_optional, "mandatory");
    assert_eq!(non_pass(&r).len(), 1, "only the opponent Digimon WITH sources");
    pick_side_field(&mut r, 0, host);
    take_sources(&mut r);
    drain_first(&mut r);
    assert_eq!(sources(&r, host), 1, "4 - 3");
    assert_eq!(trash_ids(&r, 1).iter().filter(|x| *x == "SRC").count(), 3);
    assert!(
        field_ids(&r, 1).contains(&"OPPBARE".to_string()),
        "played from the hand, not by an effect: no delete"
    );
}

#[test]
fn bt24_074_played_by_effect_also_deletes_a_sourceless_digimon() {
    let mut r = setup();
    let host = opp_with_sources(&mut r, 2);
    let bare = r.place_on_field(1, "OPPBARE", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "T-EFFPLAY", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_trash(&mut r, 0, CARD_ID);
    pick_side_field(&mut r, 0, host);
    take_sources(&mut r);
    assert_eq!(sources(&r, host), 0, "fewer than 3: all of them");
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional, "mandatory delete");
    assert_eq!(non_pass(&r).len(), 2, "OPPBARE and the just-stripped host");
    pick_side_field(&mut r, 0, bare);
    drain_first(&mut r);
    assert_eq!(field_ids(&r, 1), vec!["OPPTOP".to_string()]);
}

#[test]
fn bt24_074_when_digivolving_trashes_sources_but_never_deletes() {
    let mut r = setup();
    let host = opp_with_sources(&mut r, 3);
    r.place_on_field(1, "OPPBARE", Some(0));
    let base = r.place_on_field(0, "PURPLE4", Some(0));
    assert!(digivolve_onto(&mut r, base));
    pick_side_field(&mut r, 0, host);
    take_sources(&mut r);
    drain_first(&mut r);
    assert_eq!(sources(&r, host), 0);
    assert!(
        field_ids(&r, 1).contains(&"OPPBARE".to_string()),
        "digivolving is not being played by an effect"
    );
}

#[test]
fn bt24_074_no_opponent_sources_skips_to_the_delete_when_played_by_effect() {
    let mut r = setup();
    r.place_on_field(1, "OPPBARE", Some(0));
    push_trash(&mut r, 0, CARD_ID);
    let t = r.place_on_field(0, "T-EFFPLAY", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_trash(&mut r, 0, CARD_ID);
    let v = r.pending_selection_view().expect("straight to the delete pick");
    assert_eq!(non_pass(&r).len(), 1);
    let _ = v;
    drain_first(&mut r);
    assert!(field_ids(&r, 1).is_empty());
}

// ─── Section 4 — [On Deletion] ───────────────────────────────────────────────

#[test]
fn bt24_074_on_deletion_may_play_seadramon_or_ts_lv4_or_lower_free() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    for id in ["SEADRA4", "TS3", "TS5", "PLAIN4"] {
        push_trash(&mut r, 0, id);
    }
    r.game.delete_permanent_with_effects(me);
    let v = r.pending_selection_view().expect("[On Deletion] pick");
    assert!(v.is_optional, "\"You may play\"");
    assert_eq!(non_pass(&r).len(), 2, "SEADRA4 and TS3 (not the Lv.5 TS, not the plain Lv.4)");
    pick_trash(&mut r, 0, "SEADRA4");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"SEADRA4".to_string()));
    assert_eq!(r.memory(), 10, "without paying the cost");
}

#[test]
fn bt24_074_on_deletion_ts_level_3_qualifies() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS3");
    r.game.delete_permanent_with_effects(me);
    pick_trash(&mut r, 0, "TS3");
    drain_first(&mut r);
    assert!(field_ids(&r, 0).contains(&"TS3".to_string()));
}

#[test]
fn bt24_074_on_deletion_decline_plays_nothing() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS3");
    r.game.delete_permanent_with_effects(me);
    pass(&mut r, 0);
    drain_first(&mut r);
    assert!(field_ids(&r, 0).is_empty());
    assert!(trash_ids(&r, 0).contains(&"TS3".to_string()));
}

// ─── Section 5 — Inherited [When Attacking][Once Per Turn] ───────────────────

#[test]
fn bt24_074_inherited_place_other_digimon_as_bottom_source_unsuspends() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "PURPLE4TS"]);
    let ally = r.place_stack(0, &["SRC", "ALLY"]);
    r.attack_player(carrier, 1, false);
    let v = r.pending_selection_view().expect("optional activation");
    assert!(v.is_optional, "\"By placing …\" — declinable");
    r.accept_optional_trigger().expect("accept");
    let v = r.pending_selection_view().expect("placement pick");
    assert!(!v.is_optional, "accepted: the placement is the cost");
    assert_eq!(non_pass(&r).len(), 1, "only the OTHER Digimon");
    pick_side_field(&mut r, 0, ally);
    let carrier = find(&r, 0, "PURPLE4TS");
    assert!(
        !r.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "it unsuspends"
    );
    assert_eq!(
        source_ids(&r, carrier).first().map(String::as_str),
        Some("ALLY"),
        "ALLY is now the bottom digivolution card"
    );
    assert!(
        trash_ids(&r, 0).contains(&"SRC".to_string()),
        "the cards under the placed Digimon go to the trash (CR 4.3 §4-7-8)"
    );
    assert_eq!(r.battle_area_size(0), 1);
}

#[test]
fn bt24_074_inherited_decline_stays_suspended() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "PURPLE4TS"]);
    r.place_on_field(0, "ALLY", Some(0));
    r.attack_player(carrier, 1, false);
    pass(&mut r, 0);
    let carrier = find(&r, 0, "PURPLE4TS");
    assert!(r.game.players[0].battle_area[carrier.index as usize].is_suspended);
    assert_eq!(r.battle_area_size(0), 2, "ALLY stays");
}

#[test]
fn bt24_074_inherited_no_other_digimon_no_prompt() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "PURPLE4TS"]);
    r.attack_player(carrier, 1, false);
    assert!(r.pending_selection_view().is_none());
    let _ = r.auto_resolve();
}

#[test]
fn bt24_074_inherited_once_per_turn() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "PURPLE4TS"]);
    let a1 = r.place_on_field(0, "ALLY", Some(0));
    r.place_on_field(0, "BLUE4", Some(0));
    r.attack_player(carrier, 1, false);
    r.accept_optional_trigger().expect("accept");
    pick_side_field(&mut r, 0, a1);
    let _ = r.auto_resolve();
    let carrier = find(&r, 0, "PURPLE4TS");
    assert!(!r.game.players[0].battle_area[carrier.index as usize].is_suspended);
    // Second attack the same turn: no second placement.
    r.attack_player(carrier, 1, false);
    assert!(r.pending_selection_view().is_none(), "[Once Per Turn] spent");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"BLUE4".to_string()));
}

#[test]
fn bt24_074_top_card_does_not_use_its_inherited_text() {
    let mut r = setup();
    let me = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "ALLY", Some(0));
    r.attack_player(me, 1, false);
    assert!(
        r.pending_selection_view().is_none(),
        "the inherited [When Attacking] only works from under another card"
    );
    let _ = r.auto_resolve();
}
