//! G016 binding half — `bind_as` on `PlayTokenArgs`.
//!
//! Verifies that a `PlayToken` step with `bind_as` registers the created
//! token's `PermanentHandle` under the given name in the `Bindings` table,
//! so that subsequent steps (e.g. `AddModifier`) can reference the token.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledModifierTarget, CompiledModifierValue, CompiledPlayerRef,
    CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::ModifierType;

/// A `PlayToken` step with `bind_as: Some("tok")` must insert the created
/// permanent's handle into the bindings table under `"tok"`, so that a
/// subsequent `AddModifier` step targeting the `"tok"` binding applies the
/// modifier to the token — not to some other permanent.
#[test]
fn play_token_binds_created_handle() {
    // "petrification" is registered in the default token registry that
    // DebugRunner's game creates via `build_registry()`.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-TKB", "TokenBindSrc"))
        .hand(0, &["TST-TKB"])
        .memory(5)
        .start();

    let src_card = runner.game.players[0].hand[0].handle();
    let battle_before = runner.game.players[0].battle_area.len();

    let mut bindings = Bindings::new();

    // Step 1: play the token and bind the resulting handle as "tok".
    // Step 2: add CannotAttack modifier to "tok".
    let steps = vec![
        CompiledStep::PlayToken {
            controller: CompiledPlayerRef::You,
            token_name: "petrification".to_string(),
            bind_as: Some("tok".to_string()),
        },
        CompiledStep::AddModifier {
            target: CompiledModifierTarget::Binding(CompiledBindingRef::Named("tok".into())),
            modifier: "CannotAttack".into(),
            value: CompiledModifierValue::Literal(0),
            expiry: "end_of_turn".into(),
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Token should have been created on the battle area.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area should gain a token permanent"
    );

    // The binding "tok" must resolve to the token permanent. Post-change
    // `fix-played-binding-uses-provenance`, play-verb `bind_as` stores
    // `BindingValue::PlayedPermanent { token, fallback }` rather than
    // positional `Permanent`. The diagnostic getter `get_played_permanent`
    // returns the `(token, fallback)` pair; the fallback handle is the slot
    // the play landed in (matches the token's resolved handle since the
    // token is still a battle-area top here).
    let (_token, tok_handle) = bindings
        .get_played_permanent("tok")
        .expect("bind_as should have inserted the token handle under \"tok\"");

    // The CannotAttack modifier must be on the token — proving the handle
    // correctly identifies the created token permanent.
    assert!(
        runner
            .game
            .modifiers
            .has(tok_handle, ModifierType::CannotAttack),
        "CannotAttack modifier should be on the token (via the bound handle)"
    );
}
