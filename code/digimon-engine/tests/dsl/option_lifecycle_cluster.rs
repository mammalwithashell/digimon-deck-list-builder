//! Option-lifecycle cluster (store-champs June-2026 audit).
//!
//! Two engine gaps, proved here via hand-written `CardEffect` fixtures that
//! simulate the BLOCKED driver cards' shapes (the cards themselves are NOT
//! authored here — only the substrate):
//!
//! ## Gap 1 — `place_self_under_permanent` composes with the [Main] Option-play path
//! P-180 / EX7-071 / EX7-070 end their [Main] with "Then, place this card as
//! the bottom digivolution card of 1 of your [Three Musketeers] trait Digimon."
//! On the `play_option_from_hand` path the Option is the in-flight
//! `pending_option` (not yet a field permanent), so the old
//! `move_self_option_under_permanent` early-returned (no `source_permanent`).
//! The fix mirrors `place_self_as_delay_option_permanent`: claim `pending_option`
//! and seat the Option FACE-UP as the target's bottom digivolution source.
//! The `gap1_dsl_*` tests below exercise the YAML-reachable DSL verb
//! `place_self_under_permanent: { target, face_down }`
//! (G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL, resolved 2026-07-03), which
//! lowers to the same primitive; EX7-071 is the first shipped consumer.
//!
//! ## Gap 2 — Option USE routed from non-hand origins (trash / revealed / bound source)
//! BT25-083 ("use 1 [Three Musketeers] Option from your trash with the cost
//! reduced by 3"), BT21-062 ("use 1 [Ragnarok Cannon] from hand or trash
//! without paying the cost"), EX7-048 ("reveal top 6 … use 1 [Three Musketeers]
//! Option among them free"), BT25-085 ("use 1 [3M]/[TS] Option from hand OR
//! this Digimon's digivolution cards free"). All route through the SAME
//! `play_option_core` [Main] resolution + disposal, only the source zone differs.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledCostDelta, CompiledPlayerRef, CompiledPredicate, CompiledStep,
};
use digimon_engine::action::space::TRASH_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CostDelta, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::OptionPlayResult;

// ─── Shared fixtures ──────────────────────────────────────────────────────────

fn tm_option(id: &str, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.play_cost = cost;
    card.traits = vec!["Three Musketeers".to_string()];
    card
}

/// A vanilla Digimon usable as a self-place target / digivolution host.
fn tm_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(3000);
    card.play_cost = 4;
    card.traits = vec!["Three Musketeers".to_string()];
    card
}

fn seed_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    let iid = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(idx, player, iid));
}

// ══════════════════════════════════════════════════════════════════════════════
// Gap 1 — place THIS Option under a chosen permanent on the [Main]-play path
// ══════════════════════════════════════════════════════════════════════════════

/// An Option whose `[Main]` body seats the Option itself as the bottom
/// digivolution source of the first own Digimon (index 0) — the P-180 tail
/// shape. Faithful cards resolve the target via a `select_own_permanent`; the
/// engine method under test takes an already-resolved `PermanentHandle`, so the
/// fixture pins index 0 to stay deterministic.
struct PlaceSelfUnderFirstDigimon {
    face_down: bool,
}

impl CardEffect for PlaceSelfUnderFirstDigimon {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let face_down = self.face_down;
        vec![Effect::when_attacking(card)
            .option_main()
            .name("[Main] place self under Digimon")
            .process(move |ctx: &mut EffectContext| {
                let target = PermanentHandle {
                    player: ctx.player,
                    index: 0,
                };
                let _ = ctx.place_self_under_permanent(target, face_down);
            })
            .build()]
    }
}

#[test]
fn gap1_place_self_under_permanent_on_play_path_seats_face_up_no_trash() {
    let mut runner = DebugRunner::builder()
        .add_card(tm_digimon("TM-DIGI"))
        .add_card(tm_option("TM-OPT", 0))
        .hand(1, &["TM-OPT"])
        .memory(3)
        .start();
    runner.place_on_field(1, "TM-DIGI", Some(0)); // host at index 0
    runner.register_effect("TM-OPT", Arc::new(PlaceSelfUnderFirstDigimon { face_down: false }));

    // Sanity: host starts as a lone top card (no digivolution sources).
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "host Digimon should start with only its top card"
    );

    let result = runner.game.play_option_from_hand(1, 0);
    assert_eq!(
        result,
        OptionPlayResult::Trashed,
        "the Option-play resolves synchronously (no pending selection)"
    );

    // The Option must NOT be in trash — it MOVED under the host.
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"),
        "Gap 1: placed-under Option must NOT be trashed"
    );
    // The Option must be the bottom digivolution source (index 0) of the host,
    // seated FACE-UP.
    let host = &runner.game.players[1].battle_area[0];
    assert_eq!(
        host.card_sources.len(),
        2,
        "host should now carry the placed Option as a digivolution source"
    );
    assert_eq!(
        host.card_sources[0].card_id(&runner.game.card_data),
        "TM-OPT",
        "the Option is the BOTTOM digivolution source"
    );
    assert!(
        !host.card_sources[0].face_down,
        "Gap 1: the Option is seated FACE-UP (P-180 places it face up)"
    );
    assert!(
        runner.game.pending_option.is_none(),
        "the in-flight pending_option slot must be cleared (claimed by the place)"
    );
}

#[test]
fn gap1_place_self_under_permanent_face_down_axis_settable() {
    let mut runner = DebugRunner::builder()
        .add_card(tm_digimon("TM-DIGI"))
        .add_card(tm_option("TM-OPT", 0))
        .hand(1, &["TM-OPT"])
        .memory(3)
        .start();
    runner.place_on_field(1, "TM-DIGI", Some(0));
    runner.register_effect("TM-OPT", Arc::new(PlaceSelfUnderFirstDigimon { face_down: true }));

    let _ = runner.game.play_option_from_hand(1, 0);
    let host = &runner.game.players[1].battle_area[0];
    assert!(
        host.card_sources[0].face_down,
        "the face_down axis is honored on the play path"
    );
}

/// An Option whose `[Main]` body targets a NONEXISTENT permanent (index 99), so
/// the self-place returns false and the Option must dispose normally to trash.
struct PlaceSelfUnderMissing;
impl CardEffect for PlaceSelfUnderMissing {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("[Main] place self under missing")
            .process(|ctx: &mut EffectContext| {
                let target = PermanentHandle {
                    player: ctx.player,
                    index: 99,
                };
                // The place must FAIL (no such permanent) — the Option then
                // disposes normally.
                assert!(!ctx.place_self_under_permanent(target, false));
            })
            .build()]
    }
}

#[test]
fn gap1_place_self_under_permanent_noop_when_no_host_leaves_option_in_flight() {
    // A field Digimon makes the Option color-usable, but the effect targets a
    // nonexistent permanent → the place fails, and the Option must still be
    // disposed normally (Standard → trash) by the play pipeline.
    let mut runner = DebugRunner::builder()
        .add_card(tm_digimon("TM-DIGI"))
        .add_card(tm_option("TM-OPT", 0))
        .hand(1, &["TM-OPT"])
        .memory(3)
        .start();
    runner.place_on_field(1, "TM-DIGI", Some(0));
    runner.register_effect("TM-OPT", Arc::new(PlaceSelfUnderMissing));

    let result = runner.game.play_option_from_hand(1, 0);
    assert_eq!(result, OptionPlayResult::Trashed);
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT"),
        "with no legal host, the Option disposes to trash as a Standard Option"
    );
    // The host Digimon at index 0 must be untouched (nothing seated under it).
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "the failed place must not seat the Option under the wrong permanent"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Gap 1 — DSL verb: `place_self_under_permanent` (YAML-reachable)
// ══════════════════════════════════════════════════════════════════════════════

/// The P-180 / EX7-070 / EX7-071 [Main]-tail shape, authored END-TO-END in
/// YAML: select an own [Three Musketeers] Digimon, then seat THIS Option
/// under it via the `place_self_under_permanent` DSL step
/// (G-OPTION-PLACE-SELF-UNDER-PERMANENT-DSL — the step dispatches to the
/// engine primitive proven by the gap1_* tests above). `face_down` is
/// intentionally OMITTED to prove the serde default (`false` — face-up).
const PLACE_SELF_DSL_YAML: &str = r#"
card: TEST-PLACE-SELF-DSL
name: Place Self Under Permanent
kind: option
color: [purple]
cost: 0
effects:
  - when: main_from_hand
    summary: "[Main] Place this card as the bottom digivolution card of 1 of your [Three Musketeers] Digimon"
    process:
      - select_own_permanent:
          bind_as: tm_target
          filter: { kind: digimon, trait_has: "Three Musketeers" }
          prompt: "Place this card as the bottom digivolution card of 1 of your [Three Musketeers] Digimon"
      - place_self_under_permanent: { target: tm_target }
"#;

/// A purple [Three Musketeers] Digimon — satisfies both the Option's printed
/// color requirement and the placement filter.
fn purple_tm_digimon(id: &str) -> CardData {
    let mut card = tm_digimon(id);
    card.colors = vec![CardColor::Purple];
    card
}

/// A purple NON-[Three Musketeers] Digimon — satisfies the Option's color
/// requirement but must never be a placement candidate.
fn purple_plain_digimon(id: &str) -> CardData {
    let mut card = tm_digimon(id);
    card.colors = vec![CardColor::Purple];
    card.traits = vec!["Beast".to_string()];
    card
}

#[test]
fn gap1_dsl_place_self_under_permanent_seats_face_up_under_selected_host() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(PLACE_SELF_DSL_YAML)
        .expect("place-self YAML parses + compiles")
        .add_card(purple_tm_digimon("TM-HOST"))
        .hand(1, &["TEST-PLACE-SELF-DSL"])
        .memory(3)
        .start();
    runner.place_on_field(1, "TM-HOST", Some(0));

    let result = runner.game.play_option_from_hand(1, 0);
    assert_eq!(
        result,
        OptionPlayResult::Pending,
        "the [Main] body parks on the mandatory placement pick"
    );

    // Resolve the (sole-candidate) own-field pick.
    let (player, action) = {
        let sel = runner
            .game
            .pending_selection
            .as_ref()
            .expect("placement pick pending");
        assert!(
            !sel.is_optional,
            "the placement pick is MANDATORY when a candidate exists (DCGO canNoSelect: false)"
        );
        (sel.selecting_player, sel.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("placement pick resolves");
    runner.game.drain_effect_queue();

    // The Option MOVED under the host — bottom source, FACE-UP, not trashed.
    assert!(
        !runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TEST-PLACE-SELF-DSL"),
        "the placed Option must NOT be trashed"
    );
    let host = &runner.game.players[1].battle_area[0];
    assert_eq!(host.card_sources.len(), 2, "host gained the Option as a source");
    assert_eq!(
        host.card_sources[0].card_id(&runner.game.card_data),
        "TEST-PLACE-SELF-DSL",
        "the Option is the BOTTOM digivolution source"
    );
    assert!(
        !host.card_sources[0].face_down,
        "omitted face_down defaults to FALSE — seated FACE-UP"
    );
    assert!(
        runner.game.pending_option.is_none(),
        "the in-flight pending_option slot was claimed by the place"
    );
}

#[test]
fn gap1_dsl_place_self_under_permanent_no_candidate_select_self_skips_to_trash() {
    // Only a NON-[Three Musketeers] purple Digimon in play: the Option is
    // color-usable, but the mandatory placement select has ZERO candidates →
    // it self-skips (no prompt), the binding stays unset, the place step
    // silently no-ops, and the Option disposes normally to trash.
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(PLACE_SELF_DSL_YAML)
        .expect("place-self YAML parses + compiles")
        .add_card(purple_plain_digimon("PLAIN-HOST"))
        .hand(1, &["TEST-PLACE-SELF-DSL"])
        .memory(3)
        .start();
    runner.place_on_field(1, "PLAIN-HOST", Some(0));

    let result = runner.game.play_option_from_hand(1, 0);
    assert_eq!(
        result,
        OptionPlayResult::Trashed,
        "no placement candidate → the select self-skips and the play resolves synchronously"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "a zero-candidate mandatory select must not park a prompt"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TEST-PLACE-SELF-DSL"),
        "with no legal host the Option disposes to trash as a Standard Option"
    );
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "the non-[Three Musketeers] Digimon must never receive the placed card"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Gap 2 — Option USE from non-hand origins
// ══════════════════════════════════════════════════════════════════════════════

/// A trait-noop `OptionMain` body so the used Option is a legal Standard Option.
struct OptMainNoop;
impl CardEffect for OptMainNoop {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain no-op")
            .process(|_| {})
            .build()]
    }
}

#[test]
fn gap2_use_option_from_trash_free_disposes_and_pays_nothing() {
    // BT21-062 shape: "use 1 Option from trash without paying the cost".
    let mut src = make_test_card("SRC", "Src");
    src.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(tm_option("RAGNAROK", 5))
        .hand(1, &["SRC"])
        .memory(3)
        .start();
    seed_trash(&mut runner, 1, "RAGNAROK");
    runner.register_effect("RAGNAROK", Arc::new(OptMainNoop));

    let source_card = runner.game.players[1].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let result = ctx.use_option_from_trash(1, 0, CostDelta::Free);
        assert_eq!(
            result,
            OptionPlayResult::Trashed,
            "trash-origin free use resolves to Trashed"
        );
    }
    assert_eq!(
        runner.game.memory, before,
        "free use pays no memory even though the printed cost is 5"
    );
    assert!(
        runner.game.pending_option.is_none(),
        "the Option fully resolved"
    );
}

#[test]
fn gap2_use_option_from_trash_cost_reduced_pays_delta() {
    // BT25-083 shape: "use 1 Option from trash with the cost reduced by 3"
    // (printed use cost 5 → pays 2).
    let mut src = make_test_card("SRC", "Src");
    src.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(tm_option("TM-OPT-5", 5))
        .hand(1, &["SRC"])
        .memory(6)
        .start();
    seed_trash(&mut runner, 1, "TM-OPT-5");
    runner.register_effect("TM-OPT-5", Arc::new(OptMainNoop));

    let source_card = runner.game.players[1].hand[0].handle();
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let result = ctx.use_option_from_trash(1, 0, CostDelta::Reduce(3));
        assert_eq!(result, OptionPlayResult::Trashed);
    }
    assert_eq!(
        before - runner.game.memory,
        2,
        "printed use cost 5 reduced by 3 → 2 memory actually paid"
    );
}

#[test]
fn gap2_use_option_from_revealed_free_uses_and_clears_reveal_pool() {
    // EX7-048 shape: reveal cards, use 1 Option among them free.
    let mut src = make_test_card("SRC", "Src");
    src.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(tm_option("REV-OPT", 4))
        .deck(1, &["REV-OPT"])
        .hand(1, &["SRC"])
        .memory(3)
        .start();
    runner.register_effect("REV-OPT", Arc::new(OptMainNoop));

    let source_card = runner.game.players[1].hand[0].handle();
    // Reveal the top card into the reveal pool.
    let reveal_handle = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        ctx.reveal_top_deck(1, 1);
        ctx.game.revealed_cards[0].handle()
    };
    let before = runner.game.memory;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let result = ctx.use_option_from_revealed(reveal_handle, CostDelta::Free);
        assert_eq!(
            result,
            OptionPlayResult::Trashed,
            "revealed-origin free use resolves to Trashed"
        );
    }
    assert_eq!(runner.game.memory, before, "revealed free use pays nothing");
    assert!(
        !runner
            .game
            .revealed_cards
            .iter()
            .any(|c| c.handle() == reveal_handle),
        "the used Option is removed from the reveal pool"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "REV-OPT"),
        "the used Standard Option disposes to trash"
    );
}

#[test]
fn gap2_use_option_from_digivolution_source_free() {
    // BT25-085 shape: "use 1 Option from this Digimon's digivolution cards free".
    // Build a Digimon carrying an Option as a digivolution (under) source, then
    // use that Option from the stack.
    let mut runner = DebugRunner::builder()
        .add_card(tm_digimon("HOST"))
        .add_card(tm_option("UNDER-OPT", 4))
        .memory(3)
        .start();
    // stack: bottom = UNDER-OPT (the Option), top = HOST
    let host_handle = runner.place_stack(1, &["UNDER-OPT", "HOST"]);
    runner.register_effect("UNDER-OPT", Arc::new(OptMainNoop));

    let source_card = runner.game.players[1].battle_area[0].top_card().handle();
    // The Option is the bottom source (index 0) of the host stack.
    let opt_source = runner.game.players[1].battle_area[0].card_sources[0].handle();

    let before = runner.game.memory;
    {
        let mut ctx =
            EffectContext::new(&mut runner.game, source_card, Some(host_handle), 1);
        let result = ctx.use_option_from_source(host_handle, opt_source, CostDelta::Free);
        assert_eq!(
            result,
            OptionPlayResult::Trashed,
            "digivolution-source-origin free use resolves to Trashed"
        );
    }
    assert_eq!(runner.game.memory, before, "source-origin free use pays nothing");
    // The Option left the host's stack and went to trash.
    assert_eq!(
        runner.game.players[1].battle_area[0].card_sources.len(),
        1,
        "the Option was pulled out of the host's digivolution stack"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "UNDER-OPT"),
        "the used Standard Option disposes to trash"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Gap 2 — DSL verb: `use_option_from_trash`
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn gap2_dsl_use_option_from_trash_installs_trash_selection_over_matching_options() {
    // BT25-083 shape via the DSL verb: "use 1 [Three Musketeers] Option from
    // your trash with the cost reduced by 3".
    let mut src = make_test_card("SRC", "Src");
    src.card_kind = CardKind::Tamer;

    // TM-OPT-5: matching Option (printed use cost 5). OFF: a non-Three-Musketeers
    // Option (filtered out). DIGI: a Digimon in trash (wrong kind, filtered out).
    let mut off_opt = make_test_card("OFF-OPT", "OffOpt");
    off_opt.card_kind = CardKind::Option;
    off_opt.play_cost = 2;

    let mut runner = DebugRunner::builder()
        .add_card(src)
        .add_card(tm_option("TM-OPT-5", 5))
        .add_card(off_opt)
        .add_card(tm_digimon("DIGI"))
        .hand(1, &["SRC"])
        .memory(6)
        .start();
    seed_trash(&mut runner, 1, "TM-OPT-5"); // trash[0] — matching
    seed_trash(&mut runner, 1, "OFF-OPT"); // trash[1] — wrong trait
    seed_trash(&mut runner, 1, "DIGI"); // trash[2] — wrong kind
    runner.register_effect("TM-OPT-5", Arc::new(OptMainNoop));
    runner.register_effect("OFF-OPT", Arc::new(OptMainNoop));

    let source_card = runner.game.players[1].hand[0].handle();
    let steps = vec![CompiledStep::UseOptionFromTrash {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Option),
            trait_has: Some("Three Musketeers".to_string()),
            ..CompiledPredicate::default()
        },
        cost_delta: Some(CompiledCostDelta::Reduce(3)),
        optional: false,
        prompt: None,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 1);
        let mut bindings = Bindings::new();
        assert_eq!(
            run_steps(&steps, &mut ctx, &mut bindings),
            RunOutcome::Parked,
            "the trash-Option selection parks a pending selection"
        );
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("eligible trash Option installs a trash-selection prompt");
    assert_eq!(
        pending.valid_action_ids,
        vec![TRASH_EFFECT_START],
        "only the matching [Three Musketeers] Option (trash[0]) is selectable — \
         the off-trait Option and the Digimon are filtered out"
    );

    let before = runner.game.memory;
    runner
        .game
        .resolve_selection(1, TRASH_EFFECT_START)
        .expect("selecting the matching trash Option resolves");

    assert_eq!(
        before - runner.game.memory,
        2,
        "printed use cost 5 reduced by 3 → 2 memory paid"
    );
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-OPT-5"),
        "the used Standard Option cycles back to trash after resolving"
    );
}
