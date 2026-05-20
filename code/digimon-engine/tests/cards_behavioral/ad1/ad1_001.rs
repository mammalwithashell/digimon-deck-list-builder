//! AD1-001 Greymon — Digimon, Lv.4, Red, DP 5000, Cost 5.
//! Traits: Dinosaur, ADVENTURE.
//!
//! # Card text (cards.json — printed)
//!
//! [On Play] [When Digivolving] You may return 1 card with [Greymon],
//! [Garurumon] or [Omnimon] in its name from your trash to the hand.
//!
//! [All Turns] When your Digimon or Tamers are played or digivolve, if any
//! of them have [Garurumon] or [Tai Kamiya] in their names, this Digimon
//! may digivolve into a Digimon card with [Greymon] in its name in the
//! hand without paying the cost.
//!
//! Inherited Effect:
//! <Raid> (When this Digimon attacks, you may switch the target of attack
//! to 1 of your opponent's unsuspended Digimon with the highest DP.)
//!
//! Alt-digivolve: source perm with top card containing "Omnimon" name OR
//! ADVENTURE trait, level 3, cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_001.cs
//!
//! # Patterns this test covers
//! - A4 Trash → hand recursion (OnPlay/WhenDigivolving — return Greymon/
//!   Garurumon/Omnimon-named card from trash to hand)
//! - E2 OPT + optional decline (the "you may" return clause)
//! - D-CROSS-PERM Cross-permanent observer → effect-initiated free digivolve
//! - H9 Raid (printed inherited keyword — declarative grant_keyword)
//!
//! # No printed [Once Per Turn] on this card. Section 5 (OPT lockout) is
//! intentionally skipped — there are no OPT clauses to enforce.
//!
//! # Known engine/DSL gaps affecting these tests
//!
//! - **G-DECLARATIVE-KEYWORD**: declarative grant_keyword clauses
//!   (`kind: grant_keyword`, here: inherited Raid) compile to
//!   `EffectTiming::Declarative` but are never enqueued/fired by the
//!   engine. Behavioral verification of the Raid-keyword being installed
//!   on the carrier is `#[ignore]`'d.
//! - **G-ON-DIGIVOLVE-DNA-EVENT-CARD**: `OnDigivolve` fired from
//!   `dna_digivolve_inner` uses `TriggerSource::PlayerBattleArea` and so
//!   loses `event_card`. The all-turns observer's
//!   `event_card_name_contains` predicate cannot match a DNA-digivolution
//!   into a Garurumon-named card. Normal (user-action) digivolve into a
//!   Garurumon-named card carries `event_card` and works correctly.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c
}

fn make_named_tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = digimon_engine::enums::CardKind::Tamer;
    c
}

/// Standard fixture: AD1-001 (Greymon) registered + a generic level-3
/// platform for digivolution tests + filler. Memory pre-set to 10 so the
/// card can be played without the seesaw mid-test.
fn greymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 found in embedded DSL pack")
        .add_card(make_named_digimon("LV3-PLAT", "Lv3Platform", 3, 3000))
        .add_card(make_named_digimon("FILL", "Filler", 4, 4000))
        .memory(12)
        .start()
}

/// Push a CardSource referencing `card_id` into player `p`'s trash.
/// Returns the new card's handle.
fn push_to_trash(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].trash.push(card);
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// AD1-001 must compile. Structural shape:
///   - At least 1 alt_path (Digivolve from level-3 perm with name_contains
///     "Omnimon" OR trait ADVENTURE, cost 2)
///   - 1 inherited GrantKeyword Raid declarative clause
///   - 1 triggered clause on [OnPlay, WhenDigivolving] (own-scope, optional,
///     non-OPT) — the trash → hand recursion
///   - 1 triggered clause on [OnEnterFieldAnyone, OnDigivolve] (own-scope,
///     optional, non-OPT) — the all-turns observer
#[test]
fn ad1_001_compiles_in_embedded_pack() {
    let runner = greymon_runner();
    assert!(
        runner.compiled_card("AD1-001").is_some(),
        "AD1-001 must be present in embedded DSL pack"
    );
}

/// Card-level metadata: level 4, DP 5000, cost 5, Red.
#[test]
fn ad1_001_card_metadata_matches_print() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    assert_eq!(card.level, Some(4), "Greymon is Lv.4");
    assert_eq!(card.dp, Some(5000), "Greymon is DP 5000");
    assert_eq!(card.cost, Some(5), "Greymon costs 5 to play");
}

/// Alt-digivolve: at least one Digivolve alt_path requires level_eq 3 and
/// either (name_contains "Omnimon" OR trait_has ADVENTURE), with cost 2.
#[test]
fn ad1_001_alt_digivolve_path_omnimon_or_adventure_cost_2() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    let alt = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(2))
        })
        .expect("AD1-001 must declare a Digivolve alt_path with cost 2");

    let from = alt
        .from
        .as_ref()
        .expect("alt-digivolve must declare a `from:` predicate");

    /// Recursively check whether a CompiledPredicate node (or any descendant)
    /// satisfies `f`. Walks all_of/any_of/none_of/not branches.
    fn pred_any<F: Fn(&digimon_dsl::compiled::CompiledPredicate) -> bool + Copy>(
        p: &digimon_dsl::compiled::CompiledPredicate,
        f: F,
    ) -> bool {
        if f(p) {
            return true;
        }
        if p.all_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if p.any_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if p.none_of.iter().any(|q| pred_any(q, f)) {
            return true;
        }
        if let Some(ref n) = p.not {
            if pred_any(n, f) {
                return true;
            }
        }
        false
    }

    let level_ok = pred_any(from, |p| p.level_eq == Some(3));
    assert!(
        level_ok,
        "alt-digivolve `from:` must require level_eq: 3 somewhere in the predicate"
    );

    let omnimon = pred_any(from, |p| p.name_contains.as_deref() == Some("Omnimon"));
    let adventure = pred_any(from, |p| {
        p.trait_has
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("ADVENTURE"))
            .unwrap_or(false)
    });
    assert!(
        omnimon,
        "alt-digivolve `from:` must allow name_contains: Omnimon"
    );
    assert!(
        adventure,
        "alt-digivolve `from:` must allow trait_has: ADVENTURE"
    );
}

/// Inherited Raid declarative clause must be present.
#[test]
fn ad1_001_has_inherited_raid_grant_keyword() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    let inherited_raid = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        inherited_raid,
        "AD1-001 must declare an inherited-scope GrantKeyword Raid clause"
    );
}

/// One triggered clause must fire on [OnPlay, WhenDigivolving], be optional,
/// face-up, and NOT once-per-turn.
#[test]
fn ad1_001_has_on_play_when_digivolving_optional_clause() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("AD1-001 must have a triggered clause covering OnPlay + WhenDigivolving");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "OnPlay/WhenDigivolving clause must have FaceUp scope"
    );
    assert!(
        clause.optional,
        "OnPlay/WhenDigivolving clause is optional (\"you may\")"
    );
    assert!(
        !clause.once_per_turn,
        "OnPlay/WhenDigivolving clause is NOT printed [Once Per Turn]"
    );
}

/// One triggered clause must fire on the all-turns observer (any of:
/// OnEnterFieldAnyone, OnDigivolve), be optional, face-up, NOT once-per-turn,
/// and (eventually) `active_when: { all_turns: true }`.
#[test]
fn ad1_001_has_all_turns_observer_clause() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnEnterFieldAnyone))
        .expect("AD1-001 must have an all-turns observer clause that includes OnEnterFieldAnyone");

    // Must also fire on the digivolve half.
    assert!(
        clause.when.contains(&CompiledTiming::OnDigivolve),
        "all-turns observer must also fire on OnDigivolve"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "all-turns observer is face-up scope"
    );
    assert!(
        clause.optional,
        "all-turns observer is optional (\"may digivolve\")"
    );
    assert!(
        !clause.once_per_turn,
        "all-turns observer is NOT printed [Once Per Turn]"
    );
    assert!(
        clause.active_when.is_some(),
        "all-turns observer must declare active_when (all_turns: true)"
    );
}

/// AD1-001 must have exactly two triggered clauses: the OnPlay/WhenDigivolving
/// trash recursion and the all-turns observer.
#[test]
fn ad1_001_has_exactly_two_triggered_clauses() {
    let runner = greymon_runner();
    let card = runner.compiled_card("AD1-001").expect("AD1-001 compiles");

    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 2,
        "AD1-001 must have exactly 2 triggered clauses (trash recursion + all-turns observer)"
    );
}

// ─── SECTION 2 — Condition gating (positive + negative) ──────────────────────

/// Negative gate (OnPlay): when player 0's trash holds NO Greymon/Garurumon/
/// Omnimon-named card, playing AD1-001 must not install a selection prompt.
#[test]
fn ad1_001_on_play_no_eligible_trash_card_no_prompt() {
    let mut runner = greymon_runner();
    push_to_hand(&mut runner, 0, "AD1-001");
    // Trash is empty; no candidate. Just to be safe, push a clearly-irrelevant
    // card that won't match any name predicate.
    push_to_trash(&mut runner, 0, "FILL");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-001")
        .expect("AD1-001 in hand");
    runner.play(0, hand_idx).expect("Greymon plays from hand");

    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash candidate => no select_trash prompt installed"
    );
}

/// Positive gate (OnPlay): when player 0's trash holds an eligible card
/// (named "Greymon"), playing AD1-001 must install a Trash selection
/// prompt that is optional (the "you may").
#[test]
fn ad1_001_on_play_with_eligible_trash_installs_optional_prompt() {
    let mut runner = greymon_runner();
    // Add an eligible card to the registry and push to player 0's trash.
    runner
        .game
        .card_data
        .push(make_named_digimon("TEST-GREYMON", "Greymon", 4, 4000));
    push_to_hand(&mut runner, 0, "AD1-001");
    push_to_trash(&mut runner, 0, "TEST-GREYMON");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-001")
        .expect("AD1-001 in hand");
    runner.play(0, hand_idx).expect("Greymon plays from hand");

    let pending = runner
        .pending_selection()
        .expect("trash → hand select prompt must install when an eligible trash card exists");
    assert!(
        pending.is_optional,
        "the OnPlay/WhenDigivolving trash recursion is optional (\"you may\")"
    );
}

/// Negative gate (all-turns observer): when no Garurumon/Tai Kamiya named
/// card is being played, playing an unrelated ally must NOT install the
/// observer's free-digivolve prompt.
#[test]
fn ad1_001_all_turns_observer_no_match_no_prompt() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY-IN-HAND", "Greymon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-IN-HAND");
    // Greymon (AD1-001) on field as the observer.
    let _ad1 = runner.place_on_field(0, "AD1-001", Some(0));
    // Unrelated ally enters.
    runner.game.card_data.push(make_named_digimon(
        "UNRELATED",
        "Unrelated Digimon",
        3,
        2000,
    ));
    push_to_hand(&mut runner, 0, "UNRELATED");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "UNRELATED")
        .expect("UNRELATED in hand");
    runner.play(0, hand_idx).expect("UNRELATED plays");

    assert!(
        runner.pending_selection().is_none(),
        "an unrelated-name ally entering must not trigger the observer"
    );
}

/// Positive gate (all-turns observer): when a "Garurumon"-named ally is
/// played, the observer's optional free-digivolve prompt must install
/// (because AD1-001 on field has a Greymon-named card in hand to digivolve
/// into).
#[test]
fn ad1_001_all_turns_observer_garurumon_play_installs_prompt() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY-IN-HAND", "Greymon", 4, 4000));
    runner
        .game
        .card_data
        .push(make_named_digimon("GARU-IN-HAND", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-IN-HAND");
    push_to_hand(&mut runner, 0, "GARU-IN-HAND");
    // AD1-001 on field as the observer (placed on prior turn so no fresh-play
    // OnPlay prompt interferes).
    let _ad1 = runner.place_on_field(0, "AD1-001", Some(0));

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "GARU-IN-HAND")
        .expect("GARU-IN-HAND in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");

    let pending = runner
        .pending_selection()
        .expect("observer prompt must install when a Garurumon-named ally enters");
    assert!(
        pending.is_optional,
        "observer prompt is optional (\"may digivolve\")"
    );
}

/// Negative gate (event ownership): the observer must NOT fire on an
/// opponent's "Garurumon"-named play. The card text scopes to "your Digimon
/// or Tamers".
#[test]
fn ad1_001_all_turns_observer_opponent_play_does_not_fire() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY-IN-HAND", "Greymon", 4, 4000));
    let mut opp_garu = make_named_digimon("OPP-GARU", "Garurumon", 4, 4000);
    opp_garu.play_cost = 0;
    runner.game.card_data.push(opp_garu);
    push_to_hand(&mut runner, 0, "GREY-IN-HAND");
    push_to_hand(&mut runner, 1, "OPP-GARU");
    let _ad1 = runner.place_on_field(0, "AD1-001", Some(0));

    // Switch turn so player 1 is active and can play. The runner starts with
    // memory=12 (above the +10 ceiling); end_turn flips to -12 (below the -10
    // floor). Setting OPP-GARU's play_cost to 0 sidesteps `pay_memory`'s
    // affordability check entirely.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let hand_idx = runner
        .game
        .player(1)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OPP-GARU")
        .expect("OPP-GARU in hand");
    runner
        .play(1, hand_idx)
        .expect("opponent's Garurumon plays");

    assert!(
        runner.pending_selection().is_none(),
        "opponent's Garurumon play must not fire the observer (\"your Digimon or Tamers\")"
    );
}

// ─── SECTION 3 — Behavioral outcome ──────────────────────────────────────────

/// OnPlay branch — accepting the optional and picking a Greymon-named card
/// from trash returns it to hand (trash shrinks by 1, hand grows by 1).
#[test]
fn ad1_001_on_play_accept_returns_greymon_from_trash_to_hand() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TEST-GREYMON", "Greymon", 4, 4000));
    push_to_hand(&mut runner, 0, "AD1-001");
    push_to_trash(&mut runner, 0, "TEST-GREYMON");

    let trash_before = runner.trash_size(0);
    let hand_before = runner.hand_size(0);

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-001")
        .expect("AD1-001 in hand");
    runner.play(0, hand_idx).expect("Greymon plays");
    // Hand: -1 for the played AD1-001, but the prompt is up.
    let hand_after_play = runner.hand_size(0);
    assert_eq!(
        hand_after_play,
        hand_before - 1,
        "hand decreased by 1 from playing AD1-001"
    );

    // Pick the only legal action — the Greymon in trash.
    let (player, action) = {
        let s = runner
            .pending_selection()
            .expect("trash select prompt installed");
        (s.selecting_player, s.valid_action_ids[0])
    };
    runner
        .execute_action(player, action)
        .expect("execute select_trash action");
    runner.auto_resolve().expect("auto-resolve any follow-ups");

    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "trash decreased by 1 (Greymon moved out)"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_after_play + 1,
        "hand grew by 1 (Greymon returned)"
    );
    // The returned card should be in hand by name.
    let has_grey = runner
        .game
        .player(0)
        .hand
        .iter()
        .any(|c| c.card_name(&runner.game.card_data).contains("Greymon"));
    assert!(
        has_grey,
        "a Greymon-named card must be in player 0's hand after recovery"
    );
}

/// OnPlay branch — declining the optional leaves trash and hand unchanged
/// (relative to the post-play snapshot).
#[test]
fn ad1_001_on_play_decline_leaves_trash_unchanged() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("TEST-GREYMON", "Greymon", 4, 4000));
    push_to_hand(&mut runner, 0, "AD1-001");
    push_to_trash(&mut runner, 0, "TEST-GREYMON");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-001")
        .expect("AD1-001 in hand");
    runner.play(0, hand_idx).expect("Greymon plays");

    let trash_after_play = runner.trash_size(0);
    let hand_after_play = runner.hand_size(0);

    // Decline via PASS.
    assert!(
        runner.pending_is_optional(),
        "select_trash for OnPlay/WhenDigivolving recursion must be optional"
    );
    let player = runner
        .pending_selection()
        .expect("prompt up")
        .selecting_player;
    runner
        .execute_action(player, digimon_engine::action::space::PASS)
        .expect("PASS to decline");
    runner
        .auto_resolve()
        .expect("nothing to resolve after pass");

    assert_eq!(
        runner.trash_size(0),
        trash_after_play,
        "decline leaves trash unchanged"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_after_play,
        "decline leaves hand unchanged"
    );
}

/// OnPlay branch — only Greymon/Garurumon/Omnimon-named cards qualify. A
/// trash containing only an unrelated card (e.g. "Filler") must not install
/// any prompt.
#[test]
fn ad1_001_on_play_filters_trash_to_named_cards_only() {
    let mut runner = greymon_runner();
    push_to_hand(&mut runner, 0, "AD1-001");
    push_to_trash(&mut runner, 0, "FILL"); // name = "Filler", no match.

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-001")
        .expect("AD1-001 in hand");
    runner.play(0, hand_idx).expect("Greymon plays");

    assert!(
        runner.pending_selection().is_none(),
        "no Greymon/Garurumon/Omnimon-named card in trash → no prompt"
    );
}

/// All-turns observer — accepting the prompt and selecting a Greymon-named
/// hand card free-digivolves AD1-001 into it (the digivolved permanent's
/// top card is now the Greymon-named card; memory is unchanged from the
/// digivolve cost = 0).
#[test]
fn ad1_001_all_turns_observer_accept_free_digivolves_into_greymon() {
    let mut runner = greymon_runner();
    // Eligible Greymon-named hand card with a sane Lv5 stat block. Evo-cost
    // must include level=4 so the free-digivolve onto AD1-001 (Lv4) finds a
    // matching evo_cost entry. (`ignore_color: true` bypasses color match;
    // level must match.)
    let mut grey_lv5 = make_named_digimon("GREY-LV5", "MetalGreymon", 5, 7000);
    grey_lv5.evo_costs = vec![digimon_engine::card_data::EvoCost {
        level: 4,
        memory_cost: 3,
        card_color: 0, // Red — not consulted under ignore_color
    }];
    runner.game.card_data.push(grey_lv5);
    runner
        .game
        .card_data
        .push(make_named_digimon("GARU-IN-HAND", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU-IN-HAND");
    let ad1 = runner.place_on_field(0, "AD1-001", Some(0));

    let hand_before = runner.hand_size(0);

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "GARU-IN-HAND")
        .expect("Garurumon in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");

    // Snapshot memory immediately after the play (before observer resolution),
    // so we can isolate the digivolve cost delta from the play seesaw.
    let memory_after_play = runner.memory();

    // Accept the observer's optional prompt (or first sub-prompt).
    let mut steps = 0;
    while runner.pending_selection().is_some() && steps < 10 {
        let (player, action) = {
            let s = runner.pending_selection().unwrap();
            // Always accept first action — drives the affirmative branch.
            (s.selecting_player, s.valid_action_ids[0])
        };
        runner
            .execute_action(player, action)
            .expect("resolve observer prompt step");
        steps += 1;
    }

    // After resolution, AD1-001's slot should now be topped by a "Greymon"-named
    // card. The free digivolve places GREY-LV5 on top of AD1-001.
    let perm = &runner.game.players[0].battle_area[ad1.index as usize];
    let top_name = perm.top_card().card_name(&runner.game.card_data);
    assert!(
        top_name.contains("Greymon"),
        "after free-digivolve the top card must contain \"Greymon\"; got {:?}",
        top_name
    );
    // Memory unchanged by the cost-0 digivolve (the +1 from digivolution
    // material reuse may bump memory; the key invariant is that no MEMORY
    // PAYMENT comes off — i.e. memory_after_observer >= memory_after_play).
    assert!(
        runner.memory() >= memory_after_play,
        "free-digivolve must not pay memory; memory_after_play={}, memory_after_observer={}",
        memory_after_play,
        runner.memory()
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 2,
        "hand decreased by 2: Garurumon played, GREY-LV5 free-digivolved"
    );
}

/// All-turns observer — declining leaves AD1-001 untransformed.
#[test]
fn ad1_001_all_turns_observer_decline_leaves_perm_untouched() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY-LV5", "MetalGreymon", 5, 7000));
    runner
        .game
        .card_data
        .push(make_named_digimon("GARU-IN-HAND", "Garurumon", 4, 4000));
    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU-IN-HAND");
    let ad1 = runner.place_on_field(0, "AD1-001", Some(0));

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "GARU-IN-HAND")
        .expect("Garurumon in hand");
    runner.play(0, hand_idx).expect("Garurumon plays");

    // Decline at the first prompt.
    if let Some(s) = runner.pending_selection() {
        assert!(
            s.is_optional,
            "observer prompt must be optional (\"may digivolve\")"
        );
        let player = s.selecting_player;
        runner
            .execute_action(player, digimon_engine::action::space::PASS)
            .expect("PASS to decline");
    }
    runner.auto_resolve().expect("nothing more to resolve");

    let perm = &runner.game.players[0].battle_area[ad1.index as usize];
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        "AD1-001",
        "after decline the AD1-001 slot top card must remain AD1-001"
    );
}

/// All-turns observer — works on the digivolve half too. When a "Garurumon"
/// hand card is digivolved onto a Lv3 ally, the observer's prompt installs.
#[test]
fn ad1_001_all_turns_observer_fires_on_garurumon_digivolve() {
    let mut runner = greymon_runner();
    runner
        .game
        .card_data
        .push(make_named_digimon("GREY-LV5", "MetalGreymon", 5, 7000));
    // Garurumon hand card with a Lv3 evo-cost so it can digivolve onto LV3-PLAT.
    let mut garu = make_named_digimon("GARU-IN-HAND", "Garurumon", 4, 4000);
    garu.evo_costs = vec![digimon_engine::card_data::EvoCost {
        level: 3,
        memory_cost: 0,
        card_color: 0, // Red
    }];
    runner.game.card_data.push(garu);

    push_to_hand(&mut runner, 0, "GREY-LV5");
    push_to_hand(&mut runner, 0, "GARU-IN-HAND");
    let _ad1 = runner.place_on_field(0, "AD1-001", Some(0));
    let lv3 = runner.place_on_field(0, "LV3-PLAT", Some(0));

    let cp = runner.event_checkpoint();

    // Manually fire OnDigivolve via a Digivolved trigger source carrying the
    // newly-on-top Garurumon card handle. We perform the digivolve by hand:
    // 1. Pop the GARU card from hand.
    // 2. Push it onto the Lv3 platform's source list.
    // 3. Fire WhenDigivolving + OnDigivolve manually.
    //
    // (We avoid `Game::digivolve_from_hand` because it pays memory and runs
    // affordability checks that aren't relevant to this observer test.)
    let garu_card = runner
        .game
        .player_mut(0)
        .hand
        .pop()
        .expect("Garurumon hand card");
    let garu_handle = garu_card.handle();
    let turn = runner.game.turn_count;
    {
        let perm = runner
            .game
            .player_mut(0)
            .battle_area
            .get_mut(lv3.index as usize)
            .expect("Lv3 platform on field");
        perm.digivolve(garu_card, turn);
    }
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(lv3));
    runner.game.drain_effect_queue();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: 0,
            permanent: lv3,
            card: garu_handle,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();

    // Observer prompt must install (the AD1-001 watcher saw a Garurumon-named
    // card finish digivolving on player 0's side).
    assert!(
        runner.pending_selection().is_some(),
        "observer prompt must install on a Garurumon-named digivolve"
    );

    // Drop the unused checkpoint reference (event-log assertions are not
    // load-bearing here — the observer prompt installation is the proof).
    let _ = cp;
}

// ─── SECTION 4 — Cost firing (events) ────────────────────────────────────────
//
// The OnPlay/WhenDigivolving recursion clause has no cost (it's a "you may"
// optional with no payment), and the all-turns observer's free-digivolve
// has cost: 0. There is no security trash, no memory pay, no card discard
// to assert via the event log. This card has no cost-firing clauses to
// cover here.

// ─── SECTION 5 — OPT enforcement ─────────────────────────────────────────────
//
// AD1-001 has NO printed [Once Per Turn] markers on any clause. There is no
// OPT lockout to enforce; this section is intentionally empty.

// ─── SECTION 6 — Inherited Raid (declarative keyword) ────────────────────────

/// Structurally, AD1-001's inherited Raid clause must be present (covered
/// by ad1_001_has_inherited_raid_grant_keyword above). Behavioral
/// installation of the `Keyword::Raid` on a carrier with AD1-001 in the
/// stack is verified here: `game.has_keyword` walks the digivolution
/// stack, detecting inherited `grant_keyword` declarative clauses on
/// source cards (see `game.rs` `has_keyword` — the inherited-source
/// loop). Mirrors the proven `st20_10_inherited_reboot_grants_to_carrier_via_stack`.
#[test]
fn ad1_001_inherited_raid_installs_on_carrier() {
    let lv5 = make_test_card("PLAIN-LV5", "PlainLv5");

    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 in pack")
        .add_card(lv5)
        .memory(10)
        .start();

    // Place AD1-001, snapshot its CardSource, then place a Lv5 over it
    // and insert AD1-001 BENEATH it as a digivolution source. The source
    // must sit under the carrier's top card so the inherited-keyword
    // stack-walk in `has_keyword` (`source_index + 1 < stack_size`) sees
    // it. Mirrors `st20_10_inherited_reboot_grants_to_carrier_via_stack`.
    let _ad1 = runner.place_on_field(0, "AD1-001", Some(0));
    let ad1_source = {
        let perm = &runner.game.players[0].battle_area[0];
        perm.top_card().clone()
    };
    let lv5_handle = runner.place_on_field(0, "PLAIN-LV5", Some(0));
    {
        let game = runner.game_mut();
        game.players[0].battle_area[lv5_handle.index as usize]
            .card_sources
            .insert(0, ad1_source);
    }
    assert!(
        runner.game.has_keyword(lv5_handle, Keyword::Raid),
        "inherited Raid from AD1-001 must install on the carrier"
    );
}
