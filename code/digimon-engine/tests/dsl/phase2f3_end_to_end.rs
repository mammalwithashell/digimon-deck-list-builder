//! Phase 2f3 Task 3 — end-to-end YAML test for `as_selecting_player`.
//!
//! Loads "Opponent's Vote" — a card whose OnPlay reads:
//!     "When this Digimon plays, your opponent chooses one of YOUR Digimon.
//!      That Digimon gets -3000 DP for the turn."
//!
//! This exercises the full pipeline:
//!   YAML literal → `serde_yml::from_str` → `digimon_dsl::compile::compile`
//!   → `DslCardEffect::new` → trigger via `OnPlay` (process closure) →
//!   `as_selecting_player: { of: opponent }` installs a `select_own_permanent`
//!   parked under override player = P1 → P1 resolves the parked selection →
//!   tail's `add_dp_modifier` applies -3000 to the chosen permanent.
//!
//! Pattern mirrors `phase2f1_end_to_end.rs` (the established Phase 2f shape).
//!
//! `select_own_permanent` filters on the *controller's* permanents; the
//! `as_selecting_player: { of: opponent }` override only changes who is
//! prompted, not the candidate set. Hence "opponent picks from your perms" —
//! exactly the canonical "opponent chooses your Digimon" pattern.
//!
//! Uses `expiry: end_of_turn` — the canonical snake_case form accepted by
//! both the validator (`KNOWN_EXPIRY_KEYS`) and the engine (`lookup_expiry`).
//! See `tests/dsl/expiry_parity.rs` for the lockstep enforcement.

use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::permanent::PermanentHandle;

#[test]
fn opponents_vote_card_lets_opponent_apply_minus_3000_dp_to_controllers_perm() {
    let yaml = r#"
card: TST-VOTE
name: "Opponent's Vote"
kind: digimon
level: 4
color: [purple]
cost: 5
dp: 4000
effects:
  - when: on_play
    process:
      - as_selecting_player:
          of: opponent
          body:
            - select_own_permanent:
                bind_as: chosen
                filter: {}
                prompt: "Pick one of your Digimon"
            - add_dp_modifier:
                target: chosen
                value: -3000
                expiry: end_of_turn
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("YAML parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compiles cleanly");

    // Seed: TST-VOTE in P0's hand (the OnPlay source, not yet on field), and
    // two permanents on P0's field (TST-A, TST-B) — P1 will choose between
    // them. `make_test_card` gives every card a base DP of 2000.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-VOTE", "Opponent's Vote"))
        .add_card(make_test_card("TST-A", "Permanent A"))
        .add_card(make_test_card("TST-B", "Permanent B"))
        .hand(0, &["TST-VOTE"])
        .build();

    let perm_a: PermanentHandle = runner.place_on_field(0, "TST-A", None);
    let perm_b: PermanentHandle = runner.place_on_field(0, "TST-B", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 2);

    let base_dp_a = runner.effective_dp(perm_a).expect("perm A has DP");
    let base_dp_b = runner.effective_dp(perm_b).expect("perm B has DP");
    assert_eq!(base_dp_a, 2000, "make_test_card baseline DP");
    assert_eq!(base_dp_b, 2000, "make_test_card baseline DP");

    // Build the DSL effect and grab its OnPlay process closure.
    let dsl_effect = DslCardEffect::new(Arc::new(compiled));
    let src_card: CardHandle = runner.game.players[0].hand[0].handle();

    let effects = dsl_effect.effects(src_card);
    assert_eq!(effects.len(), 1, "exactly one OnPlay effect");
    let process = effects[0].process.as_ref().expect("OnPlay has a process");

    // Invoke the process — installs the body's select_own_permanent under
    // the AsSelectingPlayer override (P1) and parks.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        process(&mut ctx);
    }

    // Body's select parks; the prompt is routed to P1 (the opponent).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("body's select_own_permanent should have parked");
    assert_eq!(
        pending.selecting_player, 1,
        "AsSelectingPlayer with of=opponent must route the prompt to P1"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "two perms on P0's field → two valid action ids for P1 to choose from"
    );

    // P1 picks the FIRST candidate — by `install_field_selection` semantics
    // this maps to the controller's slot 0 (perm A). We assert which perm
    // the chosen action_id encodes so the post-condition is unambiguous.
    let chosen_action_id = pending.valid_action_ids[0];

    runner
        .game
        .resolve_selection(1, chosen_action_id)
        .expect("P1 (opponent) resolves the parked selection");

    // Post-conditions: pending selection cleared; perm A took -3000 DP for
    // the turn; perm B is unaffected.
    assert!(
        runner.game.pending_selection.is_none(),
        "all selections should be resolved after P1's pick"
    );
    let after_dp_a = runner
        .effective_dp(perm_a)
        .expect("perm A still on field");
    let after_dp_b = runner
        .effective_dp(perm_b)
        .expect("perm B still on field");
    assert_eq!(
        after_dp_a,
        base_dp_a - 3000,
        "the chosen permanent (P0 slot 0, perm A) takes -3000 DP for the turn"
    );
    assert_eq!(
        after_dp_b, base_dp_b,
        "the unpicked permanent (perm B) is unchanged"
    );
}
