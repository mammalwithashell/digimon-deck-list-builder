//! AD1-025 Omnimon — Lv.7, Red+White, DP 15000, Cost 15.
//! Traits: Holy Warrior, Royal Knight, ADVENTURE, Hero.
//!
//! # Card text (cards.json — printed)
//!
//! ＜Raid＞
//! ＜Blocker＞
//! ＜Partition ([WarGreymon] & [MetalGarurumon])＞
//! [On Play] [When Digivolving] Return all of your opponent's Digimon with as
//!   many or fewer digivolution cards as this Digimon to the bottom of the deck.
//!   Then, delete 1 of your opponent's Digimon.
//! [All Turns] [Once Per Turn] When any of your opponent's Digimon leave the
//!   battle area, trash 1 of their Option cards in the battle area and trash
//!   their top security card.
//!
//! DNA: Lv.6 [Greymon] + Lv.6 [Garurumon], cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Red/AD1_025.cs
//!
//! # Source priority
//! Per CLAUDE.md: printed text > RULES_CONTEXT.md (§16-28 Partition) > fandom > DCGO.
//!
//! # Patterns this audit exercises
//! - H5  GrantKeyword Blocker (face-up declarative)
//! - H9  GrantKeyword Raid (face-up declarative)
//! - Partition (declarative; sources [WarGreymon] / [MetalGarurumon])
//! - Shared OnPlay+WhenDigivolving body with bounce-then-delete
//! - All-Turns OPT observer firing on opponent Digimon leaving the battle area
//! - DNA digivolve alt-path (Lv.6 Greymon + Lv.6 Garurumon, cost 0)
//!
//! # AUDIT VERDICT (drift): see top-level audit report.
//!
//! Two pieces of drift were identified during this audit. The tests below
//! document the *intended* behavior; tests that rely on the missing raw_rust
//! body or the missing [All Turns] clause are `#[ignore]`'d with the gap tag.
//!
//! ## DRIFT 1 — `raw_rust: { fn: ad1_025_on_play_process }` is unregistered
//!
//! The YAML's `[on_play, when_digivolving]` process body is a single
//! `raw_rust: { fn: ad1_025_on_play_process }` step. That function name is
//! validated against a `StubRegistry` in `tests/dsl/phase0_exit.rs` and
//! `tests/dsl/roundtrip.rs`, but it is **NOT** registered in the production
//! engine raw_rust registry (`code/digimon-engine/src/cards/raw_rust/mod.rs::build_registry`).
//! At runtime `step::run_step_with_runtime` looks the function up via
//! `runtime.raw().step_fn(fn_name)` and silently skips when the lookup
//! returns `None`. The clause therefore parses, validates, and lowers, but
//! the OnPlay/WhenDigivolving body never executes — opponent Digimon never
//! get bounced and the follow-up delete never installs.
//!
//! ## DRIFT 2 — Missing [All Turns] [OPT] clause
//!
//! The printed text has a SECOND triggered clause that the YAML omits
//! entirely:
//!   "[All Turns] [Once Per Turn] When any of your opponent's Digimon leave
//!    the battle area, trash 1 of their Option cards in the battle area and
//!    trash their top security card."
//!
//! DCGO `AD1_025.cs` lines 152–211 implement this on `EffectTiming.OnLeaveFieldAnyone`,
//! gated by `CardEffectCommons.CanTriggerOnPermanentLeave(hashtable, IsOpponentsDigimon)`,
//! with `max-per-turn: 1` (DCGO `SetUpActivateClass(..., 1, false, ...)` arg #3),
//! optional Option-card pick (mandatory if any), then unconditional
//! `IDestroySecurity(enemy, 1, fromTop: true)`.
//!
//! ## Note on DCGO line 124 / 141 max-per-turn arg `-1`
//!
//! DCGO's shared OnPlay/WhenDigivolving `SetUpActivateClass(..., -1, false, ...)`
//! passes `-1` for max-per-turn (= unlimited) and `false` for isOptional. So
//! the OnPlay/WhenDigivolving clause is mandatory, not OPT. The YAML matches
//! (no `optional:` flag, no `once_per_turn:` flag).
//!
//! # Known engine gaps that affect the All-Turns clause once the YAML is fixed
//!
//! - **G-OPT-TRIGGERED** (qa/archetype-qa/engine-gaps.md): max_per_turn on
//!   triggered clauses is fixed for permanent-backed queued effects but does
//!   not yet cover every dispatch path. The All-Turns OPT lockout test will
//!   pass for the in-scope path on the fixed engine.
//! - **OnLeaveField timing dispatch for "any opponent Digimon"**: the DSL
//!   timing enum is `CompiledTiming::OnLeaveField`, but the lower-and-fire
//!   plumbing for an OBSERVER scope (battle-area carrier reacting to a
//!   DIFFERENT permanent leaving the OPPONENT's field) is not covered by any
//!   existing per-card test — verify before unblocking the missing clause.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledPredicate, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn make_named_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.card_kind = CardKind::Digimon;
    c
}

fn make_named_option(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Option;
    c
}

/// Standard fixture: AD1-025 (Omnimon) registered + helpful test pieces.
fn omnimon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 must be in embedded DSL pack")
        .add_card(make_named_digimon("FILL", "Filler", 4, 4000))
        .memory(20)
        .start()
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

/// Recursively check whether a CompiledPredicate node (or any descendant)
/// satisfies `f`. Walks all_of/any_of/none_of/not branches.
fn pred_any<F: Fn(&CompiledPredicate) -> bool + Copy>(p: &CompiledPredicate, f: F) -> bool {
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

// ─── SECTION 1 — Structural assertions ───────────────────────────────────────

#[test]
fn ad1_025_compiles_in_embedded_pack() {
    let runner = omnimon_runner();
    assert!(
        runner.compiled_card("AD1-025").is_some(),
        "AD1-025 must be present in embedded DSL pack"
    );
}

/// Card-level metadata: level 7, DP 15000, cost 15.
#[test]
fn ad1_025_card_metadata_matches_print() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    assert_eq!(card.level, Some(7), "Omnimon is Lv.7");
    assert_eq!(card.dp, Some(15000), "Omnimon is DP 15000");
    assert_eq!(card.cost, Some(15), "Omnimon costs 15 to play");
}

/// Face-up Raid grant_keyword clause must be present.
#[test]
fn ad1_025_has_face_up_raid_grant_keyword() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let raid = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Raid"
        )
    });
    assert!(
        raid,
        "AD1-025 must declare a face-up GrantKeyword(Raid) clause"
    );
}

/// Face-up Blocker grant_keyword clause must be present.
#[test]
fn ad1_025_has_face_up_blocker_grant_keyword() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Blocker"
        )
    });
    assert!(
        blocker,
        "AD1-025 must declare a face-up GrantKeyword(Blocker) clause"
    );
}

/// Partition declarative with two source predicates (WarGreymon and
/// MetalGarurumon) must be present.
#[test]
fn ad1_025_has_partition_with_wargreymon_and_metalgarurumon() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let partition = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition { sources, .. }) => {
            Some(sources)
        }
        _ => None,
    });
    let sources = partition.expect("AD1-025 must declare a Partition clause");
    assert_eq!(
        sources.len(),
        2,
        "Partition must list exactly 2 source predicates"
    );

    let has_wargrey = sources
        .iter()
        .any(|p| pred_any(p, |q| q.name_contains.as_deref() == Some("WarGreymon")));
    let has_metalgaru = sources
        .iter()
        .any(|p| pred_any(p, |q| q.name_contains.as_deref() == Some("MetalGarurumon")));
    assert!(
        has_wargrey,
        "Partition sources must include name_contains: WarGreymon"
    );
    assert!(
        has_metalgaru,
        "Partition sources must include name_contains: MetalGarurumon"
    );
}

/// Partition `exclude_cause` must contain at least `battle` per RULES_CONTEXT
/// §16-28 ("not by battle"). The current YAML also adds `own_effect`. Audit:
/// printed text only excludes battle, but DCGO confirms own-effect is also
/// excluded; the YAML's `[own_effect, battle]` matches DCGO behavior.
#[test]
fn ad1_025_partition_excludes_battle() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let exclude = card.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
            exclude_cause, ..
        }) => Some(exclude_cause),
        _ => None,
    });
    let xs = exclude.expect("Partition clause must exist");
    assert!(
        xs.iter().any(|s| s == "battle"),
        "Partition must exclude `battle` per Rules Manual §16-28; got {xs:?}"
    );
}

/// DNA digivolve alt-path must exist with the two materials and cost 0.
#[test]
fn ad1_025_has_dna_digivolve_alt_path() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let dna_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(
        dna_paths.len(),
        1,
        "AD1-025 must have exactly one DNA digivolve alt-path"
    );

    let path = dna_paths[0];
    assert_eq!(
        path.cost,
        Some(CompiledCost::Literal(0)),
        "DNA cost is 0 (printed text)"
    );
    assert_eq!(
        path.materials.len(),
        2,
        "DNA requires exactly 2 materials (Greymon + Garurumon)"
    );

    let has_greymon = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(6) && q.name_contains.as_deref() == Some("Greymon")
        })
    });
    let has_garurumon = path.materials.iter().any(|m| {
        pred_any(&m.filter, |q| {
            q.level_eq == Some(6) && q.name_contains.as_deref() == Some("Garurumon")
        })
    });
    assert!(
        has_greymon,
        "DNA materials must include Lv.6 [Greymon]-named Digimon"
    );
    assert!(
        has_garurumon,
        "DNA materials must include Lv.6 [Garurumon]-named Digimon"
    );
}

/// One triggered clause must fire on [OnPlay, WhenDigivolving], be MANDATORY
/// (no "you may"), and NOT once-per-turn.
#[test]
fn ad1_025_has_on_play_when_digivolving_mandatory_clause() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

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
        .expect("AD1-025 must have a triggered clause covering OnPlay + WhenDigivolving");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "OnPlay/WhenDigivolving clause must have FaceUp scope"
    );
    assert!(
        !clause.optional,
        "OnPlay/WhenDigivolving clause is MANDATORY \
         (printed text has no \"you may\"; DCGO line 125 SetUpActivateClass \
         passes isOptional=false)"
    );
    assert!(
        !clause.once_per_turn,
        "OnPlay/WhenDigivolving clause is NOT [Once Per Turn] \
         (DCGO line 125 passes max-per-turn = -1)"
    );
}

/// DRIFT — printed text has a [All Turns] [Once Per Turn] clause that
/// triggers when an opponent's Digimon leaves the battle area. The YAML
/// omits this clause entirely. This test asserts the missing clause's
/// shape (it will fail until the YAML is fixed). When the YAML is patched,
/// remove the `#[ignore]` and this test should pass.
#[test]
#[ignore = "pending YAML drift fix: missing [All Turns][OPT] clause on OnLeaveField. \
            See top-of-file DRIFT 2 and the audit verdict."]
fn ad1_025_has_all_turns_opt_observer_on_opp_digimon_leave() {
    let runner = omnimon_runner();
    let card = runner.compiled_card("AD1-025").expect("AD1-025 compiles");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnLeaveField))
        .expect(
            "AD1-025 [All Turns][OPT] observer clause should fire on \
             OnLeaveField (DCGO EffectTiming.OnLeaveFieldAnyone)",
        );

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "All-Turns observer is face-up scope"
    );
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] is printed; DCGO line 157 passes max-per-turn = 1"
    );
    assert!(
        clause
            .active_when
            .as_ref()
            .map(|p| {
                pred_any(p, |q| {
                    // Some indication this is an "all turns" / "any turn" gate.
                    q.all_turns == Some(true)
                })
            })
            .unwrap_or(false),
        "[All Turns] requires active_when: {{ all_turns: true }} so the observer \
         fires on the opponent's turn too (default would be your-turn-only)"
    );
}

// ─── SECTION 2 — Condition gating (positive + negative) ──────────────────────

/// Negative gate (OnPlay): when opponent has NO Digimon at all, the OnPlay
/// body should resolve without installing any pending selection (no opponent
/// Digimon to bounce, no opponent Digimon to delete).
///
/// IGNORED — see DRIFT 1: the raw_rust function `ad1_025_on_play_process`
/// is not registered in the production raw_rust registry, so the body
/// silently no-ops in BOTH branches. This test cannot distinguish "correct
/// no-op because no targets" from "broken because no implementation".
#[test]
#[ignore = "pending raw_rust fn registration: `ad1_025_on_play_process` is referenced by YAML but missing from \
            code/digimon-engine/src/cards/raw_rust/mod.rs::build_registry. See top-of-file DRIFT 1."]
fn ad1_025_on_play_no_opponent_digimon_no_prompt() {
    let mut runner = omnimon_runner();
    push_to_hand(&mut runner, 0, "AD1-025");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-025")
        .expect("AD1-025 in hand");
    runner.play(0, hand_idx).expect("Omnimon plays from hand");

    // Even with no opponent Digimon, the YAML's body is supposed to no-op
    // gracefully (bottom-of-deck of an empty list, then no select prompt
    // because no opp Digimon exists for the delete). Verify no prompt.
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no select prompt installed"
    );
}

/// Positive gate (OnPlay): when opponent HAS a Digimon with ≤ digivolution
/// cards as Omnimon (Omnimon plays from hand with 0 sources, so any opp
/// Digimon with 0 sources qualifies for the bottom-of-deck step), the body
/// should:
///   1. Move that opp Digimon to the bottom of the opponent's deck.
///   2. If opp still has any Digimon, install a delete-target prompt.
///
/// In this fixture the opp has exactly ONE Digimon, so step 1 returns it,
/// then step 2 finds no remaining target → no prompt. The observable proof
/// is that the opp's battle area becomes empty AND their deck grows by 1.
///
/// IGNORED — see DRIFT 1: raw_rust body unregistered, so nothing happens.
#[test]
#[ignore = "pending raw_rust fn registration: `ad1_025_on_play_process` missing. See DRIFT 1."]
fn ad1_025_on_play_returns_lower_source_opp_digimon_to_deck_bottom() {
    let mut runner = omnimon_runner();
    push_to_hand(&mut runner, 0, "AD1-025");
    let _opp_dig = runner.place_on_field(1, "FILL", Some(0));

    let opp_battle_before = runner.battle_area_size(1);
    let opp_deck_before = runner.deck_size(1);
    assert_eq!(opp_battle_before, 1, "precondition: opp has 1 Digimon");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-025")
        .expect("AD1-025 in hand");
    runner.play(0, hand_idx).expect("Omnimon plays from hand");
    runner.auto_resolve().expect("auto-resolve any follow-ups");

    assert_eq!(
        runner.battle_area_size(1),
        opp_battle_before - 1,
        "opp Digimon with 0 digi cards (≤ Omnimon's 0) must be bounced to deck"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 1,
        "bounced Digimon must land at the bottom of opp deck"
    );
}

/// Positive gate (OnPlay): when opp has TWO Digimon — one with 0 sources
/// (bounceable) and one with MORE sources than Omnimon (non-bounceable) —
/// the bounce step removes only the lower-source one, then the delete step
/// installs an OppField prompt for the surviving Digimon.
///
/// IGNORED — see DRIFT 1.
#[test]
#[ignore = "pending raw_rust fn registration: `ad1_025_on_play_process` missing. See DRIFT 1."]
fn ad1_025_on_play_then_installs_delete_prompt_for_remaining_opp_digimon() {
    let mut runner = omnimon_runner();
    push_to_hand(&mut runner, 0, "AD1-025");
    let _bounce = runner.place_on_field(1, "FILL", Some(0));

    // Build a more-sources opponent Digimon so the bounce step skips it.
    let extra = make_named_digimon("EXTRA", "ExtraStack", 5, 6000);
    runner.game.card_data.push(extra);
    let stack_top = runner.place_on_field(1, "EXTRA", Some(0));
    // Push a phantom source below the top so this perm has 1 digivolution
    // card (more than Omnimon's 0).
    runner.push_source(stack_top, "FILL");

    let hand_idx = runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "AD1-025")
        .expect("AD1-025 in hand");
    runner.play(0, hand_idx).expect("Omnimon plays");

    let pending = runner
        .pending_selection()
        .expect("delete prompt must install when opp still has Digimon after bounce");
    assert_eq!(
        pending.kind,
        SelectionKind::OppField,
        "second step is select_opponent_permanent → OppField"
    );
    assert!(
        !pending.is_optional,
        "DCGO line 107 SelectPermanentEffect.SetUp passes canNoSelect: false → \
         delete-target select is mandatory when an opp Digimon exists"
    );
}

// ─── SECTION 3 — Behavioral outcome (per branch, integrated) ─────────────────

/// Behavioral integrated test for the WhenDigivolving timing: enqueueing
/// `WhenDigivolving` on an Omnimon permanent must trigger the same shared
/// body as OnPlay (DCGO uses identical `SharedActivateCoroutine`).
///
/// IGNORED — see DRIFT 1.
#[test]
#[ignore = "pending raw_rust fn registration: `ad1_025_on_play_process` missing. See DRIFT 1."]
fn ad1_025_when_digivolving_fires_same_body_as_on_play() {
    let mut runner = omnimon_runner();
    let _opp = runner.place_on_field(1, "FILL", Some(0));
    let omni = runner.place_on_field(0, "AD1-025", None);

    let opp_battle_before = runner.battle_area_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(omni),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve");

    assert!(
        runner.battle_area_size(1) < opp_battle_before,
        "WhenDigivolving must fire the bounce body (same shared coroutine as OnPlay)"
    );
}

/// Behavioral test for the missing [All Turns] OPT clause: when an opponent's
/// Digimon leaves the battle area while Omnimon is on the field, the clause
/// should:
///   - prompt the controller to pick 1 of the opponent's option cards (only
///     if any exist; mandatory when present per DCGO line 196 canNoSelect: false),
///   - then unconditionally trash the top of the opponent's security stack.
///
/// IGNORED — see DRIFT 2: the entire [All Turns][OPT] clause is missing
/// from the YAML.
#[test]
#[ignore = "pending YAML drift fix: missing [All Turns][OPT] clause. See DRIFT 2."]
fn ad1_025_all_turns_observer_trashes_opp_option_and_top_security() {
    let mut runner = omnimon_runner();
    runner
        .game
        .card_data
        .push(make_named_option("OPP-OPT", "EnemyOption"));
    let _omni = runner.place_on_field(0, "AD1-025", Some(0));
    let opp_dig = runner.place_on_field(1, "FILL", Some(0));
    let _opp_opt = runner.place_on_field(1, "OPP-OPT", Some(0));
    // Seed opp security so the security trash has something to consume.
    push_to_hand(&mut runner, 1, "FILL");
    {
        let card = runner.game.player_mut(1).hand.pop().expect("seeded card");
        runner.game.player_mut(1).security.push(card);
    }
    let opp_security_before = runner.security_count(1);

    // Cause the opp Digimon to leave (delete via direct engine call).
    runner.game.delete_permanent_with_effects(opp_dig);
    runner.game.drain_effect_queue();

    // Resolve any prompts (the option pick).
    runner.auto_resolve().expect("auto-resolve");

    assert_eq!(
        runner.security_count(1),
        opp_security_before - 1,
        "top security must be trashed (DCGO line 205 IDestroySecurity, fromTop: true)"
    );
}

/// Cost-firing observation for the missing [All Turns] clause: trashing
/// the top of the opponent's security stack must route through the engine's
/// security-trash path so the trashed card actually lands in the opponent's
/// trash (and so observer cards subscribing to OnLoseSecurity inside the
/// engine wake up — verified at the engine boundary, not via GameEvent
/// since the public `GameEvent` enum has no security-loss variant today).
///
/// IGNORED — depends on DRIFT 2 being fixed first.
#[test]
#[ignore = "pending YAML drift fix: missing [All Turns][OPT] clause. See DRIFT 2."]
fn ad1_025_all_turns_observer_routes_security_to_opp_trash() {
    let mut runner = omnimon_runner();
    let _omni = runner.place_on_field(0, "AD1-025", Some(0));
    let opp_dig = runner.place_on_field(1, "FILL", Some(0));
    push_to_hand(&mut runner, 1, "FILL");
    {
        let card = runner.game.player_mut(1).hand.pop().expect("seeded card");
        runner.game.player_mut(1).security.push(card);
    }
    let trash_before = runner.trash_size(1);

    runner.game.delete_permanent_with_effects(opp_dig);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve");

    assert_eq!(
        runner.trash_size(1),
        trash_before + 1,
        "trashed top security must end up in the opponent's trash"
    );
}

// ─── SECTION 4 — Cost firing (events) ────────────────────────────────────────
//
// The OnPlay/WhenDigivolving body has no cost (no "by trashing", no memory
// pay beyond the regular play cost). The All-Turns clause has no cost either
// (the option pick is not a payment — it's the body itself). No cost-firing
// event-log assertions to add here beyond the OnLoseSecurity check above.

// ─── SECTION 5 — OPT enforcement ─────────────────────────────────────────────

/// OPT lockout for the [All Turns] [Once Per Turn] clause: after firing
/// once, a second opp-Digimon-leave on the same turn must NOT install a
/// new selection or trash a second security card.
///
/// IGNORED — depends on DRIFT 2 (YAML drift). Phase 2 Track C closure:
/// OPT enforcement and reset are no longer blocking; the residual failure
/// is the DRIFT 2 fix only.
#[test]
#[ignore = "pending YAML drift fix (DRIFT 2). See top-of-file. OPT half closed in Phase 2 Track C."]
fn ad1_025_all_turns_observer_opt_blocks_second_trigger_same_turn() {
    let mut runner = omnimon_runner();
    let _omni = runner.place_on_field(0, "AD1-025", Some(0));
    let opp_a = runner.place_on_field(1, "FILL", Some(0));
    let opp_b = runner.place_on_field(1, "FILL", Some(0));
    // Seed two security cards so we can detect the OPT.
    for _ in 0..2 {
        push_to_hand(&mut runner, 1, "FILL");
        let card = runner.game.player_mut(1).hand.pop().expect("seeded card");
        runner.game.player_mut(1).security.push(card);
    }
    let security_start = runner.security_count(1);

    // First trigger.
    runner.game.delete_permanent_with_effects(opp_a);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve first trigger");
    assert_eq!(
        runner.security_count(1),
        security_start - 1,
        "first OPT activation should consume one security"
    );

    // Second trigger same turn — should be locked out.
    runner.game.delete_permanent_with_effects(opp_b);
    runner.game.drain_effect_queue();
    runner.auto_resolve().expect("auto-resolve second trigger");
    assert_eq!(
        runner.security_count(1),
        security_start - 1,
        "[Once Per Turn] must lock out the second trigger on the same turn"
    );
}

// ─── SECTION 6 — Notes on declarative keyword runtime installation ───────────
//
// Per ad1_001's `inherited_raid_installs_on_carrier` precedent, behavioral
// installation of the GrantKeyword(Raid)/GrantKeyword(Blocker) modifiers on
// the carrier at runtime is gated by **G-DECLARATIVE-KEYWORD**: declarative
// grant_keyword clauses compile to `EffectTiming::Declarative` but are not
// enqueued by the engine. Structural assertions above (the GrantKeyword
// match arms with `CompiledScope::FaceUp`) are the load-bearing checks for
// these clauses until the gap closes.
