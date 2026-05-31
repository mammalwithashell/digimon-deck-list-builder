//! Cluster C — cost payability at DECLARATION vs after.
//!
//! Questions (see `card-resolution.md`):
//!   Q5  Omnimon (AD1-025) `[Assembly]` is a legal declaration even if the full
//!       cost isn't currently affordable, as long as it can be MADE payable —
//!       judge: YES.  [READY: AD1-025 impl; needs WarGreymon/MetalGarurumon in
//!       trash — AD1-004/AD1-014 are implemented.]
//!   Q26 Dorbickmon (EX3-014) with [DigiXros] targeting WarGreymon; Miraculous
//!       Mega Knight (BT17-095) DNA-evolves mid-resolution, removing the
//!       WarGreymon ⇒ Dorbickmon's cost becomes unpayable — judge: returns to
//!       hand.
//!   Q27 Same board — judge: pays 0 memory.
//!   Q30 (shared with cluster E) MedievalGallantmon (EX8-074) `<Partition>` is
//!       interruptive; cost-reduction lets it suspend Imperialdramon: Dragon
//!       Mode (EX3-063) + Chaosmon: Valdur Arm (BT20-037).
//!
//! Scenarios authored under tasks §5.

#![allow(unused_imports)]

use digimon_engine::action::{build_action_mask, PLAY_HAND_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;

/// Push a card already present in `card_data` onto player `p`'s trash.
fn seed_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} present in card_data"));
    let idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, idx));
}

// ─────────────────────────────────────────────────────────────────────────────
// Q5 — Omnimon (AD1-025) `[Assembly]` declare-then-pay legality
// ─────────────────────────────────────────────────────────────────────────────
//
// Board (card-resolution.md Q5): memory gauge 0 on Player A's side; Player A has
// a WarGreymon and a MetalGarurumon in trash; Player A declares to play Omnimon
// (AD1-025) via `[Assembly]`. JUDGE ANSWER: YES, legal — you may declare a play
// if it is possible to MAKE the cost become payable; declaring `[Assembly]`
// (reduce cost 6, materials WarGreymon × MetalGarurumon) makes it payable.
//
// DCGO: AD1_025.cs:214-255 — `AddAssemblyConditionClass`, elements
// `[WarGreymon]` ×1 + `[MetalGarurumon]` ×1, `reduceCost: 6`.
//
// ── DISCOVERY-WAVE FINDING (2026-05-28) ──────────────────────────────────────
// BLOCKED-DATA at the source-data layer (distinct from Q2's engine-primitive
// block): AD1-025's entry in `data/cards.json` has NO `[Assembly]` keyword — it
// captures only `<Raid>/<Blocker>/<Partition>`, the `[On Play]` body, and
// `xros_req = [DNA Digivolve] … Cost 0`. The real card HAS `[Assembly]`
// (confirmed in DCGO AD1_025.cs:214-255), so this is a card-data INGEST GAP.
// Consequently `cards/ad1/AD1-025.yaml` has only a `dna_digivolve` alt_path and
// no `assembly` alt_path. The `assembly` alt-path KIND itself is supported by
// the DSL (`CompiledAltPathKind::Assembly`; BT18-102 uses it), so once the data
// is corrected this is authorable without an engine change.
//
// ── RESOLVED (2026-05-29, change `fix-ad1-025-assembly-data`) ────────────────
// The un-block sequence completed: (1) `[Assembly]` added to AD1-025 in
// `data/card_overrides.json`; (2) the `assembly` alt_path authored in
// `cards/ad1/AD1-025.yaml` (materials WarGreymon × MetalGarurumon, zones
// [trash], stack_under, reduce cost 6); (3) the engine Assembly executor wired
// (G-ASSEMBLY-PLAY-EXECUTION — eligibility from trash, surfaced per-element
// selection, bottom placement, reduced cost, declare-then-pay mask). Q5 now
// pins as a live mask assertion.

/// Q5 — Omnimon (AD1-025) `[Assembly]` is a legal declaration at memory 0
/// because the reduced cost (15 − 6 = 9) can be MADE payable: with both
/// materials in trash, 0 − 9 = −9 ≥ the −10 floor, so the mask offers the play.
/// Without the assembly reduction the full cost (15) would overdraw past −10
/// and the play would be illegal — confirming the judge's declare-then-pay rule.
#[test]
fn q5_assembly_declaration_legal_when_cost_can_be_made_payable() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-025")
        .expect("AD1-025 Omnimon loads from the embedded pack")
        .dsl_card("AD1-004")
        .expect("AD1-004 WarGreymon loads")
        .dsl_card("AD1-014")
        .expect("AD1-014 MetalGarurumon loads")
        .hand(0, &["AD1-025"])
        .memory(0)
        .start();
    r.skip_mulligan();

    // Player A has a WarGreymon and a MetalGarurumon in trash.
    seed_trash(&mut r, 0, "AD1-004");
    seed_trash(&mut r, 0, "AD1-014");

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 1.0,
        "Q5: declaring Omnimon via [Assembly] is legal at memory 0 — the \
         reduced cost (15 − 6 = 9) can be made payable (judge: YES)"
    );
}

// ── Q26 / Q27 / Q30 — BLOCKED-CARD ───────────────────────────────────────────

/// Q26 — Dorbickmon (EX3-014) [DigiXros] targeting WarGreymon; Miraculous Mega
/// Knight (BT17-095) DNA-evolves mid-resolution, removing WarGreymon ⇒
/// Dorbickmon's cost becomes unpayable. Judge: returns to hand. (Q25 board:
/// AD1-004/AD1-014/AD1-025/BT17-095 implemented; only EX3-014 missing.)
#[test]
#[ignore = "BLOCKED-CARD: needs EX3-014 (Dorbickmon). AD1-004, AD1-014, AD1-025, BT17-095 implemented."]
fn q26_dorbickmon_returns_to_hand_when_cost_unpayable_after_dna_evo() {}

/// Q27 — Same board. Judge: pays 0 memory (cost unpayable ⇒ no payment).
#[test]
#[ignore = "BLOCKED-CARD: needs EX3-014 (Dorbickmon)."]
fn q27_dorbickmon_pays_zero_memory_when_returned_to_hand() {}

/// Q30 (also cluster E) — MedievalGallantmon (EX8-074) `<Partition>` is
/// interruptive; cost-reduction lets it suspend Imperialdramon: Dragon Mode
/// (EX3-063) + Chaosmon: Valdur Arm (BT20-037) (BanchoLeomon BT20-036 not yet in
/// play). Judge: suspend both with cost reduction.
#[test]
#[ignore = "BLOCKED-CARD: needs BT20-037 (Chaosmon: Valdur Arm), BT20-036 (BanchoLeomon), EX3-063 (Imperialdramon: Dragon Mode), BT16-077 (Dinobeemon), EX3-008 (Flamedramon). EX8-074 implemented."]
fn q30_partition_interruptive_suspends_both_with_cost_reduction() {}
