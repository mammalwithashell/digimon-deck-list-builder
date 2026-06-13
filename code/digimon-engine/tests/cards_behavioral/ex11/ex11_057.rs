//! EX11-057 Suzune Kazuki — Tamer, Blue/Yellow, Cost 4. Trait: LIBERATOR.
//!
//! # Card text (card image EX11-057 — authoritative; cards.json agrees)
//!
//! ```text
//! [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
//! [On Play] For each of your [Ice-Snow] trait Digimon, trash any 1
//! digivolution card from your opponent's Digimon.
//! [All Turns] When effects trash digivolution cards from your opponent's
//! Digimon, by suspending this Tamer, gain 1 memory.
//! ```
//!
//! # Security effect text
//!
//! ```text
//! Security Effect [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//!
//! DCGO/Assets/Scripts/CardEffect/EX11/Blue/EX11_057.cs:
//! - Clause 1 (OnStartMainPhase): stock
//!   `Gain1MemoryTamerOpponentDigimonEffect` — MANDATORY, opponent must have
//!   a battle-area Digimon (EX8-066 clause-1 idiom).
//! - Clause 2 (OnEnterFieldAnyone, CanTriggerOnPlay): MANDATORY
//!   (isOptional=false). CanActivateCondition = on battle area AND opponent
//!   has a battle-area Digimon. Body: `iceSnowCount` = count of OWN
//!   battle-area Digimon with TopCard.EqualsTraits("Ice-Snow"), snapshotted
//!   ONCE at resolution; if > 0, ONE cross-permanent
//!   `SelectTrashDigivolutionCards(maxCount: iceSnowCount, canNoTrash: false,
//!   isFromOnly1Permanent: false)` — clamped to min(N, available)
//!   (TrashDigivolutionCards.cs `Math.Min(digivolutionCardsSum, maxCount)`).
//! - Clause 3 (OnDigivolutionCardDiscarded): OPTIONAL (isOptional=true), no
//!   [Once Per Turn]. CanUseCondition = on battle area AND
//!   CanTriggerOnTrashDigivolutionCard(host is OPPONENT battle-area Digimon,
//!   cardEffect != null — EFFECT-initiated trashes only). CanActivateCondition
//!   = on field AND CanActivateSuspendCostEffect. Body: suspend self, gain 1
//!   memory. It DOES trigger off this card's own clause 2.
//! - Clause 4 (SecuritySkill): PlaySelfTamerSecurityEffect.
//!
//! # Known divergence (documented, not fixed here)
//!
//! The engine fires `OnDigivolutionCardTrashed` once PER SOURCE CARD trashed;
//! DCGO fires `OnDigivolutionCardDiscarded` once per per-permanent batch
//! (`ITrashDigivolutionCards`). Net effect is identical when the player
//! accepts (the suspend gate blocks further gains either way); on DECLINE the
//! engine re-prompts for later cards of the same batch where DCGO would not.
//!
//! # Coverage
//!
//! - Metadata: tamer kind, cost 4, blue/yellow, LIBERATOR trait.
//! - Structure: 4 triggered clauses; SoMP + on-play mandatory; observer
//!   optional, no OPT; security FaceUp play_from_security.
//! - Clause 1: SoMP +1 with opponent Digimon; no gain without.
//! - Clause 2: 0 Ice-Snow → no prompt; 2 Ice-Snow → exact-2 cross-permanent
//!   pick (mandatory, no PASS); clamp when opponent has fewer sources than N;
//!   non-Ice-Snow own Digimon don't count; no opponent Digimon → skip.
//! - Clause 3: fires when an effect trashes an opponent source (accept =
//!   suspend + 1 memory); decline path; suspended Tamer → no prompt; own-side
//!   trash → no prompt; fires off this card's own clause 2; [All Turns] —
//!   fires on the opponent's turn.
//! - Clause 4: structural shape + no-panic smoke.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "EX11-057";

// ─── Card-data factories ─────────────────────────────────────────────────────

/// Lv.3 Digimon WITH the [Ice-Snow] trait — counts toward the clause-2 N.
fn make_ice_snow_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.traits = vec!["Ice-Snow".to_string()];
    c
}

/// Lv.3 Digimon WITHOUT the [Ice-Snow] trait — must NOT count toward N.
fn make_plain_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn filler_deck() -> Vec<&'static str> {
    vec!["FILLER", "FILLER", "FILLER"]
}

/// Standard behavioral runner: Suzune Kazuki + the synthetic test pool.
fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-057 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(make_ice_snow_lv3("ICE-A"))
        .add_card(make_ice_snow_lv3("ICE-B"))
        .add_card(make_plain_lv3("PLAIN-A"))
        .add_card(make_plain_lv3("PLAIN-B"))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(make_test_card("OPP-SRC-B", "Opp Source B"))
        .add_card(make_test_card("OPP-SRC-C", "Opp Source C"))
        .add_card(make_plain_lv3("OPP-TOP"))
        .add_card(make_plain_lv3("OPP-TOP-B"))
        .add_card(make_plain_lv3("OPP-TOP-C"))
        .add_card(make_test_card("OWN-SRC", "Own Source"))
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(3)
        .start()
}

/// Play EX11-057 from "nowhere" onto P0's field and fire the production
/// play-event bundle (OnPlay + OnEnterFieldAnyone + OnAllyPlayed).
fn play_suzune(runner: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let tamer = runner.place_on_field(0, CARD_ID, None);
    runner.fire_play_event_triggers(0, tamer.index as usize, false, false);
    runner.game.drain_effect_queue();
    tamer
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural / metadata assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex11_057_metadata_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Suzune Kazuki");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4), "play cost 4");
    assert_eq!(card.level, None, "Tamers have no level");
    assert_eq!(card.dp, None, "Tamers have no DP");
    assert_eq!(
        card.color,
        vec![CompiledColor::Blue, CompiledColor::Yellow],
        "blue/yellow dual-color Tamer"
    );
    assert!(
        card.traits.iter().any(|t| t == "LIBERATOR"),
        "must carry the LIBERATOR trait"
    );
}

/// Exactly 4 triggered clauses: SoMP memory gain, the [On Play] source trash,
/// the [All Turns] source-trash observer, and the security self-play.
#[test]
fn ex11_057_has_four_triggered_clauses() {
    let card = compiled(CARD_ID);
    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered, 4,
        "SoMP + on-play + source-trash observer + security = 4 triggered clauses"
    );
}

/// Clause 1 ([Start of Your Main Phase]) is MANDATORY (DCGO isOptional=false,
/// no printed "you may") and not [Once Per Turn].
#[test]
fn ex11_057_clause1_somp_is_mandatory() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::StartOfYourMainPhase) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("start_of_your_main_phase clause must exist");
    assert!(!clause.optional, "SoMP memory gain is mandatory");
    assert!(!clause.once_per_turn, "no printed [Once Per Turn]");
}

/// Clause 2 ([On Play]) is MANDATORY (DCGO isOptional=false — no "you may")
/// and not [Once Per Turn].
#[test]
fn ex11_057_clause2_on_play_is_mandatory() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_play clause must exist");
    assert!(!clause.optional, "the [On Play] trash is mandatory");
    assert!(!clause.once_per_turn, "no printed [Once Per Turn]");
}

/// Clause 3 ([All Turns] observer) is OPTIONAL ("by suspending this Tamer" is
/// a declinable activation cost) on `on_digivolution_card_trashed`, with no
/// [Once Per Turn] — the Tamer suspension is the natural limiter.
#[test]
fn ex11_057_observer_clause_is_optional_no_opt() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDigivolutionCardTrashed) =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("on_digivolution_card_trashed clause must exist");
    assert!(
        clause.optional,
        "\"by suspending this Tamer\" is an optional activation cost"
    );
    assert!(
        !clause.once_per_turn,
        "no printed [Once Per Turn] — do not add one"
    );
}

/// Clause 4 ([Security]) is mandatory, FaceUp scope, plays from security.
#[test]
fn ex11_057_security_clause_is_mandatory_faceup_play_from_security() {
    let card = compiled(CARD_ID);
    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("on_security clause must exist");
    assert!(!clause.optional, "[Security] effects are mandatory");
    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromSecurity)),
        "security clause must contain play_from_security"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 behavioral: [Start of Your Main Phase] +1 memory
// ═══════════════════════════════════════════════════════════════════════════════

/// Opponent has a Digimon at the start of P0's main phase → P0 gains 1 memory.
#[test]
fn ex11_057_somp_gains_memory_when_opponent_has_digimon() {
    let mut runner = base_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "P0 must gain exactly 1 memory when the opponent has a Digimon"
    );
}

/// Opponent has NO Digimon → the gate fails → memory unchanged.
#[test]
fn ex11_057_somp_no_gain_when_opponent_has_no_digimon() {
    let mut runner = base_runner();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "no memory gain when the opponent controls no Digimon"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2 behavioral: [On Play] per-Ice-Snow source trash
// ═══════════════════════════════════════════════════════════════════════════════

/// ZERO own Ice-Snow Digimon → N = 0 → the trash sub-flow is skipped entirely
/// (DCGO `if (iceSnowCount > 0)`): no prompt, nothing trashed.
#[test]
fn ex11_057_on_play_zero_ice_snow_no_prompt() {
    let mut runner = base_runner();
    runner.place_on_field(0, "PLAIN-A", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    play_suzune(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "N = 0 → no source-pick prompt"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        2,
        "the opponent stack is untouched"
    );
}

/// TWO own Ice-Snow Digimon, three opponent sources across two stacks → ONE
/// cross-permanent exact-2 pick (min = max = N = 2, mandatory, no PASS),
/// splittable across stacks. Exactly the 2 chosen cards are trashed.
#[test]
fn ex11_057_on_play_two_ice_snow_exact_two_cross_permanent_pick() {
    use digimon_engine::action::space::{encode_source_select, PASS};

    let mut runner = base_runner();
    runner.place_on_field(0, "ICE-A", Some(0));
    runner.place_on_field(0, "ICE-B", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp_a, "OPP-SRC");
    runner.push_source(opp_a, "OPP-SRC-B");
    let opp_b = runner.place_on_field(1, "OPP-TOP-B", Some(0));
    runner.push_source(opp_b, "OPP-SRC-C");

    play_suzune(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("the [On Play] source pick must install");
    assert_eq!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0,
        },
        "min = max = N = 2 (two own Ice-Snow Digimon)"
    );
    assert_eq!(view.selecting_player, 0, "the controller picks");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "mandatory — no PASS below the resolved min"
    );
    assert!(
        view.valid_action_ids.len() >= 3,
        "all 3 opponent sources across both stacks are legal candidates; got {}",
        view.valid_action_ids.len()
    );

    // Split the pick across BOTH stacks: one source from each.
    let pick_a = encode_source_select(opp_a.index as u16, 0).expect("stack A source action");
    let pick_b = encode_source_select(opp_b.index as u16, 0).expect("stack B source action");
    runner.execute_action(0, pick_a).expect("pick from stack A");
    runner.execute_action(0, pick_b).expect("pick from stack B");
    runner.game.drain_effect_queue();

    // The trashes fire this card's own clause-3 observer (the Tamer is on the
    // field, unsuspended). Decline every observer prompt — this test is about
    // the clause-2 pick.
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        if guard > 4 {
            panic!("observer prompt loop did not terminate");
        }
        runner.decline_optional_trigger().expect("decline observer");
        runner.game.drain_effect_queue();
        guard += 1;
    }

    assert_eq!(
        runner.game.players[1].trash.len(),
        2,
        "exactly 2 digivolution cards trashed (one per Ice-Snow Digimon)"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_a.index as usize]
            .card_sources
            .len(),
        2,
        "stack A lost exactly the 1 chosen source (top + 1 survivor remain)"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp_b.index as usize]
            .card_sources
            .len(),
        1,
        "stack B lost its only source (top remains)"
    );
}

/// DCGO min(N, available) clamp: TWO Ice-Snow Digimon but only ONE opponent
/// source → the pick clamps to an exact-1 selection and still resolves.
#[test]
fn ex11_057_on_play_clamps_to_available_sources() {
    let mut runner = base_runner();
    runner.place_on_field(0, "ICE-A", Some(0));
    runner.place_on_field(0, "ICE-B", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    play_suzune(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("the clamped source pick must install");
    assert_eq!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        },
        "N = 2 clamps to the 1 available candidate (DCGO Math.Min)"
    );
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick the only source");
    runner.game.drain_effect_queue();

    // Decline the observer prompt fired by the trash.
    let mut guard = 0;
    while runner.pending_selection().is_some() {
        if guard > 4 {
            panic!("observer prompt loop did not terminate");
        }
        runner.decline_optional_trigger().expect("decline observer");
        runner.game.drain_effect_queue();
        guard += 1;
    }

    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "the single available source is trashed"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        1,
        "only the top card remains"
    );
}

/// Non-Ice-Snow own Digimon must NOT count toward N: 1 Ice-Snow + 2 plain →
/// an exact-1 pick.
#[test]
fn ex11_057_on_play_non_ice_snow_digimon_do_not_count() {
    let mut runner = base_runner();
    runner.place_on_field(0, "ICE-A", Some(0));
    runner.place_on_field(0, "PLAIN-A", Some(0));
    runner.place_on_field(0, "PLAIN-B", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");
    runner.push_source(opp, "OPP-SRC-B");

    play_suzune(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("the source pick must install");
    assert_eq!(
        view.kind,
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        },
        "only the 1 Ice-Snow Digimon counts toward N"
    );
}

/// No opponent battle-area Digimon → the DCGO CanActivateCondition gate fails
/// and the effect is skipped entirely — even when an opponent TAMER carries a
/// digivolution source (DCGO restricts hosts to Digimon; the clause-level
/// gate covers the exotic tamer-with-sources board).
#[test]
fn ex11_057_on_play_skips_without_opponent_digimon() {
    let mut opp_tamer = make_test_card("OPP-TAMER", "Opp Tamer");
    opp_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX11-057 YAML parses and compiles")
        .add_card(make_filler("FILLER"))
        .add_card(make_ice_snow_lv3("ICE-A"))
        .add_card(make_test_card("OPP-SRC", "Opp Source"))
        .add_card(opp_tamer)
        .deck(0, &filler_deck())
        .deck(1, &filler_deck())
        .memory(3)
        .start();

    runner.place_on_field(0, "ICE-A", Some(0));
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.push_source(tamer, "OPP-SRC");

    play_suzune(&mut runner);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent DIGIMON → the [On Play] clause is gated off"
    );
    assert!(runner.game.players[1].trash.is_empty(), "nothing trashed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3 behavioral: [All Turns] suspend → gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════

/// An effect trashes a digivolution card from an OPPONENT Digimon → the
/// optional pre-cost prompt installs; accepting suspends the Tamer and gains
/// 1 memory.
#[test]
fn ex11_057_observer_accept_suspends_and_gains_memory() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");
    runner.game.memory = 0;

    runner.trash_one_source(opp);

    let view = runner
        .pending_selection_view()
        .expect("the optional observer prompt must install");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional, "PASS must be able to decline");
    assert_eq!(view.selecting_player, 0, "P0 owns the trigger");
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the suspend cost must not be paid before accept"
    );

    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer must be suspended after accepting (cost paid)"
    );
    assert_eq!(runner.memory(), 1, "P0 gains exactly 1 memory");
}

/// Declining leaves the Tamer unsuspended and gains no memory.
#[test]
fn ex11_057_observer_decline_no_suspend_no_memory() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");
    runner.game.memory = 0;

    runner.trash_one_source(opp);

    assert!(
        runner.pending_selection().is_some(),
        "the optional prompt must install before the decline"
    );
    runner.decline_optional_trigger().expect("decline");
    runner.game.drain_effect_queue();

    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "decline → the Tamer must NOT be suspended"
    );
    assert_eq!(runner.memory(), 0, "decline → no memory gain");
}

/// An already-suspended Tamer cannot pay the suspend cost → no prompt.
#[test]
fn ex11_057_observer_no_prompt_when_tamer_already_suspended() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");
    runner.game.memory = 0;

    runner.trash_one_source(opp);

    assert!(
        runner.pending_selection().is_none(),
        "no prompt — the suspend cost is unpayable on a suspended Tamer"
    );
    assert_eq!(runner.memory(), 0, "no memory gain");
}

/// Trashes from the controller's OWN Digimon must NOT trigger ("from your
/// opponent's Digimon").
#[test]
fn ex11_057_observer_own_side_trash_does_not_trigger() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let own = runner.place_on_field(0, "PLAIN-A", Some(0));
    runner.push_source(own, "OWN-SRC");
    runner.game.memory = 0;

    runner.trash_one_source(own);

    assert!(
        runner.pending_selection().is_none(),
        "no prompt — the trashed-from host is the controller's own Digimon"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer stays unsuspended"
    );
    assert_eq!(runner.memory(), 0, "no memory gain");
}

/// Integration: the observer triggers off THIS CARD'S OWN clause 2. Playing
/// Suzune with 1 Ice-Snow Digimon trashes 1 opponent source, which offers the
/// suspend; accepting yields +1 memory and the suspended Tamer.
#[test]
fn ex11_057_observer_triggers_off_own_on_play_trash() {
    let mut runner = base_runner();
    runner.place_on_field(0, "ICE-A", Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");
    runner.game.memory = 0;

    let tamer = play_suzune(&mut runner);

    // Clause 2: exact-1 source pick (N = 1, 1 available).
    let view = runner
        .pending_selection_view()
        .expect("the [On Play] source pick must install");
    assert!(matches!(
        view.kind,
        SelectionKind::SourceMulti { min: 1, max: 1, .. }
    ));
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick the opponent source");
    runner.game.drain_effect_queue();

    // The trash fires clause 3 — accept the suspend.
    let view = runner
        .pending_selection_view()
        .expect("the clause-3 observer must fire off this card's own trash");
    assert_eq!(view.kind, SelectionKind::TriggerOrder);
    assert!(view.is_optional);
    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer suspends"
    );
    assert_eq!(runner.memory(), 1, "+1 memory from the observer");
    assert_eq!(
        runner.game.players[1].trash.len(),
        1,
        "the opponent source was trashed by clause 2"
    );
}

/// [All Turns]: the observer also fires on the OPPONENT'S turn. P0's gain
/// moves the turn-player-perspective memory gauge negative.
#[test]
fn ex11_057_observer_fires_on_opponents_turn() {
    let mut runner = base_runner();
    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-TOP", Some(0));
    runner.push_source(opp, "OPP-SRC");

    // Hand the turn to player 1 — it is now the opponent's turn. Keep the
    // gauge deep on P1's side: P0's +1 gain must NOT cross 0, or the turn
    // would rotate back to P0 (whose unsuspend phase clears the Tamer again).
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.turn_player(), 1, "must be P1's turn");
    runner.game.memory = 5;

    runner.trash_one_source(opp);

    let view = runner
        .pending_selection_view()
        .expect("[All Turns] — the prompt must install on the opponent's turn too");
    assert_eq!(view.selecting_player, 0, "P0 owns the optional trigger");
    runner.accept_optional_trigger().expect("accept");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "the Tamer suspends on the opponent's turn as well"
    );
    assert_eq!(
        runner.game.memory, 4,
        "P0's +1 gain on P1's turn moves the turn-player-perspective gauge 5 → 4"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause 4: [Security] smoke
// ═══════════════════════════════════════════════════════════════════════════════

/// Pushing EX11-057 into P0's security and enqueueing the security timing does
/// not panic — trigger registration is sound. (Full security-play behavior is
/// exercised end-to-end by sibling Tamer suites sharing `play_from_security`.)
#[test]
fn ex11_057_security_clause_fires_without_panic() {
    let mut runner = base_runner();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == CARD_ID)
        .expect("EX11-057 in card data");
    let next_idx = runner.game.next_card_index();
    let card = digimon_engine::card_source::CardSource::new(data_idx, 0, next_idx);
    runner.game.players[0].security.push(card);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::PlayerBattleArea(0),
    );
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
}
