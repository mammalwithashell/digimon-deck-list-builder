//! EX11-016 PolarBearmon — Digimon, Lv.5, Blue/Yellow, DP 7000, Cost 7.
//! Traits: Ice-Snow, LIBERATOR. Attribute: Vaccine. Rarity: U.
//! Standard digivolution: Lv.4 blue for 4 memory (printed evo box).
//! Alt digivolution (printed evo box): "Digivolve Lv.4 w/[Ice-Snow] trait: Cost 3".
//!
//! # Card text (card image EX11-016 — authoritative; cards.json agrees)
//!
//! **Effect:**
//! <Iceclad> (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//! [On Play] [When Digivolving] Trash any 2 digivolution cards from your
//! opponent's Digimon. Then, you may place 1 of their Digimon with no
//! digivolution cards as the top or bottom security card.
//!
//! **Inherited:**
//! [Your Turn] While your opponent has no Digimon with digivolution cards,
//! this [Ice-Snow] trait Digimon gains <Piercing> (When this Digimon attacks
//! and deletes an opponent's Digimon and survives the battle, it performs any
//! security checks it normally would.) and <Security A. +1> (This Digimon
//! checks 1 additional security card.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_016.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     EqualsTraits("Ice-Snow") && IsLevel4, digivolutionCost: 3).
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - On Play + When Digivolving (OnEnterFieldAnyone, two ActivateClass with
//!     a SHARED coroutine): outer gate = opponent has ≥1 battle-area Digimon.
//!     SelectTrashDigivolutionCards(maxCount: 2, canNoTrash: false,
//!     isFromOnly1Permanent: false) — MANDATORY cross-permanent pick of
//!     exactly min(2, available). THEN, if a sourceless opponent battle-area
//!     Digimon exists AND card.Owner.CanAddSecurity, an OPTIONAL
//!     SelectPermanentEffect (maxCount: 1, canNoSelect: TRUE — printed "you
//!     may") picks 1 such Digimon; if one was picked, a bool selection
//!     ("Security Top" / "Security Bottom", chosen by card.Owner) feeds
//!     IPutSecurityPermanent(selectedPermanent, toTop: position) — which
//!     places the permanent's TOP CARD into topCard.Owner's (= the
//!     OPPONENT's own) security, FACE-DOWN (isFaceup defaults false), and
//!     trashes any remaining digivolution cards (none by the filter).
//!   - Your Turn (None + OnDetermineDoSecurityCheck, isInheritedEffect: true):
//!     ChangeSelfSAttackStaticEffect(+1) and PierceSelfEffect, both gated on
//!     IsOwnerTurn + carrier EqualsTraits("Ice-Snow") + opponent has NO
//!     Digimon with DigivolutionCards.
//!
//! # Patterns this test covers
//! - Iceclad combat semantics (EX8-022 / EX8-023 idiom).
//! - Cross-permanent mandatory source trash clamped to min(2, available)
//!   (`select_opponent_sources` + if/else count ladder — EX8-023 idiom).
//! - OPTIONAL post-trash permanent select ("you may") over sourceless
//!   opponent Digimon, including PASS/decline.
//! - Player-chosen top/bottom security placement via `select_effect_choice`
//!   + two `place_permanent_on_security` branches into the OPPONENT's own
//!   security stack, face-down.
//! - Inherited conditional self-aura granting <Piercing> + Security A. +1
//!   (identical to sibling EX8-023).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX11-016";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// A plain Lv.4 opponent Digimon. DP parameterized so the Iceclad battle tests
/// can prove DP is NOT the comparator.
fn opp_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Opp Digimon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c
}

/// A generic digivolution-source filler card (stacked under tops).
fn source_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, "Source Filler");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

/// A RED Lv.4 Digimon with the [Ice-Snow] trait — matches ONLY PolarBearmon's
/// printed alt evo box ("Lv.4 w/[Ice-Snow] trait: Cost 3"), not the standard
/// Lv.4-blue path, so digivolving over it proves the alt path works and
/// charges its printed cost 3.
fn red_ice_snow_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv4");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier Digimon WITH the [Ice-Snow] trait — EX11-016 sits underneath it
/// as a digivolution source so its inherited [Your Turn] aura applies.
fn ice_snow_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "IceSnow Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A carrier WITHOUT the [Ice-Snow] trait — the inherited aura must stay off.
fn plain_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, "Plain Carrier");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 12;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Push a registered card into a player's hand; returns the hand index.
fn put_in_hand(runner: &mut DebugRunner, player: u8, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("put_in_hand: card {card_id} not registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, card_index));
    runner.game.player(player).hand.len() - 1
}

/// Card id of a security-stack entry.
fn security_card_id(runner: &DebugRunner, player: u8, idx: usize) -> String {
    runner.game.players[player as usize].security[idx]
        .card_id(&runner.game.card_data)
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX11-016 compiles with its printed stats: Lv.5 blue/yellow Digimon, cost 7,
/// DP 7000, traits Ice-Snow + LIBERATOR, the standard Lv.4-blue cost-4 evo
/// path, and the printed "Digivolve Lv.4 w/[Ice-Snow] trait: Cost 3" alt path.
#[test]
fn ex11_016_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-016 in compiled cards");

    assert_eq!(compiled.card, "EX11-016");
    assert_eq!(compiled.name, "PolarBearmon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(5));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    for trait_name in ["Ice-Snow", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.4 blue for 4 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(4)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(4)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "PolarBearmon must digivolve from a blue Lv.4 for cost 4 (printed evo box)"
    );

    // Printed alt evo box: Lv.4 with [Ice-Snow] trait for 3 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(4)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(4)))
                        && (from.trait_has.as_deref() == Some("Ice-Snow")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("Ice-Snow")))
                })
        }),
        "PolarBearmon must digivolve from a Lv.4 [Ice-Snow] Digimon for cost 3 (printed alt evo box)"
    );
}

/// The printed <Iceclad> keyword is encoded as a face-up grant_keyword clause
/// AND is live on the placed permanent via the canonical `Game::has_keyword`
/// query. The [On Play][When Digivolving] clause fires at both timings and is
/// NOT clause-optional (the trash is mandatory; only the inner place is "you
/// may").
#[test]
fn ex11_016_structural_shape() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-016 in compiled cards");
    let has_iceclad_clause = compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Iceclad" && *scope != CompiledScope::Inherited
        )
    });
    assert!(
        has_iceclad_clause,
        "EX11-016 must encode <Iceclad> as a face-up grant_keyword clause"
    );

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX11-016 must encode an [On Play][When Digivolving] clause");
    assert_eq!(
        triggered.scope,
        CompiledScope::FaceUp,
        "the trash-2-sources clause is face-up card text"
    );
    assert!(
        !triggered.optional,
        "the clause itself is mandatory — only the inner place select is 'you may'"
    );
    assert!(
        !triggered.once_per_turn,
        "printed text carries no [Once Per Turn]"
    );

    // Inherited [Your Turn] declarative aura: Piercing + Security A. +1.
    let aura = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                security_attack,
                grant_keyword,
                ..
            }) if *scope == CompiledScope::Inherited => {
                Some((active_when, security_attack, grant_keyword))
            }
            _ => None,
        })
        .expect("EX11-016 must encode an inherited declarative aura");
    let (active_when, security_attack, grant_keyword) = aura;
    assert!(
        active_when.is_some(),
        "the inherited aura must carry an active_when gate ([Your Turn] + \
         opponent-no-sourced-Digimon + Ice-Snow carrier trait)"
    );
    assert_eq!(
        *security_attack,
        Some(1),
        "the inherited aura grants <Security A. +1>"
    );
    assert_eq!(
        grant_keyword.as_ref().map(|g| g.keyword.as_str()),
        Some("Piercing"),
        "the inherited aura grants <Piercing>"
    );

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(bearmon, Keyword::Iceclad),
        "the placed PolarBearmon must answer has_keyword(Iceclad)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — <Iceclad> battle semantics (stack-count compare, not DP)
// ─────────────────────────────────────────────────────────────────────────────

/// Iceclad attacker wins on source count despite LOWER DP: PolarBearmon
/// (7000 DP) with 1 source under it (2 cards total) beats a 10000-DP defender
/// with 1 card.
#[test]
fn ex11_016_iceclad_wins_battle_on_source_count_despite_lower_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(opp_digimon("BIG-DEF", 10000))
        .start();

    let bearmon = runner.place_stack(0, &["SRC-A", CARD_ID]);
    let defender = runner.place_on_field(1, "BIG-DEF", Some(0));

    let result = runner.attack_digimon(bearmon, defender, false);

    assert_eq!(
        result,
        AttackResult::AttackerWins,
        "Iceclad compares stack sizes (2 > 1), not DP (7000 < 10000)"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[1].battle_area.len(), 0);
}

/// Iceclad attacker LOSES with fewer sources despite higher DP: bare
/// PolarBearmon (7000 DP, 1 card) attacks a 1000-DP defender with a 3-card
/// stack.
#[test]
fn ex11_016_iceclad_loses_with_fewer_sources_despite_higher_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("WEAK-DEF", 1000))
        .start();

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    let defender = runner.place_stack(1, &["SRC-A", "SRC-B", "WEAK-DEF"]);

    let result = runner.attack_digimon(bearmon, defender, false);

    assert_eq!(
        result,
        AttackResult::DefenderWins,
        "Iceclad compares stack sizes (1 < 3) — PolarBearmon loses despite 7000 vs 1000 DP"
    );
    assert_eq!(runner.game.players[0].battle_area.len(), 0);
    assert_eq!(runner.game.players[1].battle_area.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [On Play][When Digivolving] trash any 2 + optional security place
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (full pipeline, TOP): the opponent has TWO Digimon, each carrying
/// 1 digivolution card. On Play installs a MANDATORY exactly-2 cross-permanent
/// source pick; both Digimon end up sourceless. Then the OPTIONAL place select
/// offers both; picking one and choosing "top" moves its top card to the TOP
/// of the OPPONENT's security stack, face-down, removing it from the field.
#[test]
fn ex11_016_on_play_trashes_two_sources_then_places_chosen_digimon_top_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .add_card(filler("FILL"))
        .security(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "OPP-A"]);
    let opp_b = runner.place_stack(1, &["SRC-B", "OPP-B"]);
    let opp_security_before = runner.game.players[1].security.len();
    let own_security_before = runner.game.players[0].security.len();

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    // First selection: the cross-permanent source pick, exactly 2, mandatory.
    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the cross-permanent source selection");
    assert!(
        matches!(
            view.kind,
            SelectionKind::SourceMulti {
                min: 2,
                max: 2,
                picked: 0
            }
        ),
        "expected a mandatory exactly-2 source pick; got {:?}",
        view.kind
    );
    assert!(
        !runner.pending_is_optional(),
        "the trash is mandatory (no printed 'you may' on it)"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the first opponent digivolution card");
    let view = runner
        .pending_selection_view()
        .expect("the second pick of the mandatory 2 must still be pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick the second opponent digivolution card");

    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "both trashed sources land in the opponent's trash"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "OPP-A stripped bare"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        1,
        "OPP-B stripped bare"
    );

    // Then: the OPTIONAL place select over sourceless opponent Digimon.
    let view = runner
        .pending_selection_view()
        .expect("the 'Then, you may place' selection must install after the trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        runner.pending_is_optional(),
        "printed 'you may' — the place selection must be optional (PASS exposed)"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both (now sourceless) opponent Digimon are legal place targets"
    );
    // Pick the first candidate (OPP-A — field index 0 enumerates first).
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose OPP-A as the place target");

    // The top/bottom position choice must surface as a player selection.
    let view = runner
        .pending_selection_view()
        .expect("the top/bottom security-position choice must install");
    assert_eq!(
        view.kind,
        SelectionKind::EffectChoice,
        "top-or-bottom is a real player choice (DCGO SetBoolSelection)"
    );
    runner.execute_branch(0).expect("choose 'top' (branch 0)");
    runner.auto_resolve().expect("finish the On Play effect");

    // OPP-A left the field into the OPPONENT's security, on TOP, face-down.
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "OPP-A leaves the battle area (OPP-B remains)"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before + 1,
        "the opponent's security stack grows by exactly 1"
    );
    assert_eq!(
        runner.game.players[0].security.len(),
        own_security_before,
        "the CONTROLLER's security must be untouched — 'their' security receives the card"
    );
    let top = runner.game.players[1]
        .security
        .last()
        .expect("opponent security non-empty");
    assert_eq!(
        top.card_id(&runner.game.card_data),
        "OPP-A",
        "'top' places the chosen Digimon as the TOP security card"
    );
    assert!(
        !runner.game.players[1]
            .face_up_security
            .contains(&top.card_index),
        "the placed security card is face-down (DCGO isFaceup defaults false)"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_b, ModifierType::CannotSuspend),
        "EX11-016 (unlike sibling EX8-023) applies NO suspend/WD lock"
    );
}

/// BOTTOM placement: choosing branch 1 places the Digimon's card at the
/// BOTTOM (index 0) of the opponent's security stack.
#[test]
fn ex11_016_place_bottom_puts_card_at_bottom_of_opponent_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(filler("FILL"))
        .security(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-A", Some(0));
    let opp_security_before = runner.game.players[1].security.len();

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    // No opponent sources → the optional place select installs directly.
    let view = runner
        .pending_selection_view()
        .expect("with no opponent sources, the optional place selection installs directly");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(runner.pending_is_optional(), "printed 'you may'");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose OPP-A as the place target");

    let view = runner
        .pending_selection_view()
        .expect("the top/bottom security-position choice must install");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner
        .execute_branch(1)
        .expect("choose 'bottom' (branch 1)");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_eq!(runner.game.players[1].trash.len(), 0, "nothing was trashed");
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "OPP-A leaves the battle area"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before + 1
    );
    assert_eq!(
        security_card_id(&runner, 1, 0),
        "OPP-A",
        "'bottom' places the chosen Digimon as the BOTTOM security card (index 0)"
    );
}

/// Decline path: PASSing the optional place select ends the effect — the
/// sourceless Digimon stays on the field, no security change, and no
/// top/bottom choice is offered.
#[test]
fn ex11_016_declining_optional_place_leaves_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(filler("FILL"))
        .security(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "OPP-A", Some(0));
    let opp_security_before = runner.game.players[1].security.len();

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("the optional place selection installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(runner.pending_is_optional(), "PASS must be exposed");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional place");
    runner.auto_resolve().expect("nothing further should pend");

    assert!(
        runner.pending_selection().is_none(),
        "declining must not install the top/bottom choice"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "the declined Digimon stays on the field"
    );
    assert_eq!(
        runner.game.players[1].security.len(),
        opp_security_before,
        "the opponent's security is unchanged on decline"
    );
}

/// Up-to clamp (DCGO maxDigivolutionDiscardCount = min(available, 2)): with
/// only ONE digivolution card on the opponent's side, the pick is a mandatory
/// exactly-1 selection; the stripped Digimon then qualifies for the optional
/// place.
#[test]
fn ex11_016_on_play_trash_clamps_to_single_available_source_then_offers_place() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(opp_digimon("OPP-A", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "OPP-A"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the source selection clamped to 1");
    assert!(
        matches!(
            view.kind,
            SelectionKind::SourceMulti {
                min: 1,
                max: 1,
                picked: 0
            }
        ),
        "with 1 available source the pick clamps to a mandatory exactly-1; got {:?}",
        view.kind
    );
    assert!(!runner.pending_is_optional(), "the trash is mandatory");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash the only opponent digivolution card");

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "the single digivolution card was trashed"
    );

    // The freshly-stripped OPP-A is a legal target for the optional place.
    let view = runner
        .pending_selection_view()
        .expect("the optional place selection must install after the clamped trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(runner.pending_is_optional(), "printed 'you may'");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "exactly the stripped OPP-A qualifies"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose OPP-A");
    let view = runner
        .pending_selection_view()
        .expect("the top/bottom choice installs");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("top");
    runner.auto_resolve().expect("finish the On Play effect");

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "OPP-A moved to security"
    );
    assert_eq!(
        runner.game.players[1]
            .security
            .last()
            .expect("opponent security non-empty")
            .card_id(&runner.game.card_data),
        "OPP-A"
    );
}

/// NEGATIVE (place gate): when every opponent Digimon still has digivolution
/// cards after the trash, the optional place select must NOT install — no
/// prompt, no security change.
#[test]
fn ex11_016_no_place_prompt_when_no_sourceless_digimon_remains() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(source_filler("SRC-C"))
        .add_card(source_filler("SRC-D"))
        .add_card(source_filler("SRC-E"))
        .add_card(source_filler("SRC-F"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(opp_digimon("OPP-B", 5000))
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Both opponent stacks carry 3 sources — trashing any 2 cannot strip
    // either below 1 source.
    runner.place_stack(1, &["SRC-A", "SRC-B", "SRC-C", "OPP-A"]);
    runner.place_stack(1, &["SRC-D", "SRC-E", "SRC-F", "OPP-B"]);

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the cross-permanent source selection");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    ));
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("first source pick");
    let view = runner
        .pending_selection_view()
        .expect("second pick pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("second source pick");

    assert_eq!(runner.game.players[1].trash.len(), 2, "2 sources trashed");
    assert!(
        runner.pending_selection().is_none(),
        "every opponent Digimon still has digivolution cards — neither the \
         place select nor the top/bottom choice may install"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        2,
        "both opponent Digimon stay on the field"
    );
}

/// NEGATIVE (clause gate): with NO opponent Digimon at all, nothing happens —
/// no source selection, no place selection (DCGO outer gate requires an
/// opponent battle-area Digimon).
#[test]
fn ex11_016_on_play_with_no_opponent_digimon_does_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let bearmon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, bearmon.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon — neither the trash nor the place selection installs"
    );
}

/// WHEN DIGIVOLVING via the printed alt evo box: PolarBearmon digivolves over
/// a RED Lv.4 [Ice-Snow] base (matching ONLY the alt path) for its printed
/// cost 3, then the [When Digivolving] effect fires the same trash + place
/// pipeline.
#[test]
fn ex11_016_when_digivolving_via_ice_snow_alt_path_costs_3_and_fires_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(red_ice_snow_lv4("ICEBASE"))
        .add_card(source_filler("SRC-A"))
        .add_card(source_filler("SRC-B"))
        .add_card(opp_digimon("OPP-A", 5000))
        .add_card(filler("FILL"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;

    let opp_a = runner.place_stack(1, &["SRC-A", "SRC-B", "OPP-A"]);
    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, CARD_ID);

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "PolarBearmon must digivolve over the red Lv.4 [Ice-Snow] base via \
         the printed alt evo box (Lv.4 w/[Ice-Snow]: cost 3)"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "the alt path charges its printed cost 3"
    );

    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] must install the source selection");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    ));
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("first source pick");
    let view = runner
        .pending_selection_view()
        .expect("second pick pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("second source pick");

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "both of the opponent's digivolution cards are trashed"
    );

    // OPP-A is now sourceless → the optional place select installs.
    let view = runner
        .pending_selection_view()
        .expect("the optional place selection must install after the WD trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(runner.pending_is_optional(), "printed 'you may'");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose OPP-A");
    let view = runner
        .pending_selection_view()
        .expect("the top/bottom choice installs");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("top");
    runner.auto_resolve().expect("finish the WD effect");

    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "OPP-A moved to the opponent's security"
    );
    assert_eq!(
        runner.game.players[1]
            .security
            .last()
            .expect("opponent security non-empty")
            .card_id(&runner.game.card_data),
        "OPP-A"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Inherited [Your Turn] aura: <Piercing> + <Security A. +1>
// ─────────────────────────────────────────────────────────────────────────────

fn aura_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(ice_snow_carrier("ICE-CARRIER"))
        .add_card(plain_carrier("PLAIN-CARRIER"))
        .add_card(opp_digimon("OPP-DEF", 4000))
        .add_card(source_filler("OPP-SRC"))
        .start()
}

/// AURA ON: carrier has [Ice-Snow], it's the controller's turn, and the
/// opponent has no Digimon with digivolution cards → the carrier gains
/// <Piercing> and Security Attack +1.
#[test]
fn ex11_016_inherited_aura_grants_piercing_and_security_attack_plus_1() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // Opponent: a single sourceless Digimon (condition satisfied).
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        r.game.has_keyword(carrier, Keyword::Piercing),
        "carrier must gain <Piercing> while the inherited gate is satisfied"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        1,
        "carrier must gain <Security A. +1> while the inherited gate is satisfied"
    );
}

/// AURA OFF: an opponent Digimon has a digivolution card → no grants.
#[test]
fn ex11_016_inherited_aura_off_when_opponent_has_sourced_digimon() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent while any opponent Digimon has digivolution cards"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent while any opponent Digimon has digivolution cards"
    );
}

/// AURA OFF: the carrier lacks the [Ice-Snow] trait → no grants.
#[test]
fn ex11_016_inherited_aura_off_when_carrier_lacks_ice_snow_trait() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "PLAIN-CARRIER"]);
    r.place_on_field(opp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "<Piercing> must be absent when the carrier lacks the Ice-Snow trait"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "Security A. +1 must be absent when the carrier lacks the Ice-Snow trait"
    );
}

/// OFFICIAL Q&A + full-attack integration (regression for the "Piercing not
/// working" report): the carrier attacks the opponent's ONLY sourced Digimon.
/// At declaration the aura is OFF (that Digimon has a digivolution card). The
/// battle deletes it — at that same timing the opponent no longer has any
/// Digimon with digivolution cards, the [Your Turn] aura turns ON, and per
/// the official Q&A ("Yes, it triggers ... at the same timing as when your
/// opponent's Digimon is deleted in battle") the freshly gained <Piercing>
/// performs the post-battle security check — with <Security A. +1> also
/// live, consuming TWO security cards.
#[test]
fn ex11_016_piercing_gained_mid_battle_triggers_two_security_checks() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-016 found in embedded DSL pack")
        .add_card(ice_snow_carrier("ICE-CARRIER"))
        .add_card(opp_digimon("OPP-DEF", 4000))
        .add_card(source_filler("OPP-SRC"))
        .add_card(filler("FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(0, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let carrier = r.place_stack(tp, &[CARD_ID, "ICE-CARRIER"]);
    // The opponent's ONLY Digimon carries a digivolution card → aura OFF.
    let defender = r.place_stack(opp, &["OPP-SRC", "OPP-DEF"]);

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();
    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "aura must be OFF at attack declaration (the defender is sourced)"
    );

    let sec_before = r.game.players[opp as usize].security.len();
    let _ = r.attack_digimon(carrier, defender, false);

    assert_eq!(
        r.game.players[opp as usize].battle_area.len(),
        0,
        "the sourced defender (4000 DP) is deleted by the 11000-DP carrier"
    );
    assert_eq!(
        r.game.players[opp as usize].security.len(),
        sec_before - 2,
        "mid-battle-gained <Piercing> must fire the post-battle security \
         check, and the simultaneously gained <Security A. +1> makes it \
         check 2 cards (official EX11-016 Q&A: the effect gained at the \
         deletion timing does trigger)"
    );
    assert!(
        r.game.pending_attack.is_none() && r.game.pending_selection.is_none(),
        "the attack must fully resolve without wedging"
    );
    assert!(!r.game_over(), "the game continues");
}

/// AURA OFF: [Your Turn] — on the opponent's turn the grants are absent even
/// with the board condition satisfied.
#[test]
fn ex11_016_inherited_aura_off_on_opponents_turn() {
    let mut r = aura_runner();
    let tp = r.game.turn_player();
    let non_tp = 1 - tp;

    let carrier = r.place_stack(non_tp, &[CARD_ID, "ICE-CARRIER"]);
    r.place_on_field(tp, "OPP-DEF", Some(0));

    r.game.enter_main_phase();
    r.game.tick_declarative_effects();

    assert!(
        !r.game.has_keyword(carrier, Keyword::Piercing),
        "[Your Turn] gate: <Piercing> must be absent on the opponent's turn"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(carrier, ModifierType::SecurityAttackChange),
        0,
        "[Your Turn] gate: Security A. +1 must be absent on the opponent's turn"
    );
}
