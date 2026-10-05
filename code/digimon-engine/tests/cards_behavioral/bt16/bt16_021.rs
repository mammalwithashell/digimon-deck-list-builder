//! BT16-021 Togemogumon — Digimon, Lv.4, Blue/Green, Cost 5, DP 5000.
//! Form: Armor Form. Traits: Mammal. Attribute: Free.
//!
//! # Official text (data/card_bundles/BT16-021.md)
//!
//! ```text
//! Digivolve: Blue Lv.3 / cost 3; Green Lv.3 / cost 3
//! [Digivolve] [Wormmon]: Cost 2
//! <Blocker> <Armor Purge> [All Turns] [Once Per Turn] When an opponent's
//! Digimon becomes suspended, trash the top digivolution card of 1 of their
//! Digimon. Then, 1 of their Digimon with no digivolution cards can't attack or
//! block until the end of their turn.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_021.cs
//!
//! # Patterns
//! - static keywords: <Blocker>, <Armor Purge> (replacement)
//! - alt-digivolve by name (`name_contains: Wormmon`, cost 2)
//! - [All Turns][OPT] on_suspend observer gated on opponent Digimon
//! - mandatory opp-permanent pick → trash top source; then gated mandatory pick
//!   (no-sources filter evaluated after step 1) → CannotAttack + CannotBlock
//!   until end of opponent's turn
//! - OPT lockout + reset next turn; fires on both players' turns

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

use super::super::dsl_card_data::compiled;

const CARD_ID: &str = "BT16-021";

fn dig(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c.colors = vec![CardColor::Red];
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-021 in DSL pack")
        .add_card(dig("OPP", "Opp", CardColor::Red, 4, 5000))
        .add_card(dig("SRC", "Src", CardColor::Red, 3, 3000))
        .add_card(dig("SRC2", "Src Two", CardColor::Red, 3, 3000))
        .add_card(dig("OWN", "Own", CardColor::Blue, 4, 5000))
        .add_card(dig("WORM", "Wormmon", CardColor::Red, 3, 2000))
        .add_card(dig("BLUE3", "Blue Rookie", CardColor::Blue, 3, 2000))
        .add_card(dig("GREEN3", "Green Rookie", CardColor::Green, 3, 2000))
        .add_card(dig("YELLOW3", "Yellow Rookie", CardColor::Yellow, 3, 2000))
        .add_card(tamer("OPP-T"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(0, &["FILLER", "FILLER"])
        .security(1, &["FILLER", "FILLER"])
        .memory(5)
}

fn runner() -> DebugRunner {
    let mut r = builder().start();
    r.set_first_player(0);
    r
}

fn opp_pick(h: PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, h.index as u16)
}

fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("selection pending");
    let a = opp_pick(h);
    assert!(
        v.valid_action_ids.contains(&a),
        "{h:?} not a legal pick: {:?}",
        v.valid_action_ids
    );
    r.execute_action(v.selecting_player, a).unwrap();
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let cards = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    cards[..cards.len() - 1]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn locked(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.modifiers().has(h, ModifierType::CannotAttack) && r.modifiers().has(h, ModifierType::CannotBlock)
}

fn suspend_clause() -> CompiledTriggeredClause {
    compiled(CARD_ID)
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => {
                Some(t.clone())
            }
            _ => None,
        })
        .expect("on_suspend clause")
}

// ─── Structural ──────────────────────────────────────────────────────────────

#[test]
fn bt16_021_identity() {
    let c = compiled(CARD_ID);
    assert_eq!(c.level, Some(4));
    assert_eq!(c.cost, Some(5));
    assert_eq!(c.dp, Some(5000));
    assert!(c.color.contains(&CompiledColor::Blue));
    assert!(c.color.contains(&CompiledColor::Green));
    assert!(c.traits.iter().any(|t| t == "Mammal"));
    assert_eq!(c.form.as_deref(), Some("Armor Form"));
}

#[test]
fn bt16_021_grants_blocker_and_armor_purge() {
    let c = compiled(CARD_ID);
    let kws: Vec<String> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) => Some(format!("{keyword:?}")),
            _ => None,
        })
        .collect();
    assert!(kws.iter().any(|k| k.contains("Blocker")), "{kws:?}");
    assert!(kws.iter().any(|k| k.contains("ArmorPurge")), "{kws:?}");

    let mut r = runner();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(t, Keyword::Blocker));
}

#[test]
fn bt16_021_alt_paths_blue_green_wormmon() {
    let c = compiled(CARD_ID);
    let costs: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .map(|p| p.cost.clone())
        .collect();
    assert_eq!(costs.len(), 3, "blue Lv3, green Lv3, [Wormmon]");
    assert_eq!(
        costs
            .iter()
            .filter(|c| **c == Some(CompiledCost::Literal(3)))
            .count(),
        2
    );
    assert!(costs.contains(&Some(CompiledCost::Literal(2))));
}

#[test]
fn bt16_021_suspend_clause_is_all_turns_opt_mandatory() {
    let t = suspend_clause();
    assert!(t.once_per_turn);
    assert!(!t.optional, "no 'you may' — the effect is mandatory");
}

// ─── Digivolution ────────────────────────────────────────────────────────────

fn digivolve_from(base: &str) -> (bool, i16) {
    let mut r = runner();
    r.game.turn_count = 1;
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, before - r.memory())
}

#[test]
fn bt16_021_digivolves_from_wormmon_for_2() {
    let (ok, paid) = digivolve_from("WORM");
    assert!(ok, "a (red) [Wormmon] qualifies via the name alt-path");
    assert_eq!(paid, 2);
}

#[test]
fn bt16_021_digivolves_from_blue_or_green_lv3_for_3() {
    for base in ["BLUE3", "GREEN3"] {
        let (ok, paid) = digivolve_from(base);
        assert!(ok, "{base}");
        assert_eq!(paid, 3, "{base}");
    }
}

#[test]
fn bt16_021_cannot_digivolve_from_yellow_lv3() {
    let (ok, _) = digivolve_from("YELLOW3");
    assert!(!ok, "no yellow circle and not named Wormmon");
}

// ─── Armor Purge ─────────────────────────────────────────────────────────────

#[test]
fn bt16_021_armor_purge_prevents_deletion() {
    let mut r = runner();
    let t = r.place_stack(0, &["WORM", CARD_ID]);
    r.game
        .delete_permanent_with_cause(t, ReplacementCause::OpponentEffect);
    let v = r.pending_selection_view().expect("Armor Purge prompt");
    assert_eq!(v.kind, SelectionKind::Replacement);
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[t.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "WORM"
    );
}

// ─── [All Turns][OPT] trigger — your turn ────────────────────────────────────

#[test]
fn bt16_021_your_turn_opp_suspend_strips_then_locks() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_stack(1, &["SRC", "SRC2", "OPP"]); // 2 sources, top source = SRC2
    let b = r.place_on_field(1, "OPP", Some(0)); // no sources
    r.game.suspend(a);

    let v = r.pending_selection_view().expect("strip pick");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional, "the strip pick is mandatory");
    assert!(v.valid_action_ids.contains(&opp_pick(a)));
    assert!(
        v.valid_action_ids.contains(&opp_pick(b)),
        "DCGO: any of their Digimon may be chosen (no sources required)"
    );
    pick(&mut r, a);
    assert_eq!(source_ids(&r, a), vec!["SRC".to_string()], "top source SRC2 trashed");
    assert!(r.game.players[1].trash.iter().any(|c| c.card_id(&r.game.card_data) == "SRC2"));

    let v = r.pending_selection_view().expect("lock pick");
    assert!(!v.is_optional);
    assert!(v.valid_action_ids.contains(&opp_pick(b)));
    assert!(
        !v.valid_action_ids.contains(&opp_pick(a)),
        "A still has a digivolution card"
    );
    pick(&mut r, b);
    r.auto_resolve().unwrap();
    assert!(locked(&r, b));
    assert!(!locked(&r, a));
}

#[test]
fn bt16_021_stripped_target_becomes_lock_eligible() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_stack(1, &["SRC", "OPP"]); // exactly 1 source
    r.game.suspend(a);
    pick(&mut r, a);
    assert!(source_ids(&r, a).is_empty());
    let v = r.pending_selection_view().expect("lock pick after strip");
    assert!(
        v.valid_action_ids.contains(&opp_pick(a)),
        "the no-sources filter is evaluated after the strip"
    );
    pick(&mut r, a);
    r.auto_resolve().unwrap();
    assert!(locked(&r, a));
}

#[test]
fn bt16_021_no_lock_target_resolves_after_strip() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_stack(1, &["SRC", "SRC2", "OPP"]);
    r.game.suspend(a);
    pick(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, a).len(), 1);
    assert!(!locked(&r, a), "no Digimon without sources → no lock");
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt16_021_lock_persists_through_opponents_turn_then_expires() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let b = r.place_on_field(1, "OPP", Some(0));
    r.game.suspend(b);
    pick(&mut r, b); // strip (no-op: no sources)
    pick(&mut r, b); // lock
    r.auto_resolve().unwrap();
    assert!(locked(&r, b));
    r.end_turn(); // → opponent's turn
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    assert!(locked(&r, b), "still locked during their turn");
    r.end_turn(); // end of their turn
    let _ = r.auto_resolve();
    assert!(!locked(&r, b), "expires at the end of their turn");
}

// ─── Negative gates ──────────────────────────────────────────────────────────

#[test]
fn bt16_021_own_digimon_suspend_does_not_trigger() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let own = r.place_on_field(0, "OWN", Some(0));
    let opp = r.place_stack(1, &["SRC", "OPP"]);
    r.game.suspend(own);
    assert!(r.pending_selection().is_none());
    assert_eq!(source_ids(&r, opp).len(), 1);
}

#[test]
fn bt16_021_opponent_tamer_suspend_does_not_trigger() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let t = r.place_on_field(1, "OPP-T", Some(0));
    let opp = r.place_stack(1, &["SRC", "OPP"]);
    r.game.suspend(t);
    assert!(r.pending_selection().is_none(), "a Tamer is not a Digimon");
    assert_eq!(source_ids(&r, opp).len(), 1);
}

#[test]
fn bt16_021_not_on_field_does_not_trigger() {
    let mut r = runner();
    r.add_to_hand(0, CARD_ID);
    let opp = r.place_stack(1, &["SRC", "OPP"]);
    r.game.suspend(opp);
    assert!(r.pending_selection().is_none());
}

// ─── OPT ─────────────────────────────────────────────────────────────────────

#[test]
fn bt16_021_once_per_turn_lockout_and_reset() {
    let mut r = runner();
    r.place_on_field(0, CARD_ID, Some(0));
    let a = r.place_stack(1, &["SRC", "SRC2", "OPP"]);
    let c = r.place_stack(1, &["SRC", "SRC2", "OPP"]);
    r.game.suspend(a);
    pick(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, a).len(), 1);

    r.game.suspend(c);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: second suspend in the same turn does not trigger"
    );
    assert_eq!(source_ids(&r, c).len(), 2);

    // Next turn (opponent's): the OPT resets. Their turn-start unsuspends.
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    r.game.suspend(c);
    let v = r.pending_selection_view().expect("re-armed on the next turn");
    assert_eq!(v.selecting_player, 0, "Togemogumon's owner picks");
    pick(&mut r, c);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, c).len(), 1);
}

// ─── [All Turns] — opponent's turn via a real attack ─────────────────────────

#[test]
fn bt16_021_opponents_attack_triggers_and_lock_ends_with_their_turn() {
    let mut r = runner();
    r.skip_mulligan();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    let attacker = r.place_field_stack(1, &["OPP"], false, 0);
    let other = r.place_field_stack(1, &["OPP"], false, 0);
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    // Give the opponent memory so their attack does not auto-pass the turn.
    r.game.set_memory(5);

    r.attack_player(attacker, 0, false);
    // Attack declaration suspends the attacker → Togemogumon triggers.
    let v = r.pending_selection_view().expect("strip pick on their attack");
    assert_eq!(v.selecting_player, 0);
    pick(&mut r, attacker); // no sources → no-op strip
    let v = r.pending_selection_view().expect("lock pick");
    assert!(v.valid_action_ids.contains(&opp_pick(other)));
    pick(&mut r, other);
    assert!(locked(&r, other));
    // Decline/finish whatever remains (block timing, security check).
    while let Some(v) = r.pending_selection_view() {
        let a = if v.is_optional {
            digimon_engine::action::space::PASS
        } else {
            v.valid_action_ids[0]
        };
        r.execute_action(v.selecting_player, a).unwrap();
    }
    assert_eq!(r.turn_player(), 1, "still the opponent's turn");
    assert!(locked(&r, other), "lasts for the rest of their turn");
    r.end_turn();
    let _ = r.auto_resolve();
    assert!(
        !locked(&r, other),
        "triggered on their turn → expires at the end of that same turn"
    );
}
