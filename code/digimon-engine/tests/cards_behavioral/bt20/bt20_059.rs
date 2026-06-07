//! BT20-059 Gankoomon (X Antibody) — Digimon, Lv.6, Black, DP 13000, Cost 13.
//! Traits: Holy Warrior / X Antibody / Royal Knight. Attribute: Data.
//!
//! # Card text (cards.json / BT20-059.json)
//!
//! [When Digivolving] ＜De-Digivolve 2＞ 1 of your opponent's Digimon. (Trash
//!   up to 2 cards from the top. You can't trash past level 3 cards.) Then, if
//!   [Gankoomon] or [X Antibody] is in this Digimon's digivolution cards, until
//!   the end of your opponent's turn, none of your Digimon are affected by your
//!   opponent's Digimon's effects.
//! [Opponent's Turn] All of your Digimon with [Sistermon] or [Huckmon] in their
//!   names or the [Royal Knight] trait gain ＜Reboot＞ and ＜Blocker＞.
//!
//! Inherited Effect:
//! [Opponent's Turn] While this Digimon is [Jesmon GX] all of your Digimon gain
//!   ＜Reboot＞ and ＜Blocker＞.
//!
//! Alt-digivolve (xros_req): [Digivolve] [Gankoomon]: Cost 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT20/Black/BT20_059.cs
//!   - AddSelfDigivolutionRequirementStaticEffect (TopCard EqualsCardName
//!     "Gankoomon", digivolutionCost 2) — alt-digivolve.
//!   - OnEnterFieldAnyone ActivateClass: IDegeneration(perm, 2) on a chosen
//!     opponent Digimon (mandatory), then if digivolution cards contain
//!     [Gankoomon]/[X Antibody], install a CanNotAffectedClass over every owner
//!     battle-area Digimon (CardCondition) filtering opponent Digimon effects
//!     (SkillCondition: IsOpponentEffect && IsDigimonEffect), registered in
//!     UntilOpponentTurnEndEffects.
//!   - EffectTiming.None RebootStaticEffect + BlockerStaticEffect with
//!     PermanentCondition (owner battle-area Digimon, name Sistermon/Huckmon or
//!     Royal Knight trait) gated on IsOpponentTurn.
//!   - Inherited ESS: RebootStaticEffect + BlockerStaticEffect
//!     (isInheritedEffect: true) with carrier TopCard EqualsCardName
//!     "Jesmon GX", gated on IsOpponentTurn.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D4 declarative aura: own battle-area Digimon filtered by name OR trait,
//!   granting keywords (Reboot / Blocker), gated on opponent's turn.
//! - F6 effect immunity: board-wide "none of your Digimon are affected by your
//!   opponent's Digimon effects" via `grant_effect_immunity` over each ally,
//!   gated on a digivolution-source condition (Gankoomon / X Antibody).
//! - de_digivolve mandatory opponent-Digimon select (stop_at_level: 3).
//! - Inherited [Opponent's Turn] conditional aura (carrier is [Jesmon GX]).
//!
//! # Q28 (judge-quiz) note
//! The board-wide immunity ("none of your Digimon are affected by your
//! opponent's Digimon's effects") is the load-bearing clause for judge-quiz
//! Q28. `grant_effect_immunity` installs a `CannotBeAffected` modifier
//! (filtered source_kind: Digimon, controller: opponent) on each of your
//! Digimon. The judge scenario itself is pinned by
//! `tests/judge_quiz/a_immunity_scope.rs::q28_...` (orchestrator-owned); this
//! file proves BT20-059 installs the protection on the right targets so that an
//! opponent's [On Play] "don't activate" lock (modeled as `add_modifier`, which
//! is guarded by `EffectContext::add_modifier` → `can_affect_permanent`) cannot
//! land on a protected ally.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor,
    CompiledDeclarativeClause, CompiledPlayerRef, CompiledScope, CompiledTiming, CompiledZone,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectSourceKind, EffectTiming, Keyword};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT20-059";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A vanilla Black Lv6 Digimon, used as a stand-in opponent De-Digivolve
/// target (built as a 3-source stack so De-Digivolve 2 actually peels 2).
fn opp_stack_card(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 11;
    c
}

/// An own Digimon with a chosen name and trait set (Reboot/Blocker aura target
/// matching).
fn own_digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Black Lv5 carrier source so a placed Gankoomon X stack has a digivolution
/// source that is NOT [Gankoomon]/[X Antibody] (negative immunity gate case).
fn plain_source(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(5);
    c.dp = Some(8000);
    c.play_cost = 7;
    c
}

/// Source carrying the [Gankoomon] name so the WD immunity condition passes.
fn gankoomon_source(id: &str) -> CardData {
    let mut c = make_test_card(id, "Gankoomon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Black];
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 8;
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT20-059 YAML loads")
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt20_059_metadata_matches_printed_card() {
    let runner = builder().memory(13).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT20-059");

    assert_eq!(card.name, "Gankoomon (X Antibody)");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(13));
    assert_eq!(card.dp, Some(13000));
    assert_eq!(card.color, vec![CompiledColor::Black]);
    assert!(card.traits.iter().any(|t| t == "Holy Warrior"));
    assert!(card.traits.iter().any(|t| t == "X Antibody"));
    assert!(card.traits.iter().any(|t| t == "Royal Knight"));
}

#[test]
fn bt20_059_has_gankoomon_alt_digivolve_cost_two() {
    let runner = builder().memory(13).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT20-059");

    assert!(
        card.alt_paths.iter().any(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.from.as_ref().is_some_and(|from| {
                    // The [Gankoomon] alt path matches name "Gankoomon".
                    from.name_is.as_deref() == Some("Gankoomon")
                        || from
                            .all_of
                            .iter()
                            .any(|pred| pred.name_is.as_deref() == Some("Gankoomon"))
                })
        }),
        "BT20-059 must carry the [Gankoomon] cost-2 alt-digivolve path"
    );
}

#[test]
fn bt20_059_has_one_when_digivolving_clause() {
    let runner = builder().memory(13).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT20-059");

    let wd: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        wd.len(),
        1,
        "exactly one [When Digivolving] triggered clause"
    );
    assert_eq!(wd[0].when, vec![CompiledTiming::WhenDigivolving]);
}

#[test]
fn bt20_059_has_face_up_reboot_and_blocker_auras() {
    let runner = builder().memory(13).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT20-059");

    let keyword_auras: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::FaceUp,
                grant_keyword: Some(k),
                ..
            }) => Some(k.keyword.clone()),
            _ => None,
        })
        .collect();
    assert!(
        keyword_auras.iter().any(|k| k == "Reboot"),
        "face-up Reboot aura present (got {keyword_auras:?})"
    );
    assert!(
        keyword_auras.iter().any(|k| k == "Blocker"),
        "face-up Blocker aura present (got {keyword_auras:?})"
    );
}

#[test]
fn bt20_059_has_inherited_reboot_and_blocker_auras() {
    let runner = builder().memory(13).start();
    let card = runner.compiled_card(CARD_ID).expect("compiled BT20-059");

    let inherited_keyword_auras: Vec<String> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                grant_keyword: Some(k),
                ..
            }) => Some(k.keyword.clone()),
            _ => None,
        })
        .collect();
    assert!(
        inherited_keyword_auras.iter().any(|k| k == "Reboot"),
        "inherited Reboot aura present (got {inherited_keyword_auras:?})"
    );
    assert!(
        inherited_keyword_auras.iter().any(|k| k == "Blocker"),
        "inherited Blocker aura present (got {inherited_keyword_auras:?})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — [When Digivolving] De-Digivolve 2 (condition gating)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: opponent has a Digimon → WD installs an OppField target select.
#[test]
fn bt20_059_wd_with_opponent_digimon_installs_de_digivolve_select() {
    let mut runner = builder()
        .add_card(opp_stack_card("OPP-A", "OppA"))
        .add_card(opp_stack_card("OPP-S1", "OppS1"))
        .add_card(opp_stack_card("OPP-S2", "OppS2"))
        .memory(13)
        .start();
    runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-A"]);
    let gx = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "WD first step is select_opponent_permanent (De-Digivolve 2)"
    );
}

/// Negative: opponent has no Digimon → no De-Digivolve select installs.
#[test]
fn bt20_059_wd_without_opponent_digimon_installs_no_de_digivolve_select() {
    let mut runner = builder().memory(13).start();
    let gx = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve select with no opponent Digimon"
    );
}

/// The De-Digivolve target select is mandatory (DCGO canNoSelect: false).
#[test]
fn bt20_059_de_digivolve_select_is_mandatory() {
    let mut runner = builder()
        .add_card(opp_stack_card("OPP-A", "OppA"))
        .add_card(opp_stack_card("OPP-S1", "OppS1"))
        .add_card(opp_stack_card("OPP-S2", "OppS2"))
        .memory(13)
        .start();
    runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-A"]);
    let gx = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("OppField select");
    assert!(
        !view.is_optional,
        "De-Digivolve 2 target select must be mandatory (DCGO canNoSelect: false)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [When Digivolving] De-Digivolve 2 behavior
// ═══════════════════════════════════════════════════════════════════════════

/// De-Digivolve 2 peels 2 digivolution sources off the chosen opponent Digimon
/// (a 3-source stack → 1 source remaining).
#[test]
fn bt20_059_de_digivolve_two_peels_two_sources() {
    let mut runner = builder()
        .add_card(opp_stack_card("OPP-A", "OppA"))
        .add_card(opp_stack_card("OPP-S1", "OppS1"))
        .add_card(opp_stack_card("OPP-S2", "OppS2"))
        .memory(13)
        .start();
    // Stack: [OPP-S1 (bottom), OPP-S2, OPP-A (top)] → 3 sources total.
    let target = runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-A"]);
    let sources_before = runner.game.players[1].battle_area[target.index as usize]
        .card_sources
        .len();
    assert_eq!(sources_before, 3, "target starts with 3 card sources");

    let gx = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("OppField select");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("De-Digivolve target resolves");
    runner.auto_resolve().ok();

    // De-Digivolve removes from the TOP: with stack [OPP-S1, OPP-S2, OPP-A]
    // (all level 6, so the level-3 floor never trips), De-Digivolve 2 pops the
    // top two cards (OPP-A then OPP-S2), leaving a single-card permanent whose
    // top is the bottom-most source OPP-S1.
    let opp_perms = &runner.game.players[1].battle_area;
    assert_eq!(opp_perms.len(), 1, "the opponent Digimon survives");
    let target_now = &opp_perms[0];
    assert_eq!(
        target_now.card_sources.len(),
        1,
        "De-Digivolve 2 peels exactly 2 of the 3 sources"
    );
    assert_eq!(
        target_now.top_card().card_id(&runner.game.card_data),
        "OPP-S1",
        "the surviving face card is the bottom-most source (De-Digivolve peels from the top)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [When Digivolving] board-wide immunity (condition gating + Q28)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with [Gankoomon] in the digivolution sources, after the WD
/// resolves EVERY one of your Digimon is immune to opponent Digimon effects
/// (until end of opponent's turn). This is the protection that beats Dragomon's
/// [On Play] lock in judge-quiz Q28.
#[test]
fn bt20_059_wd_grants_board_wide_opponent_digimon_immunity_when_sources_qualify() {
    let mut runner = builder()
        .add_card(opp_stack_card("OPP-A", "OppA"))
        .add_card(opp_stack_card("OPP-S1", "OppS1"))
        .add_card(opp_stack_card("OPP-S2", "OppS2"))
        .add_card(gankoomon_source("GANKOO-SRC"))
        .add_card(own_digimon("ALLY-CIEL", "Sistermon Ciel", &["Puppet"]))
        .memory(13)
        .start();
    runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-A"]);
    // An ally already on the board when Gankoomon X digivolves.
    let ally = runner.place_on_field(0, "ALLY-CIEL", Some(0));
    // Gankoomon X stacked on a [Gankoomon] source → immunity condition passes.
    let gx = runner.place_stack(0, &["GANKOO-SRC", CARD_ID]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    // Resolve the mandatory De-Digivolve target select, then any tail.
    let view = runner.pending_selection_view().expect("OppField select");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("De-Digivolve target resolves");
    runner.auto_resolve().ok();

    // Both the ally AND Gankoomon X itself are immune to opponent Digimon
    // effects.
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Digimon),
        "the ally must be immune to opponent Digimon effects (Q28 protection)"
    );
    assert!(
        runner
            .game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Digimon),
        "Gankoomon X itself is one of 'your Digimon' and must be immune too"
    );
    // But NOT to opponent Tamer/Option effects — the protection is Digimon-only.
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Tamer),
        "the protection is Digimon-effect-only; opponent Tamer effects still apply"
    );
    // And NOT to your OWN effects — opponent-controller-scoped.
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 0, EffectSourceKind::Digimon),
        "opponent-scoped immunity must not block your own Digimon effects"
    );
}

/// Negative: with NO [Gankoomon]/[X Antibody] in the digivolution sources, the
/// WD still De-Digivolves but grants NO immunity.
#[test]
fn bt20_059_wd_grants_no_immunity_when_sources_do_not_qualify() {
    let mut runner = builder()
        .add_card(opp_stack_card("OPP-A", "OppA"))
        .add_card(opp_stack_card("OPP-S1", "OppS1"))
        .add_card(opp_stack_card("OPP-S2", "OppS2"))
        .add_card(plain_source("PLAIN-SRC"))
        .add_card(own_digimon("ALLY-CIEL", "Sistermon Ciel", &["Puppet"]))
        .memory(13)
        .start();
    runner.place_stack(1, &["OPP-S1", "OPP-S2", "OPP-A"]);
    let ally = runner.place_on_field(0, "ALLY-CIEL", Some(0));
    // Gankoomon X stacked on a plain (non-Gankoomon, non-X-Antibody) source.
    let gx = runner.place_stack(0, &["PLAIN-SRC", CARD_ID]);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(gx));
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("OppField select");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("De-Digivolve target resolves");
    runner.auto_resolve().ok();

    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Digimon),
        "no [Gankoomon]/[X Antibody] source → no board-wide immunity granted"
    );
    assert!(
        !runner
            .game
            .permanent_is_unaffected_by_effect(gx, 1, EffectSourceKind::Digimon),
        "Gankoomon X itself is also not immune without a qualifying source"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — [Opponent's Turn] Reboot + Blocker aura (target filter + gating)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: on the opponent's turn, your Sistermon/Huckmon-named or
/// Royal-Knight-trait Digimon gain Reboot AND Blocker.
#[test]
fn bt20_059_opp_turn_grants_reboot_and_blocker_to_matching_allies() {
    let mut runner = builder()
        .add_card(own_digimon("ALLY-SISTER", "Sistermon Ciel", &["Puppet"]))
        .add_card(own_digimon("ALLY-HUCK", "Huckmon", &[]))
        .add_card(own_digimon("ALLY-RK", "Gallantmon", &["Royal Knight"]))
        .memory(13)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let sister = runner.place_on_field(0, "ALLY-SISTER", Some(0));
    let huck = runner.place_on_field(0, "ALLY-HUCK", Some(0));
    let rk = runner.place_on_field(0, "ALLY-RK", Some(0));

    // Make it the opponent's (player 1's) turn.
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    for (label, h) in [("Sistermon", sister), ("Huckmon", huck), ("Royal Knight", rk)] {
        assert!(
            runner.game.has_keyword(h, Keyword::Reboot),
            "{label} ally must gain Reboot on opponent's turn"
        );
        assert!(
            runner.game.has_keyword(h, Keyword::Blocker),
            "{label} ally must gain Blocker on opponent's turn"
        );
    }
}

/// Negative (target filter): a non-matching own Digimon (no Sistermon/Huckmon
/// name, no Royal Knight trait) gains NEITHER keyword.
#[test]
fn bt20_059_opp_turn_does_not_grant_to_non_matching_ally() {
    let mut runner = builder()
        .add_card(own_digimon("ALLY-OTHER", "Agumon", &["Reptile"]))
        .memory(13)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let other = runner.place_on_field(0, "ALLY-OTHER", Some(0));

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(other, Keyword::Reboot),
        "a non-matching own Digimon must not gain Reboot"
    );
    assert!(
        !runner.game.has_keyword(other, Keyword::Blocker),
        "a non-matching own Digimon must not gain Blocker"
    );
}

/// Negative (turn gating): on YOUR turn the aura is inactive — matching allies
/// do NOT have Reboot/Blocker.
#[test]
fn bt20_059_aura_inactive_on_your_turn() {
    let mut runner = builder()
        .add_card(own_digimon("ALLY-RK", "Gallantmon", &["Royal Knight"]))
        .memory(13)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let rk = runner.place_on_field(0, "ALLY-RK", Some(0));

    // Player 0's turn (the controller's own turn).
    runner.game.turn_player_idx = 0;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(rk, Keyword::Reboot),
        "[Opponent's Turn] aura must be inactive on your own turn (Reboot)"
    );
    assert!(
        !runner.game.has_keyword(rk, Keyword::Blocker),
        "[Opponent's Turn] aura must be inactive on your own turn (Blocker)"
    );
}

/// Negative (controller scope): the aura only buffs YOUR Digimon — an opponent
/// Royal Knight is not granted the keywords.
#[test]
fn bt20_059_aura_does_not_grant_to_opponent_digimon() {
    let mut runner = builder()
        .add_card(own_digimon("OPP-RK", "Gallantmon", &["Royal Knight"]))
        .memory(13)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_rk = runner.place_on_field(1, "OPP-RK", Some(0));

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(opp_rk, Keyword::Reboot),
        "an opponent Royal Knight is not 'your Digimon'"
    );
    assert!(
        !runner.game.has_keyword(opp_rk, Keyword::Blocker),
        "an opponent Royal Knight is not 'your Digimon'"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — Inherited [Opponent's Turn] Jesmon GX ESS
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: while the carrier of this card's source is [Jesmon GX], on the
/// opponent's turn ALL of your Digimon (even non-Royal-Knight ones) gain Reboot
/// and Blocker from the inherited effect.
#[test]
fn bt20_059_inherited_grants_keywords_to_all_allies_under_jesmon_gx() {
    let jesmon_gx = {
        let mut c = make_test_card("JESMON-GX", "Jesmon GX");
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Black];
        c.level = Some(7);
        c.dp = Some(15000);
        c.play_cost = 14;
        c
    };
    let mut runner = builder()
        .add_card(jesmon_gx)
        // A plain (non-matching) own Digimon — only the inherited "all your
        // Digimon" grant can reach it.
        .add_card(own_digimon("ALLY-PLAIN", "Agumon", &["Reptile"]))
        .memory(20)
        .start();
    // BT20-059 sits as a source under a Jesmon GX top card.
    let _carrier = runner.place_stack(0, &[CARD_ID, "JESMON-GX"]);
    let plain = runner.place_on_field(0, "ALLY-PLAIN", Some(0));

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(plain, Keyword::Reboot),
        "inherited [Jesmon GX] grants Reboot to ALL your Digimon"
    );
    assert!(
        runner.game.has_keyword(plain, Keyword::Blocker),
        "inherited [Jesmon GX] grants Blocker to ALL your Digimon"
    );
}

/// Negative: the same source under a NON-Jesmon-GX carrier grants nothing to a
/// non-matching ally (the inherited condition fails).
#[test]
fn bt20_059_inherited_inactive_when_carrier_is_not_jesmon_gx() {
    let mut runner = builder()
        .add_card(plain_source("CARRIER-OTHER"))
        .add_card(own_digimon("ALLY-PLAIN", "Agumon", &["Reptile"]))
        .memory(20)
        .start();
    // BT20-059 under a non-Jesmon-GX top card.
    let _carrier = runner.place_stack(0, &[CARD_ID, "CARRIER-OTHER"]);
    let plain = runner.place_on_field(0, "ALLY-PLAIN", Some(0));

    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();

    assert!(
        !runner.game.has_keyword(plain, Keyword::Reboot),
        "inherited ESS must be inactive when the carrier is not [Jesmon GX]"
    );
    assert!(
        !runner.game.has_keyword(plain, Keyword::Blocker),
        "inherited ESS must be inactive when the carrier is not [Jesmon GX]"
    );
}
