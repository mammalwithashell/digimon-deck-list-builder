//! BT23-027 Angemon — Digimon, Lv.4, Yellow/Blue, DP 5000, Cost 5.
//! Traits: Angel/Hudie/CS. Attribute: Vaccine.
//!
//! # Card text (official Bandai DB bundle — data/card_bundles/BT23-027.md)
//!
//! ```text
//! Special Digivolution Condition:
//!   [Digivolve] [Patamon]/Lv.3 w/[CS] trait: Cost 2
//!
//! Effect:
//!   <Barrier> [On Play] [When Digivolving] <Draw 1>. Then, if it's your
//!   turn, 2 of your Digimon may DNA digivolve into [Shakkoumon] in the
//!   hand.
//!
//! Inherited Effect:
//!   <Barrier> (When this Digimon would be deleted in battle, by trashing
//!   your top security card, it isn't deleted.)
//! ```
//!
//! Standard digivolve cost circle (cards.json evo_costs / card image):
//! Yellow Lv.3 / cost 3. NOTE: the official Bandai DB bundle additionally
//! lists a "Black Lv.3 / cost 3" circle (data/card_official.json
//! digivolve_costs card_color=5), but the card image (BT23-027.webp) shows
//! only ONE digivolve circle (Yellow Lv.3 / cost 3), and BT23-027's actual
//! printed colors are Yellow/Blue (cards.json card_colors=[2,1]) — there is
//! no Black anywhere else on this card. Per CLAUDE.md's image-is-the-
//! tiebreak rule for tiny-print digivolve circles, this is treated as an
//! official-DB scrape anomaly (likely a stray element carried over from an
//! adjacent card's record) and is NOT authored. Only the Yellow Lv.3/3
//! circle is implemented, re-declared explicitly as an `alt_paths` entry
//! (per the BT22-017/BT12-021 convention — `DebugRunner::dsl_card` backfills
//! `evo_costs` purely from authored `alt_paths`, it does not read
//! `data/cards.json`, so the standard circle must be explicit in the YAML
//! for behavioral tests to exercise it even though production also derives
//! it from `cards.json` evo_costs).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Yellow/BT23_027.cs
//!
//! DCGO crosscheck:
//!   - `EffectTiming.None` self-digivolution-requirement static:
//!     `PermanentCondition = TopCard.EqualsCardName("Patamon") ||
//!     (TopCard.HasLevel && TopCard.IsLevel3 && TopCard.HasCSTraits)`,
//!     `digivolutionCost: 2`, `ignoreDigivolutionRequirement: false`.
//!   - `EffectTiming.WhenPermanentWouldBeDeleted` (own, non-inherited):
//!     `BarrierSelfEffect(false, card, null)`.
//!   - `EffectTiming.WhenPermanentWouldBeDeleted` (isInheritedEffect: true):
//!     `BarrierSelfEffect(true, card, null)`.
//!   - OP/WD shared coroutine: `DrawClass(card.Owner, 1, activateClass).Draw()`
//!     then, if `IsOwnerTurn(card)`,
//!     `DNADigivolvePermanentsIntoHandOrTrashCard(CanSelectDNACardCondition,
//!     payCost: true, isHand: true, activateClass)` where
//!     `CanSelectDNACardCondition` requires `cardSource.IsDigimon &&
//!     cardSource.CanPlayJogress(true) && cardSource.EqualsCardName("Shakkoumon")`.
//!     Both On Play and When Digivolving activate classes use
//!     `CanUseCondition = IsExistOnBattleAreaDigimon(card) &&
//!     CanTriggerOnPlay/CanTriggerWhenDigivolving`, `optional: false`
//!     (mandatory outer trigger — no "you may" gates the Draw 1 itself).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - Alt-digivolve: name-OR-(level+trait) combined in one alt_path (any_of)
//! - Own + inherited keyword grant (Barrier), verified via `has_keyword`
//! - Shared [On Play]/[When Digivolving] triggered clause, mandatory Draw 1
//! - Nested `if: { your_turn: true }` gating an effect-initiated
//!   `may_dna_digivolve_now` (G2), full behavioral drive: partner pick
//!   (OwnField) -> target pick (Hand) -> DNA merge completes
//! - Negative: on the opponent's turn, the your_turn-gated DNA offer must
//!   not install (Draw 1 remains unconditional)
//! - Negative: no eligible partner / no eligible Shakkoumon in hand -> DNA
//!   step is a silent no-op (no prompt installs beyond Draw)

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, encode_digivolve, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT23-027";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn make_colored_digimon(
    id: &str,
    name: &str,
    level: u8,
    dp: i32,
    color: digimon_engine::enums::CardColor,
    traits: &[&str],
) -> CardData {
    let mut c = make_digimon(id, name, level, dp, traits);
    c.colors = vec![color];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec![];
    c
}

/// Broad DNA req (any-level-4 pair) so BT23-027's own `target_filter`
/// (name-based: "[Shakkoumon]") is what actually gates eligibility in these
/// tests, matching DCGO's `CanSelectDNACardCondition` which checks only
/// `EqualsCardName("Shakkoumon") && CanPlayJogress(true)` — not a specific
/// color/level pairing beyond what the target card's own DNA box requires.
fn dna_req_lv4() -> DnaRequirement {
    DnaRequirement {
        level: 4,
        card_colors: vec![],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

fn make_shakkoumon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Shakkoumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.dna_costs = vec![DnaCost {
        requirement1: dna_req_lv4(),
        requirement2: dna_req_lv4(),
        memory_cost: 0,
    }];
    c
}

fn make_non_shakkoumon_lv5(id: &str) -> CardData {
    let mut c = make_test_card(id, "MetalGarurumon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.dna_costs = vec![DnaCost {
        requirement1: dna_req_lv4(),
        requirement2: dna_req_lv4(),
        memory_cost: 0,
    }];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-027 YAML parses and compiles")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_digimon("PATAMON-BASE", "Patamon", 2, 500, &["Mammal"]))
        .add_card(make_digimon("LV3-CS-BASE", "Tokomon X", 3, 900, &["CS"]))
        .add_card(make_colored_digimon(
            "YELLOW-LV3",
            "Elecmon",
            3,
            900,
            digimon_engine::enums::CardColor::Yellow,
            &["Beast"],
        ))
        .add_card(make_digimon(
            "OWN-PARTNER",
            "Gatomon",
            4,
            3000,
            &["Beast Man"],
        ))
        .add_card(make_shakkoumon("SHAKKOUMON"))
        .add_card(make_non_shakkoumon_lv5("METALGARURUMON"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

fn hand_action_for_id(runner: &DebugRunner, player: PlayerId, id: &str) -> u16 {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(PLAY_HAND_START + idx as u16)
        })
        .unwrap_or_else(|| panic!("{id} not found in player {player}'s hand"))
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt23_027_yaml_metadata_matches_printed_card() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("BT23-027 compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(5));
    assert_eq!(card.dp, Some(5000));
    assert_eq!(card.color, vec![CompiledColor::Yellow, CompiledColor::Blue]);
    for t in ["Angel", "Hudie", "CS"] {
        assert!(
            card.traits.iter().any(|x| x.eq_ignore_ascii_case(t)),
            "missing trait {t}: {:?}",
            card.traits
        );
    }
}

/// Standard printed circle: Yellow Lv.3 / cost 3 (BT22-017 convention —
/// re-declared explicitly since DebugRunner's evo_costs backfill only reads
/// authored `alt_paths`, not `data/cards.json`).
#[test]
fn bt23_027_has_standard_yellow_lv3_cost3_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_standard = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(3))
            && p.from.as_ref().is_some_and(|from| {
                from.level_eq == Some(3) && from.color_is == Some(CompiledColor::Yellow)
            })
    });
    assert!(
        has_standard,
        "BT23-027 must have the standard Yellow Lv.3 / cost 3 digivolve path; alt_paths={:?}",
        card.alt_paths
    );
}

/// The special digivolution condition: "[Patamon]/Lv.3 w/[CS] trait: Cost 2"
/// is name-is-Patamon OR (level 3 AND CS trait), single alt_path at cost 2.
/// Mirrors the BT22-008 idiom (`any_of: [name_is, all_of[level_eq, trait_has]]`).
#[test]
fn bt23_027_has_patamon_or_lv3_cs_alt_digivolve_cost_2() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let has_special = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(CompiledCost::Literal(2))
            && p.from.as_ref().is_some_and(|from| {
                from.any_of
                    .iter()
                    .any(|leaf| leaf.name_is.as_deref() == Some("Patamon"))
                    && from.any_of.iter().any(|leaf| {
                        leaf.all_of.iter().any(|l| l.level_eq == Some(3))
                            && leaf
                                .all_of
                                .iter()
                                .any(|l| l.trait_has.as_deref() == Some("CS"))
                    })
            })
    });
    assert!(
        has_special,
        "BT23-027 must have a [Patamon]/Lv.3+CS cost-2 alt digivolve path; alt_paths={:?}",
        card.alt_paths
    );
}

#[test]
fn bt23_027_grants_own_and_inherited_barrier() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let barrier_grants: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword.eq_ignore_ascii_case("Barrier") => Some(*scope),
            _ => None,
        })
        .collect();
    assert!(
        barrier_grants.contains(&CompiledScope::FaceUp),
        "own (face-up) Barrier grant expected; got {barrier_grants:?}"
    );
    assert!(
        barrier_grants.contains(&CompiledScope::Inherited),
        "inherited Barrier grant expected; got {barrier_grants:?}"
    );
}

#[test]
fn bt23_027_has_shared_on_play_when_digivolving_clause_mandatory() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
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
        .expect("shared On Play / When Digivolving clause present");
    assert!(
        !clause.optional,
        "no 'you may' gates the Draw 1 itself — outer trigger mandatory"
    );
    assert_eq!(clause.scope, CompiledScope::FaceUp);
}

/// The DNA-digivolve step must be nested under an `if: { your_turn: true }`
/// gate inside the shared clause's process (printed "then, if it's your turn").
#[test]
fn bt23_027_dna_step_is_gated_by_your_turn_condition() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .expect("On Play clause present");

    fn find_dna_under_your_turn_if(steps: &[CompiledStep]) -> bool {
        steps.iter().any(|s| match s {
            CompiledStep::If {
                condition, then, ..
            } => {
                let gated_by_your_turn = format!("{condition:?}").contains("your_turn");
                (gated_by_your_turn
                    && then
                        .iter()
                        .any(|s| matches!(s, CompiledStep::MayDnaDigivolveNow { .. })))
                    || find_dna_under_your_turn_if(then)
            }
            _ => false,
        })
    }
    assert!(
        find_dna_under_your_turn_if(&clause.process),
        "MayDnaDigivolveNow step must be nested under an `if your_turn` gate"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Alt-digivolve behavioral gating (positive/negative per branch)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt23_027_alt_digivolve_from_named_patamon_at_cost_2() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let base_perm = runner.place_on_field(0, "PATAMON-BASE", Some(0));
    let action = encode_digivolve(0, base_perm.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "cost-2 alt path from named Patamon must be action-mask legal"
    );

    let before = runner.memory();
    runner.game.decode_action(action, 0);

    assert_eq!(
        runner.memory(),
        before - 2,
        "cost-2 alt path must be charged"
    );
    assert_eq!(
        runner.game.players[0].battle_area[base_perm.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        CARD_ID
    );
}

#[test]
fn bt23_027_alt_digivolve_from_lv3_cs_trait_at_cost_2() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let base_perm = runner.place_on_field(0, "LV3-CS-BASE", Some(0));
    let action = encode_digivolve(0, base_perm.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "cost-2 alt path from Lv.3 CS-trait base must be action-mask legal"
    );

    let before = runner.memory();
    runner.game.decode_action(action, 0);

    assert_eq!(
        runner.memory(),
        before - 2,
        "cost-2 alt path must be charged"
    );
}

#[test]
fn bt23_027_standard_digivolve_from_yellow_lv3_at_cost_3() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(6).start();
    let base_perm = runner.place_on_field(0, "YELLOW-LV3", Some(0));
    let action = encode_digivolve(0, base_perm.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 1.0,
        "standard Yellow Lv.3 circle must be action-mask legal at cost 3"
    );

    let before = runner.memory();
    runner.game.decode_action(action, 0);

    assert_eq!(
        runner.memory(),
        before - 3,
        "standard path must charge cost 3"
    );
}

#[test]
fn bt23_027_alt_digivolve_rejects_lv3_plain_base_no_route() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-027 loads")
        .add_card(make_filler("DECK-PAD"))
        .add_card(make_digimon(
            "LV3-PLAIN-RED",
            "Tsunomon",
            3,
            900,
            &["Beast"],
        ))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
        .hand(0, &[CARD_ID])
        .memory(6)
        .start();
    let base_perm = runner.place_on_field(0, "LV3-PLAIN-RED", Some(0));
    let action = encode_digivolve(0, base_perm.index as u16);

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[action as usize], 0.0,
        "a Lv.3 base with no CS trait, no Patamon name, and non-Yellow color \
         must not have a legal digivolve route into BT23-027"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Barrier keyword active (own field + inherited)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt23_027_barrier_active_face_up_on_battle_area() {
    let mut runner = base().memory(0).start();
    let face = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(face, Keyword::Barrier));
}

#[test]
fn bt23_027_barrier_active_when_buried_inherited() {
    let mut runner = base()
        .add_card(make_digimon("CARRIER", "Carrier", 5, 7000, &["Beast"]))
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(runner.game.has_keyword(carrier, Keyword::Barrier));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — On Play: mandatory Draw 1, then your-turn DNA offer
// ═══════════════════════════════════════════════════════════════════════════

/// Playing Angemon on your own turn: Draw 1 fires unconditionally.
#[test]
fn bt23_027_on_play_draws_one_card_unconditionally() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("play Angemon");

    // No eligible partner/target present -> DNA step is a silent no-op;
    // only the mandatory Draw should have any effect.
    runner.auto_resolve().ok();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Draw 1 must remove exactly one card from the deck"
    );
}

/// Positive: on your turn, with 2 eligible own Digimon AND a Shakkoumon
/// card in hand, the DNA offer installs the first (anchor) OwnField
/// selection — a free `select_own_permanent` pick per the BT25-011 idiom,
/// matching the printed "2 of your Digimon" (no forced "this Digimon").
#[test]
fn bt23_027_on_play_your_turn_installs_dna_anchor_prompt_when_eligible() {
    let mut runner = base().hand(0, &[CARD_ID, "SHAKKOUMON"]).memory(10).start();
    runner.place_on_field(0, "OWN-PARTNER", Some(0));

    runner.play(0, 0).expect("play Angemon");

    let view = runner
        .pending_selection_view()
        .expect("DNA anchor OwnField selection must install");
    assert_eq!(view.kind, SelectionKind::OwnField);
}

/// Full behavioral drive: anchor pick -> partner pick -> target pick ->
/// DNA merge completes and Shakkoumon lands on the battle area. The
/// played Angemon itself is one legal anchor candidate among the
/// (Angemon, OWN-PARTNER) pair.
#[test]
fn bt23_027_on_play_your_turn_dna_digivolve_completes_into_shakkoumon() {
    let mut runner = base().hand(0, &[CARD_ID, "SHAKKOUMON"]).memory(10).start();
    let partner = runner.place_on_field(0, "OWN-PARTNER", Some(0));

    let angemon_field_idx = runner.play(0, 0).expect("play Angemon");
    let angemon = runner.perm_handle(0, angemon_field_idx);

    let anchor_view = runner
        .pending_selection_view()
        .expect("DNA anchor OwnField selection");
    assert_eq!(anchor_view.kind, SelectionKind::OwnField);
    assert!(
        anchor_view
            .valid_action_ids
            .contains(&encode_permanent(angemon)),
        "the played Angemon must be a legal anchor candidate"
    );
    assert!(
        anchor_view
            .valid_action_ids
            .contains(&encode_permanent(partner)),
        "OWN-PARTNER must also be a legal anchor candidate"
    );
    runner
        .execute_action(0, encode_permanent(angemon))
        .expect("choose Angemon as DNA anchor");

    let partner_view = runner
        .pending_selection_view()
        .expect("DNA partner OwnField selection must install after anchor pick");
    assert_eq!(partner_view.kind, SelectionKind::OwnField);
    assert!(
        partner_view
            .valid_action_ids
            .contains(&encode_permanent(partner)),
        "OWN-PARTNER must be offered as the DNA partner"
    );
    assert!(
        !partner_view
            .valid_action_ids
            .contains(&encode_permanent(angemon)),
        "the chosen anchor must be excluded from its own partner prompt"
    );
    runner
        .execute_action(0, encode_permanent(partner))
        .expect("choose OWN-PARTNER as DNA partner");

    let target_view = runner
        .pending_selection_view()
        .expect("DNA target Hand selection must install after partner pick");
    assert_eq!(target_view.kind, SelectionKind::Hand);
    let shak_action = hand_action_for_id(&runner, 0, "SHAKKOUMON");
    assert!(
        target_view.valid_action_ids.contains(&shak_action),
        "Shakkoumon must be a legal DNA-digivolve target"
    );
    runner
        .execute_action(0, shak_action)
        .expect("choose Shakkoumon as DNA result");
    runner.auto_resolve().ok();

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SHAKKOUMON"),
        "Shakkoumon must land on the battle area after the DNA digivolve"
    );
}

/// Negative: target filter excludes a non-Shakkoumon Lv.5 Digimon in hand
/// even though the DNA cost shape matches — only [Shakkoumon] is offered.
#[test]
fn bt23_027_dna_offer_excludes_non_shakkoumon_hand_card() {
    let mut runner = base()
        .hand(0, &[CARD_ID, "METALGARURUMON"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-PARTNER", Some(0));

    runner.play(0, 0).expect("play Angemon");
    runner.auto_resolve().ok();

    assert!(
        runner.pending_selection().is_none(),
        "with no Shakkoumon-named card in hand, the DNA offer must be a \
         silent no-op (only Draw 1 fires); MetalGarurumon must not \
         satisfy the target filter"
    );
}

/// Negative: no other own Digimon on the field besides the played Angemon
/// -> no eligible partner -> DNA step is a silent no-op.
#[test]
fn bt23_027_dna_offer_skipped_with_no_eligible_partner() {
    let mut runner = base().hand(0, &[CARD_ID, "SHAKKOUMON"]).memory(10).start();

    runner.play(0, 0).expect("play Angemon");
    runner.auto_resolve().ok();

    assert!(
        runner.pending_selection().is_none(),
        "with zero other own Digimon on the field, no partner is \
         eligible and the DNA offer must be a silent no-op"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Negative: DNA offer skipped entirely on opponent's turn
// ═══════════════════════════════════════════════════════════════════════════

/// [When Digivolving] fired while it is NOT the controller's turn: the
/// your_turn-gated DNA offer must not install; Draw 1 (unconditional) still
/// fires. We construct this by ending the turn so it is player 1's turn,
/// then manually enqueuing WhenDigivolving for player 0's Angemon stack
/// (mirrors the BT16-055 idiom of driving a trigger via
/// `enqueue_triggered` + `drain_effect_queue` rather than a real in-turn
/// digivolve action).
#[test]
fn bt23_027_when_digivolving_on_opponents_turn_skips_dna_offer_but_still_draws() {
    let mut runner = base()
        .hand(0, &["SHAKKOUMON"])
        .deck(0, &["DECK-PAD"; 12])
        .memory(10)
        .start();
    let stack = runner.place_stack(0, &["YELLOW-LV3", CARD_ID]);
    runner.place_on_field(0, "OWN-PARTNER", Some(1));
    runner.end_turn(); // now it's player 1's turn

    let deck_before = runner.deck_size(0);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(stack),
    );
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "Draw 1 is unconditional and must fire regardless of whose turn it is"
    );
    assert!(
        runner.pending_selection().is_none(),
        "on the opponent's turn, the your_turn-gated DNA offer must not install"
    );
    assert!(
        !runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "SHAKKOUMON"),
        "no DNA digivolve should have occurred"
    );
}
