//! EX11-017 Skadimon — Digimon, Lv.6, Blue/Yellow, DP 12000, Cost 12.
//! Traits: Ice-Snow, LIBERATOR. Form: Mega. Attribute: Vaccine. Rarity: R.
//! Standard digivolution: Lv.5 blue for 4 memory (printed evo box; cards.json
//! evo_costs [{card_color:1, level:5, memory_cost:4}]).
//! Alt digivolution (printed rule box): "Digivolve Lv.5 w/[Ice-Snow] trait:
//! Cost 3" (cards.json xros_req).
//!
//! # Card text (card image EX11-017.webp — authoritative; cards.json agrees)
//!
//! **Effect:**
//! <Iceclad> (Other than against Security Digimon, compare the number of
//! digivolution cards instead of DP in this Digimon's battles.)
//! <Barrier> (When this Digimon would be deleted in battle, by trashing the
//! top card of your security stack, prevent that deletion.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may play
//! 1 [Suzune Kazuki] or level 4 or lower [Ice-Snow] trait Digimon card from
//! your hand without paying the cost.
//! [All Turns] [Once Per Turn] When other Digimon are played or digivolve,
//! trash any 3 digivolution cards from your opponent's Digimon. Then, 1 of
//! their Digimon with no digivolution cards can't suspend until their turn
//! ends.
//!
//! **Inherited:** (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_017.cs
//!   - Alternate Digivolution Requirement (None):
//!     AddSelfDigivolutionRequirementStaticEffect(permanentCondition:
//!     EqualsTraits("Ice-Snow") && IsLevel5, digivolutionCost: 3,
//!     ignoreDigivolutionRequirement: false) — trait + level gate, NO color.
//!   - Iceclad (None): IcecladSelfStaticEffect(isInheritedEffect: false).
//!   - Barrier (WhenPermanentWouldBeDeleted): BarrierSelfEffect(false).
//!   - Shared OP/WD/WA (three ActivateClass bodies, ALL SetHashString
//!     "EX11_017_OP_WD_WA_Play_Card" → ONE shared once-per-turn counter;
//!     isOptional TRUE — printed "You may"):
//!     ValidCardToPlay: EqualsCardName("Suzune Kazuki") || (IsDigimon &&
//!     EqualsTraits("Ice-Snow") && HasLevel && Level <= 4) — a DISJUNCTIVE
//!     filter (the name branch carries no IsTamer check).
//!     SharedCanActivateCondition gates on a matching hand card existing.
//!     SelectHandEffect(maxCount: 1, canNoSelect: TRUE — decline legal) →
//!     PlayPermanentCards(payCost: false, isTapped: false, activateETB: true).
//!   - All Turns (OnEnterFieldAnyone, ActivateClass, hash
//!     "EX11_017_AT_Trash_3_Stun", isOptional FALSE — mandatory):
//!     CanUseCondition: IsExistOnBattleArea(card) && (CanTriggerOnPermanentPlay
//!     || CanTriggerWhenPermanentDigivolving) with PermanentCondition:
//!     permanent != card.PermanentOfThisCard() && battle-area Digimon —
//!     ANY OWNER ("other Digimon" — no owner restriction), self excluded.
//!     Body: SelectTrashDigivolutionCards(permanentCondition: OPPONENT
//!     battle-area Digimon, maxCount: 3, canNoTrash: FALSE,
//!     isFromOnly1Permanent: FALSE) — MANDATORY cross-permanent pick of
//!     exactly min(3, available). THEN, if a sourceless OPPONENT battle-area
//!     Digimon exists (HasMatchConditionOpponentsPermanent), MANDATORY
//!     SelectPermanentEffect(maxCount: 1, canNoSelect: FALSE) →
//!     CanNotSuspendClass appended to permanent.UntilOwnerTurnEndEffects
//!     (= until the end of the TARGET OWNER's = opponent's turn).
//!
//! # Patterns this test covers
//! - Iceclad + Barrier face-up keyword grants (EX8-028 idiom).
//! - Disjunctive free-play hand filter (name OR kind+trait+level) with a
//!   shared OPT across THREE timings (EX8-028 / AD1-024 shared-hash idiom).
//! - Free play from hand (EX11-015 play_from_hand_free idiom), decline legal.
//! - [All Turns] "other Digimon played or digivolve" observer
//!   (`on_any_digimon_played` + `on_digivolve`, AD1-024 idiom) with
//!   `event_permanent_is_source: false` self-exclusion (BT22-002 idiom).
//! - Cross-permanent mandatory source trash clamped to min(3, available)
//!   (`select_opponent_sources` + EX7-023 availability ladder, >=3 arm).
//! - Post-trash mandatory lock select → CannotSuspend, expiry
//!   end_of_opponents_turn (EX8-023 idiom). Enforcement-dependent assertions
//!   are #[ignore]d (pre-existing CannotSuspend consult-site gap).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{
    CardColor, CardKind, EffectTiming, Expiry, Keyword, ModifierType, PlaySource,
};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX11-017";

// ─── Fixture builders ─────────────────────────────────────────────────────────

/// The [Suzune Kazuki] Tamer — legal via the NAME branch of the free-play
/// filter (DCGO's name check carries no IsTamer gate; the printed card is a
/// Tamer regardless).
fn suzune(id: &str) -> CardData {
    let mut c = make_test_card(id, "Suzune Kazuki");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Blue];
    c
}

/// An [Ice-Snow] trait Digimon of the given level — the trait-branch
/// free-play candidate (legal iff level <= 4).
fn ice_snow_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, &format!("IceSnow Lv{level}"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A non-[Ice-Snow] Lv.4 Digimon — must never be a free-play candidate.
fn plain_digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, &format!("Plain Lv{level}"));
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Beast".to_string()];
    c
}

/// A generic board Digimon (Lv.4).
fn board_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, "Board Digimon");
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

/// A RED Lv.5 [Ice-Snow] Digimon — matches ONLY Skadimon's printed alt evo
/// box ("Lv.5 w/[Ice-Snow] trait: Cost 3"), not the standard Lv.5-blue path.
fn red_ice_snow_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "Red IceSnow Lv5");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Red];
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// A BLUE Lv.4 base for the synthetic evolver below.
fn blue_lv4_base(id: &str) -> CardData {
    let mut c = make_test_card(id, "Blue Lv4 Base");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c.colors = vec![CardColor::Blue];
    c
}

/// A BLUE Lv.5 evolver with an explicit printed evo-cost (blue Lv.4: 0) so
/// the DebugRunner can drive a real digivolve (synthetic-evo-costs idiom —
/// DSL-loaded cards carry empty evo_costs).
fn blue_lv5_evolver(id: &str) -> CardData {
    let mut c = make_test_card(id, "Blue Lv5 Evolver");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 7;
    c.colors = vec![CardColor::Blue];
    c.evo_costs = vec![EvoCost {
        card_color: 1, // blue
        level: 4,
        memory_cost: 0,
    }];
    c
}

fn filler(id: &str) -> CardData {
    make_test_card(id, "Filler")
}

/// Builder pre-loaded with Skadimon + opponent stack material (SRC-0..9 /
/// TOP-0..3 for `place_opp_stacks`).
fn base_builder() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-017 found in embedded DSL pack")
        .add_card(filler("FILL"));
    for i in 0..10 {
        b = b.add_card(source_filler(&format!("SRC-{i}")));
    }
    for i in 0..4 {
        b = b.add_card(board_digimon(&format!("TOP-{i}"), 4000));
    }
    b.deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(10)
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

/// Place opponent (player 1) Digimon stacks with the given SOURCE counts
/// (stack t gets `stacks[t]` digivolution cards under top card `TOP-t`).
fn place_opp_stacks(runner: &mut DebugRunner, stacks: &[usize]) {
    let mut s = 0;
    for (t, &n) in stacks.iter().enumerate() {
        let mut ids: Vec<String> = Vec::with_capacity(n + 1);
        for _ in 0..n {
            ids.push(format!("SRC-{s}"));
            s += 1;
        }
        ids.push(format!("TOP-{t}"));
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        runner.place_stack(1, &refs);
    }
}

/// Fire [On Play] for Skadimon without a full play action.
fn fire_on_play_for(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.fire_on_play(handle.player, handle.index as usize);
}

/// Fire [When Digivolving] for `handle` without a full digivolve action.
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Fire [When Attacking] for `handle` without a full attack flow.
fn fire_when_attacking(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

/// Drive the mandatory min/max=N source pick to completion (always picking
/// the first live candidate; the pickers re-enumerate between picks).
fn pick_n_sources(runner: &mut DebugRunner, n: usize) {
    for i in 0..n {
        let view = runner
            .pending_selection_view()
            .unwrap_or_else(|| panic!("source pick {}/{} must be pending", i + 1, n));
        assert!(
            !view.valid_action_ids.contains(&PASS),
            "PASS must not be exposed before the mandatory minimum is met (pick {}/{})",
            i + 1,
            n
        );
        let action = view.valid_action_ids[0];
        runner
            .execute_action(0, action)
            .unwrap_or_else(|e| panic!("source pick {}/{} failed: {e:?}", i + 1, n));
    }
}

/// True when `handle` carries a modifier entry of `ty` with the given expiry.
fn has_entry_with_expiry(
    runner: &DebugRunner,
    handle: PermanentHandle,
    ty: ModifierType,
    expiry: Expiry,
) -> bool {
    runner
        .game
        .modifiers
        .get(handle, ty)
        .iter()
        .any(|entry| entry.modifier == ty && entry.expiry == expiry)
}

/// Assert the suspend lock landed: CannotSuspend until end of opponent's turn.
fn assert_locked(runner: &DebugRunner, handle: PermanentHandle) {
    assert!(
        runner.game.modifiers.has(handle, ModifierType::CannotSuspend),
        "locked Digimon must hold CannotSuspend"
    );
    assert!(
        has_entry_with_expiry(
            runner,
            handle,
            ModifierType::CannotSuspend,
            Expiry::EndOfOpponentsTurn
        ),
        "CannotSuspend must expire at the end of the opponent's (their) turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// EX11-017 compiles with its printed stats: Lv.6 blue/yellow Digimon, cost
/// 12, DP 12000, traits Ice-Snow + LIBERATOR, the standard Lv.5-blue cost-4
/// evo path, and the printed "Digivolve Lv.5 w/[Ice-Snow] trait: Cost 3" alt
/// path (DCGO gates on EqualsTraits("Ice-Snow") && IsLevel5 — no color).
#[test]
fn ex11_017_compiles_with_printed_stats_and_digivolve_paths() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-017 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-017 in compiled cards");

    assert_eq!(compiled.card, "EX11-017");
    assert_eq!(compiled.name, "Skadimon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(6));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "cards.json card_colors [1, 2] = blue + yellow (multi-color)"
    );
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Ice-Snow", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    // Standard printed evo box: Lv.5 blue for 4 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(4))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.color_is == Some(CompiledColor::Blue)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Blue)))
                })
        }),
        "Skadimon must digivolve from a blue Lv.5 for cost 4 (printed evo box)"
    );

    // Printed alt evo box: Lv.5 with [Ice-Snow] trait for 3 memory.
    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(3))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(5)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(5)))
                        && (from.trait_has.as_deref() == Some("Ice-Snow")
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.trait_has.as_deref() == Some("Ice-Snow")))
                })
        }),
        "Skadimon must digivolve from a Lv.5 [Ice-Snow] Digimon for cost 3 (printed alt evo box)"
    );
}

/// The printed <Iceclad> and <Barrier> keywords are encoded as face-up
/// grant_keyword clauses AND are live on the placed permanent through the
/// canonical `Game::has_keyword` query.
#[test]
fn ex11_017_iceclad_and_barrier_keywords_attached() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-017 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-017 in compiled cards");
    for keyword in ["Iceclad", "Barrier"] {
        assert!(
            compiled.effects.iter().any(|clause| {
                matches!(
                    clause,
                    CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                        keyword: k,
                        scope,
                        ..
                    }) if k == keyword && *scope != CompiledScope::Inherited
                )
            }),
            "EX11-017 must encode <{keyword}> as a face-up grant_keyword clause"
        );
    }

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(skadimon, Keyword::Iceclad),
        "the placed Skadimon must answer has_keyword(Iceclad)"
    );
    assert!(
        runner.game.has_keyword(skadimon, Keyword::Barrier),
        "the placed Skadimon must answer has_keyword(Barrier)"
    );
}

/// The free-play clause is ONE clause registered at all THREE printed timings
/// ([On Play][When Digivolving][When Attacking]) — the engine's per-clause
/// OPT bookkeeping then equals DCGO's shared hash
/// "EX11_017_OP_WD_WA_Play_Card" — optional ("You may"), once-per-turn.
#[test]
fn ex11_017_free_play_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-017 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-017 in compiled cards");

    let triggered = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX11-017 must encode ONE clause at [On Play][When Digivolving][When Attacking]");

    assert_eq!(triggered.scope, CompiledScope::FaceUp);
    assert_eq!(
        triggered.when.len(),
        3,
        "exactly the three printed timings, no extras"
    );
    assert!(triggered.optional, "printed 'You may play 1 …'");
    assert!(
        triggered.once_per_turn,
        "printed [Once Per Turn]; one clause at three timings = DCGO's shared hash"
    );
}

/// The [All Turns] observer clause triggers on BOTH any-Digimon-played and
/// any-digivolve (DCGO CanTriggerOnPermanentPlay ||
/// CanTriggerWhenPermanentDigivolving), is MANDATORY (DCGO isOptional false;
/// no printed "you may"), and once-per-turn.
#[test]
fn ex11_017_observer_clause_structural_shape() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-017 found in embedded DSL pack")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX11-017 in compiled cards");

    let observer = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnAnyDigimonPlayed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("EX11-017 must encode an [All Turns] observer on on_any_digimon_played");

    assert!(
        observer.when.contains(&CompiledTiming::OnDigivolve),
        "the observer must also trigger on digivolves (printed 'played or digivolve')"
    );
    assert_eq!(observer.scope, CompiledScope::FaceUp);
    assert!(
        !observer.optional,
        "[All Turns] trash + lock is mandatory (DCGO isOptional false)"
    );
    assert!(observer.once_per_turn, "printed [Once Per Turn]");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — [OP][WD][WA][OPT] free play: disjunctive filter
// ─────────────────────────────────────────────────────────────────────────────

/// CANDIDATE SET: with hand = [Suzune Kazuki, Ice-Snow Lv.4, Ice-Snow Lv.5,
/// non-Ice-Snow Lv.4], the [On Play] hand pick offers exactly TWO candidates
/// (the disjunction's two legal branches: the name match and the
/// kind+trait+level match); the prompt is optional (printed "You may").
#[test]
fn ex11_017_free_play_on_play_offers_suzune_and_lv4_ice_snow_only() {
    let mut runner = base_builder()
        .add_card(suzune("SUZUNE"))
        .add_card(ice_snow_digimon("ICE-4", 4))
        .add_card(ice_snow_digimon("ICE-5", 5))
        .add_card(plain_digimon("PLAIN-4", 4))
        .start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "SUZUNE");
    put_in_hand(&mut runner, 0, "ICE-4");
    put_in_hand(&mut runner, 0, "ICE-5");
    put_in_hand(&mut runner, 0, "PLAIN-4");

    fire_on_play_for(&mut runner, skadimon);

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must install the optional hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "printed 'You may' — PASS must be legal"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "exactly Suzune Kazuki (name branch) and the Ice-Snow Lv.4 (trait \
         branch) are legal; the Lv.5 exceeds the level cap and the \
         non-Ice-Snow Lv.4 is trait-rejected"
    );
}

/// NAME BRANCH: with only [Suzune Kazuki] in hand, picking it plays the
/// Tamer to the battle area without paying its cost.
#[test]
fn ex11_017_free_play_plays_suzune_free() {
    let mut runner = base_builder().add_card(suzune("SUZUNE")).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "SUZUNE");

    let memory_before = runner.game.memory;
    let field_before = runner.battle_area_size(0);
    fire_on_play_for(&mut runner, skadimon);

    let view = runner
        .pending_selection_view()
        .expect("the hand selection must install");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(view.valid_action_ids.len(), 1, "only Suzune is legal");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play Suzune Kazuki free");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "Suzune Kazuki was played to the battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SUZUNE"),
        "the played permanent is Suzune Kazuki"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the play is free — no memory paid"
    );
}

/// TRAIT BRANCH: with only the Ice-Snow Lv.4 Digimon in hand, picking it
/// plays it without paying its cost.
#[test]
fn ex11_017_free_play_plays_lv4_ice_snow_free() {
    let mut runner = base_builder().add_card(ice_snow_digimon("ICE-4", 4)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "ICE-4");

    let memory_before = runner.game.memory;
    let field_before = runner.battle_area_size(0);
    fire_on_play_for(&mut runner, skadimon);

    let view = runner
        .pending_selection_view()
        .expect("the hand selection must install");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play the Ice-Snow Lv.4 free");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the Ice-Snow Lv.4 was played to the battle area"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ICE-4"),
        "the played permanent is the Ice-Snow Lv.4"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "the play is free — no memory paid"
    );
}

/// LEVEL GATE: an Ice-Snow Lv.5 alone in hand is NOT a candidate — no prompt
/// installs at all (DCGO SharedCanActivateCondition hand-existence gate).
#[test]
fn ex11_017_free_play_rejects_lv5_ice_snow() {
    let mut runner = base_builder().add_card(ice_snow_digimon("ICE-5", 5)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "ICE-5");

    fire_on_play_for(&mut runner, skadimon);

    assert!(
        runner.pending_selection_view().is_none(),
        "a level 5 [Ice-Snow] Digimon exceeds the printed 'level 4 or lower' \
         cap — no hand prompt may install"
    );
}

/// TRAIT GATE: a non-Ice-Snow Lv.4 Digimon alone in hand is NOT a candidate.
#[test]
fn ex11_017_free_play_rejects_non_ice_snow_lv4() {
    let mut runner = base_builder().add_card(plain_digimon("PLAIN-4", 4)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "PLAIN-4");

    fire_on_play_for(&mut runner, skadimon);

    assert!(
        runner.pending_selection_view().is_none(),
        "a Lv.4 Digimon without the [Ice-Snow] trait (and not named \
         [Suzune Kazuki]) is no candidate — no hand prompt may install"
    );
}

/// DECLINE: the free play is optional — PASS at the hand pick plays nothing
/// and the hand is untouched.
#[test]
fn ex11_017_free_play_decline_is_legal() {
    let mut runner = base_builder().add_card(ice_snow_digimon("ICE-4", 4)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "ICE-4");

    let field_before = runner.battle_area_size(0);
    let hand_before = runner.game.players[0].hand.len();
    fire_on_play_for(&mut runner, skadimon);

    assert!(runner.pending_is_optional(), "PASS must be legal");
    runner.execute_action(0, PASS).expect("decline the free play");
    let _ = runner.auto_resolve();

    assert_eq!(runner.battle_area_size(0), field_before, "nothing played");
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "the hand is untouched"
    );
}

/// TIMING [When Digivolving]: the same clause fires at WD.
#[test]
fn ex11_017_free_play_fires_at_when_digivolving() {
    let mut runner = base_builder().add_card(ice_snow_digimon("ICE-4", 4)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_stack(0, &["SRC-9", CARD_ID]);
    put_in_hand(&mut runner, 0, "ICE-4");

    fire_when_digivolving(&mut runner, skadimon);

    let view = runner
        .pending_selection_view()
        .expect("[When Digivolving] must install the optional hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional());
}

/// TIMING [When Attacking]: the same clause fires at WA.
#[test]
fn ex11_017_free_play_fires_at_when_attacking() {
    let mut runner = base_builder().add_card(ice_snow_digimon("ICE-4", 4)).start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "ICE-4");

    fire_when_attacking(&mut runner, skadimon);

    let view = runner
        .pending_selection_view()
        .expect("[When Attacking] must install the optional hand selection");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(runner.pending_is_optional());
}

/// SHARED OPT ACROSS TIMINGS: using the clause at [On Play] locks it for
/// [When Digivolving] AND [When Attacking] in the same turn (DCGO shared
/// hash "EX11_017_OP_WD_WA_Play_Card"); the lock clears on the next own turn.
#[test]
fn ex11_017_free_play_opt_shared_across_op_wd_wa() {
    let mut runner = base_builder()
        .add_card(ice_snow_digimon("ICE-4A", 4))
        .add_card(ice_snow_digimon("ICE-4B", 4))
        .start();
    runner.game.turn_count = 1;

    let skadimon = runner.place_on_field(0, CARD_ID, Some(0));
    put_in_hand(&mut runner, 0, "ICE-4A");
    put_in_hand(&mut runner, 0, "ICE-4B");

    // Use the clause at On Play.
    fire_on_play_for(&mut runner, skadimon);
    let view = runner
        .pending_selection_view()
        .expect("[On Play] hand selection installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("play the first Ice-Snow Lv.4 free");
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ICE-4A"),
        "the On Play activation played ICE-4A"
    );

    // Same turn: WA and WD must both be locked (a candidate remains in hand).
    fire_when_attacking(&mut runner, skadimon);
    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn] is shared across OP/WD/WA — no When Attacking prompt this turn"
    );
    fire_when_digivolving(&mut runner, skadimon);
    assert!(
        runner.pending_selection_view().is_none(),
        "[Once Per Turn] is shared across OP/WD/WA — no When Digivolving prompt this turn"
    );

    // Next own turn: the OPT clears and the clause offers again.
    runner.end_turn();
    runner.end_turn();
    fire_when_attacking(&mut runner, skadimon);
    assert!(
        runner.pending_selection_view().is_some(),
        "the OPT lock clears on the next turn"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — [All Turns][OPT] other-Digimon-played/digivolve observer
// ─────────────────────────────────────────────────────────────────────────────

/// POSITIVE (own play, cross-permanent trash + lock): Skadimon on the field,
/// opponent stacks [2, 1] (total 3 sources). Playing ANOTHER own Digimon
/// fires the observer: a MANDATORY exactly-3 cross-permanent source pick
/// strips both stacks; both Digimon are now sourceless, the MANDATORY lock
/// select offers both, and the chosen one gets CannotSuspend until the end
/// of THEIR (the opponent's) turn.
#[test]
fn ex11_017_observer_own_play_trashes_three_then_locks() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    place_opp_stacks(&mut runner, &[2, 1]);
    let opp_a = PermanentHandle { player: 1, index: 0 };
    let opp_b = PermanentHandle { player: 1, index: 1 };

    // P0 plays another Digimon normally.
    let played = runner.place_on_field(0, "TOP-3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("the observer must install the cross-permanent source selection");
    assert!(
        matches!(
            view.kind,
            SelectionKind::SourceMulti {
                min: 3,
                max: 3,
                picked: 0
            }
        ),
        "expected a mandatory exactly-3 source pick; got {:?}",
        view.kind
    );
    assert!(
        !runner.pending_is_optional(),
        "the trash is mandatory (DCGO canNoTrash: false; no printed 'you may')"
    );
    pick_n_sources(&mut runner, 3);

    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        1,
        "OPP stack A stripped bare"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        1,
        "OPP stack B stripped bare"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        3,
        "all 3 trashed sources land in the opponent's trash"
    );

    // Then: the mandatory lock select over sourceless opponent Digimon.
    let view = runner
        .pending_selection_view()
        .expect("the 'Then' lock selection must install after the trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(
        !runner.pending_is_optional(),
        "the lock selection is mandatory (DCGO canNoSelect: false)"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both (now sourceless) opponent Digimon are legal lock targets"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose the first opponent Digimon as the lock target");
    runner.auto_resolve().expect("finish the observer effect");

    assert_locked(&runner, opp_a);
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_b, ModifierType::CannotSuspend),
        "only the CHOSEN Digimon is locked"
    );
}

/// OWNER SCOPE: the observer fires when the OPPONENT plays a Digimon too
/// (DCGO PermanentCondition has no owner restriction — "other Digimon" spans
/// both players), and the selecting player is Skadimon's controller.
#[test]
fn ex11_017_observer_fires_on_opponent_play() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "must be P1's turn");
    // Give the new turn player real memory (game.memory flips at rotation
    // and is read from the TURN PLAYER's perspective) — otherwise P1's turn
    // auto-ends at the first drain boundary after the effect resolves, and
    // the freshly installed "until their turn ends" lock (correctly)
    // expires with it.
    runner.game.memory = 5;

    // P1 plays a (sourceless) Digimon on their own turn.
    let played = runner.place_on_field(1, "TOP-0", None);
    runner.fire_play_event_triggers(1, played.index as usize, false, false);

    // No opponent sources exist → the trash half silently skips and the lock
    // select installs directly, offering the just-played sourceless Digimon.
    let view = runner
        .pending_selection_view()
        .expect("the observer must fire on the opponent's play");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection")
            .selecting_player,
        0,
        "Skadimon's CONTROLLER picks the lock target"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the just-played opponent Digimon");
    runner.auto_resolve().expect("finish the observer effect");

    assert_locked(&runner, played);
}

/// DIGIVOLVE TRIGGER: the observer also fires when another Digimon
/// DIGIVOLVES (DCGO CanTriggerWhenPermanentDigivolving).
#[test]
fn ex11_017_observer_fires_on_other_digimon_digivolve() {
    let mut runner = base_builder()
        .add_card(blue_lv4_base("BASE-4"))
        .add_card(blue_lv5_evolver("EVO-5"))
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    place_opp_stacks(&mut runner, &[1]);
    let opp_a = PermanentHandle { player: 1, index: 0 };

    let base = runner.place_on_field(0, "BASE-4", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, "EVO-5");

    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "the synthetic evolver must digivolve over the blue Lv.4 base"
    );
    runner.game.drain_effect_queue();

    // Total 1 opponent source → mandatory exactly-1 pick.
    let view = runner
        .pending_selection_view()
        .expect("the observer must fire on another Digimon's digivolve");
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
    pick_n_sources(&mut runner, 1);

    let view = runner
        .pending_selection_view()
        .expect("the lock selection must install after the trash");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the stripped opponent Digimon");
    runner.auto_resolve().expect("finish the observer effect");

    assert_locked(&runner, opp_a);
}

/// SELF-EXCLUSION (play): Skadimon's OWN play must NOT fire the observer
/// (printed "other Digimon"; DCGO permanent != card.PermanentOfThisCard()).
/// The opponent carries trashable sources, so a misfire would install the
/// source selection.
#[test]
fn ex11_017_observer_does_not_fire_on_own_play() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    place_opp_stacks(&mut runner, &[1]);

    let skadimon = runner.place_on_field(0, CARD_ID, None);
    // Full play-event bundle (OnPlay + OnEnterFieldAnyone observers). The
    // hand holds no free-play candidate, so the [On Play] clause stays quiet.
    runner.fire_play_event_triggers(0, skadimon.index as usize, false, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "Skadimon's own play is not 'other Digimon' — the observer must not fire"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        0,
        "no opponent source may be trashed"
    );
}

/// SELF-EXCLUSION (digivolve): Skadimon digivolving (over a red Lv.5
/// [Ice-Snow] base via the printed alt evo box, cost 3) must NOT fire its own
/// observer — and the alt path charges its printed cost.
#[test]
fn ex11_017_observer_does_not_fire_on_own_digivolve() {
    let mut runner = base_builder().add_card(red_ice_snow_lv5("ICEBASE")).start();
    runner.game.turn_count = 1;

    place_opp_stacks(&mut runner, &[1]);

    let base = runner.place_on_field(0, "ICEBASE", Some(0));
    let hand_idx = put_in_hand(&mut runner, 0, CARD_ID);

    let memory_before = runner.game.memory;
    assert!(
        runner
            .game
            .digivolve_from_hand(0, hand_idx, base.index as usize, PlaySource::ByHand),
        "Skadimon must digivolve over the red Lv.5 [Ice-Snow] base via the \
         printed alt evo box"
    );
    assert_eq!(
        runner.game.memory,
        memory_before - 3,
        "the alt path charges its printed cost 3 (the Lv.5-blue path does \
         not match a red base)"
    );
    runner.game.drain_effect_queue();

    // The hand holds no free-play candidate, so the WD clause stays quiet;
    // a pending source selection would prove the observer misfired on self.
    assert!(
        runner.pending_selection_view().is_none(),
        "Skadimon's own digivolve is not 'other Digimon' — the observer must not fire"
    );
    assert_eq!(
        runner.game.players[1].trash.len(),
        0,
        "no opponent source may be trashed"
    );
}

/// [Once Per Turn]: a second qualifying play in the same turn must not fire
/// the observer again.
#[test]
fn ex11_017_observer_once_per_turn() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    // A sourceless opponent Digimon keeps the lock half observable.
    let opp_a = runner.place_on_field(1, "TOP-0", Some(0));

    // First qualifying play: the lock select installs (no sources → no trash).
    let first = runner.place_on_field(0, "TOP-2", None);
    runner.fire_play_event_triggers(0, first.index as usize, false, false);
    let view = runner
        .pending_selection_view()
        .expect("first play must fire the observer");
    assert_eq!(view.kind, SelectionKind::OppField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the opponent Digimon");
    runner.auto_resolve().expect("finish the first activation");
    assert_locked(&runner, opp_a);

    // Second qualifying play, same turn: locked out.
    let second = runner.place_on_field(0, "TOP-3", None);
    runner.fire_play_event_triggers(0, second.index as usize, false, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "second qualifying play in the same turn must be locked by [Once Per Turn]"
    );
}

/// Min-clamp ladder: for every partition of opponent source counts, the
/// installed selection must demand exactly min(total, 3) picks. This
/// exercises every availability-ladder branch (the >=3 / ==2 / ==1 arms and
/// the per-arm partition decompositions).
#[test]
fn ex11_017_observer_trash_clamp_ladder_demands_min_of_total_and_three() {
    // (opponent stack source-counts, expected mandatory pick count)
    let configs: &[(&[usize], u8)] = &[
        // total >= 3 → exactly 3, across all partitions of 3 (and a 4-total
        // board to prove the cap).
        (&[3], 3),
        (&[4], 3),
        (&[2, 1], 3),
        (&[1, 1, 1], 3),
        // total == 2 → exactly 2.
        (&[2], 2),
        (&[1, 1], 2),
        // total == 1 → exactly 1.
        (&[1], 1),
    ];

    for (stacks, expected) in configs {
        let mut runner = base_builder().start();
        runner.game.turn_count = 1;

        runner.place_on_field(0, CARD_ID, Some(0));
        place_opp_stacks(&mut runner, stacks);

        let played = runner.place_on_field(0, "TOP-3", None);
        runner.fire_play_event_triggers(0, played.index as usize, false, false);

        assert_eq!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti {
                min: *expected,
                max: *expected,
                picked: 0
            }),
            "opponent stacks {stacks:?}: expected a mandatory exactly-{expected} pick"
        );
        pick_n_sources(&mut runner, *expected as usize);
        let _ = runner.auto_resolve();
        assert_eq!(
            runner.trash_size(1),
            *expected as usize,
            "opponent stacks {stacks:?}: exactly {expected} sources trashed"
        );
    }
}

/// ZERO SOURCES: the trash half is skipped entirely (no source selection),
/// but the "Then" lock half still runs — DCGO's guard only requires a
/// sourceless opponent Digimon to exist.
#[test]
fn ex11_017_observer_skips_trash_with_no_sources_but_still_locks() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "TOP-0", Some(0));

    let played = runner.place_on_field(0, "TOP-2", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("with no opponent sources, the lock selection installs directly");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "no source pick should appear — the opponent has no digivolution cards"
    );
    assert!(!runner.pending_is_optional(), "mandatory — no PASS");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the sourceless opponent Digimon");
    runner.auto_resolve().expect("finish the observer effect");

    assert_eq!(runner.game.players[1].trash.len(), 0, "nothing was trashed");
    assert_locked(&runner, opp_a);
}

/// LOCK GATE: when every opponent Digimon still has digivolution cards after
/// the trash (single 4-source stack, 3 trashed, 1 remains), the lock
/// selection must NOT install and no modifier lands anywhere.
#[test]
fn ex11_017_observer_lock_skipped_when_no_sourceless_digimon_remains() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    place_opp_stacks(&mut runner, &[4]);
    let opp_a = PermanentHandle { player: 1, index: 0 };

    let played = runner.place_on_field(0, "TOP-3", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);

    let view = runner
        .pending_selection_view()
        .expect("the source selection must install");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 3,
            max: 3,
            picked: 0
        }
    ));
    pick_n_sources(&mut runner, 3);

    assert_eq!(runner.game.players[1].trash.len(), 3, "3 sources trashed");
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        2,
        "one digivolution card remains under the opponent's Digimon"
    );
    assert!(
        runner.pending_selection_view().is_none(),
        "the opponent's Digimon still has a digivolution card — the lock \
         selection must be silently skipped"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotSuspend),
        "no lock may land when no sourceless opponent Digimon exists"
    );
}

/// "Until THEIR turn ends": the lock survives the end of the granting
/// player's turn and expires at the end of the opponent's (the target
/// controller's) turn.
#[test]
fn ex11_017_lock_expires_at_end_of_opponents_turn() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "TOP-0", Some(0));

    let played = runner.place_on_field(0, "TOP-2", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);

    let view = runner.pending_selection_view().expect("lock selection");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the opponent Digimon");
    runner.auto_resolve().expect("finish the observer effect");
    assert_locked(&runner, opp_a);

    // End of the CONTROLLER's turn: the lock must persist.
    runner.game.modifiers.expire_end_of_turn(0);
    assert!(
        runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotSuspend),
        "the lock must survive the granting player's own turn end"
    );

    // End of the OPPONENT's turn: the lock expires.
    runner.game.modifiers.expire_end_of_turn(1);
    assert!(
        !runner
            .game
            .modifiers
            .has(opp_a, ModifierType::CannotSuspend),
        "CannotSuspend expires at the end of the opponent's (their) turn"
    );
}

/// ENFORCEMENT (shared with BT25-026/028, EX9-019, BT20-084, EX8-023,
/// EX7-023): a CannotSuspend-locked Digimon must be unable to declare an
/// attack (general_rule.pdf 11-2-5; consult site in `Game::can_attack*`).
#[test]
fn ex11_017_locked_digimon_cannot_declare_attack() {
    let mut runner = base_builder().start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, CARD_ID, Some(0));
    let opp_a = runner.place_on_field(1, "TOP-0", Some(0));

    let played = runner.place_on_field(0, "TOP-2", None);
    runner.fire_play_event_triggers(0, played.index as usize, false, false);
    let view = runner.pending_selection_view().expect("lock selection");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("lock the opponent Digimon");
    runner.auto_resolve().expect("finish the observer effect");
    assert_locked(&runner, opp_a);

    // The lock lasts "until their turn ends" — the opponent's whole next turn.
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "must be P1's turn");

    let security_before = runner.security_count(0);
    let result = runner.attack_player(opp_a, 0, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        result,
        AttackResult::Invalid,
        "a CannotSuspend Digimon cannot suspend to declare an attack"
    );
    assert_eq!(
        runner.security_count(0),
        security_before,
        "no security check may resolve from the blocked attack"
    );
    assert!(
        !runner.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "the locked Digimon must remain unsuspended"
    );
}
