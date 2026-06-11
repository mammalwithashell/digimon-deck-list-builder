//! AD1-024 Imperialdramon: Fighter Mode — Digimon, Lv.6, Blue/Green, DP 13000, Cost 13.
//!
//! # Card text (verified against the card image — authoritative)
//!
//! ＜Security A. +1＞ (This Digimon checks 1 additional security card.)
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//! [When Digivolving] [When Attacking] [Once Per Turn] Return 1 of your
//! opponent's lowest DP Digimon to the bottom of the deck.
//! [All Turns] [Once Per Turn] When Digimon are played or digivolve, you may
//! suspend 1 of your opponent's Digimon and unsuspend this Digimon. Then, if
//! played or digivolved by effects, you may return 1 of your opponent's
//! suspended Digimon to the bottom of the deck.
//!
//! Digivolve: Lv.5 Blue cost 5 (printed) /
//! xros_req: [Imperialdramon: Dragon Mode]: Cost 1 ; Lv.5 w/[Hero] trait: Cost 5
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_024.cs
//! - WD + WA share ONE Once-Per-Turn (SharedHashString "AD1_024_WD_WA").
//! - The WD/WA bounce select is MANDATORY when a target exists (canNoSelect: false),
//!   filtered to the opponent's lowest-DP Digimon (CardEffectCommons.IsMinDP),
//!   mode PutLibraryBottom.
//! - [All Turns] (hash "AD1_024_AT", OnEnterFieldAnyone): triggers when ANY Digimon
//!   is played or digivolves (either player). The whole body is optional (initial
//!   Yes/No, SetIsSkippable). Suspend-1-opp and unsuspend-self are COUPLED: self
//!   unsuspends only when a suspend target was actually picked (AfterSelect
//!   permanents.Count > 0) — or, when the opponent has NO Digimon at all, via a
//!   standalone unsuspend offer. The deck-bottom bounce of a SUSPENDED opponent
//!   Digimon happens only when the triggering play/digivolve was BY EFFECT
//!   (CardEffectCommons.IsByEffect), select canNoSelect: true.
//! - DCGO refunds the OPT when nothing executed (RemoveUse) — not expressible in
//!   the DSL today (no OPT-refund vocabulary); see qa/dsl-vocab-gaps.md.
//!
//! # Patterns this test file covers
//! - Shared mandatory OPT clause across [When Digivolving]/[When Attacking]
//! - selector: lowest_dp on select_opponent_permanent + return_to_deck bottom
//! - [All Turns] observer: when: [on_any_digimon_played, on_digivolve] with
//!   active_when all_turns, optional coupled suspend+unsuspend sub-block
//! - event_is_effect_initiated gating inside an if: step (by-effect bounce leg)
//! - alt_paths: standard Blue Lv.5 cost 5 + Dragon Mode name cost 1 + Lv.5 Hero cost 5

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};

use crate::dsl_card_data::compiled;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Opponent Digimon with a chosen DP — bounce / suspend target.
fn opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// Blue Lv.5 base satisfying AD1-024's standard alt-path
/// (`from: { level_eq: 5, color_is: blue }`, cost 5).
fn lv5_blue_base(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.colors = vec![CardColor::Blue];
    c.dp = Some(8000);
    c
}

/// A friendly filler Digimon whose play fires the [All Turns] observer.
fn friendly_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Runner with AD1-024 in P0's hand and a blue Lv.5 base + opp targets loaded.
fn fighter_mode_runner(extra: &[CardData]) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .dsl_card("AD1-024")
        .expect("AD1-024 must be in embedded DSL pack")
        .add_card(lv5_blue_base("AD1-024-BASE"));
    for c in extra {
        b = b.add_card(c.clone());
    }
    b.hand(0, &["AD1-024"]).memory(10).start()
}

/// Resolve the currently pending selection by its first non-PASS action.
fn pick_first(runner: &mut DebugRunner) {
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a selection must be pending");
        let action = *p
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .expect("at least one non-PASS candidate");
        (p.selecting_player, action)
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("selection resolves");
    runner.game.drain_effect_queue();
}

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

/// AD1-024 must grant ＜Security A. +1＞ and ＜Blocker＞ declaratively.
#[test]
fn ad1_024_grants_security_attack_plus_one_and_blocker() {
    let card = compiled("AD1-024");

    let keywords: Vec<(String, Option<i32>)> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                value,
                ..
            }) => Some((keyword.clone(), *value)),
            _ => None,
        })
        .collect();

    assert!(
        keywords
            .iter()
            .any(|(k, v)| k == "SecurityAttackPlus" && *v == Some(1)),
        "AD1-024 must grant SecurityAttackPlus +1; got {:?}",
        keywords
    );
    assert!(
        keywords.iter().any(|(k, _)| k == "Blocker"),
        "AD1-024 must grant Blocker; got {:?}",
        keywords
    );
}

/// The WD/WA bounce must be ONE multi-timing clause (DCGO shared hash
/// "AD1_024_WD_WA"), once-per-turn, and MANDATORY (no "you may" in print —
/// DCGO canNoSelect: false).
#[test]
fn ad1_024_shared_wd_wa_clause_is_mandatory_opt() {
    let card = compiled("AD1-024");

    let shared = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
        })
        .expect("AD1-024 must have a single multi-timing WD/WA triggered clause");

    assert!(
        !shared.optional,
        "WD/WA bounce is mandatory (printed text has no 'you may'; DCGO canNoSelect: false)"
    );
    assert!(
        shared.once_per_turn,
        "WD/WA clause must be once_per_turn (printed [Once Per Turn])"
    );
    assert_eq!(shared.scope, CompiledScope::FaceUp);
}

/// The [All Turns] observer must trigger on BOTH any-Digimon-played and
/// any-digivolve (DCGO CanTriggerOnPermanentPlay || CanTriggerWhenPermanentDigivolving),
/// be optional (initial Yes/No, SetIsSkippable), and once-per-turn.
#[test]
fn ad1_024_all_turns_observer_covers_play_and_digivolve() {
    let card = compiled("AD1-024");

    let observer = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAnyDigimonPlayed))
        .expect("AD1-024 must have an [All Turns] observer on on_any_digimon_played");

    assert!(
        observer.when.contains(&CompiledTiming::OnDigivolve),
        "the [All Turns] observer must also trigger on digivolves (printed 'played or digivolve')"
    );
    assert!(
        observer.once_per_turn,
        "[All Turns] clause must be once_per_turn"
    );
    assert!(
        observer.optional,
        "[All Turns] clause must be optional ('you may' — DCGO SetIsSkippable initial Yes/No)"
    );
}

/// Alt-paths: standard Blue Lv.5 cost 5 (printed), Dragon-Mode-name cost 1,
/// Lv.5 [Hero]-trait cost 5 (xros_req).
#[test]
fn ad1_024_has_three_digivolve_alt_paths() {
    use digimon_dsl::compiled::CompiledAltPathKind;

    let card = compiled("AD1-024");

    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .collect();

    assert_eq!(
        digivolve_paths.len(),
        3,
        "AD1-024 must have exactly 3 digivolve alt-paths (standard + Dragon Mode + Hero); got {}",
        digivolve_paths.len()
    );

    let costs: Vec<i32> = digivolve_paths
        .iter()
        .filter_map(|p| match &p.cost {
            Some(digimon_dsl::compiled::CompiledCost::Literal(v)) => Some(*v),
            _ => None,
        })
        .collect();
    assert!(
        costs.contains(&1),
        "the Dragon Mode path must cost 1; got costs {:?}",
        costs
    );
    assert_eq!(
        costs.iter().filter(|&&c| c == 5).count(),
        2,
        "the standard Blue Lv.5 path and the Hero path must both cost 5; got {:?}",
        costs
    );
}

// ─── SECTION 2 — [When Digivolving] bounce ───────────────────────────────────

/// Digivolving into AD1-024 installs a MANDATORY selection over ONLY the
/// opponent's lowest-DP Digimon, and resolving it puts that Digimon on the
/// BOTTOM of the opponent's deck.
#[test]
fn ad1_024_wd_bounces_lowest_dp_opp_to_deck_bottom() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-LOW", "OppLow", 2000),
        opp_digimon("AD1-024-OPP-HIGH", "OppHigh", 11000),
    ]);

    let base = runner.place_on_field(0, "AD1-024-BASE", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-LOW", None);
    runner.place_on_field(1, "AD1-024-OPP-HIGH", None);

    let opp_field_before = runner.battle_area_size(1);
    let opp_deck_before = runner.deck_size(1);

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "standard digivolve into AD1-024 from a blue Lv.5 must succeed"
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("[When Digivolving] bounce select must install");
    let non_pass: Vec<_> = pending
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .copied()
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "bounce select must offer exactly the ONE lowest-DP opponent Digimon; got {} candidates",
        non_pass.len()
    );
    assert!(
        !runner.pending_is_optional(),
        "the bounce is mandatory (DCGO canNoSelect: false) — PASS must not be legal"
    );

    pick_first(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "exactly one opponent Digimon must leave the battle area"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 1,
        "the opponent's deck must grow by exactly 1"
    );
    // Deck top is the Vec end (Player::draw pops); bottom is index 0.
    let bottom_id = runner.game.players[1].deck[0].card_id(&runner.game.card_data);
    assert_eq!(
        bottom_id, "AD1-024-OPP-LOW",
        "the bounced card must land on the BOTTOM of the deck (DCGO Mode.PutLibraryBottom)"
    );
    // The high-DP Digimon must be untouched.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "AD1-024-OPP-HIGH"),
        "the high-DP opponent Digimon must remain on the field"
    );
}

/// With NO opponent Digimon, the [When Digivolving] bounce silently no-ops.
/// The only selection left after the digivolve is the [All Turns] observer's
/// outer Yes/No (it fires off AD1-024's own digivolve and DCGO always shows
/// the initial prompt); declining it leaves nothing pending.
#[test]
fn ad1_024_wd_no_target_installs_no_bounce_selection() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = fighter_mode_runner(&[]);

    let base = runner.place_on_field(0, "AD1-024-BASE", Some(0));
    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "digivolve must succeed"
    );
    runner.game.drain_effect_queue();

    // No OppField bounce select may install — at most the All Turns outer
    // Yes/No confirm (Replacement kind).
    if let Some(p) = runner.game.pending_selection.as_ref() {
        assert_eq!(
            p.kind,
            SelectionKind::Replacement,
            "the only permissible selection is the [All Turns] outer confirm; got {:?} ({})",
            p.kind,
            p.prompt
        );
        let (player, _) = (p.selecting_player, ());
        runner
            .game
            .resolve_selection(player, PASS)
            .expect("declining the outer confirm resolves");
        runner.game.drain_effect_queue();
    }
    assert!(
        runner.game.pending_selection.is_none(),
        "nothing further may be pending after declining the All Turns confirm"
    );
}

/// [When Attacking] fires the same bounce when AD1-024 attacks.
#[test]
fn ad1_024_wa_bounce_fires_on_attack() {
    let mut runner = fighter_mode_runner(&[opp_digimon("AD1-024-OPP-LOW", "OppLow", 2000)]);

    let fighter = runner.place_on_field(0, "AD1-024", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-LOW", None);

    runner.attack_player(fighter, 1, false);

    assert!(
        runner.game.pending_selection.is_some(),
        "[When Attacking] must install the bounce select"
    );
}

/// Cross-timing shared OPT (DCGO SharedHashString "AD1_024_WD_WA"): after the
/// [When Digivolving] bounce consumed the OPT, attacking the same turn must
/// NOT fire the [When Attacking] bounce again.
#[test]
fn ad1_024_shared_opt_blocks_wa_after_wd_same_turn() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-LOW", "OppLow", 2000),
        opp_digimon("AD1-024-OPP-HIGH", "OppHigh", 11000),
    ]);

    let base = runner.place_on_field(0, "AD1-024-BASE", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-LOW", None);
    runner.place_on_field(1, "AD1-024-OPP-HIGH", None);

    assert!(
        runner
            .game
            .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByHand),
        "digivolve must succeed"
    );
    runner.game.drain_effect_queue();
    pick_first(&mut runner); // resolve the WD bounce — shared OPT consumed

    // The digivolve ALSO fires AD1-024's own [All Turns] observer (its own
    // digivolve is a "Digimon digivolved" event — DCGO OnEnterFieldAnyone +
    // IsExistOnBattleArea). Clear that selection chain before attacking so the
    // only thing left to observe is the WA bounce lock.
    let _ = runner.auto_resolve();

    // Same turn: AD1-024 attacks (base placed turn 0 → no summoning sickness).
    let fighter = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: base.index,
    };
    runner.attack_player(fighter, 1, false);

    assert!(
        runner.game.pending_selection.is_none(),
        "the shared WD/WA OPT must lock the [When Attacking] bounce after WD used it; \
         pending: {:?}",
        runner
            .game
            .pending_selection
            .as_ref()
            .map(|p| (p.prompt.clone(), p.kind.clone()))
    );
}

// ─── SECTION 3 — [All Turns] observer ────────────────────────────────────────

/// A NORMAL own play fires the observer: accepting the optional sub-block
/// suspends the picked opponent Digimon AND unsuspends AD1-024 (the coupled
/// pair), and — because the play was NOT by effect — no deck-bottom leg runs.
#[test]
fn ad1_024_all_turns_suspends_opp_and_unsuspends_self_on_normal_play() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
    ]);

    let fighter = runner.place_on_field(0, "AD1-024", Some(0));
    {
        let perm = &mut runner.game.players[0].battle_area[fighter.index as usize];
        perm.is_suspended = true; // so the self-unsuspend is observable
    }
    let opp = runner.place_on_field(1, "AD1-024-OPP-A", None);

    let opp_field_before = runner.battle_area_size(1);

    // P0 plays a filler Digimon NORMALLY (not by effect).
    let filler = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, filler.index as usize, false, false);

    assert!(
        runner.game.pending_selection.is_some(),
        "[All Turns] observer must install its optional selection on a Digimon play"
    );

    // Drive every selection, accepting optionals and picking first candidates.
    let _ = runner.auto_resolve();

    let opp_perm = &runner.game.players[1].battle_area[opp.index as usize];
    assert!(
        opp_perm.is_suspended,
        "the picked opponent Digimon must be suspended"
    );
    let fighter_perm = &runner.game.players[0].battle_area[fighter.index as usize];
    assert!(
        !fighter_perm.is_suspended,
        "AD1-024 must unsuspend (coupled with the suspend pick)"
    );
    // NOT by effect → no deck-bottom leg: the suspended opponent Digimon stays.
    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before,
        "a normal play must NOT trigger the by-effect deck-bottom leg"
    );
}

/// [All Turns]: the observer also fires on the OPPONENT'S turn (their play).
#[test]
fn ad1_024_all_turns_fires_on_opponents_turn_too() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
    ]);

    runner.place_on_field(0, "AD1-024", Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "must be P1's turn");

    // P1 plays a Digimon on their own turn.
    let played = runner.place_on_field(1, "AD1-024-OPP-A", None);
    runner.fire_play_event_triggers(1, played.index as usize, false, false);

    assert!(
        runner.game.pending_selection.is_some(),
        "[All Turns] must fire on the opponent's turn as well"
    );
}

/// A play BY EFFECT adds the second leg: after the coupled suspend+unsuspend,
/// AD1-024's controller may return 1 of the opponent's SUSPENDED Digimon to
/// the bottom of the deck.
#[test]
fn ad1_024_all_turns_by_effect_play_bottom_decks_a_suspended_opp() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
    ]);

    let fighter = runner.place_on_field(0, "AD1-024", Some(0));
    let opp = runner.place_on_field(1, "AD1-024-OPP-A", None);

    let opp_field_before = runner.battle_area_size(1);
    let opp_deck_before = runner.deck_size(1);

    // P0 plays a filler Digimon BY EFFECT.
    let filler = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, filler.index as usize, true, false);

    assert!(
        runner.game.pending_selection.is_some(),
        "[All Turns] observer must fire on an effect-initiated play"
    );

    // Accept everything: suspend OppA (leg 1) → OppA is now suspended →
    // by-effect leg offers it → bottom-deck it.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "the suspended opponent Digimon must leave the battle area (by-effect leg)"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 1,
        "the opponent's deck must grow by 1"
    );
    let bottom_id = runner.game.players[1].deck[0].card_id(&runner.game.card_data);
    assert_eq!(
        bottom_id, "AD1-024-OPP-A",
        "the bounced Digimon must land on the deck BOTTOM"
    );
    let fighter_perm = &runner.game.players[0].battle_area[fighter.index as usize];
    assert!(
        !fighter_perm.is_suspended,
        "AD1-024 stays/becomes unsuspended through the chain"
    );
    let _ = opp;
}

/// With NO opponent Digimon at all, accepting the optional sub-block still
/// unsuspends AD1-024 (DCGO's standalone unsuspend offer branch).
#[test]
fn ad1_024_all_turns_standalone_unsuspend_when_no_opp_digimon() {
    let mut runner = fighter_mode_runner(&[friendly_filler("AD1-024-FILLER")]);

    let fighter = runner.place_on_field(0, "AD1-024", Some(0));
    {
        let perm = &mut runner.game.players[0].battle_area[fighter.index as usize];
        perm.is_suspended = true;
    }

    let filler = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, filler.index as usize, false, false);

    let _ = runner.auto_resolve();

    let fighter_perm = &runner.game.players[0].battle_area[fighter.index as usize];
    assert!(
        !fighter_perm.is_suspended,
        "with no opponent Digimon, accepting the block must still unsuspend AD1-024 \
         (DCGO standalone unsuspend branch)"
    );
}

/// [Once Per Turn]: a second qualifying play in the same turn must not fire
/// the observer again.
#[test]
fn ad1_024_all_turns_is_once_per_turn() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
        friendly_filler("AD1-024-FILLER-2"),
    ]);

    runner.place_on_field(0, "AD1-024", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-A", None);

    let first = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, first.index as usize, false, false);
    assert!(
        runner.game.pending_selection.is_some(),
        "first play must fire the observer"
    );
    let _ = runner.auto_resolve();

    let second = runner.place_on_field(0, "AD1-024-FILLER-2", None);
    runner.fire_play_event_triggers(0, second.index as usize, false, false);
    assert!(
        runner.game.pending_selection.is_none(),
        "second play in the same turn must be locked by [Once Per Turn]"
    );
}

// ─── SECTION 4 — [All Turns] decline + OPT refund (DCGO RemoveUse) ──────────

/// Decline the selection currently pending (PASS on an optional selection).
fn decline_pending(runner: &mut DebugRunner) {
    let player = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a selection must be pending to decline")
        .selecting_player;
    runner
        .game
        .resolve_selection(player, PASS)
        .expect("decline resolves");
    runner.game.drain_effect_queue();
}

/// The observer always leads with DCGO's initial Yes/No (the outer confirm),
/// even though its first body step is itself declinable. DECLINING the outer
/// confirm refunds the [Once Per Turn] (DCGO RemoveUse): a second qualifying
/// play in the same turn fires the observer again.
#[test]
fn ad1_024_all_turns_decline_outer_confirm_refunds_opt() {
    use digimon_engine::selection::SelectionKind;

    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
        friendly_filler("AD1-024-FILLER-2"),
    ]);

    runner.place_on_field(0, "AD1-024", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-A", None);

    let first = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, first.index as usize, false, false);

    let p = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the [All Turns] outer confirm must install");
    assert_eq!(
        p.kind,
        SelectionKind::Replacement,
        "the first prompt must be the outer Yes/No confirm (DCGO initial bool prompt)"
    );
    decline_pending(&mut runner); // "No" — nothing executed

    // The OPT must be refunded: a second play the same turn prompts again.
    let second = runner.place_on_field(0, "AD1-024-FILLER-2", None);
    runner.fire_play_event_triggers(0, second.index as usize, false, false);
    assert!(
        runner.game.pending_selection.is_some(),
        "declining the outer confirm must refund the OPT (DCGO RemoveUse) — \
         the observer must fire again on the next qualifying play this turn"
    );
}

/// Accepting the outer confirm but then declining every sub-pick also refunds
/// the OPT (DCGO's executed-flag: nothing ran → RemoveUse).
#[test]
fn ad1_024_all_turns_accept_then_decline_everything_refunds_opt() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        friendly_filler("AD1-024-FILLER"),
        friendly_filler("AD1-024-FILLER-2"),
    ]);

    let fighter = runner.place_on_field(0, "AD1-024", Some(0));
    let opp = runner.place_on_field(1, "AD1-024-OPP-A", None);

    let first = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, first.index as usize, false, false);

    pick_first(&mut runner); // accept the outer confirm
    // The suspend pick is DECLINABLE (DCGO canNoSelect: true) — decline it.
    assert!(
        runner.game.pending_selection.is_some(),
        "the suspend pick must install after accepting the confirm"
    );
    assert!(
        runner.pending_is_optional(),
        "the suspend pick must be declinable (DCGO canNoSelect: true)"
    );
    decline_pending(&mut runner);

    // Nothing executed: no suspend, no unsuspend (opp Digimon existed, so no
    // standalone-unsuspend branch), not by effect (no bounce leg).
    let opp_perm = &runner.game.players[1].battle_area[opp.index as usize];
    assert!(!opp_perm.is_suspended, "nothing may have been suspended");

    // OPT refunded: a second qualifying play prompts again.
    let second = runner.place_on_field(0, "AD1-024-FILLER-2", None);
    runner.fire_play_event_triggers(0, second.index as usize, false, false);
    assert!(
        runner.game.pending_selection.is_some(),
        "accept-then-decline-everything must refund the OPT (DCGO executed-flag RemoveUse)"
    );
    let _ = fighter;
}

/// Declining the suspend pick does NOT skip the by-effect bounce leg — in DCGO
/// the two are independent once the initial Yes/No is accepted. And because the
/// bounce executed, the OPT is consumed (no refund).
#[test]
fn ad1_024_all_turns_declined_pick_still_offers_byeffect_bounce() {
    let mut runner = fighter_mode_runner(&[
        opp_digimon("AD1-024-OPP-A", "OppA", 5000),
        opp_digimon("AD1-024-OPP-B", "OppB", 7000),
        friendly_filler("AD1-024-FILLER"),
        friendly_filler("AD1-024-FILLER-2"),
    ]);

    runner.place_on_field(0, "AD1-024", Some(0));
    runner.place_on_field(1, "AD1-024-OPP-A", None);
    let opp_b = runner.place_on_field(1, "AD1-024-OPP-B", None);
    {
        // Pre-suspend OppB so the by-effect leg has a target even though the
        // suspend pick is declined.
        let perm = &mut runner.game.players[1].battle_area[opp_b.index as usize];
        perm.is_suspended = true;
    }

    let opp_field_before = runner.battle_area_size(1);
    let opp_deck_before = runner.deck_size(1);

    // Play BY EFFECT.
    let first = runner.place_on_field(0, "AD1-024-FILLER", None);
    runner.fire_play_event_triggers(0, first.index as usize, true, false);

    pick_first(&mut runner); // accept the outer confirm
    decline_pending(&mut runner); // decline the suspend pick

    // The by-effect bounce leg must still offer the suspended OppB.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the by-effect bounce select must install despite the declined pick");
    let non_pass: Vec<_> = pending
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .copied()
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the pre-suspended OppB qualifies for the by-effect bounce; got {}",
        non_pass.len()
    );
    pick_first(&mut runner);

    assert_eq!(
        runner.battle_area_size(1),
        opp_field_before - 1,
        "the suspended Digimon must leave the field"
    );
    assert_eq!(runner.deck_size(1), opp_deck_before + 1);
    assert_eq!(
        runner.game.players[1].deck[0].card_id(&runner.game.card_data),
        "AD1-024-OPP-B",
        "OppB must land on the deck bottom"
    );

    // The bounce executed → OPT consumed → no refund.
    let second = runner.place_on_field(0, "AD1-024-FILLER-2", None);
    runner.fire_play_event_triggers(0, second.index as usize, false, false);
    assert!(
        runner.game.pending_selection.is_none(),
        "the bounce executed, so the OPT is consumed — no second fire this turn"
    );
}
