//! Cast-time stack-construction for cost reduction — the `cast_time_assembly`
//! alt-path (BT15-102 Apocalymon; docs/RUST_ENGINE_GAPS.md Track E).
//!
//! Printed shape under test (via an inline fixture so the substrate is
//! covered independently of the real card):
//!   "When this card would be played, by placing up to 3 [Dark Masters]
//!    trait cards with different names from your battle area or trash under
//!    it, reduce the play cost by 4 for each one."
//!
//! DCGO behavioral anchors (BT15_102.cs):
//! - the placement multi-select surfaces at play time and is optional-zero
//!   (`canNoSelect` / "Do not place") — but ONLY while the remaining reduced
//!   cost is payable (`PayingCost - 4*placed <= MaxMemoryCost`);
//! - name-uniqueness is enforced against the CHOSEN set at mask level
//!   (`CanTargetCondition_ByPreSelecetedList` → `GetUniqueNameCardCount`);
//! - battle-area picks take the permanent's TOP card only (official Q&A);
//!   the leaving permanent's remaining digivolution cards go to trash;
//! - the hidden availability `ChangeCostClass` (`isCheckAvailability: true`)
//!   lets the play be OFFERED against the potential reduction
//!   (`min(3, unique-name candidates) × 4`);
//! - the reduced cost is what's paid; chosen cards become the new
//!   permanent's digivolution sources BEFORE its [On Play] effects drain;
//! - the play is NOT a DigiXros (`was_digixros` / `digixros_count` stay
//!   false / 0 for the pending play).

use std::sync::{Arc, Mutex};

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::GamePhase;
use digimon_engine::events::GameEvent;
use digimon_engine::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
use digimon_engine::{CardEffect, CardHandle, Effect, Game};

/// Committable physical candidates on the pending mixed-zone material page,
/// as `(action_id, card_id, zone)`. The assembly pick is an owned card choice:
/// a row's action id is a page slot, never a zone index, and rows rebind after
/// every pick — so tests address cards by identity, not by cached action ids.
fn material_rows(game: &Game) -> Vec<(u16, String, CardChoiceZone)> {
    let pending = game.pending_selection.as_ref().expect("material selection");
    // A finishing prompt with no candidates left (slot cap reached) carries
    // no card page at all: it offers nothing to commit.
    let Some(choice) = pending.card_choice.as_ref() else {
        return Vec::new();
    };
    choice
        .view(game, pending.selecting_player)
        .rows
        .into_iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .map(|row| {
            assert_eq!(row.owner, Some(pending.selecting_player));
            (
                row.action_id,
                row.card_id.expect("own battle-area/trash materials are public"),
                row.zone.expect("committable rows name their zone"),
            )
        })
        .collect()
}

fn material_ids(game: &Game) -> Vec<String> {
    let mut ids: Vec<String> = material_rows(game).into_iter().map(|(_, id, _)| id).collect();
    ids.sort();
    ids
}

/// The mask-legal action committing `card_id`, asserting it sits in `zone`.
fn material_action(game: &Game, card_id: &str, zone: CardChoiceZone) -> u16 {
    let (action, _, found) = material_rows(game)
        .into_iter()
        .find(|(_, id, _)| id == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be a legal physical candidate"));
    assert_eq!(
        format!("{found:?}"),
        format!("{zone:?}"),
        "{card_id} is offered from the wrong zone"
    );
    let player = game.pending_selection.as_ref().unwrap().selecting_player;
    assert_eq!(build_action_mask(game, player)[action as usize], 1.0);
    assert_eq!(build_action_mask(game, 1 - player)[action as usize], 0.0);
    action
}

const TEST_CTA_YAML: &str = r#"
card: TEST-CTA
name: Test Apocalymon
kind: digimon
level: 7
color: [white]
cost: 15
dp: 15000
traits: [Unidentified]

alt_paths:
  - kind: cast_time_assembly
    materials:
      - filter: { trait_has: "Dark Masters" }
        repeat: { min: 0, max: 3 }
        distinct_by: name
        zones: [battle_area, trash]
        cost_delta: -4

effects: []
"#;

fn dark_master(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 6);
    card.traits = vec!["Dark Masters".to_string()];
    card
}

fn builder_with_cta() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(TEST_CTA_YAML)
        .expect("TEST-CTA fixture compiles")
        .add_card(dark_master("DM-PIED", "Piedmon"))
        .add_card(dark_master("DM-METAL", "MetalSeadramon"))
        .add_card(dark_master("DM-MACHINE", "Machinedramon"))
        .add_card(dark_master("DM-PUPPET", "Puppetmon"))
        .add_card(dark_master("DM-PUPPET-ALT", "Puppetmon"))
        .add_card(make_test_card_with_level("VANILLA", "Vanilla", 3))
}

fn field_top_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn cta_sources(runner: &DebugRunner, player: u8) -> (String, Vec<String>) {
    let perm = runner.game.players[player as usize]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "TEST-CTA")
        .expect("TEST-CTA permanent on the field");
    let top = perm.top_card().card_id(&runner.game.card_data).to_string();
    let below: Vec<String> = perm
        .card_sources
        .iter()
        .rev()
        .skip(1)
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    (top, below)
}

fn play_cost_paid_events(runner: &DebugRunner) -> Vec<(String, i16)> {
    runner
        .game
        .events()
        .iter()
        .filter_map(|e| match e {
            GameEvent::Play {
                card_id, cost_paid, ..
            } => Some((card_id.clone(), *cost_paid)),
            _ => None,
        })
        .collect()
}

// ─── 1. The multi-select surfaces at play time and is optional-zero ─────────

#[test]
fn assembly_select_surfaces_optional_zero_and_decline_plays_at_full_cost() {
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(10).start();
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");
    runner.place_stack(0, &["VANILLA", "DM-MACHINE"]);

    let played = runner.game.play_from_hand(0, 0);
    assert_eq!(
        played, None,
        "the play must PARK on the assembly selection, not resolve synchronously"
    );
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    let sel = runner.pending_selection().expect("assembly selection");
    assert_eq!(sel.selecting_player, 0);
    assert!(
        sel.is_optional,
        "at memory 10 the full cost 15 is payable (10-15 = -5 >= -10), so 0 placements is legal"
    );
    // Candidates: the Dark Masters battle-area TOP + both trash Dark Masters
    // (the VANILLA source under Machinedramon is not offered).
    material_action(&runner.game, "DM-MACHINE", CardChoiceZone::BattleTop);
    material_action(&runner.game, "DM-PIED", CardChoiceZone::Trash);
    material_action(&runner.game, "DM-METAL", CardChoiceZone::Trash);
    assert_eq!(
        material_ids(&runner.game),
        vec!["DM-MACHINE", "DM-METAL", "DM-PIED"],
        "exactly the three Dark Masters are committable"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS (place 0) must be mask-legal"
    );

    runner.execute_action(0, PASS).expect("decline placement");

    // 10 - 15 = -5 → the turn passes and memory renormalizes to the new
    // turn player's perspective (+5 for them).
    assert_eq!(runner.memory(), 5, "full cost 15 paid from memory 10");
    assert_eq!(runner.game.turn_player(), 1, "overpay passes the turn");
    let (top, below) = cta_sources(&runner, 0);
    assert_eq!(top, "TEST-CTA");
    assert!(
        below.is_empty(),
        "no cards placed under on decline: {below:?}"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["DM-PIED".to_string(), "DM-METAL".to_string()],
        "trash untouched on decline"
    );
    assert!(
        field_top_ids(&runner, 0).contains(&"DM-MACHINE".to_string()),
        "battle-area candidate untouched on decline"
    );
    assert_eq!(
        play_cost_paid_events(&runner),
        vec![("TEST-CTA".to_string(), 15)],
        "Play event records the unreduced cost"
    );
}

// ─── 2. Different-name enforcement in the candidate mask ────────────────────

#[test]
fn picking_a_name_masks_out_same_named_candidates() {
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(10).start();
    runner.inject_trash(0, "DM-PUPPET");
    runner.inject_trash(0, "DM-PUPPET-ALT");
    runner.inject_trash(0, "DM-PIED");

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    assert_eq!(
        material_ids(&runner.game),
        vec!["DM-PIED", "DM-PUPPET", "DM-PUPPET-ALT"],
        "before any pick, both same-named Puppetmon and Piedmon are candidates"
    );

    let first = material_action(&runner.game, "DM-PUPPET", CardChoiceZone::Trash);
    runner
        .execute_action(0, first)
        .expect("pick the first Puppetmon");

    assert!(runner.pending_selection().is_some(), "selection re-installs");
    // The picked card stays in trash until commit (materials move at
    // placement time): DM-PUPPET is excluded as already-selected, the
    // same-named DM-PUPPET-ALT as a distinctness violation, and Piedmon
    // must remain.
    assert_eq!(
        runner.trash_size(0),
        3,
        "materials do not move until placement commits"
    );
    assert_eq!(
        material_ids(&runner.game),
        vec!["DM-PIED".to_string()],
        "the second same-named Puppetmon must be greyed out (distinct_by: name)"
    );
}

// ─── 3. Per-pick reduction, placement under, stack-remainder trashing ───────

#[test]
fn full_assembly_pays_reduced_cost_places_sources_and_trashes_material_stack() {
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(10).start();
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");
    runner.place_stack(0, &["VANILLA", "DM-MACHINE"]);

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    let pick = material_action(&runner.game, "DM-MACHINE", CardChoiceZone::BattleTop);
    runner
        .execute_action(0, pick)
        .expect("pick the battle-area Dark Masters");
    let pick = material_action(&runner.game, "DM-PIED", CardChoiceZone::Trash);
    runner
        .execute_action(0, pick)
        .expect("pick Piedmon from trash");
    let pick = material_action(&runner.game, "DM-METAL", CardChoiceZone::Trash);
    runner
        .execute_action(0, pick)
        .expect("pick MetalSeadramon from trash");

    // Slot cap (3) reached — the finishing prompt has no candidates left and
    // must therefore be declinable regardless of payability.
    let sel = runner.pending_selection().expect("finishing prompt");
    assert!(
        material_rows(&runner.game).is_empty(),
        "no candidates past the cap"
    );
    assert!(sel.valid_action_ids.is_empty(), "no candidate ids past the cap");
    assert!(sel.is_optional);
    assert_eq!(
        build_action_mask(&runner.game, 0)[PASS as usize],
        1.0,
        "finishing (PASS) is mask-legal at the cap"
    );
    runner.execute_action(0, PASS).expect("finish placing");

    assert_eq!(runner.memory(), 7, "cost 15 - 3×4 = 3 paid from memory 10");
    assert_eq!(
        play_cost_paid_events(&runner),
        vec![("TEST-CTA".to_string(), 3)],
        "Play event records the REDUCED cost"
    );

    let (top, below) = cta_sources(&runner, 0);
    assert_eq!(top, "TEST-CTA");
    assert_eq!(below.len(), 3, "all three chosen cards sit under the top");
    for id in ["DM-MACHINE", "DM-PIED", "DM-METAL"] {
        assert!(
            below.contains(&id.to_string()),
            "{id} must be a digivolution source: {below:?}"
        );
    }
    // The chosen battle-area permanent is gone; only its TOP card moved
    // under the played card — its own digivolution card goes to trash
    // (general_rule.pdf: digivolution cards of a leaving permanent are
    // trashed; DCGO take-material parity).
    assert!(
        !field_top_ids(&runner, 0).contains(&"DM-MACHINE".to_string()),
        "consumed battle-area material must leave the field"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["VANILLA".to_string()],
        "chosen trash cards leave the trash; the material's own stack is trashed"
    );
}

// ─── 4. DCGO CanNoSelect: cannot stop while the reduced cost is unpayable ───

#[test]
fn stop_is_masked_until_placements_make_the_cost_payable() {
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(0).start();
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");
    runner.inject_trash(0, "DM-MACHINE");

    // Mask-level: at memory 0 the printed 15 is unpayable (0-15 < -10), but
    // the potential reduction 3×4 = 12 makes the play offerable (cost 3).
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START) as usize],
        1.0,
        "declare-then-pay legality must be computed against the reduced cost"
    );

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    let sel = runner.pending_selection().expect("assembly selection");
    assert!(
        !sel.is_optional,
        "0 placed → cost 15 → 0-15 < -10: stopping must be masked (DCGO CanNoSelect)"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PASS as usize], 0.0);
    assert!(
        runner.execute_action(0, PASS).is_err(),
        "PASS is rejected while the remaining cost is unpayable"
    );

    let first = material_action(&runner.game, "DM-PIED", CardChoiceZone::Trash);
    runner
        .execute_action(0, first)
        .expect("first placement");
    let sel = runner.pending_selection().expect("selection re-installs");
    assert!(
        !sel.is_optional,
        "1 placed → cost 11 → 0-11 < -10: still forced"
    );
    assert!(!sel.valid_action_ids.contains(&PASS));
    assert_eq!(
        material_ids(&runner.game),
        vec!["DM-MACHINE", "DM-METAL"],
        "the picked Piedmon is no longer offered"
    );

    let next = material_action(&runner.game, "DM-METAL", CardChoiceZone::Trash);
    runner.execute_action(0, next).expect("second placement");
    let sel = runner.pending_selection().expect("selection re-installs");
    assert!(
        sel.is_optional,
        "2 placed → cost 7 → 0-7 >= -10: stopping becomes legal"
    );
    runner
        .execute_action(0, PASS)
        .expect("stop at two placements");

    // 0 - 7 = -7 → turn passes; memory renormalizes to +7 for the opponent.
    assert_eq!(runner.memory(), 7, "cost 15 - 2×4 = 7 paid from memory 0");
    assert_eq!(runner.game.turn_player(), 1, "overpay passes the turn");
    let (_, below) = cta_sources(&runner, 0);
    assert_eq!(below.len(), 2);
    assert_eq!(
        play_cost_paid_events(&runner),
        vec![("TEST-CTA".to_string(), 7)]
    );
}

// ─── 5. Mask availability counts UNIQUE names (hidden ChangeCost parity) ────

#[test]
fn mask_availability_counts_unique_names_only() {
    // Two same-named Puppetmon → unique-name potential = 1 × 4 → cost 11 →
    // unpayable at memory 0 → the play must NOT be offered.
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(0).start();
    runner.inject_trash(0, "DM-PUPPET");
    runner.inject_trash(0, "DM-PUPPET-ALT");

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START) as usize],
        0.0,
        "same-named candidates count once: potential 4 → cost 11 → unpayable at memory 0"
    );

    // Adding one distinctly-named card → potential 8 → cost 7 → payable.
    runner.inject_trash(0, "DM-PIED");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START) as usize],
        1.0,
        "two unique names → potential 8 → cost 7 → payable at memory 0"
    );
}

#[test]
fn mask_hides_play_with_no_candidates_and_unpayable_cost() {
    let runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(0).start();
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START) as usize],
        0.0,
        "no Dark Masters anywhere → no reduction → cost 15 unpayable at memory 0"
    );
}

// ─── 6. The assembly play is NOT a DigiXros ─────────────────────────────────

#[derive(Clone)]
struct DigiXrosProbeOnPlay {
    seen: Arc<Mutex<Vec<(bool, u8)>>>,
}

impl CardEffect for DigiXrosProbeOnPlay {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let seen = Arc::clone(&self.seen);
        vec![Effect::on_play(card)
            .name("record digixros flags")
            .process(move |ctx| {
                seen.lock()
                    .unwrap()
                    .push((ctx.was_digixros(), ctx.digixros_count()));
            })
            .build()]
    }
}

#[test]
fn assembly_play_is_not_marked_digixros_and_on_play_drains_after_placement() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(10).start();
    runner.register_effect(
        "TEST-CTA",
        Arc::new(DigiXrosProbeOnPlay {
            seen: Arc::clone(&seen),
        }),
    );
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    let first = material_action(&runner.game, "DM-PIED", CardChoiceZone::Trash);
    runner.execute_action(0, first).expect("pick");
    let next = material_action(&runner.game, "DM-METAL", CardChoiceZone::Trash);
    runner.execute_action(0, next).expect("pick");
    runner.execute_action(0, PASS).expect("finish");

    assert_eq!(
        *seen.lock().unwrap(),
        vec![(false, 0)],
        "the [On Play] body fired exactly once, AFTER assembly, and must NOT \
         observe the pending play as a DigiXros"
    );
    let (_, below) = cta_sources(&runner, 0);
    assert_eq!(
        below.len(),
        2,
        "sources were already under the permanent when [On Play] drained"
    );
}

// ─── 7. Clone-safety: clone mid-selection, resolve on the clone ─────────────

#[test]
fn clone_mid_assembly_selection_resolves_on_the_clone() {
    let mut runner = builder_with_cta().hand(0, &["TEST-CTA"]).memory(10).start();
    runner.inject_trash(0, "DM-PIED");
    runner.inject_trash(0, "DM-METAL");

    assert_eq!(runner.game.play_from_hand(0, 0), None);
    assert!(runner.pending_selection().is_some());

    let mut cloned = runner.game.clone();
    let pick = material_action(&cloned, "DM-PIED", CardChoiceZone::Trash);
    cloned
        .resolve_selection(0, pick)
        .expect("clone resolves the assembly pick via the resumable VM");
    // Finish on the clone: one placement → cost 11 → payable from 10.
    cloned.resolve_selection(0, PASS).expect("clone finishes");

    // 10 - 11 = -1 → turn passes on the clone; memory renormalizes to +1.
    assert_eq!(cloned.memory, 1, "clone paid 15-4 = 11 from memory 10");
    let clone_cta = cloned.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&cloned.card_data) == "TEST-CTA")
        .expect("clone played TEST-CTA");
    assert_eq!(clone_cta.card_sources.len(), 2, "top + 1 placed source");

    // The original is untouched and still parked on its own selection.
    assert!(runner.pending_selection().is_some());
    assert_eq!(runner.memory(), 10);
    assert_eq!(runner.trash_size(0), 2);
    runner.execute_action(0, PASS).expect("original still live");
    assert_eq!(
        runner.memory(),
        5,
        "original pays full 15 → turn passes, +5 for opponent"
    );
}
