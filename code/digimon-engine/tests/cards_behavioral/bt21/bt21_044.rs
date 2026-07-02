//! BT21-044 RizeGreymon — Digimon, Lv.5, Yellow, Cost 7, DP 7000.
//! Traits: Cyborg. Attribute: Vaccine.
//!
//! # Card text (cards.json / BT21-044.json)
//!
//! ```text
//! [On Play] [When Digivolving] For the turn, 1 of your [Marcus Damon]s is also
//! treated as a 3000 DP Digimon, can't digivolve, and gains <Rush> (This Digimon
//! can attack the turn it comes into play.) and <Alliance> (When this Digimon
//! attacks, by suspending 1 of your other Digimon, add the suspended Digimon's DP
//! to this Digimon and it gains <Security A. +1> for the attack.) Then, 1 of your
//! Digimon may attack.
//! [All Turns] [Once Per Turn] When any of your yellow or red Tamers are deleted,
//! you may place 1 [Marcus Damon] from your trash as the top security card.
//! ```
//!
//! ```text
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When any of your yellow or red Tamers are deleted,
//! you may place 1 [Marcus Damon] from your trash as the top security card.
//! ```
//!
//! Alt-digivolve (xros_req): [Digivolve] [GeoGreymon]: Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Yellow/BT21_044.cs
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt21/BT21-044.yaml
//!
//! # Patterns this test covers
//! - G1-adjacent: TreatAsDigimon (Tamer treated as a 3000 DP Digimon for the turn)
//! - H1 Rush keyword grant (on a selected permanent, end_of_turn expiry)
//! - H10 Alliance keyword grant
//! - CannotDigivolve modifier grant
//! - G-MAY-ATTACK-NOW: "Then, 1 of your Digimon may attack"
//! - F5-adjacent: on_any_deletion observer (yellow/red Tamer) -> trash->security place
//! - E2/OPT: [All Turns][Once Per Turn] lockout on the deletion observer
//! - Inherited copy of the deletion observer (scope: inherited)
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | 1 (own) | [On Play][When Digivolving] | treat a Marcus Damon as 3000 DP Digimon + CannotDigivolve + Rush + Alliance for turn; then 1 Digimon may attack |
//! | 2 (own) | [All Turns][OPT] on deletion | when own yellow/red Tamer deleted, may place a Marcus Damon from trash as top security |
//! | 3 (inherited) | [All Turns][OPT] on deletion | identical body, inherited scope |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledPlayerRef, CompiledPredicate,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

use super::super::dsl_card_data::compiled;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A [Marcus Damon] Tamer of a chosen color. The card name MUST be exactly
/// "Marcus Damon" so `name_contains: "Marcus Damon"` filters and the
/// trash-place selection match it.
fn make_marcus(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// A generic Tamer of a chosen color whose name is NOT "Marcus Damon".
fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// A simple Lv.5 yellow Digimon filler (eligible digivolution base / attacker).
fn make_digimon(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// Standard runner: RizeGreymon in P0 hand, a few Marcus Damon copies + fillers
/// registered, generous memory.
fn rizegreymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-044")
        .expect("BT21-044 in embedded DSL pack")
        .add_card(make_marcus("MARCUS-Y", CardColor::Yellow))
        .add_card(make_marcus("MARCUS-R", CardColor::Red))
        .add_card(make_tamer("TAMER-G", CardColor::Green))
        .add_card(make_digimon("LV5-BASE", CardColor::Yellow))
        .add_card(make_digimon("DIGI-Y", CardColor::Yellow))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT21-044"])
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start()
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt21_044_compiles_from_dsl_pack() {
    let _ = compiled("BT21-044");
}

#[test]
fn bt21_044_is_yellow_lv5_cyborg_digimon() {
    let card = compiled("BT21-044");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(card.traits.iter().any(|t| t == "Cyborg"));
}

/// Alt-path: [Digivolve] [GeoGreymon]: Cost 3.
#[test]
fn bt21_044_has_geogreymon_alt_path_cost3() {
    let card = compiled("BT21-044");
    let geo = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        })
        .expect("must have a GeoGreymon cost-3 digivolve alt-path");
    assert_eq!(geo.cost, Some(CompiledCost::Literal(3)));
}

/// Clause 1 fires on both [On Play] and [When Digivolving].
#[test]
fn bt21_044_clause1_fires_on_play_and_when_digivolving() {
    let card = compiled("BT21-044");
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay) && t.scope == CompiledScope::FaceUp =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 1 (own On Play) must exist");
    assert!(clause1.when.contains(&CompiledTiming::OnPlay));
    assert!(clause1.when.contains(&CompiledTiming::WhenDigivolving));
}

/// The card ships exactly two OnAnyDeletion observers: one own (FaceUp) and one
/// inherited — matching the two DCGO ActivateClass shells (main + inherited).
#[test]
fn bt21_044_has_two_on_any_deletion_observers_one_inherited() {
    let card = compiled("BT21-044");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        deletion.len(),
        2,
        "expected 2 OnAnyDeletion observers (own + inherited), got {}",
        deletion.len()
    );
    assert_eq!(
        deletion
            .iter()
            .filter(|t| t.scope == CompiledScope::Inherited)
            .count(),
        1,
        "exactly one of the two deletion observers is inherited"
    );
    assert_eq!(
        deletion
            .iter()
            .filter(|t| t.scope == CompiledScope::FaceUp)
            .count(),
        1,
        "exactly one of the two deletion observers is own (FaceUp)"
    );
}

/// Both deletion observers are [Once Per Turn] and optional ("you may place").
#[test]
fn bt21_044_deletion_observers_are_opt_and_optional() {
    let card = compiled("BT21-044");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(deletion.len(), 2);
    for t in &deletion {
        assert!(t.once_per_turn, "deletion observer must be [Once Per Turn]");
        assert!(t.optional, "deletion observer is 'you may' -> optional");
    }
}

/// The deletion observers gate on the deleted object: own yellow/red Tamer.
fn predicate_mentions_event_target_kind(p: &CompiledPredicate) -> bool {
    p.event_target_kind.is_some()
        || p.all_of.iter().any(predicate_mentions_event_target_kind)
        || p.any_of.iter().any(predicate_mentions_event_target_kind)
}

fn predicate_mentions_event_target_color(p: &CompiledPredicate) -> bool {
    p.event_target_color_any_of.is_some()
        || p.all_of.iter().any(predicate_mentions_event_target_color)
        || p.any_of.iter().any(predicate_mentions_event_target_color)
}

fn predicate_mentions_event_target_owner_you(p: &CompiledPredicate) -> bool {
    p.event_target_owner == Some(CompiledPlayerRef::You)
        || p.all_of
            .iter()
            .any(predicate_mentions_event_target_owner_you)
        || p.any_of
            .iter()
            .any(predicate_mentions_event_target_owner_you)
}

#[test]
fn bt21_044_deletion_observers_gate_on_own_yellow_red_tamer() {
    let card = compiled("BT21-044");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(deletion.len(), 2);
    for t in &deletion {
        let cond = t
            .condition
            .as_ref()
            .expect("deletion observer must gate on the deleted object");
        assert!(
            predicate_mentions_event_target_owner_you(cond),
            "must require the deleted Tamer to be yours"
        );
        assert!(
            predicate_mentions_event_target_kind(cond),
            "must require the deleted object be a Tamer"
        );
        assert!(
            predicate_mentions_event_target_color(cond),
            "must require the deleted Tamer be yellow or red"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 [On Play] / [When Digivolving] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// Helper: play RizeGreymon from hand (On Play timing).
fn play_rize(runner: &mut DebugRunner) -> PermanentHandle {
    let idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| runner.game.card_data[c.data_index].card_id == "BT21-044")
        .expect("RizeGreymon in hand");
    let battle_idx = runner
        .play(0, idx)
        .expect("RizeGreymon plays onto the field");
    PermanentHandle {
        player: 0,
        index: battle_idx as u8,
    }
}

/// POSITIVE: when a Marcus Damon is on the field, playing RizeGreymon installs a
/// mandatory selection to pick the Marcus Damon (the first prompt in clause 1).
#[test]
fn bt21_044_on_play_installs_marcus_select_when_marcus_present() {
    let mut runner = rizegreymon_runner();
    let _marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));
    play_rize(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a selection must install: pick the Marcus Damon to treat as a Digimon");
    assert_eq!(pending.selecting_player, 0, "you select your Marcus Damon");
    assert!(
        !pending.valid_action_ids.is_empty(),
        "the Marcus Damon must be a valid target"
    );
}

/// POSITIVE: after picking the Marcus Damon, it is treated as a 3000 DP Digimon,
/// can't digivolve, and gains Rush + Alliance. All for the turn.
#[test]
fn bt21_044_on_play_treats_marcus_as_3000_digimon_with_rush_alliance() {
    let mut runner = rizegreymon_runner();
    let marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));

    // Before the effect: a Tamer has no DP, no Rush/Alliance, can digivolve.
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Rush),
        "Marcus must not start with Rush"
    );
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Alliance),
        "Marcus must not start with Alliance"
    );
    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must not start treated as a Digimon"
    );

    play_rize(&mut runner);

    // Pick the Marcus Damon (the first valid action).
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("Marcus selection resolves");
    let _ = runner.auto_resolve();

    assert!(
        runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "Marcus must be treated as a Digimon"
    );
    assert_eq!(
        runner.effective_dp(marcus),
        Some(3000),
        "Marcus must be treated as a 3000 DP Digimon"
    );
    assert!(
        runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "Marcus must gain CannotDigivolve"
    );
    assert!(
        runner.game.has_keyword(marcus, Keyword::Rush),
        "Marcus must gain Rush"
    );
    assert!(
        runner.game.has_keyword(marcus, Keyword::Alliance),
        "Marcus must gain Alliance"
    );
}

/// The treated-Marcus grants are end_of_turn — they all revert after the turn ends.
#[test]
fn bt21_044_on_play_marcus_grants_expire_at_end_of_turn() {
    let mut runner = rizegreymon_runner();
    let marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));
    play_rize(&mut runner);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    // Part (b) "1 of your Digimon may attack" is optional. Marcus is now treated
    // as a Digimon, so it (and RizeGreymon) are legal attacker candidates —
    // `kind: digimon` field selection now matches a TreatAsDigimon Tamer
    // (G-TOKEN-NOT-DIGIMON-FOR-FIELD-SELECT widened `kind_matches_field`).
    // Decline the optional attack so the turn stays alive and the end-of-turn
    // expiry step actually runs (auto-driving the attack wins the game and skips
    // it). We only care that the for-the-turn grants revert.
    while let Some(sel) = runner.game.pending_selection.as_ref() {
        let who = sel.selecting_player;
        assert!(
            sel.is_optional,
            "only the optional may-attack prompt should remain after the Marcus pick"
        );
        runner
            .game
            .resolve_selection(who, PASS)
            .expect("decline the optional may-attack prompt");
    }

    // Sanity: grants are present this turn.
    assert!(runner.modifiers().has(marcus, ModifierType::TreatAsDigimon));
    assert!(runner.game.has_keyword(marcus, Keyword::Rush));

    runner.game.end_turn(); // end P0's turn -> grants expire (end_of_turn)

    assert!(
        !runner.modifiers().has(marcus, ModifierType::TreatAsDigimon),
        "TreatAsDigimon must expire at end of turn"
    );
    assert!(
        !runner
            .modifiers()
            .has(marcus, ModifierType::CannotDigivolve),
        "CannotDigivolve must expire at end of turn"
    );
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Rush),
        "Rush must expire at end of turn"
    );
    assert!(
        !runner.game.has_keyword(marcus, Keyword::Alliance),
        "Alliance must expire at end of turn"
    );
}

/// NEGATIVE: with NO Marcus Damon on the field, the Marcus-treat selection is
/// skipped — but the "Then, 1 of your Digimon may attack" step still runs. With
/// no own Digimon eligible to attack (RizeGreymon was just played and has
/// summoning sickness), no Marcus prompt installs. Net: no TreatAsDigimon
/// modifier exists anywhere.
#[test]
fn bt21_044_on_play_no_marcus_grants_nothing() {
    let mut runner = rizegreymon_runner();
    let rize = play_rize(&mut runner);
    let _ = runner.auto_resolve();

    // RizeGreymon itself must not be treated-as / get the grants.
    assert!(
        !runner.modifiers().has(rize, ModifierType::TreatAsDigimon),
        "no Marcus on field -> nothing treated as a Digimon"
    );
}

/// "Then, 1 of your Digimon may attack" — after the grant step, an optional
/// may-attack-now selection over your Digimon installs when an attack-eligible
/// own Digimon exists. We place an already-summoned (turn 0) yellow Digimon so
/// it can attack, then RizeGreymon. After the Marcus grant resolves, the
/// may-attack selection must be reachable (optional => PASS legal).
#[test]
fn bt21_044_on_play_then_one_digimon_may_attack() {
    let mut runner = rizegreymon_runner();
    // An own Digimon that has been on the field since turn 0 (can attack).
    let _attacker = runner.place_on_field(0, "DIGI-Y", Some(0));
    let marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));
    play_rize(&mut runner);

    // 1) Resolve the mandatory Marcus pick.
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");

    // 2) The "1 of your Digimon may attack" optional selection must install.
    let kind = runner
        .pending_kind()
        .expect("a may-attack selection must install after the Marcus grant");
    assert_eq!(
        kind,
        SelectionKind::OwnField,
        "the may-attack step selects 1 of your own Digimon"
    );
    assert!(
        runner.pending_is_optional(),
        "'may attack' -> the attacker selection is optional (PASS legal)"
    );

    // Grants still applied regardless of the attack choice.
    assert!(runner.modifiers().has(marcus, ModifierType::TreatAsDigimon));
}

/// Clause 1 fires on [When Digivolving] too: RizeGreymon placed atop a Lv5 stack
/// then WhenDigivolving enqueued installs the Marcus-pick selection.
#[test]
fn bt21_044_clause1_fires_when_digivolving() {
    let mut runner = rizegreymon_runner();
    let _marcus = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let rize = runner.place_stack(0, &["LV5-BASE", "BT21-044"]);

    runner.game.enqueue_triggered(
        CompiledTimingShim::when_digivolving(),
        TriggerSource::Permanent(rize),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("WhenDigivolving must install the Marcus-pick selection");
    assert_eq!(pending.selecting_player, 0);
}

// Shim so the test reads naturally; maps to the engine EffectTiming enum.
struct CompiledTimingShim;
impl CompiledTimingShim {
    fn when_digivolving() -> digimon_engine::enums::EffectTiming {
        digimon_engine::enums::EffectTiming::WhenDigivolving
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2/3 [All Turns][OPT] deletion -> trash -> security
// ════════════════════════════════════════════════════════════════════════════

/// Setup: RizeGreymon on the field (own clause active), a yellow Marcus Damon in
/// trash, and a yellow Marcus Damon Tamer on the field to be deleted.
fn deletion_runner_with_rize_on_field() -> (DebugRunner, PermanentHandle) {
    let mut runner = rizegreymon_runner();
    // RizeGreymon on the battle area so its own [All Turns] observer is active.
    let _rize = runner.place_on_field(0, "BT21-044", Some(0));
    // A Marcus Damon sitting in the trash to be placed on security.
    // Use the engine helper to seed trash by deleting a placed Marcus is noisy;
    // instead push a Marcus Damon card source directly into trash.
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    // The deletable yellow Marcus Damon Tamer on the field.
    let tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    (runner, tamer)
}

/// Push a [Marcus Damon] card directly into a player's trash.
fn seed_trash_with_marcus(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("marcus card registered");
    let next_idx = runner.game.next_card_index();
    let source = digimon_engine::card_source::CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].trash.push(source);
}

/// POSITIVE: deleting an own yellow/red Tamer installs the optional trash-place
/// selection; accepting it places the Marcus Damon from trash onto the top of
/// security. Net: trash -1, security +1.
#[test]
fn bt21_044_deletion_places_marcus_from_trash_to_top_security() {
    let (mut runner, tamer) = deletion_runner_with_rize_on_field();

    let sec_before = runner.security_count(0);
    let trash_before = runner.trash_size(0);

    runner.game.delete_permanent_with_effects(tamer);

    // The optional trash-place selection must install.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("deletion of a yellow/red Tamer installs the trash-place selection");
    assert_eq!(pending.selecting_player, 0);
    // Accept: pick the Marcus Damon in trash.
    let action = pending.valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("trash selection resolves");
    let _ = runner.auto_resolve();

    // The deleted Tamer also lands in trash (+1), and one Marcus leaves trash for
    // security (-1), so trash net is unchanged from the just-deleted Tamer minus
    // the placed Marcus. Assert security gained exactly 1.
    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "a Marcus Damon must be placed as the top security card"
    );
    // The placed card is no longer in trash; the deleted Tamer entered trash.
    // trash_before counted only the seeded Marcus (1). After: deleted Tamer in,
    // Marcus out => still has the deleted Tamer, so >= trash_before.
    assert!(
        runner.trash_size(0) >= trash_before,
        "trash accounting: deleted Tamer in, placed Marcus out"
    );
}

/// The placed-on-top security card is the Marcus Damon (top of stack).
#[test]
fn bt21_044_placed_security_card_is_marcus_on_top() {
    let (mut runner, tamer) = deletion_runner_with_rize_on_field();
    runner.game.delete_permanent_with_effects(tamer);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();

    let top = &runner.game.players[0].security[0];
    let name = runner.game.card_data[top.data_index].card_name.clone();
    assert_eq!(
        name, "Marcus Damon",
        "top security card must be the Marcus Damon"
    );
}

/// OPTIONAL DECLINE: deleting an eligible Tamer offers the place, but PASS leaves
/// security and trash unchanged ("you may place").
#[test]
fn bt21_044_deletion_decline_leaves_security_unchanged() {
    use digimon_engine::action::space::PASS;
    let (mut runner, tamer) = deletion_runner_with_rize_on_field();

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(tamer);

    assert!(
        runner.pending_is_optional(),
        "the place is 'you may' -> PASS must be legal"
    );
    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before,
        "declining must not place anything on security"
    );
}

/// NEGATIVE: deleting a Tamer of a non-matching color (green) must NOT fire the
/// observer.
#[test]
fn bt21_044_deletion_of_green_tamer_does_not_fire() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT21-044", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    let green_tamer = runner.place_on_field(0, "TAMER-G", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(green_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "a green Tamer is neither yellow nor red -> observer must not fire"
    );
    assert_eq!(runner.security_count(0), sec_before);
}

/// NEGATIVE: deleting an OPPONENT'S yellow Tamer must NOT fire your observer.
#[test]
fn bt21_044_deletion_of_opponent_tamer_does_not_fire() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT21-044", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    let opp_tamer = runner.place_on_field(1, "MARCUS-Y", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(opp_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "opponent's Tamer deletion must not fire your observer"
    );
    assert_eq!(runner.security_count(0), sec_before);
}

/// OPT: a second eligible Tamer deletion in the same turn must NOT re-fire.
#[test]
fn bt21_044_deletion_observer_is_once_per_turn() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT21-044", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-R");
    let tamer1 = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let tamer2 = runner.place_on_field(0, "MARCUS-R", Some(0));

    // First deletion fires; accept the place.
    runner.game.delete_permanent_with_effects(tamer1);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("first resolves");
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(0);

    // Second deletion same turn must NOT install a selection (OPT lockout).
    runner.game.delete_permanent_with_effects(tamer2);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must block the second eligible Tamer deletion in the same turn"
    );
    assert_eq!(
        runner.security_count(0),
        sec_after_first,
        "no second placement under the OPT lockout"
    );
}

/// The OPT lockout clears next turn: after end_turn cycles back to P0, the
/// observer fires again on a fresh eligible deletion.
#[test]
fn bt21_044_deletion_opt_clears_next_turn() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT21-044", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-R");
    let tamer1 = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.delete_permanent_with_effects(tamer1);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("first resolves");
    let _ = runner.auto_resolve();

    // Cycle back to P0's turn.
    runner.game.end_turn();
    runner.game.end_turn();

    let tamer2 = runner.place_on_field(0, "MARCUS-R", Some(0));
    runner.game.delete_permanent_with_effects(tamer2);
    assert!(
        runner.game.pending_selection.is_some(),
        "OPT lockout must clear next turn — observer fires again"
    );
}
