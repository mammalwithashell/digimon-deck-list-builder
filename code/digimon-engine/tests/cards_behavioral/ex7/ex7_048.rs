//! EX7-048 Gundramon — Digimon, Lv.6, Black, DP 12000, Cost 12.
//! Traits: Machine / Three Musketeers. Form: Mega. Attribute: Virus.
//!
//! # Card text (data/card_bundles/EX7-048.md — official Bandai DB, verbatim;
//! cross-checked against the card image EX7-048.webp)
//!
//! <Blocker> [On Play] [When Digivolving] Reveal the top 6 cards of your
//! deck. You may use 1 [Three Musketeers] trait Option card among them
//! without paying the cost. Return the rest to the top or bottom of the
//! deck. [All Turns] When any of your [Three Musketeers] trait Digimon would
//! leave the battle area other than by your effects, by trashing 1 Option
//! card from this Digimon's digivolution cards, they don't leave.
//!
//! Digivolution requirements (official Bandai DB):
//!   - Standard circle: Black Lv.5 / cost 4
//!   - xros_req: "[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4"
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Black/EX7_048.cs
//!
//! # DCGO crosscheck
//! - Alt-digivolve: AddSelfDigivolutionRequirementStaticEffect gated on
//!   `TopCard.HasText("Three Musketeers")` (IN TEXT, not trait — modelled
//!   with `in_text_contains`), cost 4, level 5,
//!   ignoreDigivolutionRequirement: false.
//! - <Blocker>: BlockerSelfStaticEffect(isInheritedEffect: false) — face-up
//!   only, no inherited reprint.
//! - Shared OP/WD (SimplifiedRevealDeckTopCardsAndSelect): reveal 6, ONE
//!   Custom bucket filtering `IsOption && !CanNotPlayThisOption &&
//!   ContainsTraits("Three Musketeers")`, canNoSelect: true; pick routed
//!   through PlayOptionCards(payCost: false, root: Library) — a USE;
//!   remainder RemainingCardsPlace.DeckTopOrBottom (single player-elected
//!   choice for the whole leftover pool, fires on take AND decline).
//! - [All Turns] (WhenRemoveField): CanUse = IsExistOnBattleAreaDigimon +
//!   CanTriggerWhenPermanentRemoveField(own battle-area DIGIMON w/ Three
//!   Musketeers trait) + !IsByEffect(own effect); CanActivate requires >=1
//!   Option among this Digimon's DigivolutionCards; body = pick the leaving
//!   Digimon → pick 1 Option digivolution card → trash it → cancel the
//!   leave. NO "other" qualifier — Gundramon can protect ITSELF.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - A3/E2: reveal 6 + optional single bucket pick + mandatory remainder
//! - §3.1 StackPosition::Choice remainder (BT24-063 idiom)
//! - Option USE from the reveal pool (`use_option_from_revealed` via the
//!   CardList bucket binding — the widened-executor regression, end-to-end
//!   with the REAL [Three Musketeers] Option EX7-070 Der Blitz)
//! - F/14.1: optional cost-paying leave replacement (protect-by-trashing-
//!   Option, tests/dsl/protect_others_leave_replacement.rs vocabulary)
//! - H: <Blocker> face-up-only grant; two digivolve alt-paths

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause,
    CompiledStackPosition, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX7-048";
/// EX7-070 Der Blitz — a REAL [Three Musketeers] trait Option from this set
/// (anti-pattern 6: prefer a real DSL card). Its [Main]: delete 1 opponent
/// Digimon with the lowest play cost; then place itself as the bottom
/// digivolution card of 1 of your [Three Musketeers] Digimon.
const TM_OPTION: &str = "EX7-070";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// An own [Three Musketeers] Digimon — replacement subject fixture, and the
/// kind-gate negative for the reveal bucket (trait matches, kind is Digimon).
fn tm_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["Three Musketeers".to_string()];
    card
}

/// A [Three Musketeers] trait TAMER — the replacement protects DIGIMON only
/// (printed "your [Three Musketeers] trait Digimon"; DCGO
/// IsPermanentExistsOnOwnerBattleAreaDigimon), so this must NOT be protected.
fn tm_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["Three Musketeers".to_string()];
    card
}

/// A plain Option with NO [Three Musketeers] trait — bucket trait-negative,
/// and the effect-free digivolution-source Option trashed as the replacement
/// cost (the cost pick filters IsOption, any trait).
fn plain_option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Black];
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

/// An own Digimon with no [Three Musketeers] trait — replacement subject
/// negative.
fn off_trait_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Black];
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = vec!["OffTrait".to_string()];
    card
}

/// An opponent Digimon — the delete target for EX7-070's [Main].
fn opp_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 3;
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn gundramon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX7-048 YAML loads")
        .dsl_card(TM_OPTION)
        .expect("EX7-070 YAML loads")
}

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn deck_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .deck
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

fn trash_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card_id)
}

/// Six-card deck with `top` as the top card (last in the array → reveal
/// index 0) and 5 fillers below it.
fn deck_of_six<'a>(top: &'a str) -> [&'a str; 6] {
    ["FILL-5", "FILL-4", "FILL-3", "FILL-2", "FILL-1", top]
}

fn with_fillers(builder: DebugRunnerBuilder) -> DebugRunnerBuilder {
    builder
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .add_card(make_filler("FILL-3"))
        .add_card(make_filler("FILL-4"))
        .add_card(make_filler("FILL-5"))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex7_048_yaml_compiles_and_has_printed_metadata() {
    let runner = gundramon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("EX7-048 present in embedded DSL pack");
    assert_eq!(compiled.card, "EX7-048");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.cost, Some(12));
    assert_eq!(compiled.dp, Some(12000));
    for trait_name in ["Machine", "Three Musketeers"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

/// Two digivolve alt-paths, both cost 4: the standard printed Lv.5 Black
/// circle (G-DSL-FIXTURE-EVO-COSTS) and the special "Lv.5 w/[Three
/// Musketeers] in text" xros_req.
#[test]
fn ex7_048_has_two_digivolve_alt_paths_at_cost_4() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let digivolve_cost4: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(4))
        })
        .collect();
    assert_eq!(
        digivolve_cost4.len(),
        2,
        "standard Lv.5 Black + special Lv.5 w/[Three Musketeers]-in-text, both cost 4; alt_paths: {:?}",
        compiled.alt_paths
    );
}

/// <Blocker> is declared exactly once (face-up only — DCGO passes
/// isInheritedEffect: false and there is no separate inherited line).
#[test]
fn ex7_048_declares_blocker_once() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let blocker_grants = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                    keyword,
                    ..
                }) if keyword == "Blocker"
            )
        })
        .count();
    assert_eq!(blocker_grants, 1, "exactly one <Blocker> grant (face-up)");
}

/// Exactly one shared [On Play]/[When Digivolving] triggered clause; the
/// clause itself is mandatory (the reveal + remainder placement always
/// resolve — only the Option-use pick is optional) and has no printed OPT.
#[test]
fn ex7_048_has_shared_on_play_when_digivolving_clause() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        shared.len(),
        1,
        "exactly one shared On Play/When Digivolving clause"
    );
    assert!(
        !shared[0].optional,
        "clause itself is mandatory (reveal + remainder always resolve)"
    );
    assert!(
        !shared[0].once_per_turn,
        "no [Once Per Turn] is printed on this clause"
    );
}

/// The shared clause reveals the top 6, offers exactly one optional (min:0,
/// max:1) bucket, USES the pick (not plays-as-permanent), and places the
/// remainder with a player-elected top/bottom choice.
#[test]
fn ex7_048_shared_clause_reveals_6_and_uses_option_with_remainder_choice() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared clause");

    assert!(
        shared
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 6, .. })),
        "must reveal top 6"
    );
    let bucket_ok = shared.process.iter().any(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1
                && buckets[0].min == 0
                && buckets[0].max == 1)
    });
    assert!(bucket_ok, "must offer exactly one optional single-pick bucket");
    assert!(
        shared
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::UseOptionFromRevealed { .. })),
        "the pick must be a USE of the Option (DCGO PlayOptionCards payCost: false), not a play"
    );
    assert!(
        shared.process.iter().any(|s| matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Choice,
                ..
            }
        )),
        "remainder placement must be a player-elected top/bottom choice"
    );
}

/// The remainder placement must NOT be a fixed top or fixed bottom — the
/// printed text says "top OR bottom" (a single player-elected choice).
#[test]
fn ex7_048_remainder_is_not_fixed_position() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared clause");
    let fixed = shared.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Top,
                ..
            } | CompiledStep::PlaceRemainderOnDeck {
                position: CompiledStackPosition::Bottom,
                ..
            }
        )
    });
    assert!(!fixed, "remainder must not be hard-coded to a fixed end");
}

/// The [All Turns] protect clause is an OPTIONAL replacement on
/// when_would_leave_battle_area.
#[test]
fn ex7_048_has_optional_leave_replacement() {
    let runner = gundramon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let replacement = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement {
                trigger,
                optional,
                ..
            }) => Some((trigger.clone(), *optional)),
            _ => None,
        })
        .expect("must declare a replacement clause");
    assert_eq!(replacement.0, "when_would_leave_battle_area");
    assert!(replacement.1, "\"by trashing ... they don't leave\" is optional");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — <Blocker> (face-up only)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex7_048_carrier_has_blocker_on_field() {
    let mut runner = gundramon_runner().memory(10).start();
    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(gundramon, Keyword::Blocker),
        "Gundramon must carry <Blocker> while on the field"
    );
}

/// Negative: <Blocker> is NOT an inherited effect — a Digimon with EX7-048
/// as a digivolution SOURCE does not gain it.
#[test]
fn ex7_048_blocker_is_not_inherited() {
    let mut runner = gundramon_runner()
        .add_card(off_trait_digimon("CARRIER"))
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(carrier, Keyword::Blocker),
        "no inherited <Blocker> line is printed — a carrier stacked over EX7-048 must not gain it"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play][When Digivolving] reveal 6 / use 1 free / remainder
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] reveals the top 6; the [Three Musketeers] Option EX7-070 at
/// reveal index 0 is a legal pick via an OPTIONAL RevealBucket selection.
#[test]
fn ex7_048_on_play_installs_optional_reveal_bucket() {
    let mut runner = with_fillers(gundramon_runner())
        .deck(0, &deck_of_six(TM_OPTION))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert!(runner.pending_is_optional(), "'You may use' → optional");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the [Three Musketeers] Option at reveal index 0 must be a legal pick"
    );
}

/// THE payoff (reviewer-demanded end-to-end): [On Play], pick the real
/// [Three Musketeers] Option EX7-070 Der Blitz via the bucket, and prove its
/// effect ACTUALLY RESOLVED — the opponent's Digimon is deleted (board
/// impact), the used Option lands in its printed destination (the bottom
/// digivolution card of an own [Three Musketeers] Digimon — NOT silently
/// back to deck), no memory is paid, and the mandatory remainder top/bottom
/// choice still fires for the other 5 cards.
#[test]
fn ex7_048_on_play_pick_uses_tm_option_free_end_to_end() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(opp_digimon("OPP-3"))
        .deck(0, &deck_of_six(TM_OPTION))
        .memory(0) // cannot afford Der Blitz's use cost 6 — the use must be FREE
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-3", Some(0));
    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();
    let trash_before = runner.trash_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    // 1. Optional bucket pick — take Der Blitz (reveal index 0).
    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick Der Blitz");
    runner.game.drain_effect_queue();

    // 2. Der Blitz [Main] 3a: mandatory pick of the lowest-play-cost
    //    opponent Digimon (single candidate) → deleted.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Der Blitz's delete-target pick installs — its [Main] body is resolving");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner
        .game
        .resolve_selection(sp, aid)
        .expect("delete the opponent Digimon");

    // 3. Der Blitz [Main] 3b: mandatory pick of an own [Three Musketeers]
    //    Digimon to receive Der Blitz as its bottom digivolution card
    //    (single candidate — Gundramon itself).
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Der Blitz's place-under pick installs");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner
        .game
        .resolve_selection(sp, aid)
        .expect("place Der Blitz under Gundramon");
    runner.game.drain_effect_queue();

    // 4. Gundramon's mandatory remainder placement: a single top/bottom
    //    choice for the whole 5-card leftover pool.
    let view = runner
        .pending_selection_view()
        .expect("remainder top/bottom choice must install after the use resolves");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let labels: Vec<&str> = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert_eq!(labels, vec!["Top of deck", "Bottom of deck"]);
    runner.execute_branch(0).expect("choose top");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    // ── The Option's effect ACTUALLY RESOLVED ──
    assert!(
        !battle_area_contains(&runner, 1, "OPP-3"),
        "Der Blitz's [Main] deleted the opponent's Digimon (board impact happened)"
    );
    assert!(
        trash_contains(&runner, 1, "OPP-3"),
        "the deleted opponent Digimon went to its owner's trash"
    );
    // The used Option went to ITS printed destination — the bottom
    // digivolution card of the chosen [Three Musketeers] Digimon.
    let gundramon_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Gundramon still on field");
    assert_eq!(
        gundramon_perm
            .card_sources
            .first()
            .map(|c| c.card_id(&runner.game.card_data)),
        Some(TM_OPTION),
        "Der Blitz became Gundramon's BOTTOM digivolution card"
    );
    assert!(
        !deck_contains(&runner, 0, TM_OPTION),
        "the used Option did NOT silently return to the deck"
    );
    assert!(
        !trash_contains(&runner, 0, TM_OPTION),
        "the used Option is under Gundramon, not in trash (its own placement tail)"
    );
    assert!(
        runner.game.revealed_cards.is_empty(),
        "the reveal pool fully drained"
    );
    // ── Free use + mandatory remainder ──
    assert_eq!(
        runner.memory(),
        memory_before,
        "the use paid NO cost (Der Blitz's printed use cost is 6)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "only the used Option left the deck; the other 5 returned to the top"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "no own card was trashed — the remainder RETURNS to the deck"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "the whole flow resolved cleanly"
    );
}

/// [When Digivolving] fires the identical shared body end-to-end (bottom
/// branch for the remainder this time).
#[test]
fn ex7_048_when_digivolving_pick_uses_tm_option_free_end_to_end() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(opp_digimon("OPP-3"))
        .deck(0, &deck_of_six(TM_OPTION))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let _opp = runner.place_on_field(1, "OPP-3", Some(0));
    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();

    fire_timing(&mut runner, EffectTiming::WhenDigivolving, gundramon);

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs on When Digivolving");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick Der Blitz");
    runner.game.drain_effect_queue();

    // Der Blitz 3a (delete target) then 3b (place-under target).
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("delete-target pick installs");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("delete");
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("place-under pick installs");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("place under");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("remainder top/bottom choice installs");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("choose bottom");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        !battle_area_contains(&runner, 1, "OPP-3"),
        "Der Blitz's [Main] resolved off the When Digivolving trigger too"
    );
    assert_eq!(runner.memory(), memory_before, "free use");
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "5 remainder cards returned to the bottom"
    );
    assert!(
        !deck_contains(&runner, 0, TM_OPTION),
        "the used Option did not return to the deck"
    );
}

/// Declining the optional use still resolves the MANDATORY remainder
/// placement — all 6 revealed cards (including the declined Option) return
/// to the deck, nothing is used or trashed, and no memory moves.
#[test]
fn ex7_048_declining_pick_still_places_remainder_top() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(opp_digimon("OPP-3"))
        .deck(0, &deck_of_six(TM_OPTION))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-3", Some(0));
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    let memory_before = runner.memory();

    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);
    let view = runner.pending_selection_view().expect("bucket prompt");
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline the optional use");
    runner.game.drain_effect_queue();

    // The mandatory top/bottom choice installs even on decline.
    let view = runner
        .pending_selection_view()
        .expect("remainder choice installs even when the use is declined");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("choose top");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 6 revealed cards return to the deck on decline"
    );
    assert!(
        deck_contains(&runner, 0, TM_OPTION),
        "the declined Option returns to the deck with the rest"
    );
    assert_eq!(runner.trash_size(0), trash_before, "nothing trashed");
    assert_eq!(runner.memory(), memory_before, "no memory moved");
    assert!(
        battle_area_contains(&runner, 1, "OPP-3"),
        "nothing resolved against the opponent when declined"
    );
}

/// Decline + the BOTTOM branch of the remainder choice.
#[test]
fn ex7_048_declining_pick_still_places_remainder_bottom() {
    let mut runner = with_fillers(gundramon_runner())
        .deck(0, &deck_of_six(TM_OPTION))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);
    runner
        .execute_action(0, PASS)
        .expect("decline the optional use");
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("remainder choice installs");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    runner.execute_branch(1).expect("choose bottom");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 6 revealed cards return to the deck (bottom branch)"
    );
    assert_eq!(runner.trash_size(0), trash_before, "nothing trashed");
}

/// Negative: an Option WITHOUT the [Three Musketeers] trait is not a legal
/// pick (trait gate).
#[test]
fn ex7_048_non_tm_option_not_eligible() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(plain_option("PLAIN-OPT"))
        .deck(0, &deck_of_six("PLAIN-OPT"))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    // With no eligible card the bucket auto-advances straight to the
    // mandatory remainder choice; tolerate either shape (bt24_063 idiom).
    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
                "an off-trait Option must not be a legal pick: {:?}",
                view.valid_action_ids
            );
        } else {
            assert_eq!(
                view.kind,
                SelectionKind::EffectChoice,
                "with no eligible pick, the only other legal prompt is the remainder choice"
            );
        }
    }
}

/// Negative: a [Three Musketeers] trait DIGIMON in the reveal is not a legal
/// pick (kind gate — the printed text says "Option card").
#[test]
fn ex7_048_tm_digimon_not_eligible_kind_gate() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(tm_digimon("TM-DIGI"))
        .deck(0, &deck_of_six("TM-DIGI"))
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
                "a [Three Musketeers] DIGIMON must not be a legal pick: {:?}",
                view.valid_action_ids
            );
        } else {
            assert_eq!(view.kind, SelectionKind::EffectChoice);
        }
    }
}

/// Negative: with NO eligible Option among the 6, the reveal still runs and
/// ALL 6 cards return to the deck via the mandatory remainder choice.
#[test]
fn ex7_048_no_eligible_pick_still_returns_all_six() {
    let mut runner = with_fillers(gundramon_runner())
        .add_card(make_filler("FILL-6"))
        .deck(0, &["FILL-6", "FILL-5", "FILL-4", "FILL-3", "FILL-2", "FILL-1"])
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    let field_before = runner.battle_area_size(0);

    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::RevealBucket { .. }) {
            assert!(
                !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
                "no card should be pickable"
            );
            let _ = runner.execute_action(0, PASS);
            runner.game.drain_effect_queue();
        }
    }
    if let Some(view) = runner.pending_selection_view() {
        assert_eq!(view.kind, SelectionKind::EffectChoice);
        runner.execute_branch(0).expect("choose top");
        runner.game.drain_effect_queue();
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "all 6 revealed cards return to the deck"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "nothing is played or used"
    );
}

/// Negative: a short deck (fewer than 6 cards) resolves without stalling.
#[test]
fn ex7_048_short_deck_terminates_cleanly() {
    let mut runner = with_fillers(gundramon_runner())
        .deck(0, &["FILL-1", TM_OPTION])
        .memory(0)
        .start();

    let gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    fire_timing(&mut runner, EffectTiming::OnPlay, gundramon);

    let mut guard = 0;
    while let Some(view) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 20, "selection loop did not terminate");
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
        runner.game.drain_effect_queue();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [All Turns] protect [Three Musketeers] Digimon by trashing an
// Option from this Digimon's digivolution cards
// ═══════════════════════════════════════════════════════════════════════════════

/// Gundramon stacked over 1 effect-free Option digivolution source, ready to
/// pay the protect cost.
fn place_protected_gundramon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.place_stack(0, &["OPT-UNDER", CARD_ID])
}

fn replacement_runner() -> DebugRunnerBuilder {
    gundramon_runner()
        .add_card(plain_option("OPT-UNDER"))
        .add_card(tm_digimon("SUBJECT-3M"))
        .add_card(tm_tamer("TAMER-3M"))
        .add_card(off_trait_digimon("SUBJECT-OFF"))
}

/// Accept: trash the Option from Gundramon's digivolution cards → the
/// leaving [Three Musketeers] Digimon stays.
#[test]
fn ex7_048_protect_accept_trashes_option_and_saves_subject() {
    let mut runner = replacement_runner().memory(5).start();
    let gundramon = place_protected_gundramon(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    // Outer accept prompt.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("the protect replacement offers an accept prompt");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");

    // Nested cost pick: which Option of Gundramon's digivolution cards.
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("nested Option-digivolution-card pick installs");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("trash the Option");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    assert!(
        trash_contains(&runner, 0, "OPT-UNDER"),
        "the Option digivolution card was trashed as the cost"
    );
    assert!(
        battle_area_contains(&runner, 0, "SUBJECT-3M"),
        "the [Three Musketeers] Digimon is protected (stays)"
    );
    let gundramon_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("Gundramon stays on field");
    assert_eq!(
        gundramon_perm.card_sources.len(),
        1,
        "Gundramon lost its Option source (2 → 1) but remains"
    );
}

/// KEY faithfulness point: the printed text has NO "other" qualifier —
/// Gundramon itself is a [Three Musketeers] Digimon and can protect ITSELF
/// by trashing its own Option digivolution card.
#[test]
fn ex7_048_protects_itself_no_other_gate() {
    let mut runner = replacement_runner().memory(5).start();
    let gundramon = place_protected_gundramon(&mut runner);

    runner
        .game
        .delete_permanent_with_cause(gundramon, ReplacementCause::OpponentEffect);

    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("Gundramon's OWN leave offers the protect prompt (no 'other' gate)");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("nested Option pick");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("trash");

    assert!(
        battle_area_contains(&runner, 0, CARD_ID),
        "Gundramon protected ITSELF and stays on the field"
    );
    assert!(
        trash_contains(&runner, 0, "OPT-UNDER"),
        "its Option digivolution card paid the cost"
    );
}

/// Decline: the subject leaves and the Option source is untouched.
#[test]
fn ex7_048_protect_decline_lets_subject_leave() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    let sp = runner
        .game
        .pending_selection
        .as_ref()
        .expect("accept prompt installs")
        .selecting_player;
    runner.game.resolve_selection(sp, PASS).expect("decline");

    assert!(runner.game.pending_selection.is_none(), "resolved");
    assert!(
        !battle_area_contains(&runner, 0, "SUBJECT-3M"),
        "the subject leaves when the protect is declined"
    );
    assert!(
        !trash_contains(&runner, 0, "OPT-UNDER"),
        "the Option source is untouched on decline"
    );
}

/// "Other than by YOUR effects": an own-effect leave is NOT protectable.
#[test]
fn ex7_048_own_effect_cause_not_offered() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "an own-effect leave does NOT offer the protect replacement"
    );
    assert!(
        !battle_area_contains(&runner, 0, "SUBJECT-3M"),
        "the subject leaves"
    );
}

/// Unpayable: with no Option among Gundramon's digivolution cards the
/// optional cost cannot be paid, so the prompt is not offered.
#[test]
fn ex7_048_unpayable_without_option_source_not_offered() {
    let mut runner = replacement_runner().memory(5).start();
    // Lone Gundramon — NO Option digivolution source.
    let _gundramon = runner.place_on_field(0, CARD_ID, Some(0));
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "an unpayable trash cost is not offered"
    );
    assert!(
        !battle_area_contains(&runner, 0, "SUBJECT-3M"),
        "the subject leaves when Gundramon cannot pay"
    );
}

/// Kind gate: the printed text protects [Three Musketeers] trait DIGIMON —
/// a trait-bearing TAMER leaving is NOT protected (DCGO
/// IsPermanentExistsOnOwnerBattleAreaDigimon).
#[test]
fn ex7_048_tm_tamer_not_protected_kind_gate() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let tamer = runner.place_on_field(0, "TAMER-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(tamer, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "a [Three Musketeers] TAMER is not a protectable subject (Digimon only)"
    );
    assert!(
        !battle_area_contains(&runner, 0, "TAMER-3M"),
        "the Tamer leaves"
    );
}

/// Trait gate: an own Digimon WITHOUT the trait is not protected.
#[test]
fn ex7_048_off_trait_digimon_not_protected() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-OFF", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "an off-trait subject does not match the protect filter"
    );
    assert!(
        !battle_area_contains(&runner, 0, "SUBJECT-OFF"),
        "the off-trait subject leaves"
    );
}

/// Ownership gate: the OPPONENT'S [Three Musketeers] Digimon leaving is not
/// protectable ("any of YOUR ... Digimon").
#[test]
fn ex7_048_opponent_tm_digimon_not_protected() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let opp_subject = runner.place_on_field(1, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(opp_subject, ReplacementCause::Battle);

    assert!(
        runner.game.pending_selection.is_none(),
        "the opponent's [Three Musketeers] Digimon is not a protectable subject"
    );
    assert!(
        !battle_area_contains(&runner, 1, "SUBJECT-3M"),
        "the opponent's Digimon leaves"
    );
}

/// "Other than by your effects" INCLUDES battle: a battle deletion IS
/// protectable.
#[test]
fn ex7_048_battle_cause_is_protected() {
    let mut runner = replacement_runner().memory(5).start();
    place_protected_gundramon(&mut runner);
    let subject = runner.place_on_field(0, "SUBJECT-3M", Some(0));

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::Battle);

    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a battle-cause leave IS protectable (not by your effects)");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("accept");
    let (aid, sp) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("nested Option pick");
        (p.valid_action_ids[0], p.selecting_player)
    };
    runner.game.resolve_selection(sp, aid).expect("trash");

    assert!(
        battle_area_contains(&runner, 0, "SUBJECT-3M"),
        "the battle-leaving subject is protected"
    );
}
